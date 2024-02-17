extern crate ui_derive;
pub use ui_derive::Visibility;
/// Visibility-related functions, can be impl using `derive`
///
/// Require `is_visible:bool` in the struct
pub trait Visibility {
    fn is_visible(&self) -> bool;
    fn show(&mut self);
    fn hide(&mut self);
    fn set_visible(&mut self, b: bool);
}
pub mod utils;
pub mod widgets;
pub use utils::{EventState, Infailable};

#[cfg(test)]
mod tests {
    use super::Visibility;
    #[test]
    fn set() {
        #[derive(Visibility)]
        struct Test {
            is_visible: bool,
        }
        let mut x = Test { is_visible: false };
        assert!(!x.is_visible);
        assert!(!x.is_visible());
        x.show();
        assert!(x.is_visible());
        x.hide();
        assert!(!x.is_visible());
        x.set_visible(true);
        assert!(x.is_visible());
    }
    // Due to the leak of is_visible in this struct, It won't even pass build

    // #[test]
    // #[should_panic]
    // fn bad(){
    //     #[derive(Visibility)]
    //     struct BadTest{place:bool}
    // }
}
