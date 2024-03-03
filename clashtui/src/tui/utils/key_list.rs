use crossterm::event::{KeyCode, KeyEvent};
#[derive(PartialEq)]
pub enum Keys {
    ProfileSwitch,
    ProfileUpdate,
    ProfileUpdateAll,
    ProfileImport,
    ProfileDelete,
    ProfileTestConfig,
    TemplateSwitch,
    ClashsrvctlRestart,

    Select,
    Esc,
    Edit,
    Preview,
    LogCat,
    AppQuit,
    AppConfig,
    ClashConfig,
    AppHelp,

    Reserved,
}

impl From<KeyCode> for Keys {
    fn from(value: KeyCode) -> Self {
        match value {
            KeyCode::Char('P') => Keys::ProfileSwitch,
            KeyCode::Char('u') => Keys::ProfileUpdate,
            KeyCode::Char('U') => Keys::ProfileUpdateAll,
            KeyCode::Char('i') => Keys::ProfileImport,
            KeyCode::Char('d') => Keys::ProfileDelete,
            KeyCode::Char('t') => Keys::ProfileTestConfig,

            KeyCode::Char('T') => Keys::TemplateSwitch,

            KeyCode::Char('R') => Keys::ClashsrvctlRestart,

            KeyCode::Enter => Keys::Select,
            KeyCode::Esc => Keys::Esc,
            KeyCode::Char('e') => Keys::Edit,
            KeyCode::Char('p') => Keys::Preview,
            KeyCode::Char('L') => Keys::LogCat,
            KeyCode::Char('Q') => Keys::AppQuit,
            KeyCode::Char('?') => Keys::AppHelp,
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
