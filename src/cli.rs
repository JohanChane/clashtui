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
        #[cfg(feature = "customized-theme")]
        load_theme_realtime,
    } = CliCmds::parse();

    if let Some(generate_shell_completion) = generate_shell_completion {
        complete::gen_complete(generate_shell_completion);
        eprint!("generate completion success");
        return Err(());
    }

    if let Some(config_dir) = config_dir {
        super::DataDir::set(config_dir);
    }

    #[cfg(feature = "customized-theme")]
    if load_theme_realtime {
        crate::tui::Theme::enable_realtime();
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
#[command(
    version = crate::consts::PKG_VERSION,
    long_version = crate::consts::FULL_VERSION,
    about,
    after_help=concat!("If you have any question or suggestion, please visit ", env!("CARGO_PKG_REPOSITORY"))
)]
pub(crate) struct CliCmds {
    #[command(subcommand)]
    command: Option<ArgCommand>,
    #[arg(long, require_equals=true, num_args=0..=1, default_missing_value=None)]
    // `clashtui --generate-shell-completion` in fact get `Some(None)`
    // while `clashtui` get `None`
    /// generate shell completion
    generate_shell_completion: Option<Option<clap_complete::Shell>>,
    #[arg(long, require_equals = true)]
    /// specify the ClashTUI config directory
    pub config_dir: Option<std::path::PathBuf>,
    #[arg(long, short, action=clap::ArgAction::Count)]
    /// increase log level, default is Warning
    verbose: u8,
    #[cfg(feature = "customized-theme")]
    #[arg(long)]
    /// allow theme change without restart
    load_theme_realtime: bool,
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
    Update {
        /// check ci/alpha release instead
        #[arg(long, short = 'c')]
        ci: bool,
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
    /// not support any version
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
        with_proxy: bool,
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

use crate::backend::Mode;
impl From<ModeCommand> for Mode {
    fn from(value: ModeCommand) -> Self {
        match value {
            ModeCommand::Rule => Mode::Rule,
            ModeCommand::Direct => Mode::Direct,
            ModeCommand::Global => Mode::Global,
        }
    }
}
