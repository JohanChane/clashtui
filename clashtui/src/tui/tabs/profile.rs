use std::{
    fs::{self, remove_file, OpenOptions},
    io::Write,
    path::Path,
};

use super::profile_input::ProfileInputPopup;
use crate::tui::{
    symbols::{PROFILE, TEMPALTE},
    utils::Keys,
    widgets::{ConfirmPopup, List, MsgPopup},
    EventState, Visibility,
};
use crate::utils::{SharedClashTuiState, SharedClashTuiUtil};
use crate::{msgpopup_methods, utils::get_modify_time};
crate::define_enum!(
    PTOp,
    [
        ProfileUpdate,
        ProfileUpdateAll,
        ProfileSelect,
        ProfileDelete
    ]
);

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
    profile_input: ProfileInputPopup,

    clashtui_util: SharedClashTuiUtil,
    clashtui_state: SharedClashTuiState,
    op: Option<PTOp>,
}

impl ProfileTab {
    pub fn new(clashtui_util: SharedClashTuiUtil, clashtui_state: SharedClashTuiState) -> Self {
        let profiles = List::new(PROFILE.to_string());
        let templates = List::new(TEMPALTE.to_string());

        let mut instance = Self {
            is_visible: true,
            profile_list: profiles,
            template_list: templates,
            msgpopup: Default::default(),
            confirm_popup: ConfirmPopup::new(),
            fouce: Fouce::Profile,
            profile_input: ProfileInputPopup::new(),

            clashtui_util,
            clashtui_state,
            op: None,
        };

        instance.update_profile_list();
        instance
            .profile_list
            .select(instance.clashtui_state.borrow().get_profile());
        let template_names: Vec<String> = instance
            .clashtui_util
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
            if let Err(err) = self.clashtui_util.select_profile(profile_name) {
                self.popup_txt_msg(err.to_string())
            } else {
                self.clashtui_state
                    .borrow_mut()
                    .set_profile(profile_name.clone())
            }
        };
    }
    fn handle_update_profile_ev(&mut self, does_update_all: bool) {
        if let Some(profile_name) = self.profile_list.selected() {
            match self
                .clashtui_util
                .update_local_profile(profile_name, does_update_all)
            {
                Ok(res) => {
                    let mut msg = crate::utils::concat_update_profile_result(res);

                    if profile_name == self.clashtui_state.borrow().get_profile() {
                        if let Err(err) = self.clashtui_util.select_profile(profile_name) {
                            log::error!("{:?}", err);
                            msg.push(err.to_string());
                        } else {
                            msg.push("Update and selected".to_string());
                        }
                    } else {
                        msg.push("Updated".to_string());
                    }

                    self.popup_list_msg(msg);
                }
                Err(err) => {
                    self.popup_txt_msg(format!("Failed to Update: {}", err));
                }
            }
        }
    }
    fn handle_delete_profile_ev(&mut self) {
        if let Some(profile_name) = self.profile_list.selected() {
            match remove_file(self.clashtui_util.get_profile_path_unchecked(profile_name)) {
                Ok(_) => {
                    self.update_profile_list();
                }
                Err(err) => {
                    self.popup_txt_msg(err.to_string());
                }
            }
        }
    }

    fn handle_import_profile_ev(&mut self) {
        let profile_name = self.profile_input.name_input.get_input_data();
        let uri = self.profile_input.uri_input.get_input_data();
        let profile_name = profile_name.trim();
        let uri = uri.trim();

        if uri.is_empty() || profile_name.is_empty() {
            self.popup_txt_msg("Url or Name is empty!".to_string());
            return;
        }

        if uri.starts_with("http://") || uri.starts_with("https://") {
            match OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(self.clashtui_util.get_profile_path_unchecked(profile_name))
            {
                Ok(mut file) => {
                    if let Err(err) = write!(file, "{}", uri) {
                        self.popup_txt_msg(err.to_string());
                    } else {
                        self.update_profile_list();
                    }
                }
                Err(err) => self.popup_txt_msg(err.to_string()),
            }
        } else if Path::new(uri).is_file() {
            let uri_path = Path::new(uri);
            if uri_path.exists() {
                self.popup_txt_msg("Failed to import: file exists".to_string());
                return;
            }
            self.clashtui_util
                .get_profile_yaml_path(profile_name)
                .map_err(|e| self.popup_txt_msg(e.to_string()))
                .into_iter()
                .map_while(|path| fs::copy(Path::new(uri), path).err())
                .for_each(|e| self.popup_txt_msg(e.to_string()));

            self.update_profile_list();
        } else {
            self.popup_txt_msg("Url is invalid.".to_string());
        }
    }

    fn handle_create_template_ev(&mut self) {
        if let Some(template_name) = self.template_list.selected() {
            if let Err(err) = self.clashtui_util.create_yaml_with_template(template_name) {
                self.popup_txt_msg(err.to_string());
            } else {
                self.popup_txt_msg("Created".to_string());
                self.update_profile_list();
            }
        }
    }

    fn update_profile_list(&mut self) {
        let profile_names: Vec<String> = self.clashtui_util.get_profile_names().unwrap();
        if !profile_names
            .iter()
            .filter_map(|v| get_modify_time(v).ok())
            .all(|t| {
                // Within one day
                t > std::time::SystemTime::now() - std::time::Duration::from_secs(24 * 60 * 60)
            })
        {
            self.popup_txt_msg(
                "Some profile might haven't updated for more than one day".to_string(),
            )
        };
        self.profile_list.set_items(profile_names);
    }
}
use crossterm::event::{Event, KeyEventKind};
impl super::TabEvent for ProfileTab {
    fn popup_event(&mut self, ev: &crossterm::event::Event) -> Result<EventState, ui::Infailable> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        let mut event_state = self.msgpopup.event(ev)?;
        if event_state.is_notconsumed() {
            event_state = match self.confirm_popup.event(ev)? {
                EventState::Yes => {
                    self.op.replace(PTOp::ProfileDelete);
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

    fn event(&mut self, ev: &crossterm::event::Event) -> Result<EventState, std::io::Error> {
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
                            self.op.replace(PTOp::ProfileSelect);
                            EventState::WorkDone
                        }
                        Keys::ProfileUpdate => {
                            self.popup_txt_msg("Updating...".to_string());
                            self.op.replace(PTOp::ProfileUpdate);
                            EventState::WorkDone
                        }
                        Keys::ProfileUpdateAll => {
                            self.popup_txt_msg("Updating...".to_string());
                            self.op.replace(PTOp::ProfileUpdateAll);
                            EventState::WorkDone
                        }
                        Keys::ProfileImport => {
                            self.profile_input.show();
                            EventState::WorkDone
                        }
                        Keys::ProfileDelete => {
                            self.confirm_popup
                                .popup_msg("`y` to Delete, `Esc` to cancel".to_string());
                            EventState::WorkDone
                        }
                        Keys::Edit => {
                            // Hmm, now every time I call edit, an window will pop up
                            // even if there is no error. But I think it's fine, maybe
                            // I'll solve it one day
                            self.popup_txt_msg(
                                self.profile_list
                                    .selected()
                                    .into_iter()
                                    .map(|profile_name| {
                                        self.clashtui_util.edit_file(
                                            &self
                                                .clashtui_util
                                                .get_profile_path_unchecked(profile_name),
                                        )
                                    })
                                    .map_while(|r| r.err())
                                    .map(|err| {
                                        log::error!("{}", err);
                                        err.to_string()
                                    })
                                    .collect(),
                            );
                            EventState::WorkDone
                        }
                        Keys::Preview => {
                            if let Some(profile_name) = self.profile_list.selected() {
                                let mut profile_path = Some(
                                    self.clashtui_util.get_profile_path_unchecked(profile_name),
                                );
                                let mut lines: Vec<String> =
                                    std::fs::read_to_string(profile_path.as_ref().unwrap())?
                                        .lines()
                                        .map(|s| s.to_string())
                                        .collect();

                                if !self.clashtui_util.is_profile_yaml(profile_name) {
                                    lines.push(String::new());
                                    profile_path =
                                        self.clashtui_util.get_profile_yaml_path(profile_name).ok();
                                    if let Some(profile_path) = profile_path {
                                        lines.extend(
                                            std::fs::read_to_string(profile_path)?
                                                .lines()
                                                .map(|s| s.to_string()),
                                        );
                                    } else {
                                        lines.push(
                                            "yaml file isn't exists. Please update it.".to_string(),
                                        );
                                    }
                                }
                                self.popup_list_msg(lines);
                            }
                            EventState::WorkDone
                        }
                        Keys::ProfileTestConfig => {
                            if let Some(profile_name) = self.profile_list.selected() {
                                let path =
                                    self.clashtui_util.get_profile_yaml_path(profile_name)?;
                                match self
                                    .clashtui_util
                                    .test_profile_config(path.to_str().unwrap(), false)
                                {
                                    Ok(output) => self.popup_list_msg(
                                        output.lines().map(|line| line.trim().to_string()),
                                    ),
                                    Err(err) => self.popup_txt_msg(err.to_string()),
                                }
                            }
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
                                let path = self.clashtui_util.get_template_path_unchecked(name);
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
                                    .map(|name| {
                                        self.clashtui_util.get_template_path_unchecked(name)
                                    })
                                    .map_while(|tpl_file_path| {
                                        self.clashtui_util.edit_file(&tpl_file_path).err()
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
                PTOp::ProfileUpdate => self.handle_update_profile_ev(false),
                PTOp::ProfileUpdateAll => self.handle_update_profile_ev(true),
                PTOp::ProfileSelect => self.handle_select_profile_ev(),
                PTOp::ProfileDelete => self.handle_delete_profile_ev(),
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

msgpopup_methods!(ProfileTab);
