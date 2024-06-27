use crate::utils::VERSION;
/// Mihomo (Clash.Meta) Control Client
///
/// A tool for mihomo
#[derive(clap::Parser)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[command(version=VERSION, about, long_about)]
pub struct CliCmds {
    #[command(subcommand)]
    command: Option<ArgCommand>,
    /// generate completion for current shell
    #[arg(long)]
    generate_shell_completion: bool,
    /// specify target shell
    ///
    /// avaliable when --generate-shell-completion is set
    #[arg(long)]
    shell: Option<clap_complete::Shell>,
}

pub struct PackedArgCommand(ArgCommand);
#[derive(clap::Subcommand)]
#[cfg_attr(debug_assertions, derive(Debug))]
enum ArgCommand {
    Profile(Profile),
    Service(Service),
    Mode(Mode),
}

/// proxy mode related
#[derive(clap::Args)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct Mode {
    #[command(subcommand)]
    command: ModeCommand,
}
#[derive(clap::Subcommand)]
#[cfg_attr(debug_assertions, derive(Debug))]
enum ModeCommand {
    /// rule
    Rule,
    /// direct
    Direct,
    /// global
    Global,
}

/// profile related
#[derive(clap::Args)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct Profile {
    #[command(subcommand)]
    command: ProfileCommand,
}
#[derive(clap::Subcommand)]
#[cfg_attr(debug_assertions, derive(Debug))]
enum ProfileCommand {
    Update(ProfileUpdate),
    Select(ProfileSelect),
}
/// update the selected profile or all
#[derive(clap::Args)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct ProfileUpdate {
    /// update all profiles
    #[arg(short, long)]
    all: bool,
    /// the profile name
    #[arg(short, long)]
    name: Option<String>,
}
/// select profile
#[derive(clap::Args)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct ProfileSelect {
    /// the profile name
    #[arg(short, long)]
    name: String,
}

/// service related
#[derive(clap::Args)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct Service {
    #[command(subcommand)]
    command: ServiceCommand,
}
#[derive(clap::Subcommand)]
#[cfg_attr(debug_assertions, derive(Debug))]
enum ServiceCommand {
    /// start/restart service, can be soft
    Restart(ServiceRestart),
    /// stop service
    Stop,
}

#[derive(clap::Args)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct ServiceRestart {
    /// restart by send POST request to mihomo
    #[arg(short, long)]
    soft: bool,
}

pub fn parse_args() -> Result<Option<PackedArgCommand>, ()> {
    use clap::Parser;
    let CliCmds {
        command,
        generate_shell_completion,
        shell,
    } = CliCmds::parse();
    if generate_shell_completion {
        gen_complete(shell);
        return Err(());
    }
    Ok(command.map(PackedArgCommand))
}

pub fn handle_cli(
    command: PackedArgCommand,
    backend: crate::utils::ClashBackend,
) -> std::io::Result<String> {
    use crate::utils::api;
    let PackedArgCommand(command) = command;
    match command {
        ArgCommand::Profile(Profile { command }) => match command {
            ProfileCommand::Update(ProfileUpdate { all, name }) => {
                if all {
                    backend
                        .get_profile_names()
                        .unwrap()
                        .into_iter()
                        .inspect(|s| println!("\nProfile: {s}"))
                        .filter_map(|v| {
                            backend
                                .update_profile(&v, false)
                                .map_err(|e| println!("- Error! {e}"))
                                .ok()
                        })
                        .flatten()
                        .for_each(|s| println!("- {s}"));
                    if let Err(e) = backend.select_profile(&backend.cfg.current_profile.borrow()) {
                        eprintln!("Select Profile: {e}")
                    };
                    Ok("Done".to_string())
                } else if let Some(_name) = name {
                    todo!()
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Not providing Profile",
                    ))
                }
            }
            ProfileCommand::Select(ProfileSelect { name: _name }) => {
                todo!()
            }
        },
        ArgCommand::Service(Service { command }) => match command {
            ServiceCommand::Restart(ServiceRestart { soft }) => {
                if soft {
                    backend
                        .restart_clash()
                        .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))
                } else {
                    backend.clash_srv_ctl(crate::utils::ClashSrvOp::StartClashService)
                }
            }
            ServiceCommand::Stop => {
                backend.clash_srv_ctl(crate::utils::ClashSrvOp::StopClashService)
            }
        },
        #[cfg(target_os = "linux")]
        ArgCommand::Mode(Mode { command }) => match command {
            ModeCommand::Rule => Ok(backend
                .update_state(None, Some(api::Mode::Rule.into()))
                .to_string()),
            ModeCommand::Direct => Ok(backend
                .update_state(None, Some(api::Mode::Direct.into()))
                .to_string()),
            ModeCommand::Global => Ok(backend
                .update_state(None, Some(api::Mode::Global.into()))
                .to_string()),
        },
        #[cfg(target_os = "windows")]
        ArgCommand::Mode(Mode { command }) => match command {
            ModeCommand::Rule => Ok(backend
                .update_state(None, Some(api::Mode::Rule.into()), None)
                .to_string()),
            ModeCommand::Direct => Ok(backend
                .update_state(None, Some(api::Mode::Direct.into()), None)
                .to_string()),
            ModeCommand::Global => Ok(backend
                .update_state(None, Some(api::Mode::Global.into()), None)
                .to_string()),
        },
    }
}
pub fn gen_complete(shell: Option<clap_complete::Shell>) {
    use clap::CommandFactory;
    let gen = if let Some(gen) = shell {
        eprintln!("Target Shell: {gen}");
        gen
    } else {
        match clap_complete::shells::Shell::from_env() {
            Some(gen) => {
                eprintln!("Current Shell: {gen}");
                gen
            }
            None => {
                eprintln!("Unable to determine what shell this is");
                eprintln!("Try use --shell to specify");
                return;
            }
        }
    };
    clap_complete::generate(
        gen,
        &mut CliCmds::command(),
        "clashtui",
        &mut std::io::stdout(),
    )
}
