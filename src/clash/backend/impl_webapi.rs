use super::ClashBackend;

impl ClashBackend {
    pub fn update_mode(&self, mode: String) -> anyhow::Result<()> {
        let load = format!(r#"{{"mode": "{mode}"}}"#);
        self.api.config_patch(load)?;
        Ok(())
    }
}
