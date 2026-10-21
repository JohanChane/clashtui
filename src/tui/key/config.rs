//! Parse user configuration once, then compile it into the two-level runtime map.

use super::{Document, Key, KeyDesc, KeyMap, MaybeMap, instancing::files::FileMap};
use anyhow::{Context, Result, bail, ensure};
use serde::de::DeserializeOwned;
use serde_yml::{Mapping, Value};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

pub struct Binding<A> {
    pub on: Vec<Key>,
    pub action: Option<A>,
    pub desc: Option<String>,
    pub group: Option<String>,
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

fn parse_action<A: DeserializeOwned>(value: Value) -> Result<A> {
    // Accept dev's `action: { Action: Edit }` as well as YAML enum tags.
    let value = match value {
        Value::Mapping(map) if map.len() == 1 => {
            let (tag, value) = map.into_iter().next().unwrap();
            let tag = tag.as_str().context("action variant must be a string")?;
            Value::Tagged(Box::new(serde_yml::value::TaggedValue {
                tag: serde_yml::value::Tag::new(tag),
                value,
            }))
        }
        other => other,
    };
    match serde_yml::from_value(value.clone()) {
        Ok(action) => Ok(action),
        Err(original) => {
            // Keep dev's public action names while internal enums use grouped variants.
            let alias = match value.as_str() {
                Some("SortByHost") => Some(("SortBy", "Host")),
                Some("SortByRule") => Some(("SortBy", "Rule")),
                Some("SortByChains") => Some(("SortBy", "Chains")),
                Some("SortByDownload") => Some(("SortBy", "Download")),
                Some("SortByUpload") => Some(("SortBy", "Upload")),
                Some("SortByDlSpeed") => Some(("SortBy", "DlSpeed")),
                Some("SortByUlSpeed") => Some(("SortBy", "UlSpeed")),
                Some("SortByName") => Some(("Sort", "ByName")),
                Some("SortByDelay") => Some(("Sort", "ByDelay")),
                Some("ResetSort") => Some(("Sort", "Reset")),
                Some("GlobalSortByName") => Some(("Sort", "GlobalByName")),
                Some("GlobalSortByDelay") => Some(("Sort", "GlobalByDelay")),
                Some("GlobalResetSort") => Some(("Sort", "GlobalReset")),
                _ => None,
            };
            if let Some((tag, value)) = alias {
                return Ok(serde_yml::from_value(Value::Tagged(Box::new(
                    serde_yml::value::TaggedValue {
                        tag: serde_yml::value::Tag::new(tag),
                        value: value.into(),
                    },
                )))?);
            }
            if let Value::Tagged(tagged) = value
                && tagged.tag == "Action"
                && matches!(tagged.value.as_str(), Some("GoTop" | "GoEnd"))
            {
                return Ok(serde_yml::from_value(tagged.value)?);
            }
            Err(original.into())
        }
    }
}

fn optional_action<A: DeserializeOwned>(value: Value) -> Result<Option<A>> {
    if value.is_null() {
        Ok(None)
    } else {
        parse_action(value).map(Some)
    }
}

fn legacy_key(value: Value) -> Result<Key> {
    if let Value::String(key) = value {
        return key.parse();
    }
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct LegacyKey {
        code: super::KeyCode,
        #[serde(default = "super::KeyModifiers::empty")]
        modifiers: super::KeyModifiers,
        #[serde(default)]
        shift: bool,
        #[serde(default)]
        ctrl: bool,
        #[serde(default)]
        alt: bool,
        #[serde(default)]
        super_: bool,
    }
    let key: LegacyKey = serde_yml::from_value(value)?;
    let mut modifiers = key.modifiers;
    for (enabled, modifier) in [
        (key.shift, super::KeyModifiers::SHIFT),
        (key.ctrl, super::KeyModifiers::CONTROL),
        (key.alt, super::KeyModifiers::ALT),
        (key.super_, super::KeyModifiers::SUPER),
    ] {
        if enabled {
            modifiers.insert(modifier);
        }
    }
    Ok(Key {
        code: key.code,
        modifiers,
    }
    .normalize())
}

fn parse_entries<A: Copy + Eq + Hash + DeserializeOwned>(value: Value) -> Result<Vec<Binding<A>>> {
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
        Value::Mapping(map) if map.contains_key("common") || map.contains_key("submap") => {
            // Keep files generated by the earlier SubMap branch readable.
            let map: FileMap<A> = serde_yml::from_value(Value::Mapping(map))?;
            Ok(map.into_bindings())
        }
        Value::Mapping(map) => map
            .into_iter()
            .map(|(key, value)| {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                struct WithDesc {
                    action: Value,
                    #[serde(default)]
                    desc: Option<String>,
                }
                let (action, desc) = if value.as_mapping().is_some_and(|m| m.contains_key("action"))
                {
                    let value: WithDesc = serde_yml::from_value(value)?;
                    (value.action, value.desc)
                } else {
                    (value, None)
                };
                Ok(Binding {
                    on: vec![legacy_key(key)?],
                    action: optional_action(action)?,
                    desc,
                    group: None,
                })
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

/// Generate `[(key, description 'about the key's action')]` at runtime,
/// alongside the dispatch map, so user descriptions need no leaked strings.
pub fn merge<A>(defaults: FileMap<A>, overrides: Value) -> Result<(KeyMap<A>, KeyDesc)>
where
    A: Copy + Eq + Hash + DeserializeOwned + Document,
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

    let mut map = KeyMap::new();
    let mut docs = Vec::new();
    for binding in bindings {
        let action = binding.action.expect("unbound keys have been removed");
        docs.push((
            super::notation::format_sequence(&binding.on),
            binding.desc.unwrap_or_else(|| action.get_doc().to_owned()),
        ));
        match binding.on.as_slice() {
            [key] => {
                map.insert(*key, MaybeMap::Action(action));
            }
            [first, second] => {
                let entry = map.entry(*first).or_insert_with(|| MaybeMap::SubMap {
                    name: binding
                        .group
                        .or_else(|| groups.get(first).cloned())
                        .unwrap_or_else(|| first.to_string()),
                    inner: HashMap::new(),
                });
                if let MaybeMap::SubMap { inner, .. } = entry {
                    inner.insert(*second, action);
                } else {
                    unreachable!("prefix conflicts have been validated");
                }
            }
            _ => unreachable!("sequence lengths have been validated"),
        }
    }
    docs.sort_by(|a, b| a.0.cmp(&b.0));
    Ok((map, docs))
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
                let key = format!("files/{name}");
                ensure!(!map.contains_key(&key), "scope `{key}` is configured twice");
                map.insert(key.into(), value);
            }
        }
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
                    "files/profile",
                    "files/template"
                ]
                .contains(&key),
                "unknown keymap scope `{key}`"
            );
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
