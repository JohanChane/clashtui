use crate::tui::binding::{Binding, Keymap, Scope};
use crossterm::event::KeyCode;
use std::sync::OnceLock;

static KEYMAP: OnceLock<Keymap<GlobalAction>> = OnceLock::new();

pub fn init(keymap: Keymap<GlobalAction>) {
    KEYMAP.set(keymap).expect("Keymap init twice");
}

pub fn get() -> &'static Keymap<GlobalAction> {
    KEYMAP.get_or_init(|| {
        Keymap::merge(&default_bindings(), &[], Scope::Global)
            .expect("default bindings must be conflict-free")
    })
}

pub fn default_bindings() -> Vec<Binding<GlobalAction>> {
    use GlobalAction::*;

    fn quick_map(code: KeyCode) -> crate::tui::Key {
        crate::tui::Key {
            code,
            shift: matches!(code, KeyCode::Char(c) if c.is_ascii_uppercase()),
            ctrl: false,
            alt: false,
            super_: false,
        }
    }

    fn key_str(s: &str) -> crate::tui::Key {
        use std::str::FromStr;
        crate::tui::Key::from_str(s).expect("invalid key string")
    }

    vec![
        Binding {
            on: vec![quick_map(KeyCode::Char('1'))],
            exec: GotoStatus,
            desc: Some("Go to Status".into()),
        },
        Binding {
            on: vec![quick_map(KeyCode::Char('2'))],
            exec: GotoFile,
            desc: Some("Go to File".into()),
        },
        Binding {
            on: vec![quick_map(KeyCode::Char('3'))],
            exec: GotoProxies,
            desc: Some("Go to Proxies".into()),
        },
        Binding {
            on: vec![quick_map(KeyCode::Char('4'))],
            exec: GotoConnections,
            desc: Some("Go to Connections".into()),
        },
        Binding {
            on: vec![quick_map(KeyCode::Char('5'))],
            exec: GotoLogs,
            desc: Some("Go to Logs".into()),
        },
        Binding {
            on: vec![quick_map(KeyCode::Char('6'))],
            exec: GotoSettings,
            desc: Some("Go to Settings".into()),
        },
        Binding {
            on: vec![quick_map(KeyCode::Char('7'))],
            exec: GotoService,
            desc: Some("Go to Service".into()),
        },
        Binding {
            on: vec![quick_map(KeyCode::Tab)],
            exec: CycleTab,
            desc: Some("Cycle tabs".into()),
        },
        Binding {
            on: vec![key_str("?")],
            exec: ToggleHelp,
            desc: Some("Toggle help".into()),
        },
        Binding {
            on: vec![key_str("q")],
            exec: Quit,
            desc: Some("Quit".into()),
        },
        Binding {
            on: vec![key_str("<C-c>")],
            exec: Quit,
            desc: Some("Quit".into()),
        },
        Binding {
            on: vec![key_str("<C-g>"), quick_map(KeyCode::Char('c'))],
            exec: OpenDataDir,
            desc: Some("Open core data dir".into()),
        },
        Binding {
            on: vec![key_str("<C-g>"), quick_map(KeyCode::Char('m'))],
            exec: OpenInstallDir,
            desc: Some("Open core install dir".into()),
        },
        Binding {
            on: vec![key_str("<C-g>"), quick_map(KeyCode::Char('f'))],
            exec: RestartService,
            desc: Some("Start/Restart core service".into()),
        },
        Binding {
            on: vec![key_str("<C-g>"), quick_map(KeyCode::Char('t'))],
            exec: TerminateAll,
            desc: Some("Close all connections".into()),
        },
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GlobalAction {
    GotoStatus,
    GotoFile,
    GotoProxies,
    GotoConnections,
    GotoLogs,
    GotoSettings,
    GotoService,
    CycleTab,
    ToggleHelp,
    Quit,
    OpenDataDir,
    OpenInstallDir,
    RestartService,
    TerminateAll,
}
