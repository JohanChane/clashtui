use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;

pub fn run_fzf(items: &[String], prompt: &str) -> Option<usize> {
    crate::tui::EXT_PROC.store(true, Ordering::SeqCst);
    let _ = crate::tui::hold(true);

    let mut child = match Command::new("fzf")
        .args([
            "--delimiter",
            "\t",
            "--with-nth",
            "2",
            "--prompt",
            &format!("{prompt}> "),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = crate::tui::hold(false);
            crate::tui::widget::popmsg::Confirm::err(anyhow::anyhow!("fzf: {e}"));
            crate::tui::EXT_PROC.store(false, Ordering::SeqCst);
            return None;
        }
    };

    {
        let stdin = child.stdin.as_mut().unwrap();
        for (i, item) in items.iter().enumerate() {
            if writeln!(stdin, "{}\t{}", i, item).is_err() {
                break;
            }
        }
    }

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            let _ = crate::tui::hold(false);
            crate::tui::widget::popmsg::Confirm::err(anyhow::anyhow!("fzf wait: {e}"));
            crate::tui::EXT_PROC.store(false, Ordering::SeqCst);
            return None;
        }
    };

    let _ = crate::tui::hold(false);
    crate::tui::EXT_PROC.store(false, Ordering::SeqCst);

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim();
    if line.is_empty() {
        return None;
    }

    line.split('\t').next()?.parse().ok()
}
