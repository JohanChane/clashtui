use ratatui::{text::Text, widgets::Paragraph};

use super::dev::*;

newtype_tab!(StatusTab(Tab<Status>));

#[derive(Clone, Copy)]
enum Key {}

impl TryFrom<&crate::tui::Key> for Key {
    type Error = ();

    fn try_from(_: &crate::tui::Key) -> Result<Self, Self::Error> {
        Err(())
    }
}

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

impl BasicTabContent for Status {
    type Key = Key;

    type State = ();

    const TITLE: &str = "Status";

    fn after_sync(&self, task_set: &mut FutureSet<Self>) {
        async {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            let version = tri!(restful::control::version(), or_set);
            let config = tri!(restful::config::fetch(), or_set);

            wrapper(|content: &mut Self| {
                content.version = Some(version);
                content.config = Some(config);
            })
        }
        .spawn_at(task_set);
    }
}

impl TabContent for Status {
    fn init(&mut self, task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        self.error = Some("Waiting".to_owned());
        async {
            let version = tri!(restful::control::version());
            let config = tri!(restful::config::fetch());

            wrapper(|content: &mut Self| {
                content.version = Some(version);
                content.config = Some(config);
            })
        }
        .spawn_at(task_set);
    }

    fn handle_key_event(
        &mut self,
        _key: Self::Key,
        _task_set: &mut FutureSet<Self>,
        _state: &mut Self::State,
    ) {
    }

    fn render(&self, f: &mut Frame, area: Rect, _state: &mut Self::State) {
        let block = Block::bordered()
            .border_style(Theme::get().tab.tab_focused)
            .title(Self::TITLE);
        let lines = if let Some(cfg) = self.config.as_ref() {
            cfg.build()
        } else {
            vec![
                self.error
                    .as_deref()
                    .map(|s| s.to_owned())
                    .expect("if there is not content, there should be an error"),
            ]
        };
        let widget = Paragraph::new(Text::from_iter(lines)).block(block);
        f.render_widget(widget, area);
    }
}
