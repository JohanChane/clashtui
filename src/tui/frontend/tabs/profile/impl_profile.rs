use super::*;
use popups::*;

impl ProfileTab {
    pub(super) fn handle_profile_key_event(&mut self, ev: &KeyEvent) -> EventState {
        let name = self
            .profiles
            .selected()
            .and_then(|index| self.profiles.get_items().get(index).cloned());

        match ev.code.into() {
            Keys::Import => {
                self.popup_content = Some(PopMsg::new(Import::new()));
            }
            #[cfg(feature = "template")]
            Keys::ProfileSwitch => {
                self.focus = Focus::Template;
            }
            // place in temp_content and build msg popup content
            Keys::Delete => {
                if let Some(name) = name {
                    self.popup_content = Some(PopMsg::new(Remove::new(name)));
                }
            }
            // place in temp_content and build msg popup content
            Keys::ProfileUpdate => {
                if let Some(name) = name {
                    self.popup_content = Some(PopMsg::new(Update::new(name)));
                }
            }
            // Keys::ProfileUpdateAll => todo!(),
            // Keys::ProfileInfo => todo!(),
            Keys::ProfileTestConfig => {
                if let Some(name) = name {
                    self.backend_content = Some(Call::Profile(BackendOp::Profile(
                        ProfileOp::Test(name, false),
                    )));
                    self.popup_content = Some(PopMsg::working());
                }
            }
            Keys::Preview => {
                if let Some(name) = name {
                    self.backend_content =
                        Some(Call::Profile(BackendOp::Profile(ProfileOp::Preview(name))));
                    self.popup_content = Some(PopMsg::working());
                }
            }
            Keys::Edit => {
                if let Some(name) = name {
                    self.backend_content =
                        Some(Call::Profile(BackendOp::Profile(ProfileOp::Edit(name))));
                    self.popup_content = Some(PopMsg::working());
                }
            }
            Keys::Search => {
                self.popup_content = Some(PopMsg::new(Search));
            }
            _ => return EventState::NotConsumed,
        };
        EventState::Consumed
    }
}

mod popups {
    use std::marker::PhantomData;

    use super::*;
    use crate::tui::widget::{Popmsg, PopupState};

    gen_order_remove!(|e| BackendOp::Profile(ProfileOp::Remove(e)));

    /// * While `S` is `()`, we ask for the name of profile
    /// * Then, `S` set to `String` and we ask for the path/url
    pub struct Import<S = ()> {
        name: S,
    }
    impl Import {
        pub fn new() -> Self {
            Self { name: () }
        }
    }

    impl Popmsg for Import {
        fn start(&self, pop: &mut crate::tui::widget::Popup) {
            pop.start()
                .clear_all()
                .set_title("Name")
                .with_input()
                .finish()
        }

        fn next(
            self: Box<Self>,
            pop: &mut crate::tui::widget::Popup,
        ) -> crate::tui::widget::PopupState {
            let Some(PopRes::Input(name)) = pop.collect() else {
                unreachable!("Should always be Input")
            };
            PopupState::Next(Box::new(Import { name }))
        }
    }
    impl Popmsg for Import<String> {
        fn start(&self, pop: &mut crate::tui::widget::Popup) {
            pop.start()
                .clear_all()
                .set_title("Url")
                .with_input()
                .finish()
        }

        fn next(
            self: Box<Self>,
            pop: &mut crate::tui::widget::Popup,
        ) -> crate::tui::widget::PopupState {
            let Self { name, .. } = *self;
            let Some(PopRes::Input(path_or_url)) = pop.collect() else {
                unreachable!("Should always be Input")
            };
            crate::tui::widget::PopupState::ToBackend(Call::Profile(BackendOp::Profile(
                ProfileOp::Add(name, path_or_url),
            )))
        }
    }

    /// * While `Default` is `bool`, we ask weather to carry on with default choices
    /// * Then `Default` is set to `()`, we ask for the whole choices
    ///   * While `B` is `()`, we ask `with_proxy`
    ///   * Then `B` is `bool`, we ask `no_pp`
    ///
    /// `new` will create one that goes from the first to the last
    pub struct Update<Default = bool, B = ()> {
        name: String,
        with_proxy: B,
        __marker: PhantomData<Default>,
    }
    impl Update {
        pub fn new(name: String) -> Self {
            Self {
                name,
                with_proxy: (),
                __marker: PhantomData,
            }
        }
    }

    impl Popmsg for Update {
        fn start(&self, pop: &mut crate::tui::widget::Popup) {
            pop.start()
                .clear_all()
                .set_title("Update Options")
                .set_question("The default action is without proxy and keeping proxy-providers")
                .set_choices(
                    ["Accept", "Edit", "Cancel"]
                        .into_iter()
                        .map(|s| s.to_owned()),
                )
                .finish();
        }

        fn next(self: Box<Self>, pop: &mut crate::tui::widget::Popup) -> PopupState {
            let Self { name, .. } = *self;
            let Some(PopRes::Selected(idx)) = pop.collect() else {
                unreachable!()
            };
            match idx {
                0 => PopupState::ToBackend(Call::Profile(BackendOp::Profile(ProfileOp::Update(
                    name, false, false,
                )))),
                1 => PopupState::Next(Box::new(Update {
                    name,
                    with_proxy: (),
                    __marker: PhantomData::<()>,
                })),
                2 => PopupState::Canceled,
                _ => unreachable!(),
            }
        }
    }
    impl Popmsg for Update<()> {
        fn start(&self, pop: &mut crate::tui::widget::Popup) {
            pop.start()
                .clear_all()
                .set_title("Update Options")
                .set_question("You can decide how to update profile.")
                .set_choices(
                    ["Cancel", "with proxy", "without proxy"]
                        .into_iter()
                        .map(|s| s.to_owned()),
                )
                .finish();
        }

        fn next(self: Box<Self>, pop: &mut crate::tui::widget::Popup) -> PopupState {
            let Self { name, .. } = *self;
            let Some(PopRes::Selected(idx)) = pop.collect() else {
                unreachable!()
            };
            let with_proxy = match idx {
                0 => return PopupState::Canceled,
                1 => true,
                2 => false,
                _ => unreachable!(),
            };

            PopupState::Next(Box::new(Update {
                name,
                with_proxy,
                __marker: PhantomData,
            }))
        }
    }
    impl Popmsg for Update<(), bool> {
        fn start(&self, pop: &mut crate::tui::widget::Popup) {
            pop.start()
                .clear_all()
                .set_title("Merge proxy-provider")
                .set_question("Skip proxy-provider merging?\nWhich is 'no_pp'")
                .set_choices(["No", "Yes"].into_iter().map(|s| s.to_owned()))
                .finish();
        }

        fn next(self: Box<Self>, pop: &mut crate::tui::widget::Popup) -> PopupState {
            let Self {
                name, with_proxy, ..
            } = *self;
            let Some(PopRes::Selected(idx)) = pop.collect() else {
                unreachable!()
            };
            let no_pp = match idx {
                0 => true,
                1 => false,
                _ => unreachable!(),
            };
            PopupState::ToBackend(Call::Profile(BackendOp::Profile(ProfileOp::Update(
                name, with_proxy, no_pp,
            ))))
        }
    }
}
