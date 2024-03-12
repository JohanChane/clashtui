use super::ClashTuiUtil;
use std::{io::Error, path::Path};
// IPC Related
impl ClashTuiUtil {
    /// Exec `cmd` for given `path`
    ///
    /// Auto detect `cmd` is_empty and use system default app to open `path`
    fn spawn_open(cmd: &str, path: &Path) -> Result<(), Error> {
        use super::ipc::spawn;
        if !cmd.is_empty() {
            let opendir_cmd_with_path = cmd.replace("%s", path.to_str().unwrap_or(""));
            #[cfg(target_os = "windows")]
            return spawn("cmd", vec!["/C", opendir_cmd_with_path.as_str()]);
            #[cfg(target_os = "linux")]
            spawn("sh", vec!["-c", opendir_cmd_with_path.as_str()])
        } else {
            #[cfg(target_os = "windows")]
            return spawn("cmd", vec!["/C", "start", path.to_str().unwrap_or("")]);
            #[cfg(target_os = "linux")]
            spawn("xdg-open", vec![path.to_str().unwrap_or("")])
        }
    }

    pub fn edit_file(&self, path: &Path) -> Result<(), Error> {
        Self::spawn_open(&self.tui_cfg.edit_cmd, path)
    }
    pub fn open_dir(&self, path: &Path) -> Result<(), Error> {
        Self::spawn_open(&self.tui_cfg.open_dir_cmd, path)
    }
}
