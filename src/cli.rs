mod utils;
mod handler;
mod widgets;

pub use handler::handle_cli;
pub use widgets::{Confirm, Select};

#[derive(clap::Parser)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[command(
    version = utils::PKG_VERSION,
    long_version = utils::FULL_VERSION,
    about,
    after_help=concat!("If you have any question or suggestion, please visit ", env!("CARGO_PKG_REPOSITORY"))
)]
/// Mihomo (Clash.Meta) TUI Client
///
/// A tool for mihomo, also support other Clash API
pub struct Cmds {
    #[command(subcommand)]
    pub(crate) command: Option<ArgCommand>,
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
    pub verbose: u8,
    #[cfg(feature = "customized-theme")]
    #[arg(long)]
    /// allow theme change without restart
    load_theme_realtime: bool,
}

/// Parse args, also handle envs(like `CLASHTUI_CONFIG_DIR`)
pub fn from_env() -> Cmds {
    use clap::Parser;
    let instance = Cmds::parse();

    Cmds {
        config_dir: instance
            .config_dir
            .or(std::env::var_os("CLASHTUI_CONFIG_DIR").map(std::path::PathBuf::from)),
        ..instance
    }
}

impl Cmds {
    /// `--generate_shell_completion` and `migrate`
    pub fn handle_early_exit(self) -> Option<Self> {
        if let Some(generate_shell_completion) = self.generate_shell_completion {
            utils::gen_complete(generate_shell_completion);
            eprint!("generate completion success");
            return None;
        }

        if let Some(ArgCommand::Migrate { version }) = self.command {
            if let Err(e) = match version {
                #[cfg(feature = "migration_v0_3_0")]
                OldVersion::V0_3_0 => crate::utils::config::v0_3_0::migrate(),
                #[cfg(not(any(feature = "migration_v0_3_0")))]
                OldVersion::NotSupported => {
                    Err::<(), anyhow::Error>(anyhow::anyhow!("unsupported version"))
                }
            } {
                eprintln!("migrate error: {e}");
            }
            return None;
        }

        #[cfg(feature = "customized-theme")]
        if self.load_theme_realtime {
            crate::tui::Theme::enable_realtime();
        }

        Some(self)
    }
}

#[derive(clap::Subcommand)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub(crate) enum ArgCommand {
    /// profile related
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
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
    #[cfg(feature = "migration_v0_3_0")]
    /// v0.3.0
    V0_3_0,
    #[cfg(not(any(feature = "migration_v0_3_0")))]
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

#[derive(Clone, clap::ValueEnum)]
#[cfg_attr(debug_assertions, derive(Debug))]
enum ProfileTypeFilter {
    /// file-based profiles
    File,
    /// URL-based profiles
    Url,
    /// template-based profiles
    Template,
    /// sing-box profiles
    Singbox,
}

impl ProfileTypeFilter {
    fn matches(&self, dtype: &crate::config::database::ProfileType) -> bool {
        match (self, dtype) {
            (ProfileTypeFilter::File, crate::config::database::ProfileType::File) => true,
            (ProfileTypeFilter::Url, crate::config::database::ProfileType::Url(_)) => true,
            (ProfileTypeFilter::Template, crate::config::database::ProfileType::Template { .. }) => true,
            (ProfileTypeFilter::Singbox, crate::config::database::ProfileType::Singbox) => true,
            _ => false,
        }
    }
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
        /// filter by profile type
        #[arg(long, value_enum)]
        r#type: Option<ProfileTypeFilter>,
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
        /// filter by profile type
        #[arg(long, value_enum)]
        r#type: Option<ProfileTypeFilter>,
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

// use crate::backend::Mode;
// impl From<ModeCommand> for Mode {
//     fn from(value: ModeCommand) -> Self {
//         match value {
//             ModeCommand::Rule => Mode::Rule,
//             ModeCommand::Direct => Mode::Direct,
//             ModeCommand::Global => Mode::Global,
//         }
//     }
// }
