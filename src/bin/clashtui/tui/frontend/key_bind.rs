use crossterm::event::KeyCode;
/// build [Keys]
///
/// can auto expand `document` into help content,
/// access them by [Keys::const_doc]
///
/// ### Note
/// for keys that are not bind to a char (e.g. [Keys::Select] which is bind to [KeyCode::Enter]),
/// **must use `[""]`** to help expand it.
/// 
/// ### Example
/// this doc may not display correctly as it uses `#` for a key word.
/// ```rust
/// use crossterm::event::KeyCode;
/// use clashtui::tui::frontend::key_bind::define_keys;
///
/// define_keys!{
///     #[derive(PartialEq)]
///     pub enum Test{
///         # Aera1
///         /// means func1
///         Func1(KeyCode::Char('a')),
///         #Aera2,
///         /// part1,
///         /// part2
///         Func2(KeyCode::Char('b')),
///         /// doc on Esc
///         Func3(KeyCode::Esc["Esc"]),
///         ///     more space
///         Func4(KeyCode::Char('c')),
///         #       Aera3,
///         /// true
///         #[cfg(all())]
///         Func5(KeyCode::Enter["Enter"]),
///         /// false
///         #[cfg(any())]
///         Func5(KeyCode::Space["Space"]),
///         # Aera4
///         # Aera5
///         ##[cfg(any())]
///         # Aera6
///     }
/// }
/// assert!(Test::Func1 == KeyCode::Char('a').into());
/// assert!(Test::Func3 == KeyCode::Esc.into());
/// assert_eq!(Test::const_doc(), [
///     "# Aera1",
///     "a: means func1",
///     "# Aera2",
///     "b: part1, part2",
///     "Esc: doc on Esc",
///     "c:     more space",
///     "# Aera3",
///     "Enter: true",
///     "# Aera4",
///     "# Aera5",
/// ]);
/// ```
macro_rules! define_keys {
    ($(#[$attr:meta])*
    $vis:vis enum $name: ident
    {$(
        $(##[cfg($prompt_cfg_attr:meta)])*
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
            #[doc = concat!("give a const array of [`", stringify!($name), "`]'s doc,
            with its length [`", stringify!($name), "::const_len`]")]
            #[doc = "this is used for build help content"]
            pub const fn const_doc() -> [&'static str; $name::doc_len()]{
                [$(
                    $(#[cfg($prompt_cfg_attr)])* 
                    concat!("# ",stringify!($prompt)), 
                    $(
                        $(#[cfg($cfg_attr)])* 
                        (concat!($($ch)? $($chs)?, ":" $(, $doc)*)),
                    )*
                )*]
            }
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
                    $(
                        $(#[cfg($cfg_attr)])* 
                        replace_expr!($variant, ()),
                    )*
                )*])
            }
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
        // Down(KeyCode::Down["Down"]),
        // Up(KeyCode::Up["Up"]),
        // Left(KeyCode::Left["Left"]),
        // Right(KeyCode::Right["Right"]),
        // Esc(KeyCode::Esc["Esc"]),
        // Tab(KeyCode::Tab["Tab"]),
        # Profile
        /// Switch to template sub tab
        #[cfg(feature = "template")]
        ProfileSwitch(KeyCode::Char('t')),
        /// Update profile
        ProfileUpdate(KeyCode::Char('u')),
        // ProfileUpdateAll(KeyCode::Char('a')),
        /// Import new profile
        ProfileImport(KeyCode::Char('i')),
        /// Delete this profile
        ProfileDelete(KeyCode::Char('d')),
        /// Test this profile
        ProfileTestConfig(KeyCode::Char('s')),
        // ProfileInfo(KeyCode::Char('n')),
        # #[cfg(feature = "template")]
        # Template
        /// Switch to profile sub tab
        TemplateSwitch(KeyCode::Char('p')),
        # Global
        /// Edit this
        Edit(KeyCode::Char('e')),
        /// Preview content in program
        Preview(KeyCode::Char('v')),
        /// Restart clash core
        SoftRestart(KeyCode::Char('R')),
        /// Show recent log
        LogCat(KeyCode::Char('L')),
        // AppConfig(KeyCode::Char('H')),
        // ClashConfig(KeyCode::Char('G')),
        /// Get help
        AppHelp(KeyCode::Char('?')),
        /// Show informations about program and mihomo
        AppInfo(KeyCode::Char('I')),
        /// Quit program
        AppQuit(KeyCode::Char('q')),
    }
}
