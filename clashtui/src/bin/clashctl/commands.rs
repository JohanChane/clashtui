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
    generate_shell_completion:bool,
}
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
    Rule,
    Direct,
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
    Restart(ServiceRestart),
    Stop,
}

/// restart, can be soft
#[derive(clap::Args)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct ServiceRestart {
    /// restart by send POST request to mihomo
    #[arg(short, long)]
    soft: bool,
}

pub struct Clinfo {
    // judge usage via flag
    pub profile: Option<String>,
    pub flags: enumflags2::BitFlags<Flag>,
}
#[derive(Clone, Copy)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[enumflags2::bitflags]
#[repr(u16)]
pub enum Flag {
    UpdateAll,
    Update,
    Select,
    Restart,
    RestartSoft,
    Stop,
    Rule,
    Direct,
    Global,
    Tui,
}
pub fn handle_cli_args(backend: clashtui::utils::ClashBackend) -> Option<std::io::Result<String>> {
    parse_args().ok().map(|v| handle_flags(v, backend))
}
pub fn parse_args() -> Result<Clinfo, ()> {
    let mut infos = Clinfo {
        profile: None,
        flags: enumflags2::BitFlags::empty(),
    };
    use clap::Parser;
    let CliCmds { command,generate_shell_completion } = CliCmds::parse();
    println!(">{command:?}");
    if generate_shell_completion{
        gen_complete();
        return Err(());
    }
    match command {
        Some(command) => match command {
            ArgCommand::Profile(Profile { command }) => match command {
                ProfileCommand::Update(ProfileUpdate { all, name }) => {
                    if all {
                        infos.flags.insert(Flag::UpdateAll)
                    } else if let Some(n) = name {
                        infos.flags.insert(Flag::Update);
                        infos.profile.replace(n);
                    } else {
                        return Err(());
                    }
                }
                ProfileCommand::Select(ProfileSelect { name }) => {
                    infos.flags.insert(Flag::Select);
                    infos.profile.replace(name);
                }
            },
            ArgCommand::Service(Service { command }) => match command {
                ServiceCommand::Restart(ServiceRestart { soft }) => infos.flags.insert(if soft {
                    Flag::RestartSoft
                } else {
                    Flag::Restart
                }),
                ServiceCommand::Stop => infos.flags.insert(Flag::Stop),
            },
            ArgCommand::Mode(Mode { command }) => match command {
                ModeCommand::Rule => infos.flags.insert(Flag::Rule),
                ModeCommand::Direct => infos.flags.insert(Flag::Direct),
                ModeCommand::Global => infos.flags.insert(Flag::Global),
            },
        },
        None => infos.flags.insert(Flag::Tui),
    }
    Ok(infos)
}

pub fn handle_flags(
    infos: Clinfo,
    backend: clashtui::utils::ClashBackend,
) -> std::io::Result<String> {
    let Clinfo { profile, flags } = infos;
    if flags.contains(Flag::Direct) {
        Ok(backend
            .update_state(None, Some(api::Mode::Direct.into()))
            .to_string())
    } else if flags.contains(Flag::Rule) {
        Ok(backend
            .update_state(None, Some(api::Mode::Rule.into()))
            .to_string())
    } else if flags.contains(Flag::Global) {
        Ok(backend
            .update_state(None, Some(api::Mode::Global.into()))
            .to_string())
    } else if flags.contains(Flag::Restart) {
        backend.clash_srv_ctl(clashtui::utils::ClashSrvOp::StartClashService)
    } else if flags.contains(Flag::RestartSoft) {
        backend.restart_clash()
    } else if flags.contains(Flag::Stop) {
        backend.clash_srv_ctl(clashtui::utils::ClashSrvOp::StopClashService)
    } else if flags.contains(Flag::Update) {
        if let Some(_name) = profile {
            todo!()
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Not providing Profile",
            ))
        }
    } else if flags.contains(Flag::UpdateAll) {
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
        Ok("Done".to_string())
    } else if flags.contains(Flag::Select) {
        todo!()
    } else {
        unreachable!()
    }
    // ignore Tui and Version
}
pub fn gen_complete() {
    use clap::CommandFactory;
    match clap_complete::shells::Shell::from_env() {
        Some(gen) => clap_complete::generate(gen, &mut CliCmds::command(), "clashcli", &mut std::io::stdout()),
        None => eprintln!("Unable to determine what shell this is"),
    }
}