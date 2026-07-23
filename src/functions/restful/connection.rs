use super::*;

use serde::Deserialize;

#[cfg_attr(test, derive(Debug))]
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ConnInfo {
    pub download_total: u64,
    pub upload_total: u64,
    pub connections: Option<Vec<Conn>>,
}

#[cfg_attr(test, derive(Debug, Clone))]
#[derive(Deserialize)]
pub struct Conn {
    pub id: String,
    pub metadata: ConnMetaData,
    pub upload: u64,
    pub download: u64,
    // #[allow(dead_code)]
    // pub start: String,
    pub chains: Vec<String>,
    #[serde(default)]
    pub rule: Option<String>,
    // #[allow(dead_code)]
    // #[serde(default, rename = "rulePayload")]
    // pub rule_payload: Option<String>,
}

#[cfg_attr(test, derive(Debug, Clone))]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnMetaData {
    #[cfg_attr(not(test), allow(dead_code))]
    pub network: String,
    // #[serde(rename = "type", default)]
    // #[allow(dead_code)]
    // pub ctype: String,
    pub host: String,
    // #[serde(default)]
    // #[allow(dead_code)]
    // pub process: String,
    // #[serde(default)]
    // #[cfg_attr(not(test), allow(dead_code))]
    // pub process_path: String,

    // #[serde(rename = "sourceIP")]
    // #[allow(dead_code)]
    // pub source_ip: String,
    // #[allow(dead_code)]
    // pub source_port: String,
    #[serde(default)]
    pub remote_destination: String,
    #[serde(default, rename = "destinationPort")]
    pub destination_port: String,
    #[serde(default, rename = "destinationIP")]
    pub destination_ip: Option<String>,
    // #[allow(dead_code)]
    // #[serde(default, rename = "sniffHost")]
    // pub sniff_host: Option<String>,
}

/// return [ConnInfo]
pub fn get_connections() -> Result<ConnInfo> {
    request(Method::Get, "/connections", None).and_then(|r| r.json())
}

/// if `id` is some, will try to terminate that connection,
/// otherwise try to terminate **all** connections.
pub fn terminate_connection(id: Option<String>) -> Result<()> {
    request_return_204(
        Method::Delete,
        &format!(
            "/connections{}",
            id.map(|c| format!("/{c}")).unwrap_or_default()
        ),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::connection::*;

    fn load_singbox_connections() -> ConnInfo {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/apidata/sing-box/connections.json"
        );
        let data = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&data).unwrap()
    }

    #[test]
    fn singbox_conninfo_totals() {
        let info = load_singbox_connections();
        assert!(info.download_total > 0);
        assert!(info.upload_total > 0);
    }

    #[test]
    fn singbox_connections_has_entries() {
        let info = load_singbox_connections();
        let conns = info.connections.expect("connections missing");
        assert!(!conns.is_empty());
    }

    #[test]
    fn singbox_conn_has_chains() {
        let info = load_singbox_connections();
        let conn = &info.connections.unwrap()[0];
        assert!(!conn.chains.is_empty());
    }

    // #[test]
    // fn singbox_conn_metadata_empty_process_path() {
    //     let info = load_singbox_connections();
    //     for conn in info.connections.unwrap() {
    //         assert_eq!(conn.metadata.process_path, "");
    //     }
    // }

    #[test]
    fn singbox_conn_rule_is_some() {
        let info = load_singbox_connections();
        for conn in info.connections.unwrap() {
            assert!(conn.rule.is_some());
        }
    }

    #[test]
    fn singbox_conn_udp_connection() {
        let info = load_singbox_connections();
        let conns = info.connections.unwrap();
        let udp = conns
            .iter()
            .find(|c| c.metadata.network == "udp")
            .expect("UDP connection missing");
        assert_eq!(udp.metadata.destination_port, "53");
        assert_eq!(udp.metadata.host, "");
    }

    #[test]
    fn singbox_conn_has_destination_ip() {
        let info = load_singbox_connections();
        let conns = info.connections.unwrap();
        assert!(conns[0].metadata.destination_ip.is_some());
    }
}
