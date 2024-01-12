pub mod prelude;
mod list;
pub mod style;

pub use self::list::ClashTuiList;
pub use self::style::{SharedTheme, Theme};

#[macro_export]
macro_rules! title_methods {
    ($type:ident) => {
        impl $type {
            pub fn get_title(&self) -> &String {
                &self.title
            }
        }
    };
}

#[macro_export]
macro_rules! visible_methods {
    ($type:ident) => {
        impl $type {
            pub fn is_visible(&self) -> bool {
                self.is_visible
            }
            pub fn show(&mut self) {
                self.is_visible = true;
            }
            pub fn hide(&mut self) {
                self.is_visible = false;
            }
        }
    };
}

#[macro_export]
macro_rules! fouce_methods {
    ($type:ident) => {
        impl $type {
            pub fn is_fouce(&self) -> bool {
                self.is_fouce
            }

            pub fn set_fouce(&mut self, is_fouce: bool) {
                self.is_fouce = is_fouce;
            }
        }
    };
}
