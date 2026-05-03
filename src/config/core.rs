use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ConfigFile {
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
    pub fn args<'a>(
        &self,
        work_type: &'a str,
        service_name: &'a str,
        is_user: bool,
    ) -> Vec<&'a str> {
        match self {
            // systemctl --user start service
            ServiceController::Systemd if is_user => vec!["--user", work_type, service_name],
            ServiceController::Systemd => vec![work_type, service_name],

            // rc-service service start --user
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

#[derive(Debug, serde:: Deserialize)]
#[serde(rename_all = "kebab-case")]
/// Get necessary info
pub struct BasicInfo {
    external_controller: String,
    mixed_port: Option<u32>,
    port: Option<u32>,
    socks_port: Option<u32>,
    pub secret: Option<String>,
    pub global_ua: Option<String>,
}

impl BasicInfo {
    const LOCALHOST: &str = "127.0.0.1";
    pub const DEFAULT: &str = "external-controller:127.0.0.1:9090\nmixed-port:7890";

    pub fn get_external_controller(&self) -> String {
        let str = match self.external_controller.strip_prefix("http://") {
            Some(str) => str,
            None => self.external_controller.as_str(),
        };
        if let Some(after) = str.strip_prefix("0.0.0.0") {
            format!("http://{}{}", Self::LOCALHOST, after)
        } else {
            format!("http://{}", str)
        }
    }
    pub fn get_proxy_addr(&self) -> Option<String> {
        match self.mixed_port.or(self.port) {
            Some(p) => Some(format!("http://{}:{p}", Self::LOCALHOST)),
            None => self
                .socks_port
                .map(|p| format!("socks5://{}:{p}", Self::LOCALHOST)),
        }
    }
}
