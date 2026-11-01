//! Presentation of captured script output; terminal writes run on the render thread.

use super::popmsg::prelude::Confirm;
use crate::{config::scripts::CommandOutput, functions::process::CapturedOutcome};
use anyhow::{Result, ensure};
use std::{
    io::Write,
    sync::{LazyLock, Mutex, mpsc},
};

struct Output {
    title: String,
    mode: CommandOutput,
    outcome: CapturedOutcome,
}
static OUTPUTS: LazyLock<(mpsc::Sender<Output>, Mutex<mpsc::Receiver<Output>>)> =
    LazyLock::new(|| {
        let (sender, receiver) = mpsc::channel();
        (sender, Mutex::new(receiver))
    });

pub(crate) fn check(mode: CommandOutput, outcome: &CapturedOutcome) -> Result<()> {
    if mode == CommandOutput::Silent {
        ensure!(
            outcome.status.success(),
            "Command exited: {}",
            outcome.status
        );
    } else {
        outcome.ensure_success()?;
    }
    if mode == CommandOutput::Terminal {
        ensure!(
            !outcome.stdout_overflow,
            "Terminal output exceeded its limit; nothing was forwarded"
        );
    }
    Ok(())
}

pub(crate) fn submit(title: String, mode: CommandOutput, outcome: CapturedOutcome) -> Result<()> {
    check(mode, &outcome)?;
    if super::term::is_running() {
        OUTPUTS.0.send(Output {
            title,
            mode,
            outcome,
        })?;
    } else {
        present(&title, mode, &outcome, false)?;
    }
    Ok(())
}

pub(super) fn poll() {
    while let Ok(output) = OUTPUTS.1.lock().unwrap().try_recv() {
        if let Err(error) = present(&output.title, output.mode, &output.outcome, true) {
            Confirm::err(format!("{error:#}"));
        }
    }
}

pub(super) fn present(
    title: &str,
    mode: CommandOutput,
    outcome: &CapturedOutcome,
    tui: bool,
) -> Result<()> {
    check(mode, outcome)?;
    match mode {
        CommandOutput::Silent => (),
        CommandOutput::Popup => {
            let text = outcome.diagnostic();
            if tui {
                Confirm::title(title.to_owned())
                    .with_prompt(if text.is_empty() {
                        format!("Command exited: {}", outcome.status)
                    } else {
                        text
                    })
                    .build_and_send();
            } else {
                std::io::stdout().write_all(text.as_bytes())?;
                std::io::stdout().flush()?;
            }
        }
        CommandOutput::Terminal => {
            // Deliberately generic: scripts own control-sequence construction.
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(&outcome.stdout)?;
            stdout.flush()?;
            let mut diagnostic = crate::functions::process::display_output(&outcome.stderr);
            if outcome.stderr_overflow {
                diagnostic.push_str("\n[stderr truncated]");
            }
            if !diagnostic.is_empty() {
                if tui {
                    Confirm::title(title.to_owned())
                        .with_prompt(diagnostic)
                        .build_and_send();
                } else {
                    eprint!("{diagnostic}");
                }
            }
        }
    }
    Ok(())
}
