use anyhow::bail;
use anyhow::Result;

use super::*;

pub fn handle_cli(cmd: Cmds) -> Result<()> {
    let Some(command) = cmd.command else {
        return Ok(());
    };

    match command {
        ArgCommand::Profile { command } => handle_profile(command),
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        ArgCommand::Service { command } => handle_service(command),
        ArgCommand::Mode { mode } => handle_mode(mode),
        ArgCommand::Update { ci, target } => handle_update(ci, target),
        ArgCommand::Migrate { .. } => unreachable!("migrate handled in handle_early_exit"),
    }
}

// ── Profile ──────────────────────────────────────────────────────────

fn handle_profile(command: ProfileCommand) -> Result<()> {
    match command {
        ProfileCommand::Update {
            all,
            name,
            with_proxy,
            without_proxyprovider,
            r#type: type_filter,
        } => {
            let profiles: Vec<crate::config::database::Profile> = if all {
                crate::functions::file::profile::db::get_all()
            } else if let Some(name) = &name {
                match crate::functions::file::profile::db::get(name) {
                    Some(pf) => vec![pf],
                    None => {
                        eprintln!("Profile not found: {name}");
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("No profile selected! Use --all or --name <NAME>.");
                return Ok(());
            };

            let profiles: Vec<_> = if let Some(filter) = &type_filter {
                profiles.into_iter().filter(|pf| filter.matches(&pf.dtype)).collect()
            } else {
                profiles
            };

            if profiles.is_empty() {
                if type_filter.is_some() {
                    println!("No profiles match the given type filter.");
                } else if all {
                    println!("No profiles in database.");
                }
                return Ok(());
            }

            let rt = tokio::runtime::Runtime::new()?;
            for pf in &profiles {
                println!("Updating profile: {}", pf.name);
                if without_proxyprovider {
                    let mut pm = crate::config::CONFIG.data.lock().unwrap();
                    pm.set_no_pp(&pf.name, true);
                    pm.to_file()?;
                }
                let pf = pf.clone();
                match rt.block_on(crate::functions::file::profile::update_profile(
                    pf, with_proxy,
                )) {
                    Ok(result) => {
                        println!("  Updated: {} ({} resources)", result.name, result.net_updates.len());
                    }
                    Err(e) => {
                        eprintln!("  Error: {e}");
                    }
                }
            }
            println!("Done.");
            Ok(())
        }
        ProfileCommand::Select { name } => {
            if let Some(name) = name {
                let Some(pf) = crate::functions::file::profile::db::get(&name) else {
                    eprintln!("Profile not found in database: {name}");
                    std::process::exit(1);
                };
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(crate::functions::file::profile::select(pf))?;
                println!("Profile selected: {name}");
            } else {
                let current = crate::functions::file::profile::db::get_current();
                println!("Current Profile: {}", current.name);
            }
            Ok(())
        }
        ProfileCommand::List {
            name_only,
            r#type: type_filter,
        } => {
            let pfs = crate::functions::file::profile::db::get_all();
            let mut pfs: Vec<_> = if let Some(filter) = &type_filter {
                pfs.into_iter().filter(|pf| filter.matches(&pf.dtype)).collect()
            } else {
                pfs
            };
            pfs.sort_by(|a, b| a.name.cmp(&b.name));
            if pfs.is_empty() {
                if type_filter.is_some() {
                    println!("No profiles match the given type filter.");
                } else {
                    println!("No profiles found.");
                }
                return Ok(());
            }
            for pf in &pfs {
                if name_only {
                    println!("{}", pf.name);
                } else {
                    println!(
                        "{}: {}",
                        pf.name,
                        pf.dtype.get_domain().as_deref().unwrap_or("Unknown")
                    );
                }
            }
            Ok(())
        }
    }
}

// ── Service ──────────────────────────────────────────────────────────

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn handle_service(command: ServiceCommand) -> Result<()> {
    match command {
        ServiceCommand::Restart { soft } => {
            if soft {
                crate::functions::restful::control::restart(None)
                    .map_err(|e| anyhow::anyhow!("Soft restart failed: {e}"))?;
                println!("Core restarted (soft).");
            } else {
                let output = crate::functions::command::restart_service(None)?;
                println!("{output}");
            }
            Ok(())
        }
        ServiceCommand::Stop => {
            let output = crate::functions::command::stop_service(None)?;
            println!("{output}");
            Ok(())
        }
    }
}

// ── Mode ─────────────────────────────────────────────────────────────

fn handle_mode(mode: Option<ModeCommand>) -> Result<()> {
    match mode {
        None => {
            let config =
                crate::functions::restful::config::fetch()
                    .map_err(|e| anyhow::anyhow!("Failed to fetch config: {e}"))?;
            println!("{}", config.mode);
            Ok(())
        }
        Some(mode_cmd) => {
            let mode_str = match mode_cmd {
                ModeCommand::Rule => "Rule",
                ModeCommand::Direct => "Direct",
                ModeCommand::Global => "Global",
            };
            let payload = serde_json::json!({"mode": mode_str}).to_string();
            crate::functions::restful::config::patch(payload)
                .map_err(|e| anyhow::anyhow!("Failed to set mode: {e}"))?;
            println!("Mode set to: {mode_str}");
            Ok(())
        }
    }
}

// ── Update ───────────────────────────────────────────────────────────

fn handle_update(ci: bool, target: Target) -> Result<()> {
    match target {
        Target::Clashtui => update_clashtui(ci),
        Target::Mihomo => update_mihomo(ci),
    }
}

fn update_clashtui(ci: bool) -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    let repo = "JohanChane/clashtui";
    let release = fetch_latest_release(repo, ci)?;
    let latest = release.tag_name.trim_start_matches('v');

    if latest == current {
        println!("Already up to date (v{current}).");
        return Ok(());
    }

    println!("New version available: v{latest} (current: v{current})");

    let asset = find_linux_asset(&release.assets)?;
    println!("Downloading {}...", asset.name);
    download_and_replace(&asset.browser_download_url)?;
    println!("Updated to v{latest}.");
    Ok(())
}

fn update_mihomo(ci: bool) -> Result<()> {
    let current = crate::functions::restful::control::version()
        .map_err(|e| anyhow::anyhow!("Failed to fetch core version: {e}"))?;
    let current = current.trim().trim_matches('"');

    let repo = "MetaCubeX/mihomo";
    let release = fetch_latest_release(repo, ci)?;
    let latest = release.tag_name.trim_start_matches('v');

    if latest == current.trim_start_matches('v') {
        println!("Already up to date (v{latest}).");
        return Ok(());
    }

    println!("New version available: v{latest} (current: {current})");

    let mihomo_path = {
        let path = &crate::config::CONFIG.cfg_file.mihomo.core.config_path;
        if path.is_empty() {
            std::env::current_dir()?.join("mihomo")
        } else {
            std::path::PathBuf::from(path)
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("mihomo")
        }
    };

    let asset = find_linux_asset(&release.assets)?;
    println!("Downloading {}...", asset.name);
    download_to_path(&asset.browser_download_url, &mihomo_path)?;
    println!("Updated mihomo to v{latest}.");
    Ok(())
}

// ── GitHub helpers ───────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct GhRelease {
    tag_name: String,
    assets: Vec<GhAsset>,
}

#[derive(serde::Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

fn fetch_latest_release(repo: &str, ci: bool) -> Result<GhRelease> {
    let url = if ci {
        format!("https://api.github.com/repos/{repo}/releases?per_page=1")
    } else {
        format!("https://api.github.com/repos/{repo}/releases/latest")
    };

    let mut releases: Vec<GhRelease> = if ci {
        minreq::get(url)
            .with_header("User-Agent", "clashtui")
            .with_timeout(10)
            .send()
            .map_err(|e| anyhow::anyhow!("Failed to fetch releases: {e}"))?
            .json()
            .map_err(|e| anyhow::anyhow!("Failed to parse releases: {e}"))?
    } else {
        vec![minreq::get(url)
            .with_header("User-Agent", "clashtui")
            .with_timeout(10)
            .send()
            .map_err(|e| anyhow::anyhow!("Failed to fetch latest release: {e}"))?
            .json()
            .map_err(|e| anyhow::anyhow!("Failed to parse release: {e}"))?]
    };

    if releases.is_empty() {
        bail!("No releases found");
    }
    Ok(releases.remove(0))
}

fn find_linux_asset(assets: &[GhAsset]) -> Result<&GhAsset> {
    assets
        .iter()
        .find(|a| {
            let n = a.name.to_lowercase();
            n.contains("linux") && !n.contains("musl") && !n.contains("aarch")
        })
        .or_else(|| assets.first())
        .ok_or_else(|| anyhow::anyhow!("No suitable asset found"))
}

fn download_and_replace(url: &str) -> Result<()> {
    let exe = std::env::current_exe()?;
    let mut new_path = exe.clone();
    let mut new_ext = std::ffi::OsString::from("new");
    if let Some(ext) = new_path.extension() {
        new_ext.push(".");
        new_ext.push(ext);
    }
    new_path.set_extension(new_ext);

    download_to_path(url, &new_path)?;

    self_replace::self_replace(&new_path)?;
    let _ = std::fs::remove_file(&new_path);
    Ok(())
}

fn download_to_path(url: &str, dest: &std::path::Path) -> Result<()> {
    let response = minreq::get(url)
        .with_header("User-Agent", "clashtui")
        .with_timeout(300)
        .send()
        .map_err(|e| anyhow::anyhow!("Download failed: {e}"))?;

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(dest, response.as_bytes())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(dest, perms)?;
    }
    Ok(())
}
