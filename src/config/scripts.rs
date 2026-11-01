//! Shared, platform-independent script command configuration.

use super::CoreType;
use crate::functions::command::sanitize_windows_path;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Component, Path, PathBuf},
};

pub const FILE_NAME: &str = "scripts.json";
const VARIABLES: [&str; 8] = [
    "root",
    "config_root",
    "core_type",
    "service_name",
    "core_bin",
    "core_config_dir",
    "core_config_path",
    "proxy_port",
];

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptConfig {
    pub root: String,
    pub actions: Vec<ScriptAction>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptAction {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_cores"
    )]
    pub cores: Option<Vec<CoreType>>,
    pub cmd: ScriptCommand,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptCommand {
    pub program: String,
    pub args: Vec<String>,
    #[serde(default = "default_interactive")]
    pub interactive: bool,
    #[serde(default)]
    pub output: CommandOutput,
    #[serde(default)]
    pub keep_children_on_success: bool,
}

const fn default_interactive() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandOutput {
    #[default]
    Popup,
    Silent,
    Terminal,
}

impl Default for ScriptCommand {
    fn default() -> Self {
        Self {
            program: String::new(),
            args: Vec::new(),
            interactive: true,
            output: CommandOutput::Popup,
            keep_children_on_success: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedCommand {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub cwd: PathBuf,
    pub interactive: bool,
    pub output: CommandOutput,
    pub keep_children_on_success: bool,
}

impl Default for ResolvedCommand {
    fn default() -> Self {
        Self {
            program: OsString::new(),
            args: Vec::new(),
            cwd: PathBuf::new(),
            interactive: true,
            output: CommandOutput::Popup,
            keep_children_on_success: false,
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

impl ScriptAction {
    pub fn applies_to(&self, core: CoreType) -> bool {
        self.cores
            .as_ref()
            .is_none_or(|cores| cores.contains(&core))
    }
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

impl ScriptConfig {
    pub fn load(config_root: &Path) -> Result<Self> {
        let path = config_root.join(FILE_NAME);
        reject_symlinks(&path)?;
        let text = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "Failed to read {}; run the installer or create scripts.json manually",
                path.display()
            )
        })?;
        Self::parse(&text).with_context(|| format!("Invalid {}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Self> {
        let config: Self =
            serde_json::from_str(text).context("Invalid script configuration JSON")?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.root.trim().is_empty(),
            "root: directory must not be empty"
        );
        ensure!(!self.root.contains('\0'), "root: NUL is not allowed");
        validate_host_path(&self.root).context("root")?;
        for (index, action) in self.actions.iter().enumerate() {
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
            validate_template(&action.cmd.program)
                .with_context(|| format!("{field}.cmd.program"))?;
            for (i, arg) in action.cmd.args.iter().enumerate() {
                validate_template(arg).with_context(|| format!("{field}.cmd.args[{i}]"))?;
            }
        }
        for core in CoreType::ALL {
            let mut seen = HashMap::new();
            for (index, action) in self.actions.iter().enumerate() {
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

    pub fn actions_for(&self, core: CoreType) -> impl Iterator<Item = &ScriptAction> {
        self.actions
            .iter()
            .filter(move |action| action.applies_to(core))
    }

    pub fn find(&self, core: CoreType, id: &str) -> Option<&ScriptAction> {
        self.actions_for(core).find(|action| action.id == id)
    }

    pub fn resolve(
        &self,
        config_root: &Path,
        action: &ScriptAction,
        mut lookup: impl FnMut(&str) -> Result<String>,
    ) -> Result<ResolvedCommand> {
        self.validate()?;
        let config_root = std::path::absolute(config_root).context("Resolve CONFIG_ROOT")?;
        let root = Path::new(&self.root);
        let root = if root.is_absolute() {
            root.to_path_buf()
        } else {
            config_root.join(root)
        };
        let mut values = HashMap::<String, String>::new();
        let mut variable = |name: &str| -> Result<String> {
            if let Some(value) = values.get(name) {
                return Ok(value.clone());
            }
            let value = match name {
                "root" => root.to_str().context("root is not valid UTF-8")?.to_owned(),
                "config_root" => config_root
                    .to_str()
                    .context("CONFIG_ROOT is not valid UTF-8")?
                    .to_owned(),
                _ => lookup(name).with_context(|| format!("Failed to resolve ${{{name}}}"))?,
            };
            ensure!(!value.is_empty(), "${{{name}}}: required value is empty");
            ensure!(!value.contains('\0'), "${{{name}}}: NUL is not allowed");
            // Convert typed paths before embedding them in arguments for external tools.
            let value = if cfg!(windows)
                && matches!(
                    name,
                    "root" | "config_root" | "core_bin" | "core_config_dir" | "core_config_path"
                ) {
                sanitize_windows_path(&value)
            } else {
                value
            };
            values.insert(name.to_owned(), value.clone());
            Ok(value)
        };
        let program = expand(&action.cmd.program, &mut variable)
            .with_context(|| format!("action {} cmd.program", action.id))?;
        ensure!(
            !program.trim().is_empty(),
            "action {}: empty program",
            action.id
        );
        validate_host_path(&program)
            .with_context(|| format!("action {} cmd.program", action.id))?;
        let program_path = Path::new(&program);
        let program = if program_path.is_absolute() {
            program_path.as_os_str().to_owned()
        } else if program.contains('/') || (cfg!(windows) && program.contains('\\')) {
            root.join(program_path).into_os_string()
        } else {
            // Command performs PATH lookup for a bare program name.
            OsString::from(program)
        };
        let args = action
            .cmd
            .args
            .iter()
            .enumerate()
            .map(|(index, arg)| {
                expand(arg, &mut variable)
                    .map(OsString::from)
                    .with_context(|| format!("action {} cmd.args[{index}]", action.id))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(ResolvedCommand {
            program,
            args,
            interactive: action.cmd.interactive,
            output: action.cmd.output,
            keep_children_on_success: action.cmd.keep_children_on_success,
            cwd: if cfg!(windows) {
                sanitize_windows_path(root.to_str().context("root is not valid UTF-8")?).into()
            } else {
                root
            },
        })
    }
}

fn validate_template(text: &str) -> Result<()> {
    expand(text, &mut |_| Ok(String::new())).map(|_| ())
}

fn expand(text: &str, lookup: &mut impl FnMut(&str) -> Result<String>) -> Result<String> {
    ensure!(!text.contains('\0'), "NUL is not allowed");
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find('$') {
        output.push_str(&rest[..index]);
        rest = &rest[index..];
        if let Some(next) = rest.strip_prefix("$${") {
            output.push_str("${");
            rest = next;
        } else if let Some(next) = rest.strip_prefix("${") {
            let end = next.find('}').context("Unclosed ${variable}")?;
            let name = &next[..end];
            ensure!(VARIABLES.contains(&name), "Unknown variable ${{{name}}}");
            output.push_str(&lookup(name)?);
            rest = &next[end + 1..];
        } else {
            output.push('$');
            rest = &rest[1..];
        }
    }
    output.push_str(rest);
    Ok(output)
}

pub(crate) fn validate_host_path(path: &str) -> Result<()> {
    if cfg!(windows) {
        validate_windows_path(path)?;
    }
    Ok(())
}

fn validate_windows_path(path: &str) -> Result<()> {
    let bytes = path.as_bytes();
    let drive_relative = bytes.len() >= 2
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && !matches!(bytes.get(2), Some(b'/' | b'\\'));
    let rooted_without_drive = (path.starts_with('\\') && !path.starts_with("\\\\"))
        || (path.starts_with('/') && !path.starts_with("//"));
    ensure!(
        !drive_relative && !rooted_without_drive,
        "Path depends on the current Windows drive: {path}"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn action(id: &str, cores: Option<Vec<CoreType>>) -> ScriptAction {
        ScriptAction {
            id: id.into(),
            label: Some(id.into()),
            cores,
            cmd: ScriptCommand {
                program: "test-command".into(),
                args: vec![],
                ..Default::default()
            },
        }
    }

    fn config(actions: Vec<ScriptAction>) -> ScriptConfig {
        ScriptConfig {
            root: "scripts/coresrvctrl".into(),
            actions,
        }
    }

    #[test]
    fn execution_options_default_validate_and_resolve() {
        let mut config = config(vec![action("copy", None)]);
        let bare = r#"{"program":"tool","args":[]}"#;
        let cmd: ScriptCommand = serde_json::from_str(bare).unwrap();
        assert!(cmd.interactive);
        assert_eq!(cmd.output, CommandOutput::Popup);
        assert!(!cmd.keep_children_on_success);
        for output in ["popup", "silent", "terminal"] {
            config.actions[0].cmd = serde_json::from_value(json!({
                "program": "tool", "args": [], "interactive": false,
                "output": output, "keep_children_on_success": true,
            }))
            .unwrap();
            let resolved = config
                .resolve(
                    &std::env::temp_dir(),
                    &config.actions[0],
                    |_| unreachable!(),
                )
                .unwrap();
            assert!(!resolved.interactive);
            assert!(resolved.keep_children_on_success);
            assert_eq!(resolved.output, config.actions[0].cmd.output);
        }
        for (key, value) in [
            ("interactive", json!(null)),
            ("interactive", json!("false")),
            ("output", json!("return")),
            ("output", json!(true)),
            ("keep_children_on_success", json!(null)),
            ("show_output", json!(true)),
        ] {
            let mut value_cmd: serde_json::Value = serde_json::from_str(bare).unwrap();
            value_cmd[key] = value;
            assert!(
                serde_json::from_value::<ScriptCommand>(value_cmd).is_err(),
                "{key}"
            );
        }
    }

    #[test]
    fn load_from_canonicalized_config_root() {
        let root = std::env::temp_dir().join(format!(
            "clashtui-scripts-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        std::fs::create_dir(&root).unwrap();
        let expected = config(vec![action("start_core", None)]);
        std::fs::write(root.join(FILE_NAME), serde_json::to_vec(&expected).unwrap()).unwrap();

        // Match config::init, including the verbatim path prefix on Windows.
        let config_root = root.canonicalize().unwrap();
        let loaded = ScriptConfig::load(&config_root);

        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(loaded.unwrap(), expected);
    }

    #[test]
    fn disjoint_scopes_preserve_order_and_choose_target() {
        let config = config(vec![
            action("start_core", None),
            action("restart_core", Some(vec![CoreType::Singbox])),
            action("restart_core", Some(vec![CoreType::Mihomo])),
            action("custom-action", Some(vec![CoreType::Mihomo])),
        ]);
        config.validate().unwrap();
        assert_eq!(
            config
                .actions_for(CoreType::Mihomo)
                .map(|a| a.id.as_str())
                .collect::<Vec<_>>(),
            ["start_core", "restart_core", "custom-action"]
        );
        assert_eq!(
            config.find(CoreType::Singbox, "restart_core"),
            Some(&config.actions[1])
        );
        assert!(config.find(CoreType::Singbox, "custom-action").is_none());
    }

    #[test]
    fn conflicting_scopes_are_rejected_before_filtering() {
        for scopes in [
            None,
            Some(vec![CoreType::Mihomo]),
            Some(CoreType::ALL.to_vec()),
        ] {
            let config = config(vec![
                action("start_core", scopes),
                action("start_core", Some(vec![CoreType::Mihomo])),
            ]);
            let error = config.validate().unwrap_err().to_string();
            assert!(error.contains("actions[1].cores"), "{error}");
            assert!(error.contains("mihomo"), "{error}");
        }
    }

    #[test]
    fn unlabeled_actions_remain_resolvable_and_serialize_without_a_label() {
        for label in [None, Some(json!(null))] {
            let mut value = json!({"root":".", "actions":[{
                "id":"status_core", "cmd":{"program":"tool", "args":["${core_type}"]}
            }]});
            if let Some(label) = label {
                value["actions"][0]["label"] = label;
            }
            let config = ScriptConfig::parse(&value.to_string()).unwrap();
            let action = config.find(CoreType::Mihomo, "status_core").unwrap();
            assert!(action.label.is_none());
            let command = config
                .resolve(Path::new("."), action, |_| Ok("mihomo".into()))
                .unwrap();
            assert_eq!(command.program, "tool");
            assert_eq!(command.args, ["mihomo"]);
            assert!(
                serde_json::to_value(&config).unwrap()["actions"][0]
                    .get("label")
                    .is_none()
            );
        }
    }

    #[test]
    fn malformed_schema_and_core_arrays_are_rejected() {
        let valid = json!({"root":"scripts", "actions":[{
            "id":"start_core", "cmd":{"program":"sh", "args":[]}
        }]});
        for cores in [
            json!([]),
            json!(null),
            json!(["sing-box"]),
            json!(["MIHOMO"]),
            json!(["mihomo", "mihomo"]),
        ] {
            let mut value = valid.clone();
            value["actions"][0]["cores"] = cores;
            assert!(ScriptConfig::parse(&value.to_string()).is_err());
        }
        for (field, bad) in [
            ("label", json!("")),
            ("label", json!(" ")),
            ("label", json!(42)),
            ("id", json!("1bad")),
            ("unknown", json!(true)),
        ] {
            let mut value = valid.clone();
            value["actions"][0][field] = bad;
            assert!(ScriptConfig::parse(&value.to_string()).is_err());
        }
        for bad in [json!(null), json!({}), json!([1])] {
            let mut value = valid.clone();
            value["actions"][0]["cmd"]["args"] = bad;
            assert!(ScriptConfig::parse(&value.to_string()).is_err());
        }
        assert!(ScriptConfig::parse(r#"{"root":"scripts","actions":[]}"#).is_ok());
        assert!(ScriptConfig::parse(r#"{"root":"scripts","actions":[],"version":1}"#).is_err());
    }

    #[test]
    fn paths_are_independent_of_the_invocation_directory() {
        let base = std::env::temp_dir().join("clashtui script root");
        let config = config(vec![action("start_core", None)]);
        for (program, expected) in [
            ("sudo".to_owned(), OsString::from("sudo")),
            (
                "./service.sh".to_owned(),
                base.join(&config.root)
                    .join("./service.sh")
                    .into_os_string(),
            ),
            (
                "systemd/service.sh".to_owned(),
                base.join(&config.root)
                    .join("systemd/service.sh")
                    .into_os_string(),
            ),
            (
                base.join("absolute.sh").to_string_lossy().into_owned(),
                base.join("absolute.sh").into_os_string(),
            ),
        ] {
            let mut action = config.actions[0].clone();
            action.cmd.program = program;
            let plan = config
                .resolve(&base, &action, |_| bail!("unexpected lookup"))
                .unwrap();
            assert_eq!(plan.program, expected);
            assert_eq!(plan.cwd, base.join(&config.root));
        }
    }

    #[test]
    fn root_is_literal_and_absolute_root_is_preserved() {
        let base = std::env::temp_dir();
        let mut config = config(vec![action("test", None)]);
        config.root = "~/$HOME/scripts".into();
        let plan = config
            .resolve(&base, &config.actions[0], |_| bail!("unexpected lookup"))
            .unwrap();
        assert_eq!(plan.cwd, base.join("~/$HOME/scripts"));
        config.root = base.join("absolute root").to_string_lossy().into_owned();
        let plan = config
            .resolve(&base, &config.actions[0], |_| bail!("unexpected lookup"))
            .unwrap();
        assert_eq!(plan.cwd, PathBuf::from(&config.root));
    }

    #[test]
    fn replacement_is_single_pass_and_preserves_each_argument() {
        let mut action = action("test", None);
        action.cmd.args = vec![
            "${service_name}".into(),
            "--name=${service_name}".into(),
            "$${unknown}".into(),
            "".into(),
            "${root}/中文 file.sh".into(),
        ];
        let config = config(vec![action]);
        let base = std::env::temp_dir();
        let mut lookups = Vec::new();
        let plan = config
            .resolve(&base, &config.actions[0], |name| {
                lookups.push(name.to_owned());
                Ok("a b ' \" $(touch nope) ${core_type}".into())
            })
            .unwrap();
        assert_eq!(lookups, ["service_name"]);
        assert_eq!(plan.args[0], "a b ' \" $(touch nope) ${core_type}");
        assert_eq!(plan.args[1], "--name=a b ' \" $(touch nope) ${core_type}");
        assert_eq!(plan.args[2], "${unknown}");
        assert_eq!(plan.args[3], "");
        let root = base.join(&config.root).display().to_string();
        let root = if cfg!(windows) {
            sanitize_windows_path(&root)
        } else {
            root
        };
        assert_eq!(plan.args[4], OsString::from(format!("{root}/中文 file.sh")));
    }

    #[test]
    fn missing_and_unknown_variables_fail_without_resolving_unused_values() {
        let mut action = action("test", None);
        action.cmd.args = vec!["${service_name}".into()];
        let config = config(vec![action]);
        let plan = config
            .resolve(Path::new("."), &config.actions[0], |name| {
                assert_eq!(name, "service_name");
                Ok("only-name-configured".into())
            })
            .unwrap();
        assert_eq!(plan.args, [OsString::from("only-name-configured")]);
        assert!(
            config
                .resolve(Path::new("."), &config.actions[0], |_| Ok(String::new()))
                .is_err()
        );
        for template in ["${unknown}", "${service_name", "\0"] {
            let mut invalid = config.clone();
            invalid.actions[0].cmd.args[0] = template.into();
            let error = format!("{:#}", invalid.validate().unwrap_err());
            assert!(error.contains("actions[0].cmd.args[0]"), "{error}");
        }
    }

    #[test]
    fn windows_paths_must_not_depend_on_current_drive() {
        for invalid in [r"C:foo", "C:", r"\foo", "/foo"] {
            assert!(validate_windows_path(invalid).is_err(), "{invalid}");
        }
        for valid in [
            r"C:\Windows\cmd.exe",
            "C:/Windows/cmd.exe",
            r"\\server\share\script.exe",
            "//server/share/script.exe",
            "powershell.exe",
            "./script.ps1",
        ] {
            validate_windows_path(valid).unwrap();
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_paths_are_sanitized_without_changing_other_arguments() {
        let base = Path::new(r"\\?\C:\Users\中文 user\clashtui");
        let mut action = action("test", None);
        action.cmd.args = vec![
            "${root}/windows/service.ps1".into(),
            "${config_root}".into(),
            "${core_bin}".into(),
            "${core_config_dir}".into(),
            "${core_config_path}".into(),
            "${service_name}".into(),
            r"literal=\\?\C:\unchanged".into(),
        ];
        let config = config(vec![action]);
        let command = config
            .resolve(base, &config.actions[0], |name| {
                Ok(match name {
                    "core_bin" => r"\\?\C:\core\mihomo.exe",
                    "core_config_dir" => r"\\?\C:\core\data",
                    "core_config_path" => r"\\?\C:\core\data\config.yaml",
                    "service_name" => r"\\?\C:\unchanged",
                    _ => unreachable!(),
                }
                .into())
            })
            .unwrap();
        assert_eq!(
            command.args,
            [
                "C:/Users/中文 user/clashtui/scripts/coresrvctrl/windows/service.ps1",
                "C:/Users/中文 user/clashtui",
                "C:/core/mihomo.exe",
                "C:/core/data",
                "C:/core/data/config.yaml",
                r"\\?\C:\unchanged",
                r"literal=\\?\C:\unchanged",
            ]
            .map(OsString::from)
        );
        assert_eq!(
            command.cwd,
            Path::new("C:/Users/中文 user/clashtui/scripts/coresrvctrl")
        );
    }
}
