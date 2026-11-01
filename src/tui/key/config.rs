//! Parse, validate and resolve configuration before building runtime lookup tables.

use super::{DefaultBindings, Document, KeySequence, Scope};
use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;
use serde_yml::{Mapping, Value};
use std::collections::{HashMap, HashSet};

struct Binding<A> {
    on: KeySequence,
    action: Option<A>,
    desc: Option<String>,
}

pub struct ResolvedBinding<A> {
    pub on: KeySequence,
    // Keep explicit unbindings so input fallbacks cannot revive them.
    pub action: Option<A>,
    pub desc: String,
}

impl<A> ResolvedBinding<A> {
    pub fn map_action<B>(self, map: impl FnOnce(A) -> B) -> ResolvedBinding<B> {
        ResolvedBinding {
            on: self.on,
            action: self.action.map(map),
            desc: self.desc,
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

fn parse_on(value: Value) -> Result<KeySequence> {
    let keys = match value {
        Value::String(s) => vec![s],
        Value::Sequence(seq) => seq
            .into_iter()
            .map(serde_yml::from_value::<String>)
            .collect::<Result<Vec<_>, _>>()?,
        _ => bail!("`on` must be a key string or a list of key strings"),
    };
    match keys.as_slice() {
        [key] => Ok(KeySequence::Single(key.parse()?)),
        [first, second] => Ok(KeySequence::Chord([first.parse()?, second.parse()?])),
        _ => bail!("`on` must contain 1 or 2 keys; modifiers count as part of one key"),
    }
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
        if binding.action.is_some() {
            binding.on.validate_action()?;
        }
        if let Some(previous) = seen.insert(binding.on, i) {
            bail!(
                "duplicate key binding `{}` in entries #{} and #{}",
                super::notation::format_sequence(binding.on.as_slice()),
                previous + 1,
                i + 1
            );
        }
    }
    for binding in bindings.iter().filter(|b| b.action.is_some()) {
        if let KeySequence::Chord([first, _]) = binding.on
            && let Some(index) = seen.get(&KeySequence::Single(first))
            && bindings[*index].action.is_some()
        {
            bail!(
                "prefix conflict: `{}` is both a single-key binding and the first key of `{}`",
                first,
                super::notation::format_sequence(binding.on.as_slice())
            );
        }
    }
    Ok(())
}

/// Apply overrides, retaining unbound keys as fallback guards.
pub fn resolve<A>(defaults: DefaultBindings<A>, overrides: Value) -> Result<Vec<ResolvedBinding<A>>>
where
    A: DeserializeOwned + Document,
{
    let defaults: Vec<_> = defaults
        .bindings
        .into_iter()
        .map(|(on, action)| Binding {
            on,
            action: Some(action),
            desc: None,
        })
        .collect();
    validate(&defaults).context("invalid built-in defaults")?;
    let mut bindings = parse_entries(overrides)?;
    // Duplicate user entries must not disappear when defaults are replaced.
    validate(&bindings)?;
    let replaced: HashSet<_> = bindings.iter().map(|b| b.on).collect();
    bindings.extend(defaults.into_iter().filter(|b| !replaced.contains(&b.on)));
    validate(&bindings)?;

    Ok(bindings
        .into_iter()
        .map(|binding| ResolvedBinding {
            desc: binding.desc.unwrap_or_else(|| {
                binding
                    .action
                    .as_ref()
                    .map_or("", Document::get_doc)
                    .to_owned()
            }),
            on: binding.on,
            action: binding.action,
        })
        .collect())
}

/// Select core-specific overrides and resolve nested file scopes to typed scopes.
pub fn sections(mut map: Mapping, core: &str) -> Result<HashMap<Scope, Value>> {
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
    let mut common = parse_sections(map)?;
    if let Some(selected) = selected {
        // A core-specific scope replaces the common scope before defaults are merged.
        common.extend(parse_sections(selected)?);
    }
    Ok(common)
}

fn parse_sections(map: Mapping) -> Result<HashMap<Scope, Value>> {
    let mut sections = HashMap::new();
    for (name, value) in map {
        let name = name.as_str().context("scope name must be a string")?;
        let scope = match name {
            "app" => Scope::App,
            "connections" => Scope::Connections,
            "proxies" => Scope::Proxies,
            "srvctl" => Scope::Srvctl,
            "settings" => Scope::Settings,
            "logs" => Scope::Logs,
            "file" => {
                let Value::Mapping(file) = value else {
                    bail!("`file` must contain profile/template sections")
                };
                for (name, value) in file {
                    let name = name.as_str().context("file scope must be a string")?;
                    let scope = match name {
                        "profile" => Scope::Profile,
                        "template" => Scope::Template,
                        _ => bail!("unknown file scope `{name}`"),
                    };
                    sections.insert(scope, value);
                }
                continue;
            }
            _ => bail!("unknown keymap scope `{name}`"),
        };
        sections.insert(scope, value);
    }
    Ok(sections)
}
