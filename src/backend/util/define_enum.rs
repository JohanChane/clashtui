#[macro_export]
macro_rules! define_enum {
    ($(#[$attr:meta])*
    $vis:vis enum $name: ident,
    {$($(#[$item_attr:meta])? $variant:ident $(,)?)*}) => {
        $(#[$attr])*
        $vis enum $name {
            $($(#[$item_attr])? $variant),*
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                match value {
                    $($(#[$item_attr])? stringify!($variant) => $name::$variant,)*
                    _ => panic!("Invalid value for conversion"),
                }
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                match value {
                    $($(#[$item_attr])? $name::$variant => String::from(stringify!($variant)),)*
                }
            }
        }
    };
}
