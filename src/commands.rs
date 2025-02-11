use crate::utils::consts::VERSION;

mod complete;
mod handler;
mod widgets;

pub(crate) use handler::handle_cli;
pub(crate) use widgets::{Confirm, Select};

pub(crate) struct PackedArgs(ArgCommand);

/// collect args and parse
///
/// also handle `--generate_shell_completion` and `migrate`
/// and then exit early by returning `Err`
///
/// ### Errors
///
/// This function will return an error only if `--generate_shell_completion`
/// is provided. The content will written to StdOut
pub(crate) fn parse_args() -> Result<(Option<PackedArgs>, u8), ()> {
    use clap::Parser;
    let CliCmds {
        command,
        generate_shell_completion,
        config_dir,
        verbose,
    } = CliCmds::parse();
    if let Some(generate_shell_completion) = generate_shell_completion {
        complete::gen_complete(generate_shell_completion);
        eprint!("generate completion success");
        return Err(());
    }
    if let Some(config_dir) = config_dir {
        super::DataDir::set(config_dir);
    }
    if let Some(ArgCommand::Migrate { version }) = &command {
        if let Err(e) = match version {
            #[cfg(feature = "migration_v0_2_3")]
            OldVersion::V0_2_3 => crate::utils::config::v0_2_3::migrate(),
            #[cfg(not(any(feature = "migration_v0_2_3")))]
            OldVersion::NotSupported => {
                Err::<(), anyhow::Error>(anyhow::anyhow!("unsupported version"))
            }
        } {
            eprintln!("migrate error: {e}");
        }
        return Err(());
    }
    // Pack the content to avoid visibility warning
    Ok((command.map(PackedArgs), verbose))
}

/// Mihomo (Clash.Meta) TUI Client
///
/// A tool for mihomo, also support other Clash API
#[derive(clap::Parser)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[command(version=VERSION, about, after_help="If you have any question or suggestion, please visit https://github.com/JohanChane/clashtui")]
pub(crate) struct CliCmds {
    #[command(subcommand)]
    command: Option<ArgCommand>,
    // `clashtui --generate-shell-completion` in fact get `Some(None)`
    // while `clashtui` get `None`
    /// generate shell completion
    #[arg(long, require_equals=true, num_args=0..=1, default_missing_value=None)]
    generate_shell_completion: Option<Option<clap_complete::Shell>>,
    /// specify the ClashTUI config directory
    #[arg(long, require_equals = true)]
    pub config_dir: Option<std::path::PathBuf>,
    /// increase log level, default is Warning
    #[arg(long, short, action=clap::ArgAction::Count)]
    verbose: u8,
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
    /// set proxy mode,
    /// leave empty to get current mode
    Mode {
        #[command(subcommand)]
        mode: Option<ModeCommand>,
    },
    /// check for update
    CheckUpdate {
        /// download the first item (filtered by arch)
        #[arg(long, short = 'y')]
        without_ask: bool,
        /// check ci/alpha release instead
        #[arg(long, short = 'c')]
        check_ci: bool,
        /// target to check
        #[command(subcommand)]
        target: Target,
    },
    /// migrate config from old version
    Migrate {
        /// the old version
        #[command(subcommand)]
        version: OldVersion,
    },
}

#[derive(Debug, clap::Subcommand)]
enum OldVersion {
    #[cfg(feature = "migration_v0_2_3")]
    /// v0.2.3
    V0_2_3,
    #[cfg(not(any(feature = "migration_v0_2_3")))]
    /// not supported any version
    NotSupported,
}

#[derive(Debug, clap::Subcommand)]
enum Target {
    /// check for ClashTUI
    Clashtui,
    /// check for Mihomo
    Mihomo,
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
        /// update profile with proxyprovider removed
        #[arg(long)]
        without_proxyprovider: bool,
    },
    /// select profile
    Select {
        /// the profile name
        #[arg(short, long)]
        name: Option<String>,
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

impl From<ModeCommand> for crate::clash::webapi::Mode {
    fn from(value: ModeCommand) -> Self {
        use crate::clash::webapi::Mode;
        match value {
            ModeCommand::Rule => Mode::Rule,
            ModeCommand::Direct => Mode::Direct,
            ModeCommand::Global => Mode::Global,
        }
    }
}
