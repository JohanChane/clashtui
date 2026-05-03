pub mod chord;
pub mod dualtab;
pub mod popmsg;
pub mod tab;

#[macro_export]
macro_rules! new_type_impl_tuiwidget {
    ($i:ident) => {
        impl crate::tui::TuiWidget for $i {
            fn handle_key_event(&mut self, kv: &KeyEvent) {
                self.0.handle_key_event(kv);
            }

            fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
                self.0.render(f, area);
            }

            fn sync(&mut self) {
                self.0.sync();
            }
        }
    };
}
