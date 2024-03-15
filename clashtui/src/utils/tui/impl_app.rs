use super::ClashTuiUtil;
use crate::utils::state::_State;
use std::path::Path;
// IPC Related
impl ClashTuiUtil {
    #[cfg(target_os = "windows")]
    pub fn update_state(
        &self,
        new_pf: Option<String>,
        new_mode: Option<String>,
        new_sysp: Option<bool>,
    ) -> _State {
        if let Some(b) = new_sysp {
            let _ = if b {
                super::ipc::enable_system_proxy(&self.clash_api.proxy_addr)
            } else {
                super::ipc::disable_system_proxy()
            };
        }
        let (pf, mode, tun) = self._update_state(new_pf, new_mode);
        let sysp = super::ipc::is_system_proxy_enabled().map_or_else(
            |v| {
                log::error!("{}", v);
                None
            },
            Some,
        );
        _State {
            profile: pf,
            mode,
            tun,
            sysproxy: sysp,
        }
    }

    #[cfg(target_os = "linux")]
    pub fn update_state(&self, new_pf: Option<String>, new_mode: Option<String>) -> _State {
        let (pf, mode, tun) = self._update_state(new_pf, new_mode);
        _State {
            profile: pf,
            mode,
            tun,
        }
    }

    pub fn fetch_recent_logs(&self, num_lines: usize) -> Vec<String> {
        std::fs::read_to_string(self.clashtui_dir.join("clashtui.log"))
            .unwrap_or_default()
            .lines()
            .rev()
            .take(num_lines)
            .map(String::from)
            .collect()
    }
    /// Exec `cmd` for given `path`
    ///
    /// Auto detect `cmd` is_empty and use system default app to open `path`
    fn spawn_open(cmd: &str, path: &Path) -> std::io::Result<()> {
        use crate::utils::ipc::spawn;
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

    pub fn edit_file(&self, path: &Path) -> std::io::Result<()> {
        Self::spawn_open(&self.tui_cfg.edit_cmd, path)
    }
    pub fn open_dir(&self, path: &Path) -> std::io::Result<()> {
        Self::spawn_open(&self.tui_cfg.open_dir_cmd, path)
    }
    fn _update_state(
        &self,
        new_pf: Option<String>,
        new_mode: Option<String>,
    ) -> (String, Option<api::Mode>, Option<api::TunStack>) {
        if let Some(v) = new_mode {
            let load = format!(r#"{{"mode": "{}"}}"#, v);
            let _ = self
                .clash_api
                .config_patch(load)
                .map_err(|e| log::error!("Patch Errr: {}", e));
        }

        let pf = match new_pf {
            Some(v) => {
                self.tui_cfg.update_profile(&v);
                v
            }
            None => self.tui_cfg.current_profile.borrow().clone(),
        };

        if let Err(e) = self.fetch_remote() {
            if e.kind() != std::io::ErrorKind::ConnectionRefused {
                log::warn!("{}", e);
            }
        }
        let (mode, tun) = match self.clash_remote_config.get() {
            Some(v) => (
                Some(v.mode),
                if v.tun.enable {
                    Some(v.tun.stack)
                } else {
                    None
                },
            ),
            None => (None, None),
        };
        (pf, mode, tun)
    }
}
