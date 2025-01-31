use crossterm::event::KeyCode;
/// build [Keys]
///
/// can auto expand `document` into help content,
/// access them by [Keys::const_doc]
///
/// ### Note
/// for keys that are not bind to a char (e.g. [Keys::Select] which is bind to [KeyCode::Enter]),
/// **must use `[""]`** to help expand it.
macro_rules! define_keys {
    ($(#[$attr:meta])*
    $vis:vis enum $name: ident
    {$(
        $(# #[cfg($prompt_cfg_attr:meta)])*
        # $prompt:ident $(,)?
        $(
            $(#[doc = $doc:expr])*
            $(#[cfg($cfg_attr:meta)])*
            $variant:ident (KeyCode::$ch_type:ident $(($ch:expr))? $([$chs:expr])?) $(,)?
        )*
    )*}
    ) => {
        $(#[$attr])*
        $vis enum $name {
        $(
            $(#[cfg($prompt_cfg_attr)])*
        $(
            $(#[doc = $doc])*
            $(#[cfg($cfg_attr)])*
            $variant,
        )*)*
            Reserved,
        }

        impl From<KeyCode> for $name {
            fn from(value: KeyCode) -> Self {
                match value{
                $(
                    $(#[cfg($prompt_cfg_attr)])*
                $(
                    $(#[cfg($cfg_attr)])*
                    KeyCode::$ch_type$(($ch))? => $name::$variant,
                )*)*
                    _ => $name::Reserved,
                }
            }
        }

        impl $name {
        //     #[doc = concat!("give a const array of [`", stringify!($name), "`]'s doc,
        //     with its length [`", stringify!($name), "::doc_len`]")]
        //     #[doc = "this is used for build help content"]
        //     pub const fn const_doc() -> [&'static str; $name::doc_len()]{
        //         [$(
        //             $(#[cfg($prompt_cfg_attr)])*
        //             concat!("# ",stringify!($prompt)),
        //             $(#[cfg($prompt_cfg_attr)])*
        //             $(
        //                 $(#[cfg($cfg_attr)])*
        //                 (concat!($($ch)? $($chs)?, ":" $(, $doc)*)),
        //             )*
        //         )*]
        //     }
            #[doc = concat!("give the length of [`", stringify!($name), "::const_doc`]")]
            pub const fn doc_len() -> usize{
                macro_rules! replace_expr {
                    ($_t:tt, $e:expr) => {
                        $e
                    };
                }
                <[()]>::len(&[$(
                    $(#[cfg($prompt_cfg_attr)])*
                    replace_expr!($prompt, ()),
                    $(#[cfg($prompt_cfg_attr)])*
                    $(
                        $(#[cfg($cfg_attr)])*
                        replace_expr!($variant, ()),
                    )*
                )*])
            }
            pub const ALL_DOC: [&'static str; Self::doc_len()] = [$(
                            $(#[cfg($prompt_cfg_attr)])*
                            concat!("# ",stringify!($prompt)),
                            $(#[cfg($prompt_cfg_attr)])*
                            $(
                                $(#[cfg($cfg_attr)])*
                                (concat!($($ch)? $($chs)?, ":" $(, $doc)*)),
                            )*
                        )*];
        }
    };
}
// this macro mean the shortcut are **unchangeable**
define_keys! {
#[derive(PartialEq)]
    pub enum Keys {
        # Common
        /// Action
        Select(KeyCode::Enter["Enter"]),
        /// Using for debug
        #[cfg(debug_assertions)]
        Debug(KeyCode::Char('\\')),
        // Up(KeyCode::Up["Up"]),
        // Left(KeyCode::Left["Left"]),
        // Right(KeyCode::Right["Right"]),
        // Esc(KeyCode::Esc["Esc"]),
        // Tab(KeyCode::Tab["Tab"]),
        # Profile_Template
        /// Switch to template sub tab
        #[cfg(feature = "template")]
        ProfileSwitch(KeyCode::Char('t')),
        /// Switch to profile sub tab
        #[cfg(feature = "template")]
        TemplateSwitch(KeyCode::Char('p')),
        /// Edit this
        Edit(KeyCode::Char('e')),
        /// Preview content in program
        Preview(KeyCode::Char('v')),
        /// Update profile
        ProfileUpdate(KeyCode::Char('u')),
        // ProfileUpdateAll(KeyCode::Char('a')),
        /// Import new
        Import(KeyCode::Char('i')),
        /// Delete this
        Delete(KeyCode::Char('d')),
        /// Test this profile
        ProfileTestConfig(KeyCode::Char('s')),
        // ProfileInfo(KeyCode::Char('n')),
        # #[cfg(feature = "connection-tab")]
        # Connction
        /// Terminate all running connections
        ConnKillAll(KeyCode::Char('c')),
        /// Search the content
        Search(KeyCode::Char('/')),
        # Global
        /// Restart clash core
        SoftRestart(KeyCode::Char('R')),
        /// Show recent log
        LogCat(KeyCode::Char('L')),
        // AppConfig(KeyCode::Char('H')),
        // ClashConfig(KeyCode::Char('G')),
        /// Get help
        AppHelp(KeyCode::Char('?')),
        /// Quit program
        AppQuit(KeyCode::Char('q')),
    }
}
