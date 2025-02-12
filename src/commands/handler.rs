use crate::clash::webapi::Mode as cMode;

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
            } => {
                if all {
                    backend
                        .get_all_profiles()
                        .into_iter()
                        .inspect(|s| println!("Profile: {}", s.name))
                        .filter_map(|v| {
                            backend
                                .update_profile(v, with_proxy)
                                .map_err(|e| println!("- Error! {e}"))
                                .ok()
                        })
                        .flatten()
                        .for_each(|s| println!("- {s}"));
                    if let Err(e) = backend.select_profile(backend.get_current_profile()) {
                        eprintln!("Select Profile: {e}")
                    };
                } else if let Some(name) = name {
                    println!("Profile: {name}");
                    let pf = if let Some(v) = backend.get_profile(name) {
                        v
                    } else {
                        anyhow::bail!("Not found in database!");
                    };
                    match backend.update_profile(pf, with_proxy) {
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
                let pf = if let Some(v) = backend.get_profile(&name) {
                    v
                } else {
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
                    Ok(backend.clash_srv_ctl(ServiceOp::StartClashService)?)
                }
            }
            ServiceCommand::Stop => Ok(backend.clash_srv_ctl(ServiceOp::StopClashService)?),
        },
        ArgCommand::Mode { mode } => match mode {
            ModeCommand::Rule => Ok(backend
                .update_state(None, Some(cMode::Rule.into()))?
                .to_string()),
            ModeCommand::Direct => Ok(backend
                .update_state(None, Some(cMode::Direct.into()))?
                .to_string()),
            ModeCommand::Global => Ok(backend
                .update_state(None, Some(cMode::Global.into()))?
                .to_string()),
        },
        ArgCommand::CheckUpdate {
            without_ask,
            check_ci,
        } => {
            for (info, current_version) in backend
                .check_update(check_ci)
                .map_err(|e| anyhow::anyhow!("failed to fetch github release due to {e}"))?
            {
                println!("\n{}", info.as_info(current_version));
                if !without_ask {
                    if !Confirm::default()
                        .append_prompt("Do you want to download one now?")
                        .interact()?
                    {
                        continue;
                    }
                    let asset = Select::default()
                        .append_start_prompt("Avaliable asserts:")
                        .append_end_prompt("Type the num:")
                        .append_items(info.assets.iter())
                        .interact()?;
                    println!("\nDownload start for {}", asset);
                    let path =
                        backend.download_to_file(&asset.name, &asset.browser_download_url)?;
                    println!("\nDownloaded to {}", path.display());
                } else if let Some(asset) = info.assets.first() {
                    println!("\nDownload start for {}", asset);
                    let path =
                        backend.download_to_file(&asset.name, &asset.browser_download_url)?;
                    println!("\nDownloaded to {}", path.display());
                }
            }
            Ok("Done".to_owned())
        }
    }
}
