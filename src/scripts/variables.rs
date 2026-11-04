//! Resolve variables lazily for one action; only proxy_port is cached.
use super::config::ScriptConfig;
use crate::config::{CONFIG, CoreType};
use anyhow::{Context, Result, bail, ensure};
use std::path::Path;
const VARIABLES: [&str; 9] = [
    "root",
    "config_root",
    "core_type",
    "service_name",
    "core_bin",
    "core_config_dir",
    "core_config_path",
    "proxy_port",
    "path",
];

pub(super) struct ScriptContext<'a> {
    pub config: &'a ScriptConfig,
    pub core: CoreType,
    pub path: Option<&'a Path>,
    pub proxy_port: Option<u16>,
}
impl ScriptContext<'_> {
    pub fn expand(&mut self, template: &str) -> Result<String> {
        expand(template, &mut |name| {
            let value = self
                .value(name)
                .with_context(|| format!("Failed to resolve ${{{name}}}"))?;
            ensure!(!value.is_empty(), "${{{name}}}: required value is empty");
            ensure!(!value.contains('\0'), "${{{name}}}: NUL is not allowed");
            Ok(
                if cfg!(windows)
                    && matches!(
                        name,
                        "root"
                            | "config_root"
                            | "core_bin"
                            | "core_config_dir"
                            | "core_config_path"
                            | "path"
                    )
                {
                    sanitize_windows_path(&value)
                } else {
                    value
                },
            )
        })
    }

    fn value(&mut self, name: &str) -> Result<String> {
        match name {
            "root" => return path_text(&self.config.root),
            "config_root" => return path_text(&self.config.config_root),
            "core_type" => return Ok(self.core.id().into()),
            "path" => {
                return path_text(&std::path::absolute(
                    self.path.context("This action has no target path")?,
                )?);
            }
            _ => (),
        }
        let (paths, service, controller, secret) = match self.core {
            CoreType::Mihomo => (
                &CONFIG.cfg_file.mihomo.core,
                &CONFIG.cfg_file.mihomo.core_service,
                &CONFIG.external_controller,
                &CONFIG.secret,
            ),
            CoreType::Singbox => (
                &CONFIG.cfg_file.singbox.core,
                &CONFIG.cfg_file.singbox.core_service,
                &CONFIG.singbox_external_controller,
                &CONFIG.singbox_secret,
            ),
        };
        let raw = match name {
            "service_name" => &service.service_name,
            "core_bin" => &paths.bin_path,
            "core_config_dir" => &paths.config_dir,
            "core_config_path" => &paths.config_path,
            "proxy_port" => {
                if self.proxy_port.is_none() {
                    let response = crate::functions::restful::config::fetch_from(
                        controller,
                        secret.as_deref(),
                    )
                    .context("Cannot query target core proxy port")?;
                    self.proxy_port = Some(proxy_port(
                        valid_port(response.get("mixed-port")),
                        valid_port(response.get("port")),
                    )?);
                }
                return Ok(self.proxy_port.unwrap().to_string());
            }
            _ => bail!("Unknown script variable: {name}"),
        };
        ensure!(!raw.is_empty(), "{}: missing ${{{name}}}", self.core.id());
        if name == "service_name" {
            return Ok(raw.clone());
        }
        let path = Path::new(raw);
        if name == "core_bin" && !path.is_absolute() && path.components().count() == 1 {
            let search = std::env::var_os("PATH").context("PATH is missing for core_bin lookup")?;
            let mut names = vec![raw.clone()];
            #[cfg(windows)]
            if path.extension().is_none() {
                names.extend(
                    std::env::var("PATHEXT")
                        .unwrap_or_else(|_| ".EXE;.COM;.BAT;.CMD".into())
                        .split(';')
                        .map(|extension| format!("{raw}{extension}")),
                );
            }
            #[cfg(not(windows))]
            let _ = &mut names;
            for directory in std::env::split_paths(&search) {
                for name in &names {
                    let candidate = directory.join(name);
                    if candidate.is_file() {
                        return Ok(std::path::absolute(candidate)?
                            .to_str()
                            .context("Core path is not UTF-8")?
                            .to_owned());
                    }
                }
            }
            bail!("core_bin {raw:?} was not found in PATH");
        }
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.config.config_root.join(path)
        };
        Ok(std::path::absolute(path)?
            .to_str()
            .context("Core path is not UTF-8")?
            .to_owned())
    }
}
fn path_text(path: &Path) -> Result<String> {
    Ok(path.to_str().context("Path is not valid UTF-8")?.to_owned())
}
pub(crate) fn sanitize_windows_path(path: &str) -> String {
    if let Some(path) = path.strip_prefix(r"\\?\UNC\") {
        return format!("//{}", path.replace('\\', "/"));
    }
    path.strip_prefix(r"\\?\")
        .unwrap_or(path)
        .replace('\\', "/")
}
pub(super) fn validate_template(text: &str) -> Result<()> {
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

fn valid_port(value: Option<&serde_json::Value>) -> Option<u16> {
    value?.as_u64()?.try_into().ok().filter(|port| *port != 0)
}

fn proxy_port(mixed_port: Option<u16>, port: Option<u16>) -> Result<u16> {
    mixed_port
        .filter(|v| *v != 0)
        .or(port.filter(|v| *v != 0))
        .context("Target core has no valid mixed-port or port")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_is_single_pass_and_literal_escape_does_not_lookup() {
        let mut queried = Vec::new();
        let text = expand("${path} $${root} ${core_type}", &mut |name| {
            queried.push(name.to_owned());
            Ok(match name {
                "path" => "中文 'quote' $(data) ${root}",
                _ => "mihomo",
            }
            .into())
        })
        .unwrap();
        assert_eq!(text, "中文 'quote' $(data) ${root} ${root} mihomo");
        assert_eq!(queried, ["path", "core_type"]);
        assert!(expand("${unknown}", &mut |_| panic!("must validate first")).is_err());
    }
    #[test]
    fn context_paths_are_lazy_and_do_not_require_application_configuration() {
        let root = std::env::temp_dir();
        let config = super::super::config::parse(&root, r#"{"root":".","actions":[]}"#).unwrap();
        let path = root.join("中文 'quote' ${root}");
        let mut context = ScriptContext {
            config: &config,
            core: CoreType::Singbox,
            path: Some(&path),
            proxy_port: None,
        };
        let expected = if cfg!(windows) {
            sanitize_windows_path(path.to_str().unwrap())
        } else {
            path.to_str().unwrap().to_owned()
        };
        assert_eq!(context.expand("${path}").unwrap(), expected);
        assert_eq!(context.expand("${core_type}").unwrap(), "singbox");
        assert!(context.proxy_port.is_none());
        context.path = None;
        assert!(context.expand("${path}").is_err());
    }
    #[test]
    fn port_fallback_and_windows_paths() {
        assert_eq!(proxy_port(Some(0), Some(7890)).unwrap(), 7890);
        assert_eq!(proxy_port(Some(7891), Some(7890)).unwrap(), 7891);
        assert!(proxy_port(None, Some(0)).is_err());
        for value in [
            serde_json::json!(-1),
            serde_json::json!(65536),
            serde_json::json!("bad"),
        ] {
            assert!(valid_port(Some(&value)).is_none());
        }
        assert_eq!(
            sanitize_windows_path(r"\\?\UNC\server\share\中文 file"),
            "//server/share/中文 file"
        );
        assert_eq!(sanitize_windows_path(r"\\?\C:\space name"), "C:/space name");
        for path in [r"C:relative", r"\rooted", "/rooted"] {
            assert!(validate_windows_path(path).is_err());
        }
        assert!(validate_windows_path(r"C:\absolute").is_ok());
    }
}
