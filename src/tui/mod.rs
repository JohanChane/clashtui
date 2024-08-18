mod statusbar;
pub(crate) mod symbols;
mod tabbar;
mod tabs;
mod utils;
use crate::ui::utils::tools;
use crate::ui::{widgets, EventState, Theme, Visibility};

use statusbar::StatusBar;
use tabbar::TabBar;

mod app;
mod impl_app;
pub use app::App;
