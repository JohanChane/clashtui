#[cfg(feature = "customized-theme")]
use crate::config::theme_path;
#[cfg(feature = "customized-theme")]
use std::sync::Once;

use ratatui::style::{Color, Modifier, Style};
use std::sync::RwLock;

static GLOBAL_THEME: RwLock<Theme> = RwLock::new(Theme::new());
#[cfg(feature = "customized-theme")]
static RELOAD_ON_GET: Once = Once::new();

#[cfg_attr(feature = "customized-theme", derive(serde::Deserialize))]
pub struct Theme {
    pub popup: Popup,
    pub tab: Tab,
    pub bars: Bars,
    pub connection_tab: ConnectionTab,
    pub profile_tab: ProfileTab,
    pub browser: Browser,
}

impl Theme {
    pub fn get() -> std::sync::RwLockReadGuard<'static, Theme> {
        #[cfg(feature = "customized-theme")]
        if RELOAD_ON_GET.is_completed() {
            if let Ok(theme) = || -> anyhow::Result<Self> {
                let file = std::fs::File::open(theme_path())?;
                let mut value: serde_yml::Mapping = serde_yml::from_reader(file)?;
                let core_type = crate::config::CONFIG.core_type();
                let core_key = match core_type {
                    crate::config::CoreType::Mihomo => "mihomo",
                    crate::config::CoreType::Singbox => "sing-box",
                };
                if let Some(core_section) = value.remove(core_key) {
                    if let serde_yml::Value::Mapping(core_map) = core_section {
                        value.remove(match core_type {
                            crate::config::CoreType::Mihomo => "sing-box",
                            crate::config::CoreType::Singbox => "mihomo",
                        });
                        for (key, val) in core_map {
                            value.insert(key, val);
                        }
                    }
                } else {
                    value.remove("mihomo");
                    value.remove("sing-box");
                }
                Ok(serde_yml::from_value(serde_yml::Value::Mapping(value))?)
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
        let path = theme_path();
        match || -> anyhow::Result<Self> {
            let file = std::fs::File::open(&path)?;
            let mut value: serde_yml::Mapping = serde_yml::from_reader(file)?;
            let core_type = crate::config::CONFIG.core_type();
            // Extract core-specific section
            let core_key = match core_type {
                crate::config::CoreType::Mihomo => "mihomo",
                crate::config::CoreType::Singbox => "sing-box",
            };
            if let Some(core_section) = value.remove(core_key) {
                if let serde_yml::Value::Mapping(core_map) = core_section {
                    // Remove sing-box/mihomo (the other one)
                    value.remove(match core_type {
                        crate::config::CoreType::Mihomo => "sing-box",
                        crate::config::CoreType::Singbox => "mihomo",
                    });
                    // Merge: core-specific values override common
                    for (key, val) in core_map {
                        value.insert(key, val);
                    }
                }
            } else {
                // Remove both core-specific sections if no core match found
                value.remove("mihomo");
                value.remove("sing-box");
            }
            Ok(serde_yml::from_value(serde_yml::Value::Mapping(value))?)
        }() {
            Ok(theme) => set(theme),
            Err(err) => {
                log::warn!("Failed to load theme: {err}");
                log::warn!("Loading default theme");
                let theme = Self::default();
                // log::warn!("Recreate theme file at {}", path.display());
                // if let Err(e) = || -> anyhow::Result<()> {
                //     Ok(serde_yml::to_writer(std::fs::File::create(&path)?, &theme)?)
                // }() {
                //     log::error!("Failed to create theme file at {}", path.display());
                //     log::error!("due to {e}")
                // }
                set(theme)
            }
        }
    }
    #[cfg(feature = "customized-theme")]
    pub fn enable_realtime() {
        RELOAD_ON_GET.call_once(|| {});
    }
    const fn new() -> Self {
        Self {
            popup: Popup::new(),
            tab: Tab::new(),
            bars: Bars::new(),
            connection_tab: ConnectionTab::new(),
            profile_tab: ProfileTab::new(),
            browser: Browser::new(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! expanding {
    ($name:ident, $($value:ident: $exprs:expr,)+) => {
#[cfg_attr(
    feature = "customized-theme",
    derive(serde::Deserialize)
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
    };
}

expanding!(Bars,
    tabbar_text: Style::new().fg(Color::Rgb(0, 153, 153)),
    tabbar_highlight: Style::new().fg(Color::Rgb(46, 204, 113)),
    block: Style::new().fg(Color::Gray),
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

expanding!(Tab,
    tab_focused: Style::new().fg(Color::Rgb(0, 204, 153)),
    dualtab_unfocused: Style::new().fg(Color::Rgb(220, 220, 220)),
    item_highlighted: Style::new()
        .bg(Color::Rgb(64, 64, 64))
        .add_modifier(Modifier::BOLD),
    item_unhighlighted: Style::new(),
);

expanding!(ProfileTab,
    update_interval: Style::new().fg(Color::Red),
);

expanding!(ConnectionTab,
    table_static: Style::new().fg(Color::Gray).bg(Color::DarkGray),
);
