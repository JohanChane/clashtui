use crate::utils::{BackEnd, ServiceOp};

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
                if all {
                    backend
                        .get_all_profiles()
                        .into_iter()
                        .inspect(|s| println!("Profile: {}", s.name))
                        .filter_map(|v| {
                            backend
                                .update_profile(v, with_proxy, without_proxyprovider)
                                .map_err(|e| println!("- Error! {e}"))
                                .ok()
                        })
                        .flatten()
                        .for_each(|s| println!("- {s}"));
                    if let Err(e) = backend.select_profile(backend.get_current_profile()) {
                        eprintln!("Select Profile: {e}")
                    };
                } else if let Some(name) = name {
                    println!("Target Profile: {name}");
                    let Some(pf) = backend.get_profile(name) else {
                        anyhow::bail!("Not found in database!");
                    };
                    match backend.update_profile(pf, with_proxy, without_proxyprovider) {
                        Ok(v) => {
                            v.into_iter().for_each(|s| println!("- {s}"));
                        }
                        Err(e) => {
                            println!("- Error! {e}")
                        }
                    }
                } else {
                    anyhow::bail!("Not providing Profile");
                }
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
        } => {
            use crate::utils::self_update::{download_to_file, CheckUpdate};
            for (info, current_version) in backend
                .check_update(check_ci)
                .map_err(|e| anyhow::anyhow!("failed to fetch github release due to {e}"))?
            {
                println!("\n{}", info.as_info(current_version));
                if let Some(asset) = if !without_ask {
                    if !Confirm::default()
                        .append_prompt("Do you want to download now?")
                        .interact()?
                    {
                        continue;
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
                    let path = std::env::current_dir()?.join(&asset.name);
                    download_to_file(&path, &asset.browser_download_url)?;
                    println!("Downloaded to {}", path.display());
                    println!();
                }
            }
            Ok("Done".to_owned())
        }
    }
}
