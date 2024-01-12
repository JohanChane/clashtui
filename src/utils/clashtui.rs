use log;
use std::cell::RefCell;
use std::collections::HashMap;
use std::process::Command;
use std::{
    fs::{create_dir_all, File},
    io::{BufRead, BufReader, Error, Read},
    path::{Path, PathBuf},
};

#[cfg(target_os = "windows")]
use encoding::all::GBK;

use super::clash::ClashUtil;
use super::clash_state::_State;
use super::configs::{ClashConfig, ClashTuiConfig, ClashTuiConfigLoadError};
use super::utils as Utils;
use super::{ClashTuiOp, ConfigOp};

pub struct ClashTuiUtil {
    pub clashtui_dir: PathBuf,
    pub profile_dir: PathBuf,

    clash_api: ClashUtil,
    clashtui_config: RefCell<ClashTuiConfig>,
    clash_remote_config: RefCell<Option<ClashConfig>>,

    err_track: Vec<ClashTuiConfigLoadError>,
}

impl ClashTuiUtil {
    pub fn new(clashtui_dir: &PathBuf, profile_dir: &PathBuf, is_init: bool) -> Self {
        let ret = load_app_config(clashtui_dir, is_init);
        let mut err_track = ret.3;
        let clash_api = ClashUtil::new(ret.1, ret.2);
        let cur_remote = match clash_api.config_get() {
            Ok(v) => v,
            Err(_) => String::new(),
        };
        let remote = ClashConfig::from_str(cur_remote.as_str());
        if remote.is_none() {
            err_track.push(ClashTuiConfigLoadError::LoadClashConfig(
                "Fail to load config from clash core. Is it Running?\n".into(),
            ));
            log::warn!("Fail to connect to clash. Is it Running?");
        }
        Self {
            clashtui_dir: clashtui_dir.clone(),
            profile_dir: profile_dir.clone(),
            clash_api,
            clashtui_config: RefCell::new(ret.0),
            clash_remote_config: RefCell::new(remote),
            err_track,
        }
    }

    pub fn get_err_track(&self) -> Vec<ClashTuiConfigLoadError> {
        return self.err_track.clone();
    }

    pub fn save_config(&self) {
        if let Err(x) = self
            .clashtui_config
            .borrow()
            .to_file(self.clashtui_dir.join("config.yaml").to_str().unwrap())
        {
            log::error!("Error while saving config: {}", x);
        };
    }

    pub fn update_config(&self, conf: &ConfigOp, data: String) {
        let mut config = self.clashtui_config.borrow_mut();
        log::debug!("Updated Config: {:?}:{}", conf, data);
        match conf {
            ConfigOp::ClashConfigDir => config.clash_cfg_dir = data,
            ConfigOp::ClashCorePath => config.clash_core_path = data,
            ConfigOp::ClashConfigFile => config.clash_cfg_path = data,
            ConfigOp::ClashServiceName => config.clash_srv_name = data,
        };
        drop(config);
        self.save_config();
    }

    fn fetch_remote(&self) -> Option<reqwest::Error> {
        let cur_remote = match self.clash_api.config_get() {
            Ok(v) => v,
            Err(e) => return Some(e),
        };
        let remote = ClashConfig::from_str(cur_remote.as_str());
        *self.clash_remote_config.borrow_mut() = remote;
        None
    }

    pub fn restart_clash(&self) -> Result<String, reqwest::Error> {
        self.clash_api.restart(None)
    }

    pub fn patch_config(&self) -> Option<reqwest::Error> {
        let body = serde_json::json!({
            "path": self.clashtui_config.borrow().clash_cfg_path.as_str(),
            "payload": ""
        })
        .to_string();

        self.clash_api.config_reload(body)
    }

    pub fn select_profile(&self, profile_name: &String) -> Result<(), Error> {
        if let Err(err) = self.merge_profile(profile_name) {
            log::error!(
                "Failed to Merge Profile `{}`: {}",
                profile_name,
                err.to_string()
            );
            return Err(Error::new(std::io::ErrorKind::Other, err));
        } else {
        };
        if let Some(err) = self.patch_config() {
            log::error!(
                "Failed to Patch Profile `{}`: {}",
                profile_name,
                err.to_string()
            );
            return Err(Error::new(std::io::ErrorKind::Other, err));
        } else {
        };
        Ok(())
    }

    pub fn merge_profile(&self, profile_name: &String) -> anyhow::Result<()> {
        let basic_clash_cfg_path = self.clashtui_dir.join("basic_clash_config.yaml");
        let mut dst_parsed_yaml = parse_yaml(&basic_clash_cfg_path)?;
        let profile_yaml_path = self.get_profile_yaml_path(profile_name);
        let profile_parsed_yaml = parse_yaml(&profile_yaml_path).or_else(|e| {
            anyhow::bail!(
                "Maybe need to update first. Failed to parse {}: {}",
                profile_yaml_path.to_str().unwrap(),
                e.to_string()
            );
        })?;

        if let serde_yaml::Value::Mapping(dst_mapping) = &mut dst_parsed_yaml {
            if let serde_yaml::Value::Mapping(mapping) = &profile_parsed_yaml {
                for (key, value) in mapping.iter() {
                    if let serde_yaml::Value::String(k) = key {
                        match k.as_str() {
                            "proxy-groups" | "proxy-providers" | "proxies" | "sub-rules"
                            | "rules" | "rule-providers" => {
                                dst_mapping.insert(key.clone(), value.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let final_clash_cfg_file = File::create(&self.clashtui_config.borrow().clash_cfg_path)?;
        serde_yaml::to_writer(final_clash_cfg_file, &dst_parsed_yaml)?;

        Ok(())
    }

    pub fn update_local_profile(
        &self,
        profile_name: &String,
        does_update_all: bool,
    ) -> anyhow::Result<(Vec<(String, String)>, Vec<(String, String)>)> {
        let net_res_keys = if !does_update_all {
            vec!["proxy-providers"]
        } else {
            vec!["proxy-providers", "rule-providers"]
        };

        let profile_path = self.profile_dir.join(profile_name);
        let mut profile_yaml_path = profile_path.clone();
        let mut net_res: Vec<(String, String)> = Vec::new();
        // ## 如果是订阅链接
        if !self.is_profile_yaml(profile_name) {
            let mut file = File::open(profile_path)?;
            let mut file_content = String::new();
            file.read_to_string(&mut file_content)?;

            let sub_url = file_content.trim();
            let mut response = self.clash_api.mock_clash_core(sub_url)?;

            profile_yaml_path = self.get_profile_yaml_path(profile_name);
            let directory = profile_yaml_path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;
            if !directory.exists() {
                create_dir_all(directory)?;
            }
            let mut output_file = File::create(&profile_yaml_path)?;
            response.copy_to(&mut output_file)?;

            net_res.push((
                sub_url.to_string(),
                profile_yaml_path.to_string_lossy().to_string(),
            ))
        }

        // ## 更新 yaml 的网络资源
        let mut file = File::open(profile_yaml_path)?;
        let mut yaml_content = String::new();
        file.read_to_string(&mut yaml_content)?;

        let parsed_yaml: serde_yaml::Value = serde_yaml::from_str(yaml_content.as_str()).unwrap();

        for key in &net_res_keys {
            let providers =
                if let Some(serde_yaml::Value::Mapping(providers)) = parsed_yaml.get(key) {
                    providers
                } else {
                    continue;
                };

            for (_, provider_value) in providers {
                let provider_content =
                    if let serde_yaml::Value::Mapping(provider_content) = provider_value {
                        provider_content
                    } else {
                        continue;
                    };

                let t = if let Some(serde_yaml::Value::String(t)) = provider_content.get("type") {
                    t
                } else {
                    continue;
                };

                if t != "http" {
                    continue;
                }

                if let (
                    Some(serde_yaml::Value::String(url)),
                    Some(serde_yaml::Value::String(path)),
                ) = (provider_content.get("url"), provider_content.get("path"))
                {
                    net_res.push((url.clone(), path.clone()))
                }
            }
        }

        let mut updated_res = vec![];
        let mut not_updated_res = vec![];
        for (url, path) in &net_res {
            match self.download_file(
                url,
                &Path::new(&self.clashtui_config.borrow().clash_cfg_dir).join(path),
            ) {
                Ok(_) => {
                    updated_res.push((url.clone(), path.clone()));
                }
                Err(err) => {
                    not_updated_res.push((url.clone(), path.clone()));
                    log::error!("Failed to Download file: {}", err);
                }
            }
        }

        Ok((updated_res, not_updated_res))
    }

    fn download_file(&self, url: &String, path: &PathBuf) -> anyhow::Result<()> {
        let mut response = self.clash_api.mock_clash_core(url)?;

        let directory = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;
        if !directory.exists() {
            create_dir_all(directory)?;
        }

        let mut output_file = File::create(path)?;
        response.copy_to(&mut output_file)?;
        Ok(())
    }

    pub fn create_yaml_with_template(&self, template_name: &String) -> anyhow::Result<()> {
        let template_dir = self.clashtui_dir.join("templates");
        let template_path = template_dir.join(template_name);
        let tpl_parsed_yaml = parse_yaml(&template_path)?;
        let mut out_parsed_yaml = tpl_parsed_yaml.clone();

        let proxy_url_file =
            File::open(self.clashtui_dir.join("templates/template_proxy_providers")).unwrap();
        let proxy_url_reader = BufReader::new(proxy_url_file);
        let mut proxy_urls = Vec::new();
        for line in proxy_url_reader.lines() {
            let line = line.as_ref().unwrap().trim();
            if !line.is_empty() && !line.starts_with('#') {
                proxy_urls.push(line.to_string());
            }
        }

        // ## proxy-providers
        // e.g. {provider: [provider0, provider1, ...]}
        let mut pp_names: HashMap<String, Vec<String>> = HashMap::new(); // proxy-provider names
        let mut new_proxy_providers = HashMap::new();
        let pp_mapping = if let Some(serde_yaml::Value::Mapping(pp_mapping)) =
            tpl_parsed_yaml.get("proxy-providers")
        {
            pp_mapping
        } else {
            anyhow::bail!("Failed to parse `proxy-providers`");
        };

        for (pp_key, pp_value) in pp_mapping {
            if pp_value.get("tpl_param") == None {
                new_proxy_providers.insert(pp_key.clone(), pp_value.clone());
                continue;
            }

            let pp = if let serde_yaml::Value::Mapping(pp) = pp_value {
                pp
            } else {
                anyhow::bail!("Failed to parse `proxy-providers` value");
            };

            let mut new_pp = pp.clone();
            new_pp.remove("tpl_param");

            for (i, url) in proxy_urls.iter().enumerate() {
                // name: e.g. provier0, provider1, ...
                let the_pp_name = format!("{}{}", pp_key.as_str().unwrap(), i);
                pp_names
                    .entry(pp_key.as_str().unwrap().to_string())
                    .or_insert_with(Vec::new)
                    .push(the_pp_name.clone());

                new_pp.insert(
                    serde_yaml::Value::String("url".to_string()),
                    serde_yaml::Value::String(url.clone()),
                );
                new_pp.insert(
                    serde_yaml::Value::String("path".to_string()),
                    serde_yaml::Value::String(format!("proxy-providers/tpl/{}.yaml", the_pp_name)),
                );
                new_proxy_providers.insert(
                    serde_yaml::Value::String(the_pp_name.clone()),
                    serde_yaml::Value::Mapping(new_pp.clone()),
                );
            }
        }
        out_parsed_yaml["proxy-providers"] = serde_yaml::to_value(new_proxy_providers).unwrap();

        // ## proxy-groups
        // e.g. {Auto: [Auto-provider0, Auto-provider1, ...], Select: [Select-provider0, ...]}
        let mut pg_names: HashMap<String, Vec<String>> = HashMap::new();
        let mut new_proxy_groups = serde_yaml::Sequence::new();
        let pg_value = if let Some(serde_yaml::Value::Sequence(pg_value)) =
            tpl_parsed_yaml.get("proxy-groups")
        {
            pg_value
        } else {
            anyhow::bail!("Failed to parse `proxy-groups`.");
        };

        for the_pg_value in pg_value {
            if the_pg_value.get("tpl_param") == None {
                new_proxy_groups.push(the_pg_value.clone());
                continue;
            }

            let the_pg = if let serde_yaml::Value::Mapping(the_pg) = the_pg_value {
                the_pg
            } else {
                anyhow::bail!("Failed to parse `proxy-groups` value");
            };

            let mut new_pg = the_pg.clone();
            new_pg.remove("tpl_param");

            let provider_keys = if let Some(serde_yaml::Value::Sequence(provider_keys)) =
                the_pg["tpl_param"].get("providers")
            {
                provider_keys
            } else {
                anyhow::bail!("Failed to parse `providers` in `tpl_param`");
            };

            for the_provider_key in provider_keys {
                let the_pk_str = if let serde_yaml::Value::String(the_pk_str) = the_provider_key {
                    the_pk_str
                } else {
                    anyhow::bail!("Failed to parse string in `providers`");
                };

                let names = if let Some(names) = pp_names.get(the_pk_str) {
                    names
                } else {
                    continue;
                };

                let the_pg_name = if let Some(serde_yaml::Value::String(the_pg_name)) =
                    the_pg_value.get("name")
                {
                    the_pg_name
                } else {
                    anyhow::bail!("Failed to parse `name` in `proxy-groups`")
                };

                for n in names {
                    // new_pg_name: e.g. Auto-provider0, Auto-provider1, Select-provider0, ...
                    let new_pg_name = format!("{}-{}", the_pg_name, n); // proxy-group
                                                                        // names

                    pg_names
                        .entry(the_pg_name.clone())
                        .or_insert_with(Vec::new)
                        .push(new_pg_name.clone());

                    new_pg["name"] = serde_yaml::Value::String(new_pg_name.clone());
                    new_pg.insert(
                        serde_yaml::Value::String("use".to_string()),
                        serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(n.clone())]),
                    );

                    new_proxy_groups.push(serde_yaml::Value::Mapping(new_pg.clone()));
                }
            }
        }
        out_parsed_yaml["proxy-groups"] = serde_yaml::Value::Sequence(new_proxy_groups);

        // ### replace special keys in group-providers
        // e.g. <provider> => provider0, provider1
        // e.g. <Auto> => Auto-provider0, Auto-provider1
        // e.g. <Select> => Select-provider0, Select-provider1
        let pg_sequence = if let Some(serde_yaml::Value::Sequence(pg_sequence)) =
            out_parsed_yaml.get_mut("proxy-groups")
        {
            pg_sequence
        } else {
            anyhow::bail!("Failed to parse `proxy-groups`");
        };

        for the_pg_seq in pg_sequence {
            if let Some(providers) = the_pg_seq.get("use") {
                let mut new_providers = Vec::new();
                for p in providers.as_sequence().unwrap() {
                    let p_str = p.as_str().unwrap();
                    if p_str.starts_with('<') && p_str.ends_with('>') {
                        let trimmed_p_str = p_str.trim_matches(|c| c == '<' || c == '>');
                        let provider_names = pp_names.get(trimmed_p_str).unwrap();
                        new_providers.extend(provider_names.iter().cloned());
                    } else {
                        new_providers.push(p_str.to_string().clone());
                    }
                }
                the_pg_seq["use"] = serde_yaml::Value::Sequence(
                    new_providers
                        .iter()
                        .map(|p| serde_yaml::Value::String(p.clone()))
                        .collect(),
                );
            }

            if let Some(serde_yaml::Value::Sequence(groups)) = the_pg_seq.get("proxies") {
                let mut new_groups = Vec::new();
                for g in groups {
                    let g_str = g.as_str().unwrap();
                    if g_str.starts_with('<') && g_str.ends_with('>') {
                        let trimmed_g_str = g_str.trim_matches(|c| c == '<' || c == '>');
                        let group_names = pg_names.get(trimmed_g_str).unwrap();
                        new_groups.extend(group_names.iter().cloned());
                    } else {
                        new_groups.push(g_str.to_string().clone());
                    }
                }
                the_pg_seq["proxies"] = serde_yaml::Value::Sequence(
                    new_groups
                        .iter()
                        .map(|g| serde_yaml::Value::String(g.clone()))
                        .collect(),
                );
            }
        }

        log::error!("testssdfs");
        let out_yaml_path = self.profile_dir.join(template_name);
        let out_yaml_file = File::create(out_yaml_path).unwrap();
        serde_yaml::to_writer(out_yaml_file, &out_parsed_yaml)?;

        Ok(())
    }

    pub fn get_profile_names(&self) -> anyhow::Result<Vec<String>> {
        Utils::get_file_names(self.profile_dir.as_path()).and_then(|mut v| {
            v.sort();
            Ok(v)
        })
    }
    pub fn get_template_names(&self) -> anyhow::Result<Vec<String>> {
        Utils::get_file_names(self.clashtui_dir.join("templates").as_path()).and_then(|mut v| {
            v.sort();
            Ok(v)
        })
    }
    #[cfg(target_os = "linux")]
    pub fn clash_srv_ctl(&self, op: ClashTuiOp) -> Result<String, Error> {
        let tuiconf = self.clashtui_config.borrow();
        match op {
            ClashTuiOp::StartClashService => {
                let output = match Command::new("systemctl")
                    .arg("restart")
                    .arg(tuiconf.clash_srv_name.as_str())
                    .output()
                {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                };

                return Utils::string_process_output(output);
            }
            ClashTuiOp::StopClashService => {
                let output = match Command::new("systemctl")
                    .arg("stop")
                    .arg(tuiconf.clash_srv_name.as_str())
                    .output()
                {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                };
                return Utils::string_process_output(output);
            }
            ClashTuiOp::TestClashConfig => {
                return self.test_profile_config(&Path::new(&tuiconf.clash_cfg_path), false);
            }
            ClashTuiOp::SetPermission => {
                let output = match Command::new("setcap")
                    .arg("'cap_net_admin,cap_net_bind_service=+ep'")
                    .arg(self.clashtui_config.borrow().clash_core_path.clone())
                    .output()
                {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                };
                return Utils::string_process_output(output);
            }
        }
    }
    #[cfg(target_os = "windows")]
    pub fn clash_srv_ctl(&self, op: ClashTuiOp) -> Result<String, Error> {
        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        //let nssm_path = exe_dir.join("nssm");
        //let nssm_path_str = nssm_path.to_str().unwrap();
        let nssm_path_str = "nssm";

        let output = match op {
            ClashTuiOp::StartClashService => {
                Self::start_process_as_admin(
                    nssm_path_str,
                    format!("restart {}", self.clash_srv_name).as_str(),
                    true,
                )?;

                Command::new(nssm_path_str)
                    .arg("status")
                    .arg(self.clash_srv_name.as_str())
                    .output()?
            }

            ClashTuiOp::StopClashService => {
                Self::start_process_as_admin(
                    nssm_path_str,
                    &format!("stop {}", self.clash_srv_name),
                    true,
                )?;

                Command::new(nssm_path_str)
                    .arg("status")
                    .arg(self.clash_srv_name.as_str())
                    .output()?
            }

            ClashTuiOp::TestClashConfig => {
                return self.test_profile_config(&self.clash_cfg_path, false);
            }

            ClashTuiOp::InstallSrv => {
                Self::start_process_as_admin(
                    nssm_path_str,
                    &format!(
                        "install {} \"{}\" -d \"{}\" -f \"{}\"",
                        self.clash_srv_name,
                        self.clash_core_path.to_str().unwrap(),
                        self.clash_cfg_dir.to_str().unwrap(),
                        self.clash_cfg_path.to_str().unwrap()
                    ),
                    true,
                )?;

                Command::new(nssm_path_str)
                    .arg("status")
                    .arg(self.clash_srv_name.as_str())
                    .output()?
            }

            ClashTuiOp::UnInstallSrv => Self::execute_powershell_script_as_admin(
                &format!(
                    "{0} stop {1}; {0} remove {1}",
                    nssm_path_str, self.clash_srv_name
                ),
                true,
            )?,

            ClashTuiOp::EnableLoopback => {
                let exe_dir = std::env::current_exe()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .to_path_buf();
                Self::start_process_as_admin(
                    exe_dir.join("EnableLoopback").to_str().unwrap(),
                    "",
                    false,
                )?
            }
            _ => {
                bail!("Do nothing for the ClashTuiOp.")
            }
        };

        Self::string_process_output(output)
    }

    #[cfg(target_os = "linux")]
    pub fn update_state(&self, new_pf: Option<String>) -> _State {
        if new_pf.is_none() {
            // act when app is initing.
            if let Some(e) = self.fetch_remote() {
                if !e.is_connect() {
                    log::warn!("{}", e);
                }
            };
        }
        let mut tuiconf = self.clashtui_config.borrow_mut();
        let pf = match new_pf {
            Some(v) => {
                tuiconf.update_profile(v.clone());
                v
            }
            None => tuiconf.cur_profile.clone(),
        };
        let mode;
        let tun = match self.clash_remote_config.borrow().as_ref() {
            Some(v) => {
                mode = v.mode.to_string();
                if v.tun.enable {
                    v.tun.stack.to_string()
                } else {
                    "False".to_string()
                }
            }
            None => {
                mode = "UnKnown".to_string();
                "Unknown".to_string()
            }
        };
        _State::new(pf, mode, tun)
    }

    pub fn test_profile_config(&self, path: &Path, geodata_mode: bool) -> Result<String, Error> {
        let tuiconf = self.clashtui_config.borrow();
        let cmd = if geodata_mode {
            format!(
                "{} -m -d {} -f {} -t",
                tuiconf.clash_core_path.as_str(),
                tuiconf.clash_cfg_dir.as_str(),
                path.to_str().unwrap(),
            )
        } else {
            format!(
                "{} -d {} -f {} -t",
                tuiconf.clash_core_path.as_str(),
                tuiconf.clash_cfg_dir.as_str(),
                path.to_str().unwrap(),
            )
        };
        #[cfg(target_os = "linux")]
        let output = Command::new("sh").arg("-c").arg(&cmd).output().unwrap();
        #[cfg(target_os = "windows")]
        let output = Command::new("cmd").arg("/C").arg(&cmd).output()?;
        return Utils::string_process_output(output);
    }

    // 目前是根据文件后缀来判断, 而不是文件内容。这样可以减少 io。
    pub fn is_profile_yaml(&self, profile_name: &String) -> bool {
        let profile_path = self.profile_dir.join(profile_name);
        let extension = profile_path.extension();
        if extension == Some("yaml".as_ref()) || extension == Some("yml".as_ref()) {
            return true;
        }

        return false;
    }

    pub fn get_profile_yaml_path(&self, profile_name: &String) -> PathBuf {
        let profile_path = self.profile_dir.join(profile_name);

        if self.is_profile_yaml(profile_name) {
            return profile_path;
        } else {
            let profile_cache_dir = self.clashtui_dir.join("profile_cache");
            let profile_yaml_name = Path::new(profile_name).with_extension("yaml");
            let profile_yaml_path = profile_cache_dir.join(profile_yaml_name);

            return profile_yaml_path;
        }
    }

    /*
    pub fn is_yaml(path: &Path) -> bool {
        if let Ok(file_content) = fs::read_to_string(&path) {
            if let Ok(_) = serde_yaml::from_str::<serde_yaml::Value>(&file_content) {
                return true;
            }
        }

        false
    }
    pub fn edit_file(&self, path: &PathBuf) -> Result<String> {
        if let Some(edit_cmd) = self
            .get_clashtui_config()
            .get("default")
            .and_then(|default| default.get("edit_cmd"))
            .and_then(|edit_cmd| edit_cmd.as_str())
        {
            let edit_cmd_with_path = edit_cmd.replace("%s", path.to_str().unwrap_or(""));

            let output = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .arg("/C")
                    .arg(&edit_cmd_with_path)
                    .spawn()?;
            } else {
                Command::new("sh")
                    .arg("-c")
                    .arg(&edit_cmd_with_path)
                    .spawn()?;
            };

            return Ok("Done".to_string());
        }

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .arg("/C")
                .arg("start")
                .arg(path.to_str().unwrap_or(""))
                .spawn()?;
        } else {
            Command::new("xdg-open")
                .arg(path.to_str().unwrap_or(""))
                .spawn()?;
        };

        Ok("Done".to_string())
    }
    pub fn open_dir(&self, path: &Path) -> Result<String> {
        if let Some(opendir_cmd) = self
            .get_clashtui_config()
            .get("default")
            .and_then(|default| default.get("opendir_cmd"))
            .and_then(|opendir_cmd| opendir_cmd.as_str())
        {
            let opendir_cmd_with_path = opendir_cmd.replace("%s", path.to_str().unwrap_or(""));

            if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .arg("/C")
                    .arg("opendir_cmd_with_path")
                    .spawn()?;
            } else {
                Command::new("sh")
                    .arg("-c")
                    .arg(&opendir_cmd_with_path)
                    .spawn()?;
            }

            return Ok("Done".to_string());
        }

        if cfg!(target_os = "windows") {
            Command::new("cmd")
                .arg("/C")
                .arg("start")
                .arg(path.to_str().unwrap_or(""))
                .spawn()?;
        } else {
            Command::new("xdg-open")
                .arg(path.to_str().unwrap_or(""))
                .spawn()?;
        };

        Ok("Done".to_string())
    }
    */

    pub fn fetch_recent_logs(&self, num_lines: usize) -> Vec<String> {
        let log = std::fs::read_to_string(self.clashtui_dir.join("clashtui.log"))
            .unwrap_or_else(|_| String::new());
        log.lines()
            .rev()
            .take(num_lines)
            .map(String::from)
            .collect()
    }
}

trait MonkeyPatchVec {
    // to make the code more 'beautiful'
    fn push_if_not_exist(&mut self, value: ClashTuiConfigLoadError);
}
impl MonkeyPatchVec for Vec<ClashTuiConfigLoadError> {
    fn push_if_not_exist(&mut self, value: ClashTuiConfigLoadError) {
        if !self.contains(&value) {
            self.push(value)
        };
    }
}

fn get_proxy_addr(yaml_data: &serde_yaml::Value) -> String {
    let host = "127.0.0.1";
    if let Some(port) = yaml_data.get("mixed-port").and_then(|v| v.as_u64()) {
        return format!("http://{}:{}", host, port);
    }
    if let Some(port) = yaml_data.get("port").and_then(|v| v.as_u64()) {
        return format!("http://{}:{}", host, port);
    }
    if let Some(port) = yaml_data.get("socks-port").and_then(|v| v.as_u64()) {
        return format!("socks5://{}:{}", host, port);
    }

    format!("http://{}:7890", host)
}

fn parse_yaml(yaml_path: &Path) -> anyhow::Result<serde_yaml::Value> {
    let mut file = File::open(yaml_path)?;
    let mut yaml_content = String::new();
    file.read_to_string(&mut yaml_content)?;
    let parsed_yaml_content: serde_yaml::Value =
        serde_yaml::from_str(yaml_content.as_str()).unwrap();
    Ok(parsed_yaml_content)
}

fn load_app_config(
    clashtui_dir: &PathBuf,
    skip_load_conf: bool,
) -> (ClashTuiConfig, String, String, Vec<ClashTuiConfigLoadError>) {
    let mut err_collect = Vec::new();
    let basic_clash_config_path = Path::new(clashtui_dir).join("basic_clash_config.yaml");
    let basic_clash_config_value: serde_yaml::Value =
        match parse_yaml(basic_clash_config_path.as_path()) {
            Ok(r) => r,
            Err(_) => {
                err_collect.push(ClashTuiConfigLoadError::LoadProfileConfig(
                    "Fail to load User Defined Config\n".into(),
                ));
                serde_yaml::Value::from("")
            }
        };
    let controller_api = if let Some(controller_api) = basic_clash_config_value
        .get("external-controller")
        .and_then(|v| v.as_str())
    {
        format!("http://{}", controller_api)
    } else {
        "http://127.0.0.1:9090".to_string()
    };
    log::debug!("controller_api: {}", controller_api);

    let proxy_addr = get_proxy_addr(&basic_clash_config_value);
    log::debug!("proxy_addr: {}", proxy_addr);

    let configs;
    if !skip_load_conf {
        let config_path = clashtui_dir.join("config.yaml");
        configs = match ClashTuiConfig::from_file(config_path.to_str().unwrap()) {
            Ok(v) => {
                if !v.check() {
                    err_collect.push(ClashTuiConfigLoadError::LoadAppConfig(
                        "Some Key Configs are missing, or Default\n".into(),
                    ));
                    log::warn!("Empty Config?");
                    log::debug!("{:?}", v)
                };
                v
            }
            Err(e) => {
                err_collect.push(ClashTuiConfigLoadError::LoadAppConfig(
                    "Fail to load configs, using Default\n".into(),
                ));
                log::error!("Unable to load config file. {}", e);
                ClashTuiConfig::default()
            }
        };
    } else {
        configs = ClashTuiConfig::default();
    }

    (configs, controller_api, proxy_addr, err_collect)
}
