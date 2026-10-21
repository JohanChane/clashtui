//! Immutable bindings for each page/focus context. All shortcut views use these maps.

use super::config::ResolvedBinding;
use super::{Document, Key, KeyDesc, KeyDescRef, KeyMap, MaybeMap, config};
use crate::tui::{app::AppKey, tab};
use anyhow::{Context, Result, ensure};
use serde_yml::{Mapping, Value};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Scope {
    App,
    Connections,
    Proxies,
    Srvctl,
    Settings,
    Logs,
    Profile,
    Template,
}

impl Scope {
    pub fn name(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Connections => "connections",
            Self::Proxies => "proxies",
            Self::Srvctl => "srvctl",
            Self::Settings => "settings",
            Self::Logs => "logs",
            Self::Profile => "file.profile",
            Self::Template => "file.template",
        }
    }

    fn section(self) -> &'static str {
        match self {
            Self::Profile => "files/profile",
            Self::Template => "files/template",
            _ => self.name(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::tui) enum Action {
    App(AppKey),
    Connections(tab::connections::Key),
    Proxies(tab::proxies::Key),
    Srvctl(tab::srvctl::Key),
    Settings(tab::settings::Key),
    Logs(tab::logs::Key),
    Profile(tab::files::profile::Key),
    Template(tab::files::template::Key),
}

impl Document for Action {
    fn get_doc(&self) -> &'static str {
        match self {
            Self::App(action) => action.get_doc(),
            Self::Connections(action) => action.get_doc(),
            Self::Proxies(action) => action.get_doc(),
            Self::Srvctl(action) => action.get_doc(),
            Self::Settings(action) => action.get_doc(),
            Self::Logs(action) => action.get_doc(),
            Self::Profile(action) => action.get_doc(),
            Self::Template(action) => action.get_doc(),
        }
    }
}

pub(in crate::tui) struct EffectiveKeymap {
    pub bindings: Vec<ResolvedBinding<Action>>,
    pub map: KeyMap<Action>,
    page_docs: KeyDesc,
    global_docs: KeyDesc,
}

impl EffectiveKeymap {
    fn new(
        globals: &[ResolvedBinding<Action>],
        page: Vec<ResolvedBinding<Action>>,
        scope: Scope,
    ) -> Result<Self> {
        for global in globals {
            for local in &page {
                ensure!(
                    !global.on.starts_with(&local.on) && !local.on.starts_with(&global.on),
                    "key binding conflict: `app` `{}` ({}) conflicts with `{}` `{}` ({}); rebind or unbind one of these sequences",
                    super::notation::format_sequence(&global.on),
                    global.action.get_doc(),
                    scope.name(),
                    super::notation::format_sequence(&local.on),
                    local.action.get_doc(),
                );
            }
        }
        let bindings: Vec<_> = globals.iter().cloned().chain(page).collect();
        let (map, _) = config::compile(bindings.iter().cloned());
        let mut page_docs = Vec::new();
        let mut global_docs = Vec::new();
        for binding in &bindings {
            let docs = if matches!(binding.action, Action::App(_)) {
                &mut global_docs
            } else {
                &mut page_docs
            };
            docs.push((
                super::notation::format_sequence(&binding.on),
                binding.desc.clone(),
            ));
        }
        page_docs.sort();
        global_docs.sort();
        Ok(Self {
            bindings,
            map,
            page_docs,
            global_docs,
        })
    }

    pub fn page_docs(&self) -> KeyDescRef<'_> {
        &self.page_docs
    }

    pub fn global_docs(&self) -> KeyDescRef<'_> {
        &self.global_docs
    }

    pub fn starts_chord(&self, key: &Key) -> bool {
        matches!(self.map.get(key), Some(MaybeMap::SubMap(_)))
    }
}

pub(in crate::tui) struct Keymaps {
    contexts: HashMap<Scope, EffectiveKeymap>,
}

impl Keymaps {
    pub fn from_config(value: Mapping, core: &str) -> Result<Self> {
        use tab::{connections, files, logs, proxies, settings, srvctl};
        type Loader = fn(Value) -> Result<Vec<ResolvedBinding<Action>>>;
        let mut sections = config::sections(value, core)?;
        let globals = crate::tui::app::km::km::bindings(sections.remove("app").unwrap_or_default())
            .context("failed to load keymap for app")?;
        let mut contexts = HashMap::new();
        contexts.insert(
            Scope::App,
            EffectiveKeymap::new(&globals, vec![], Scope::App)?,
        );
        let loaders: [(Scope, Loader); 7] = [
            (Scope::Connections, connections::km::bindings),
            (Scope::Proxies, proxies::km::bindings),
            (Scope::Srvctl, srvctl::km::bindings),
            (Scope::Settings, settings::km::bindings),
            (Scope::Logs, logs::km::bindings),
            (Scope::Profile, files::profile::km::bindings),
            (Scope::Template, files::template::km::bindings),
        ];
        for (scope, load) in loaders {
            let page = load(sections.remove(scope.section()).unwrap_or_default())
                .with_context(|| format!("failed to load keymap for {}", scope.name()))?;
            contexts.insert(scope, EffectiveKeymap::new(&globals, page, scope)?);
        }
        Ok(Self { contexts })
    }

    pub fn get(&self, scope: Scope) -> &EffectiveKeymap {
        &self.contexts[&scope]
    }
}
