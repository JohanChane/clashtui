use crate::backend::{BackEnd, Profile, ServiceOp};

use super::*;

pub fn handle_cli(command: PackedArgs, backend: BackEnd) -> anyhow::Result<String> {
    // let var: Option<bool> = std::env::var("CLASHTUI_")
    //     .ok()
    //     .and_then(|s| s.parse().ok());
    let PackedArgs(command) = command;
    match command {
        ArgCommand::Profile { command } => match command {
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
                Ok("Done".to_owned())
            }
            ProfileCommand::Select { name } => {
                let Some(pf) = backend.get_profile(&name) else {
                    anyhow::bail!("Not found in database!");
                };
                if let Err(e) = backend.select_profile(pf) {
                    eprint!("Cannot select {name} due to {e}");
                    return Err(e);
                };
                Ok("Done".to_owned())
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
                                pf.dtype.get_domain().as_ref().map_or("Unknown", |v| v)
                            )
                        }
                    })
                    .for_each(|pf| println!("{}", pf));
                Ok("Done".to_owned())
            }
        },
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        ArgCommand::Service { command } => match command {
            ServiceCommand::Restart { soft } => {
                if soft {
                    backend.restart_clash().map_err(|e| anyhow::anyhow!(e))
                } else {
                    backend
                        .clash_srv_ctl(ServiceOp::StartClashService)
                        .map_err(|e| anyhow::anyhow!(e))
                }
            }
            ServiceCommand::Stop => Ok(backend.clash_srv_ctl(ServiceOp::StopClashService)?),
        },
        ArgCommand::Mode { mode } => Ok(backend.update_state(None, Some(mode.into()))?.to_string()),
        ArgCommand::CheckUpdate {
            without_ask,
            check_ci,
            target,
        } => {
            use crate::utils::self_update::Request;
            macro_rules! expand {
                ($request:expr, $name:literal, $version:expr) => {
                    $request
                        .get_info()
                        .map_err(|e| anyhow::anyhow!("failed to fetch github release due to {e}"))?
                        .check($version, check_ci)
                        .map(|info| info.rename($name).filter_asserts())
                };
                () => {};
            }
            let (info, current_version, mut path) = match target {
                Target::Clashtui => {
                    let current_version = VERSION.to_owned();
                    let path = std::env::current_exe()?;
                    let info = expand!(Request::s_clashtui(check_ci), "ClashTUI", VERSION);
                    (info, current_version, path)
                }
                Target::Mihomo => {
                    let current_version = backend
                        .get_clash_version()
                        .ok()
                        .unwrap_or("v0.0.0".to_owned());
                    let path = std::path::PathBuf::from(&backend.get_config().basic.clash_bin_pth);
                    let info = expand!(
                        Request::s_mihomo(check_ci),
                        "Mihomo",
                        backend
                            .get_clash_version()
                            .ok()
                            .as_deref()
                            .unwrap_or("v0.0.0")
                    );
                    (info, current_version, path)
                }
            };
            if let Some(info) = info {
                println!("\n{}", info.as_info(current_version));
                if let Some(asset) = if !without_ask {
                    if !Confirm::default()
                        .append_prompt("Do you want to download now?")
                        .interact()?
                    {
                        return Ok("Abort".to_owned());
                    }
                    println!();
                    Select::default()
                        .append_start_prompt("Avaliable asserts:")
                        .set_end_prompt("Type the num")
                        .append_items(info.assets.iter())
                        .interact()?
                } else {
                    info.assets.first()
                } {
                    println!();
                    println!(
                        "Download start for {} {}",
                        asset.name, asset.browser_download_url
                    );
                    // add '.new' to the file name if the file already exists
                    // else, use the original name
                    if path.file_name() == Some(std::ffi::OsStr::new(&asset.name)) {
                        path.set_file_name(format!("{}.new", asset.name));
                    } else {
                        path.set_file_name(&asset.name);
                    }
                    asset.download(&path)?;
                    println!("Downloaded to {}", path.display());
                    println!();
                }
            }
            Ok("Done".to_owned())
        }
    }
}
