use ratatui::style::Color;
use std::sync::OnceLock;

static GLOBAL_THEME: OnceLock<Theme> = OnceLock::new();

// #[derive(serde::Serialize, serde::Deserialize)]
pub struct Theme {
    pub popup_block_fg: Color,

    pub list_block_fg_fouced: Color,
    pub list_block_fg_unfouced: Color,
    pub list_hl_bg_fouced: Color,

    pub tab_txt_fg: Color,
    pub tab_hl_fg: Color,

    pub statusbar_txt_fg: Color,
}

impl Theme {
    pub fn load(_ph: Option<&std::path::PathBuf>) -> Result<(), String> {
        let _ = GLOBAL_THEME.set(Self::default());
        Ok(())
    }
    pub fn get() -> &'static Self {
        GLOBAL_THEME.get_or_init(Self::default)
    }
    //pub fn load(ph: Option<&std::path::PathBuf>) -> Result<Self, String> {
    //    ph.map_or_else(
    //        || Ok(Self::default()),
    //        |ph| {
    //            std::fs::File::open(ph)
    //                .map_err(|e| e.to_string())
    //                .and_then(|f| serde_yaml::from_reader(f).map_err(|e| e.to_string()))
    //        },
    //    )
    //}
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            popup_block_fg: Color::Rgb(0, 85, 119),

            list_block_fg_fouced: Color::Rgb(0, 204, 153),
            list_block_fg_unfouced: Color::Rgb(220, 220, 220),
            list_hl_bg_fouced: Color::Rgb(64, 64, 64),

            tab_txt_fg: Color::Rgb(0, 153, 153),
            tab_hl_fg: Color::Rgb(46, 204, 113),

            statusbar_txt_fg: Color::Rgb(20, 122, 122),
        }
    }
}
