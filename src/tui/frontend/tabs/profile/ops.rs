use crate::tui::widget::{PopRes, Popmsg, PopupState};

#[derive(Debug)]
pub enum BackendOp {
    Profile(ProfileOp),
    #[cfg(feature = "template")]
    Template(TemplateOp),
}
#[derive(Debug)]
pub enum ProfileOp {
    /// Send after [ProfileOp::Update],[ProfileOp::Add],[ProfileOp::Remove]
    /// and [TemplateOp::Generate] if enable template feature
    GetALL,
    /// go without ask
    Select(String),
    /// go without ask
    Add(String, String),
    /// ask for two option(`Yes/No`)
    Remove(String),
    /// ask for three option(`Yes/No`)
    Update(String, bool, bool),
    /// test the profile content
    ///
    /// > I don't really know what `geodata_mode` can do,
    /// > but I'll keep it
    Test(String, bool),
    /// ask for preview
    ///
    /// though this is asked by tab, but it will be handled at frontend
    Preview(String),
    /// ask for edit
    ///
    /// though this is asked by tab, but it will be handled at frontend
    Edit(String),
}
#[cfg(feature = "template")]
#[derive(Debug)]
pub enum TemplateOp {
    /// Send after [TemplateOp::Add],[TemplateOp::Remove]
    GetALL,
    /// go without ask
    Generate(String),
    /// go without ask
    Add(String),
    /// ask for two option(`Yes/No`)
    Remove(String),
    /// ask for preview
    ///
    /// though this is asked by tab, but it will be handled at frontend
    Preview(String),
    /// ask for edit
    ///
    /// though this is asked by tab, but it will be handled at frontend
    Edit(String),
    Uses(String, Vec<String>),
}

pub struct Search;
impl Popmsg for Search {
    fn start(&self, pop: &mut crate::tui::widget::Popup) {
        pop.start()
            .clear_all()
            .set_title("Name")
            .with_input()
            .finish()
    }

    fn next(self: Box<Self>, pop: &mut crate::tui::widget::Popup) -> PopupState {
        let Some(PopRes::Input(vec)) = pop.collect() else {
            unreachable!("Should always be Input")
        };
        PopupState::ToFrontend(PopRes::Input(vec))
    }
}
macro_rules! gen_order_remove {
    ($e:expr) => {
        pub struct Remove {
            name: String,
        }
        impl Remove {
            pub fn new(name: String) -> Self {
                Self { name }
            }
        }

        impl Popmsg for Remove {
            fn start(&self, pop: &mut crate::tui::widget::Popup) {
                pop.start()
                    .clear_all()
                    .set_title("Warning")
                    .set_question("Are you sure to delete this?")
                    .set_choices(["No", "Yes"].into_iter().map(|s| s.to_owned()))
                    .finish();
            }

            fn next(self: Box<Self>, pop: &mut crate::tui::widget::Popup) -> PopupState {
                let Self { name } = *self;
                let Some(PopRes::Selected(idx)) = pop.collect() else {
                    unreachable!("Should always be Choices")
                };
                match idx {
                    0 => PopupState::Canceled,
                    1 => PopupState::ToBackend(Call::Profile($e(name))),
                    _ => unreachable!(),
                }
            }
        }
    };
}
