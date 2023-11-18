use ratatui::style::{Color, Modifier, Style};
use std::rc::Rc;

pub type SharedTheme = Rc<Theme>;

pub struct Theme {
    pub popup_block_fg: Color,

    pub list_block_fg_fouced: Color,
    pub list_block_fg_unfouced: Color,
    pub list_hl_bg_fouced: Color,

    pub tab_txt_fg: Color,
    pub tab_hl_fg: Color,

    pub statusbar_txt_fg: Color,
}

impl Theme {}

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
