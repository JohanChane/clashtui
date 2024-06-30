use serde::Deserialize;
#[derive(Debug, Deserialize)]
pub struct ConnInfo {
    #[serde(rename = "downloadTotal")]
    pub download_total: u64,
    #[serde(rename = "uploadTotal")]
    pub upload_total: u64,
    pub connections: Option<Vec<Conn>>,
}

impl TryFrom<String> for ConnInfo {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&value).map_err(|e| format!("{e:?}"))
    }
}

#[derive(Debug, Deserialize)]
pub struct Conn {
    pub id: String,
    pub metadata: ConnMetaData,
    pub upload: u64,
    pub download: u64,
    pub start: String,
    pub chains: Vec<String>,
}

#[derive(Debug, Deserialize)]
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
