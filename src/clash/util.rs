mod define_enum;

pub fn get_modify_time<P>(file_path: P) -> std::io::Result<std::time::SystemTime>
where
    P: AsRef<std::path::Path>,
{
    let file = std::fs::metadata(file_path)?;
    if file.is_file() {
        file.modified()
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Not a file",
        ))
    }
}
pub fn extract_domain(url: &str) -> Option<&str> {
    if let Some(protocol_end) = url.find("://") {
        let rest = &url[(protocol_end + 3)..];
        if let Some(path_start) = rest.find('/') {
            return Some(&rest[..path_start]);
        } else {
            return Some(rest);
        }
    }
    None
}

/*
pub fn timestamp_to_readable(timestamp: u64) -> String {
    let duration = std::time::Duration::from_secs(timestamp);
    let datetime = std::time::UNIX_EPOCH + duration;
    let datetime: chrono::DateTime<chrono::Utc> = datetime.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}
*/
