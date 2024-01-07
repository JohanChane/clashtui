use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::{
    fs::{remove_file, OpenOptions},
    io::Write,
};

use super::CommonTab;
use super::keys::{match_key, SharedKeyList};
use crate::ui::widgets::{ClashTuiList, SharedTheme};
use crate::ui::EventState;
use crate::ui::SharedSymbols;
use crate::ui::{ConfirmPopup, MsgPopup, ProfileInputPopup};
use crate::utils::SharedClashTuiState;
use crate::utils::{ClashTuiUtil, SharedClashTuiUtil};
use crate::{msgpopup_methods, title_methods, visible_methods};

pub enum Fouce {
    Profile,
    Template,
}

pub struct ProfileTab {
    title: String,
    is_visible: bool,
    fouce: Fouce,

    pub profile_list: ClashTuiList,
    pub template_list: ClashTuiList,
    msgpopup: MsgPopup,
    confirm_popup: ConfirmPopup,
    profile_input: ProfileInputPopup,

    pub key_list: SharedKeyList,
    pub symbols: SharedSymbols,
    pub clashtui_util: SharedClashTuiUtil,
    pub clashtui_state: SharedClashTuiState,

    pub input_name: String,
    pub input_uri: String,
}

impl ProfileTab {
    pub fn new(
        title: String,
        key_list: SharedKeyList,
        symbols: SharedSymbols,
        clashtui_util: SharedClashTuiUtil,
        clashtui_state: SharedClashTuiState,
        theme: SharedTheme,
    ) -> Self {
        let profiles = ClashTuiList::new(symbols.profile.clone(), Rc::clone(&theme));
        let templates = ClashTuiList::new(symbols.template.clone(), Rc::clone(&theme));

        let mut instance = Self {
            title,
            is_visible: true,
            profile_list: profiles,
            template_list: templates,
            msgpopup: MsgPopup::new(),
            confirm_popup: ConfirmPopup::new(),
            fouce: Fouce::Profile,
            profile_input: ProfileInputPopup::new(),

            key_list,
            symbols,
            clashtui_util,
            clashtui_state,

            input_name: String::new(),
            input_uri: String::new(),
        };

        instance.update_profile_list();
        instance
            .profile_list
            .select(instance.clashtui_state.borrow().get_profile());
        let template_names: Vec<String> = instance.clashtui_util.get_template_names().unwrap();
        instance.template_list.set_items(template_names);

        instance.switch_fouce(Fouce::Profile);

        instance
    }

    pub fn popup_event(&mut self, ev: &Event) -> Result<EventState> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        let mut event_state = self.msgpopup.event(ev)?;
        if event_state.is_notconsumed() {
            event_state = self.confirm_popup.event(ev)?;
        }
        if event_state.is_notconsumed() {
            event_state = self.profile_input.event(ev)?;

            if event_state == EventState::WorkDone {
                if let Event::Key(key) = ev {
                    if key.kind != KeyEventKind::Press {
                        return Ok(EventState::NotConsumed);
                    }

                    match key.code {
                        KeyCode::Enter => {
                            self.handle_import_profile_ev();
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(event_state)
    }

    pub fn switch_fouce(&mut self, fouce: Fouce) {
        self.profile_list.set_fouce(false);
        self.template_list.set_fouce(false);

        match fouce {
            Fouce::Profile => {
                self.profile_list.set_fouce(true);
            }
            Fouce::Template => {
                self.template_list.set_fouce(true);
            }
        }
        self.fouce = fouce;
    }

    pub fn handle_select_profile_ev(&mut self) -> Option<String> {
        if let Some(profile_name) = self.profile_list.selected() {
            if let Err(err) = self.clashtui_util.select_profile(profile_name) {
                self.popup_txt_msg(err.to_string());
                None
            } else {
                Some(profile_name.to_string())
            }
        } else {
            None
        }
    }
    pub fn handle_update_profile_ev(&mut self, does_update_all: bool) {
        if let Some(profile_name) = self.profile_list.selected() {
            match self
                .clashtui_util
                .update_profile(profile_name, does_update_all)
            {
                Ok(res) => {
                    let mut msg = ClashTuiUtil::concat_update_profile_result(res);

                    if profile_name == self.clashtui_state.borrow_mut().get_profile() {
                        if let Err(err) = self.clashtui_util.select_profile(profile_name) {
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
                    self.popup_txt_msg(format!("Failed to Update: {}", err.to_string()));
                }
            }
        }
    }
    pub fn handle_delete_profile_ev(&mut self) {
        if let Some(profile_name) = self.profile_list.selected() {
            match remove_file(self.clashtui_util.profile_dir.join(profile_name)) {
                Ok(()) => {
                    self.update_profile_list();
                }
                Err(err) => {
                    self.popup_txt_msg(err.to_string());
                }
            }
        }
    }

    pub fn handle_import_profile_ev(&mut self) {
        let profile_name = self.profile_input.name_input.get_input_data();
        let uri = self.profile_input.uri_input.get_input_data();
        let profile_name = profile_name.trim();
        let uri = uri.trim();

        if uri.is_empty() || profile_name.is_empty() {
            self.popup_txt_msg("Uri or Name is empty!".to_string());
            return;
        }

        if uri.starts_with("http://") || uri.starts_with("https://") {
            match OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(self.clashtui_util.profile_dir.join(profile_name))
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

            let _ = fs::copy(
                Path::new(uri),
                Path::new(
                    &self
                        .clashtui_util
                        .profile_dir
                        .join(Path::new(profile_name).with_extension("yaml")),
                ),
            );
            self.update_profile_list();
        } else {
            self.popup_txt_msg("Uri is invalid.".to_string());
        }
    }

    pub fn handle_create_template_ev(&mut self) {
        if let Some(template_name) = self.template_list.selected() {
            if let Err(err) = self.clashtui_util.create_yaml_with_template(template_name) {
                self.popup_txt_msg(err.to_string());
            } else {
                self.popup_txt_msg("Created".to_string());
                self.update_profile_list();
            }
        }
    }

    pub fn update_profile_list(&mut self) {
        let profile_names: Vec<String> = self.clashtui_util.get_profile_names().unwrap();
        self.profile_list.set_items(profile_names);
    }
}

impl CommonTab for ProfileTab {
    
    fn event(&mut self, ev: &Event) -> Result<EventState, ()> {
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
                    event_state = if match_key(key, &self.key_list.template_switch) {
                        self.switch_fouce(Fouce::Template);
                        EventState::WorkDone
                    } else if match_key(key, &self.key_list.profile_select) {
                        self.popup_txt_msg("Selecting...".to_string());
                        EventState::ProfileSelect
                    } else if match_key(key, &self.key_list.profile_update) {
                        self.popup_txt_msg("Updating...".to_string());
                        EventState::ProfileUpdate
                    } else if match_key(key, &self.key_list.profile_update_all) {
                        self.popup_txt_msg("Updating...".to_string());
                        EventState::ProfileUpdateAll
                    } else if match_key(key, &self.key_list.profile_import) {
                        self.profile_input.show();
                        EventState::WorkDone
                    } else if match_key(key, &self.key_list.profile_delete) {
                        self.confirm_popup.popup_msg(
                            EventState::ProfileDelete,
                            "`y` to Delete, `Esc` to cancel".to_string(),
                        );
                        EventState::WorkDone
                    //} else if match_key(key, &self.key_list.edit) {
                    //    if let Some(profile_name) = self.profile_list.selected() {
                    //        if let Err(err) = self
                    //            .clashtui_util
                    //            .edit_file(&self.clashtui_util.profile_dir.join(profile_name))
                    //        {
                    //            log::error!("{}", err);
                    //            self.popup_txt_msg(err.to_string());
                    //        }
                    //    }
                    //    EventState::WorkDone
                    } else if match_key(key, &self.key_list.preview) {
                        if let Some(profile_name) = self.profile_list.selected() {
                            let profile_path = self.clashtui_util.profile_dir.join(profile_name);
                            let file_content = std::fs::read_to_string(&profile_path).unwrap();
                            let mut lines: Vec<String> =
                                file_content.lines().map(|s| s.to_string()).collect();

                            if !self.clashtui_util.is_profile_yaml(profile_name) {
                                let yaml_path =
                                    self.clashtui_util.get_profile_yaml_path(profile_name);
                                if yaml_path.is_file() {
                                    let yaml_content = std::fs::read_to_string(&yaml_path).unwrap();
                                    let yaml_lines: Vec<String> =
                                        yaml_content.lines().map(|s| s.to_string()).collect();
                                    lines.push("".to_string());
                                    lines.extend(yaml_lines);
                                } else {
                                    lines.push("".to_string());
                                    lines.push(
                                        "yaml file isn't exists. Please update it.".to_string(),
                                    );
                                }
                            }

                            self.popup_list_msg(lines);
                        }
                        EventState::WorkDone
                    } else if match_key(key, &self.key_list.profile_test_config) {
                        if let Some(profile_name) = self.profile_list.selected() {
                            let path = self.clashtui_util.get_profile_yaml_path(profile_name);
                            match self.clashtui_util.test_profile_config(&path, false) {
                                Ok(output) => {
                                    let list_msg: Vec<String> = output
                                        .lines()
                                        .map(|line| line.trim().to_string())
                                        .collect();
                                    self.popup_list_msg(list_msg);
                                }
                                Err(err) => self.popup_txt_msg(err.to_string()),
                            }
                        }
                        EventState::WorkDone
                    } else {
                        EventState::NotConsumed
                    };
                }
                Fouce::Template => {
                    event_state = if match_key(key, &self.key_list.profile_switch) {
                        self.switch_fouce(Fouce::Profile);
                        EventState::WorkDone
                    } else if match_key(key, &self.key_list.template_create) {
                        self.handle_create_template_ev();
                        EventState::WorkDone
                    } else if match_key(key, &self.key_list.preview) {
                        if let Some(name) = self.template_list.selected() {
                            let path = self
                                .clashtui_util
                                .clashtui_dir
                                .join(format!("templates/{}", name));
                            let content = std::fs::read_to_string(&path).unwrap();
                            let lines: Vec<String> =
                                content.lines().map(|s| s.to_string()).collect();

                            self.popup_list_msg(lines);
                        }
                        EventState::WorkDone
                    //} else if match_key(key, &self.key_list.edit) {
                    //    if let Some(name) = self.template_list.selected() {
                    //        let tpl_file_path = self
                    //            .clashtui_util
                    //            .clashtui_dir
                    //            .join(format!("templates/{}", name));
                    //        if let Err(err) = self.clashtui_util.edit_file(&tpl_file_path) {
                    //            self.popup_txt_msg(err.to_string());
                    //        }
                    //    }
                    //    EventState::WorkDone
                    } else {
                        EventState::NotConsumed
                    };
                }
            }

            if event_state == EventState::NotConsumed {
                event_state = self.profile_list.event(ev).unwrap();
                if event_state.is_notconsumed() {
                    event_state = self.template_list.event(ev).unwrap();
                }
            }
        }

        Ok(event_state)
    }

    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        if !self.is_visible() {
            return;
        }

        let chunks = Layout::default()
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        self.profile_list.draw(f, chunks[0]);
        self.template_list.draw(f, chunks[1]);

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
        self.msgpopup.draw(f);
        self.confirm_popup.draw(f);
    }

}

title_methods!(ProfileTab);
visible_methods!(ProfileTab);
msgpopup_methods!(ProfileTab);
