mod key_list;
mod list;
pub mod symbols;
mod theme;
pub mod tools;

pub use self::key_list::Keys;
pub use self::list::ClashTuiList;
pub use self::theme::Theme;

pub type SharedTheme = std::rc::Rc<Theme>;

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

pub trait Visibility {
    fn is_visible(&self) -> bool;
    fn show(&mut self);
    fn hide(&mut self);
    fn set_visible(&mut self, b: bool);
}
#[macro_export]
macro_rules! visible_methods {
    ($type:ident) => {
        // In fact out-date, consider rm it
        impl $crate::ui::utils::Visibility for $type {
            fn is_visible(&self) -> bool {
                self.is_visible
            }
            fn show(&mut self) {
                self.is_visible = true;
            }
            fn hide(&mut self) {
                self.is_visible = false;
            }
            fn set_visible(&mut self, b: bool) {
                self.is_visible = b;
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
