use anyhow::Context;

use super::{PROXY_GROUPS, PROXY_PROVIDERS};

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
struct PGparam {
    providers: Vec<String>,
}
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
struct PGitem {
    name: String,
    #[serde(rename = "use")]
    #[serde(skip_serializing_if = "Option::is_none")]
    us_: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proxies: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tpl_param: Option<PGparam>,
    #[serde(rename = "type")]
    __type: String,
    #[serde(flatten)]
    __others: serde_yml::Value,
}

pub(super) fn gen_template(
    tpl: serde_yml::Mapping,
    template_name: &str,
    urls: &[String],
) -> anyhow::Result<serde_yml::Mapping> {
    let tpl_name = std::path::Path::new(template_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(template_name);
    gen_template_with_urls(tpl, tpl_name, urls)
}

pub(super) fn gen_template_with_urls(
    tpl: serde_yml::Mapping,
    tpl_name: &str,
    proxy_urls: &[String],
) -> anyhow::Result<serde_yml::Mapping> {
    use std::collections::HashMap;

    let mut out_parsed_yaml = tpl.clone();

    // ## proxy-providers
    let mut pp_names: HashMap<String, Vec<String>> = HashMap::new();
    let mut new_proxy_providers = serde_yml::Mapping::new();
    let pp_mapping = if let Some(serde_yml::Value::Mapping(pp_mapping)) = tpl.get(PROXY_PROVIDERS) {
        pp_mapping
    } else {
        anyhow::bail!("Failed to parse `proxy-providers`");
    };

    for (pp_key, pp_value) in pp_mapping {
        if pp_value.get("tpl_param").is_none() {
            new_proxy_providers.insert(pp_key.clone(), pp_value.clone());
            continue;
        }

        let pp = pp_value
            .as_mapping()
            .ok_or_else(|| anyhow::anyhow!("Failed to parse `proxy-providers` value"))?;

        let pp_key_str = pp_key
            .as_str()
            .with_context(|| "Proxy-provider key is not a string")?;

        for (i, url) in proxy_urls.iter().enumerate() {
            let mut new_pp = pp.clone();
            new_pp.remove("tpl_param");
            let the_pp_name = format!("{}{}", pp_key_str, i);
            pp_names
                .entry(pp_key_str.to_string())
                .or_default()
                .push(the_pp_name.clone());

            new_pp.insert(
                serde_yml::Value::String("url".into()),
                serde_yml::Value::String(url.clone()),
            );
            new_pp.insert(
                serde_yml::Value::String("path".into()),
                serde_yml::Value::String(format!(
                    "proxy-providers/tpl/{}/{}.yaml",
                    tpl_name, the_pp_name
                )),
            );
            new_proxy_providers.insert(
                serde_yml::Value::String(the_pp_name),
                serde_yml::Value::Mapping(new_pp),
            );
        }
    }
    out_parsed_yaml[PROXY_PROVIDERS] = serde_yml::Value::Mapping(new_proxy_providers);

    // ## proxy-groups
    let mut pg_names: HashMap<String, Vec<String>> = HashMap::new();
    let mut new_proxy_groups = serde_yml::Sequence::new();
    let pg_value = if let Some(serde_yml::Value::Sequence(pg_value)) = tpl.get(PROXY_GROUPS) {
        pg_value
    } else {
        anyhow::bail!("Failed to parse `proxy-groups`.");
    };

    for the_pg_value in pg_value {
        if the_pg_value.get("tpl_param").is_none() {
            new_proxy_groups.push(the_pg_value.clone());
            continue;
        }

        let the_pg = if let serde_yml::Value::Mapping(the_pg) = the_pg_value {
            the_pg
        } else {
            anyhow::bail!("Failed to parse `proxy-groups` value");
        };

        let mut new_pg = the_pg.clone();
        new_pg.remove("tpl_param");

        let provider_keys = if let Some(serde_yml::Value::Sequence(provider_keys)) =
            the_pg["tpl_param"].get("providers")
        {
            provider_keys
        } else {
            anyhow::bail!("Failed to parse `providers` in `tpl_param`");
        };

        for the_provider_key in provider_keys {
            let the_pk_str = if let serde_yml::Value::String(the_pk_str) = the_provider_key {
                the_pk_str
            } else {
                anyhow::bail!("Failed to parse string in `providers`")
            };

            let names = if let Some(names) = pp_names.get(the_pk_str) {
                names
            } else {
                continue;
            };

            let the_pg_name =
                if let Some(serde_yml::Value::String(the_pg_name)) = the_pg_value.get("name") {
                    the_pg_name
                } else {
                    anyhow::bail!("Failed to parse `name` in `proxy-groups`");
                };

            for n in names {
                let new_pg_name = format!("{}-{}", the_pg_name, n);

                pg_names
                    .entry(the_pg_name.clone())
                    .or_default()
                    .push(new_pg_name.clone());

                new_pg["name"] = serde_yml::Value::String(new_pg_name.clone());
                new_pg.insert(
                    serde_yml::Value::String("use".into()),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String(n.clone())]),
                );

                new_proxy_groups.push(serde_yml::Value::Mapping(new_pg.clone()));
            }
        }
    }
    out_parsed_yaml[PROXY_GROUPS] = serde_yml::Value::Sequence(new_proxy_groups);

    // ### replace special keys in group-providers
    let pg_sequence = if let Some(serde_yml::Value::Sequence(pg_sequence)) =
        out_parsed_yaml.get_mut(PROXY_GROUPS)
    {
        pg_sequence
    } else {
        anyhow::bail!("Failed to parse `proxy-groups`");
    };

    for the_pg_seq in pg_sequence {
        if let Some(providers) = the_pg_seq.get("use") {
            let prov_seq = providers
                .as_sequence()
                .with_context(|| "`use` field is not a sequence")?;
            let mut new_providers = Vec::new();
            for p in prov_seq {
                let p_str = p
                    .as_str()
                    .with_context(|| "Non-string value in `use` list")?;
                if p_str.starts_with('<') && p_str.ends_with('>') {
                    let trimmed_p_str = p_str.trim_matches(|c| c == '<' || c == '>');
                    let provider_names = pp_names
                        .get(trimmed_p_str)
                        .with_context(|| "Can't find the proxy-provider name.")?;
                    new_providers.extend(provider_names.iter().cloned());
                } else {
                    new_providers.push(p_str.to_string());
                }
            }
            the_pg_seq["use"] = serde_yml::Value::Sequence(
                new_providers
                    .into_iter()
                    .map(serde_yml::Value::String)
                    .collect(),
            );
        }

        if let Some(serde_yml::Value::Sequence(groups)) = the_pg_seq.get("proxies") {
            let mut new_groups = Vec::new();
            for g in groups {
                let g_str = g
                    .as_str()
                    .with_context(|| "Non-string value in `proxies` list")?;
                if g_str.starts_with('<') && g_str.ends_with('>') {
                    let trimmed_g_str = g_str.trim_matches(|c| c == '<' || c == '>');
                    let group_names = pg_names
                        .get(trimmed_g_str)
                        .with_context(|| "Can't find the proxy-group name.")?;
                    new_groups.extend(group_names.iter().cloned());
                } else {
                    new_groups.push(g_str.to_string());
                }
            }
            the_pg_seq["proxies"] = serde_yml::Value::Sequence(
                new_groups
                    .into_iter()
                    .map(serde_yml::Value::String)
                    .collect(),
            );
        }
    }

    out_parsed_yaml.insert(
        "clashtui".into(),
        serde_yml::Value::Null,
    );

    Ok(out_parsed_yaml)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn testdata_path(name: &str) -> std::path::PathBuf {
        let manifest_dir = std::env!("CARGO_MANIFEST_DIR");
        std::path::PathBuf::from(manifest_dir)
            .join("src/functions/file/template/testdata")
            .join(name)
    }

    fn load_yaml<P: AsRef<std::path::Path>>(
        path: P,
    ) -> anyhow::Result<serde_yml::Mapping> {
        let file = std::fs::File::open(path)?;
        Ok(serde_yml::from_reader(file)?)
    }

    #[test]
    fn test_simple_expansion() {
        let tpl = load_yaml(testdata_path("simple_tpl.yaml")).unwrap();
        let expected = serde_yml::from_reader::<_, serde_yml::Value>(
            std::fs::File::open(testdata_path("simple_tpl_output.yaml")).unwrap(),
        )
        .unwrap();

        let result = gen_template_with_urls(tpl, "simple_tpl", &["https://example.com/sub1.yaml".into()]).unwrap();
        let result_value = serde_yml::to_value(result).unwrap();

        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_multi_provider_expansion() {
        let tpl = load_yaml(testdata_path("multi_provider_tpl.yaml")).unwrap();
        let expected = serde_yml::from_reader::<_, serde_yml::Value>(
            std::fs::File::open(testdata_path("multi_provider_tpl_output.yaml")).unwrap(),
        )
        .unwrap();

        let result = gen_template_with_urls(tpl, "multi_provider_tpl", &[
            "https://example.com/sub1.yaml".into(),
            "https://example.com/sub2.yaml".into(),
        ]).unwrap();
        let result_value = serde_yml::to_value(result).unwrap();

        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_no_tpl_param_passthrough() {
        let tpl = load_yaml(testdata_path("no_tpl_param_tpl.yaml")).unwrap();
        let expected = serde_yml::from_reader::<_, serde_yml::Value>(
            std::fs::File::open(testdata_path("no_tpl_param_tpl_output.yaml")).unwrap(),
        )
        .unwrap();

        let result = gen_template_with_urls(tpl, "no_tpl_param_tpl", &[]).unwrap();
        let result_value = serde_yml::to_value(result).unwrap();

        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_empty_uses() {
        let tpl = load_yaml(testdata_path("empty_uses_tpl.yaml")).unwrap();
        let expected = serde_yml::from_reader::<_, serde_yml::Value>(
            std::fs::File::open(testdata_path("empty_uses_tpl_output.yaml")).unwrap(),
        )
        .unwrap();

        let result = gen_template_with_urls(tpl, "empty_uses_tpl", &["https://example.com/sub1.yaml".into()]).unwrap();
        let result_value = serde_yml::to_value(result).unwrap();

        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_ordering_preserved_proxy_groups() {
        let tpl = load_yaml(testdata_path("simple_tpl.yaml")).unwrap();
        let result = gen_template_with_urls(tpl, "simple_tpl", &["https://example.com/sub1.yaml".into()]).unwrap();

        let groups = result
            .get(PROXY_GROUPS)
            .and_then(|v| v.as_sequence())
            .unwrap();

        // Expected order: Select, Auto-pvd0, Direct
        let names: Vec<&str> = groups
            .iter()
            .filter_map(|g| g.get("name").and_then(|n| n.as_str()))
            .collect();
        assert_eq!(names, vec!["Select", "Auto-pvd0", "Direct"]);
    }

    #[test]
    fn test_ordering_preserved_proxy_providers() {
        let tpl = load_yaml(testdata_path("simple_tpl.yaml")).unwrap();
        let result = gen_template_with_urls(tpl, "simple_tpl", &["https://example.com/sub1.yaml".into()]).unwrap();

        let providers = result
            .get(PROXY_PROVIDERS)
            .and_then(|v| v.as_mapping())
            .unwrap();

        // pvd0 generated from pvd template, static follows
        let keys: Vec<&str> = providers.keys().filter_map(|k| k.as_str()).collect();
        assert_eq!(keys, vec!["pvd0", "static"]);
    }

    #[test]
    fn test_angle_bracket_provider_placeholder() {
        let tpl = load_yaml(testdata_path("multi_provider_tpl.yaml")).unwrap();
        let result = gen_template_with_urls(tpl, "multi_provider_tpl", &[
            "https://example.com/sub1.yaml".into(),
            "https://example.com/sub2.yaml".into(),
        ]).unwrap();

        let groups = result
            .get(PROXY_GROUPS)
            .and_then(|v| v.as_sequence())
            .unwrap();

        let all_in_one = groups
            .iter()
            .find(|g| g.get("name").and_then(|n| n.as_str()) == Some("AllInOne"))
            .unwrap();

        let uses: Vec<&str> = all_in_one
            .get("use")
            .and_then(|v| v.as_sequence())
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        assert_eq!(uses, vec!["pvd0", "pvd1", "pvd20", "pvd21"]);
    }

    #[test]
    fn test_angle_bracket_group_placeholder() {
        let tpl = load_yaml(testdata_path("multi_provider_tpl.yaml")).unwrap();
        let result = gen_template_with_urls(tpl, "multi_provider_tpl", &[
            "https://example.com/sub1.yaml".into(),
            "https://example.com/sub2.yaml".into(),
        ]).unwrap();

        let groups = result
            .get(PROXY_GROUPS)
            .and_then(|v| v.as_sequence())
            .unwrap();

        let select = groups
            .iter()
            .find(|g| g.get("name").and_then(|n| n.as_str()) == Some("Select"))
            .unwrap();

        let proxies: Vec<&str> = select
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        assert_eq!(
            proxies,
            vec!["DIRECT", "Auto-pvd0", "Auto-pvd1", "Fallback-pvd20", "Fallback-pvd21"]
        );
    }

    #[test]
    fn test_missing_proxy_providers_section() {
        let tpl = load_yaml(testdata_path("missing_pp_tpl.yaml")).unwrap();
        let result = gen_template_with_urls(tpl, "missing_pp_tpl", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_proxy_groups_section() {
        let tpl = load_yaml(testdata_path("missing_pg_tpl.yaml")).unwrap();
        let result = gen_template_with_urls(tpl, "missing_pg_tpl", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_tpl_param_providers_key() {
        let tpl = load_yaml(testdata_path("missing_providers_key_tpl.yaml")).unwrap();
        let result = gen_template_with_urls(tpl, "missing_providers_key_tpl", &["https://example.com/sub1.yaml".into()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_placeholder_to_nonexistent_target() {
        let tpl = load_yaml(testdata_path("bad_placeholder_tpl.yaml")).unwrap();
        let result = gen_template_with_urls(tpl, "bad_placeholder_tpl", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_clashtui_marker_present() {
        let tpl = load_yaml(testdata_path("simple_tpl.yaml")).unwrap();
        let result = gen_template_with_urls(tpl, "simple_tpl", &["https://example.com/sub1.yaml".into()]).unwrap();

        assert!(result.contains_key("clashtui"));
        assert_eq!(result.get("clashtui").unwrap(), &serde_yml::Value::Null);
    }

    #[test]
    fn test_multi_url_expansion() {
        let tpl = load_yaml(testdata_path("simple_tpl.yaml")).unwrap();
        let result = gen_template_with_urls(tpl, "simple_tpl", &[
            "https://a.example.com/p1.yaml".into(),
            "https://b.example.com/p2.yaml".into(),
        ]).unwrap();

        let providers = result
            .get(PROXY_PROVIDERS)
            .and_then(|v| v.as_mapping())
            .unwrap();

        // pvd0 and pvd1 generated, static follows
        let keys: Vec<&str> = providers.keys().filter_map(|k| k.as_str()).collect();
        assert_eq!(keys, vec!["pvd0", "pvd1", "static"]);

        let pvd0 = providers.get("pvd0").unwrap().as_mapping().unwrap();
        assert_eq!(
            pvd0.get("url").and_then(|v| v.as_str()),
            Some("https://a.example.com/p1.yaml")
        );
        assert_eq!(
            pvd0.get("path").and_then(|v| v.as_str()),
            Some("proxy-providers/tpl/simple_tpl/pvd0.yaml")
        );

        let pvd1 = providers.get("pvd1").unwrap().as_mapping().unwrap();
        assert_eq!(
            pvd1.get("url").and_then(|v| v.as_str()),
            Some("https://b.example.com/p2.yaml")
        );
        assert_eq!(
            pvd1.get("path").and_then(|v| v.as_str()),
            Some("proxy-providers/tpl/simple_tpl/pvd1.yaml")
        );
    }

    #[test]
    fn test_empty_urls_no_tpl_param_entries() {
        let tpl = load_yaml(testdata_path("simple_tpl.yaml")).unwrap();
        // pvd has tpl_param but no URLs → no providers generated
        // Auto needs pvd → no groups generated
        // Select has <Auto> placeholder → unresolvable, must error
        let result = gen_template_with_urls(tpl, "simple_tpl", &[]);
        assert!(result.is_err());
    }
}
