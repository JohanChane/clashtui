use serde_yml::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceSection {
    ProxyProvider,
    RuleProvider,
}

impl std::fmt::Display for ResourceSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceSection::ProxyProvider => write!(f, "proxy-provider"),
            ResourceSection::RuleProvider => write!(f, "rule-provider"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetResource {
    pub name: String,
    pub url: String,
    pub path: String,
    pub section: ResourceSection,
}

#[derive(Clone, Debug)]
pub struct NetResourceUpdate {
    pub name: String,
    pub url: String,
    pub path: String,
    pub section: ResourceSection,
    pub ok: bool,
    pub error: Option<String>,
}

pub fn format_net_updates(updates: &[NetResourceUpdate]) -> String {
    updates
        .iter()
        .map(|u| {
            let domain = extract_domain(&u.url).unwrap_or(&u.url);
            if u.ok {
                format!("  {} {} {}: ok", u.section, u.name, domain)
            } else {
                format!(
                    "  {} {} {}: FAILED — {}",
                    u.section,
                    u.name,
                    domain,
                    u.error.as_deref().unwrap_or("unknown")
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_domain(url: &str) -> Option<&str> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    rest.split('/').next()
}

pub trait ExtractNetResources {
    fn extract(&self, sections: &[ResourceSection]) -> Vec<NetResource>;
}

impl ExtractNetResources for serde_yml::Mapping {
    fn extract(&self, sections: &[ResourceSection]) -> Vec<NetResource> {
        let mut resources = Vec::new();

        for section in sections {
            let key = match section {
                ResourceSection::ProxyProvider => "proxy-providers",
                ResourceSection::RuleProvider => "rule-providers",
            };

            let section_val = match self.get(&Value::String(key.to_string())) {
                Some(Value::Mapping(map)) => map,
                _ => continue,
            };

            for (provider_key, provider_val) in section_val {
                let provider_map = match provider_val.as_mapping() {
                    Some(m) => m,
                    None => continue,
                };

                let name = match provider_key.as_str() {
                    Some(s) => s.to_owned(),
                    None => continue,
                };

                let url = match provider_map
                    .get(&Value::String("url".to_string()))
                    .and_then(|v| v.as_str())
                {
                    Some(s) => s.to_owned(),
                    None => continue,
                };

                let path = match provider_map
                    .get(&Value::String("path".to_string()))
                    .and_then(|v| v.as_str())
                {
                    Some(s) => s.to_owned(),
                    None => continue,
                };

                resources.push(NetResource {
                    name,
                    url,
                    path,
                    section: section.clone(),
                });
            }
        }

        resources
    }
}

pub fn extract_singbox_net_resources(content: &serde_json::Value) -> Vec<NetResource> {
    let mut resources = Vec::new();

    if let serde_json::Value::Array(outbounds) = &content["outbounds"] {
        for outbound in outbounds {
            let name = outbound["tag"].as_str().unwrap_or("unnamed");
            if let Some(url) = outbound["outbound"]["url"].as_str() {
                if let Some(path) = outbound["outbound"]["path"].as_str() {
                    resources.push(NetResource {
                        name: name.to_owned(),
                        url: url.to_owned(),
                        path: path.to_owned(),
                        section: ResourceSection::ProxyProvider,
                    });
                }
            }
        }
    }

    if let serde_json::Value::Array(rule_sets) = &content["route"]["rule_set"] {
        for rule_set in rule_sets {
            let tag = rule_set["tag"].as_str().unwrap_or("unnamed");
            let is_remote = rule_set["type"].as_str() == Some("remote");
            if is_remote {
                if let Some(url) = rule_set["url"].as_str() {
                    let path = rule_set["path"].as_str().unwrap_or("rules.db");
                    resources.push(NetResource {
                        name: tag.to_owned(),
                        url: url.to_owned(),
                        path: path.to_owned(),
                        section: ResourceSection::RuleProvider,
                    });
                }
            }
        }
    }

    resources
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    fn load_test_yaml() -> serde_yml::Mapping {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/functions/file/testdata/net_resource_test.yaml"
        );
        serde_yml::from_reader(File::open(path).unwrap()).unwrap()
    }

    #[test]
    fn extract_all_sections() {
        let yaml = load_test_yaml();
        let resources =
            yaml.extract(&[ResourceSection::ProxyProvider, ResourceSection::RuleProvider]);
        assert_eq!(resources.len(), 4, "should find 2 PP + 2 RP = 4 resources");

        let pp_count = resources
            .iter()
            .filter(|r| r.section == ResourceSection::ProxyProvider)
            .count();
        assert_eq!(pp_count, 2);

        let rp_count = resources
            .iter()
            .filter(|r| r.section == ResourceSection::RuleProvider)
            .count();
        assert_eq!(rp_count, 2);
    }

    #[test]
    fn filter_proxy_providers_only() {
        let yaml = load_test_yaml();
        let resources = yaml.extract(&[ResourceSection::ProxyProvider]);
        assert_eq!(resources.len(), 2);
        for r in &resources {
            assert_eq!(r.section, ResourceSection::ProxyProvider);
        }
    }

    #[test]
    fn filter_rule_providers_only() {
        let yaml = load_test_yaml();
        let resources = yaml.extract(&[ResourceSection::RuleProvider]);
        assert_eq!(resources.len(), 2);
        for r in &resources {
            assert_eq!(r.section, ResourceSection::RuleProvider);
        }
    }

    #[test]
    fn filter_both_sections() {
        let yaml = load_test_yaml();
        let resources =
            yaml.extract(&[ResourceSection::ProxyProvider, ResourceSection::RuleProvider]);
        assert_eq!(resources.len(), 4);
    }

    #[test]
    fn empty_filter() {
        let yaml = load_test_yaml();
        let resources = yaml.extract(&[]);
        assert!(resources.is_empty());
    }

    #[test]
    fn verify_extracted_fields() {
        let yaml = load_test_yaml();
        let resources =
            yaml.extract(&[ResourceSection::ProxyProvider, ResourceSection::RuleProvider]);

        let pp_dcdn = resources
            .iter()
            .find(|r| r.name == "pp-dcdn")
            .expect("pp-dcdn should exist");
        assert_eq!(pp_dcdn.url, "https://cdn.example.com/dcdn.yaml");
        assert_eq!(pp_dcdn.path, "./proxy-providers/dcdn.yaml");
        assert_eq!(pp_dcdn.section, ResourceSection::ProxyProvider);

        let pp_aws = resources
            .iter()
            .find(|r| r.name == "pp-aws")
            .expect("pp-aws should exist");
        assert_eq!(
            pp_aws.url,
            "https://s3.amazonaws.com/bucket/proxies.yaml"
        );
        assert_eq!(pp_aws.path, "./proxy-providers/aws.yaml");
        assert_eq!(pp_aws.section, ResourceSection::ProxyProvider);

        let rp_reject = resources
            .iter()
            .find(|r| r.name == "rp-reject")
            .expect("rp-reject should exist");
        assert_eq!(rp_reject.url, "https://rules.example.org/reject.yaml");
        assert_eq!(rp_reject.path, "./rule-providers/reject.yaml");
        assert_eq!(rp_reject.section, ResourceSection::RuleProvider);

        let rp_ads = resources
            .iter()
            .find(|r| r.name == "rp-ads")
            .expect("rp-ads should exist");
        assert_eq!(rp_ads.url, "https://filters.example.net/ads.yaml");
        assert_eq!(rp_ads.path, "./rule-providers/ads.yaml");
        assert_eq!(rp_ads.section, ResourceSection::RuleProvider);
    }

    #[test]
    fn no_provider_sections() {
        let mut yaml = serde_yml::Mapping::new();
        yaml.insert(
            Value::String("proxies".to_string()),
            Value::Sequence(vec![]),
        );
        let resources =
            yaml.extract(&[ResourceSection::ProxyProvider, ResourceSection::RuleProvider]);
        assert!(resources.is_empty());
    }

    #[test]
    fn provider_section_is_scalar() {
        let mut yaml = serde_yml::Mapping::new();
        yaml.insert(
            Value::String("proxy-providers".to_string()),
            Value::String("not-a-mapping".to_string()),
        );
        let resources = yaml.extract(&[ResourceSection::ProxyProvider]);
        assert!(resources.is_empty());
    }

    #[test]
    fn extract_singbox_empty_json() {
        let json: serde_json::Value = serde_json::json!({});
        let resources = extract_singbox_net_resources(&json);
        assert!(resources.is_empty());
    }

    #[test]
    fn extract_singbox_outbounds_with_url() {
        let json: serde_json::Value = serde_json::json!({
            "outbounds": [
                {
                    "tag": "hk-node",
                    "type": "vless",
                    "outbound": {
                        "url": "https://example.com/hk.json",
                        "path": "./outbounds/hk.json"
                    }
                }
            ]
        });
        let resources = extract_singbox_net_resources(&json);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "hk-node");
        assert_eq!(resources[0].url, "https://example.com/hk.json");
        assert_eq!(resources[0].path, "./outbounds/hk.json");
    }

    #[test]
    fn extract_singbox_rule_set_remote() {
        let json: serde_json::Value = serde_json::json!({
            "route": {
                "rule_set": [
                    {
                        "type": "remote",
                        "tag": "geoip-cn",
                        "format": "binary",
                        "url": "https://example.com/geoip.db",
                        "path": "./rules/geoip.db"
                    }
                ]
            }
        });
        let resources = extract_singbox_net_resources(&json);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "geoip-cn");
        assert_eq!(resources[0].section, ResourceSection::RuleProvider);
    }

    #[test]
    fn extract_singbox_rule_set_local_ignored() {
        let json: serde_json::Value = serde_json::json!({
            "route": {
                "rule_set": [
                    {
                        "type": "local",
                        "tag": "my-rules",
                        "path": "./rules/local.json"
                    }
                ]
            }
        });
        let resources = extract_singbox_net_resources(&json);
        assert!(resources.is_empty());
    }

    #[test]
    fn extract_singbox_no_outbounds_no_route() {
        let json: serde_json::Value = serde_json::json!({
            "log": {"level": "info"}
        });
        let resources = extract_singbox_net_resources(&json);
        assert!(resources.is_empty());
    }
}
