mod key_list;
mod list;

pub use self::key_list::Keys;
pub use self::list::HelpPopUp;

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
macro_rules! msgpopup_methods {
    ($type:ident) => {
        impl $type {
            pub fn popup_txt_msg(&mut self, msg: String) {
                self.msgpopup.push_txt_msg(msg);
                self.msgpopup.show();
            }
            pub fn popup_list_msg(&mut self, msg: Vec<String>) {
                self.msgpopup.push_list_msg(msg);
                self.msgpopup.show();
            }
            pub fn hide_msgpopup(&mut self) {
                self.msgpopup.hide();
            }
        }
    };
}
