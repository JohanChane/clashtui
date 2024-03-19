use ui::event::{KeyCode, KeyEvent};
#[derive(PartialEq)]
pub enum Keys {
    ProfileSwitch,
    ProfileUpdate,
    ProfileUpdateAll,
    ProfileImport,
    ProfileDelete,
    ProfileTestConfig,
    TemplateSwitch,
    Edit,
    Preview,

    Down,
    Up,
    // Left,
    // Right,
    Select,
    Esc,
    Tab,

    SoftRestart,
    LogCat,
    AppQuit,
    AppConfig,
    ClashConfig,
    AppHelp,
    AppInfo,

    Reserved,
}

impl From<KeyCode> for Keys {
    fn from(value: KeyCode) -> Self {
        match value {
            // Convention: Global Shortcuts As much as possible use uppercase. And Others as much as possible use lowcase to avoid conflicts with global shortcuts.

            // ## Common shortcuts
            KeyCode::Down | KeyCode::Char('j') => Keys::Down,
            KeyCode::Up | KeyCode::Char('k') => Keys::Up,
            KeyCode::Enter => Keys::Select,
            KeyCode::Esc => Keys::Esc,
            KeyCode::Tab => Keys::Tab,

            // ## Profile Tab shortcuts
            KeyCode::Char('p') => Keys::ProfileSwitch,      // Not Global shortcuts
            KeyCode::Char('t') => Keys::TemplateSwitch,     // Not Global shortcuts

            // ## For operating file in Profile and Template Windows
            KeyCode::Char('e') => Keys::Edit,
            KeyCode::Char('v') => Keys::Preview,

            // ## Profile windows shortcuts
            KeyCode::Char('u') => Keys::ProfileUpdate,
            KeyCode::Char('a') => Keys::ProfileUpdateAll,
            KeyCode::Char('i') => Keys::ProfileImport,
            KeyCode::Char('d') => Keys::ProfileDelete,
            KeyCode::Char('s') => Keys::ProfileTestConfig,

            // ## Global Shortcuts (As much as possible use uppercase. And Others as much as possible use lowcase to avoid conflicts with global shortcuts.)
            KeyCode::Char('q') => Keys::AppQuit,   // Exiting is a common operation, and most software also exits with "q", so let's use "q".
            KeyCode::Char('R') => Keys::SoftRestart,
            KeyCode::Char('L') => Keys::LogCat,
            KeyCode::Char('?') => Keys::AppHelp,
            KeyCode::Char('I') => Keys::AppInfo,
            KeyCode::Char('H') => Keys::AppConfig,
            KeyCode::Char('G') => Keys::ClashConfig,

            _ => Keys::Reserved,
        }
    }
}

// impl core::cmp::PartialEq<KeyCode> for Keys {
//     fn eq(&self, other: &KeyCode) -> bool {
//         <KeyCode as Into<Keys>>::into(*other) == *self
//     }
// }

impl core::cmp::PartialEq<KeyEvent> for Keys {
    fn eq(&self, other: &KeyEvent) -> bool {
        <KeyCode as Into<Keys>>::into(other.code) == *self
    }
}
