mod complete;
mod handler;
use crate::utils::consts::VERSION;

pub(crate) use handler::handle_cli;

pub(crate) struct PackedArgs(ArgCommand);

pub(crate) fn parse_args() -> Result<Option<PackedArgs>, ()> {
    use clap::Parser;
    use complete::gen_complete;
    let CliCmds {
        command,
        generate_shell_completion,
        shell,
    } = CliCmds::parse();
    if generate_shell_completion {
        gen_complete(shell);
        return Err(());
    }
    Ok(command.map(PackedArgs))
}

/// Mihomo (Clash.Meta) Control Client
///
/// A tool for mihomo
#[derive(clap::Parser)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[command(version=VERSION, about, long_about)]
pub(crate) struct CliCmds {
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

#[derive(clap::Subcommand)]
#[cfg_attr(debug_assertions, derive(Debug))]
enum ArgCommand {
    Profile(Profile),
    #[cfg(any(target_os = "linux", target_os = "windows"))]
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
    /// update profile with proxy
    #[arg(long)]
    with_proxy: bool,
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
