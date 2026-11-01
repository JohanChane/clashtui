//! Pass selected text to the configured copy action.

use crate::config::scripts::{CommandOutput, ScriptConfig};
use crate::functions::process::CapturedOutcome;
use anyhow::Result;
use std::{
    path::Path,
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

const INPUT_LIMIT: usize = 1024 * 1024;
static BUSY: AtomicBool = AtomicBool::new(false);
type Completion = Result<Option<(CommandOutput, CapturedOutcome)>>;
static RESULTS: LazyLock<(mpsc::Sender<Completion>, Mutex<mpsc::Receiver<Completion>>)> =
    LazyLock::new(|| {
        let (sender, receiver) = mpsc::channel();
        (sender, Mutex::new(receiver))
    });

/// Match ratatui's clamping when a filter shrinks a list under its cursor.
pub(super) fn selected<I: IntoIterator>(items: I, index: Option<usize>) -> Option<I::Item> {
    items.into_iter().take(index?.saturating_add(1)).last()
}

pub(super) fn path_text(path: &Path) -> String {
    let text = path.to_string_lossy();
    if cfg!(windows) {
        crate::functions::command::sanitize_windows_path(&text)
    } else {
        text.into_owned()
    }
}

/// Take a snapshot before starting work: filters and live lists may change later.
pub(super) fn copy(text: Option<String>, source: &'static str, format: &'static str) {
    let Some(text) = text else {
        super::popmsg::prelude::Confirm::err("Nothing selected to copy");
        return;
    };
    if text.len() > INPUT_LIMIT {
        super::popmsg::prelude::Confirm::err("Clipboard text exceeds 1 MiB");
        return;
    }
    if BUSY.swap(true, Ordering::AcqRel) {
        return;
    }
    tokio::task::spawn_blocking(move || {
        let result = (|| {
            let root = crate::config::config_dir_path();
            let scripts = ScriptConfig::load(&root)?;
            let command =
                crate::functions::service::resolve_current(&scripts, &root, "copy", None)?;
            let interactive = command.interactive;
            let execute = || {
                let outcome = crate::functions::process::run(
                    &command,
                    Some(text.into_bytes()),
                    &[
                        ("CLASHTUI_COPY_SOURCE", source),
                        ("CLASHTUI_COPY_FORMAT", format),
                    ],
                    interactive,
                )?;
                if interactive {
                    outcome.ensure_success()?;
                    Ok(None)
                } else {
                    Ok(Some((command.output, outcome)))
                }
            };
            if interactive {
                super::with_terminal(execute)
            } else {
                execute()
            }
        })();
        let _ = RESULTS.0.send(result);
        // BUSY is released by poll, after any terminal output has been delivered.
    });
}

/// Called on the render thread. Terminal output cannot interleave with drawing or
/// a foreground terminal lease; forwarding bytes is not a clipboard acknowledgement.
pub(super) fn poll() -> Option<Result<Option<&'static str>>> {
    let result = RESULTS.1.lock().unwrap().try_recv().ok()?;
    BUSY.store(false, Ordering::Release);
    Some(result.and_then(|completion| {
        let Some((mode, outcome)) = completion else {
            return Ok(None);
        };
        super::script_output::present("Copy", mode, &outcome, true)?;
        Ok(match mode {
            CommandOutput::Terminal if !outcome.stdout.is_empty() => {
                Some("Copy output sent to terminal")
            }
            CommandOutput::Terminal => Some("Copy command completed"),
            CommandOutput::Popup | CommandOutput::Silent => None,
        })
    }))
}
