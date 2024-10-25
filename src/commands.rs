mod complete;
mod handler;
mod interactive;
use crate::utils::consts::VERSION;

pub(crate) use handler::handle_cli;
pub use interactive::{Confirm, Select};

pub(crate) struct PackedArgs(ArgCommand);

pub(crate) fn parse_args() -> Result<Option<PackedArgs>, ()> {
    use clap::Parser;
    use complete::gen_complete;
    let CliCmds {
        command,
        generate_shell_completion,
    } = CliCmds::parse();
    if let Some(generate_shell_completion) = generate_shell_completion {
        gen_complete(generate_shell_completion);
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
    // `clashtui --generate-shell-completion` in fact get `Some(None)`
    // while `clashtui` get `None`
    /// generate shell completion
    #[arg(long, require_equals=true, num_args=0..=1, default_missing_value=None)]
    generate_shell_completion: Option<Option<clap_complete::Shell>>,
}

#[derive(clap::Subcommand)]
#[cfg_attr(debug_assertions, derive(Debug))]
enum ArgCommand {
    /// profile related
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    /// service related
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
    /// proxy mode related
    Mode {
        #[command(subcommand)]
        mode: ModeCommand,
    },
    /// check for update
    CheckUpdate {
        /// download the first item (filtered by arch)
        #[arg(long, short = 'y')]
        without_ask: bool,
        /// check ci/alpha release instead
        #[arg(long, short = 'c')]
        check_ci: bool,
    },
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

#[derive(clap::Subcommand)]
#[cfg_attr(debug_assertions, derive(Debug))]
enum ProfileCommand {
    /// update the selected profile or all
    Update {
        /// update all profiles,
        /// this will also update config clash is using,
        /// while --name does not
        #[arg(short, long)]
        all: bool,
        /// the profile name
        #[arg(short, long)]
        name: Option<String>,
        /// update profile with proxy
        #[arg(long)]
        with_proxy: Option<bool>,
    },
    /// select profile
    Select {
        /// the profile name
        #[arg(short, long)]
        name: String,
    },
    /// list all profile
    List {
        /// without domain hint
        #[arg(long)]
        name_only: bool,
    },
}

#[derive(clap::Subcommand)]
#[cfg_attr(debug_assertions, derive(Debug))]
enum ServiceCommand {
    /// start/restart service, can be soft
    Restart {
        /// restart by send POST request to mihomo
        #[arg(short, long)]
        soft: bool,
    },
    /// stop service
    Stop,
}
