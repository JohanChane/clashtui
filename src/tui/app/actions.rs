//! Global actions, tab targets and their default bindings.

use crate::tui::key::{DefaultBindings, Document, Key};
use crossterm::event::{KeyCode, KeyModifiers};

#[derive_aliases::derive(..Action, Debug)]
pub enum AppAction {
    TabNext,
    TabPrev,
    Quit,
    Help,
    #[serde(untagged)]
    Tab(TabTarget),
    #[serde(untagged)]
    Global(GlobalAction),
}

#[derive_aliases::derive(..Action, Debug)]
#[repr(u8)]
pub enum TabTarget {
    #[serde(rename = "TabStatus")]
    Status,
    #[serde(rename = "TabFile")]
    File,
    #[serde(rename = "TabProxies")]
    Proxies,
    #[serde(rename = "TabConnections")]
    Connections,
    #[serde(rename = "TabLogs")]
    Logs,
    #[serde(rename = "TabSettings")]
    Settings,
    #[serde(rename = "TabCoreSrvCtl")]
    CoreSrvCtl,
}

impl TabTarget {
    pub(in crate::tui) const ALL: [Self; 7] = [
        Self::Status,
        Self::File,
        Self::Proxies,
        Self::Connections,
        Self::Logs,
        Self::Settings,
        Self::CoreSrvCtl,
    ];
}

#[derive_aliases::derive(..Action, Debug)]
pub enum GlobalAction {
    OpenCoreDataDir,
    OpenCoreInstallDir,
    RestartCoreService,
    CloseAllConnections,
}

impl GlobalAction {
    pub(super) fn execute(self) -> anyhow::Result<()> {
        use crate::{
            config::{CONFIG, CoreType},
            functions::{command, restful::connection},
        };
        match self {
            Self::OpenCoreDataDir => {
                let dir = crate::config::core_data_dir(CONFIG.core_type());
                command::open_dir(&dir)
            }
            Self::OpenCoreInstallDir => {
                let dir = match CONFIG.core_type() {
                    CoreType::Mihomo => &CONFIG.cfg_file.mihomo.core.config_dir,
                    CoreType::Singbox => &CONFIG.cfg_file.singbox.core.config_dir,
                };
                let dir = std::path::Path::new(dir);
                command::open_dir(dir.parent().unwrap_or(dir))
            }
            Self::RestartCoreService => {
                let output = command::restart_service()?;
                anyhow::ensure!(!output.starts_with("Error"), "{output}");
                Ok(())
            }
            Self::CloseAllConnections => Ok(connection::terminate_all_connections()?),
        }
    }
}

key_map!(App, AppAction, defaults());

fn defaults() -> DefaultBindings<AppAction> {
    DefaultBindings::new()
        .with_keys(
            TabTarget::ALL
                .into_iter()
                .enumerate()
                .map(|(index, tab)| {
                    (
                        KeyCode::Char((b'1' + index as u8) as char),
                        AppAction::Tab(tab),
                    )
                })
                .chain(APP_KEYS),
        )
        .with_chords(
            Key {
                code: KeyCode::Char('g'),
                modifiers: KeyModifiers::CONTROL,
            },
            [
                (
                    KeyCode::Char('c'),
                    AppAction::Global(GlobalAction::OpenCoreDataDir),
                ),
                (
                    KeyCode::Char('m'),
                    AppAction::Global(GlobalAction::OpenCoreInstallDir),
                ),
                (
                    KeyCode::Char('f'),
                    AppAction::Global(GlobalAction::RestartCoreService),
                ),
                (
                    KeyCode::Char('t'),
                    AppAction::Global(GlobalAction::CloseAllConnections),
                ),
            ],
        )
}

const APP_KEYS: [(KeyCode, AppAction); 4] = [
    (KeyCode::Tab, AppAction::TabNext),
    (KeyCode::BackTab, AppAction::TabPrev),
    (KeyCode::Char('q'), AppAction::Quit),
    (KeyCode::Char('?'), AppAction::Help),
];
impl Document for AppAction {
    fn get_doc(&self) -> &'static str {
        match self {
            Self::Tab(tab) => match tab {
                TabTarget::Status => "Switch to Status",
                TabTarget::File => "Switch to File",
                TabTarget::Proxies => "Switch to Proxies",
                TabTarget::Connections => "Switch to Connections",
                TabTarget::Logs => "Switch to Logs",
                TabTarget::Settings => "Switch to Settings",
                TabTarget::CoreSrvCtl => "Switch to CoreSrvCtl",
            },
            Self::TabNext => "Tab Next",
            Self::TabPrev => "Tab Previous",
            Self::Quit => "Quit",
            Self::Help => "Help",
            Self::Global(action) => match action {
                GlobalAction::OpenCoreDataDir => "Open core data dir",
                GlobalAction::OpenCoreInstallDir => "Open core install dir",
                GlobalAction::RestartCoreService => "Start/restart core service",
                GlobalAction::CloseAllConnections => "Close all connections",
            },
        }
    }
}
