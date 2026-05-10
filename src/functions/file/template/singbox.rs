use anyhow::Context;
use serde_json::Value as JsonValue;

use super::{PROXY_GROUPS, PROXY_PROVIDERS};

fn interval_to_duration(seconds: u64) -> String {
    if seconds >= 3600 && seconds % 3600 == 0 {
        format!("{}h", seconds / 3600)
    } else if seconds >= 60 && seconds % 60 == 0 {
        format!("{}m", seconds / 60)
    } else {
        format!("{}s", seconds)
    }
}

fn translate_rule(rule_str: &str) -> Option<JsonValue> {
    let parts: Vec<&str> = rule_str.splitn(3, ',').collect();
    if parts.len() < 2 {
        return None;
    }
    let matcher = parts[0].trim();
    let value = parts[1].trim();
    let target = parts.get(2).map(|s| s.trim().to_string()).unwrap_or_default();

    match matcher {
        "DOMAIN-SUFFIX" => Some(serde_json::json!({
            "domain_suffix": [value],
            "outbound": target
        })),
        "DOMAIN-KEYWORD" => Some(serde_json::json!({
            "domain_keyword": [value],
            "outbound": target
        })),
        "DOMAIN" => Some(serde_json::json!({
            "domain": [value],
            "outbound": target
        })),
        "IP-CIDR" | "IP-CIDR6" => Some(serde_json::json!({
            "ip_cidr": [value],
            "outbound": target
        })),
        "PROCESS-NAME" => Some(serde_json::json!({
            "process_name": [value],
            "outbound": target
        })),
        "GEOSITE" => Some(serde_json::json!({
            "rule_set": format!("geosite-{value}"),
            "outbound": target
        })),
        "GEOIP" => Some(serde_json::json!({
            "rule_set": format!("geoip-{value}"),
            "outbound": target
        })),
        "MATCH" => None,
        _ => {
            log::warn!("Unsupported rule matcher in sing-box template: {matcher}");
            None
        }
    }
}

fn download_subscription(url: &str, with_proxy: bool) -> anyhow::Result<Vec<JsonValue>> {
    let mut response =
        crate::functions::restful::download::profile(url, with_proxy)?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut response, &mut buf)?;

    if let Ok(values) = serde_json::from_slice::<Vec<JsonValue>>(&buf) {
        return Ok(values);
    }
    if let Ok(value) = serde_json::from_slice::<JsonValue>(&buf) {
        if let Some(arr) = value.get("proxies").and_then(|v| v.as_array()) {
            return Ok(arr.clone());
        }
        if let Some(arr) = value.as_array() {
            return Ok(arr.clone());
        }
        if value.is_object() {
            return Ok(vec![value]);
        }
    }

    let yaml: serde_yml::Mapping = serde_yml::from_slice(&buf)
        .map_err(|e| anyhow::anyhow!("Failed to parse subscription as JSON or YAML: {e}"))?;
    let proxies: Vec<serde_yml::Value> = yaml
        .get("proxies")
        .and_then(|v| v.as_sequence())
        .cloned()
        .unwrap_or_default();
    let json_proxies: Vec<JsonValue> = proxies
        .into_iter()
        .map(|v| serde_json::to_value(v).unwrap_or(JsonValue::Null))
        .filter(|v| !v.is_null())
        .collect();
    Ok(json_proxies)
}

/// Expand a sing-box JSON template into a complete sing-box JSON config.
///
/// The template is a sing-box-style JSON object with extra template markers:
/// - `"tpl_param": {}` on proxy-provider objects marks them for URL expansion
/// - `"tpl_param": {"providers": ["pvd"]}` on proxy-groups marks groups for expansion
/// - `"<name>"` placeholders in `"outbounds"` or `"use"` lists
/// - `"rules"` as inline string array (mihomo-style, translated to sing-box)
pub async fn gen_template_singbox(
    tpl: &JsonValue,
    template_name: &str,
    urls: &[String],
    with_proxy: bool,
) -> anyhow::Result<JsonValue> {
    use std::collections::HashMap;

    let tpl_name = std::path::Path::new(template_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(template_name);

    // --- Download subscription URLs → proxy nodes ---
    let mut provider_proxies: HashMap<String, Vec<JsonValue>> = HashMap::new();
    let mut download_handles = Vec::new();
    for (i, url) in urls.iter().enumerate() {
        let url = url.clone();
        let pp_name = format!("pvd{i}");
        download_handles.push(tokio::task::spawn_blocking(move || {
            (pp_name, download_subscription(&url, with_proxy))
        }));
    }
    for handle in download_handles {
        let (pp_name, result) = handle.await?;
        match result {
            Ok(proxies) => {
                let tagged: Vec<JsonValue> = proxies
                    .into_iter()
                    .map(|mut proxy| {
                        if let Some(obj) = proxy.as_object_mut() {
                            if !obj.contains_key("tag") {
                                let tag = format!(
                                    "{pp_name}-{}",
                                    obj.get("server")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("node")
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
                log::warn!("Failed to download subscription for {pp_name}: {e}");
            }
        }
    }

    let mut pp_tags: HashMap<String, Vec<String>> = HashMap::new();
    for (pp_name, proxies) in &provider_proxies {
        let tags: Vec<String> = proxies
            .iter()
            .filter_map(|v| v.get("tag").and_then(|t| t.as_str()).map(String::from))
            .collect();
        pp_tags.insert(pp_name.clone(), tags);
    }

    // --- Build outbounds ---
    let mut outbounds: Vec<JsonValue> = Vec::new();
    for proxies in provider_proxies.values() {
        outbounds.extend(proxies.clone());
    }

    // --- Expand proxy-groups ---
    let pg_value = tpl
        .get(PROXY_GROUPS)
        .context("Missing proxy-groups section in template")?;
    let pg_sequence = pg_value
        .as_array()
        .context("proxy-groups must be an array")?;

    let mut pg_names: HashMap<String, Vec<String>> = HashMap::new();

    for the_pg_value in pg_sequence {
        // tpl_param can be null or an object with "providers"
        let has_tpl_param = the_pg_value.get("tpl_param").is_some_and(|v| !v.is_null());

        if !has_tpl_param {
            // Passthrough group
            let name = the_pg_value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let pg_type = the_pg_value
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("select");
            let sb_type = match pg_type {
                "select" => "selector",
                "url-test" => "urltest",
                "fallback" => "urltest",
                "load-balance" => "selector",
                _ => "selector",
            };

            let mut sb_group = serde_json::json!({
                "type": sb_type,
                "tag": name,
            });

            if let Some(us_) = the_pg_value.get("use") {
                if let Some(use_seq) = us_.as_array() {
                    let mut resolved: Vec<String> = Vec::new();
                    for u in use_seq {
                        let u_str = u.as_str().unwrap_or("");
                        if u_str.starts_with('<') && u_str.ends_with('>') {
                            let key = u_str.trim_matches(|c| c == '<' || c == '>');
                            if let Some(tags) = pp_tags.get(key) {
                                resolved.extend(tags.clone());
                            }
                        } else {
                            resolved.push(u_str.to_string());
                        }
                    }
                    sb_group["outbounds"] = serde_json::json!(resolved);
                }
            }

            if let Some(proxies) = the_pg_value.get("proxies") {
                if let Some(proxy_seq) = proxies.as_array() {
                    let mut resolved: Vec<String> = Vec::new();
                    for p in proxy_seq {
                        let p_str = p.as_str().unwrap_or("");
                        if p_str.starts_with('<') && p_str.ends_with('>') {
                            let key = p_str.trim_matches(|c| c == '<' || c == '>');
                            if let Some(names) = pg_names.get(key) {
                                resolved.extend(names.clone());
                            }
                        } else {
                            resolved.push(p_str.to_string());
                        }
                    }
                    if !resolved.is_empty() {
                        sb_group["outbounds"] = serde_json::json!(resolved);
                    }
                }
            }

            if sb_type == "urltest" {
                if let Some(url) = the_pg_value.get("url").and_then(|v| v.as_str()) {
                    sb_group["url"] = JsonValue::String(url.to_string());
                }
                if let Some(interval) = the_pg_value.get("interval").and_then(|v| v.as_u64()) {
                    sb_group["interval"] = JsonValue::String(interval_to_duration(interval));
                }
            }

            outbounds.push(sb_group);
            continue;
        }

        // Template group — expand via tpl_param.providers
        let pg_type = the_pg_value
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("select");
        let sb_type = match pg_type {
            "select" => "selector",
            "url-test" => "urltest",
            "fallback" => "urltest",
            "load-balance" => "selector",
            _ => "selector",
        };

        let provider_keys = the_pg_value["tpl_param"]
            .get("providers")
            .and_then(|v| v.as_array())
            .context("tpl_param.providers must be an array")?;

        let group_name = the_pg_value
            .get("name")
            .and_then(|v| v.as_str())
            .context("proxy-group must have a name")?;

        for the_provider_key in provider_keys {
            let pk_str = the_provider_key
                .as_str()
                .context("provider key in tpl_param.providers must be a string")?;

            for (pp_name, tags) in &pp_tags {
                if !pp_name.starts_with(pk_str) {
                    continue;
                }

                let new_group_name = format!("{group_name}-{pp_name}");

                let mut sb_group = serde_json::json!({
                    "type": sb_type,
                    "tag": new_group_name,
                    "outbounds": tags.clone(),
                });

                if sb_type == "urltest" {
                    if let Some(url) = the_pg_value.get("url").and_then(|v| v.as_str()) {
                        sb_group["url"] = JsonValue::String(url.to_string());
                    }
                    if let Some(interval) = the_pg_value.get("interval").and_then(|v| v.as_u64())
                    {
                        sb_group["interval"] =
                            JsonValue::String(interval_to_duration(interval));
                    }
                }

                pg_names
                    .entry(group_name.to_string())
                    .or_default()
                    .push(new_group_name);

                outbounds.push(sb_group);
            }
        }
    }

    // --- Build route section ---
    let mut route_rules: Vec<JsonValue> = Vec::new();
    let mut rule_sets: Vec<JsonValue> = Vec::new();
    let mut route_final: Option<String> = None;

    if let Some(rules) = tpl.get("rules") {
        if let Some(rules_seq) = rules.as_array() {
            for rule in rules_seq {
                let rule_str = rule.as_str().unwrap_or("");
                if rule_str.starts_with("MATCH") {
                    let target = rule_str
                        .splitn(2, ',')
                        .nth(1)
                        .map(|s| s.trim())
                        .unwrap_or("Proxy");
                    route_final = Some(target.to_string());
                } else if let Some(rule_json) = translate_rule(rule_str) {
                    route_rules.push(rule_json);
                }
            }
        }
    }

    if let Some(rps) = tpl.get("rule-providers") {
        if let Some(rps_map) = rps.as_object() {
            for (rp_name, rp_value) in rps_map {
                let url = rp_value
                    .get("url")
                    .and_then(|v| v.as_str());

                if let Some(url) = url {
                    rule_sets.push(serde_json::json!({
                        "tag": rp_name,
                        "type": "remote",
                        "format": "binary",
                        "url": url,
                    }));
                    route_rules.push(serde_json::json!({
                        "rule_set": rp_name,
                        "outbound": "Proxy",
                    }));
                }
            }
        }
    }

    let mut output = serde_json::json!({
        "outbounds": outbounds,
    });

    if !route_rules.is_empty() || route_final.is_some() || !rule_sets.is_empty() {
        let mut route = serde_json::json!({});
        if !route_rules.is_empty() {
            route["rules"] = JsonValue::Array(route_rules);
        }
        if !rule_sets.is_empty() {
            route["rule_set"] = JsonValue::Array(rule_sets);
        }
        if let Some(final_outbound) = route_final {
            route["final"] = JsonValue::String(final_outbound);
        }
        output["route"] = route;
    }

    output["clashtui_template_name"] = JsonValue::String(tpl_name.to_string());

    Ok(output)
}
