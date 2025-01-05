use super::ClashTuiUtil;
use crate::utils::state::_State;
use std::path::Path;
// IPC Related
impl ClashTuiUtil {
    pub fn update_state(&self, new_pf: Option<String>, new_mode: Option<String>, no_pp: Option<bool>) -> _State {
        let (pf, mode, tun, no_pp_value) = self._update_state(new_pf, new_mode, no_pp);
        _State {
            profile: pf,
            mode,
            tun,
            no_pp: no_pp_value,
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
            let open_cmd = cmd.replace("%s", path.to_str().unwrap_or(""));
            return spawn("sh", vec!["-c", open_cmd.as_str()]);
        } else {
            return spawn("xdg-open", vec![path.to_str().unwrap_or("")]);
        }
    }

    pub fn edit_file(&self, path: &Path) -> std::io::Result<()> {
        Self::spawn_open(self.tui_cfg.edit_cmd.as_str(), path)
    }
    pub fn open_dir(&self, path: &Path) -> std::io::Result<()> {
        Self::spawn_open(self.tui_cfg.open_dir_cmd.as_str(), path)
    }
    fn _update_state(
        &self,
        new_pf: Option<String>,
        new_mode: Option<String>,
        no_pp: Option<bool>,
    ) -> (String, Option<api::Mode>, Option<api::TunStack>, bool) {
        if let Some(v) = new_mode {
            let load = format!(r#"{{"mode": "{}"}}"#, v);
            let _ = self
                .clash_api
                .config_patch(load)
                .map_err(|e| log::error!("Patch Errr: {}", e));
        }

        let pf = match new_pf {
            Some(v) => {
                self.clashtui_data.borrow_mut().update_profile(&v);
                v
            }
            None => self.clashtui_data.borrow().current_profile.clone(),
        };
        let clash_cfg = self
            .fetch_remote()
            .map_err(|e| log::warn!("Fetch Remote:{e}"))
            .ok();
        let (mode, tun) = match clash_cfg {
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

        if let Some(v) = no_pp {
            self.clashtui_data.borrow_mut().no_pp = v;
        }
        let no_pp_value = self.clashtui_data.borrow().no_pp;

        (pf, mode, tun, no_pp_value)
    }
}
