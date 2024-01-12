use ratatui::style::Color;
use std::{fs::File, rc::Rc};

pub type SharedTheme = Rc<Theme>;

#[derive(serde::Serialize, serde::Deserialize)]
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
    pub fn load_theme(ph: &std::path::PathBuf) -> Result<Self, String> {
        File::open(ph)
            .map_err(|e| e.to_string())
            .and_then(|f| serde_yaml::from_reader(f).map_err(|e| e.to_string()))
    }
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
