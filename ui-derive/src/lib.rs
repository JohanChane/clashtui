use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Visibility)]
pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);
    let output = quote! {
        impl Visibility for #ident {
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
    output.into()
}

// macro_rules! title_methods {
//     ($type:ident) => {
//         impl $type {
//             pub fn get_title(&self) -> &String {
//                 &self.title
//             }
//         }
//     };
// }
//
// macro_rules! fouce_methods {
//     ($type:ident) => {
//         impl $type {
//             pub fn is_fouce(&self) -> bool {
//                 self.is_fouce
//             }
//
//             pub fn set_fouce(&mut self, is_fouce: bool) {
//                 self.is_fouce = is_fouce;
//             }
//         }
//     };
// }
//
// macro_rules! msgpopup_methods {
//     ($type:ident) => {
//         impl $type {
//             pub fn popup_txt_msg(&mut self, msg: String) {
//                 self.msgpopup.push_txt_msg(msg);
//                 self.msgpopup.show();
//             }
//             pub fn popup_list_msg(&mut self, msg: Vec<String>) {
//                 self.msgpopup.push_list_msg(msg);
//                 self.msgpopup.show();
//             }
//             pub fn hide_msgpopup(&mut self) {
//                 self.msgpopup.hide();
//             }
//         }
//     };
// }
//
