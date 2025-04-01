use super::*;
use popups::*;

impl ProfileTab {
    /// - Caught event -> [EventState::WorkDone]
    /// - unrecognized event -> [EventState::NotConsumed]
    pub(super) fn handle_template_key_event(&mut self, ev: &KeyEvent) -> EventState {
        let name = self
            .templates
            .selected()
            .and_then(|index| self.templates.get_items().get(index).cloned());

        match ev.code.into() {
            Keys::Import => {
                self.popup_content = Some(PopMsg::new(Import));
            }
            Keys::TemplateSwitch => {
                self.focus = Focus::Profile;
            }
            // place in temp_content and build msg popup content
            Keys::Delete => {
                if let Some(name) = name {
                    self.popup_content = Some(PopMsg::new(Remove::new(name)));
                }
            }
            // Keys::ProfileInfo => todo!(),
            Keys::Preview => {
                if let Some(name) = name {
                    self.backend_content = Some(Call::Profile(BackendOp::Template(
                        TemplateOp::Preview(name),
                    )));
                    self.popup_content = Some(PopMsg::working());
                }
            }
            Keys::Edit => {
                if let Some(name) = name {
                    self.popup_content = Some(PopMsg::new(Edit::new(
                        name,
                        self.profiles.get_items().to_owned(),
                    )));
                }
            }
            Keys::Search => {
                self.popup_content = Some(PopMsg::new(Search));
            }
            _ => return EventState::NotConsumed,
        };
        EventState::WorkDone
    }
}

mod popups {
    use super::*;
    use crate::tui::widget::{Popmsg, PopupState};
    use std::marker::PhantomData;

    gen_order_remove!(|e| BackendOp::Template(TemplateOp::Remove(e)));

    /// * While `M` is `bool`, we ask for what to edit.
    ///   * If just the content or none, we are done here.
    ///   * Otherwise, set `M` to `()` and go next
    /// * Then we ask for which will be used
    /// > FIX ME: here we use `Space` and **NO** color to warn, edit it in popup.rs
    pub struct Edit<M = bool> {
        name: String,
        uses: Vec<String>,
        __marker: PhantomData<M>,
    }
    impl Edit {
        pub fn new(name: String, uses: Vec<String>) -> Self {
            Self {
                name,
                uses,
                __marker: PhantomData,
            }
        }
    }

    impl Popmsg for Edit {
        fn start(&self, pop: &mut crate::tui::widget::Popup) {
            pop.start()
                .clear_all()
                .set_title("Select")
                .set_question("Which you want to edit?")
                .set_choices(
                    ["None", "Uses", "Content"]
                        .into_iter()
                        .map(|s| s.to_owned()),
                )
                .finish();
        }

        fn next(self: Box<Self>, pop: &mut crate::tui::widget::Popup) -> PopupState {
            let Some(PopRes::Selected(idx)) = pop.collect() else {
                unreachable!("Should always be Choices")
            };
            let Self {
                name,
                uses,
                __marker,
            } = *self;
            match idx {
                0 => PopupState::Canceled,
                1 => PopupState::Next(Box::new(Edit::<()> {
                    __marker: PhantomData,
                    name,
                    uses,
                })),
                2 => PopupState::ToBackend(Call::Profile(BackendOp::Template(TemplateOp::Edit(
                    name,
                )))),
                _ => unreachable!(),
            }
        }
    }
    impl Popmsg for Edit<()> {
        fn start(&self, pop: &mut crate::tui::widget::Popup) {
            pop.start()
                .clear_all()
                .set_title("Edit Uses")
                .set_choices(self.uses.iter().cloned())
                .set_multi()
                .finish();
        }

        fn next(self: Box<Self>, pop: &mut crate::tui::widget::Popup) -> PopupState {
            let Some(PopRes::SelectedMulti(selected)) = pop.collect() else {
                unreachable!("Should always be Choices")
            };
            let Self {
                name,
                uses,
                __marker,
            } = *self;
            PopupState::ToBackend(Call::Profile(BackendOp::Template(TemplateOp::Uses(
                name,
                uses.into_iter()
                    .enumerate()
                    .filter_map(|(idx, profile_name)| {
                        selected.contains(&idx).then_some(profile_name)
                    })
                    .collect(),
            ))))
        }
    }

    pub struct Import;
    impl Popmsg for Import {
        fn start(&self, pop: &mut crate::tui::widget::Popup) {
            pop.start()
                .clear_all()
                .set_title("Path")
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
            crate::tui::widget::PopupState::ToBackend(Call::Profile(BackendOp::Template(
                TemplateOp::Add(name),
            )))
        }
    }
}
