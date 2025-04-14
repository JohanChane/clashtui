use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub(super) struct ConfigFile {
    pub basic: Basic,
    pub service: Service,
    pub timeout: Option<u64>,
    pub edit_cmd: String,
    pub open_dir_cmd: String,
    #[serde(skip_serializing)]
    pub hack: Hack,
}
impl Default for ConfigFile {
    fn default() -> Self {
        let common_cmd = if cfg!(windows) { "start %s" } else { "open %s" };
        Self {
            basic: Default::default(),
            service: Default::default(),
            timeout: Default::default(),
            edit_cmd: common_cmd.to_owned(),
            open_dir_cmd: common_cmd.to_owned(),
            hack: Default::default(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Basic {
    pub clash_config_dir: String,
    pub clash_bin_path: String,
    pub clash_config_path: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Service {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[serde(alias = "clash_srv_nam")]
    pub clash_service_name: String,
    #[cfg(target_os = "linux")]
    pub is_user: bool,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Hack {
    pub service_controller: ServiceController,
}
impl Default for Hack {
    fn default() -> Self {
        Self {
            service_controller: if cfg!(windows) {
                ServiceController::Nssm
            } else {
                ServiceController::Systemd
            },
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum ServiceController {
    Systemd,
    Nssm,
    OpenRc,
}
impl ServiceController {
    pub fn apply_args<'a>(
        &self,
        work_type: &'a str,
        service_name: &'a str,
        is_user: bool,
    ) -> Vec<&'a str> {
        match self {
            // systemctl --user start service
            ServiceController::Systemd if is_user => vec!["--user", work_type, service_name],
            ServiceController::Systemd => vec![work_type, service_name],

            // rc-service start service --user
            ServiceController::OpenRc if is_user => vec![service_name, work_type, "--user"],
            ServiceController::OpenRc => vec![service_name, work_type],

            ServiceController::Nssm => vec![work_type, service_name],
        }
    }
    pub fn bin_name(&self) -> &'static str {
        match self {
            ServiceController::Systemd => "systemctl",
            ServiceController::Nssm => "nssm",
            ServiceController::OpenRc => "rc-service",
        }
    }
}
