mod impl_profile;
#[cfg(feature = "template")]
mod impl_template;
mod ops;

use crossterm::event::KeyEvent;
pub use ops::*;

use crate::tui::{
    frontend::{consts::TAB_TITLE_PROFILE, key_bind::Keys},
    widget::{List, PopRes},
    Drawable, EventState,
};
use ratatui::prelude as Ra;
use Ra::{Frame, Rect};

use super::{Call, CallBack, PopMsg, TabCont};

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
    /// hold content for msg ask
    temp_content: Option<TmpOps>,
    is_profiles_outdated: bool,
    #[cfg(feature = "template")]
    is_templates_outdated: bool,
}

impl Default for ProfileTab {
    fn default() -> Self {
        #[cfg(feature = "template")]
        use crate::tui::frontend::consts::TAB_TITLE_TEMPLATE;
        let profiles = List::new(TAB_TITLE_PROFILE.to_string());
        #[cfg(feature = "template")]
        let templates = List::new(TAB_TITLE_TEMPLATE.to_owned());

        Self {
            profiles,
            #[cfg(feature = "template")]
            templates,
            focus: Focus::Profile,
            popup_content: None,
            backend_content: None,
            temp_content: None,
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
        //
        // since default content is to init templates
        // every thing should have inited
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
                    .replace(PopMsg::Prompt(format!("Done\n{}", result.join("\n"))));
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
                self.popup_content.replace(PopMsg::Prompt(
                    ["Done".to_string()]
                        .into_iter()
                        .chain(result)
                        .collect::<Vec<_>>()
                        .join("\n"),
                ));
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

    fn apply_popup_result(&mut self, res: PopRes) -> EventState {
        match self.focus {
            Focus::Profile => {
                if let Some(op) = self.temp_content.take() {
                    match op {
                        TmpOps::EditWhich(..) | TmpOps::EditUses(..) => unreachable!(),
                        TmpOps::UpdateWithProxy(name) => {
                            let PopRes::Selected(selected) = res else {
                                unreachable!("Should always be Choices")
                            };
                            let with_proxy = match selected {
                                // regarded as cancel
                                // if get No, this order is dropped
                                // as it is already moved out by `take`
                                0 => return EventState::WorkDone,
                                // regarded as yes
                                // if get Yes, we confirm this order and ready to send it
                                1 => Some(true),
                                // regarded as extra-choices
                                2 => Some(false),
                                3 => None,
                                // ignore others
                                _ => unreachable!(),
                            };
                            self.temp_content =
                                Some(TmpOps::UpdateWithProxyProvider(name, with_proxy));
                            self.popup_content = Some(PopMsg::AskChoices(
                                "Skip proxy-provider merging?\nWhich is 'no_pp'".to_owned(),
                                vec!["No".to_owned(), "Yes".to_owned()],
                            ));
                        }
                        TmpOps::UpdateWithProxyProvider(name, with_proxy) => {
                            let PopRes::Selected(selected) = res else {
                                unreachable!("Should always be Choices")
                            };
                            let no_pp = match selected {
                                // regarded as cancel
                                // if get No, this order is dropped
                                // as it is already moved out by `take`
                                0 => true,
                                // regarded as yes
                                // if get Yes, we confirm this order and ready to send it
                                1 => false,
                                // regarded as extra-choices
                                // ignore others
                                _ => unreachable!(),
                            };
                            self.backend_content = Some(Call::Profile(BackendOp::Profile(
                                ProfileOp::Update(name, with_proxy, no_pp),
                            )));
                            self.popup_content = Some(PopMsg::Prompt("Working".to_owned()));
                        }
                        TmpOps::Remove(name) => {
                            let PopRes::Selected(selected) = res else {
                                unreachable!("Should always be Choices")
                            };
                            match selected {
                                // if get No, this order is dropped
                                // as it is already moved out by `take`
                                0 => (),
                                // if get Yes, we confirm this order and ready to send it
                                1 => {
                                    self.backend_content = Some(Call::Profile(BackendOp::Profile(
                                        ProfileOp::Remove(name),
                                    )))
                                }
                                // regarded as extra-choices
                                // ignore others
                                _ => unreachable!(),
                            };
                            self.popup_content = Some(PopMsg::Prompt("Working".to_owned()));
                        }
                        TmpOps::SetFilter => {
                            let PopRes::Input(vec) = res else {
                                unreachable!("Should always be Input")
                            };
                            self.profiles.set_filter(vec)
                        }
                        TmpOps::Import => {
                            let PopRes::Input(name) = res else {
                                unreachable!("Should always be Input")
                            };
                            self.temp_content = Some(TmpOps::ImportWithName(name));
                            self.popup_content = Some(PopMsg::Input("Url".to_owned()));
                        }
                        TmpOps::ImportWithName(name) => {
                            let PopRes::Input(url) = res else {
                                unreachable!("Should always be Input")
                            };
                            self.backend_content =
                                Some(Call::Profile(BackendOp::Profile(ProfileOp::Add(name, url))));
                            self.popup_content = Some(PopMsg::Prompt("Working".to_owned()));
                        }
                    }
                };
            }
            #[cfg(feature = "template")]
            Focus::Template => {
                if let Some(op) = self.temp_content.take() {
                    match op {
                        TmpOps::ImportWithName(..)
                        | TmpOps::UpdateWithProxy(..)
                        | TmpOps::UpdateWithProxyProvider(..) => unreachable!(),
                        TmpOps::Remove(name) => {
                            let PopRes::Selected(selected) = res else {
                                unreachable!("Should always be Choices")
                            };
                            match selected {
                                // if get No, this order is dropped
                                // as it is already moved out by `take`
                                0 => (),
                                // if get Yes, we confirm this order and ready to send it
                                1 => {
                                    self.backend_content = Some(Call::Profile(BackendOp::Template(
                                        TemplateOp::Remove(name),
                                    )))
                                }
                                // regarded as extra-choices
                                // ignore others
                                _ => unreachable!(),
                            };
                            self.popup_content = Some(PopMsg::Prompt("Working".to_owned()));
                        }
                        TmpOps::SetFilter => {
                            let PopRes::Input(name) = res else {
                                unreachable!("Should always be Input")
                            };
                            self.templates.set_filter(name);
                        }
                        TmpOps::Import => {
                            let PopRes::Input(name) = res else {
                                unreachable!("Should always be Input")
                            };
                            self.backend_content =
                                Some(Call::Profile(BackendOp::Template(TemplateOp::Add(name))));
                            self.popup_content = Some(PopMsg::Prompt("Working".to_owned()));
                        }
                        TmpOps::EditWhich(name) => {
                            let PopRes::Selected(selected) = res else {
                                unreachable!("Should always be Choices")
                            };
                            match selected {
                                // regarded as cancel
                                // if get No, this order is dropped
                                // as it is already moved out by `take`
                                0 => return EventState::WorkDone,
                                // edit uses
                                1 => {
                                    self.temp_content = Some(TmpOps::EditUses(name));
                                    self.popup_content = Some(PopMsg::SelectMulti(
                                        "Edit Uses".to_owned(),
                                        self.profiles.get_items().to_owned(),
                                    ));
                                }
                                // edit content
                                2 => {
                                    self.backend_content = Some(Call::Profile(
                                        BackendOp::Template(TemplateOp::Edit(name)),
                                    ));
                                    self.popup_content = Some(PopMsg::Prompt("Working".to_owned()));
                                }
                                // ignore others
                                _ => unreachable!(),
                            };
                        }
                        TmpOps::EditUses(name) => {
                            let PopRes::SelectedMulti(selected) = res else {
                                unreachable!("Should always be Choices")
                            };
                            self.backend_content =
                                Some(Call::Profile(BackendOp::Template(TemplateOp::Uses(
                                    name,
                                    self.profiles
                                        .get_items()
                                        .iter()
                                        .enumerate()
                                        .filter_map(|(idx, profile_name)| {
                                            selected
                                                .contains(&idx)
                                                .then_some(profile_name.to_owned())
                                        })
                                        .collect(),
                                ))));
                        }
                    }
                }
            }
        }
        EventState::WorkDone
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
    /// - Catched event -> [EventState::WorkDone]
    /// - unrecognized event -> [EventState::NotConsumed]
    fn handle_key_event(&mut self, ev: &KeyEvent) -> EventState {
        match self.focus {
            Focus::Profile => {
                if self.profiles.handle_key_event(ev) == EventState::Yes {
                    self.backend_content = self
                        .profiles
                        .selected()
                        .inspect(|_| {
                            self.popup_content = Some(PopMsg::Prompt("Working".to_owned()))
                        })
                        .and_then(|index| self.profiles.get_items().get(index).cloned())
                        .map(ProfileOp::Select)
                        .map(BackendOp::Profile)
                        .map(Call::Profile);
                    EventState::WorkDone
                } else {
                    self.handle_profile_key_event(ev)
                }
            }
            #[cfg(feature = "template")]
            Focus::Template => {
                if self.templates.handle_key_event(ev) == EventState::Yes {
                    // Enter means select
                    self.backend_content = self
                        .templates
                        .selected()
                        .inspect(|_| {
                            self.popup_content = Some(PopMsg::Prompt("Working".to_owned()))
                        })
                        .and_then(|index| self.templates.get_items().get(index).cloned())
                        .map(TemplateOp::Generate)
                        .map(BackendOp::Template)
                        .map(Call::Profile);
                    EventState::WorkDone
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
