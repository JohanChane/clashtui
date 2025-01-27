use super::*;

impl ProfileTab {
    /// - Catched event -> [EventState::WorkDone]
    /// - unrecognized event -> [EventState::NotConsumed]
    pub(super) fn handle_template_key_event(&mut self, ev: &KeyEvent) -> EventState {
        let name = self
            .templates
            .selected()
            .and_then(|index| self.templates.get_items().get(index).cloned());

        match ev.code.into() {
            Keys::Import => {
                self.popup_content = Some(PopMsg::Input(vec!["Path".to_owned()]));
                // helper for apply_popup_result
                self.temp_content = Some(Call::Profile(BackendOp::Template(TemplateOp::Add(
                    String::new(),
                ))))
            }
            Keys::TemplateSwitch => {
                self.focus = Focus::Profile;
            }
            // place in temp_content and build msg popup content
            Keys::Delete => {
                if let Some(name) = name {
                    self.temp_content =
                        Some(Call::Profile(BackendOp::Template(TemplateOp::Remove(name))));
                    self.popup_content = Some(PopMsg::AskChoices(
                        vec!["Are you sure to delete this?".to_owned()],
                        vec![],
                    ))
                }
            }
            // Keys::ProfileInfo => todo!(),
            Keys::Preview => {
                if let Some(name) = name {
                    self.backend_content = Some(Call::Profile(BackendOp::Template(
                        TemplateOp::Preview(name),
                    )));
                    self.popup_content = Some(PopMsg::Prompt(vec!["Working".to_owned()]));
                }
            }
            Keys::Edit => {
                if let Some(name) = name {
                    self.backend_content =
                        Some(Call::Profile(BackendOp::Template(TemplateOp::Edit(name))));
                    self.popup_content = Some(PopMsg::Prompt(vec!["Working".to_owned()]));
                }
            }
            Keys::Search => {
                self.popup_content = Some(PopMsg::Input(vec!["Name".to_owned()]));
            }
            _ => return EventState::NotConsumed,
        };
        EventState::WorkDone
    }
}
