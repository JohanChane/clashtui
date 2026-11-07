use super::dev::*;
use crate::config::{CONFIG, CoreType};
use crate::tui::core::{ApiResource, ApiState};
use ratatui::{text::Text, widgets::Paragraph};

newtype_tab!(@no_action StatusTab(Tab<Status>));

#[derive(Default)]
struct Status;
impl BasicTabContent for Status {
    type Action = std::convert::Infallible;
    type State = ();
    const TITLE: &str = "Status";
    fn on_enter(&mut self, _: &mut FutureSet<Self>, _: &mut (), business: &mut Business) {
        business.enter(ApiResource::Status);
    }
    fn on_leave(&mut self, _: &mut FutureSet<Self>, _: &mut (), business: &mut Business) {
        business.leave(ApiResource::Status);
    }
}
impl TabContent for Status {
    fn init(&mut self, _: &mut FutureSet<Self>, _: &mut ()) {}
    fn handle_action(
        &mut self,
        action: Self::Action,
        _: &mut FutureSet<Self>,
        _: &mut (),
        _: &mut Business,
    ) {
        match action {}
    }
    fn render(&self, f: &mut Frame, area: Rect, _: &mut (), view: BusinessView<'_>) {
        let block = Block::bordered()
            .border_style(Theme::get().section("status").border)
            .title(Self::TITLE);
        let index = match CONFIG.core_type() {
            CoreType::Mihomo => 0,
            CoreType::Singbox => 1,
        };
        let mut lines = vec![
            format!("core: {}", CONFIG.core_type()),
            format!("service: {}", view.core.state.services[index].description),
        ];
        if let ApiState::Ready { version } = &view.core.state.api {
            lines.push(format!("version: {version}"));
        }
        let config = view
            .core
            .api_allowed()
            .then_some(view.core.queries.status.data.as_ref())
            .flatten();
        if let Some(config) = config {
            lines.extend(config.build());
        }
        let error = view
            .core
            .diagnostic()
            .or_else(|| view.core.queries.status.error.clone());
        if let Some(error) = error {
            lines.push(error);
        } else if config.is_none() {
            lines.push("Loading configuration...".into());
        }
        f.render_widget(Paragraph::new(Text::from_iter(lines)).block(block), area);
    }
}
