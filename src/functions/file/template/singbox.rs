use anyhow::Context;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;

use super::resolve_template_placeholder;
use crate::config::database::ProxyProviderGroups;

fn proxy_provider_cache_path(url: &str) -> PathBuf {
    let hash = format!("{:x}", md5::compute(url.as_bytes()));
    crate::config::singbox_proxy_providers_path().join(format!("{hash}.json"))
}

fn load_cached_proxies(url: &str) -> Option<Vec<JsonValue>> {
    let path = proxy_provider_cache_path(url);
    if !path.exists() {
        return None;
    }
    let file = std::fs::File::open(&path).ok()?;
    serde_json::from_reader(file).ok()
}

fn save_cached_proxies(url: &str, proxies: &[JsonValue]) {
    let path = proxy_provider_cache_path(url);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(file) = std::fs::File::create(&path) {
        let _ = serde_json::to_writer(file, proxies);
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn interval_to_duration(seconds: u64) -> String {
    if seconds >= 3600 && seconds % 3600 == 0 {
        format!("{}h", seconds / 3600)
    } else if seconds >= 60 && seconds % 60 == 0 {
        format!("{}m", seconds / 60)
    } else {
        format!("{}s", seconds)
    }
}

fn download_subscription(url: &str, with_proxy: bool) -> anyhow::Result<Vec<JsonValue>> {
    let mut response = crate::functions::restful::download::profile(url, with_proxy)?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut response, &mut buf)?;

    let proxies: Vec<JsonValue> = if let Ok(values) = serde_json::from_slice::<Vec<JsonValue>>(&buf)
    {
        values
    } else if let Ok(value) = serde_json::from_slice::<JsonValue>(&buf) {
        // Sing-box config: extract from outbounds
        if let Some(arr) = value.get("outbounds").and_then(|v| v.as_array()) {
            arr.clone()
        } else {
            // Single object or other — wrap as single entry
            return Ok(vec![value]);
        }
    } else {
        // Mihomo YAML format
        let yaml: serde_yml::Mapping = serde_yml::from_slice(&buf)
            .map_err(|e| anyhow::anyhow!("Failed to parse subscription as JSON or YAML: {e}"))?;
        return Ok(yaml
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|v| serde_json::to_value(v).unwrap_or(JsonValue::Null))
            .filter(|v| !v.is_null())
            .collect());
    };

    Ok(proxies)
}

/// Deduplicate proxy tags across proxy-providers for sing-box.
#[cfg_attr(not(test), allow(dead_code))]
fn dedup_singbox_proxy_tags(
    providers: std::collections::HashMap<String, Vec<JsonValue>>,
) -> std::collections::HashMap<String, Vec<JsonValue>> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut result: std::collections::HashMap<String, Vec<JsonValue>> =
        std::collections::HashMap::new();

    for (pp_name, proxies) in providers {
        let mut renamed_proxies = Vec::new();
        for mut proxy in proxies {
            if let Some(obj) = proxy.as_object_mut() {
                if let Some(tag) = obj.get("tag").and_then(|v| v.as_str()) {
                    let tag_str = tag.to_string();
                    if seen.contains(&tag_str) {
                        let new_tag = format!("{}-{}", tag_str, pp_name);
                        seen.insert(new_tag.clone());
                        obj.insert("tag".to_string(), JsonValue::String(new_tag));
                    } else {
                        seen.insert(tag_str);
                    }
                }
            }
            renamed_proxies.push(proxy);
        }
        result.insert(pp_name, renamed_proxies);
    }

    result
}

/// Resolve a `${...}` placeholder in the context of the `default` field.
///
/// Resolved provider names (from `${PPG.<group>.<provider>}`) are mapped to
/// their corresponding generated group tags (e.g. `"singbox"` → `"Select-singbox"`)
/// by searching `pg_names` values for a suffix match.
fn resolve_default_placeholder(
    value: &str,
    pg_names: &HashMap<String, Vec<String>>,
    groups: &ProxyProviderGroups,
) -> anyhow::Result<String> {
    let names = resolve_template_placeholder(value, pg_names, groups)?;
    let name = names
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("resolved placeholder returned empty for: {value}"))?;
    let suffix = format!("-{name}");
    for group_tags in pg_names.values() {
        if let Some(tag) = group_tags.iter().find(|t| t.ends_with(&suffix)) {
            return Ok(tag.clone());
        }
    }
    Ok(name)
}

/// Expand a sing-box JSON template into a complete sing-box JSON config.
///
/// The template is a full sing-box JSON config with template markers in `outbounds`:
/// - `"expand_group_with": ["${PPG.<group>}"]` on an outbound marks it for expansion
///   (one copy per proxy-provider in the group, each named `<tag>-<provider_name>`)
/// - `"${PPG.<group>}"` in `outbounds` lists expands to all proxy-provider names in that group
/// - `"${PPG.<group>.<provider>}"` expands to a specific provider name
/// - `"${PGG.<name>}"` or `"${PGG.<name>.<provider>}"` in `outbounds` lists expands to
///   generated group tags
/// - Placeholders in `default` field are also resolved (provider names mapped to group tags)
///
/// Other sections (dns, inbounds, route, experimental, log) pass through unchanged.
/// If the template includes `rules` / `rule-providers` (mihomo-style), they are
/// translated to sing-box native `route` rules/rule_set.
pub async fn gen_template_singbox(
    tpl: &JsonValue,
    _template_name: &str,
    groups: &ProxyProviderGroups,
    with_proxy: bool,
    force_refresh: bool,
) -> anyhow::Result<JsonValue> {
    use std::collections::HashMap;

    // --- Download subscription URLs → proxy nodes ---
    let mut provider_proxies: HashMap<String, Vec<JsonValue>> = HashMap::new();
    let mut download_handles = Vec::new();
    for providers in groups.values() {
        for (pp_name, url) in providers {
            let url = url.clone();
            let pp_name = pp_name.clone();
            download_handles.push(tokio::task::spawn_blocking(move || {
                if !force_refresh {
                    if let Some(cached) = load_cached_proxies(&url) {
                        log::info!("Using cached proxies for {pp_name} ({})", cached.len());
                        return (pp_name, Ok(cached));
                    }
                }
                match download_subscription(&url, with_proxy) {
                    Ok(proxies) => {
                        save_cached_proxies(&url, &proxies);
                        (pp_name, Ok(proxies))
                    }
                    Err(e) => {
                        if let Some(cached) = load_cached_proxies(&url) {
                            log::warn!(
                                "Failed to download subscription for {pp_name}: {e}, using cache"
                            );
                            (pp_name, Ok(cached))
                        } else {
                            (pp_name, Err(e))
                        }
                    }
                }
            }));
        }
    }
    let mut download_errors: Vec<String> = Vec::new();
    for handle in download_handles {
        let (pp_name, result) = handle.await?;
        match result {
            Ok(proxies) => {
                // Auto-generate tags for nodes missing them
                let tagged: Vec<JsonValue> = proxies
                    .into_iter()
                    .map(|mut proxy| {
                        if let Some(obj) = proxy.as_object_mut() {
                            if !obj.contains_key("tag") {
                                let tag = format!(
                                    "{pp_name}-{}",
                                    obj.get("server").and_then(|v| v.as_str()).unwrap_or("node")
                                );
                                obj.insert("tag".to_string(), JsonValue::String(tag));
                            }
                        }
                        proxy
                    })
                    .collect();
                log::info!("Downloaded {} proxies for {pp_name}", tagged.len());
                provider_proxies.insert(pp_name, tagged);
            }
            Err(e) => {
                download_errors.push(format!("{pp_name}: {e}"));
                log::warn!("Failed to download subscription for {pp_name}: {e}");
            }
        }
    }
    if !download_errors.is_empty() {
        anyhow::bail!(
            "Failed to download proxy providers — profile not generated:\n{}",
            download_errors.join("\n")
        );
    }

    expand_singbox_template(tpl, provider_proxies, groups)
}

pub fn expand_singbox_template(
    tpl: &JsonValue,
    mut provider_proxies: HashMap<String, Vec<JsonValue>>,
    groups: &ProxyProviderGroups,
) -> anyhow::Result<JsonValue> {
    // Filter out group-type entries (selector, urltest, etc.) —
    // only keep actual proxy nodes
    for proxies in provider_proxies.values_mut() {
        proxies.retain(|p| {
            let t = p.get("type").and_then(|v| v.as_str()).unwrap_or("");
            !matches!(
                t,
                "selector"
                    | "urltest"
                    | "select"
                    | "url-test"
                    | "fallback"
                    | "load-balance"
                    | "direct"
                    | "block"
                    | "dns"
                    | ""
            )
        });
    }

    // Build tag index: provider_name → [tag, ...]
    let mut pp_tags: HashMap<String, Vec<String>> = HashMap::new();
    for (pp_name, proxies) in &provider_proxies {
        let tags: Vec<String> = proxies
            .iter()
            .filter_map(|v| v.get("tag").and_then(|t| t.as_str()).map(String::from))
            .collect();
        pp_tags.insert(pp_name.clone(), tags);
    }

    // --- Clone template and process outbounds ---
    let mut output = tpl.clone();

    let tpl_outbounds = output
        .get("outbounds")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut new_outbounds: Vec<JsonValue> = Vec::new();
    let mut pg_names: HashMap<String, Vec<String>> = HashMap::new();

    // --- First pass: process expand_group_with outbounds to populate pg_names ---
    for ob in &tpl_outbounds {
        if ob.get("expand_group_with").is_none() {
            continue;
        }

        let expand_keys = ob["expand_group_with"]
            .as_array()
            .context("expand_group_with must be an array")?;

        let group_tag = ob
            .get("tag")
            .and_then(|v| v.as_str())
            .context("expand_group_with outbound must have a tag")?;

        for the_expand_key in expand_keys {
            let pk_str = the_expand_key
                .as_str()
                .context("expand_group_with entries must be strings")?;
            let provider_names = resolve_template_placeholder(pk_str, &pg_names, groups)?;

            for pp_name in &provider_names {
                let tags = pp_tags.get(pp_name).cloned().unwrap_or_default();

                // Skip empty providers
                if tags.is_empty() {
                    continue;
                }

                let new_tag = format!("{group_tag}-{pp_name}");

                pg_names
                    .entry(group_tag.to_string())
                    .or_default()
                    .push(new_tag);
            }
        }
    }

    // --- Second pass: emit all outbounds with placeholders resolved ---
    for ob in tpl_outbounds {
        let has_expand = ob.get("expand_group_with").is_some();

        if has_expand {
            // --- Template group: expand one per proxy-provider in group ---
            let ob_type = ob.get("type").and_then(|v| v.as_str()).unwrap_or("urltest");
            let sb_type = match ob_type {
                "select" => "selector",
                "url-test" | "urltest" => "urltest",
                "fallback" => "urltest",
                _ => "selector",
            };

            let expand_keys = ob["expand_group_with"]
                .as_array()
                .context("expand_group_with must be an array")?;

            let group_tag = ob
                .get("tag")
                .and_then(|v| v.as_str())
                .context("expand_group_with outbound must have a tag")?;

            for the_expand_key in expand_keys {
                let pk_str = the_expand_key
                    .as_str()
                    .context("expand_group_with entries must be strings")?;
                let provider_names = resolve_template_placeholder(pk_str, &pg_names, groups)?;

                for pp_name in &provider_names {
                    let tags = pp_tags.get(pp_name).cloned().unwrap_or_default();

                    // Skip empty providers
                    if tags.is_empty() {
                        continue;
                    }

                    let new_tag = format!("{group_tag}-{pp_name}");

                    let mut sb_group = serde_json::json!({
                        "type": sb_type,
                        "tag": new_tag,
                        "outbounds": tags,
                    });

                    if sb_type == "urltest" {
                        if let Some(url) = ob.get("url").and_then(|v| v.as_str()) {
                            sb_group["url"] = JsonValue::String(url.to_string());
                        }
                        if let Some(interval) = ob.get("interval").and_then(|v| v.as_str()) {
                            sb_group["interval"] = JsonValue::String(interval.to_string());
                        }
                        if let Some(tolerance) = ob.get("tolerance") {
                            sb_group["tolerance"] = tolerance.clone();
                        }
                    }
                    if let Some(default) = ob.get("default") {
                        sb_group["default"] = default.clone();
                    }
                    if let Some(interrupt) = ob.get("interrupt_exist_connections") {
                        sb_group["interrupt_exist_connections"] = interrupt.clone();
                    }

                    new_outbounds.push(sb_group);
                }
            }
        } else {
            // --- Passthrough outbound: resolve ${} placeholders in outbounds list ---
            let mut ob = ob.clone();
            if let Some(outbounds_arr) = ob.get("outbounds").and_then(|v| v.as_array()) {
                let mut resolved: Vec<String> = Vec::new();
                for item in outbounds_arr {
                    let item_str = item.as_str().unwrap_or("");
                    if item_str.starts_with("${") && item_str.ends_with('}') {
                        let names = resolve_template_placeholder(item_str, &pg_names, groups)
                            .with_context(|| {
                                format!("Can't resolve placeholder in outbounds: {item_str}")
                            })?;
                        for name in names {
                            if let Some(tags) = pp_tags.get(&name) {
                                resolved.extend(tags.clone());
                            } else {
                                resolved.push(name);
                            }
                        }
                    } else {
                        resolved.push(item_str.to_string());
                    }
                }
                ob["outbounds"] = serde_json::json!(resolved);
            }
            // Resolve ${} placeholders in default field
            if let Some(default_val) = ob.get("default").and_then(|v| v.as_str()) {
                if default_val.starts_with("${") && default_val.ends_with('}') {
                    let resolved_default =
                        resolve_default_placeholder(default_val, &pg_names, groups)?;
                    ob["default"] = JsonValue::String(resolved_default);
                }
            }
            new_outbounds.push(ob);
        }
    }

    // Append downloaded proxy nodes at the end of outbounds
    for proxies in provider_proxies.values() {
        new_outbounds.extend(proxies.clone());
    }

    output["outbounds"] = JsonValue::Array(new_outbounds);

    // Inject clashtui.proxy_provider_groups if non-empty
    if !groups.is_empty() {
        let ppg_json = serde_json::to_value(groups).unwrap_or_default();
        output["clashtui"] = serde_json::json!({
            "proxy_provider_groups": ppg_json
        });
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn interval_to_duration_hours() {
        assert_eq!(interval_to_duration(3600), "1h");
        assert_eq!(interval_to_duration(7200), "2h");
    }

    #[test]
    fn interval_to_duration_minutes() {
        assert_eq!(interval_to_duration(300), "5m");
        assert_eq!(interval_to_duration(60), "1m");
        assert_eq!(interval_to_duration(120), "2m");
    }

    #[test]
    fn interval_to_duration_seconds() {
        assert_eq!(interval_to_duration(0), "0s");
        assert_eq!(interval_to_duration(45), "45s");
        assert_eq!(interval_to_duration(59), "59s");
    }

    #[test]
    fn interval_to_duration_non_divisible() {
        assert_eq!(interval_to_duration(3661), "3661s");
        assert_eq!(interval_to_duration(61), "61s");
    }

    #[test]
    fn dedup_no_duplicates_preserves_tags() {
        let mut providers = HashMap::new();
        providers.insert(
            "pvd0".to_string(),
            vec![
                json!({"tag": "node1", "type": "ss", "server": "s1.com", "server_port": 443}),
                json!({"tag": "node2", "type": "vmess", "server": "s2.com", "server_port": 8443}),
            ],
        );
        providers.insert(
            "pvd1".to_string(),
            vec![json!({"tag": "node3", "type": "trojan", "server": "s3.com", "server_port": 443})],
        );
        let result = dedup_singbox_proxy_tags(providers);
        assert_eq!(result["pvd0"].len(), 2);
        assert_eq!(result["pvd0"][0]["tag"], "node1");
        assert_eq!(result["pvd0"][1]["tag"], "node2");
        assert_eq!(result["pvd1"][0]["tag"], "node3");
    }

    #[test]
    fn dedup_renames_colliding_tags() {
        let mut providers = HashMap::new();
        providers.insert(
            "pvd0".to_string(),
            vec![json!({"tag": "shared", "type": "ss", "server": "a.com", "server_port": 443})],
        );
        providers.insert(
            "pvd1".to_string(),
            vec![json!({"tag": "shared", "type": "vmess", "server": "b.com", "server_port": 8443})],
        );
        let result = dedup_singbox_proxy_tags(providers);
        let pvd0_tag = result["pvd0"][0]["tag"].as_str().unwrap();
        let pvd1_tag = result["pvd1"][0]["tag"].as_str().unwrap();
        assert_ne!(pvd0_tag, pvd1_tag, "colliding tags should be made unique");
        assert!(pvd0_tag.starts_with("shared"));
        assert!(pvd1_tag.starts_with("shared"));
    }

    #[test]
    fn dedup_handles_missing_tag() {
        let mut providers = HashMap::new();
        providers.insert(
            "pvd0".to_string(),
            vec![json!({"type": "ss", "server": "a.com", "server_port": 443})],
        );
        let result = dedup_singbox_proxy_tags(providers);
        assert_eq!(result["pvd0"].len(), 1);
        assert!(result["pvd0"][0].get("tag").is_none());
    }

    #[test]
    fn resolve_default_placeholder_exact_match() {
        let tag_map: HashMap<String, Vec<String>> = vec![(
            "Auto-pvd".to_string(),
            vec!["Auto-pvd-pvd0".to_string(), "Auto-pvd-pvd1".to_string()],
        )]
        .into_iter()
        .collect();
        let ppg: ProxyProviderGroups = vec![(
            "pvd".to_string(),
            vec![
                ("pvd0".to_string(), "https://e.com/1.json".to_string()),
                ("pvd1".to_string(), "https://e.com/2.json".to_string()),
            ]
            .into_iter()
            .collect(),
        )]
        .into_iter()
        .collect();

        let result = resolve_default_placeholder("${PPG.pvd.pvd0}", &tag_map, &ppg);
        let tag = result.unwrap();
        assert!(tag.starts_with("Auto-pvd"));
    }

    #[test]
    fn resolve_default_placeholder_fallback() {
        let tag_map: HashMap<String, Vec<String>> = HashMap::new();
        let ppg: ProxyProviderGroups = HashMap::new();

        let result = resolve_default_placeholder("${PPG.pvd.pvd0}", &tag_map, &ppg);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_default_placeholder_invalid_format() {
        let tag_map: HashMap<String, Vec<String>> = HashMap::new();
        let ppg: ProxyProviderGroups = HashMap::new();

        let result = resolve_default_placeholder("plain-string", &tag_map, &ppg);
        assert!(result.is_err());
    }

    fn load_json_fixture(path: &str) -> JsonValue {
        let full = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), path);
        let data = std::fs::read_to_string(full).unwrap();
        serde_json::from_str(&data).unwrap()
    }

    fn make_groups() -> ProxyProviderGroups {
        let mut groups = ProxyProviderGroups::new();
        let mut pvd = std::collections::BTreeMap::new();
        pvd.insert(
            "hajimi".to_string(),
            "https://e.com/hajimi.json".to_string(),
        );
        pvd.insert("manbo".to_string(), "https://e.com/manbo.json".to_string());
        groups.insert("pvd".to_string(), pvd);
        groups
    }

    fn make_provider_proxies() -> (
        HashMap<String, Vec<JsonValue>>,
        HashMap<String, Vec<JsonValue>>,
    ) {
        let hajimi = load_json_fixture("tests/proxy-providers/sing-box/hk.json");
        let manbo = load_json_fixture("tests/proxy-providers/sing-box/jp.json");

        let hajimi_arr: Vec<JsonValue> = hajimi.as_array().unwrap().clone();
        let manbo_arr: Vec<JsonValue> = manbo.as_array().unwrap().clone();

        let mut provider_proxies = HashMap::new();
        provider_proxies.insert("hajimi".to_string(), hajimi_arr.clone());
        provider_proxies.insert("manbo".to_string(), manbo_arr.clone());

        let mut expected = HashMap::new();
        expected.insert("hajimi".to_string(), hajimi_arr);
        expected.insert("manbo".to_string(), manbo_arr);
        (provider_proxies, expected)
    }

    #[test]
    fn expand_template_produces_expanded_groups() {
        let tpl = load_json_fixture("tests/templates/sing-box/expand_test_tpl.json");
        let groups = make_groups();
        let (provider_proxies, _) = make_provider_proxies();

        let result = expand_singbox_template(&tpl, provider_proxies, &groups).unwrap();
        let outbounds = result["outbounds"].as_array().unwrap();

        let auto_hajimi = outbounds
            .iter()
            .find(|ob| ob["tag"] == "Auto-hajimi")
            .expect("Auto-hajimi missing");
        assert_eq!(auto_hajimi["type"], "urltest");
        assert_eq!(auto_hajimi["outbounds"].as_array().unwrap().len(), 3);

        let auto_manbo = outbounds
            .iter()
            .find(|ob| ob["tag"] == "Auto-manbo")
            .expect("Auto-manbo missing");
        assert_eq!(auto_manbo["type"], "urltest");

        let select_hajimi = outbounds
            .iter()
            .find(|ob| ob["tag"] == "Select-hajimi")
            .expect("Select-hajimi missing");
        assert_eq!(select_hajimi["type"], "selector");

        let select_manbo = outbounds
            .iter()
            .find(|ob| ob["tag"] == "Select-manbo")
            .expect("Select-manbo missing");
        assert_eq!(select_manbo["type"], "selector");
    }

    #[test]
    fn expand_template_resolves_pgg_placeholder() {
        let tpl = load_json_fixture("tests/templates/sing-box/expand_test_tpl.json");
        let groups = make_groups();
        let (provider_proxies, _) = make_provider_proxies();

        let result = expand_singbox_template(&tpl, provider_proxies, &groups).unwrap();
        let outbounds = result["outbounds"].as_array().unwrap();

        let entry = outbounds.iter().find(|ob| ob["tag"] == "Entry").unwrap();
        let entry_outbounds: Vec<String> = entry["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();

        assert!(entry_outbounds.contains(&"Auto-hajimi".to_string()));
        assert!(entry_outbounds.contains(&"Auto-manbo".to_string()));
        assert!(entry_outbounds.contains(&"Select-hajimi".to_string()));
        assert!(entry_outbounds.contains(&"Select-manbo".to_string()));
        assert!(entry_outbounds.contains(&"direct".to_string()));
    }

    #[test]
    fn expand_template_resolves_default_placeholder() {
        let tpl = load_json_fixture("tests/templates/sing-box/expand_test_tpl.json");
        let groups = make_groups();
        let (provider_proxies, _) = make_provider_proxies();

        let result = expand_singbox_template(&tpl, provider_proxies, &groups).unwrap();
        let outbounds = result["outbounds"].as_array().unwrap();

        let entry = outbounds.iter().find(|ob| ob["tag"] == "Entry").unwrap();
        let default = entry["default"].as_str().unwrap();
        assert!(default.starts_with("Auto"));

        let select_hajimi = outbounds
            .iter()
            .find(|ob| ob["tag"] == "Select-hajimi")
            .unwrap();
        let sel_default = select_hajimi["default"].as_str().unwrap();
        assert_eq!(sel_default, "${PPG.pvd.hajimi}");
    }

    #[test]
    fn expand_template_appends_proxy_nodes() {
        let tpl = load_json_fixture("tests/templates/sing-box/expand_test_tpl.json");
        let groups = make_groups();
        let (provider_proxies, _) = make_provider_proxies();

        let result = expand_singbox_template(&tpl, provider_proxies, &groups).unwrap();
        let outbounds = result["outbounds"].as_array().unwrap();

        let tags: Vec<&str> = outbounds
            .iter()
            .filter_map(|ob| ob.get("tag").and_then(|t| t.as_str()))
            .collect();
        assert!(tags.contains(&"hk-01"));
        assert!(tags.contains(&"hk-02"));
        assert!(tags.contains(&"hk-03"));
        assert!(tags.contains(&"jp-01"));
        assert!(tags.contains(&"jp-02"));
        assert!(tags.contains(&"jp-03"));
    }

    #[test]
    fn expand_template_injects_clashtui_metadata() {
        let tpl = load_json_fixture("tests/templates/sing-box/expand_test_tpl.json");
        let groups = make_groups();
        let (provider_proxies, _) = make_provider_proxies();

        let result = expand_singbox_template(&tpl, provider_proxies, &groups).unwrap();
        assert!(result.get("clashtui").is_some());
        let ppg = &result["clashtui"]["proxy_provider_groups"];
        assert!(ppg.get("pvd").is_some());
    }

    #[test]
    fn expand_template_preserves_non_outbound_sections() {
        let tpl = load_json_fixture("tests/templates/sing-box/expand_test_tpl.json");
        let groups = make_groups();
        let (provider_proxies, _) = make_provider_proxies();

        let result = expand_singbox_template(&tpl, provider_proxies, &groups).unwrap();
        assert_eq!(result["log"]["level"], "info");
        assert!(result["dns"]["servers"].as_array().is_some());
        assert!(result["inbounds"].as_array().is_some());
        assert!(result["route"]["rules"].as_array().is_some());
    }

    #[test]
    fn expand_template_fails_with_empty_providers() {
        let tpl = load_json_fixture("tests/templates/sing-box/expand_test_tpl.json");
        let groups = make_groups();
        let provider_proxies = HashMap::new();

        let result = expand_singbox_template(&tpl, provider_proxies, &groups);
        assert!(result.is_err());
    }
}
