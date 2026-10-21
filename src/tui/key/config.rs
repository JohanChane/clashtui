//! Parse user configuration once, then compile it into the two-level runtime map.

use super::{
    Document, Key, KeyDesc, KeyMap, MaybeMap, SubMap, WhichHints, instancing::defaults::DefaultMap,
};
use anyhow::{Context, Result, bail, ensure};
use serde::de::DeserializeOwned;
use serde_yml::{Mapping, Value};
use std::collections::{HashMap, HashSet};

pub struct Binding<A> {
    pub on: Vec<Key>,
    pub action: Option<A>,
    pub desc: Option<String>,
    pub group: Option<String>,
}

#[derive(Clone)]
pub struct ResolvedBinding<A> {
    pub on: Vec<Key>,
    pub action: A,
    pub desc: String,
    pub group: Option<String>,
}

impl<A> ResolvedBinding<A> {
    pub fn map_action<B>(self, map: impl FnOnce(A) -> B) -> ResolvedBinding<B> {
        ResolvedBinding {
            on: self.on,
            action: map(self.action),
            desc: self.desc,
            group: self.group,
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    on: Value,
    action: Value,
    #[serde(default)]
    desc: Option<String>,
}

fn parse_on(value: Value) -> Result<Vec<Key>> {
    let keys = match value {
        Value::String(s) => vec![s],
        Value::Sequence(seq) => seq
            .into_iter()
            .map(serde_yml::from_value::<String>)
            .collect::<Result<Vec<_>, _>>()?,
        _ => bail!("`on` must be a key string or a list of key strings"),
    };
    ensure!(
        (1..=2).contains(&keys.len()),
        "`on` must contain 1 or 2 keys; modifiers count as part of one key"
    );
    keys.iter().map(|s| s.parse()).collect()
}

fn optional_action<A: DeserializeOwned>(value: Value) -> Result<Option<A>> {
    match value {
        Value::Null => Ok(None),
        Value::String(name) => serde_yml::from_value(Value::String(name.clone()))
            .with_context(|| format!("unknown action `{name}` for this scope"))
            .map(Some),
        _ => bail!("`action` must be a plain action name (e.g. Edit), or null to unbind"),
    }
}

fn parse_entries<A: DeserializeOwned>(value: Value) -> Result<Vec<Binding<A>>> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Sequence(entries) => entries
            .into_iter()
            .enumerate()
            .map(|(i, value)| {
                let parse = || -> Result<Binding<A>> {
                    let entry: Entry = serde_yml::from_value(value)?;
                    Ok(Binding {
                        on: parse_on(entry.on)?,
                        action: optional_action(entry.action).context("invalid action")?,
                        desc: entry.desc,
                        group: None,
                    })
                };
                parse().with_context(|| format!("entry #{}", i + 1))
            })
            .collect(),
        _ => bail!("expected a list of {{on, action, desc}} bindings"),
    }
}

// Check for duplicates (e.g. 's' => Search/Import) before building HashMaps,
// where insertion would otherwise overwrite a conflicting action.
fn validate<A>(bindings: &[Binding<A>]) -> Result<()> {
    let mut seen = HashMap::new();
    for (i, binding) in bindings.iter().enumerate() {
        ensure!(
            (1..=2).contains(&binding.on.len()),
            "entry #{} must contain 1 or 2 keys",
            i + 1
        );
        ensure!(
            binding.action.is_none()
                || binding.on.len() != 2
                || binding.on[1] != Key::from_code(super::KeyCode::Esc),
            "Esc is reserved for cancelling a pending chord; choose another second key"
        );
        if let Some(previous) = seen.insert(binding.on.as_slice(), i) {
            bail!(
                "duplicate key binding `{}` in entries #{} and #{}",
                super::notation::format_sequence(&binding.on),
                previous + 1,
                i + 1
            );
        }
    }
    for binding in bindings
        .iter()
        .filter(|b| b.on.len() == 2 && b.action.is_some())
    {
        if let Some(index) = seen.get(&binding.on[..1])
            && bindings[*index].action.is_some()
        {
            bail!(
                "prefix conflict: `{}` is both a single-key binding and the first key of `{}`",
                binding.on[0],
                super::notation::format_sequence(&binding.on)
            );
        }
    }
    Ok(())
}

/// Apply overrides and remove unbound keys before composing input contexts.
pub fn resolve<A>(defaults: DefaultMap<A>, overrides: Value) -> Result<Vec<ResolvedBinding<A>>>
where
    A: DeserializeOwned + Document,
{
    let defaults = defaults.into_bindings();
    validate(&defaults).context("invalid built-in defaults")?;
    let mut bindings = parse_entries(overrides)?;
    // Duplicate user entries must not disappear when defaults are replaced.
    validate(&bindings)?;
    let replaced: HashSet<_> = bindings.iter().map(|b| b.on.clone()).collect();
    let groups: HashMap<_, _> = defaults
        .iter()
        .filter_map(|b| b.group.as_ref().map(|name| (b.on[0], name.clone())))
        .collect();
    bindings.extend(defaults.into_iter().filter(|b| !replaced.contains(&b.on)));
    bindings.retain(|b| b.action.is_some());
    validate(&bindings)?;

    let mut bindings: Vec<_> = bindings
        .into_iter()
        .map(|binding| {
            let action = binding.action.expect("unbound keys have been removed");
            ResolvedBinding {
                desc: binding.desc.unwrap_or_else(|| action.get_doc().to_owned()),
                group: binding
                    .group
                    .or_else(|| groups.get(&binding.on[0]).cloned()),
                on: binding.on,
                action,
            }
        })
        .collect();
    bindings.sort_by_cached_key(|binding| super::notation::format_sequence(&binding.on));
    Ok(bindings)
}

/// Generate `[(key, description 'about the key's action')]` at runtime,
/// alongside the dispatch map, so user descriptions need no leaked strings.
pub fn compile<A>(bindings: impl IntoIterator<Item = ResolvedBinding<A>>) -> (KeyMap<A>, KeyDesc) {
    let mut map = KeyMap::new();
    let mut docs = Vec::new();
    for binding in bindings {
        let action = binding.action;
        let description = binding.desc;
        docs.push((
            super::notation::format_sequence(&binding.on),
            description.clone(),
        ));
        match binding.on.as_slice() {
            [key] => {
                map.insert(*key, MaybeMap::Action(action));
            }
            [first, second] => {
                let entry = map.entry(*first).or_insert_with(|| {
                    MaybeMap::SubMap(SubMap {
                        hints: WhichHints {
                            prefix: *first,
                            name: binding.group.unwrap_or_else(|| first.to_string()),
                            keys: Vec::new(),
                        },
                        inner: HashMap::new(),
                    })
                });
                if let MaybeMap::SubMap(submap) = entry {
                    submap.inner.insert(*second, action);
                    submap.hints.keys.push((second.to_string(), description));
                } else {
                    unreachable!("prefix conflicts have been validated");
                }
            }
            _ => unreachable!("sequence lengths have been validated"),
        }
    }
    for entry in map.values_mut() {
        if let MaybeMap::SubMap(submap) = entry {
            submap.hints.keys.sort_by(|a, b| a.0.cmp(&b.0));
        }
    }
    docs.sort_by(|a, b| a.0.cmp(&b.0));
    (map, docs)
}

#[cfg(test)]
pub fn merge<A>(defaults: DefaultMap<A>, overrides: Value) -> Result<(KeyMap<A>, KeyDesc)>
where
    A: DeserializeOwned + Document,
{
    resolve(defaults, overrides).map(compile)
}

/// Normalize dev's nested file scopes and optional core-specific sections.
pub fn sections(mut map: Mapping, core: &str) -> Result<Mapping> {
    let mut selected = None;
    for name in ["mihomo", "sing-box"] {
        if let Some(value) = map.remove(name) {
            let Value::Mapping(value) = value else {
                bail!("`{name}` must contain keymap sections")
            };
            if name == core {
                selected = Some(value);
            }
        }
    }
    fn flatten(mut map: Mapping) -> Result<Mapping> {
        for key in map.keys() {
            let key = key.as_str().context("scope name must be a string")?;
            ensure!(
                [
                    "app",
                    "connections",
                    "proxies",
                    "srvctl",
                    "settings",
                    "logs",
                    "file"
                ]
                .contains(&key),
                "unknown keymap scope `{key}`"
            );
        }
        if let Some(file) = map.remove("file") {
            let Value::Mapping(file) = file else {
                bail!("`file` must contain profile/template sections")
            };
            for (name, value) in file {
                let name = name.as_str().context("file scope must be a string")?;
                ensure!(
                    ["profile", "template"].contains(&name),
                    "unknown file scope `{name}`"
                );
                map.insert(format!("files/{name}").into(), value);
            }
        }
        Ok(map)
    }
    let mut common = flatten(map)?;
    if let Some(selected) = selected {
        // As in dev, a core-specific scope replaces the common scope.
        common.extend(flatten(selected)?);
    }
    Ok(common)
}
