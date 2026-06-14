pub mod content;
pub mod handlers;
pub mod render;
pub mod tree;

use super::dev::*;
pub use content::Proxies;

newtype_tab!(ProxiesTab(Tab<Proxies>));

key_map!(
    Key,
    [
        (KeyCode::Up, Key::MoveUp),
        (KeyCode::Down, Key::MoveDown),
        (KeyCode::Char('k'), Key::MoveUp),
        (KeyCode::Char('j'), Key::MoveDown),
        (KeyCode::Char('h'), Key::Parent),
        (KeyCode::Char('l'), Key::Expand),
        (KeyCode::Enter, Key::Select),
        // (
        //     [KeyCode::Char('g'), KeyCode::Char('g')],
        //     Key::GoTop
        // ),
        // ([KeyCode::Char('G')], Key::GoBottom),
        (KeyCode::Char('/'), Key::Search),
        // (
        //     [KeyCode::Char('s'), KeyCode::Char('n')],
        //     Key::SortByName
        // ),
        // (
        //     [KeyCode::Char('s'), KeyCode::Char('d')],
        //     Key::SortByDelay
        // ),
        // (
        //     [KeyCode::Char('s'), KeyCode::Char('r')],
        //     Key::ResetSort
        // ),
        // (
        //     [KeyCode::Char('S'), KeyCode::Char('n')],
        //     Key::GlobalSortByName
        // ),
        // (
        //     [KeyCode::Char('S'), KeyCode::Char('d')],
        //     Key::GlobalSortByDelay
        // ),
        // (
        //     [KeyCode::Char('S'), KeyCode::Char('r')],
        //     Key::GlobalResetSort
        // ),
        // (
        //     [KeyCode::Char('a'), KeyCode::Char('f')],
        //     Key::CollapseAll
        // ),
        // (
        //     [KeyCode::Char('a'), KeyCode::Char('e')],
        //     Key::ExpandAll
        // ),
        (KeyCode::Char('t'), Key::TestDelay),
        // (
        //     [KeyCode::Char('a'), KeyCode::Char('t')],
        //     Key::TestAllDelay
        // ),
        (KeyCode::Char('r'), Key::Refresh),
        (KeyCode::Char('f'), Key::GroupSelect),
        (KeyCode::Char('F'), Key::FzfFind),
    ]
);

#[derive_aliases::derive(..Key)]
pub enum Key {
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
    Sort(Sort),
}

#[derive_aliases::derive(..Action)]
enum Sort {
    Reset,
    ByName,
    ByDelay,
    GlobalReset,
    GlobalByName,
    GlobalByDelay,
}

impl AsStaticStr for Key {
    fn as_static_str(&self) -> &'static str {
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
