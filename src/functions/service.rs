//! Preserve the synchronous CLI restart/stop contract without TUI initialization.
use crate::{
    command::{self, ProcessOutputMode},
    config::{self, CONFIG},
    scripts::{self, ActionRequest, ActionResult, OutputMode},
};
use anyhow::{Result, ensure};
use std::{
    io::Write,
    sync::{Arc, atomic::AtomicBool},
};

pub fn execute_current(id: &str) -> Result<String> {
    let core = CONFIG.core_type();
    let config = scripts::config::load(&config::config_dir_path())?;
    let cancel = Arc::new(AtomicBool::new(false));
    let result = config.execute(core, ActionRequest::Run(id.into()), &cancel, |spec| {
        let _signals = command::signal_scope(cancel.clone()).map_err(command::CommandError::io)?;
        let interactive = matches!(spec.output, ProcessOutputMode::Inherit);
        if interactive {
            println!("{}: {id}", core.id());
        }
        let result = command::run(spec, &cancel);
        if interactive && let Ok(output) = &result {
            println!("Command exited: {}", output.status);
        }
        result
    })?;
    if let ActionResult::Completed { display, process } = result {
        let output = match process {
            Ok(output) => output,
            Err(error) => {
                let diagnostic = if display == OutputMode::Silent {
                    String::new()
                } else {
                    error.captured.diagnostic()
                };
                let message = format!("{error}\n{diagnostic}");
                return Err(if diagnostic.is_empty() {
                    error.into()
                } else {
                    anyhow::Error::new(error).context(message)
                });
            }
        };
        ensure!(
            output.status.success(),
            "Command exited: {}{}",
            output.status,
            if display == OutputMode::Silent {
                String::new()
            } else {
                format!("\n{}", output.captured.diagnostic())
            }
        );
        match display {
            OutputMode::Silent => (),
            OutputMode::Popup => {
                print!("{}", output.captured.diagnostic());
                std::io::stdout().flush()?;
            }
            OutputMode::Terminal => {
                std::io::stdout().write_all(&output.captured.stdout)?;
                std::io::stdout().flush()?;
                eprint!("{}", command::display_output(&output.captured.stderr));
                if output.captured.stderr_truncated {
                    eprintln!("\n[output truncated]");
                }
            }
        }
    }
    Ok(format!("{} {id}: OK", core.id()))
}
