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

use crate::config::CoreType;
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
    detected_core_type: Option<CoreType>,
    error: Option<String>,
    paused: bool,
}

impl BasicTabContent for Status {
    type Key = Key;

    type State = ();

    const TITLE: &str = "Status";

    fn after_sync(&self, task_set: &mut FutureSet<Self>) {
        if self.paused {
            return;
        }
        async {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            let version = tri!(restful::control::version(), or_set);
            let config = tri!(restful::config::fetch(), or_set);
            let detected = tri!(restful::core_detect::detect_core_type(), or_set);

            wrapper(move |content: &mut Self| {
                let configured = CONFIG.core_type();
                content.detected_core_type = Some(detected);
                if detected == configured {
                    content.version = Some(version);
                    content.config = Some(config);
                    crate::config::set_core_mismatch(false);
                } else {
                    content.error = Some(format!(
                        "API returned {detected} data, but {configured} is configured"
                    ));
                    crate::config::set_core_mismatch(true);
                }
            })
        }
        .spawn_at(task_set);
    }

    fn on_enter(&mut self, task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        self.paused = false;

        // Synchronous detection so other tabs see the flag before their first fetch
        let was_unknown = self.detected_core_type.is_none();
        match restful::core_detect::detect_core_type() {
            Ok(detected) => {
                let configured = CONFIG.core_type();
                self.detected_core_type = Some(detected);
                let mismatch = detected != configured;
                crate::config::set_core_mismatch(mismatch);
                if mismatch {
                    let msg = format!("API returned {detected} data, but {configured} is configured");
                    self.error = Some(msg.clone());
                    if was_unknown {
                        crate::tui::widget::popmsg::Confirm::err(msg);
                    }
                }
            }
            Err(e) => {
                self.error = Some(format!("Core detection failed: {e}"));
            }
        }

        if crate::config::is_core_mismatch() {
            return;
        }

        async {
            let version = tri!(restful::control::version());
            let config = tri!(restful::config::fetch());
            let detected = tri!(restful::core_detect::detect_core_type());

            wrapper(move |content: &mut Self| {
                let configured = CONFIG.core_type();
                content.detected_core_type = Some(detected);
                if detected == configured {
                    content.version = Some(version);
                    content.config = Some(config);
                    crate::config::set_core_mismatch(false);
                } else {
                    content.error = Some(format!(
                        "API returned {detected} data, but {configured} is configured"
                    ));
                    crate::config::set_core_mismatch(true);
                }
            })
        }
        .spawn_at(task_set);
    }

    fn on_leave(&mut self, _task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        self.paused = true;
    }
}

impl TabContent for Status {
    fn init(&mut self, _task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        self.paused = true;
        self.error = Some("Waiting".to_owned());
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
        let mut lines: Vec<String> = vec![];
        let configured = CONFIG.core_type();
        if let Some(detected) = self.detected_core_type {
            if detected == configured {
                lines.push(format!("core: {detected}"));
            } else {
                lines.push(format!("core: {detected} (configured: {configured}, MISMATCH)"));
            }
        }
        let matched = self
            .detected_core_type
            .map_or(true, |d| d == configured);
        if matched {
            if let Some(ref ver) = self.version {
                lines.push(format!("version: {ver}"));
            }
            if let Some(cfg) = self.config.as_ref() {
                lines.extend(cfg.build());
            }
        }
        if lines.is_empty() {
            lines.push(
                self.error
                    .as_deref()
                    .map(|s| s.to_owned())
                    .expect("if there is not content, there should be an error"),
            );
        }
        let widget = Paragraph::new(Text::from_iter(lines)).block(block);
        f.render_widget(widget, area);
    }
}
