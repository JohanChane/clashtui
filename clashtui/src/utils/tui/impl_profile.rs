use crate::utils::tui::{NetProviderMap, ProfileType};

use super::{ClashTuiUtil, Resp, ProfileItem};
use crate::utils::{is_yaml, utils as Utils};
use api::{UrlType, UrlItem, ProfileSectionType};
use std::{
    fs::{create_dir_all, File},
    io::Error,
    path::{Path, PathBuf},
};
use std::fmt;

#[derive(Debug)]
enum UserInfoError {
    RegexError(regex::Error),
    ParseError(std::num::ParseIntError),
}

impl fmt::Display for UserInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            UserInfoError::RegexError(ref err) => write!(f, "Regex error: {}", err),
            UserInfoError::ParseError(ref err) => write!(f, "Parse error: {}", err),
        }
    }
}

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
                let tpl_name_no_ext = Path::new(template_name).file_stem().unwrap_or_else(|| Path::new(template_name).as_os_str()).to_str().unwrap_or(template_name);
                new_pp.insert(
                    serde_yaml::Value::String("path".to_string()),
                    serde_yaml::Value::String(format!("proxy-providers/tpl/{}/{}.yaml", tpl_name_no_ext, the_pp_name)),
                );
                new_proxy_providers.insert(
                    serde_yaml::Value::String(the_pp_name.clone()),
                    serde_yaml::Value::Mapping(new_pp.clone()),
                );
            }
        }
        out_parsed_yaml.to_mut()["proxy-providers"] = serde_yaml::Value::Mapping(new_proxy_providers);

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

        // ## add `clashtui` section
        out_parsed_yaml.to_mut()["clashtui"] = serde_yaml::Value::Null;

        // ## write to file
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
        if self.get_profile_type(profile_name).is_some_and(|t| Self::is_profile_with_suburl(&t)) {
            let _ = remove_file(self.get_profile_cache_unchecked(profile_name));  // Not important
        }
        remove_file(self.get_profile_path_unchecked(profile_name)).map_err(|e| e.to_string())
    }

    pub fn test_profile_config(&self, path: &str, geodata_mode: bool) -> std::io::Result<String> {
        use crate::utils::ipc::exec;
        let cmd = format!(
            "{} {} -d {} -f {} -t",
            self.tui_cfg.clash_bin_path,
            if geodata_mode { "-m" } else { "" },
            self.tui_cfg.clash_cfg_dir,
            path,
        );
        exec("sh", vec!["-c", cmd.as_str()])
    }

    pub fn select_profile(&self, profile_name: &String, no_pp: bool) -> std::io::Result<()> {
        if let Err(err) = self.merge_profile(profile_name, no_pp) {
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

    fn merge_profile(&self, profile_name: &String, no_pp: bool) -> std::io::Result<()> {
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

        if no_pp {
            if let Ok(net_res) =
                self.extract_net_provider_helper(&dst_parsed_yaml, &vec![ProfileSectionType::ProxyProvider]) {
                self.no_proxy_providers(&mut dst_parsed_yaml, &net_res)?;
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
    }

    // The advantage of using this interface for updates is that you can know the reason for update failures without needing to check mihomo's logs. The downside is that it requires resolving file permission issues.
    pub fn update_profile_with_clashtui(
        &self,
        profile_name: &String,
        does_update_all: bool,
    ) -> std::io::Result<Vec<String>> {
        let profile_yaml_path;

        let mut result = Vec::new();
        let mut profile_item = self.gen_profile_item(profile_name)?;
        let with_proxy = self.check_proxy();
        if ! with_proxy {       // mihomo is stop.
            if let Some(ref mut url_item) = profile_item.url_item {
                url_item.with_proxy = false;
            }
        }
        if Self::is_profile_with_suburl(&profile_item.typ) {
            profile_yaml_path = self.get_profile_yaml_path(profile_name)?;

            // Update the file to keep up-to-date
            self.download_profile(&profile_item)?;

            let sub_url = profile_item.url_item.ok_or(Error::new(
                std::io::ErrorKind::Other,
                "No sub-url found".to_string(),))?.url;
            let url_domain = Utils::extract_domain(sub_url.as_str()).unwrap_or("No domain");
            result.push(format!("Updated: {}, {}", profile_name, url_domain));
        } else {
            profile_yaml_path = self.get_profile_yaml_path(profile_name)?;
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
                let url_domain = Utils::extract_domain(url.url.as_str()).unwrap_or("No domain");
                let url_item = UrlItem::new(UrlType::Generic, url.url.clone(), None, with_proxy);
                match self.download_url_content(
                    &Path::new(&self.tui_cfg.clash_cfg_dir).join(&path),
                    &url_item
                ) {
                    Ok(_) => result.push(format!("Updated: {}, {}", name, url_domain)),
                    Err(e) => result.push(format!("Not updated: {}, {}, {}", name, url_domain, e)),

                }
            }
        }

        Ok(result)
    }

    pub fn gen_profile_info(&self, profile_name: &String, is_cur_profile: bool) -> std::io::Result<Vec<String>> {
        use std::time::{SystemTime, UNIX_EPOCH, Duration};
        use chrono::{DateTime, Utc};

        let with_proxy = self.check_proxy();

        // ## Profile
        let mut info = vec!["## Profile".to_string()];
        let profile_type = self.get_profile_type(profile_name)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Profile type not found"))?;
        info.push(format!("ProfileType: {}", profile_type));
        if profile_type == ProfileType::SubUrl {
            let profile_url = self.extract_profile_url(profile_name)?;
            info.push(format!("Url: {}", profile_url));

            let url_item = UrlItem::new(UrlType::Generic, profile_url, None, with_proxy);
            if let Ok(rsp) = self.dl_remote_profile(&url_item) {
                info.push(format!("-   subscription-userinfo: {}",
                        self.str_human_readable_userinfo(&rsp).unwrap_or_else(|e| e.to_string())));
                for key in vec!["profile-update-interval"] {
                    info.push(format!("-   {}: {}",
                            key,
                            rsp.get_headers().get(key).unwrap_or(&"None".to_string())));
                }
            }
        }

        // ## Providers
        let profile_yaml_path = self.get_profile_yaml_path(profile_name)?;
        let yaml_content = std::fs::read_to_string(&profile_yaml_path)?;
        let parsed_yaml = match serde_yaml::from_str::<serde_yaml::Value>(&yaml_content) {
            Ok(value) => value,
            Err(err) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
        };

        let clash_cfg_dir = Path::new(&self.tui_cfg.clash_cfg_dir);
        let net_providers = self.extract_net_provider_helper(&parsed_yaml, &vec![ProfileSectionType::Profile, ProfileSectionType::ProxyProvider, ProfileSectionType::RuleProvider]);
        if let Ok(net_res) = net_providers {
            let now = std::time::SystemTime::now();
            // ### Proxy Providers
            net_res.get(&ProfileSectionType::ProxyProvider).map(|pp_net_res| {
                info.push("## Proxy Providers".to_string());
                for (name, url, path) in pp_net_res {
                    let p = clash_cfg_dir.join(path).to_path_buf();
                    let dur_str = Utils::gen_file_dur_str(&p, Some(now)).unwrap_or("None".to_string());
                    info.push(format!("-   {} ({}): {}", 
                            name, 
                            dur_str,
                            url.url));

                    let url_item = UrlItem::new(UrlType::Generic, url.url.clone(), None, with_proxy);
                    if let Ok(rsp) = self.dl_remote_profile(&url_item) {
                        info.push(format!("    -   subscription-userinfo: {}",
                                self.str_human_readable_userinfo(&rsp).unwrap_or_else(|e| e.to_string())));
                        for key in vec!["profile-update-interval"] {
                            info.push(format!("    -   {}: {}",
                                    key,
                                    rsp.get_headers().get(key).unwrap_or(&"None".to_string())));
                        }
                    }
                }

            });

            // ### Rule Providers
            net_res.get(&ProfileSectionType::RuleProvider).map(|pp_net_res| {
                info.push("## Rule Providers".to_string());
                for (name, url, path) in pp_net_res {
                    let p = clash_cfg_dir.join(path).to_path_buf();
                    let dur_str = Utils::gen_file_dur_str(&p, Some(now)).unwrap_or("None".to_string());
                    info.push(format!("-   {} ({}): {}",
                            name,
                            dur_str,
                            url.url));
                }
            });
        }

        if !is_cur_profile {
            return Ok(info);
        }

        // ## GEO Database
        info.push("## GEO Database".to_string());
        // ### github release
        let dat_url_item = UrlItem {
            typ: UrlType::Generic,
            url: "https://api.github.com/repos/MetaCubeX/meta-rules-dat/releases".to_string(),
            token: None,
            with_proxy: with_proxy,
        };
        let rule_dat_release_rsp = self.dl_remote_profile(&dat_url_item)?;
        let releases = rule_dat_release_rsp.to_json()?;
        let latest_release = releases.as_array()
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Failed to parse releases as array"))?
                .get(0)
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "No release found"))?;

        //let name = latest_release["name"].as_str().unwrap();
        //let latest_release_time = latest_release["published_at"].as_str().unwrap();

        // <name, updated_at>
        let mut update_time_strs = std::collections::HashMap::<String, String>::new();
        let mut update_times = std::collections::HashMap::<String, SystemTime>::new();
        if let Some(assets) = latest_release["assets"].as_array() {
            for asset in assets {
                let name = asset["name"].as_str().unwrap();
                let updated_at_str = asset["updated_at"].as_str().unwrap();
                update_time_strs.insert(name.to_string(), updated_at_str.to_string());

                let updated_at_utc = DateTime::parse_from_rfc3339(updated_at_str).unwrap().with_timezone(&Utc);
                let updated_at_systemtime = UNIX_EPOCH + Duration::from_secs(updated_at_utc.timestamp() as u64) + Duration::from_nanos(updated_at_utc.timestamp_subsec_nanos() as u64);
                update_times.insert(name.to_string(), updated_at_systemtime);
            }
        }

        // ### Generate GEO database info
        let clash_cfg = self.fetch_remote()?;
        let mut geofiles = Vec::<(&str, &str)>::new();
        geofiles.push(("GeoSite.dat", "geosite.dat"));      // <local_name, release_name>
        if !clash_cfg.geodata_mode {
            geofiles.push(("geoip.metadb", "geoip.metadb"));
        } else {
            geofiles.push(("geoip.dat", "geoip.dat"));
        }
        let now = std::time::SystemTime::now();
        for (local_name, release_name) in geofiles {
            let path = clash_cfg_dir.join(local_name);
            let mtime = Utils::get_mtime(path.clone()).unwrap_or(SystemTime::UNIX_EPOCH);
            let updated_at = update_times.get(release_name).unwrap_or(&mtime);
            info.push(format!("{}: {{dur: {}, update: {}}}",
                    local_name,
                    Utils::gen_file_dur_str(&path, Some(now)).unwrap_or("None".to_string()),
                    if updated_at > &mtime {
                        format!("not latest {}",
                            Utils::str_duration(updated_at.duration_since(mtime).unwrap_or_default())
                        )
                    } else {
                        format!("latest {}",
                            update_time_strs.get("geosite.dat").unwrap().clone()
                        )
                    }),
                );
        }

        // ## Clash Config
        info.push("## Clash Config".to_string());
        info.push(format!("external-controller: {}", self.clash_api.api));
        info.push(format!("proxy-address: {}", self.clash_api.proxy_addr));
        info.push(format!("mode: {}", clash_cfg.mode));
        info.push(format!("log-level: {}", clash_cfg.log_level));
        info.push(format!("tun: {}", if clash_cfg.tun.enable {clash_cfg.tun.stack.to_string()} else {"disabled".to_string()}));
        
        let mut dns_enable_str = "disable";
        if let Some(serde_yaml::Value::Mapping(dns)) = parsed_yaml.get("dns") {
            if let Some(serde_yaml::Value::Bool(enable)) = dns.get("enable") {
                if *enable {
                    dns_enable_str = "enabled";
                }
            }
        }
        info.push(format!("dns: {}", dns_enable_str));

        info.push(format!("mixed-port: {}", clash_cfg.mixed_port));
        info.push(format!("allow-lan: {}", clash_cfg.allow_lan));
        info.push(format!("bind-address: {}", clash_cfg.bind_address));
        info.push(format!("ipv6: {}", clash_cfg.ipv6));
        info.push(format!("unified-delay: {}", clash_cfg.unified_delay));
        info.push(format!("tcp-concurrent: {}", clash_cfg.tcp_concurrent));
        if clash_cfg.geo_auto_update {
            info.push(format!("geo-update-interval: {}", clash_cfg.geo_update_interval));
        } else {
            info.push(format!("geo-auto-update: {}", clash_cfg.geo_auto_update));
        }
        info.push(format!("geodata-mode: {}", clash_cfg.geodata_mode));
        info.push(format!("find-process-mode: {}", if clash_cfg.find_process_mode == "" {"strict"} else {clash_cfg.find_process_mode.as_str()}));
        info.push(format!("global-ua: {}", self.clash_api.clash_ua));
        info.push(format!("global-client-fingerprint: {}", if clash_cfg.global_client_fingerprint == "" {"chrome"} else {clash_cfg.global_client_fingerprint.as_str()}));

        Ok(info)
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

        let profile_name_path: &Path = profile_name.as_ref();
        let profile_name_str = profile_name_path.to_str().ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid profile name",
        ))?;
        let profile_type = self.get_profile_type(profile_name_str)
            .ok_or(Error::new(
                std::io::ErrorKind::NotFound,
                "profile type is invalid",
            ))?;
        if Self::is_profile_with_suburl(&profile_type) {
            path = self.get_profile_cache_unchecked(profile_name);
            Ok(path)
        } else {
            Ok(path)
        }
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
        let profile_path = self.get_profile_path_unchecked(&profile_name);

        if is_yaml(&profile_path) {
            if let Ok(parsed_yaml) = Utils::parse_yaml(profile_path.as_path()) {
                if let serde_yaml::Value::Mapping(map) = parsed_yaml {
                    if let Some(clashtui) = map.get("clashtui").and_then(|v| v.as_mapping()) {
                        if let Some(_) = clashtui.get("profile_url").and_then(|v| v.as_mapping()) {
                            return Some(ProfileType::CtPfWithSubUrl);
                        }
                        return Some(ProfileType::CtPf);
                    } else {
                        return Some(ProfileType::ClashPf)
                    }
                }
            }
        } else {
            return self.extract_profile_url(profile_name)
                .ok().and(Some(ProfileType::SubUrl));
        }

        None
    }

    pub fn is_clashtui_profile(profile_type: &ProfileType) -> bool {
        match profile_type {
            ProfileType::ClashPf => false,
            _ => true
        }
    }

    pub fn is_profile_with_suburl(profile_type: &ProfileType) -> bool {
        match profile_type {
            ProfileType::SubUrl | ProfileType::CtPfWithSubUrl => true,
            ProfileType::ClashPf | ProfileType::CtPf => false,
        }
    }

    pub fn extract_profile_url(&self, profile_name: &str) -> std::io::Result<String> {
        use std::io::BufRead;
        use regex::Regex;

        let profile_path = self.get_profile_path_unchecked(&profile_name);
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

        Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("No URL found in {}", profile_name)
            )
        )
    }

    pub fn gen_profile_item(&self, profile_name: &str) -> std::io::Result<ProfileItem> {


        let profile_path = self.get_profile_path_unchecked(profile_name);

        // ## ProfileType
        let profile_type = self.get_profile_type(profile_name).ok_or(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unknown profile type"))?;

        // ## Generate the profile_item
        let mut profile_item = ProfileItem::from_url(profile_type, profile_name.to_string());
        if profile_type == ProfileType::ClashPf {
            return Ok(profile_item);
        }

        if profile_type == ProfileType::SubUrl {
            let profile_url = Some(self.extract_profile_url(profile_name)?);
            let url_item = UrlItem::new(UrlType::Generic, profile_url.unwrap(), None, self.check_proxy());
            profile_item.url_item = Some(url_item);
        } else if profile_type == ProfileType::CtPfWithSubUrl {
            let profile_parsed_yaml = Utils::parse_yaml(profile_path.as_path())?;
            if let Some(profile) = profile_parsed_yaml.get("clashtui")
                    .and_then(|v| v.get("profile_url")) {
                let url_item = UrlItem::from_yaml(profile);
                profile_item.url_item = Some(url_item);
            }
        } else if profile_type == ProfileType::CtPf {
        }

        Ok(profile_item)
    }

    fn download_url_content(&self, path: &PathBuf, url_item: &UrlItem) -> std::io::Result<()> {
        let directory = path
            .parent()
            .ok_or_else(|| Error::new(std::io::ErrorKind::NotFound, "Invalid file path"))?;
        if !directory.exists() {
            create_dir_all(directory)?;
        }

        let response = self.dl_remote_profile(url_item)?;
        let mut output_file = File::create(path)?;      // will truncate the file
        response.copy_to(&mut output_file)?;
        Ok(())
    }

    fn download_profile(&self, profile_item: &ProfileItem) -> std::io::Result<()> {
        let path = self.get_profile_cache_unchecked(profile_item.name.clone());

        let directory = path
            .parent()
            .ok_or_else(|| Error::new(std::io::ErrorKind::NotFound, "Invalid file path"))?;
        if !directory.exists() {
            create_dir_all(directory)?;
        }

        if let Some(ref url_item) = profile_item.url_item {
            self.download_url_content(&path, url_item)?;
        }

        Ok(())
    }

    pub fn extract_net_providers(&self, profile_yaml_path: &PathBuf, provider_types: &Vec<ProfileSectionType>) -> std::io::Result<NetProviderMap> {
        let yaml_content = std::fs::read_to_string(&profile_yaml_path)?;
        let parsed_yaml = match serde_yaml::from_str::<serde_yaml::Value>(&yaml_content) {
            Ok(value) => value,
            Err(err) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
        };

        self.extract_net_provider_helper(&parsed_yaml, provider_types)
    }

    fn extract_net_provider_helper(&self, parsed_yaml: &serde_yaml::Value, provider_types: &Vec<ProfileSectionType>) -> std::io::Result<NetProviderMap> {
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

            let mut providers: Vec<(String, UrlItem, String)> = Vec::new();
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
                    let url_item = if url.as_mapping().is_some() {
                        UrlItem::from_yaml(url)
                    } else {
                        UrlItem::new(UrlType::Generic, url.as_str().unwrap().to_string(), None, true)
                    };

                    if let (serde_yaml::Value::String(name), serde_yaml::Value::String(path)) = (name, path) {
                        providers.push((name.clone(), url_item, path.clone()));
                    } else if let (serde_yaml::Value::String(name), serde_yaml::Value::String(path)) = (name, path) {
                        providers.push((name.clone(), url_item, path.clone()));
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

    // Check if need to correct perms of files in clash_cfg_dir. If perm is incorrect return false.
    pub fn check_perms_of_ccd_files(&self) -> bool {
        use std::os::unix::fs::{PermissionsExt, MetadataExt};

        let dir = Path::new(self.tui_cfg.clash_cfg_dir.as_str());
        //let group_name = Utils::get_file_group_name(&dir.to_path_buf());
        //if group_name.is_none() {
        //    return false;
        //}

        // check set-group-id
        if let Ok(metadata) = std::fs::metadata(dir) {
            let permissions = metadata.permissions();
            if permissions.mode() & 0o2000 == 0 {
                return false;
            }
        }

        if let Ok(metadata) = std::fs::metadata(dir) {
            if let Some(dir_group) = 
                nix::unistd::Group::from_gid(nix::unistd::Gid::from_raw(metadata.gid())).unwrap()
            {
                if  Utils::find_files_not_in_group(&dir.to_path_buf(), dir_group.name.as_str()).len() > 0
                    || Utils::find_files_not_group_writable(&dir.to_path_buf()).len() > 0
                    {
                        return false;
                    }
            }
        }

        return true;
    }

    /*** Update profile with api
    // format: {type, [(name, result)]}
    pub type UpdateProviderType = std::collections::HashMap<ProfileSectionType, Vec<(String, std::io::Result<String>)>>;

    // Using api update, the user needs to check the logs to understand why the updates failed. The success rate of my testing updates is not as high as using clashtui.
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

    // For Showing provider duration (now - mtime) in `update_profile_with_api`.
    // format: {type: [(name, modifytime)]}
    pub type ProfileTimeMap = std::collections::HashMap<ProfileSectionType, Vec<(String, Option<std::time::SystemTime>)>>;
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
    ***/

    fn str_human_readable_userinfo(&self, rsp: &Resp) -> Result<String, UserInfoError> {
        use regex::Regex;

        let sub_userinfo = rsp.get_headers().get("subscription-userinfo").map(|v| v.as_str());
        if let Some(info) = sub_userinfo {
            let re = Regex::new(r"(\w+)=(\d+)").map_err(|e| UserInfoError::RegexError(e))?;
            let mut res = String::new();
            for cap in re.captures_iter(info) {
                let key = &cap[1];
                let value: u64 = cap[2].parse().map_err(|e| UserInfoError::ParseError(e))?;
                if key != "expire" {
                    res.push_str(&format!("{}: {}; ", key, Utils::bytes_to_readable(value)));
                } else {
                    res.push_str(&format!("{}: {}; ", key, Utils::timestamp_to_readable(value)));
                }
            }
            Ok(res)
        } else {
            Ok("None".to_string())
        }
    }

    fn no_proxy_providers(&self, parsed_yaml: &mut serde_yaml::Value, net_res: &NetProviderMap) -> std::io::Result<()> {
        use std::collections::{HashMap, HashSet};
        use serde_yaml::Value;

        // ## Load the proxy providers
        let mut proxy_map = HashMap::<String, Vec<Value>>::new(); // <proxy_provider_name, proxies>
        let clash_cfg_dir = Path::new(&self.tui_cfg.clash_cfg_dir);
        if let Some(proxy_providers) = net_res.get(&ProfileSectionType::ProxyProvider) {
            for (name, _, path) in proxy_providers {
                let tmp_yaml_content = std::fs::read_to_string(&clash_cfg_dir.join(path))?;
                let tmp_parsed_yaml = match serde_yaml::from_str::<serde_yaml::Value>(&tmp_yaml_content) {
                    Ok(value) => value,
                    Err(err) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
                };
                
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
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to parse `proxy-groups`"));
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
                            new_proxies.extend(names.iter().map(|s| Value::String(s.clone())).collect::<Vec<_>>());
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
                .flat_map(|(_, p_seq)| serde_yaml::to_value(p_seq).unwrap().as_sequence().unwrap().clone())
                .collect();

            if let Some(Value::Sequence(ref mut proxies)) = dst_yaml.get_mut("proxies") {
                proxies.append(&mut new_proxies);
            } else {
                dst_yaml.insert("proxies".into(), Value::Sequence(new_proxies));
            }
        }

        Ok(())
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
    os::unix::fs::symlink(original, target)
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
            .is_some_and(|t| ClashTuiUtil::is_profile_with_suburl(&t))
        {
            profile_yaml_path = sym.get_profile_cache_unchecked(profile_name);
        }
        let _ = sym.extract_net_providers(&profile_yaml_path, &vec![ProfileSectionType::ProxyProvider]);
    }

}
