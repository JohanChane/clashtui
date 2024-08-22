use clashtui::{backend::ServiceOp, webapi::Mode as cMode};

use crate::utils::BackEnd;

use super::*;

pub fn handle_cli(command: PackedArgs, backend: BackEnd) -> anyhow::Result<String> {
    let PackedArgs(command) = command;
    match command {
        ArgCommand::Profile(Profile { command }) => match command {
            ProfileCommand::Update(ProfileUpdate {
                all,
                name,
                with_proxy,
            }) => {
                if all {
                    backend
                        .get_all_profiles()
                        .into_iter()
                        .inspect(|s| println!("\nProfile: {}", s.name))
                        .filter_map(|v| {
                            backend
                                .update_profile(&v, false, Some(with_proxy))
                                .map_err(|e| println!("- Error! {e}"))
                                .ok()
                        })
                        .flatten()
                        .for_each(|s| println!("- {s}"));
                    if let Err(e) = backend.select_profile(backend.get_current_profile()) {
                        eprintln!("Select Profile: {e}")
                    };
                    Ok("Done".to_string())
                } else if let Some(_name) = name {
                    println!("Update Profile:{_name}");
                    todo!()
                } else {
                    anyhow::bail!("Not providing Profile");
                }
            }
            ProfileCommand::Select(ProfileSelect { name: _name }) => {
                todo!()
            }
        },
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        ArgCommand::Service(Service { command }) => match command {
            ServiceCommand::Restart(ServiceRestart { soft }) => {
                if soft {
                    backend.restart_clash().map_err(|e| anyhow::anyhow!(e))
                } else {
                    Ok(backend.clash_srv_ctl(ServiceOp::StartClashService)?)
                }
            }
            ServiceCommand::Stop => Ok(backend.clash_srv_ctl(ServiceOp::StopClashService)?),
        },
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        ArgCommand::Mode(Mode { command }) => match command {
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
        #[cfg(target_os = "windows")]
        ArgCommand::Mode(Mode { command }) => match command {
            ModeCommand::Rule => Ok(backend
                .update_state(None, Some(cMode::Rule.into()), None)?
                .to_string()),
            ModeCommand::Direct => Ok(backend
                .update_state(None, Some(cMode::Direct.into()), None)?
                .to_string()),
            ModeCommand::Global => Ok(backend
                .update_state(None, Some(cMode::Global.into()), None)?
                .to_string()),
        },
    }
}
