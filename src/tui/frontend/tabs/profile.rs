mod impl_profile;
#[cfg(feature = "template")]
mod impl_template;
mod ops;

use crossterm::event::KeyEvent;
pub use ops::*;

use crate::{
    tui::{
        frontend::{consts::TAB_TITLE_PROFILE, key_bind::Keys},
        widget::{List, PopRes},
        Drawable, EventState,
    },
    utils::CallBack,
};
use ratatui::prelude as Ra;
use Ra::{Frame, Rect};

use super::{Call, PopMsg, TabCont};

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
    temp_content: Option<Call>,
    is_profiles_inited: bool,
    #[cfg(feature = "template")]
    is_templates_inited: bool,
}

impl ProfileTab {
    /// Creates a new [`ProfileTab`].
    pub fn new() -> Self {
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
            is_profiles_inited: false,
            #[cfg(feature = "template")]
            is_templates_inited: false,
        }
    }
}

impl Default for ProfileTab {
    fn default() -> Self {
        Self::new()
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
        if self.is_profiles_inited {
            #[cfg(feature = "template")]
            if self.is_templates_inited {
                self.backend_content.take()
            } else {
                Some(Call::Profile(BackendOp::Template(TemplateOp::GetALL)))
            }
            #[cfg(not(feature = "template"))]
            self.backend_content.take()
        } else {
            Some(Call::Profile(BackendOp::Profile(ProfileOp::GetALL)))
        }
    }

    fn get_popup_content(&mut self) -> Option<PopMsg> {
        self.popup_content.take()
    }

    fn apply_backend_call(&mut self, op: CallBack) {
        match op {
            CallBack::ProfileCTL(result) => {
                // require a refresh
                self.is_profiles_inited = false;
                self.popup_content.replace(PopMsg::Prompt(
                    ["Done".to_string()].into_iter().chain(result).collect(),
                ));
            }
            CallBack::ProfileInit(content, times) => {
                if !self.is_profiles_inited {
                    self.profiles.set_items(content);
                    self.profiles.set_extras(
                        times
                            .into_iter()
                            .map(|t| t.map(display_duration).unwrap_or("Never/Err".to_string())),
                    );
                    self.is_profiles_inited = true;
                }
            }
            #[cfg(feature = "template")]
            CallBack::TemplateCTL(result) => {
                // require a refresh
                self.is_templates_inited = false;
                self.is_profiles_inited = false;
                self.popup_content.replace(PopMsg::Prompt(
                    ["Done".to_string()].into_iter().chain(result).collect(),
                ));
            }
            #[cfg(feature = "template")]
            CallBack::TemplateInit(content) => {
                if !self.is_templates_inited {
                    self.templates.set_items(content);
                    self.is_templates_inited = true;
                }
            }
            _ => unreachable!("{} get unexpected op: {:?}", TAB_TITLE_PROFILE, op),
        }
    }

    fn apply_popup_result(&mut self, res: PopRes) -> EventState {
        match self.focus {
            Focus::Profile => match res {
                PopRes::Selected(selected) => {
                    if let Some(op) = self.temp_content.take() {
                        if let Call::Profile(BackendOp::Profile(ProfileOp::Update(name, _))) = op {
                            let the_choice = match selected {
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
                            self.backend_content = Some(Call::Profile(BackendOp::Profile(
                                ProfileOp::Update(name, the_choice),
                            )));
                        } else {
                            match selected {
                                // if get No, this order is dropped
                                // as it is already moved out by `take`
                                0 => (),
                                // if get Yes, we confirm this order and ready to send it
                                1 => self.backend_content = Some(op),
                                // regarded as extra-choices
                                // ignore others
                                _ => unreachable!(),
                            };
                        }
                        self.popup_content = Some(PopMsg::Prompt(vec!["Working".to_owned()]));
                    }
                }
                PopRes::Input(mut vec) => {
                    match vec.len() {
                        2 => {
                            self.backend_content = Some(Call::Profile(BackendOp::Profile(
                                // there will only be 2 String, using swap_remove is safe
                                ProfileOp::Add(vec.swap_remove(0), vec.swap_remove(0)),
                            )));
                            self.popup_content =
                                Some(PopMsg::Prompt(vec!["Processing".to_owned()]));
                        }
                        1 => self.profiles.set_filter(vec.swap_remove(0)),
                        _ => unimplemented!(),
                    }
                }
                PopRes::Selected_(..) => unreachable!(),
            },
            #[cfg(feature = "template")]
            Focus::Template => match res {
                PopRes::Selected(selected) => {
                    // Remove request
                    if let Some(op) = self.temp_content.take() {
                        match selected {
                            // if get No, this order is dropped
                            // as it is already moved out by `take`
                            0 => (),
                            // if get Yes, we confirm this order and ready to send it
                            1 => self.backend_content = Some(op),
                            // regarded as extra-choices
                            // ignore others
                            _ => unreachable!(),
                        };
                        self.popup_content = Some(PopMsg::Prompt(vec!["Working".to_owned()]));
                    }
                }
                PopRes::Input(mut vec) => {
                    match vec.len() {
                        1 => {
                            if self.temp_content.take().is_some() {
                                // we are trying to import a template
                                self.backend_content = Some(Call::Profile(BackendOp::Template(
                                    TemplateOp::Add(vec.swap_remove(0)),
                                )));
                            } else {
                                self.templates.set_filter(vec.swap_remove(0))
                            }
                        }
                        _ => unimplemented!(),
                    }
                }
                PopRes::Selected_(..) => unreachable!(),
            },
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
                        .map(|index| self.profiles.get_items()[index].clone())
                        .map(ProfileOp::Select)
                        .map(BackendOp::Profile)
                        .map(Call::Profile);
                    self.popup_content = Some(PopMsg::Prompt(vec!["Working".to_owned()]));
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
                        .map(|index| self.templates.get_items()[index].clone())
                        .map(TemplateOp::Generate)
                        .map(BackendOp::Template)
                        .map(Call::Profile);
                    self.popup_content = Some(PopMsg::Prompt(vec!["Working".to_owned()]));
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
