pub mod chord;
pub mod dualtab;
pub mod fzffind;
pub mod help;
pub mod popmsg;
pub mod tab;

#[macro_export]
macro_rules! new_type_impl_tuiwidget {
    ($i:ident) => {
        impl crate::tui::TuiWidget for $i {
            fn handle_key_event(&mut self, kv: &crate::tui::Key) {
                self.0.handle_key_event(kv);
            }

            fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
                self.0.render(f, area);
            }

            fn sync(&mut self) {
                self.0.sync();
            }

            fn on_enter(&mut self) {
                self.0.on_enter();
            }

            fn on_leave(&mut self) {
                self.0.on_leave();
            }
        }
    };
}
