pub mod dualtab;
pub mod help;
pub mod popmsg;
pub mod tab;
pub mod which;

#[macro_export]
macro_rules! new_type_impl_tuiwidget {
    ($i:ident) => {
        impl $crate::tui::TuiWidget for $i {
            fn render(
                &mut self,
                f: &mut ratatui::Frame,
                area: ratatui::layout::Rect,
                view: $crate::tui::business::BusinessView<'_>,
            ) {
                self.0.render(f, area, view);
            }

            fn on_enter(&mut self, business: &mut $crate::tui::business::Business) {
                self.0.on_enter(business);
            }
            fn on_leave(&mut self, business: &mut $crate::tui::business::Business) {
                self.0.on_leave(business);
            }
            fn sync_from(&mut self, view: $crate::tui::business::BusinessView<'_>) {
                self.0.sync_from(view);
            }
            fn advance(&mut self) {
                self.0.advance();
            }
            fn poll_tasks(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
                self.0.poll_tasks(cx)
            }
            fn finish_task(&mut self, visible: bool) -> bool {
                self.0.finish_task(visible)
            }
            fn has_tasks(&self) -> bool {
                self.0.has_tasks()
            }
            fn needs_animation(&self) -> bool {
                self.0.needs_animation()
            }
        }
    };
}
