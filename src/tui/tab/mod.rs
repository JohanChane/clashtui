mod dev {

    pub use crate::tui::widget::dualtab::*;
    pub use crate::tui::widget::tab::*;
    pub use crossterm::event::KeyCode;
    pub use ratatui::prelude::{Frame, Rect};
    pub use ratatui::style::{Color, Stylize as _};
    pub use ratatui::widgets::{Block, List, ListState, StatefulWidget};

    pub use crate::tui::popmsg::prelude::*;
    pub(crate) use crate::tui::theme::Theme;
}

macro_rules! tri {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                Confirm::err(e);
                return do_nothing();
            }
        }
    };
    ($e:expr, or_cancel) => {
        match $e {
            Ok(v) => v,
            Err(_) => {
                return do_nothing();
            }
        }
    };
    ($e:expr, or_set) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                return wrapper(move |content: &mut Self| {
                    content.error = Some(e.to_string());
                });
            }
        }
    };
}

macro_rules! mod_agent {
    ($keymap_name:ident, $scope:expr, $action_ty:ty, [$($tokens:tt)*]) => {
        #[allow(clippy::vec_init_then_push)]
        pub(crate) mod $keymap_name {
            use super::*;
            use std::sync::OnceLock;
            use crate::tui::binding::{Binding, Keymap};

            static KEYMAP: OnceLock<Keymap<$action_ty>> = OnceLock::new();

            pub fn init(keymap: Keymap<$action_ty>) {
                KEYMAP.set(keymap).expect("Keymap init twice");
            }

            pub fn get() -> &'static Keymap<$action_ty> {
                KEYMAP.get_or_init(|| {
                    Keymap::merge(&default_bindings(), &[], $scope)
                        .expect("default bindings must be conflict-free")
                })
            }

            pub fn default_bindings() -> Vec<Binding<$action_ty>> {
                let mut v = Vec::new();
                mod_agent!(@push v, $($tokens)*);
                v
            }
        }
    };

    // Single TT muncher -- unifies [KeyCode] and key("str") token forms
    // These arms must be at the top level of the macro, NOT inside the module block
    (@push $v:ident,
     ([$($codes:expr),+], $map:expr, $desc:expr)
     $($rest:tt)*) => {
        $v.push(crate::tui::binding::Binding {
            on: vec![$($crate::tui::Key {
                code: $codes,
                shift: matches!($codes, crossterm::event::KeyCode::Char(c) if c.is_ascii_uppercase()),
                ctrl: false, alt: false, super_: false,
            }),+],
            exec: $map,
            desc: Some($desc.into()),
        });
        mod_agent!(@push $v, $($rest)*);
    };
    (@push $v:ident,
     (key($($s:literal),+), $map:expr, $desc:expr)
     $($rest:tt)*) => {
        $v.push(crate::tui::binding::Binding {
            on: vec![$({
                use std::str::FromStr;
                crate::tui::Key::from_str($s).expect("invalid key string in mod_agent!")
            }),+],
            exec: $map,
            desc: Some($desc.into()),
        });
        mod_agent!(@push $v, $($rest)*);
    };
    (@push $v:ident, , $($rest:tt)*) => {
        mod_agent!(@push $v, $($rest)*);
    };
    (@push $v:ident,) => {};
}

macro_rules! newtype_tab {
    ($(#[$m:meta])* $tab:ident($ty:ident<$inner:ident>)) => {
        $(#[$m])*
        #[derive(Default)]
        pub struct $tab($ty<$inner>);

        crate::new_type_impl_tuiwidget!($tab);

        impl crate::tui::tab::TuiTab for $tab {
            fn title(&self) -> &'static str {
                $inner::TITLE
            }

            fn shortcuts(&self) -> &[crate::tui::binding::DisplayBinding] {
                self.0.shortcuts()
            }

            fn dispatch_by_seq(&mut self, seq: &[crate::tui::Key]) {
                self.0.dispatch_by_seq(seq)
            }
        }
    };
    ($(#[$m:meta])* $tab:ident($inner:ty), $title:literal) => {
        $(#[$m])*
        #[derive(Default)]
        pub struct $tab($inner);

        crate::new_type_impl_tuiwidget!($tab);

        impl crate::tui::tab::TuiTab for $tab {
            fn title(&self) -> &'static str {
                $title
            }

            fn shortcuts(&self) -> &[crate::tui::binding::DisplayBinding] {
                self.0.shortcuts()
            }

            fn dispatch_by_seq(&mut self, seq: &[crate::tui::Key]) {
                self.0.dispatch_by_seq(seq)
            }
        }
    };
}

pub trait TuiTab: super::TuiWidget {
    fn title(&self) -> &'static str;
    fn shortcuts(&self) -> &[crate::tui::binding::DisplayBinding];
    fn dispatch_by_seq(&mut self, seq: &[crate::tui::Key]);
}

pub(crate) mod connections;
pub(crate) mod files;
pub(crate) mod logs;
pub(crate) mod proxies;
pub(crate) mod settings;
pub(crate) mod srvctl;
mod status;

macro_rules! enum_dispatch {
    ($vis:vis enum $ident:ident {
        $($item:ident,)+
    }) => {
    #[allow(clippy::large_enum_variant, clippy::enum_variant_names)]
    $vis enum $ident {
        $($item($item),)+
    }

    $(impl From<$item> for Tab {
        fn from(value: $item) -> Self {
            Self::$item(value)
        }
    })+

    impl crate::tui::TuiWidget for Tab {

        fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
            match self {
                $(Self::$item(inner) => inner.render(f, area),)+
            }
        }

        fn sync(&mut self) {
            match self {
                $(Self::$item(inner) => inner.sync(),)+
            }
        }

        fn on_enter(&mut self) {
            match self {
                $(Self::$item(inner) => inner.on_enter(),)+
            }
        }

        fn on_leave(&mut self) {
            match self {
                $(Self::$item(inner) => inner.on_leave(),)+
            }
        }
    }

    impl TuiTab for Tab {
        fn title(&self) -> &'static str {
            match self {
                $(Self::$item(inner) => inner.title(),)+
            }
        }

        fn shortcuts(&self) -> &[crate::tui::binding::DisplayBinding] {
            match self {
                $(Self::$item(inner) => inner.shortcuts(),)+
            }
        }

        fn dispatch_by_seq(&mut self, seq: &[crate::tui::Key]) {
            match self {
                $(Self::$item(inner) => inner.dispatch_by_seq(seq),)+
            }
        }
    }

    };
}

pub mod prelude {
    pub use super::TuiTab;
    pub use super::connections::ConnectionsTab;
    pub use super::files::FileTab;
    pub use super::logs::LogsTab;
    pub use super::proxies::ProxiesTab;
    pub use super::settings::SettingsTab;
    pub use super::srvctl::CoreSrvCtlTab;
    pub use super::status::StatusTab;

    pub fn agent_init(keymap: &mut serde_yml::Mapping) -> anyhow::Result<()> {
        use anyhow::Context;

        // Helper: dispatch Sequence (list-format) into each tab's keymap module.
        macro_rules! init_section {
            ($keymap:expr, $section:literal, $mod:ident, $action_ty:ty, $scope:expr) => {
                if let Some(section_val) = $keymap.remove($section) {
                    match section_val {
                        serde_yml::Value::Sequence(seq) => {
                            let entries: Vec<crate::tui::agent::RawEntry> =
                                serde_yml::from_value(serde_yml::Value::Sequence(seq))
                                    .context(concat!("parsing ", $section, " entries"))?;
                            let defaults = super::$mod::keymap::default_bindings();
                            let overrides: Vec<crate::tui::binding::Binding<$action_ty>> = entries
                                .iter()
                                .map(|e| {
                                    let exec: $action_ty = serde_yml::from_value(e.exec.clone())?;
                                    anyhow::Ok(crate::tui::binding::Binding {
                                        on: e.on.clone(),
                                        exec,
                                        desc: e.desc.clone(),
                                    })
                                })
                                .collect::<anyhow::Result<Vec<_>>>()?;
                            let keymap =
                                crate::tui::binding::Keymap::merge(&defaults, &overrides, $scope)?;
                            super::$mod::keymap::init(keymap);
                        }
                        _ => {
                            anyhow::bail!(
                                "Section `{}` must be a Sequence (list-format). \
                                 Mapping format is no longer supported.",
                                $section
                            );
                        }
                    }
                }
            };
        }

        init_section!(
            keymap,
            "connections",
            connections,
            super::connections::Key,
            crate::tui::binding::Scope::Connections
        );
        init_section!(
            keymap,
            "proxies",
            proxies,
            super::proxies::Key,
            crate::tui::binding::Scope::Proxies
        );
        init_section!(
            keymap,
            "srvctl",
            srvctl,
            super::srvctl::SrvCtlKey,
            crate::tui::binding::Scope::SrvCtl
        );
        init_section!(
            keymap,
            "settings",
            settings,
            super::settings::SettingsKey,
            crate::tui::binding::Scope::Settings
        );
        init_section!(
            keymap,
            "logs",
            logs,
            super::logs::Key,
            crate::tui::binding::Scope::Logs
        );

        // FileTab: init profile and template individually (previously nested under `file:`)
        {
            use super::files::profile;
            use super::files::template;

            if let Some(section_val) = keymap.remove("file/profile") {
                match section_val {
                    serde_yml::Value::Sequence(seq) => {
                        let entries: Vec<crate::tui::agent::RawEntry> =
                            serde_yml::from_value(serde_yml::Value::Sequence(seq))
                                .context("parsing file/profile entries")?;
                        let defaults = profile::keymap::default_bindings();
                        let overrides: Vec<crate::tui::binding::Binding<profile::Key>> = entries
                            .iter()
                            .map(|e| {
                                let exec: profile::Key = serde_yml::from_value(e.exec.clone())?;
                                anyhow::Ok(crate::tui::binding::Binding {
                                    on: e.on.clone(),
                                    exec,
                                    desc: e.desc.clone(),
                                })
                            })
                            .collect::<anyhow::Result<Vec<_>>>()?;
                        let km = crate::tui::binding::Keymap::merge(
                            &defaults,
                            &overrides,
                            crate::tui::binding::Scope::FileProfile,
                        )?;
                        profile::keymap::init(km);
                    }
                    _ => anyhow::bail!(
                        "`file/profile` must be a Sequence (list-format). Mapping format is no longer supported."
                    ),
                }
            }

            if let Some(section_val) = keymap.remove("file/template") {
                match section_val {
                    serde_yml::Value::Sequence(seq) => {
                        let entries: Vec<crate::tui::agent::RawEntry> =
                            serde_yml::from_value(serde_yml::Value::Sequence(seq))
                                .context("parsing file/template entries")?;
                        let defaults = template::keymap::default_bindings();
                        let overrides: Vec<crate::tui::binding::Binding<template::Key>> = entries
                            .iter()
                            .map(|e| {
                                let exec: template::Key = serde_yml::from_value(e.exec.clone())?;
                                anyhow::Ok(crate::tui::binding::Binding {
                                    on: e.on.clone(),
                                    exec,
                                    desc: e.desc.clone(),
                                })
                            })
                            .collect::<anyhow::Result<Vec<_>>>()?;
                        let km = crate::tui::binding::Keymap::merge(
                            &defaults,
                            &overrides,
                            crate::tui::binding::Scope::FileTemplate,
                        )?;
                        template::keymap::init(km);
                    }
                    _ => anyhow::bail!(
                        "`file/template` must be a Sequence (list-format). Mapping format is no longer supported."
                    ),
                }
            }
        }

        // Warn about leftover keys (e.g. old `file:` nested format)
        for leftover in keymap.keys() {
            if let Some(k) = leftover.as_str() {
                log::warn!(
                    "Unrecognized keymap section `{k}` -- ignored. Did you mean `file/profile` or `file/template`?"
                );
            }
        }

        // Global scope: init from YAML or use defaults-only
        {
            let defaults = crate::tui::global_keymap::default_bindings();
            let overrides: Vec<
                crate::tui::binding::Binding<crate::tui::global_keymap::GlobalAction>,
            > = if let Some(section_val) = keymap.remove("global") {
                match section_val {
                    serde_yml::Value::Sequence(seq) => {
                        let entries: Vec<crate::tui::agent::RawEntry> =
                            serde_yml::from_value(serde_yml::Value::Sequence(seq))
                                .context("parsing global entries")?;
                        entries
                            .iter()
                            .map(|e| {
                                let exec = serde_yml::from_value(e.exec.clone())?;
                                anyhow::Ok(crate::tui::binding::Binding {
                                    on: e.on.clone(),
                                    exec,
                                    desc: e.desc.clone(),
                                })
                            })
                            .collect::<anyhow::Result<Vec<_>>>()?
                    }
                    _ => anyhow::bail!(
                        "`global` section must be a Sequence (list-format). Mapping format is no longer supported."
                    ),
                }
            } else {
                Vec::new()
            };
            let keymap = crate::tui::binding::Keymap::merge(
                &defaults,
                &overrides,
                crate::tui::binding::Scope::Global,
            )?;
            crate::tui::global_keymap::init(keymap);
        }

        Ok(())
    }

    enum_dispatch!(
        pub enum Tab {
            ConnectionsTab,
            FileTab,
            ProxiesTab,
            SettingsTab,
            CoreSrvCtlTab,
            StatusTab,
            LogsTab,
        }
    );
}
