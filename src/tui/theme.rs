use ratatui::style::{Color, Modifier, Style};
use std::sync::OnceLock;

/// using [OnceLock],
/// because the path to theme file is wanted
static GLOBAL_THEME: OnceLock<Theme> = OnceLock::new();

#[cfg_attr(
    feature = "customized-theme",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Default)]
pub struct Theme {
    pub popup: Popup,
    pub input: Input,
    pub connection_tab: ConnctionTab,
    pub list: List,
    pub bars: Bars,
    pub profile_tab: ProfileTab,
}

impl Theme {
    const NOT_LOADED: &str = "Global Theme should be loaded before used";
    const ALREADY_LOADED: &str = "Global Theme should be loaded only once";

    pub fn get() -> &'static Self {
        GLOBAL_THEME.get().expect(Self::NOT_LOADED)
    }
    #[cfg(not(feature = "customized-theme"))]
    pub fn load(_pb: Option<&std::path::Path>) -> anyhow::Result<()> {
        GLOBAL_THEME
            .set(Self::default())
            .map_err(|_| anyhow::anyhow!(Self::ALREADY_LOADED))
    }
    #[cfg(feature = "customized-theme")]
    pub fn load(ph: Option<&std::path::Path>) -> anyhow::Result<()> {
        GLOBAL_THEME
            .set(match ph {
                Some(pb) => serde_yml::from_reader(std::fs::File::open(pb)?)?,
                None => Self::default(),
            })
            .map_err(|_| anyhow::anyhow!(Self::ALREADY_LOADED))
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
impl Default for $name {
    fn default() -> Self {
        Self {
            $($value: $exprs,)+
        }
    }
}
    };
}

expanding!(Bars,
    tabbar_text: Style::default().fg(Color::Rgb(0, 153, 153)),
    tabbar_highlight: Style::default().fg(Color::Rgb(46, 204, 113)),
    statusbar_text: Style::default().fg(Color::Rgb(20, 122, 122)),
);

expanding!(Popup,
    block: Style::default().fg(Color::Rgb(0, 102, 102)),
    text: Style::default().fg(Color::Rgb(46, 204, 113)),
);

expanding!(Input,
    selected: Style::default().fg(Color::Yellow),
    unselected: Style::default().fg(Color::Reset),
);

expanding!(List,
    block_selected: Style::default().fg(Color::Rgb(0, 204, 153)),
    block_unselected: Style::default().fg(Color::Rgb(220, 220, 220)),
    highlight: Style::default()
        .bg(Color::Rgb(64, 64, 64))
        .add_modifier(Modifier::BOLD),
);

expanding!(ProfileTab,
    update_interval: Style::default().fg(Color::Red),
);

expanding!(ConnctionTab,
    table_static: Style::default().fg(Color::Gray).bg(Color::DarkGray),
);
