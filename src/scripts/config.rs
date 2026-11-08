//! Read and validate scripts.json once; retain definitions, not execution state.
use super::variables::{validate_host_path, validate_template};
use crate::config::CoreType;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
};
pub const FILE_NAME: &str = "scripts.json";

#[derive(Debug)]
pub struct ScriptConfig {
    pub(super) config_root: PathBuf,
    pub(super) root: PathBuf,
    pub(super) actions: Vec<ActionDefinition>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionDefinition {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_cores"
    )]
    pub cores: Option<Vec<CoreType>>,
    pub cmd: CommandTemplate,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandTemplate {
    pub program: String,
    pub args: Vec<String>,
    #[serde(default = "default_interactive")]
    pub interactive: bool,
    #[serde(default)]
    pub output: OutputMode,
    #[serde(default)]
    pub keep_children_on_success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}

const fn default_interactive() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    #[default]
    Popup,
    Silent,
    Terminal,
}

impl Default for CommandTemplate {
    fn default() -> Self {
        Self {
            program: String::new(),
            args: Vec::new(),
            interactive: true,
            output: OutputMode::Popup,
            keep_children_on_success: false,
            timeout_secs: None,
        }
    }
}

fn deserialize_cores<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Vec<CoreType>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // Omission means all cores; an explicit null is not an array.
    Vec::<CoreType>::deserialize(deserializer).map(Some)
}

/// Check existing ancestors as well as the target; dangling symlinks count too.
fn reject_symlinks(path: &Path) -> Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        // A Windows prefix such as \\?\C: is not a complete filesystem path.
        // Inspect it after the root or next component has been appended.
        if matches!(component, Component::Prefix(_)) {
            continue;
        }
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => ensure!(
                !metadata.file_type().is_symlink(),
                "Refusing symbolic link: {}",
                current.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Failed to inspect {}", current.display()));
            }
        }
    }
    Ok(())
}

pub fn load(config_root: &Path) -> Result<ScriptConfig> {
    let path = config_root.join(FILE_NAME);
    reject_symlinks(&path)?;
    let text = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "Failed to read {}; run the installer or create scripts.json manually",
            path.display()
        )
    })?;
    parse(config_root, &text).with_context(|| format!("Invalid {}", path.display()))
}

pub(super) fn parse(config_root: &Path, text: &str) -> Result<ScriptConfig> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Manifest {
        root: String,
        actions: Vec<ActionDefinition>,
    }
    let Manifest { root, actions } =
        serde_json::from_str(text).context("Invalid script configuration JSON")?;
    ensure!(!root.trim().is_empty(), "root: directory must not be empty");
    ensure!(!root.contains('\0'), "root: NUL is not allowed");
    validate_host_path(&root)?;
    validate(&actions)?;
    let config_root = std::path::absolute(config_root)?;
    let root = config_root.join(root);
    Ok(ScriptConfig {
        config_root,
        root,
        actions,
    })
}

impl ActionDefinition {
    pub(super) fn applies_to(&self, core: CoreType) -> bool {
        self.cores
            .as_ref()
            .is_none_or(|cores| cores.contains(&core))
    }
}

fn validate(actions: &[ActionDefinition]) -> Result<()> {
    for (index, action) in actions.iter().enumerate() {
        let field = format!("actions[{index}]");
        let mut id = action.id.bytes();
        ensure!(
            id.next().is_some_and(|c| c.is_ascii_lowercase())
                && id.all(|c| c.is_ascii_lowercase()
                    || c.is_ascii_digit()
                    || c == b'_'
                    || c == b'-'),
            "{field}.id: expected [a-z][a-z0-9_-]*"
        );
        if let Some(label) = &action.label {
            ensure!(!label.trim().is_empty(), "{field}.label: must not be empty");
        }
        if let Some(cores) = &action.cores {
            ensure!(!cores.is_empty(), "{field}.cores: must not be empty");
            for (i, core) in cores.iter().enumerate() {
                ensure!(
                    !cores[..i].contains(core),
                    "{field}.cores[{i}]: duplicate core {}",
                    core.id()
                );
            }
        }
        ensure!(
            !action.cmd.program.trim().is_empty(),
            "{field}.cmd.program: must not be empty"
        );
        ensure!(
            action.cmd.timeout_secs != Some(0),
            "{field}.cmd.timeout_secs: must be positive; omit for no timeout"
        );
        validate_template(&action.cmd.program).with_context(|| format!("{field}.cmd.program"))?;
        for (i, arg) in action.cmd.args.iter().enumerate() {
            validate_template(arg).with_context(|| format!("{field}.cmd.args[{i}]"))?;
        }
    }
    for core in CoreType::ALL {
        let mut seen = HashMap::new();
        for (index, action) in actions.iter().enumerate() {
            if action.applies_to(core)
                && let Some(first) = seen.insert(action.id.as_str(), index)
            {
                bail!(
                    "actions[{index}].cores: ({}, {}) overlaps actions[{first}]",
                    core.id(),
                    action.id
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn manifest(actions: serde_json::Value) -> Result<ScriptConfig> {
        parse(
            &std::env::temp_dir(),
            &serde_json::json!({"root":"scripts", "actions": actions}).to_string(),
        )
    }
    #[test]
    fn scopes_reject_overlap_and_preserve_menu_order() {
        let action = |label: &str, cores: serde_json::Value| serde_json::json!({"id":"start_core","label":label,"cores":cores,"cmd":{"program":"tool","args":[]}});
        let config = manifest(serde_json::json!([
            action("M", serde_json::json!(["mihomo"])),
            action("S", serde_json::json!(["singbox"]))
        ]))
        .unwrap();
        assert_eq!(
            config
                .menu(CoreType::Singbox)
                .next()
                .unwrap()
                .label
                .as_deref(),
            Some("S")
        );
        assert!(
            manifest(serde_json::json!([
                action("M", serde_json::json!(["mihomo"])),
                action("Both", serde_json::json!(["mihomo", "singbox"]))
            ]))
            .is_err()
        );
        assert!(config.config_root.is_absolute());
        assert_eq!(config.root, std::env::temp_dir().join("scripts"));
    }
    #[test]
    fn malformed_fields_and_templates_fail_before_execution() {
        let valid = serde_json::json!({"id":"test","cmd":{"program":"tool","args":[]}});
        for patch in [
            serde_json::json!({"cores":null}),
            serde_json::json!({"cores":[]}),
            serde_json::json!({"cores":["mihomo","mihomo"]}),
            serde_json::json!({"cores":["unknown"]}),
            serde_json::json!({"unexpected":true}),
            serde_json::json!({"id":""}),
        ] {
            let mut action = valid.clone();
            action
                .as_object_mut()
                .unwrap()
                .extend(patch.as_object().unwrap().clone());
            assert!(manifest(serde_json::json!([action])).is_err());
        }
        for cmd in [
            serde_json::json!({"program":"tool"}),
            serde_json::json!({"program":"","args":[]}),
            serde_json::json!({"program":"tool","args":["${unknown}"]}),
            serde_json::json!({"program":"tool","args":["${root"]}),
            serde_json::json!({"program":"tool","args":[],"timeout_secs":0}),
            serde_json::json!({"program":"tool","args":[],"output":"bad"}),
        ] {
            assert!(manifest(serde_json::json!([{"id":"test","cmd":cmd}])).is_err());
        }
    }
    #[test]
    fn loaded_definition_does_not_change_when_file_changes() {
        let dir = std::env::temp_dir().join(format!("clashtui-config-{}", fastrand::u64(..)));
        std::fs::create_dir(&dir).unwrap();
        let dir = dir.canonicalize().unwrap();
        let path = dir.join(FILE_NAME);
        std::fs::write(
            &path,
            r#"{"root":".","actions":[{"id":"copy","cmd":{"program":"original","args":[]}}]}"#,
        )
        .unwrap();
        let config = load(&dir).unwrap();
        std::fs::write(&path, "invalid").unwrap();
        assert_eq!(config.actions[0].cmd.program, "original");
        assert!(load(&dir).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
