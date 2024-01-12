use crossterm::event::{KeyCode, KeyEvent};

pub struct ClashTuiKeyEvent {
    pub code: KeyCode,
    //pub modifiers: KeyModifiers,
}

impl ClashTuiKeyEvent {
    pub const fn new(code: KeyCode) -> Self {
        Self { code }
    }
}

#[inline]
pub fn match_key(ev: &KeyEvent, binding: &ClashTuiKeyEvent) -> bool {
    ev.code == binding.code
}

pub struct KeyList {
    pub profile_switch: ClashTuiKeyEvent,
    pub profile_select: ClashTuiKeyEvent,
    pub profile_update: ClashTuiKeyEvent,
    pub profile_update_all: ClashTuiKeyEvent,
    pub profile_import: ClashTuiKeyEvent,
    pub profile_delete: ClashTuiKeyEvent,
    pub profile_test_config: ClashTuiKeyEvent,
    pub template_switch: ClashTuiKeyEvent,
    pub template_create: ClashTuiKeyEvent,
    pub clashsrvctl_select: ClashTuiKeyEvent,
    pub clashsrvctl_restart: ClashTuiKeyEvent,
    pub config_select: ClashTuiKeyEvent,

    //pub edit: ClashTuiKeyEvent,
    pub preview: ClashTuiKeyEvent,
    //pub app_home_open: ClashTuiKeyEvent,
    //pub clash_cfg_dir_open: ClashTuiKeyEvent,
    pub log_cat: ClashTuiKeyEvent,
    pub app_quit: ClashTuiKeyEvent,
    pub app_help: ClashTuiKeyEvent,
}

impl KeyList {}

impl Default for KeyList {
    fn default() -> Self {
        Self {
            profile_switch: ClashTuiKeyEvent::new(KeyCode::Char('P')),
            profile_select: ClashTuiKeyEvent::new(KeyCode::Enter),
            profile_update: ClashTuiKeyEvent::new(KeyCode::Char('u')),
            profile_update_all: ClashTuiKeyEvent::new(KeyCode::Char('U')),
            profile_import: ClashTuiKeyEvent::new(KeyCode::Char('i')),
            profile_delete: ClashTuiKeyEvent::new(KeyCode::Char('d')),
            profile_test_config: ClashTuiKeyEvent::new(KeyCode::Char('t')),

            template_switch: ClashTuiKeyEvent::new(KeyCode::Char('T')),
            template_create: ClashTuiKeyEvent::new(KeyCode::Enter),
            clashsrvctl_select: ClashTuiKeyEvent::new(KeyCode::Enter),
            clashsrvctl_restart: ClashTuiKeyEvent::new(KeyCode::Char('R')),
            config_select: ClashTuiKeyEvent::new(KeyCode::Enter),

            //edit: ClashTuiKeyEvent::new(KeyCode::Char('e')),
            preview: ClashTuiKeyEvent::new(KeyCode::Char('p')),
            //app_home_open: ClashTuiKeyEvent::new(KeyCode::Char('H')),
            //clash_cfg_dir_open: ClashTuiKeyEvent::new(KeyCode::Char('G')),
            log_cat: ClashTuiKeyEvent::new(KeyCode::Char('L')),
            app_quit: ClashTuiKeyEvent::new(KeyCode::Char('Q')),
            app_help: ClashTuiKeyEvent::new(KeyCode::Char('?')),
        }
    }
}
