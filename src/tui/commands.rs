//! One command task, including cleanup and result consumption, at a time.
use crate::{
    command::{self, CommandResult, CommandSpec, ProcessOutput, ProcessOutputMode},
    config::CoreType,
    scripts::{self, ActionRequest, ActionResult, OutputMode, StatusReport},
};
use anyhow::{Context, Result, bail, ensure};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

pub type TaskId = u64;
#[derive(Debug, Clone)]
pub struct ProfileTarget {
    pub core: CoreType,
    pub name: String,
    pub path: PathBuf,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckPurpose {
    Inspect,
    Apply,
}
#[derive(Debug)]
pub struct ScriptTarget {
    pub core: CoreType,
    pub id: String,
    pub path: Option<PathBuf>,
    pub copy: bool,
    pub changes_service: bool,
}
#[derive(Debug)]
pub enum CommandEvent {
    Script {
        task: TaskId,
        target: ScriptTarget,
        result: Result<ActionResult>,
    },
    Status {
        task: TaskId,
        core: CoreType,
        result: Result<StatusReport>,
    },
    Tested {
        task: TaskId,
        target: ProfileTarget,
        result: Result<String>,
    },
    Checked {
        task: TaskId,
        target: ProfileTarget,
        purpose: CheckPurpose,
        result: Result<()>,
    },
    CoreSwitched {
        task: TaskId,
        src: CoreType,
        dst: CoreType,
        result: Result<()>,
    },
    Failed {
        task: TaskId,
        error: String,
    },
}
impl CommandEvent {
    pub fn task(&self) -> TaskId {
        match self {
            Self::Script { task, .. }
            | Self::Status { task, .. }
            | Self::Tested { task, .. }
            | Self::Checked { task, .. }
            | Self::CoreSwitched { task, .. }
            | Self::Failed { task, .. } => *task,
        }
    }
}
enum Job {
    Script(CoreType, ActionRequest),
    Test(ProfileTarget, bool),
    Check(ProfileTarget, CheckPurpose),
    SwitchCore(CoreType, CoreType),
}
impl Job {
    fn is_query(&self) -> bool {
        matches!(self, Self::Script(_, ActionRequest::Status))
    }
    fn is_copy(&self) -> bool {
        matches!(self, Self::Script(_, ActionRequest::Copy(_)))
    }
    fn execute(self, task: TaskId, cancel: &AtomicBool) -> CommandEvent {
        match self {
            Self::SwitchCore(src, dst) => {
                let result = scripts::get().and_then(|config| {
                    switch_core(
                        cancel,
                        |request| {
                            config.execute(src, request, cancel, |spec| run(task, spec, cancel))
                        },
                        || crate::config::CONFIG.data.lock().unwrap().switch_core(dst),
                    )
                });
                CommandEvent::CoreSwitched {
                    task,
                    src,
                    dst,
                    result,
                }
            }
            Self::Script(core, request) => {
                let query = matches!(request, ActionRequest::Status);
                let target = ScriptTarget {
                    core,
                    id: request.id().to_owned(),
                    path: request.path().map(Path::to_owned),
                    copy: matches!(request, ActionRequest::Copy(_)),
                    changes_service: matches!(request, ActionRequest::Run(_)),
                };
                let result = scripts::get().and_then(|config| {
                    config.execute(core, request, cancel, |spec| run(task, spec, cancel))
                });
                if query {
                    CommandEvent::Status {
                        task,
                        core,
                        result: result.and_then(|result| match result {
                            ActionResult::Status(report) => Ok(report),
                            _ => bail!("Invalid status result"),
                        }),
                    }
                } else {
                    CommandEvent::Script {
                        task,
                        target,
                        result,
                    }
                }
            }
            Self::Test(target, geodata) => {
                let result =
                    crate::functions::command::test_config(Some(&target.path), geodata, cancel);
                CommandEvent::Tested {
                    task,
                    target,
                    result,
                }
            }
            Self::Check(target, purpose) => {
                let result =
                    crate::functions::command::check_config(target.core, &target.path, cancel);
                CommandEvent::Checked {
                    task,
                    target,
                    purpose,
                    result,
                }
            }
        }
    }
}
struct Active {
    id: TaskId,
    query: bool,
    copy: bool,
    discard: bool,
    worker: thread::JoinHandle<CommandEvent>,
}
pub struct Commands {
    active: Option<Active>,
    pending_user: Option<Job>,
    awaiting: Option<TaskId>,
    cancel: Arc<AtomicBool>,
    closing: bool,
    next: TaskId,
}
impl Default for Commands {
    fn default() -> Self {
        Self {
            active: None,
            pending_user: None,
            awaiting: None,
            cancel: Arc::new(AtomicBool::new(false)),
            closing: false,
            next: 0,
        }
    }
}
impl Commands {
    pub fn cancellation(&self) -> Arc<AtomicBool> {
        self.cancel.clone()
    }
    pub fn is_busy(&self) -> bool {
        self.active.is_some() || self.awaiting.is_some()
    }
    pub fn submit(&mut self, core: CoreType, request: ActionRequest) -> Result<()> {
        self.enqueue(Job::Script(core, request))
    }
    pub fn switch_core(&mut self, src: CoreType, dst: CoreType) -> Result<()> {
        ensure!(src != dst, "Already using {dst}");
        self.enqueue(Job::SwitchCore(src, dst))
    }
    pub fn test_config(&mut self, target: ProfileTarget, geodata: bool) -> Result<()> {
        ensure!(
            target.core == CoreType::Mihomo,
            "Test is only available for Mihomo"
        );
        self.enqueue(Job::Test(target, geodata))
    }
    pub fn check_config(&mut self, target: ProfileTarget, purpose: CheckPurpose) -> Result<()> {
        self.enqueue(Job::Check(target, purpose))
    }
    fn enqueue(&mut self, job: Job) -> Result<()> {
        ensure!(!self.closing, "Commands are shutting down");
        if let Some(active) = &mut self.active {
            if job.is_copy()
                && (active.copy || self.pending_user.as_ref().is_some_and(Job::is_copy))
            {
                return Ok(());
            }
            if active.query && !job.is_query() && self.pending_user.is_none() {
                active.discard = true;
                self.cancel.store(true, Ordering::Release);
                self.pending_user = Some(job);
                return Ok(());
            }
        }
        ensure!(!self.is_busy(), "Another command is running");
        self.start(job);
        Ok(())
    }
    fn start(&mut self, job: Job) {
        self.next += 1;
        let id = self.next;
        self.cancel
            .store(command::shutdown_requested(), Ordering::Release);
        let cancel = self.cancel.clone();
        self.active = Some(Active {
            id,
            query: job.is_query(),
            copy: job.is_copy(),
            discard: false,
            worker: thread::spawn(move || job.execute(id, &cancel)),
        });
    }
    pub fn poll(&mut self) -> Option<CommandEvent> {
        if !self.active.as_ref()?.worker.is_finished() {
            return None;
        }
        let active = self.active.take().unwrap();
        let event = active
            .worker
            .join()
            .unwrap_or_else(|_| CommandEvent::Failed {
                task: active.id,
                error: "Command worker panicked".into(),
            });
        self.awaiting = Some(active.id);
        if active.discard {
            // Discard only after reaping the worker; the next job cannot observe
            // an old status result after it has started.
            drop(event);
            self.acknowledge(active.id);
            None
        } else {
            Some(event)
        }
    }
    pub fn acknowledge(&mut self, task: TaskId) {
        if self.awaiting != Some(task) {
            return;
        }
        self.awaiting = None;
        if !self.closing
            && let Some(job) = self.pending_user.take()
        {
            self.start(job);
        }
    }
    pub fn cancel(&mut self) {
        self.pending_user = None;
        self.cancel.store(true, Ordering::Release);
    }
    pub fn begin_shutdown(&mut self) {
        self.closing = true;
        self.cancel();
    }
    pub fn is_drained(&self) -> bool {
        !self.is_busy() && self.pending_user.is_none()
    }
}
impl Drop for Commands {
    fn drop(&mut self) {
        self.begin_shutdown();
        // Terminal acquire and Enter waits observe cancel without needing App.
        if let Some(active) = self.active.take() {
            let _ = active.worker.join();
        }
    }
}
fn switch_core(
    cancel: &AtomicBool,
    mut execute: impl FnMut(ActionRequest) -> Result<ActionResult>,
    persist: impl FnOnce() -> Result<()>,
) -> Result<()> {
    // Always stop: status protocols may report transitional states as inactive.
    let stopped =
        execute(ActionRequest::Run("stop_core".into())).context("Failed to stop source core")?;
    let ActionResult::Completed { display, process } = stopped else {
        bail!("Invalid stop_core result");
    };
    let output = process.context("Failed to stop source core")?;
    ensure!(
        output.status.success(),
        "stop_core failed: {}{}",
        output.status,
        if display == OutputMode::Silent {
            String::new()
        } else {
            format!("\n{}", output.captured.diagnostic())
        }
    );
    let report = execute(ActionRequest::Status).context("Failed to verify source core stopped")?;
    let ActionResult::Status(report) = report else {
        bail!("Invalid status_core result");
    };
    ensure!(
        matches!(
            report.state,
            scripts::ServiceState::Inactive | scripts::ServiceState::Missing
        ),
        "Source core is not confirmed stopped: {}",
        report.state.as_str()
    );
    if cancel.load(Ordering::Acquire) {
        return Err(command::CommandError::cancelled().into());
    }
    persist().context("Failed to save destination core")
}

fn run(task: TaskId, spec: CommandSpec, cancel: &AtomicBool) -> CommandResult<ProcessOutput> {
    if matches!(spec.output, ProcessOutputMode::Inherit) {
        super::term::with_terminal(task, cancel, || {
            let result = command::run(spec, cancel);
            if let Ok(output) = &result {
                println!("Command exited: {}", output.status);
            }
            result
        })
    } else {
        command::run(spec, cancel)
    }
}

/// Called only by App on the render thread, before it acknowledges the event.
pub fn present(
    target: &ScriptTarget,
    result: &Result<ActionResult>,
) -> Result<Option<&'static str>> {
    let result = match result {
        Ok(result) => result,
        Err(error) => bail!("{}", error_text(error)),
    };
    let ActionResult::Completed { display, process } = result else {
        return Ok(None);
    };
    let output = match process {
        Ok(output) => output,
        Err(error) => bail!(
            "{error}{}",
            if *display == OutputMode::Silent {
                String::new()
            } else {
                format!("\n{}", error.captured.diagnostic())
            }
        ),
    };
    ensure!(
        output.status.success(),
        "Command exited: {}{}",
        output.status,
        if *display == OutputMode::Silent {
            String::new()
        } else {
            format!("\n{}", output.captured.diagnostic())
        }
    );
    let title = match &target.path {
        Some(path) => format!("{}: {} ({})", target.core.id(), target.id, path.display()),
        None => format!("{}: {}", target.core.id(), target.id),
    };
    let diagnostic = match display {
        OutputMode::Silent => return Ok(None),
        OutputMode::Popup => {
            let text = output.captured.diagnostic();
            if text.is_empty() {
                format!("Command exited: {}", output.status)
            } else {
                text
            }
        }
        OutputMode::Terminal => {
            ensure!(
                !output.captured.stdout_truncated,
                "Terminal output exceeded its limit; nothing was forwarded"
            );
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(&output.captured.stdout)?;
            stdout.flush()?;
            let mut text = command::display_output(&output.captured.stderr);
            if output.captured.stderr_truncated {
                text.push_str("\n[stderr truncated]");
            }
            text
        }
    };
    if !diagnostic.is_empty() {
        super::popmsg::prelude::Confirm::title(title)
            .with_prompt(diagnostic)
            .build_and_send();
    }
    Ok((target.copy && *display == OutputMode::Terminal).then_some(
        if output.captured.stdout.is_empty() {
            "Copy command completed"
        } else {
            "Copy output sent to terminal"
        },
    ))
}
pub fn error_text(error: &anyhow::Error) -> String {
    let mut text = format!("{error:#}");
    if let Some(error) = error.downcast_ref::<command::CommandError>() {
        let diagnostic = error.captured.diagnostic();
        if !diagnostic.is_empty() {
            text.push('\n');
            text.push_str(&diagnostic);
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };
    fn stop_result(success: bool) -> ActionResult {
        #[cfg(unix)]
        use std::os::unix::process::ExitStatusExt;
        #[cfg(windows)]
        use std::os::windows::process::ExitStatusExt;
        ActionResult::Completed {
            display: OutputMode::Popup,
            process: Ok(ProcessOutput {
                status: std::process::ExitStatus::from_raw(if success { 0 } else { 256 }),
                captured: command::CapturedOutput::default(),
            }),
        }
    }
    fn status_result(state: scripts::ServiceState) -> ActionResult {
        ActionResult::Status(StatusReport {
            state,
            diagnostic: None,
        })
    }
    #[test]
    fn switch_core_stops_then_verifies_then_persists() {
        for state in [
            scripts::ServiceState::Inactive,
            scripts::ServiceState::Missing,
        ] {
            let calls = std::cell::RefCell::new(Vec::new());
            switch_core(
                &AtomicBool::new(false),
                |request| {
                    calls.borrow_mut().push(request.id().to_owned());
                    Ok(match request {
                        ActionRequest::Run(id) if id == "stop_core" => stop_result(true),
                        ActionRequest::Status => status_result(state),
                        _ => panic!("unexpected request"),
                    })
                },
                || {
                    calls.borrow_mut().push("persist".into());
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(*calls.borrow(), ["stop_core", "status_core", "persist"]);
        }
    }
    #[test]
    fn switch_core_failures_and_cancellation_never_persist() {
        let scenarios = [
            vec![Err(anyhow::anyhow!("stop unavailable"))],
            vec![Ok(stop_result(false))],
            vec![Ok(ActionResult::Completed {
                display: OutputMode::Popup,
                process: Err(command::CommandError::cancelled()),
            })],
            vec![Ok(stop_result(true)), Err(anyhow::anyhow!("status failed"))],
            vec![
                Ok(stop_result(true)),
                Ok(status_result(scripts::ServiceState::Active)),
            ],
            vec![
                Ok(stop_result(true)),
                Ok(status_result(scripts::ServiceState::Unsupported)),
            ],
        ];
        for results in scenarios {
            let mut results = results.into_iter();
            assert!(
                switch_core(
                    &AtomicBool::new(false),
                    |_| results.next().unwrap(),
                    || panic!("must not persist")
                )
                .is_err()
            );
            assert!(results.next().is_none());
        }
        let cancel = AtomicBool::new(false);
        assert!(
            switch_core(
                &cancel,
                |request| {
                    Ok(if matches!(request, ActionRequest::Status) {
                        cancel.store(true, Ordering::Release);
                        status_result(scripts::ServiceState::Inactive)
                    } else {
                        stop_result(true)
                    })
                },
                || panic!("must not persist after cancellation")
            )
            .is_err()
        );
    }
    #[test]
    fn switch_core_reports_persistence_failure() {
        let error = switch_core(
            &AtomicBool::new(false),
            |request| {
                Ok(if matches!(request, ActionRequest::Status) {
                    status_result(scripts::ServiceState::Inactive)
                } else {
                    stop_result(true)
                })
            },
            || bail!("disk full"),
        )
        .unwrap_err();
        assert_eq!(
            format!("{error:#}"),
            "Failed to save destination core: disk full"
        );
    }
    fn fake_query(commands: &mut Commands) -> mpsc::Sender<()> {
        let (send, receive) = mpsc::channel();
        commands.next = 1;
        commands.active = Some(Active {
            id: 1,
            query: true,
            copy: false,
            discard: false,
            worker: thread::spawn(move || {
                receive.recv().unwrap();
                CommandEvent::Status {
                    task: 1,
                    core: CoreType::Mihomo,
                    result: Ok(StatusReport {
                        state: scripts::ServiceState::Active,
                        diagnostic: None,
                    }),
                }
            }),
        });
        send
    }
    fn wait_finished(commands: &Commands) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !commands.active.as_ref().unwrap().worker.is_finished() {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(1));
        }
    }
    #[test]
    fn completion_holds_slot_until_app_acknowledges() {
        let mut commands = Commands::default();
        fake_query(&mut commands).send(()).unwrap();
        wait_finished(&commands);
        let event = commands.poll().unwrap();
        assert!(commands.is_busy());
        assert!(
            commands
                .submit(CoreType::Mihomo, ActionRequest::Run("custom".into()))
                .is_err()
        );
        commands.acknowledge(event.task() + 1);
        assert!(commands.is_busy());
        commands.acknowledge(event.task());
        assert!(!commands.is_busy());
    }
    #[test]
    fn user_request_waits_for_preempted_query_to_be_reaped_and_discarded() {
        let mut commands = Commands::default();
        let release = fake_query(&mut commands);
        commands
            .submit(CoreType::Mihomo, ActionRequest::Run("custom".into()))
            .unwrap();
        assert!(commands.cancel.load(Ordering::Acquire));
        assert_eq!(commands.active.as_ref().unwrap().id, 1);
        assert!(commands.poll().is_none());
        release.send(()).unwrap();
        wait_finished(&commands);
        assert!(
            commands.poll().is_none(),
            "preempted report must not reach App"
        );
        assert_eq!(commands.active.as_ref().unwrap().id, 2);
        commands.begin_shutdown();
        wait_finished(&commands);
        if let Some(event) = commands.poll() {
            commands.acknowledge(event.task());
        }
        assert!(commands.is_drained());
    }
    #[test]
    fn shutdown_drops_pending_work_before_reaping_query() {
        let mut commands = Commands::default();
        let release = fake_query(&mut commands);
        commands
            .submit(CoreType::Mihomo, ActionRequest::Copy("text".into()))
            .unwrap();
        commands.begin_shutdown();
        release.send(()).unwrap();
        wait_finished(&commands);
        assert!(commands.poll().is_none());
        assert!(commands.is_drained());
        assert!(
            commands
                .submit(CoreType::Mihomo, ActionRequest::Status)
                .is_err()
        );
    }
    #[test]
    fn failed_terminal_output_never_reaches_terminal_and_silent_hides_it() {
        #[cfg(unix)]
        use std::os::unix::process::ExitStatusExt;
        #[cfg(windows)]
        use std::os::windows::process::ExitStatusExt;
        let target = ScriptTarget {
            core: CoreType::Mihomo,
            id: "copy".into(),
            path: None,
            copy: true,
            changes_service: false,
        };
        for display in [OutputMode::Popup, OutputMode::Silent, OutputMode::Terminal] {
            let result = Ok(ActionResult::Completed {
                display,
                process: Ok(ProcessOutput {
                    status: std::process::ExitStatus::from_raw(7 << 8),
                    captured: command::CapturedOutput {
                        stdout: b"\x1b]52;c;private\x07".to_vec(),
                        stderr: b"diagnostic".to_vec(),
                        ..Default::default()
                    },
                }),
            });
            let text = present(&target, &result).unwrap_err().to_string();
            assert!(!text.contains('\x1b'));
            assert_eq!(text.contains("diagnostic"), display != OutputMode::Silent);
            assert_eq!(text.contains("private"), display != OutputMode::Silent);
        }
    }
}
