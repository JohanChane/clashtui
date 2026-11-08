//! Built-in action inputs and status protocol; execution is supplied by the caller.
use super::{
    config::{ActionDefinition, OutputMode, ScriptConfig},
    variables::{ScriptContext, validate_host_path},
};
use crate::{
    command::{CommandError, CommandResult, CommandSpec, Input, ProcessOutput, ProcessOutputMode},
    config::CoreType,
};
use anyhow::{Context, Result, bail, ensure};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

#[derive(Debug)]
pub enum ActionRequest {
    Run(String),
    Copy(String),
    EditFile(PathBuf),
    OpenDir(PathBuf),
    Status,
}
impl ActionRequest {
    pub fn id(&self) -> &str {
        match self {
            Self::Run(id) => id,
            Self::Copy(_) => "copy",
            Self::EditFile(_) => "edit_file",
            Self::OpenDir(_) => "open_dir",
            Self::Status => "status_core",
        }
    }
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::EditFile(path) | Self::OpenDir(path) => Some(path),
            _ => None,
        }
    }
}
#[derive(Debug)]
pub enum ActionResult {
    Completed {
        display: OutputMode,
        process: CommandResult<ProcessOutput>,
    },
    Status(StatusReport),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceState {
    Active,
    Inactive,
    Missing,
    Unsupported,
}
impl ServiceState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Missing => "missing",
            Self::Unsupported => "unsupported",
        }
    }
}
#[derive(Debug, Clone)]
pub struct StatusReport {
    pub state: ServiceState,
    pub diagnostic: Option<String>,
}

impl ScriptConfig {
    fn find(&self, core: CoreType, id: &str) -> Option<&ActionDefinition> {
        self.actions
            .iter()
            .find(|action| action.applies_to(core) && action.id == id)
    }
    pub fn menu(&self, core: CoreType) -> impl Iterator<Item = &ActionDefinition> {
        self.actions.iter().filter(move |action| {
            action.applies_to(core) && action.label.is_some() && !context_action(&action.id)
        })
    }
    pub fn execute(
        &self,
        core: CoreType,
        request: ActionRequest,
        cancel: &AtomicBool,
        run: impl FnOnce(CommandSpec) -> CommandResult<ProcessOutput>,
    ) -> Result<ActionResult> {
        if cancel.load(Ordering::Acquire) {
            return Err(CommandError::cancelled().into());
        }
        if let ActionRequest::Run(id) = &request {
            ensure!(!context_action(id), "{id} requires action input");
        }
        let status = matches!(request, ActionRequest::Status);
        let Some(action) = self.find(core, request.id()) else {
            if status {
                return Ok(ActionResult::Status(StatusReport {
                    state: ServiceState::Unsupported,
                    diagnostic: None,
                }));
            }
            bail!(
                "{} does not support {} in scripts.json",
                core.id(),
                request.id()
            );
        };
        if let ActionRequest::Copy(text) = &request {
            ensure!(text.len() <= 1024 * 1024, "Copy text exceeds 1 MiB");
        }
        let mut context = ScriptContext {
            config: self,
            core,
            path: request.path(),
            proxy_port: None,
        };
        let program = context.expand(&action.cmd.program)?;
        ensure!(!program.trim().is_empty(), "Empty command program");
        validate_host_path(&program)?;
        let path = Path::new(&program);
        let program = if !path.is_absolute()
            && (program.contains('/') || (cfg!(windows) && program.contains('\\')))
        {
            self.root.join(path).into_os_string()
        } else {
            program.into()
        };
        let args = action
            .cmd
            .args
            .iter()
            .map(|arg| context.expand(arg).map(Into::into))
            .collect::<Result<_>>()?;
        let interactive = action.cmd.interactive && !status;
        let stdin = match request {
            ActionRequest::Copy(text) => Input::Bytes(text.into_bytes()),
            _ if interactive => Input::Inherit,
            _ => Input::Closed,
        };
        let output = if interactive {
            ProcessOutputMode::Inherit
        } else {
            ProcessOutputMode::Capture {
                stdout_limit: if !status && action.cmd.output == OutputMode::Terminal {
                    2 * 1024 * 1024
                } else {
                    64 * 1024
                },
                stderr_limit: 64 * 1024,
                fail_on_stdout_overflow: status || action.cmd.output == OutputMode::Terminal,
            }
        };
        if cancel.load(Ordering::Acquire) {
            return Err(CommandError::cancelled().into());
        }
        let process = run(CommandSpec {
            program,
            args,
            cwd: Some(self.root.clone()),
            stdin,
            output,
            timeout: if status {
                Some(Duration::from_secs(5))
            } else {
                action.cmd.timeout_secs.map(Duration::from_secs)
            },
            keep_children_on_success: !status && action.cmd.keep_children_on_success,
        });
        if status {
            let output = process?;
            ensure!(
                output.status.success(),
                "status_core failed ({}): {}",
                output.status,
                output.captured.diagnostic()
            );
            let state = parse_status(&output.captured.stdout)
                .with_context(|| output.captured.diagnostic())?;
            let mut diagnostic = crate::command::display_output(&output.captured.stderr)
                .trim()
                .to_owned();
            if output.captured.stderr_truncated {
                diagnostic.push_str("\n[output truncated]");
            }
            Ok(ActionResult::Status(StatusReport {
                state,
                diagnostic: (!diagnostic.is_empty()).then_some(diagnostic),
            }))
        } else {
            Ok(ActionResult::Completed {
                display: if interactive {
                    OutputMode::Silent
                } else {
                    action.cmd.output
                },
                process,
            })
        }
    }
}
fn context_action(id: &str) -> bool {
    matches!(id, "copy" | "edit_file" | "open_dir")
}
fn parse_status(output: &[u8]) -> Result<ServiceState> {
    let output =
        std::str::from_utf8(output).context("status_core stdout must be UTF-8 without BOM")?;
    match output
        .strip_suffix("\r\n")
        .or_else(|| output.strip_suffix('\n'))
        .unwrap_or(output)
    {
        "active" => Ok(ServiceState::Active),
        "inactive" => Ok(ServiceState::Inactive),
        "missing" => Ok(ServiceState::Missing),
        _ => bail!("status_core must print exactly active, inactive or missing on one line"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{CapturedOutput, CommandErrorKind};
    fn output(text: &[u8]) -> ProcessOutput {
        #[cfg(unix)]
        use std::os::unix::process::ExitStatusExt;
        #[cfg(windows)]
        use std::os::windows::process::ExitStatusExt;
        ProcessOutput {
            status: std::process::ExitStatus::from_raw(0),
            captured: CapturedOutput {
                stdout: text.into(),
                ..Default::default()
            },
        }
    }
    #[test]
    fn copy_keeps_bytes_for_both_interactive_modes_and_retains_error_display() {
        for interactive in [false, true] {
            let config = super::super::config::parse(&std::env::temp_dir(), &serde_json::json!({"root":".","actions":[{"id":"copy","cmd":{"program":"tool","args":[],"interactive":interactive,"output":"terminal"}}]}).to_string()).unwrap();
            let result = config.execute(CoreType::Mihomo, ActionRequest::Copy("中文 ${path}".into()), &AtomicBool::new(false), |spec| {
                assert!(matches!(spec.stdin, Input::Bytes(ref bytes) if bytes == "中文 ${path}".as_bytes()));
                assert_eq!(matches!(spec.output, ProcessOutputMode::Inherit), interactive);
                Err(CommandError::cancelled())
            }).unwrap();
            let ActionResult::Completed { display, process } = result else {
                panic!()
            };
            assert_eq!(
                display,
                if interactive {
                    OutputMode::Silent
                } else {
                    OutputMode::Terminal
                }
            );
            assert_eq!(process.unwrap_err().kind, CommandErrorKind::Cancelled);
            assert!(
                config
                    .execute(
                        CoreType::Mihomo,
                        ActionRequest::Run("copy".into()),
                        &AtomicBool::new(false),
                        |_| panic!()
                    )
                    .is_err()
            );
            assert!(
                config
                    .execute(
                        CoreType::Mihomo,
                        ActionRequest::Copy("x".repeat(1024 * 1024 + 1)),
                        &AtomicBool::new(false),
                        |_| panic!()
                    )
                    .is_err()
            );
        }
    }
    #[test]
    fn internal_status_overrides_io_and_keeps_successful_stderr() {
        let config = super::super::config::parse(&std::env::temp_dir(), r#"{"root":".","actions":[{"id":"status_core","cmd":{"program":"tool","args":[],"interactive":true,"output":"terminal","keep_children_on_success":true,"timeout_secs":50}}]}"#).unwrap();
        let result = config
            .execute(
                CoreType::Mihomo,
                ActionRequest::Status,
                &AtomicBool::new(false),
                |spec| {
                    assert!(matches!(spec.stdin, Input::Closed));
                    assert!(matches!(
                        spec.output,
                        ProcessOutputMode::Capture {
                            stdout_limit: 65536,
                            stderr_limit: 65536,
                            fail_on_stdout_overflow: true
                        }
                    ));
                    assert!(!spec.keep_children_on_success);
                    assert_eq!(spec.timeout, Some(Duration::from_secs(5)));
                    let mut output = output(b"active\r\n");
                    output.captured.stderr = b"warning".to_vec();
                    output.captured.stderr_truncated = true;
                    Ok(output)
                },
            )
            .unwrap();
        let ActionResult::Status(report) = result else {
            panic!()
        };
        assert_eq!(report.state, ServiceState::Active);
        assert_eq!(
            report.diagnostic.as_deref(),
            Some("warning\n[output truncated]")
        );
        for invalid in [
            "active\n\n",
            " active",
            "ACTIVE",
            "active extra",
            "\u{feff}active",
        ] {
            assert!(parse_status(invalid.as_bytes()).is_err());
        }
    }
    #[test]
    fn context_actions_are_hidden_and_target_path_is_literal() {
        let config = super::super::config::parse(&std::env::temp_dir(), r#"{"root":".","actions":[{"id":"edit_file","label":"Edit","cmd":{"program":"tool","args":["${path}"]}},{"id":"custom","label":"Visible","cmd":{"program":"tool","args":[]}},{"id":"hidden","cmd":{"program":"tool","args":[]}}]}"#).unwrap();
        assert_eq!(
            config
                .menu(CoreType::Mihomo)
                .map(|a| a.id.as_str())
                .collect::<Vec<_>>(),
            ["custom"]
        );
        let path = std::env::temp_dir().join("中文 ${root}");
        config
            .execute(
                CoreType::Mihomo,
                ActionRequest::EditFile(path.clone()),
                &AtomicBool::new(false),
                |spec| {
                    let expected = if cfg!(windows) {
                        super::super::variables::sanitize_windows_path(path.to_str().unwrap())
                            .into()
                    } else {
                        path.into_os_string()
                    };
                    assert_eq!(spec.args, [expected]);
                    Ok(output(b""))
                },
            )
            .unwrap();
        let result = config
            .execute(
                CoreType::Mihomo,
                ActionRequest::Status,
                &AtomicBool::new(false),
                |_| panic!(),
            )
            .unwrap();
        assert!(matches!(
            result,
            ActionResult::Status(StatusReport {
                state: ServiceState::Unsupported,
                ..
            })
        ));
    }
}
