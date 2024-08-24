use super::*;

impl BackEnd {
    pub fn clash_srv_ctl(&self, op: ServiceOp) -> std::io::Result<String> {
        self.inner.clash_srv_ctl(op)
    }
    pub fn restart_clash(&self) -> Result<String, String> {
        self.inner.api.restart(None).map_err(|e| e.to_string())
    }
    pub fn update_state(
        &self,
        new_pf: Option<String>,
        new_mode: Option<String>,
    ) -> anyhow::Result<State> {
        if let Some(mode) = new_mode {
            self.inner.update_mode(mode)?;
        }
        if let Some(pf) = new_pf.as_ref() {
            if let Some(pf) = self.inner.get_profile(pf) {
                self.select_profile(pf)?;
            } else {
                anyhow::bail!("Not a recorded profile");
            };
        }
        #[cfg(target_os = "windows")]
        let sysp = self.inner.is_system_proxy_enabled().map_or_else(
            |v| {
                log::error!("{}", v);
                None
            },
            Some,
        );
        let ClashConfig { mode, tun, .. } = self.inner.api.config_get()?;
        Ok(State {
            profile: new_pf.unwrap_or(self.get_current_profile().name),
            mode: Some(mode),
            tun: if tun.enable { Some(tun.stack) } else { None },
            #[cfg(target_os = "windows")]
            sysproxy: sysp,
        })
    }
}