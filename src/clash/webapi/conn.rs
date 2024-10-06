use super::CResult;
use minreq::Method;
use serde::Deserialize;

use super::ClashUtil;
#[cfg_attr(test, derive(Debug))]
#[derive(Deserialize, Default)]
pub struct ConnInfo {
    #[serde(rename = "downloadTotal")]
    pub download_total: u64,
    #[serde(rename = "uploadTotal")]
    pub upload_total: u64,
    pub connections: Option<Vec<Conn>>,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Deserialize)]
pub struct Conn {
    pub id: String,
    pub metadata: ConnMetaData,
    pub upload: u64,
    pub download: u64,
    pub start: String,
    pub chains: Vec<String>,
}

#[allow(unused)]
#[cfg_attr(test, derive(Debug))]
#[derive(Deserialize)]
pub struct ConnMetaData {
    pub network: String,
    #[serde(rename = "type")]
    pub ctype: String,
    pub host: String,
    pub process: String,
    #[serde(rename = "processPath")]
    pub process_path: String,

    #[serde(rename = "sourceIP")]
    pub source_ip: String,
    #[serde(rename = "sourcePort")]
    pub source_port: String,
    #[serde(rename = "remoteDestination")]
    pub remote_destination: String,
    #[serde(rename = "destinationPort")]
    pub destinatio_port: String,
}

impl ClashUtil {
    /// returne [ConnInfo]
    pub fn get_connections(&self) -> CResult<ConnInfo> {
        self.request(Method::Get, "/connections", None)
            .and_then(|r| r.json())
            .map_err(|e| e.into())
    }
    /// if `id` is some, will try to terminate that connection,
    /// otherwise try to terminate all connections.
    ///
    /// ### Return: [bool]
    /// true on success
    ///
    /// > NOTE:
    /// > Empty str is returned if connection is terminated successfully
    pub fn terminate_connection(&self, id: Option<String>) -> CResult<bool> {
        self.request(
            Method::Delete,
            &format!(
                "/connections{}",
                id.map(|c| format!("/{c}")).unwrap_or_default()
            ),
            None,
        )
        .and_then(|r| {
            r.as_str().map(|s| {
                // try to catch failiure
                log::debug!("terminate conn:{s}");
                s.is_empty()
            })
        })
        .map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::ClashUtil;
    #[test]
    #[ignore = "no env on ci"]
    fn terminate_connection() {
        let sym = ClashUtil::build_test();
        let table = sym.get_connections().expect("Get connections failed");
        println!("BEFORE => {table:?}");
        let ret = sym
            .terminate_connection(Some(table.connections.unwrap()[0].id.clone()))
            .unwrap();
        println!("RESULT => {ret:?}");
        let table = sym.get_connections().expect("Get connections failed");
        println!("AFTER => {table:?}");
        let ret = sym.terminate_connection(None).unwrap();
        println!("RESULT => {ret:?}");
        let table = sym.get_connections().expect("Get connections failed");
        println!("AFTER => {table:?}");
    }
}
