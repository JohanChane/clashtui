
#[macro_export]
macro_rules! define_enum {
    ($(#[$attr:meta])*
    $vis:vis $name: ident,
    [$($variant:ident),*]) => {
        $(#[$attr])*
        $vis enum $name {
            $($variant),*
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                match value {
                    $(stringify!($variant) => $name::$variant,)*
                    _ => panic!("Invalid value for conversion"),
                }
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                match value {
                    $($name::$variant => String::from(stringify!($variant)),)*
                }
            }
        }
    };
}

