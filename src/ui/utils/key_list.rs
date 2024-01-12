use crossterm::event::{KeyCode, KeyEvent};

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
    //Edit,
    Preview,
    LogCat,
    AppQuit,
    AppHelp,
}

impl Keys {
    pub fn bindto(self) -> KeyCode{
        match self {
            Keys::ProfileSwitch => KeyCode::Char('P'),
            Keys::ProfileUpdate => KeyCode::Char('u'),
            Keys::ProfileUpdateAll => KeyCode::Char('U'),
            Keys::ProfileImport => KeyCode::Char('i'),
            Keys::ProfileDelete => KeyCode::Char('d'),
            Keys::ProfileTestConfig => KeyCode::Char('t'),

            Keys::TemplateSwitch => KeyCode::Char('T'),

            Keys::ClashsrvctlRestart => KeyCode::Char('R'),
            
            Keys::Select => KeyCode::Enter,
            //Keys::Edit => KeyCode::Char('e'),
            Keys::Preview => KeyCode::Char('p'),
            Keys::LogCat => KeyCode::Char('L'),
            Keys::AppQuit => KeyCode::Char('Q'),
            Keys::AppHelp => KeyCode::Char('?'),
        }
    }
    pub fn is(self, code:&KeyEvent) -> bool{
        self.bindto() == code.code
    }
}