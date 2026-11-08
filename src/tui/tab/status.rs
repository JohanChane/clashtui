use ratatui::{text::Text, widgets::Paragraph};

use super::dev::*;

newtype_tab!(@no_action StatusTab(Status));

use std::time::Duration;

use crate::config::CONFIG;
use crate::functions::restful::{self, config_struct::*};

macro_rules! tri {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                crate::tui::widget::popmsg::Confirm::err(e);
                return do_nothing();
            }
        }
    };
    ($e:expr, or_set) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                return wrapper(move |content: &mut Self| {
                    content.error = Some(e.to_string());
                });
            }
        }
    };
}

#[derive(Default)]
struct Status {
    config: Option<ClashConfig>,
    version: Option<String>,
    error: Option<String>,
}

impl Page for Status {
    type Action = std::convert::Infallible;
    type State = ();
    const TITLE: &str = "Status";
    const TICK: Option<Duration> = Some(Duration::from_secs(1));

    fn on_event(&mut self, event: &Event, cx: &mut Cx<Self>) {
        match event {
            Event::Shown
            | Event::Tick
            | Event::Core(CoreEvent::StatusChanged(_) | CoreEvent::ConfigChanged) => {
                match monitor::status() {
                    CoreStatus::Ready { version } => {
                        self.version = Some(version);
                        self.error = None;
                        self.fetch_config(cx);
                    }
                    status => self.show(status),
                }
            }
            _ => {}
        }
    }

    fn on_action(&mut self, action: Self::Action, _cx: &mut Cx<Self>, _state: &mut Self::State) {
        match action {}
    }

    fn render(&self, f: &mut Frame, area: Rect, _state: &mut Self::State, _focused: bool) {
        let block = Block::bordered()
            .border_style(Theme::get().section("status").border)
            .title(Self::TITLE);
        let mut lines: Vec<String> = vec![];
        if let Some(ref ver) = self.version {
            lines.push(format!("core: {}", CONFIG.core_type()));
            lines.push(format!("version: {ver}"));
        }
        if let Some(cfg) = self.config.as_ref() {
            lines.extend(cfg.build());
        }
        if lines.is_empty() {
            lines.push(
                self.error
                    .clone()
                    .unwrap_or_else(|| "Detecting core...".to_owned()),
            );
        } else if let Some(err) = &self.error {
            lines.push(err.clone());
        }
        let widget = Paragraph::new(Text::from_iter(lines)).block(block);
        f.render_widget(widget, area);
    }
}

impl Status {
    fn show(&mut self, status: CoreStatus) {
        self.version = None;
        self.config = None;
        self.error = status.ready().err();
    }

    fn fetch_config(&mut self, cx: &mut Cx<Self>) {
        async {
            let config = tri!(
                tokio::task::spawn_blocking(restful::config::fetch)
                    .await
                    .unwrap(),
                or_set
            );
            wrapper(move |content: &mut Self| {
                content.config = Some(config);
                content.error = None;
            })
        }
        .spawn_at(cx);
    }
}
