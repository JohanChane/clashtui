use crate::utils::tui::{NetProviderMap, UpdateProviderType, ProfileType};
use api::ProfileTimeMap;

use super::ClashTuiUtil;
use crate::utils::{is_yaml, utils as Utils};
use api::ProfileSectionType;
use std::{
    fs::{create_dir_all, File},
    io::Error,
    path::{Path, PathBuf},
};

impl ClashTuiUtil {
    pub fn crt_yaml_with_template(&self, template_name: &String) -> Result<(), String> {
        use std::borrow::Cow;
        use std::collections::HashMap;
        use std::io::{BufRead, BufReader};
        let template_dir = self.clashtui_dir.join("templates");
        let template_path = template_dir.join(template_name);
        let tpl_parsed_yaml =
            Utils::parse_yaml(&template_path).map_err(|e| format!("parse failed: {e:?}"))?;
        let mut out_parsed_yaml = Cow::Borrowed(&tpl_parsed_yaml);

        let proxy_url_file =
            File::open(self.clashtui_dir.join("templates/template_proxy_providers"))
                .map_err(|e| format!("open template_proxy_providers: {e:?}"))?;
        let proxy_urls: Vec<String> = BufReader::new(proxy_url_file)
            .lines()
            .map_while(Result::ok)
            .filter(|v| {
                let val = v.trim();
                !(val.is_empty() || val.starts_with('#'))
            })
            .collect();

        // ## proxy-providers
        // e.g. {provider: [provider0, provider1, ...]}
        let mut pp_names: HashMap<String, Vec<String>> = HashMap::new(); // proxy-provider names
        let mut new_proxy_providers = HashMap::new();
        let pp_mapping = if let Some(serde_yaml::Value::Mapping(pp_mapping)) =
            tpl_parsed_yaml.get("proxy-providers")
        {
            pp_mapping
        } else {
            return Err(String::from("Failed to parse `proxy-providers`"));
        };

        for (pp_key, pp_value) in pp_mapping {
            if pp_value.get("tpl_param").is_none() {
                new_proxy_providers.insert(pp_key.clone(), pp_value.clone());
                continue;
            }

            let pp = pp_value
                .as_mapping()
                .ok_or("Failed to parse `proxy-providers` value".to_string())?;

            for (i, url) in proxy_urls.iter().enumerate() {
                let mut new_pp = pp.clone();
                new_pp.remove("tpl_param");
                // name: e.g. provier0, provider1, ...
                let the_pp_name = format!("{}{}", pp_key.as_str().unwrap(), i);
                pp_names
                    .entry(pp_key.as_str().unwrap().to_string())
                    .or_default()
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
        out_parsed_yaml.to_mut()["proxy-providers"] =
            serde_yaml::to_value(new_proxy_providers).unwrap();

        // ## proxy-groups
        // e.g. {Auto: [Auto-provider0, Auto-provider1, ...], Select: [Select-provider0, ...]}
        let mut pg_names: HashMap<String, Vec<String>> = HashMap::new();
        let mut new_proxy_groups = serde_yaml::Sequence::new();
        let pg_value = if let Some(serde_yaml::Value::Sequence(pg_value)) =
            tpl_parsed_yaml.get("proxy-groups")
        {
            pg_value
        } else {
            return Err(String::from("Failed to parse `proxy-groups`."));
        };

        for the_pg_value in pg_value {
            if the_pg_value.get("tpl_param").is_none() {
                new_proxy_groups.push(the_pg_value.clone());
                continue;
            }

            let the_pg = if let serde_yaml::Value::Mapping(the_pg) = the_pg_value {
                the_pg
            } else {
                return Err(String::from("Failed to parse `proxy-groups` value"));
            };

            let mut new_pg = the_pg.clone();
            new_pg.remove("tpl_param");

            let provider_keys = if let Some(serde_yaml::Value::Sequence(provider_keys)) =
                the_pg["tpl_param"].get("providers")
            {
                provider_keys
            } else {
                return Err(String::from("Failed to parse `providers` in `tpl_param`"));
            };

            for the_provider_key in provider_keys {
                let the_pk_str = if let serde_yaml::Value::String(the_pk_str) = the_provider_key {
                    the_pk_str
                } else {
                    return Err(String::from("Failed to parse string in `providers`"));
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
                    return Err(String::from("Failed to parse `name` in `proxy-groups`"));
                };

                for n in names {
                    // new_pg_name: e.g. Auto-provider0, Auto-provider1, Select-provider0, ...
                    let new_pg_name = format!("{}-{}", the_pg_name, n); // proxy-group
                                                                        // names

                    pg_names
                        .entry(the_pg_name.clone())
                        .or_default()
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
        out_parsed_yaml.to_mut()["proxy-groups"] = serde_yaml::Value::Sequence(new_proxy_groups);

        // ### replace special keys in group-providers
        // e.g. <provider> => provider0, provider1
        // e.g. <Auto> => Auto-provider0, Auto-provider1
        // e.g. <Select> => Select-provider0, Select-provider1
        let pg_sequence = if let Some(serde_yaml::Value::Sequence(pg_sequence)) =
            out_parsed_yaml.to_mut().get_mut("proxy-groups")
        {
            pg_sequence
        } else {
            return Err(String::from("Failed to parse `proxy-groups`"));
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
                        new_providers.push(p_str.to_string());
                    }
                }
                the_pg_seq["use"] = serde_yaml::Value::Sequence(
                    new_providers
                        .into_iter()
                        .map(serde_yaml::Value::String)
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
                        new_groups.push(g_str.to_string());
                    }
                }
                the_pg_seq["proxies"] = serde_yaml::Value::Sequence(
                    new_groups
                        .into_iter()
                        .map(serde_yaml::Value::String)
                        .collect(),
                );
            }
        }

        let out_yaml_path = self.profile_dir.join(template_name);
        let out_yaml_file = File::create(out_yaml_path).map_err(|e| e.to_string())?;
        serde_yaml::to_writer(out_yaml_file, &out_parsed_yaml).map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn crt_profile(&self, profile_name: String, uri: String) -> Result<(), String> {
        let profile_name = profile_name.trim();
        let uri = uri.trim();

        if uri.is_empty() || profile_name.is_empty() {
            return Err("Url or Name is empty!".to_string());
        }

        if uri.starts_with("http://") || uri.starts_with("https://") {
            use std::io::Write as _;
            std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(self.get_profile_path_unchecked(profile_name))
                .and_then(|mut f| write!(f, "{}", uri))
                .map_err(|e| e.to_string())
        } else if Path::new(uri).is_file() {
            let uri_path = self.get_profile_path_unchecked(uri);
            if uri_path.exists() {
                return Err("Failed to import: file exists".to_string());
            }
            std::fs::copy(uri, uri_path)
                .map_err(|e| e.to_string())
                .map(|_| ())
        } else {
            Err("Url is invalid.".to_string())
        }
    }

    pub fn rmf_profile(&self, profile_name: &String) -> Result<(), String> {
        use std::fs::remove_file;
        if self.get_profile_type(profile_name).is_some_and(|t| t == ProfileType::Url) {
            let _ = remove_file(self.get_profile_cache_unchecked(profile_name));  // Not important
        }
        remove_file(self.get_profile_path_unchecked(profile_name)).map_err(|e| e.to_string())
    }

    pub fn test_profile_config(&self, path: &str, geodata_mode: bool) -> std::io::Result<String> {
        use crate::utils::ipc::exec;
        let cmd = format!(
            "{} {} -d {} -f {} -t",
            self.tui_cfg.clash_core_path,
            if geodata_mode { "-m" } else { "" },
            self.tui_cfg.clash_cfg_dir,
            path,
        );
        return exec("cmd", vec!["/C", cmd.as_str()]);
    }

    pub fn select_profile(&self, profile_name: &String) -> std::io::Result<()> {
        if let Err(err) = self.merge_profile(profile_name) {
            log::error!(
                "Failed to Merge Profile `{}`: {}",
                profile_name,
                err.to_string()
            );
            return Err(Error::new(std::io::ErrorKind::Other, err));
        };
        let body = serde_json::json!({
            "path": self.tui_cfg.clash_cfg_path.as_str(),
            "payload": ""
        })
        .to_string();
        if let Err(err) = self.config_reload(body) {
            log::error!(
                "Failed to Patch Profile `{}`: {}",
                profile_name,
                err.to_string()
            );
            return Err(Error::new(std::io::ErrorKind::Other, err));
        };
        Ok(())
    }

    fn merge_profile(&self, profile_name: &String) -> std::io::Result<()> {
        let basic_clash_cfg_path = self.clashtui_dir.join(super::BASIC_FILE);
        let mut dst_parsed_yaml = Utils::parse_yaml(&basic_clash_cfg_path)?;
        let profile_yaml_path = self.get_profile_yaml_path(profile_name)?;
        let profile_parsed_yaml = Utils::parse_yaml(&profile_yaml_path).map_err(|e| {
            Error::new(
                e.kind(),
                format!(
                    "Maybe need to update first. Failed to parse {}: {e}",
                    profile_yaml_path.to_str().unwrap()
                ),
            )
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

        let final_clash_cfg_file = File::create(&self.tui_cfg.clash_cfg_path)?;
        serde_yaml::to_writer(final_clash_cfg_file, &dst_parsed_yaml)
            .map_err(|e| Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        Ok(())
    }

    pub fn update_profile(
        &self,
        profile_name: &String,
        does_update_all: bool,
    ) -> std::io::Result<Vec<String>> {
        self.update_profile_with_clashtui(profile_name, does_update_all)
        //self.update_profile_with_api(profile_name, does_update_all)
    }

    // The advantage of using this interface for updates is that you can know the reason for update failures without needing to check mihomo's logs. The downside is that it requires resolving file permission issues.
    pub fn update_profile_with_clashtui(
        &self,
        profile_name: &String,
        does_update_all: bool,
    ) -> std::io::Result<Vec<String>> {
        let mut profile_yaml_path = self.profile_dir.join(profile_name);
        let mut result = Vec::new();
        if self.get_profile_type(profile_name)
            .is_some_and(|t| t == ProfileType::Url)
        {
            let sub_url = self.extract_profile_url(profile_name)?;
            profile_yaml_path = self.get_profile_cache_unchecked(profile_name);
            // Update the file to keep up-to-date
            self.download_profile(sub_url.as_str(), &profile_yaml_path)?;

            result.push(format!("Updated: {}, {}", profile_name, sub_url));
        }

        let mut section_types = vec![ProfileSectionType::ProxyProvider];
        if does_update_all {
            section_types.push(ProfileSectionType::RuleProvider);
        }

        let mut net_providers = NetProviderMap::new();
        if let Ok(providers) = self.extract_net_providers(&profile_yaml_path, &section_types) {
            net_providers.extend(providers);
        }

        for (_, providers) in net_providers {
            for (name, url, path) in providers {
                match self.download_profile(&url, &Path::new(&self.tui_cfg.clash_cfg_dir).join(&path)) {
                    Ok(_) => result.push(format!("Updated: {}, {}", name, url)),
                    Err(e) => result.push(format!("Not updated: {}, {}, {}", name, url, e)),

                }
            }
        }

        Ok(result)
    }

    // Using api update, the user needs to check the logs to understand why the updates failed. The success rate of my testing updates is not as high as using clashtui.
    #[cfg(target_feature = "deprecated")]
    pub fn update_profile_with_api(
        &self,
        profile_name: &String,
        does_update_all: bool,
    ) -> std::io::Result<Vec<String>> {
        let mut result = Vec::new();
        if self.get_profile_type(profile_name)
            .is_some_and(|t| t == ProfileType::Url)
        {
            let sub_url = self.extract_profile_url(profile_name)?;
            let profile_yaml_path = self.get_profile_cache_unchecked(profile_name);
            // Update the file to keep up-to-date
            self.download_profile(sub_url.as_str(), &profile_yaml_path)?;

            result.push(
                format!("Updated: {}, {}", profile_name, sub_url)
            );
        }

        let mut provider_types = vec![ProfileSectionType::ProxyProvider];
        if does_update_all {
            provider_types.push(ProfileSectionType::RuleProvider);
        }

        let mut update_providers_result = UpdateProviderType::new();
        let mut update_times = ProfileTimeMap::new();
        for t in provider_types {
            // Get result of update providers
            update_providers_result.insert(t, self.clash_api.update_providers(t)?);

            // Get update times after update providers
            if let Ok(name_times) = self.clash_api.extract_provider_utimes_with_api(t) {
                update_times.insert(t, name_times);
            }
        }

        // Add results of updating providers
        for (section_type, res) in update_providers_result {
            for (name, r) in res {
                // Generate duration_str
                let duration_str = if let Ok(d) = Self::cal_mtime_duration(&update_times, section_type, &name) {
                    Utils::str_duration(d)
                } else {
                    "No update times or can't cal the duration".to_string()
                };

                let line = match r {
                    Ok(_) => {
                        format!("Sent update request: {}, duration = '{}'", name, duration_str)
                    }
                    Err(err) => {
                        log::error!("Not Sent update request:{err}");
                        format!("Not Sent update request: {}, duration = '{}'", name, duration_str)
                    }
                };
                result.push(line);
            }
        }

        Ok(result)
    }

    fn download_profile(&self, url: &str, path: &PathBuf) -> std::io::Result<()> {
        let directory = path
            .parent()
            .ok_or_else(|| Error::new(std::io::ErrorKind::NotFound, "Invalid file path"))?;
        if !directory.exists() {
            create_dir_all(directory)?;
        }

        let response = self.dl_remote_profile(url)?;
        let mut output_file = File::create(path)?;      // will truncate the file
        response.copy_to(&mut output_file)?;
        Ok(())
    }

    fn get_provider_mtime(&self, section_types: Vec<ProfileSectionType>, profile_yaml_path: &PathBuf) -> std::io::Result<ProfileTimeMap> {
        let mut modify_info = ProfileTimeMap::new();
        if let Ok(net_res) = self.extract_net_providers(profile_yaml_path, &section_types) {
            for (key, res) in net_res {
                let name_and_times = res.into_iter().map(|(name, _, path)| {
                    let clash_cfg_dir = Path::new(&self.tui_cfg.clash_cfg_dir);
                    let time = Utils::get_mtime(clash_cfg_dir.join(path)).ok();
                    (name, time)
                }).collect();
                modify_info.insert(key, name_and_times);
            }
        }

        Ok(modify_info)
    }

    pub fn extract_net_providers(&self, profile_yaml_path: &PathBuf, provider_types: &Vec<ProfileSectionType>) -> std::io::Result<NetProviderMap> {
        let yaml_content = std::fs::read_to_string(&profile_yaml_path)?;
        let parsed_yaml = match serde_yaml::from_str::<serde_yaml::Value>(&yaml_content) {
            Ok(value) => value,
            Err(err) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
        };

        let provider_keys: Vec<_> = provider_types.iter().filter_map(|s_type| {
            match s_type {
                ProfileSectionType::ProxyProvider => Some("proxy-providers"),
                ProfileSectionType::RuleProvider => Some("rule-providers"),
                _ => None,
            }
        }).collect();

        let mut net_providers = NetProviderMap::new();
        for section_key in provider_keys {
            let section_val = if let Some(val) = parsed_yaml.get(section_key) {
                val
            } else {
                continue;
            };

            let the_section_val = if let serde_yaml::Value::Mapping(val) = section_val {
                val
            } else {
                continue;
            };

            let mut providers: Vec<(String, String, String)> = Vec::new();
            for (provider_key, provider_val) in the_section_val {
                let provider = if let Some(val) = provider_val.as_mapping() {
                    val
                } else {
                    continue;
                };

                if let (Some(name), Some(url), Some(path)) = (
                    Some(provider_key),
                    provider.get(&serde_yaml::Value::String("url".to_string())),
                    provider.get(&serde_yaml::Value::String("path".to_string())),
                ) {
                    if let (serde_yaml::Value::String(name), serde_yaml::Value::String(url), serde_yaml::Value::String(path)) = (name, url, path) {
                        providers.push((name.clone(), url.clone(), path.clone()));
                    }
                }
            }

            if section_key == "proxy-providers" {
                net_providers.insert(ProfileSectionType::ProxyProvider, providers);
            } else if section_key == "rule-providers" {
                net_providers.insert(ProfileSectionType::RuleProvider, providers);
            }
        }

        Ok(net_providers)
    }

    // duration: now - mtime
    fn cal_mtime_duration(mtimes: &ProfileTimeMap, section_type: ProfileSectionType, name: &String) -> std::io::Result<std::time::Duration> {
        let mt = Self::extract_the_mtime(mtimes, section_type, name).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "No the mtime in mtimes"))?;
        let now = std::time::SystemTime::now();
        now.duration_since(mt).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    fn extract_the_mtime<'a>(mtimes: &'a ProfileTimeMap, section_type: ProfileSectionType, name: &String) -> &'a Option<std::time::SystemTime> {
        if let Some(times) = mtimes.get(&section_type) {
            for (n, the_mtime) in times {
                if n == name {
                    return the_mtime;
                }
            }
        }

        &None
    }
}

impl ClashTuiUtil {
    pub fn get_profile_names(&self) -> std::io::Result<Vec<String>> {
        Utils::get_file_names(&self.profile_dir).map(|mut v| {
            v.sort();
            v
        })
    }
    pub fn get_template_names(&self) -> std::io::Result<Vec<String>> {
        Utils::get_file_names(self.clashtui_dir.join("templates")).map(|mut v| {
            v.sort();
            v
        })
    }
    /// Wrapped `self.profile_dir.join(profile_name)`
    pub fn get_profile_path_unchecked<P: AsRef<Path>>(&self, profile_name: P) -> PathBuf {
        self.profile_dir.join(profile_name)
    }
    /// Wrapped `self.profile_dir.join(profile_name)`
    pub fn get_template_path_unchecked<P: AsRef<Path>>(&self, name: P) -> PathBuf {
        self.clashtui_dir.join("templates").join(name)
    }
    /// Check the `profiles` and `profile_cache` path
    pub fn get_profile_yaml_path<P>(&self, profile_name: P) -> std::io::Result<PathBuf>
    where
        P: AsRef<Path> + AsRef<std::ffi::OsStr>,
    {
        let mut path = self.get_profile_path_unchecked(&profile_name);
        if is_yaml(&path) {
            Some(path)
        } else {
            path = self.get_profile_cache_unchecked(profile_name);
            is_yaml(&path).then_some(path)
        }
        .ok_or(Error::new(
            std::io::ErrorKind::NotFound,
            "No valid yaml file",
        ))
    }
    fn get_profile_cache_unchecked<P>(&self, profile_name: P) -> PathBuf
    where
        P: AsRef<Path> + AsRef<std::ffi::OsStr>,
    {
        self.clashtui_dir
            .join("profile_cache")
            .join(profile_name)
            .with_extension("yaml")
    }

    pub fn get_profile_type(&self, profile_name: &str) -> Option<ProfileType> {
        let profile_path = self.get_profile_path_unchecked(profile_name);
        if is_yaml(&profile_path) {
            return Some(ProfileType::Yaml);
        }

        match self.extract_profile_url(profile_name) {
            Ok(_) => return Some(ProfileType::Url),
            Err(e) => {
                log::warn!("{}", e);
            }
        }

        None
    }

    pub fn extract_profile_url(&self, profile_name: &str) -> std::io::Result<String> {
        use std::io::BufRead;
        use regex::Regex;

        let profile_path = self.profile_dir.join(profile_name);
        let file = File::open(profile_path)?;
        let reader = std::io::BufReader::new(file);

        let url_regex = Regex::new(r#"(http|ftp|https):\/\/([\w_-]+(?:(?:\.[\w_-]+)+))([\w.,@?^=%&:\/~+#-]*[\w@?^=%&\/~+#-])"#)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Invalid regex: {}", e)))?;


        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            if !line.starts_with("#") {     // `#` is comment char
                if url_regex.is_match(&line) {
                    return Ok(line.to_string());
                }
            }
        }

        Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!("No URL found in {}", profile_name)))
    }
}
/// # Limitations
///
/// Windows treats symlink creation as a [privileged action][symlink-security],
/// therefore this function is likely to fail unless the user makes changes to
/// their system to permit symlink creation. Users can try enabling Developer
/// Mode, granting the `SeCreateSymbolicLinkPrivilege` privilege, or running
/// the process as an administrator.
///
/// [symlink-security]: https://docs.microsoft.com/en-us/windows/security/threat-protection/security-policy-settings/create-symbolic-links
#[allow(unused)]
fn crt_symlink_file<P: AsRef<std::path::Path>>(original: P, target: P) -> std::io::Result<()> {
    use std::os;
    return os::windows::fs::symlink_file(original, target);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sym() -> ClashTuiUtil {
        let exe_dir = std::env::current_dir().unwrap();
        println!("{exe_dir:?}");
        let clashtui_dir = exe_dir.parent().unwrap().join("Example");
        let (util, _) = ClashTuiUtil::new(
            &clashtui_dir.to_path_buf(),
            true
        );
        util
    }

    #[test]
    fn test_extrat_profile_net_res() {
        let sym = sym();

        let profile_name = "profile1.yaml";
        let mut profile_yaml_path = sym.profile_dir.join(profile_name);
        if sym.get_profile_type(profile_name)
            .is_some_and(|t| t == ProfileType::Url)
        {
            profile_yaml_path = sym.get_profile_cache_unchecked(profile_name);
        }
        let net_providers = sym.extract_net_providers(&profile_yaml_path, &vec![ProfileSectionType::ProxyProvider]);
    }

}
