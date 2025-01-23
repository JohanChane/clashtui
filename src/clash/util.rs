pub fn extract_domain(url: &str) -> Option<&str> {
    if let Some(protocol_end) = url.find("://") {
        let rest = &url[(protocol_end + 3)..];
        return if let Some(path_start) = rest.find('/') {
            Some(&rest[..path_start])
        } else {
            Some(rest)
        };
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
