use ratatui::style::Color;
use std::sync::OnceLock;

static GLOBAL_THEME: OnceLock<Theme> = OnceLock::new();

// Given that this is a single-theard app, sync do not seem to be important
// But OnceCell impl !Sync, so this new type is needed
// However, there seems to be not need to bypass this, so I should just keep it
//
// struct Bx(std::cell::OnceCell<Theme>);
// unsafe impl Sync for Bx {}

// #[derive(serde::Serialize, serde::Deserialize)]
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
    pub fn load(_ph: Option<&std::path::PathBuf>) -> Result<(), anyhow::Error> {
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
