#[macro_export]
/// build a new enum
/// with [`std::fmt::Display`], [`From<&str>`], [`Into<String>`]
///
/// this also provide `const_array`, `const_len`
/// to get a const array of every item in this enum
///
/// ### WARNING
/// current, only `#[cfg(predicate)]` is tested,
/// other thing like `#[default]` will fail.
macro_rules! define_enum {
    ($(#[$attr:meta])*
    $vis:vis enum $name: ident
    {$($(#[$item_attr:meta])* $variant:ident $(,)?)*}) => {
        $(#[$attr])*
        $vis enum $name {
            $($(#[$item_attr])* $variant),*
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                match value {
                    $($(#[$item_attr])* stringify!($variant) => $name::$variant,)*
                    _ => panic!("Invalid value for conversion"),
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}",
                    match self {
                        $($(#[$item_attr])* $name::$variant => stringify!($variant)),*
                    }
                )
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.to_string()
            }
        }

        impl $name {
            #[doc = concat!("give a const array of [`", stringify!($name), "`],
            with its length [`", stringify!($name), "::const_len`]")]
            pub const fn const_array() -> [$name; $name::const_len()] {
                [$($(#[$item_attr])* $name::$variant),*]
            }
            #[doc = concat!("give the length of [`", stringify!($name), "::const_array`]")]
            pub const fn const_len() -> usize {
                macro_rules! replace_expr {
                    ($_t:tt, $e:expr) => {
                        $e
                    };
                }
                <[()]>::len(&[$($(#[$item_attr])* replace_expr!($variant, ())),*])
            }
        }
    };
}

#[cfg(test)]
mod test {
    define_enum! {
        #[derive(Clone, Copy, PartialEq, Debug)]
        pub enum Test
        {
            Test1,
            // this mean true
            #[cfg(all())]
            Test2,
            // this mean false
            #[cfg(any())]
            #[cfg(any())]
            Test3,
        }
    }
    #[test]
    fn test() {
        assert_eq!(Test::Test1.to_string(), String::from("Test1"));
        assert_eq!(Into::<String>::into(Test::Test1), String::from("Test1"));

        assert_eq!(Test::const_array(), [Test::Test1, Test::Test2]);
        assert_eq!(Test::const_len(), 2);
    }
}
