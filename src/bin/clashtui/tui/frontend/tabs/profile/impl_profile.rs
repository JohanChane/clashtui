use super::*;

impl ProfileTab {
    pub(super) fn handle_profile_key_event(&mut self, ev: &KeyEvent) -> EventState {
        let name = self
            .profiles
            .selected()
            .map(|index| self.profiles.get_items()[index].clone());

        match ev.code.into() {
            Keys::Select => {
                if let Some(name) = name {
                    let pak = Call::Profile(BackendOp::Profile(ProfileOp::Select(name)));
                    self.backend_content.replace(pak);
                }
            }
            Keys::ProfileImport => {
                self.last_focus = self.focus;
                self.focus = Focus::Input;
            }
            #[cfg(feature = "template")]
            Keys::ProfileSwitch => {
                self.focus = Focus::Template;
            }
            // place in temp_content and build msg popup content
            Keys::ProfileDelete => {
                if let Some(name) = name {
                    self.temp_content =
                        Some(Call::Profile(BackendOp::Profile(ProfileOp::Remove(name))));
                    self.popup_content = Some(PopMsg::Ask(
                        vec!["Are you sure to delete this?".to_owned()],
                        None,
                        None,
                    ))
                }
            }
            // place in temp_content and build msg popup content
            Keys::ProfileUpdate => {
                if let Some(name) = name {
                    self.temp_content = Some(Call::Profile(BackendOp::Profile(ProfileOp::Update(
                        name, None,
                    ))));
                    self.popup_content = Some(PopMsg::Ask(
                        vec![
                            "Update Options".to_owned(),
                            "You can decide how to update profile".to_owned(),
                            "The default action is Auto (Currently, without proxy)".to_owned(),
                        ],
                        Some("with proxy".to_owned()),
                        Some("without proxy".to_owned()),
                    ))
                }
            }
            Keys::ProfileUpdateAll => todo!(),
            Keys::ProfileInfo => todo!(),
            Keys::ProfileTestConfig => todo!(),
            Keys::Preview => todo!(),
            Keys::Edit => todo!(),
            _ => return EventState::NotConsumed,
        };
        EventState::WorkDone
    }
}