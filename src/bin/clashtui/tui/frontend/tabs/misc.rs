macro_rules! build_tabs {
    ($(#[$attr:meta])*
    $vis:vis enum $name: ident
    {$($variant:ident($typ:ty),)*}) => {
        $(#[$attr])*
        $vis enum $name {
            $($variant($typ)),*
        }

        impl TabCont for $name{
            fn get_backend_call(&mut self) -> Option<Call> {
                match self {
                    $(Self::$variant(ref mut tab) => tab.get_backend_call(),)*
                }
            }

            fn get_popup_content(&mut self) -> Option<PopMsg> {
                match self {
                    $(Self::$variant(ref mut tab) => tab.get_popup_content(),)*
                }
            }

            fn apply_backend_call(&mut self, op: CallBack) {
                match self {
                    $(Self::$variant(ref mut tab) => tab.apply_backend_call(op),)*
                }
            }

            fn apply_popup_result(&mut self, evst: EventState) -> EventState {
                match self {
                    $(Self::$variant(ref mut tab) => tab.apply_popup_result(evst),)*
                }
            }
        }

        impl Drawable for $name{
            fn handle_key_event(&mut self, ev: &KeyEvent) -> EventState {
                match self {
                    $(Self::$variant(ref mut tab) => tab.handle_key_event(ev),)*
                }
            }

            fn render(&mut self, f: &mut Frame, area: Rect, is_fouced: bool) {
                match self {
                    $(Self::$variant(ref mut tab) => tab.render(f, area, is_fouced),)*
                }
            }
        }

    };
}

use super::*;

impl From<ServiceTab> for TabContainer {
    fn from(value: ServiceTab) -> Self {
        Self(Tabs::Service(value))
    }
}

impl TabCont for TabContainer {
    fn get_backend_call(&mut self) -> Option<Call> {
        self.0.get_backend_call()
    }

    fn get_popup_content(&mut self) -> Option<PopMsg> {
        self.0.get_popup_content()
    }

    fn apply_backend_call(&mut self, op: CallBack) {
        self.0.apply_backend_call(op)
    }

    fn apply_popup_result(&mut self, evst: EventState) -> EventState {
        self.0.apply_popup_result(evst)
    }
}
impl Drawable for TabContainer {
    /// call [`TabCont::apply_popup_result`] first
    fn handle_key_event(&mut self, ev: &KeyEvent) -> EventState {
        self.0.handle_key_event(ev)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, is_fouced: bool) {
        self.0.render(f, area, is_fouced)
    }
}
