//! Immutable bindings for each page/focus context. All shortcut views use these maps.

use super::config::ResolvedBinding;
use super::{Document, Key, config, notation};
use crate::tui::{
    app::{AppKey, TabTarget},
    tab,
};
use anyhow::{Context, Result, ensure};
use serde_yml::{Mapping, Value};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

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
    pub const ALL: [Self; 8] = [
        Self::App,
        Self::Connections,
        Self::Proxies,
        Self::Srvctl,
        Self::Settings,
        Self::Logs,
        Self::Profile,
        Self::Template,
    ];

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl Action {
    fn scope(self) -> Scope {
        match self {
            Self::App(_) => Scope::App,
            Self::Connections(_) => Scope::Connections,
            Self::Proxies(_) => Scope::Proxies,
            Self::Srvctl(_) => Scope::Srvctl,
            Self::Settings(_) => Scope::Settings,
            Self::Logs(_) => Scope::Logs,
            Self::Profile(_) => Scope::Profile,
            Self::Template(_) => Scope::Template,
        }
    }
}

/// An index into one immutable Keymaps instance; never a copy of a binding.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::tui) struct BindingId(usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::tui) enum KeySequence {
    Single(Key),
    Chord([Key; 2]),
}

impl KeySequence {
    pub fn as_slice(&self) -> &[Key] {
        match self {
            Self::Single(key) => std::slice::from_ref(key),
            Self::Chord(keys) => keys,
        }
    }
}

/// The sole owner of a resolved binding's action and presentation text.
#[derive(Debug)]
pub(in crate::tui) struct RuntimeBinding {
    /// The defining scope, even when another page indexes this binding.
    pub source_scope: Scope,
    pub on: KeySequence,
    pub action: Action,
    pub desc: String,
    /// Format once; help and tab titles borrow this text every frame.
    pub key_text: String,
}

#[derive(Default)]
struct KeyLookup {
    dispatch: HashMap<Key, DispatchEntry>,
    // Guards only: these never become executable bindings or help rows.
    disabled: HashSet<KeySequence>,
}

pub(super) enum DispatchEntry {
    Single(BindingId),
    Chord(ChordIndex),
}

#[derive(Default)]
pub(super) struct ChordIndex {
    submap: HashMap<Key, BindingId>,
}

/// Ordered references into the binding store, without copying descriptions.
#[derive(Clone, Copy)]
pub(in crate::tui) struct BindingView<'a> {
    bindings: &'a [RuntimeBinding],
    ids: &'a [BindingId],
}

impl<'a> BindingView<'a> {
    pub fn iter(self) -> impl ExactSizeIterator<Item = &'a RuntimeBinding> + Clone {
        self.ids.iter().map(move |id| &self.bindings[id.0])
    }

    pub fn len(self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(self) -> bool {
        self.ids.is_empty()
    }
}

/// A transient presentation of the exact branch used for matching.
/// Only the reference list is allocated.
#[derive(Debug)]
pub(in crate::tui) struct ChordView<'a> {
    pub prefix: Key,
    bindings: Vec<&'a RuntimeBinding>,
}

impl<'a> ChordView<'a> {
    pub fn bindings(&self) -> &[&'a RuntimeBinding] {
        &self.bindings
    }

    pub fn rows(&self) -> impl Iterator<Item = (String, &'a str)> + '_ {
        self.bindings
            .iter()
            .map(|binding| (binding.on.as_slice()[1].to_string(), binding.desc.as_str()))
    }
}

pub(in crate::tui) struct Keymaps {
    // Only this array owns binding data; all other collections store IDs.
    bindings: Box<[RuntimeBinding]>,
    // Source-only lists, sorted for help; App is not repeated in page lists.
    scope_bindings: HashMap<Scope, Box<[BindingId]>>,
    // Effective execution tables: global bindings plus each page's bindings.
    key_lookup: HashMap<Scope, KeyLookup>,
    // Reverse index for tab titles, preserving aliases and chord shortcuts.
    tab_shortcuts: [Box<[BindingId]>; TabTarget::ALL.len()],
}

impl Keymaps {
    pub fn from_config(value: Mapping, core: &str) -> Result<Self> {
        use tab::{connections, files, logs, proxies, settings, srvctl};
        type Loader = fn(Value) -> Result<Vec<ResolvedBinding<Action>>>;
        let mut sections = config::sections(value, core)?;
        let loaders: [(Scope, Loader); 8] = [
            (Scope::App, crate::tui::app::km::km::bindings),
            (Scope::Connections, connections::km::bindings),
            (Scope::Proxies, proxies::km::bindings),
            (Scope::Srvctl, srvctl::km::bindings),
            (Scope::Settings, settings::km::bindings),
            (Scope::Logs, logs::km::bindings),
            (Scope::Profile, files::profile::km::bindings),
            (Scope::Template, files::template::km::bindings),
        ];
        let mut scopes = HashMap::new();
        for (scope, load) in loaders {
            let bindings = load(sections.remove(scope.section()).unwrap_or_default())
                .with_context(|| format!("failed to load keymap for {}", scope.name()))?;
            scopes.insert(scope, bindings);
        }
        Self::from_bindings(scopes)
    }

    /// Move resolved data into one store, then compile ID-only lookup tables.
    pub fn from_bindings(mut scopes: HashMap<Scope, Vec<ResolvedBinding<Action>>>) -> Result<Self> {
        let mut bindings = Vec::new();
        let mut scope_bindings = HashMap::new();
        let mut scope_disabled = HashMap::new();
        for scope in Scope::ALL {
            let mut ids = Vec::new();
            let mut disabled = HashSet::new();
            for binding in scopes.remove(&scope).unwrap_or_default() {
                let on = match binding.on.as_slice() {
                    [key] => KeySequence::Single(*key),
                    [first, second] => {
                        ensure!(
                            binding.action.is_none()
                                || *second != Key::from_code(super::KeyCode::Esc),
                            "Esc is reserved for cancelling a pending chord; choose another second key"
                        );
                        KeySequence::Chord([*first, *second])
                    }
                    _ => anyhow::bail!("binding must contain 1 or 2 keys"),
                };
                let Some(action) = binding.action else {
                    disabled.insert(on);
                    continue;
                };
                ensure!(
                    action.scope() == scope,
                    "binding action belongs to another scope"
                );
                ids.push(BindingId(bindings.len()));
                bindings.push(RuntimeBinding {
                    source_scope: scope,
                    on,
                    action,
                    desc: binding.desc,
                    key_text: notation::format_sequence(on.as_slice()),
                });
            }
            ids.sort_by(|a, b| bindings[a.0].key_text.cmp(&bindings[b.0].key_text));
            scope_bindings.insert(scope, ids.into_boxed_slice());
            scope_disabled.insert(scope, disabled);
        }
        let mut maps = Self {
            bindings: bindings.into_boxed_slice(),
            scope_bindings,
            key_lookup: HashMap::new(),
            tab_shortcuts: std::array::from_fn(|_| Box::default()),
        };
        for scope in Scope::ALL {
            maps.validate_global_conflicts(scope)?;
            let ids = maps.scope_bindings[&Scope::App]
                .iter()
                .chain(maps.page_ids(scope))
                .copied();
            let mut lookup = KeyLookup::compile(ids, &maps.bindings)?;
            lookup.disabled.extend(&scope_disabled[&Scope::App]);
            lookup.disabled.extend(&scope_disabled[&scope]);
            maps.key_lookup.insert(scope, lookup);
        }
        let mut tabs: [Vec<BindingId>; TabTarget::ALL.len()] = std::array::from_fn(|_| Vec::new());
        for &id in &maps.scope_bindings[&Scope::App] {
            if let Action::App(AppKey::Tab(target)) = maps.binding(id).action {
                tabs[target as usize].push(id);
            }
        }
        maps.tab_shortcuts = tabs.map(Vec::into_boxed_slice);
        Ok(maps)
    }

    fn validate_global_conflicts(&self, scope: Scope) -> Result<()> {
        for global in self.global_bindings().iter() {
            for local in self.page_bindings(scope).iter() {
                ensure!(
                    !global.on.as_slice().starts_with(local.on.as_slice())
                        && !local.on.as_slice().starts_with(global.on.as_slice()),
                    "key binding conflict: `app` `{}` ({}) conflicts with `{}` `{}` ({}); rebind or unbind one of these sequences",
                    global.key_text,
                    global.action.get_doc(),
                    scope.name(),
                    local.key_text,
                    local.action.get_doc(),
                );
            }
        }
        Ok(())
    }

    pub fn binding(&self, id: BindingId) -> &RuntimeBinding {
        &self.bindings[id.0]
    }

    fn view<'a>(&'a self, ids: &'a [BindingId]) -> BindingView<'a> {
        BindingView {
            bindings: &self.bindings,
            ids,
        }
    }

    fn page_ids(&self, scope: Scope) -> &[BindingId] {
        if scope == Scope::App {
            &[]
        } else {
            &self.scope_bindings[&scope]
        }
    }

    pub fn global_bindings(&self) -> BindingView<'_> {
        self.view(&self.scope_bindings[&Scope::App])
    }

    pub fn page_bindings(&self, scope: Scope) -> BindingView<'_> {
        self.view(self.page_ids(scope))
    }

    pub fn tab_bindings(&self, target: TabTarget) -> BindingView<'_> {
        self.view(&self.tab_shortcuts[target as usize])
    }

    fn effective_ids(&self, scope: Scope) -> Vec<BindingId> {
        let mut ids: Vec<_> = self.scope_bindings[&Scope::App]
            .iter()
            .chain(self.page_ids(scope))
            .copied()
            .collect();
        ids.sort_by(|a, b| self.binding(*a).key_text.cmp(&self.binding(*b).key_text));
        ids
    }

    pub fn effective_bindings(
        &self,
        scope: Scope,
    ) -> impl ExactSizeIterator<Item = (BindingId, &RuntimeBinding)> {
        self.effective_ids(scope)
            .into_iter()
            .map(|id| (id, self.binding(id)))
    }

    fn chord(&self, scope: Scope, prefix: &Key) -> Option<&ChordIndex> {
        match self.lookup(scope, prefix) {
            Some(DispatchEntry::Chord(chord)) => Some(chord),
            _ => None,
        }
    }

    pub(super) fn lookup(&self, scope: Scope, key: &Key) -> Option<&DispatchEntry> {
        self.key_lookup[&scope].dispatch.get(key)
    }

    pub(super) fn second(&self, scope: Scope, prefix: &Key, key: &Key) -> Option<BindingId> {
        self.chord(scope, prefix)?.submap.get(key).copied()
    }

    pub(super) fn has_disabled_prefix(&self, scope: Scope, key: Key) -> bool {
        self.key_lookup[&scope]
            .disabled
            .iter()
            .any(|sequence| sequence.as_slice()[0] == key)
    }

    pub(super) fn is_disabled(&self, scope: Scope, sequence: KeySequence) -> bool {
        self.key_lookup[&scope].disabled.contains(&sequence)
    }

    pub fn chord_view(&self, scope: Scope, prefix: Key) -> Option<ChordView<'_>> {
        let chord = self.chord(scope, &prefix)?;
        let mut bindings: Vec<_> = chord.submap.values().map(|id| self.binding(*id)).collect();
        // All candidates have the same prefix, so full key text orders second keys too.
        bindings.sort_by(|a, b| a.key_text.cmp(&b.key_text));
        Some(ChordView { prefix, bindings })
    }

    /// Expand binding IDs and chord candidates for inspection.
    pub fn describe(&self, scope: Scope) -> String {
        let mut output = String::from("ID\tScope\tKeys\tAction\tDescription\n");
        for (id, b) in self.effective_bindings(scope) {
            writeln!(
                output,
                "{}\t{}\t{}\t{:?}\t{}",
                id.0,
                b.source_scope.name(),
                b.key_text,
                b.action,
                b.desc
            )
            .unwrap();
        }
        let mut prefixes: Vec<_> = self.key_lookup[&scope]
            .dispatch
            .iter()
            .filter_map(|(key, entry)| matches!(entry, DispatchEntry::Chord(_)).then_some(*key))
            .collect();
        prefixes.sort_by_cached_key(ToString::to_string);
        for prefix in prefixes {
            let view = self.chord_view(scope, prefix).expect("indexed chord");
            let chord = self.chord(scope, &prefix).expect("indexed chord");
            let candidates = view
                .bindings
                .iter()
                .map(|binding| {
                    let second = binding.on.as_slice()[1];
                    format!("{second}={}", chord.submap[&second].0)
                })
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(output, "Prefix {prefix}: candidates [{candidates}]").unwrap();
        }
        output
    }
}

impl KeyLookup {
    fn compile(ids: impl Iterator<Item = BindingId>, bindings: &[RuntimeBinding]) -> Result<Self> {
        let mut lookup = Self::default();
        for id in ids {
            let binding = &bindings[id.0];
            match binding.on {
                KeySequence::Single(key) => {
                    ensure!(
                        lookup
                            .dispatch
                            .insert(key, DispatchEntry::Single(id))
                            .is_none(),
                        "duplicate or conflicting key binding `{}`",
                        binding.key_text
                    );
                }
                KeySequence::Chord([first, second]) => {
                    let entry = lookup
                        .dispatch
                        .entry(first)
                        .or_insert_with(|| DispatchEntry::Chord(ChordIndex::default()));
                    let DispatchEntry::Chord(chord) = entry else {
                        anyhow::bail!("prefix conflict for `{}`", binding.key_text);
                    };
                    ensure!(
                        chord.submap.insert(second, id).is_none(),
                        "duplicate key binding `{}`",
                        binding.key_text
                    );
                }
            }
        }
        Ok(lookup)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn maps(yaml: &str) -> Keymaps {
        Keymaps::from_config(serde_yml::from_str(yaml).unwrap(), "mihomo").unwrap()
    }

    #[test]
    fn source_records_are_unique_and_global_ids_are_shared_by_every_lookup() {
        let maps = maps("{}");
        let mut seen = HashSet::new();
        assert_eq!(maps.scope_bindings.len(), 8);
        assert_eq!(maps.key_lookup.len(), 8);
        for scope in Scope::ALL {
            let ids = &maps.scope_bindings[&scope];
            for &id in ids {
                assert!(seen.insert(id), "a binding belongs to exactly one source");
                assert_eq!(maps.binding(id).source_scope, scope);
            }
            assert!(
                ids.windows(2)
                    .all(|pair| maps.binding(pair[0]).key_text <= maps.binding(pair[1]).key_text)
            );
            for &id in &maps.scope_bindings[&Scope::App] {
                let found = match maps.binding(id).on {
                    KeySequence::Single(key) => match maps.lookup(scope, &key) {
                        Some(DispatchEntry::Single(id)) => Some(*id),
                        _ => None,
                    },
                    KeySequence::Chord([first, second]) => maps.second(scope, &first, &second),
                };
                assert_eq!(
                    found,
                    Some(id),
                    "global binding must reuse its ID in {scope:?}"
                );
                assert!(std::ptr::eq(maps.binding(found.unwrap()), maps.binding(id)));
            }
            for (prefix, entry) in &maps.key_lookup[&scope].dispatch {
                if let DispatchEntry::Chord(chord) = entry {
                    let view = maps.chord_view(scope, *prefix).unwrap();
                    assert_eq!(view.bindings.len(), chord.submap.len());
                    for (second, id) in &chord.submap {
                        let binding = maps.binding(*id);
                        assert_eq!(binding.on, KeySequence::Chord([*prefix, *second]));
                        assert!(
                            view.bindings
                                .iter()
                                .any(|candidate| std::ptr::eq(*candidate, binding))
                        );
                    }
                }
            }
        }
        assert_eq!(seen.len(), maps.bindings.len());
    }

    #[test]
    fn empty_sources_keep_their_indices_without_phantom_bindings() {
        let maps = Keymaps::from_bindings(HashMap::new()).unwrap();
        assert!(maps.bindings.is_empty());
        for scope in Scope::ALL {
            assert!(maps.scope_bindings[&scope].is_empty());
            assert!(maps.key_lookup[&scope].dispatch.is_empty());
            assert_eq!(maps.effective_bindings(scope).len(), 0);
        }
        assert!(maps.tab_shortcuts.iter().all(|ids| ids.is_empty()));
    }

    #[test]
    fn runtime_construction_rejects_unmatchable_sequences_and_wrong_sources() {
        for keys in [vec![], vec!["g", "g", "g"], vec!["g", "<Esc>"]] {
            let binding = ResolvedBinding {
                on: keys.into_iter().map(|key| key.parse().unwrap()).collect(),
                action: Some(Action::Logs(tab::logs::Key::Clear)),
                desc: "Clear".into(),
            };
            assert!(Keymaps::from_bindings([(Scope::Logs, vec![binding])].into()).is_err());
        }
        let binding = ResolvedBinding {
            on: vec!["x".parse().unwrap()],
            action: Some(Action::App(AppKey::Quit)),
            desc: "Quit".into(),
        };
        assert!(Keymaps::from_bindings([(Scope::Logs, vec![binding])].into()).is_err());
    }

    #[test]
    fn tab_shortcuts_keep_all_aliases_and_omit_fully_unbound_targets() {
        let maps = maps(
            "app: [{on: '<F2>', action: TabLogs}, {on: [z, l], action: TabLogs}, {on: '6', action: null}]",
        );
        assert_eq!(
            maps.tab_bindings(TabTarget::Logs)
                .iter()
                .map(|b| b.key_text.as_str())
                .collect::<Vec<_>>(),
            ["5", "<F2>", "z l"]
        );
        assert!(maps.tab_bindings(TabTarget::Settings).is_empty());
        for target in TabTarget::ALL {
            for binding in maps.tab_bindings(target).iter() {
                assert_eq!(binding.source_scope, Scope::App);
                assert!(binding.action == Action::App(AppKey::Tab(target)));
            }
        }
    }

    #[test]
    fn inspection_expands_records_and_chord_candidates() {
        let maps = maps("app: [{on: [g, p], action: TabPrev, desc: Previous page}]");
        let text = maps.describe(Scope::Logs);
        assert!(text.contains("app\tg p\tApp(TabPrev)\tPrevious page\n"));
        assert!(text.contains("logs\tg g\tLogs(GoTop)\tGo to top\n"));
        assert!(text.contains("Prefix g: candidates [g="));
        let keys: Vec<_> = maps
            .effective_bindings(Scope::Logs)
            .map(|(_, b)| b.key_text.as_str())
            .collect();
        assert!(keys.windows(2).all(|pair| pair[0] <= pair[1]));
        assert_eq!(
            maps.effective_bindings(Scope::App).len(),
            maps.global_bindings().len()
        );
    }
}
