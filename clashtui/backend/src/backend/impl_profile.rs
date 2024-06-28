use super::ClashBackend;
use crate::utils::{extract_domain, get_file_names, ipc, is_yaml, parse_yaml};
use std::{
    fs::{create_dir_all, File},
    io::Error,
    path::{Path, PathBuf},
};
enum Merge {
    /// the target mihomo config file path
    Target(String),
    /// the config file storing `Merge::Target`
    Config(String),
    /// the profile need to be merged
    Profile(String),
}

impl ClashBackend {
    pub fn crt_yaml_with_template(&self, template_name: &String) -> Result<(), String> {
        use std::borrow::Cow;
        use std::collections::HashMap;
        use std::io::{BufRead, BufReader};
        let template_dir = self.home_dir.join("templates");
        let template_path = template_dir.join(template_name);
        let tpl_parsed_yaml =
            parse_yaml(&template_path).map_err(|e| format!("parse failed: {e:?}"))?;
        let mut out_parsed_yaml = Cow::Borrowed(&tpl_parsed_yaml);

        let proxy_url_file = File::open(self.home_dir.join("templates/template_proxy_providers"))
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
        let mut new_proxy_providers = serde_yaml::Mapping::new();
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
                let tpl_name_no_ext = Path::new(template_name)
                    .file_stem()
                    .unwrap_or_else(|| Path::new(template_name).as_os_str())
                    .to_str()
                    .unwrap_or(template_name);
                new_pp.insert(
                    serde_yaml::Value::String("path".to_string()),
                    serde_yaml::Value::String(format!(
                        "proxy-providers/tpl/{}/{}.yaml",
                        tpl_name_no_ext, the_pp_name
                    )),
                );
                new_proxy_providers.insert(
                    serde_yaml::Value::String(the_pp_name.clone()),
                    serde_yaml::Value::Mapping(new_pp.clone()),
                );
            }
        }
        out_parsed_yaml.to_mut()["proxy-providers"] =
            serde_yaml::Value::Mapping(new_proxy_providers);

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

        let out_yaml_path = self.gen_profile_path(template_name);
        let out_yaml_file = File::create(out_yaml_path).map_err(|e| e.to_string())?;
        serde_yaml::to_writer(out_yaml_file, &out_parsed_yaml).map_err(|e| e.to_string())?;
        use crate::utils::config::ProfileType;
        self.cfg.profiles.insert(
            template_name,
            ProfileType::Generated(
                out_parsed_yaml
                    .as_str()
                    .expect("Err:pathbuf as str")
                    .to_string(),
            ),
        );

        Ok(())
    }

    pub fn crt_profile(&self, profile_name: String, uri: String) -> Result<(), String> {
        use crate::utils::config::ProfileType;
        let profile_name = profile_name.trim();
        let uri = uri.trim();

        if uri.is_empty() || profile_name.is_empty() {
            return Err("Url or Name is empty!".to_string());
        }

        if uri.starts_with("http://") || uri.starts_with("https://") {
            match self
                .cfg
                .profiles
                .insert(profile_name, ProfileType::Url(uri.to_string()))
            {
                Some(_) => Err(format!("Name `{profile_name}` already in use")),
                None => Ok(()),
            }
        } else if Path::new(uri).is_file() {
            let uri_path = self.gen_profile_path(profile_name);
            if uri_path.exists() {
                return Err(format!("Failed to import: file `{profile_name}` exists"));
            }
            std::fs::copy(uri, uri_path).map_err(|e| e.to_string())?;
            self.cfg.profiles.insert(profile_name, ProfileType::File);
            Ok(())
        } else {
            Err("Url is invalid.".to_string())
        }
    }

    /// remove profile and check if current profile is removed
    ///
    /// need to manually refresh the state
    pub fn rmf_profile(&self, profile_name: &String) -> Result<(), String> {
        use std::fs::remove_file;
        match self.cfg.profiles.remove(profile_name) {
            Some(_) => {
                if self.cfg.current_profile.borrow().eq(profile_name) {
                    self.cfg.update_profile("Removed");
                };
                remove_file(self.gen_profile_path(profile_name)).map_err(|e| e.to_string())
            }
            None => Err("No such key".to_string()),
        }
    }

    pub fn test_profile_config(&self, path: &str, geodata_mode: bool) -> std::io::Result<String> {
        let cmd = format!(
            "{} {} -d {} -f {} -t",
            self.cfg.clash_bin_pth,
            if geodata_mode { "-m" } else { "" },
            self.cfg.clash_cfg_dir,
            path,
        );
        #[cfg(target_os = "windows")]
        return ipc::exec("cmd", vec!["/C", cmd.as_str()]);
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        ipc::exec("sh", vec!["-c", cmd.as_str()])
    }

    pub fn select_profile(&self, profile_name: &String) -> std::io::Result<()> {
        if let Err(err) = self.merge_profile(profile_name) {
            let emsg = format!(
                "Failed to Merge Profile `{profile_name}` due to {}",
                match err {
                    Merge::Target(e) => format!("Mihomo Config file: {e}"),
                    Merge::Config(e) => format!("Program Config file: {e}"),
                    Merge::Profile(e) => format!("Profile: {e}"),
                }
            );
            log::error!("{emsg:?}");
            return Err(Error::new(std::io::ErrorKind::Other, emsg));
        };
        if let Err(err) = self.config_reload(api::build_payload(&self.cfg.clash_cfg_pth)) {
            let emsg = format!("Failed to Patch Profile `{profile_name}` due to {}", err);
            log::error!("{emsg:?}");
            return Err(Error::new(std::io::ErrorKind::Other, emsg));
        };
        Ok(())
    }

    fn merge_profile(&self, profile_name: &String) -> Result<(), Merge> {
        let mut dst_parsed_yaml = parse_yaml(&self.home_dir.join(crate::consts::BASIC_FILE))
            .map_err(|e| Merge::Config(e.to_string()))?;
        let profile_parsed_yaml = self
            .get_profile_yaml(profile_name)
            .map_err(|e| Merge::Profile(format!("{e}. Maybe need to update first.")))
            .map(|p| {
                parse_yaml(&p).expect(
                    "get_profile_yaml mark it as valid yaml file, call parse should be safe",
                )
            })?;
        use serde_yaml::Value::Mapping;
        if let (Mapping(dst_mapping), Mapping(mapping)) =
            (&mut dst_parsed_yaml, &profile_parsed_yaml)
        {
            let _filter = [
                "proxy-groups",
                "proxy-providers",
                "proxies",
                "sub-rules",
                "rules",
                "rule-providers",
            ];
            mapping
                .iter()
                .map_while(|(k, v)| k.as_str().map(|_| (k, v)))
                .filter(|(k, _)| _filter.contains(&k.as_str().unwrap()))
                .for_each(|(k, v)| {
                    dst_mapping.insert(k.clone(), v.clone());
                });
        }
        match try_create_file(&self.cfg.clash_cfg_pth).map_err(Merge::Target)? {
            CrtFile::Ok(f) => {
                serde_yaml::to_writer(f, &dst_parsed_yaml).map_err(|e| Merge::Target(e.to_string()))
            }
            CrtFile::Tmp(f) => {
                serde_yaml::to_writer(f, &dst_parsed_yaml)
                    .map_err(|e| Merge::Target(e.to_string()))?;
                #[cfg(target_os = "linux")]
                ipc::exec_with_sbin("mv", vec![TMP_PATH, &self.cfg.clash_cfg_pth])
                    .map_err(|e| Merge::Target(e.to_string()))?;
                #[cfg(target_os = "windows")]
                todo!();
                Ok(())
            }
        }
    }

    pub fn trim_proxy_providers(&self) -> anyhow::Result<()>{
        let current_config = std::fs::File::open(&self.cfg.clash_cfg_pth)?;
        let mut parsed_yaml:serde_yaml::Value = serde_yaml::from_reader(current_config)?;
        let net_res = extract_net_provider_helper(&parsed_yaml, &vec![ProfileSectionType::ProxyProvider])?;
        Ok(no_proxy_providers(&self.cfg.clash_cfg_dir, &mut parsed_yaml, &net_res)?)
    }

    pub fn update_profile(
        &self,
        profile_name: &String,
        does_update_all: bool,
    ) -> std::io::Result<Vec<String>> {
        let profile_yaml_path = self.gen_profile_path(profile_name);
        let mut net_res: Vec<(String, String)> = Vec::new();
        // if it's just the link
        if self.is_upgradable(profile_name) {
            let sub_url = self
                .get_profile_link(profile_name)
                .expect("have a key but no value")
                .to_owned();

            // Update the file to keep up-to-date
            self.download_profile(&sub_url, &profile_yaml_path)?;

            net_res.push((sub_url, profile_yaml_path.to_string_lossy().to_string()))
        }

        // Update the resouce in the file (if there is)
        {
            let parsed_yaml: serde_yaml::Value =
                serde_yaml::from_reader(File::open(profile_yaml_path)?)
                    .map_err(|e| Error::new(std::io::ErrorKind::InvalidData, e))?;
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
                let url_domain = extract_domain(url.as_str()).unwrap_or("No domain");
                match self.download_profile(&url, &Path::new(&self.cfg.clash_cfg_dir).join(path)) {
                    Ok(_) => format!("Updated: {profile_name}({url_domain})"),
                    Err(err) => {
                        log::error!("Update profile:{err}");
                        format!("Not Updated: {profile_name}({url_domain})")
                    }
                }
            })
            .collect::<Vec<String>>())
    }

    fn download_profile(&self, url: &str, path: &PathBuf) -> std::io::Result<()> {
        let directory = path
            .parent()
            .ok_or_else(|| Error::new(std::io::ErrorKind::NotFound, "Invalid file path"))?;
        if !directory.exists() {
            create_dir_all(directory)?;
        }

        let response = self
            .dl_remote_profile(
                url,
                std::env::var(crate::consts::PROXY_ENVAR)
                    .map(|s| s.parse::<bool>().unwrap_or(false))
                    .unwrap_or(false),
            )
            .map_err(|s| Error::new(std::io::ErrorKind::Other, s))?;
        let mut output_file = File::create(path)?;
        response.copy_to(&mut output_file)?;
        Ok(())
    }
}

impl ClashBackend {
    pub fn get_profile_names(&self) -> std::io::Result<Vec<String>> {
        let mut l: Vec<String> = self.cfg.profiles.all();
        l.sort();
        Ok(l)
    }
    /// if that is import via link, return `Some`
    /// else return `None`
    pub fn get_profile_link<P: AsRef<str>>(&self, profile_name: P) -> Option<String> {
        self.cfg
            .profiles
            .get(profile_name.as_ref())
            .and_then(|p| p.into_inner())
    }
    pub fn get_template_names(&self) -> std::io::Result<Vec<String>> {
        get_file_names(self.home_dir.join("templates")).map(|mut v| {
            v.sort();
            v
        })
    }
    pub fn is_upgradable<P: AsRef<str>>(&self, profile_name: P) -> bool {
        self.cfg
            .profiles
            .get(profile_name.as_ref())
            .is_some_and(|v| !v.is_null())
    }
    /// Wrapped `self.profile_dir.join(profile_name)`
    pub fn gen_profile_path<P: AsRef<Path>>(&self, profile_name: P) -> PathBuf {
        self.home_dir.join("profiles").join(profile_name)
    }
    /// Wrapped `self.profile_dir.join(profile_name)`
    pub fn gen_template_path<P: AsRef<Path>>(&self, name: P) -> PathBuf {
        self.home_dir.join("templates").join(name)
    }
    /// Make sure that's a valid yaml file
    pub fn get_profile_yaml<P>(&self, profile_name: P) -> std::io::Result<PathBuf>
    where
        P: AsRef<Path> + AsRef<std::ffi::OsStr>,
    {
        let path = self.gen_profile_path(&profile_name);
        if is_yaml(&path) {
            Ok(path)
        } else {
            Err(Error::new(
                std::io::ErrorKind::NotFound,
                "No valid yaml file",
            ))
        }
    }
}
enum CrtFile {
    Ok(File),
    Tmp(File),
}
const TMP_PATH: &str = "/tmp/clashtui_mihomo_config_file.tmp";
fn try_create_file<P: AsRef<Path>>(path: P) -> Result<CrtFile, String> {
    match File::create(path) {
        Ok(f) => Ok(CrtFile::Ok(f)),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                Ok(CrtFile::Tmp(
                    File::create(TMP_PATH).map_err(|e| e.to_string())?,
                ))
            } else {
                Err(format!("Unexpected Error: {e}"))
            }
        }
    }
}
type NetProviderMap = std::collections::HashMap<ProfileSectionType, Vec<(String, String, String)>>;
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum ProfileSectionType {
    ProxyProvider,
    RuleProvider,
}
fn extract_net_provider_helper(
    parsed_yaml: &serde_yaml::Value,
    provider_types: &Vec<ProfileSectionType>,
) -> std::io::Result<NetProviderMap> {
    let mut net_providers = NetProviderMap::new();
    for provider_type in provider_types {
        let provider_str = match provider_type {
            ProfileSectionType::ProxyProvider => "proxy-providers",
            ProfileSectionType::RuleProvider => "rule-providers",
        };
        let providers: Vec<(String, String, String)> = parsed_yaml
            .get(provider_str)
            .and_then(|m| m.as_mapping())
            .into_iter()
            .flat_map(|m| m.into_iter())
            .filter_map(|(name, cont)| cont.as_mapping().and_then(|m| Some((name, m))))
            .filter_map(|(name, cont)| {
                if let (Some(name), Some(url), Some(path)) = (
                    name.as_str(),
                    cont.get("url").and_then(|s| s.as_str()),
                    cont.get("path").and_then(|s| s.as_str()),
                ) {
                    Some((name.to_owned(), url.to_owned(), path.to_owned()))
                } else {
                    None
                }
            })
            .collect();
        net_providers.insert(*provider_type, providers);
    }
    Ok(net_providers)
}

fn no_proxy_providers<P: AsRef<Path>>(
    clash_home_dir: P,
    parsed_yaml: &mut serde_yaml::Value,
    net_res: &NetProviderMap,
) -> std::io::Result<()> {
    use serde_yaml::Value;
    use std::collections::{HashMap, HashSet};

    // ## Load the proxy providers
    let mut proxy_map = HashMap::<String, Vec<Value>>::new(); // <proxy_provider_name, proxies>
    if let Some(proxy_providers) = net_res.get(&ProfileSectionType::ProxyProvider) {
        for (name, _, path) in proxy_providers {
            let tmp_yaml_fp = std::fs::File::open(clash_home_dir.as_ref().join(path))?;
            let tmp_parsed_yaml: serde_yaml::Value = serde_yaml::from_reader(&tmp_yaml_fp)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

            if let Some(Value::Sequence(proxies)) = tmp_parsed_yaml.get("proxies") {
                proxy_map.insert(name.clone(), proxies.clone());
            }
        }
    }

    // ## Rename the same proxy name between the proxy_providers
    let mut proxy_names = HashMap::<String, Vec<String>>::new();
    let mut unique_proxy_names = HashSet::<String>::new();
    for (pp_name, proxies) in proxy_map.iter_mut() {
        proxy_names.insert(pp_name.to_string(), Vec::new());
        for proxy in proxies {
            if let Some(names) = proxy_names.get_mut(pp_name) {
                if let Some(Value::String(name)) = proxy.get_mut("name") {
                    if unique_proxy_names.contains(name) {
                        *name = format!("{}{}", pp_name, name);
                    }
                    names.push(name.clone());
                    unique_proxy_names.insert(name.clone());
                }
            }
        }
    }
    std::mem::drop(unique_proxy_names);

    // ## Repace the proxy-providers in proxy-groups
    let proxy_groups = if let Some(serde_yaml::Value::Sequence(proxy_groups)) =
        parsed_yaml.get_mut("proxy-groups")
    {
        proxy_groups
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Failed to parse `proxy-groups`",
        ));
    };

    for the_pg_mapping in proxy_groups {
        if let Value::Mapping(the_pg) = the_pg_mapping {
            let mut new_proxies = Vec::<Value>::new();

            if let Some(Value::Sequence(proxies)) = the_pg.get("proxies") {
                for p in proxies {
                    new_proxies.push(p.clone());
                }
            }
            if let Some(Value::Sequence(providers)) = the_pg.get("use") {
                for p in providers {
                    if let Some(names) = proxy_names.get(p.as_str().unwrap()) {
                        new_proxies.extend(
                            names
                                .iter()
                                .map(|s| Value::String(s.clone()))
                                .collect::<Vec<_>>(),
                        );
                    }
                }
            }

            if new_proxies.is_empty() {
                new_proxies.push(Value::String("COMPATIBLE".into()));
            }
            the_pg.insert("proxies".into(), Value::Sequence(new_proxies));
            the_pg.remove("use");
        }
    }

    if let Value::Mapping(ref mut dst_yaml) = parsed_yaml {
        // ## drop proxy-providers
        dst_yaml.remove("proxy-providers");

        // ## Add the `proxies` to the yaml
        let mut new_proxies = proxy_map
            .into_iter()
            .flat_map(|(_, p_seq)| {
                serde_yaml::to_value(p_seq)
                    .unwrap()
                    .as_sequence()
                    .unwrap()
                    .clone()
            })
            .collect();

        if let Some(Value::Sequence(ref mut proxies)) = dst_yaml.get_mut("proxies") {
            proxies.append(&mut new_proxies);
        } else {
            dst_yaml.insert("proxies".into(), Value::Sequence(new_proxies));
        }
    }

    Ok(())
}
