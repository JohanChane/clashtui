/// this do only one thing -- build [`Tabs`]
macro_rules! build_tabs {
    ($(#[$attr:meta])*
    $vis:vis enum $name: ident
    {$($(#[$attr_:meta])* $variant:ident($typ:ty),)*}) => {
        $(#[$attr])*
        $vis enum $name {
            $(
                $(#[$attr_])*
                $variant($typ)
            ),*
        }

        impl TabCont for $name{
            fn get_backend_call(&mut self) -> Option<Call> {
                match self {
                    $($(#[$attr_])* Self::$variant(ref mut tab) => tab.get_backend_call(),)*
                }
            }

            fn get_popup_content(&mut self) -> Option<PopMsg> {
                match self {
                    $($(#[$attr_])* Self::$variant(ref mut tab) => tab.get_popup_content(),)*
                }
            }

            fn apply_backend_call(&mut self, op: CallBack) {
                match self {
                    $($(#[$attr_])* Self::$variant(ref mut tab) => tab.apply_backend_call(op),)*
                }
            }

            fn apply_popup_result(&mut self, evst: EventState) -> EventState {
                match self {
                    $($(#[$attr_])* Self::$variant(ref mut tab) => tab.apply_popup_result(evst),)*
                }
            }
        }

        impl Drawable for $name{
            fn handle_key_event(&mut self, ev: &KeyEvent) -> EventState {
                match self {
                    $($(#[$attr_])* Self::$variant(ref mut tab) => tab.handle_key_event(ev),)*
                }
            }

            fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::prelude::Rect, is_fouced: bool) {
                match self {
                    $($(#[$attr_])* Self::$variant(ref mut tab) => tab.render(f, area, is_fouced),)*
                }
            }
        }

        $(
            $(#[$attr_])*
            impl From<$typ> for $name{
                fn from(value: $typ) -> Self {
                    $name::$variant(value)
                }
            }
        )*

    };
}
