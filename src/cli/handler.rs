use crate::{
    backend::{BackEnd, Profile, ServiceOp},
    consts::{PKG_NAME, PKG_VERSION},
};

use super::*;

pub fn handle_cli(command: PackedArgs, backend: BackEnd) -> anyhow::Result<()> {
    // let var: Option<bool> = std::env::var("CLASHTUI_")
    //     .ok()
    //     .and_then(|s| s.parse().ok());
    match command.0 {
        ArgCommand::Profile { command } => {
            let backend = backend.as_profile();
            match command {
                ProfileCommand::Update {
                    all,
                    name,
                    with_proxy,
                    without_proxyprovider,
                } => {
                    let iter: Box<dyn Iterator<Item = Profile>> = if all {
                        Box::new(backend.get_all_profiles().into_iter())
                    } else if let Some(name) = name {
                        Box::new(backend.get_profile(name).into_iter())
                    } else {
                        eprintln!("No profile selected!");
                        Box::new(std::iter::empty())
                    };
                    iter.inspect(|s| println!("### Profile: {}", s.name))
                        .filter_map(|v| {
                            backend
                                .update_profile(v, with_proxy, without_proxyprovider)
                                .map_err(|e| println!("- Error! {e}"))
                                .ok()
                        })
                        .flatten()
                        .for_each(|s| println!("- {s}"));
                    backend.select_profile(backend.get_current_profile())?;
                    println!("Done");
                    Ok(())
                }
                ProfileCommand::Select { name: Some(name) } => {
                    let Some(pf) = backend.get_profile(&name) else {
                        anyhow::bail!("Not found in database!");
                    };
                    if let Err(e) = backend.select_profile(pf) {
                        eprint!("Cannot select {name} due to {e}");
                        return Err(e);
                    };
                    println!("Done");
                    Ok(())
                }
                ProfileCommand::Select { name: None } => {
                    println!("Current Profile: {}", backend.get_current_profile().name);
                    Ok(())
                }
                ProfileCommand::List { name_only } => {
                    let mut pfs = backend.get_all_profiles();
                    pfs.sort_by(|a, b| a.name.cmp(&b.name));
                    pfs.into_iter()
                        .map(|pf| {
                            if name_only {
                                pf.name
                            } else {
                                format!(
                                    "{} : {}",
                                    pf.name,
                                    pf.dtype.get_domain().as_deref().unwrap_or("Unknown")
                                )
                            }
                        })
                        .for_each(|pf| println!("{}", pf));
                    println!("Done");
                    Ok(())
                }
            }
        }
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        ArgCommand::Service { command } => {
            let op = match command {
                ServiceCommand::Restart { soft: true } => ServiceOp::RestartClashCore,
                ServiceCommand::Restart { soft: false } => ServiceOp::RestartClashService,
                ServiceCommand::Stop => ServiceOp::StopClashService,
            };
            let res = backend.clash_srv_ctl(op)?;
            println!("{res}");
            Ok(())
        }
        ArgCommand::Mode { mode } => {
            let state = backend.update_state(None, mode.map(Into::into))?;
            println!("{state}");
            Ok(())
        }
        ArgCommand::Update {
            ci: check_ci,
            target,
        } => {
            use crate::utils::self_update::Request;

            let current_version = target.current_version(&backend);
            let path = target.path(&backend)?;

            let Some(info) = match target {
                Target::Clashtui => Request::s_clashtui(check_ci)
                    .get_info()
                    .map(|info| info.rename(PKG_NAME).filter_asserts()),
                Target::Mihomo => Request::s_mihomo(check_ci)
                    .get_info()
                    .map(|info| info.rename("Mihomo").filter_asserts()),
            }
            .map_err(|e| anyhow::anyhow!("failed to fetch github release due to {e}"))?
            .check(&current_version, check_ci) else {
                println!("Up to date");
                println!("current version is {}", current_version);
                return Ok(());
            };

            println!("\n{}", info.as_info(current_version));
            let Some(asset) = Select::default()
                .append_start_prompt("Available asserts:")
                .append_items(info.assets.iter())
                .set_end_prompt("Which you want to download:")
                .interact()?
            else {
                println!("Abort");
                return Ok(());
            };

            println!();
            println!(
                "Download start for [{}]({})",
                asset.name, asset.browser_download_url
            );
            let new_path = {
                let mut new_path = path;
                let mut new_ext = std::ffi::OsString::from("new");
                if let Some(ext) = new_path.extension() {
                    new_ext.push(".");
                    new_ext.push(ext);
                }
                new_path.set_extension(new_ext);
                new_path
            };
            println!("To {}", new_path.display());
            println!();

            asset.download(&new_path)?;

            match self_replace::self_replace(&new_path) {
                Ok(()) => {
                    let _ = std::fs::remove_file(new_path);
                }
                Err(e) => {
                    anyhow::bail!(
                        "Failed to replace self but download is finished. You may have to do it manually\nError due to {e}"
                    )
                }
            };
            println!("Done");
            Ok(())
        }
        ArgCommand::Migrate { .. } => unreachable!(),
    }
}

impl Target {
    pub fn current_version(&self, backend: &BackEnd) -> String {
        match self {
            Target::Clashtui => PKG_VERSION.to_owned(),
            Target::Mihomo => backend
                .get_clash_version()
                .ok()
                .unwrap_or("v0.0.0".to_owned()),
        }
    }
    pub fn path(&self, backend: &BackEnd) -> anyhow::Result<std::path::PathBuf> {
        match self {
            Target::Clashtui => Ok(std::env::current_exe()?),
            Target::Mihomo => {
                let _path = backend.get_mihomo_bin_path();
                if _path.is_empty() {
                    Ok(std::env::current_dir()?.join("mihomo"))
                } else {
                    Ok(std::path::PathBuf::from(_path))
                }
            }
        }
    }
}
