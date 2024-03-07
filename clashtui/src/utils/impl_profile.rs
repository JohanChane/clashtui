use super::{is_yaml, parse_yaml, utils as Utils, ClashTuiUtil};
use std::{
    fs::{create_dir_all, File},
    io::Error,
    path::{Path, PathBuf},
};

impl ClashTuiUtil {
    pub fn create_yaml_with_template(&self, template_name: &String) -> Result<(), String> {
        use std::borrow::Cow;
        use std::collections::HashMap;
        use std::io::{BufRead, BufReader};
        let template_dir = self.clashtui_dir.join("templates");
        let template_path = template_dir.join(template_name);
        let tpl_parsed_yaml = parse_yaml(&template_path).map_err(|e| e.to_string())?;
        let mut out_parsed_yaml = Cow::Borrowed(&tpl_parsed_yaml);

        let proxy_url_file =
            File::open(self.clashtui_dir.join("templates/template_proxy_providers"))
                .map_err(|e| e.to_string())?;
        let mut proxy_urls = Vec::new();
        BufReader::new(proxy_url_file)
            .lines()
            .map_while(Result::ok)
            .filter(|v| {
                let val = v.trim();
                !(val.is_empty() || val.starts_with('#'))
            })
            .for_each(|v| proxy_urls.push(v));

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

            let pp = if let serde_yaml::Value::Mapping(pp) = pp_value {
                pp
            } else {
                return Err(String::from("Failed to parse `proxy-providers` value"));
            };

            let mut new_pp = pp.clone();
            new_pp.remove("tpl_param");

            for (i, url) in proxy_urls.iter().enumerate() {
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

        log::error!("testssdfs");
        let out_yaml_path = self.profile_dir.join(template_name);
        let out_yaml_file = File::create(out_yaml_path).map_err(|e| e.to_string())?;
        serde_yaml::to_writer(out_yaml_file, &out_parsed_yaml).map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn test_profile_config(&self, path: &str, geodata_mode: bool) -> Result<String, Error> {
        use super::ipc::exec;
        let cmd = format!(
            "{} {} -d {} -f {} -t",
            self.tui_cfg.clash_core_path,
            if geodata_mode { "-m" } else { "" },
            self.tui_cfg.clash_cfg_dir,
            path,
        );
        #[cfg(target_os = "windows")]
        return exec("cmd", vec!["/C", cmd.as_str()]);
        #[cfg(target_os = "linux")]
        exec("sh", vec!["-c", cmd.as_str()])
    }

    pub fn select_profile(&self, profile_name: &String) -> Result<(), Error> {
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
        let basic_clash_cfg_path = self.clashtui_dir.join(super::tui::BASIC_FILE);
        let mut dst_parsed_yaml = parse_yaml(&basic_clash_cfg_path)?;
        let profile_yaml_path = self.get_profile_yaml_path(profile_name)?;
        let profile_parsed_yaml = parse_yaml(&profile_yaml_path).map_err(|e| {
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

    pub fn update_local_profile(
        &self,
        profile_name: &String,
        does_update_all: bool,
    ) -> Result<Vec<String>, Error> {
        let mut profile_yaml_path = self.profile_dir.join(profile_name);
        let mut net_res: Vec<(String, String)> = Vec::new();
        // if it's just the link
        if !self.is_profile_yaml(profile_name) {
            let file_content = std::io::read_to_string(File::open(profile_yaml_path)?)?;
            let sub_url = file_content.trim();

            profile_yaml_path = self.get_profile_cache_unchecked(profile_name);
            // Update the file to keep up-to-date
            self.download_file(sub_url, &profile_yaml_path)?;

            net_res.push((
                sub_url.to_string(),
                profile_yaml_path.to_string_lossy().to_string(),
            ))
        }

        // Update the resouce in the file (if there is)
        {
            let yaml_content = std::io::read_to_string(File::open(profile_yaml_path)?)?;
            let parsed_yaml = serde_yaml::Value::from(yaml_content.as_str());
            drop(yaml_content);
            net_res.extend(
                if !does_update_all {
                    vec!["proxy-providers"]
                } else {
                    vec!["proxy-providers", "rule-providers"]
                }
                .into_iter()
                .filter_map(|key| parsed_yaml.get(key))
                .filter_map(|val| val.as_mapping())
                // flatten inner iter
                .flat_map(|providers| {
                    providers
                        .into_iter()
                        .filter_map(|(_, provider_value)| provider_value.as_mapping())
                        // pass only when type is http
                        .filter(|&provider_content| {
                            provider_content
                                .get("type")
                                .and_then(|v| v.as_str())
                                .is_some_and(|t| t == "http")
                        })
                        .filter_map(|provider_content| {
                            if let (
                                Some(serde_yaml::Value::String(url)),
                                Some(serde_yaml::Value::String(path)),
                            ) = (provider_content.get("url"), provider_content.get("path"))
                            {
                                Some((url.clone(), path.clone()))
                            } else {
                                None
                            }
                        })
                }),
            );
        }

        Ok(net_res
            .into_iter()
            .map(|(url, path)| {
                match self.download_file(&url, &Path::new(&self.tui_cfg.clash_cfg_dir).join(path)) {
                    Ok(_) => format!("Updated: {url}"),
                    Err(err) => {
                        log::error!("Update profile:{err}");
                        format!("Not Updated: {url}")
                    }
                }
            })
            .collect::<Vec<String>>())
    }

    fn download_file(&self, url: &str, path: &PathBuf) -> Result<(), Error> {
        let response = self.dl_remote_profile(url)?;

        let directory = path
            .parent()
            .ok_or_else(|| Error::new(std::io::ErrorKind::NotFound, "Invalid file path"))?;
        if !directory.exists() {
            create_dir_all(directory)?;
        }

        let mut output_file = File::create(path)?;
        response.copy_to(&mut output_file)?;
        Ok(())
    }
}

impl ClashTuiUtil {
    pub fn get_profile_names(&self) -> Result<Vec<String>, Error> {
        Utils::get_file_names(&self.profile_dir).map(|mut v| {
            v.sort();
            v
        })
    }
    pub fn get_template_names(&self) -> Result<Vec<String>, Error> {
        Utils::get_file_names(self.clashtui_dir.join("templates")).map(|mut v| {
            v.sort();
            v
        })
    }
    /// Wrapped `self.profile_dir.join(profile_name)`
    pub fn get_profile_path_unchecked<P>(&self, profile_name: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.profile_dir.join(profile_name)
    }
    /// Wrapped `self.profile_dir.join(profile_name)`
    pub fn get_template_path_unchecked<P>(&self, name: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.clashtui_dir.join("templates").join(name)
    }
    /// Check the `profiles` and `profile_cache` path
    pub fn get_profile_yaml_path<P>(&self, profile_name: P) -> Result<PathBuf, Error>
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
    /// Check file only in `profiles`
    ///
    /// Judging by format
    pub fn is_profile_yaml<P>(&self, profile_name: P) -> bool
    where
        P: AsRef<Path>,
    {
        let profile_path = self.get_profile_path_unchecked(profile_name);
        is_yaml(&profile_path)
    }
}
