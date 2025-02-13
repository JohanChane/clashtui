#[cfg(feature = "customized-theme")]
use crate::utils::consts::THEME_PATH;
#[cfg(feature = "customized-theme")]
use std::sync::Once;

use ratatui::style::{Color, Modifier, Style};
use std::sync::RwLock;

static GLOBAL_THEME: RwLock<Theme> = RwLock::new(Theme::new());
#[cfg(feature = "customized-theme")]
static RELOAD_ON_GET: Once = Once::new();

#[cfg_attr(
    feature = "customized-theme",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Default)]
pub struct Theme {
    pub popup: Popup,
    pub input: Input,
    pub list: List,
    pub bars: Bars,
    pub connection_tab: ConnctionTab,
    pub profile_tab: ProfileTab,
    pub browser: Browser,
}

impl Theme {
    pub fn get() -> std::sync::RwLockReadGuard<'static, Theme> {
        #[cfg(feature = "customized-theme")]
        if RELOAD_ON_GET.is_completed() {
            if let Ok(theme) = || -> anyhow::Result<Self> {
                Ok(serde_yml::from_reader(std::fs::File::open(
                    THEME_PATH.as_path(),
                )?)?)
            }() {
                let mut lock = GLOBAL_THEME.write().unwrap();
                let _ = std::mem::replace(&mut *lock, theme);
            }
        }
        GLOBAL_THEME.read().unwrap()
    }
    #[cfg(not(feature = "customized-theme"))]
    // load default theme, done at [RwLock] init
    // so do nothing here
    pub fn load() {}
    #[cfg(feature = "customized-theme")]
    pub fn load() {
        let set = |theme: Theme| {
            let mut lock = GLOBAL_THEME.write().unwrap();
            let _ = std::mem::replace(&mut *lock, theme);
        };
        match || -> anyhow::Result<Self> {
            Ok(serde_yml::from_reader(std::fs::File::open(
                THEME_PATH.as_path(),
            )?)?)
        }() {
            Ok(theme) => set(theme),
            Err(err) => {
                log::warn!("Failed to load theme: {err}");
                log::warn!("Loading default theme");
                log::warn!("Recreate theme file at {}", THEME_PATH.display());
                let theme = Self::default();
                if let Err(e) = || -> anyhow::Result<()> {
                    Ok(serde_yml::to_writer(
                        std::fs::File::create(THEME_PATH.as_path())?,
                        &theme,
                    )?)
                }() {
                    log::error!("Failed to create theme file at {}", THEME_PATH.display());
                    log::error!("due to {e}")
                }
                set(theme)
            }
        }
    }
    #[cfg(feature = "customized-theme")]
    pub fn enable_realtime() {
        RELOAD_ON_GET.call_once(|| {});
    }
}

impl Theme {
    const fn new() -> Self {
        Self {
            popup: Popup::new(),
            input: Input::new(),
            list: List::new(),
            bars: Bars::new(),
            connection_tab: ConnctionTab::new(),
            profile_tab: ProfileTab::new(),
            browser: Browser::new(),
        }
    }
}

macro_rules! expanding {
    ($name:ident, $($value:ident: $exprs:expr,)+) => {
#[cfg_attr(
    feature = "customized-theme",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct $name{
    $(pub $value: Style,)+
}
impl $name {
    const fn new() -> Self {
        Self {
            $($value: $exprs,)+
        }
    }
}
impl Default for $name {
    fn default() -> Self {
        Self::new()
    }
}
    };
}

expanding!(Bars,
    tabbar_text: Style::new().fg(Color::Rgb(0, 153, 153)),
    tabbar_highlight: Style::new().fg(Color::Rgb(46, 204, 113)),
    statusbar_text: Style::new().fg(Color::Rgb(20, 122, 122)),
);

expanding!(Popup,
    block: Style::new().fg(Color::Rgb(0, 102, 102)),
    text: Style::new().fg(Color::Rgb(46, 204, 113)),
);

expanding!(Browser,
    char_highlight: Style::new().fg(Color::Magenta),
    dir: Style::new().fg(Color::LightCyan),
    file: Style::new(),
);

expanding!(Input,
    selected: Style::new().fg(Color::Yellow),
    unselected: Style::new().fg(Color::Reset),
);

expanding!(List,
    block_selected: Style::new().fg(Color::Rgb(0, 204, 153)),
    block_unselected: Style::new().fg(Color::Rgb(220, 220, 220)),
    highlight: Style::new()
        .bg(Color::Rgb(64, 64, 64))
        .add_modifier(Modifier::BOLD),
    unhighlight: Style::new(),
);

expanding!(ProfileTab,
    update_interval: Style::new().fg(Color::Red),
);

expanding!(ConnctionTab,
    table_static: Style::new().fg(Color::Gray).bg(Color::DarkGray),
);
