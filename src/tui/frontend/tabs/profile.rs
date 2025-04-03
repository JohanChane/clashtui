use crate::tui::{
    Drawable, EventState,
    frontend::{consts::TAB_TITLE_PROFILE, key_bind::Keys},
    widget::{List, PopRes},
};

use Ra::{Frame, Rect};
use crossterm::event::KeyEvent;
use ratatui::prelude as Ra;

use super::{Call, CallBack, PopMsg, TabCont};

#[macro_use]
mod ops;
mod impl_profile;
#[cfg(feature = "template")]
mod impl_template;

pub use ops::*;

#[derive(PartialEq, Clone, Copy)]
enum Focus {
    Profile,
    #[cfg(feature = "template")]
    Template,
}

pub(in crate::tui::frontend) struct ProfileTab {
    profiles: List,
    #[cfg(feature = "template")]
    templates: List,
    focus: Focus,
    popup_content: Option<PopMsg>,
    backend_content: Option<Call>,
    is_profiles_outdated: bool,
    #[cfg(feature = "template")]
    is_templates_outdated: bool,
}

impl Default for ProfileTab {
    fn default() -> Self {
        #[cfg(feature = "template")]
        use crate::tui::frontend::consts::TAB_TITLE_TEMPLATE;
        let profiles = List::new(TAB_TITLE_PROFILE);
        #[cfg(feature = "template")]
        let templates = List::new(TAB_TITLE_TEMPLATE);

        Self {
            profiles,
            #[cfg(feature = "template")]
            templates,
            focus: Focus::Profile,
            popup_content: None,
            backend_content: None,
            is_profiles_outdated: true,
            #[cfg(feature = "template")]
            is_templates_outdated: true,
        }
    }
}

impl std::fmt::Display for ProfileTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", crate::tui::frontend::consts::TAB_TITLE_PROFILE)
    }
}

impl TabCont for ProfileTab {
    fn get_backend_call(&mut self) -> Option<Call> {
        // if not is_inited, init profiles
        // else take content
        if self.is_profiles_outdated {
            Some(Call::Profile(BackendOp::Profile(ProfileOp::GetALL)))
        } else {
            #[cfg(feature = "template")]
            if self.is_templates_outdated {
                Some(Call::Profile(BackendOp::Template(TemplateOp::GetALL)))
            } else {
                self.backend_content.take()
            }
            #[cfg(not(feature = "template"))]
            self.backend_content.take()
        }
    }

    fn get_popup_content(&mut self) -> Option<PopMsg> {
        self.popup_content.take()
    }

    fn apply_backend_call(&mut self, op: CallBack) {
        match op {
            CallBack::ProfileCTL(result) => {
                // require a refresh
                self.is_profiles_outdated = true;
                self.popup_content
                    .replace(PopMsg::msg(format!("Done\n{}", result.join("\n"))));
            }
            CallBack::ProfileInit(content, times) => {
                if self.is_profiles_outdated {
                    self.profiles.set_items(content);
                    self.profiles.set_extras(
                        times
                            .into_iter()
                            .map(|t| t.map(display_duration).unwrap_or("Never/Err".to_string())),
                    );
                    self.is_profiles_outdated = false;
                }
            }
            #[cfg(feature = "template")]
            CallBack::TemplateCTL(result) => {
                // require a refresh
                self.is_templates_outdated = true;
                self.is_profiles_outdated = true;
                self.popup_content
                    .replace(PopMsg::msg(format!("Done\n{}", result.join("\n"))));
            }
            #[cfg(feature = "template")]
            CallBack::TemplateInit(content) => {
                if self.is_templates_outdated {
                    self.templates.set_items(content);
                    self.is_templates_outdated = false;
                }
            }
            _ => unreachable!("{} get unexpected op: {:?}", TAB_TITLE_PROFILE, op),
        }
    }

    fn apply_popup_result(&mut self, res: PopRes) {
        let PopRes::Input(name) = res else {
            unreachable!("Should always be Input")
        };
        match self.focus {
            Focus::Profile => {
                self.profiles.set_filter(name);
            }
            #[cfg(feature = "template")]
            Focus::Template => {
                self.templates.set_filter(name);
            }
        }
    }
}

impl Drawable for ProfileTab {
    fn render(&mut self, f: &mut Frame, area: Rect, _: bool) {
        #[cfg(feature = "template")]
        {
            use Ra::{Constraint, Layout};
            let chunks = Layout::default()
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);
            self.profiles
                .render(f, chunks[0], self.focus == Focus::Profile);
            self.templates
                .render(f, chunks[1], self.focus == Focus::Template);
        }
        #[cfg(not(feature = "template"))]
        self.profiles.render(f, area, self.focus == Focus::Profile);
    }
    fn handle_key_event(&mut self, ev: &KeyEvent) -> EventState {
        match self.focus {
            Focus::Profile => {
                if self.profiles.handle_key_event(ev) == EventState::Yes {
                    self.backend_content = self
                        .profiles
                        .selected()
                        .inspect(|_| {
                            self.popup_content = Some(PopMsg::working());
                        })
                        .and_then(|index| self.profiles.get_items().get(index).cloned())
                        .map(ProfileOp::Select)
                        .map(BackendOp::Profile)
                        .map(Call::Profile);
                    EventState::Consumed
                } else {
                    self.handle_profile_key_event(ev)
                }
            }
            #[cfg(feature = "template")]
            Focus::Template => {
                if self.templates.handle_key_event(ev) == EventState::Yes {
                    self.backend_content = self
                        .templates
                        .selected()
                        .inspect(|_| {
                            self.popup_content = Some(PopMsg::working());
                        })
                        .and_then(|index| self.templates.get_items().get(index).cloned())
                        .map(TemplateOp::Generate)
                        .map(BackendOp::Template)
                        .map(Call::Profile);
                    EventState::Consumed
                } else {
                    self.handle_template_key_event(ev)
                }
            }
        }
        .unify()
    }
}

fn display_duration(t: std::time::Duration) -> String {
    use std::time::Duration;
    if t.is_zero() {
        "Just Now".to_string()
    } else if t < Duration::from_secs(60 * 59) {
        let min = t.as_secs() / 60;
        format!("In {} mins", min + 1)
    } else if t < Duration::from_secs(3600 * 24) {
        let hou = t.as_secs() / 3600;
        format!("In {hou} hours")
    } else {
        let day = t.as_secs() / (3600 * 24);
        format!("In about {day} days")
    }
}
