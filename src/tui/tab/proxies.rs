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
        (KeyCode::Up, Key::MoveUp, "Move up"),
        (KeyCode::Down, Key::MoveDown, "Move down"),
        (KeyCode::Char('k'), Key::MoveUp, "Move up"),
        (KeyCode::Char('j'), Key::MoveDown, "Move down"),
        (KeyCode::Char('h'), Key::Parent, "Go to parent"),
        (KeyCode::Char('l'), Key::Expand, "Expand/Jump to group"),
        (KeyCode::Enter, Key::Select, "Select"),
        // (
        //     [KeyCode::Char('g'), KeyCode::Char('g')],
        //     Key::GoTop,
        //     "Go to top"
        // ),
        // ([KeyCode::Char('G')], Key::GoBottom, "Go to bottom"),
        (KeyCode::Char('/'), Key::Search, "Search/Filter"),
        // (
        //     [KeyCode::Char('s'), KeyCode::Char('n')],
        //     Key::SortByName,
        //     "Sort by name"
        // ),
        // (
        //     [KeyCode::Char('s'), KeyCode::Char('d')],
        //     Key::SortByDelay,
        //     "Sort by delay"
        // ),
        // (
        //     [KeyCode::Char('s'), KeyCode::Char('r')],
        //     Key::ResetSort,
        //     "Reset sort"
        // ),
        // (
        //     [KeyCode::Char('S'), KeyCode::Char('n')],
        //     Key::GlobalSortByName,
        //     "Global sort by name"
        // ),
        // (
        //     [KeyCode::Char('S'), KeyCode::Char('d')],
        //     Key::GlobalSortByDelay,
        //     "Global sort by delay"
        // ),
        // (
        //     [KeyCode::Char('S'), KeyCode::Char('r')],
        //     Key::GlobalResetSort,
        //     "Global reset sort"
        // ),
        // (
        //     [KeyCode::Char('a'), KeyCode::Char('f')],
        //     Key::CollapseAll,
        //     "Collapse all"
        // ),
        // (
        //     [KeyCode::Char('a'), KeyCode::Char('e')],
        //     Key::ExpandAll,
        //     "Expand all"
        // ),
        (KeyCode::Char('t'), Key::TestDelay, "Test delay"),
        // (
        //     [KeyCode::Char('a'), KeyCode::Char('t')],
        //     Key::TestAllDelay,
        //     "Test all delay"
        // ),
        (KeyCode::Char('r'), Key::Refresh, "Refresh"),
        (KeyCode::Char('f'), Key::GroupSelect, "Group select"),
        (KeyCode::Char('F'), Key::FzfFind, "Find proxy"),
    ]
);

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
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
    SortByName,
    SortByDelay,
    ResetSort,
    GlobalSortByName,
    GlobalSortByDelay,
    GlobalResetSort,
    TestDelay,
    TestAllDelay,
    Refresh,
    Search,
    FzfFind,
    GroupSelect,
}
