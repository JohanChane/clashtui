use ratatui::style::Color;
use std::sync::OnceLock;

/// using [OnceLock],
/// because the path to theme file is wanted
static GLOBAL_THEME: OnceLock<Theme> = OnceLock::new();

#[cfg_attr(
    feature = "customized-theme",
    derive(serde::Serialize, serde::Deserialize)
)]
#[allow(unused)]
pub struct Theme {
    pub popup_block_fg: Color,
    pub popup_text_fg: Color,

    pub input_text_selected_fg: Color,
    pub input_text_unselected_fg: Color,

    pub table_static_bg: Color,

    pub list_block_fouced_fg: Color,
    pub list_block_unfouced_fg: Color,
    pub list_hl_bg_fouced: Color,

    pub tabbar_text_fg: Color,
    pub tabbar_hl_fg: Color,

    pub statusbar_text_fg: Color,

    pub profile_update_interval_fg: Color,
}

impl Theme {
    const NOT_LOADED: &str = "Global Theme should be loaded before used";
    const ALREADY_LOADED: &str = "Global Theme should be loaded only once";
    #[cfg(not(feature = "customized-theme"))]
    pub fn load(_pb: Option<&std::path::PathBuf>) -> anyhow::Result<()> {
        GLOBAL_THEME
            .set(Self::default())
            .map_err(|_| anyhow::anyhow!(Self::ALREADY_LOADED))
    }
    pub fn get() -> &'static Self {
        GLOBAL_THEME.get().expect(Self::NOT_LOADED)
    }
    #[cfg(feature = "customized-theme")]
    pub fn load(ph: Option<&std::path::PathBuf>) -> anyhow::Result<()> {
        GLOBAL_THEME
            .set(match ph {
                Some(pb) => serde_yml::from_reader(std::fs::File::open(pb)?)?,
                None => Self::default(),
            })
            .map_err(|_| anyhow::anyhow!(Self::ALREADY_LOADED))
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            popup_block_fg: Color::Rgb(0, 102, 102),
            popup_text_fg: Color::Rgb(46, 204, 113),

            input_text_selected_fg: Color::Yellow,
            input_text_unselected_fg: Color::Reset,

            table_static_bg: Color::DarkGray,

            list_block_fouced_fg: Color::Rgb(0, 204, 153),
            list_block_unfouced_fg: Color::Rgb(220, 220, 220),
            list_hl_bg_fouced: Color::Rgb(64, 64, 64),

            tabbar_text_fg: Color::Rgb(0, 153, 153),
            tabbar_hl_fg: Color::Rgb(46, 204, 113),

            statusbar_text_fg: Color::Rgb(20, 122, 122),

            profile_update_interval_fg: Color::Red,
        }
    }
}
