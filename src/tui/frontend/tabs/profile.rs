mod impl_profile;
mod input;
mod ops;

use crossterm::event::KeyEvent;
use input::InputPopup;
pub use ops::*;

use crate::{
    tui::{
        frontend::{consts::TAB_TITLE_PROFILE, key_bind::Keys},
        widget::List,
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
    Input,
}
pub(in crate::tui::frontend) struct ProfileTab {
    profiles: List,
    #[cfg(feature = "template")]
    templates: List,
    focus: Focus,
    /// only used for return to last focus after the input popup closes
    last_focus: Focus,
    popup_content: Option<PopMsg>,
    backend_content: Option<Call>,
    /// hold content for msg ask
    temp_content: Option<Call>,
    is_profiles_inited: bool,
    #[cfg(feature = "template")]
    is_templates_inited: bool,
    input_popup: std::cell::OnceCell<InputPopup>,
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
            last_focus: Focus::Profile,
            popup_content: None,
            backend_content: None,
            temp_content: None,
            is_profiles_inited: false,
            #[cfg(feature = "template")]
            is_templates_inited: false,
            input_popup: std::cell::OnceCell::new(),
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
            CallBack::TemplateInit(content) => {
                if !self.is_templates_inited {
                    self.templates.set_items(content);
                    self.is_templates_inited = true;
                }
            }
            _ => unreachable!("{} get unexpected op: {:?}", TAB_TITLE_PROFILE, op),
        }
    }

    fn apply_popup_result(&mut self, evst: EventState) -> EventState {
        if let Some(op) = self.temp_content.take() {
            if let Call::Profile(BackendOp::Profile(ProfileOp::Update(name, _))) = op {
                let the_choice = match evst {
                    // if get Yes, we confirm this order and ready to send it
                    EventState::Yes => Some(true),
                    EventState::Choice2 => Some(false),
                    EventState::Choice3 => None,
                    // if get No, this order is dropped
                    // as it is already moved out by `take`
                    EventState::Cancel => return EventState::WorkDone,
                    // ignore others
                    EventState::NotConsumed | EventState::WorkDone => {
                        return EventState::NotConsumed
                    }
                };
                self.backend_content = Some(Call::Profile(BackendOp::Profile(ProfileOp::Update(
                    name, the_choice,
                ))));
            } else {
                match evst {
                    // if get Yes, we confirm this order and ready to send it
                    EventState::Yes => self.backend_content = Some(op),
                    // if get No, this order is dropped
                    // as it is already moved out by `take`
                    EventState::Cancel => (),
                    // ignore others
                    EventState::Choice2
                    | EventState::Choice3
                    | EventState::NotConsumed
                    | EventState::WorkDone => return EventState::NotConsumed,
                };
            }
            self.popup_content = Some(PopMsg::Prompt(vec!["Working".to_owned()]));
        } else {
            // apply for [PopMsg::Prompt]
            match evst {
                EventState::Yes | EventState::Cancel => (),
                EventState::Choice2
                | EventState::Choice3
                | EventState::WorkDone
                | EventState::NotConsumed => return EventState::NotConsumed,
            };
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
        if self.focus == Focus::Input {
            if let Some(ip) = self.input_popup.get_mut() {
                ip.render(f, area, true);
            } else {
                // this is called very first
                let _ = self.input_popup.set(Default::default());
                self.input_popup.get_mut().unwrap().render(f, area, true);
            }
        }
    }
    // call [`TabCont::apply_popup_result`] first
    fn handle_key_event(&mut self, ev: &KeyEvent) -> EventState {
        match self.focus {
            Focus::Profile => {
                if self.profiles.handle_key_event(ev) == EventState::Yes {
                    let name = self
                        .profiles
                        .selected()
                        .map(|index| self.profiles.get_items()[index].clone());
                    if let Some(name) = name {
                        let pak = Call::Profile(BackendOp::Profile(ProfileOp::Select(name)));
                        self.backend_content = Some(pak);
                        self.popup_content = Some(PopMsg::Prompt(vec!["Working".to_owned()]));
                    }
                    EventState::WorkDone
                } else {
                    self.handle_profile_key_event(ev)
                }
            }
            #[cfg(feature = "template")]
            Focus::Template => {
                if self.templates.handle_key_event(ev) == EventState::Yes {
                    // Enter means select
                    todo!("build op")
                } else {
                    match ev.code.into() {
                        Keys::Preview => todo!(),
                        Keys::TemplateSwitch => self.focus = Focus::Profile,
                        Keys::Edit => todo!(),
                        _ => return EventState::NotConsumed,
                    };
                    EventState::WorkDone
                }
            }
            Focus::Input => {
                let ip = self.input_popup.get_mut().unwrap();
                let evst = ip.handle_key_event(ev);
                match evst {
                    EventState::Yes => {
                        let (name, url) = ip.get_name_url();
                        self.backend_content =
                            Some(Call::Profile(BackendOp::Profile(ProfileOp::Add(name, url))));
                        self.popup_content = Some(PopMsg::Prompt(vec!["Processing".to_owned()]));
                        self.focus = self.last_focus
                    }
                    EventState::Cancel => self.focus = self.last_focus,
                    EventState::NotConsumed
                    | EventState::WorkDone
                    | EventState::Choice2
                    | EventState::Choice3 => (),
                }
                evst
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
