use super::*;
use serde_json::{Value, json};

fn primary() -> Value {
    serde_json::from_str(include_str!(
        "../../../../tests/apidata/mihomo/provider-groups.json"
    ))
    .unwrap()
}

fn providers() -> Value {
    serde_json::from_str(include_str!(
        "../../../../tests/apidata/mihomo/provider-nodes.json"
    ))
    .unwrap()
}

fn fetch_fixture(primary: Value, providers: Value) -> ProxyCatalog {
    fetch_proxies_with(|path| match path {
        "/proxies" => Ok(primary.clone()),
        "/providers/proxies" => Ok(providers.clone()),
        _ => panic!("unexpected endpoint {path}"),
    })
    .unwrap()
}

#[test]
fn provider_only_nodes_keep_their_source_and_history() {
    let catalog = fetch_fixture(primary(), providers());
    assert!(!catalog.primary.contains_key("node /日本"));
    let node = catalog.resolve("node /日本").unwrap();
    assert_eq!(node.provider, Some("subscription /一"));
    assert_eq!(node.proxy.history.last().unwrap().delay, 212);
    assert_eq!(
        node.test_url,
        Some("https://provider.example/check?q=a%20b&x=1")
    );
    assert!(node.proxy.udp && node.proxy.tcp);
    assert!(
        catalog
            .resolve("untested")
            .unwrap()
            .proxy
            .history
            .is_empty()
    );
    assert_eq!(catalog.resolve("failed").unwrap().proxy.history[0].delay, 0);
    assert_eq!(
        catalog.primary["Select"].test_url.as_deref(),
        Some("https://group.example/check")
    );
}

#[test]
fn provider_healthcheck_encodes_both_path_segments_and_nested_url() {
    let catalog = fetch_fixture(primary(), providers());
    let node = catalog.resolve("node /日本").unwrap();
    assert_eq!(
        proxy_delay_endpoint(&node.target(), node.test_url, 5000),
        "/providers/proxies/subscription%20%2F%E4%B8%80/node%20%2F%E6%97%A5%E6%9C%AC/healthcheck?url=https%3A%2F%2Fprovider.example%2Fcheck%3Fq%3Da%2520b%26x%3D1&timeout=5000"
    );
}

#[test]
fn compatible_aliases_do_not_redirect_inline_nodes_to_provider_routes() {
    let catalog = fetch_fixture(primary(), providers());
    let node = catalog.resolve("inline").unwrap();
    assert_eq!(node.provider, None);
    assert_eq!(
        proxy_delay_endpoint(&node.target(), None, 1000),
        "/proxies/inline/delay?url=https%3A%2F%2Fwww.gstatic.com%2Fgenerate_204&timeout=1000"
    );
}

#[test]
fn empty_healthcheck_url_falls_back_to_provider_then_default() {
    let mut response = providers();
    response["providers"]["subscription /一"]["proxies"][0]["testUrl"] = json!("");
    let catalog = fetch_fixture(primary(), response.clone());
    assert_eq!(
        catalog.resolve("node /日本").unwrap().test_url,
        Some("https://provider.example/check?q=a%20b&x=1")
    );
    response["providers"]["subscription /一"]["testUrl"] = json!("");
    let catalog = fetch_fixture(primary(), response);
    let node = catalog.resolve("node /日本").unwrap();
    assert_eq!(node.test_url, None);
    assert!(
        proxy_delay_endpoint(&node.target(), Some(""), 5000)
            .contains("url=https%3A%2F%2Fwww.gstatic.com%2Fgenerate_204&")
    );
}

#[test]
fn same_names_in_different_providers_remain_distinct_and_ambiguous() {
    let mut response = providers();
    response["providers"]["other"] = json!({
        "vehicleType": "File",
        "proxies": [{"name": "node /日本", "type": "Vmess", "id": "other-id",
                     "history": [{"delay": 999}]}]
    });
    let mut catalog = fetch_fixture(primary(), response);
    assert!(matches!(
        catalog.resolve("node /日本"),
        Err(LookupError::Ambiguous)
    ));
    let before = catalog.clone();
    catalog.record_group_delays(
        &before,
        "Select",
        &HashMap::from([("node /日本".to_owned(), 42)]),
    );
    assert_eq!(
        catalog.providers["subscription /一"].proxies[0].history[0].delay,
        212
    );
    assert_eq!(catalog.providers["other"].proxies[0].history[0].delay, 999);
    assert_eq!(catalog.providers["other"].proxies[0].history.len(), 1);
}

#[test]
fn primary_and_provider_name_collision_does_not_pick_primary_arbitrarily() {
    let mut response = providers();
    response["providers"]["other"] = json!({
        "vehicleType": "Inline",
        "proxies": [{"name": "inline", "type": "Vless", "id": "different-id"}]
    });
    let catalog = fetch_fixture(primary(), response);
    assert!(matches!(
        catalog.resolve("inline"),
        Err(LookupError::Ambiguous)
    ));
}

#[test]
fn old_flattened_api_is_deduplicated_by_id_or_provider_origin() {
    for with_id in [true, false] {
        let response = providers();
        let mut primary = primary();
        let mut node = response["providers"]["subscription /一"]["proxies"][0].clone();
        if !with_id {
            node.as_object_mut().unwrap().remove("id");
        }
        primary["proxies"]["node /日本"] = node;
        let catalog = fetch_fixture(primary, response);
        let resolved = catalog.resolve("node /日本").unwrap();
        assert_eq!(resolved.provider, Some("subscription /一"));
        assert_eq!(resolved.proxy.history[0].delay, 212);
    }
}

#[test]
fn successful_zero_delay_is_preserved_and_targets_only_its_provider() {
    let mut catalog = fetch_fixture(primary(), providers());
    let target = catalog.resolve("node /日本").unwrap().target();
    catalog.record_delay(&target, 0);
    assert_eq!(
        catalog
            .resolve("node /日本")
            .unwrap()
            .proxy
            .history
            .last()
            .unwrap()
            .delay,
        0
    );
    assert_eq!(catalog.primary["inline"].history[0].delay, 15);
}

#[test]
fn stale_results_do_not_apply_to_replaced_provider_nodes() {
    let before = fetch_fixture(primary(), providers());
    let target = before.resolve("node /日本").unwrap().target();
    let mut after = before.clone();
    after.providers["subscription /一"].proxies[0].id = Some("replacement-id".into());
    after.record_delay(&target, 999);
    after.record_group_delays(
        &before,
        "Select",
        &HashMap::from([("node /日本".to_owned(), 888)]),
    );
    assert_eq!(
        after.providers["subscription /一"].proxies[0].history.len(),
        1
    );
}

#[test]
fn group_results_update_only_unambiguous_members() {
    let before = fetch_fixture(primary(), providers());
    let mut after = before.clone();
    after.record_group_delays(
        &before,
        "Select",
        &HashMap::from([
            ("node /日本".to_owned(), 37),
            ("failed".to_owned(), 0),
            ("GLOBAL".to_owned(), 999),
        ]),
    );
    assert_eq!(
        after
            .resolve("node /日本")
            .unwrap()
            .proxy
            .history
            .last()
            .unwrap()
            .delay,
        37
    );
    assert_eq!(
        after
            .resolve("failed")
            .unwrap()
            .proxy
            .history
            .last()
            .unwrap()
            .delay,
        0
    );
    assert!(after.primary["GLOBAL"].history.is_empty());
}

#[test]
fn unsupported_provider_api_keeps_primary_nodes_usable() {
    let mut paths = Vec::new();
    let catalog = fetch_proxies_with(|path| {
        paths.push(path.to_owned());
        match path {
            "/proxies" => Ok(primary()),
            "/providers/proxies" => Err(minreq::Error::IoError(std::io::Error::other("HTTP 404"))),
            _ => panic!("unexpected endpoint"),
        }
    })
    .unwrap();
    assert_eq!(paths, ["/proxies", "/providers/proxies"]);
    assert!(catalog.providers.is_empty());
    assert!(catalog.resolve("inline").is_ok());
    assert!(matches!(
        catalog.resolve("node /日本"),
        Err(LookupError::Missing)
    ));
}

#[test]
fn invalid_provider_data_does_not_remove_primary_or_valid_providers() {
    for bad in [
        Value::Null,
        json!({"providers": []}),
        json!({"providers": {"bad": null}}),
    ] {
        let catalog = fetch_fixture(primary(), bad);
        assert!(catalog.resolve("inline").is_ok());
    }
    let mut response = providers();
    response["providers"]["bad"] = json!({"proxies": "invalid"});
    assert!(
        fetch_fixture(primary(), response)
            .resolve("node /日本")
            .is_ok()
    );
}

#[test]
fn primary_failure_is_not_masked_by_provider_data() {
    let mut paths = Vec::new();
    let response = fetch_proxies_with(|path| {
        paths.push(path.to_owned());
        Err(minreq::Error::IoError(std::io::Error::other("unavailable")))
    });
    assert!(response.is_err());
    assert_eq!(paths, ["/proxies"]);
}
