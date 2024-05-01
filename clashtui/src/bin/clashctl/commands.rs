/// Mihomo (Clash.Meta) Control Client
///
/// A tool for mihomo
#[derive(argh::FromArgs)]
struct CliEnv {
    /// print version information and exit
    #[argh(switch, short = 'v')]
    pub version: bool,
    #[argh(subcommand)]
    command: Option<ArgCommand>,
}
#[derive(argh::FromArgs)]
#[argh(subcommand)]
enum ArgCommand {
    Profile(Profile),
    Service(Service),
    Mode(Mode),
}

/// proxy mode related
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "mode")]
struct Mode {
    #[argh(subcommand)]
    command: ModeCommand,
}
#[derive(argh::FromArgs)]
#[argh(subcommand)]
enum ModeCommand {
    Rule(ModeRule),
    Direct(ModeDirect),
    Global(ModeGlobal),
}
/// Rule
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "rule")]
struct ModeRule {}
/// Direct
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "direct")]
struct ModeDirect {}
/// Global
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "global")]
struct ModeGlobal {}

/// profile related
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "profile")]
struct Profile {
    #[argh(subcommand)]
    command: ProfileCommand,
}
#[derive(argh::FromArgs)]
#[argh(subcommand)]
enum ProfileCommand {
    Update(ProfileUpdate),
    Select(ProfileSelect),
}
/// update the selected profile or all
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "update")]
struct ProfileUpdate {
    /// update all profiles
    #[argh(switch, short = 'a')]
    all: bool,
    /// the profile name
    #[argh(option, short = 'n')]
    name: Option<String>,
}
/// select profile
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "select")]
struct ProfileSelect {
    /// the profile name
    #[argh(positional, short = 'n')]
    name: String,
}

/// service related
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "service")]
struct Service {
    #[argh(subcommand)]
    command: ServiceCommand,
}
#[derive(argh::FromArgs)]
#[argh(subcommand)]
enum ServiceCommand {
    Restart(ServiceRestart),
    Stop(ServiceStop),
}

/// stop
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "stop")]
struct ServiceStop {}
/// restart, can be soft
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "restart")]
struct ServiceRestart {
    /// restart by send POST request to mihomo
    #[argh(switch, short = 's')]
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
    Version,
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
    let CliEnv { version, command } = argh::from_env();
    if version {
        infos.flags.insert(Flag::Version);
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
                ServiceCommand::Stop(_) => infos.flags.insert(Flag::Stop),
            },
            ArgCommand::Mode(Mode { command }) => match command {
                ModeCommand::Rule(_) => infos.flags.insert(Flag::Rule),
                ModeCommand::Direct(_) => infos.flags.insert(Flag::Direct),
                ModeCommand::Global(_) => infos.flags.insert(Flag::Global),
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
