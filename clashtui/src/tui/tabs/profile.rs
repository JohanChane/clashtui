use super::profile_input::ProfileInputPopup;
use crate::tui::{
    impl_app::MonkeyPatch,
    symbols::{PROFILE, TEMPALTE},
    utils::Keys,
    widgets::{ConfirmPopup, List, MsgPopup},
    EventState, Visibility,
};
use crate::utils::{get_modify_time, SharedBackend, SharedState};
crate::utils::define_enum!(PTOp, [Update, UpdateAll, Select, Delete]);
#[derive(PartialEq)]
enum Fouce {
    Profile,
    Template,
}

#[derive(Visibility)]
pub struct ProfileTab {
    is_visible: bool,
    fouce: Fouce,

    profile_list: List,
    template_list: List,
    msgpopup: MsgPopup,
    confirm_popup: ConfirmPopup,
    profile_input: Box<ProfileInputPopup>,

    util: SharedBackend,
    state: SharedState,
    op: Option<PTOp>,
    confirm_op: Option<PTOp>,
}

impl ProfileTab {
    pub fn new(util: SharedBackend, state: SharedState) -> Self {
        let profiles = List::new(PROFILE.to_string());
        let templates = List::new(TEMPALTE.to_string());

        let mut instance = Self {
            is_visible: true,
            profile_list: profiles,
            template_list: templates,
            msgpopup: Default::default(),
            confirm_popup: ConfirmPopup::new(),
            fouce: Fouce::Profile,
            profile_input: ProfileInputPopup::new().into(),

            util,
            state,
            op: None,
            confirm_op: None,
        };

        instance.update_profile_list();
        instance
            .profile_list
            .select(instance.state.borrow().get_profile());
        let template_names: Vec<String> = instance
            .util
            .get_template_names()
            .expect("Unable to init ProfileTab");
        instance.template_list.set_items(template_names);

        instance
    }
    fn switch_fouce(&mut self, fouce: Fouce) {
        self.fouce = fouce;
    }

    fn handle_select_profile_ev(&mut self) {
        if let Some(profile_name) = self.profile_list.selected() {
            if let Err(err) = self.util.select_profile(profile_name) {
                self.popup_txt_msg(err.to_string())
            } else {
                self.state.borrow_mut().set_profile(profile_name.clone())
            }
        };
    }
    fn handle_update_profile_ev(&mut self, does_update_all: bool) {
        if let Some(profile_name) = self.profile_list.selected() {
            match self.util.update_profile(profile_name, does_update_all) {
                Ok(mut msg) => {
                    if profile_name == self.state.borrow().get_profile() {
                        if let Err(err) = self.util.select_profile(profile_name) {
                            log::error!("{profile_name} => {err:?}");
                            msg.push(err.to_string());
                        } else {
                            msg.push("Update and selected".to_string());
                        }
                    } else {
                        msg.push("Update success".to_string());
                    }

                    self.popup_list_msg(msg);
                }
                Err(err) => {
                    log::error!("{profile_name} => {err:?}");
                    self.popup_txt_msg(format!("Failed to Update: {err}"));
                }
            };
            self.update_profile_list();
        }
    }
    fn handle_delete_profile_ev(&mut self) {
        if let Some(profile_name) = self.profile_list.selected() {
            if let Err(e) = self.util.rmf_profile(profile_name) {
                self.popup_txt_msg(e);
            };
            self.state.borrow_mut().refresh();
            self.update_profile_list();
        }
    }

    fn handle_import_profile_ev(&mut self) {
        let profile_name = self.profile_input.name_input.get_input_data();
        let uri = self.profile_input.uri_input.get_input_data();
        match self.util.crt_profile(profile_name, uri) {
            Ok(_) => self.update_profile_list(),
            Err(err) => self.popup_txt_msg(err),
        };
    }

    fn handle_create_template_ev(&mut self) {
        if let Some(template_name) = self.template_list.selected() {
            if let Err(err) = self.util.crt_yaml_with_template(template_name) {
                log::error!("Create Template => {err:?}");
                self.popup_txt_msg(err);
            } else {
                self.popup_txt_msg("Created".to_string());
                self.update_profile_list();
            }
        }
    }

    fn handle_gen_profile_info_ev(&mut self) {
        if let Some(profile_name) = self.profile_list.selected() {
            let is_cur_profile = profile_name == self.clashtui_state.borrow().get_profile();
            match self
                .clashtui_util
                .gen_profile_info(profile_name, is_cur_profile)
            {
                Ok(info) => {
                    self.popup_list_msg(info);
                }
                Err(e) => {
                    self.popup_txt_msg(e.to_string());
                }
            }
        }
    }

    fn update_profile_list(&mut self) {
        let profile_names = self.util.get_profile_names().unwrap();
        let profile_times: Vec<Option<std::time::SystemTime>> = profile_names
            .iter()
            .map(|v| {
                self.util
                    .get_profile_yaml(v)
                    .and_then(get_modify_time)
                    .map_err(|e| log::error!("{v} => {e}"))
                    .ok()
            })
            .collect();
        let now = std::time::SystemTime::now();
        self.profile_list.set_items(profile_names);
        self.profile_list
            .set_extras(profile_times.into_iter().map(|t| {
                t.map(|t| {
                    utils::str_duration(
                        now.duration_since(t)
                            .expect("Clock may have gone backwards"),
                    )
                })
                .unwrap_or("Never/Err".to_string())
            }))
    }
}
use ui::event::{Event, KeyEventKind};
impl super::TabEvent for ProfileTab {
    fn popup_event(&mut self, ev: &Event) -> Result<EventState, ui::Infailable> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        let mut event_state = self.msgpopup.event(ev)?;
        if event_state.is_notconsumed() {
            event_state = match self.confirm_popup.event(ev)? {
                EventState::Yes => {
                    self.op = self.confirm_op.take();
                    EventState::WorkDone
                }
                EventState::Cancel | EventState::WorkDone => EventState::WorkDone,
                _ => EventState::NotConsumed,
            };
        }
        if event_state.is_notconsumed() {
            event_state = self.profile_input.event(ev)?;

            if event_state == EventState::WorkDone {
                if let Event::Key(key) = ev {
                    if key.kind != KeyEventKind::Press {
                        return Ok(EventState::NotConsumed);
                    }
                    if &Keys::Select == key {
                        self.handle_import_profile_ev();
                    }
                }
            }
        }

        Ok(event_state)
    }

    fn event(&mut self, ev: &Event) -> Result<EventState, std::io::Error> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        let mut event_state = EventState::NotConsumed;

        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                return Ok(EventState::NotConsumed);
            }

            match self.fouce {
                Fouce::Profile => {
                    event_state = match key.code.into() {
                        Keys::TemplateSwitch => {
                            self.switch_fouce(Fouce::Template);
                            EventState::WorkDone
                        }
                        Keys::Select => {
                            self.popup_txt_msg("Selecting...".to_string());
                            self.op.replace(PTOp::Select);
                            EventState::WorkDone
                        }
                        Keys::ProfileUpdate => {
                            self.popup_txt_msg("Updating...".to_string());
                            self.op.replace(PTOp::Update);
                            EventState::WorkDone
                        }
                        Keys::ProfileUpdateAll => {
                            self.popup_txt_msg("Updating...".to_string());
                            self.op.replace(PTOp::UpdateAll);
                            EventState::WorkDone
                        }
                        Keys::ProfileImport => {
                            self.profile_input.show();
                            EventState::WorkDone
                        }
                        Keys::ProfileDelete => {
                            self.confirm_popup
                                .popup_msg("`y` to Delete, `Esc` to cancel".to_string());
                            self.confirm_op.replace(PTOp::Delete);
                            EventState::WorkDone
                        }
                        Keys::Edit => {
                            // Hmm, now every time I call edit, an window will pop up
                            // even if there is no error. But I think it's fine, maybe
                            // I'll solve it one day
                            //
                            // I fix it, because msg is empty.
                            self.popup_txt_msg(
                                self.profile_list
                                    .selected()
                                    .into_iter()
                                    .map(|profile_name| {
                                        self.util
                                            .edit_file(&self.util.gen_profile_path(profile_name))
                                    })
                                    .map_while(|r| r.err())
                                    .map(|err| {
                                        log::error!("{err}");
                                        err.to_string()
                                    })
                                    .collect(),
                            );
                            EventState::WorkDone
                        }
                        Keys::Preview => {
                            if let Some(profile_name) = self.profile_list.selected() {
                                let mut lines: Vec<String> = vec![];
                                if self.util.is_upgradable(profile_name) {
                                    lines.push(
                                        self.util
                                            .get_profile_link(profile_name)
                                            .expect("have a key but no value"),
                                    );
                                } else {
                                    lines.push("Imported local file".to_owned());
                                }
                                lines.push(String::new());
                                let content: Vec<String> = std::fs::read_to_string(
                                    self.util.gen_profile_path(profile_name),
                                )?
                                .lines()
                                .map(|s| s.to_string())
                                .collect();
                                if !content.is_empty() {
                                    lines.extend(content);
                                } else {
                                    lines.push(
                                        "yaml file isn't exists. Please update it.".to_string(),
                                    );
                                }

                                self.popup_list_msg(lines);
                            }
                            EventState::WorkDone
                        }
                        Keys::ProfileTestConfig => {
                            if let Some(profile_name) = self.profile_list.selected() {
                                let path = self.util.get_profile_yaml(profile_name)?;
                                match self.util.test_profile_config(path.to_str().unwrap(), false) {
                                    Ok(output) => self.popup_list_msg(
                                        output.lines().map(|line| line.trim().to_string()),
                                    ),
                                    Err(err) => self.popup_txt_msg(err.to_string()),
                                }
                            }
                            EventState::WorkDone
                        }
                        Keys::ProfileInfo => {
                            self.popup_txt_msg("Generating info...".to_string());
                            self.op.replace(PTOp::GenInfo);
                            EventState::WorkDone
                        }
                        Keys::ProfileNoPp => {
                            self.clashtui_state.borrow_mut().switch_no_pp();
                            EventState::WorkDone
                        }
                        _ => EventState::NotConsumed,
                    };
                }
                Fouce::Template => {
                    event_state = match key.code.into() {
                        Keys::ProfileSwitch => {
                            self.switch_fouce(Fouce::Profile);
                            EventState::WorkDone
                        }
                        Keys::Select => {
                            self.handle_create_template_ev();
                            EventState::WorkDone
                        }
                        Keys::Preview => {
                            if let Some(name) = self.template_list.selected() {
                                let path = self.util.gen_template_path(name);
                                self.popup_list_msg(
                                    std::fs::read_to_string(path)?
                                        .lines()
                                        .map(|s| s.to_string()),
                                );
                            }
                            EventState::WorkDone
                        }
                        Keys::Edit => {
                            self.popup_txt_msg(
                                self.template_list
                                    .selected()
                                    .into_iter()
                                    .map(|name| self.util.gen_template_path(name))
                                    .map_while(|tpl_file_path| {
                                        self.util.edit_file(&tpl_file_path).err()
                                    })
                                    .map(|err| err.to_string())
                                    .collect(),
                            );
                            EventState::WorkDone
                        }
                        _ => EventState::NotConsumed,
                    };
                }
            }

            if event_state == EventState::NotConsumed {
                event_state = match self.fouce {
                    Fouce::Profile => self.profile_list.event(ev),
                    Fouce::Template => self.template_list.event(ev),
                }?;
            }
        }

        Ok(event_state)
    }
    fn late_event(&mut self) {
        if let Some(op) = self.op.take() {
            self.hide_msgpopup();
            match op {
                PTOp::Update => self.handle_update_profile_ev(false),
                PTOp::UpdateAll => self.handle_update_profile_ev(true),
                PTOp::Select => self.handle_select_profile_ev(),
                PTOp::Delete => self.handle_delete_profile_ev(),
                PTOp::GenInfo => self.handle_gen_profile_info_ev(),
            }
        }
    }
    fn draw(&mut self, f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect) {
        if !self.is_visible() {
            return;
        }
        use ratatui::prelude::{Constraint, Layout};

        let chunks = Layout::default()
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        let fouce = self.fouce == Fouce::Profile;
        self.profile_list.draw(f, chunks[0], fouce);
        self.template_list.draw(f, chunks[1], !fouce);

        let input_area = Layout::default()
            .constraints([
                Constraint::Percentage(25),
                Constraint::Length(8),
                Constraint::Min(0),
            ])
            .horizontal_margin(10)
            .vertical_margin(1)
            .split(f.size())[1];

        self.profile_input.draw(f, input_area);
        self.msgpopup.draw(f, area);
        self.confirm_popup.draw(f, area);
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
crate::msgpopup_methods!(ProfileTab);
