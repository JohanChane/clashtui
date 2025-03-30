use super::*;

impl ProfileTab {
    /// - Caught event -> [EventState::WorkDone]
    /// - unrecognized event -> [EventState::NotConsumed]
    pub(super) fn handle_profile_key_event(&mut self, ev: &KeyEvent) -> EventState {
        let name = self
            .profiles
            .selected()
            .and_then(|index| self.profiles.get_items().get(index).cloned());

        match ev.code.into() {
            Keys::Import => {
                self.temp_content = Some(TmpOps::Import);
                self.popup_content = Some(PopMsg::Input("Name".to_owned()));
            }
            #[cfg(feature = "template")]
            Keys::ProfileSwitch => {
                self.focus = Focus::Template;
            }
            // place in temp_content and build msg popup content
            Keys::Delete => {
                if let Some(name) = name {
                    self.temp_content = Some(TmpOps::Remove(name));
                    self.popup_content = Some(PopMsg::AskChoices(
                        "Are you sure to delete this?".to_owned(),
                        vec!["No".to_owned(), "Yes".to_owned()],
                    ))
                }
            }
            // place in temp_content and build msg popup content
            Keys::ProfileUpdate => {
                if let Some(name) = name {
                    self.temp_content = Some(TmpOps::UpdateWithProxy(name));
                    self.popup_content = Some(PopMsg::AskChoices(
                        r#"Update Options
You can decide how to update profile
The default action is Auto (Currently, without proxy)"#
                            .to_owned(),
                        vec![
                            "No".to_owned(),
                            "Yes".to_owned(),
                            "with proxy".to_owned(),
                            "without proxy".to_owned(),
                        ],
                    ))
                }
            }
            // Keys::ProfileUpdateAll => todo!(),
            // Keys::ProfileInfo => todo!(),
            Keys::ProfileTestConfig => {
                if let Some(name) = name {
                    self.backend_content = Some(Call::Profile(BackendOp::Profile(
                        ProfileOp::Test(name, false),
                    )));
                    self.popup_content = Some(PopMsg::Prompt("Working".to_owned()));
                }
            }
            Keys::Preview => {
                if let Some(name) = name {
                    self.backend_content =
                        Some(Call::Profile(BackendOp::Profile(ProfileOp::Preview(name))));
                    self.popup_content = Some(PopMsg::Prompt("Working".to_owned()));
                }
            }
            Keys::Edit => {
                if let Some(name) = name {
                    self.backend_content =
                        Some(Call::Profile(BackendOp::Profile(ProfileOp::Edit(name))));
                    self.popup_content = Some(PopMsg::Prompt("Working".to_owned()));
                }
            }
            Keys::Search => {
                self.temp_content = Some(TmpOps::SetFilter);
                self.popup_content = Some(PopMsg::Input("Name".to_owned()));
            }
            _ => return EventState::NotConsumed,
        };
        EventState::WorkDone
    }
}
