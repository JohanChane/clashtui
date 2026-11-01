pub mod content;
pub mod handlers;
pub mod render;
pub mod tree;

use super::dev::*;
use content::Proxies;

newtype_tab!(ProxiesTab(Tab<Proxies>), Proxies);

key_map!(
    Proxies,
    Action,
    DefaultBindings::new()
        .with_keys([
            (KeyCode::Up, Action::MoveUp),
            (KeyCode::Down, Action::MoveDown),
            (KeyCode::Char('k'), Action::MoveUp),
            (KeyCode::Char('j'), Action::MoveDown),
            (KeyCode::Char('h'), Action::Parent),
            (KeyCode::Char('l'), Action::Expand),
            (KeyCode::Enter, Action::Select),
            (KeyCode::Char('/'), Action::Search),
            (KeyCode::Char('t'), Action::TestDelay),
            (KeyCode::Char('r'), Action::Refresh),
            (KeyCode::Char('f'), Action::GroupSelect),
            (KeyCode::Char('F'), Action::FzfFind),
            (KeyCode::Char('G'), Action::GoBottom),
        ])
        .with_chords(KeyCode::Char('g'), [(KeyCode::Char('g'), Action::GoTop)])
        .with_chords(
            KeyCode::Char('s'),
            [
                (KeyCode::Char('n'), Action::Sort(Sort::ByName)),
                (KeyCode::Char('d'), Action::Sort(Sort::ByDelay)),
                (KeyCode::Char('r'), Action::Sort(Sort::Reset)),
            ]
        )
        .with_chords(
            KeyCode::Char('S'),
            [
                (KeyCode::Char('n'), Action::Sort(Sort::GlobalByName)),
                (KeyCode::Char('d'), Action::Sort(Sort::GlobalByDelay)),
                (KeyCode::Char('r'), Action::Sort(Sort::GlobalReset)),
            ]
        )
        .with_chords(
            KeyCode::Char('a'),
            [
                (KeyCode::Char('f'), Action::CollapseAll),
                (KeyCode::Char('e'), Action::ExpandAll),
                (KeyCode::Char('t'), Action::TestAllDelay),
            ]
        )
);

#[derive_aliases::derive(..Action, Debug)]
pub(in crate::tui) enum Action {
    MoveUp,
    MoveDown,
    Parent,
    Expand,
    Select,
    GoTop,
    GoBottom,
    CollapseAll,
    ExpandAll,
    TestDelay,
    TestAllDelay,
    Refresh,
    Search,
    FzfFind,
    GroupSelect,
    #[serde(untagged)]
    Sort(Sort),
}

#[derive_aliases::derive(..Action, Debug)]
pub(in crate::tui) enum Sort {
    #[serde(rename = "ResetSort")]
    Reset,
    #[serde(rename = "SortByName")]
    ByName,
    #[serde(rename = "SortByDelay")]
    ByDelay,
    #[serde(rename = "GlobalResetSort")]
    GlobalReset,
    #[serde(rename = "GlobalSortByName")]
    GlobalByName,
    #[serde(rename = "GlobalSortByDelay")]
    GlobalByDelay,
}

impl Document for Action {
    fn get_doc(&self) -> &'static str {
        use crate::tui::key::consts::*;
        match self {
            Self::MoveUp => MOVE_UP,
            Self::MoveDown => MOVE_DOWN,
            Self::Parent => "Go to parent",
            Self::Expand => "Expand/Jump to group",
            Self::Select => "Select",
            Self::GoTop => GO_TOP,
            Self::GoBottom => GO_BOTTOM,
            Self::CollapseAll => "Collapse all",
            Self::ExpandAll => "Expand all",
            Self::TestDelay => "Test delay",
            Self::TestAllDelay => "Test all delay",
            Self::Refresh => "Refresh",
            Self::Search => FILTER,
            Self::FzfFind => "Find proxy",
            Self::GroupSelect => "Group select",
            Self::Sort(Sort::ByDelay) => "Sort by delay",
            Self::Sort(Sort::ByName) => "Sort by name",
            Self::Sort(Sort::Reset) => "Reset sort",
            Self::Sort(Sort::GlobalByDelay) => "Global sort by delay",
            Self::Sort(Sort::GlobalByName) => "Global sort by name",
            Self::Sort(Sort::GlobalReset) => "Global reset sort",
        }
    }
}
