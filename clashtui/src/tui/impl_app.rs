use backend::ClashBackend;
use std::path::Path;
/// A little trick, work for now
pub trait MonkeyPatch {
    fn fetch_recent_logs(&self, num_lines: usize) -> Vec<String>;
    fn edit_file(&self, path: &Path) -> std::io::Result<()>;
    fn open_dir(&self, path: &Path) -> std::io::Result<()>;
}
// IPC Related
impl MonkeyPatch for ClashBackend {
    fn fetch_recent_logs(&self, num_lines: usize) -> Vec<String> {
        std::fs::read_to_string(self.home_dir.join("clashtui.log"))
            .unwrap_or_default()
            .lines()
            .rev()
            .take(num_lines)
            .map(String::from)
            .collect()
    }

    fn edit_file(&self, path: &Path) -> std::io::Result<()> {
        spawn_open(&self.cfg.edit_cmd, path)
    }
    fn open_dir(&self, path: &Path) -> std::io::Result<()> {
        spawn_open(&self.cfg.open_dir_cmd, path)
    }
}
/// Exec `cmd` for given `path`
///
/// Auto detect `cmd` is_empty and use system default app to open `path`
fn spawn_open(cmd: &str, path: &Path) -> std::io::Result<()> {
    use backend::utils::spawn;
    if !cmd.is_empty() {
        let opendir_cmd_with_path = cmd.replace("%s", path.to_str().unwrap_or(""));
        #[cfg(target_os = "windows")]
        return spawn("cmd", vec!["/C", opendir_cmd_with_path.as_str()]);
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        spawn("sh", vec!["-c", opendir_cmd_with_path.as_str()])
    } else {
        #[cfg(target_os = "windows")]
        return spawn("cmd", vec!["/C", "start", path.to_str().unwrap_or("")]);
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        spawn("xdg-open", vec![path.to_str().unwrap_or("")])
    }
}
