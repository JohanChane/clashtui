mod define_enum;

// pub fn get_file_names<P>(dir: P) -> std::io::Result<Vec<String>>
// where
//     P: AsRef<std::path::Path>,
// {
//     let mut file_names: Vec<String> = Vec::new();

//     for entry in std::fs::read_dir(dir)? {
//         let path = entry?.path();
//         if path.is_file() {
//             if let Some(file_name) = path.file_name() {
//                 file_names.push(file_name.to_string_lossy().to_string());
//             }
//         }
//     }
//     Ok(file_names)
// }
/// Judging by format
// pub(crate) fn is_yaml(path: &std::path::Path) -> bool {
//     std::fs::File::open(path).is_ok_and(|f| {
//         serde_yaml::from_reader::<std::fs::File, serde_yaml::Value>(f).is_ok_and(|v| v.is_mapping())
//     })
// }

// pub(crate) fn parse_yaml(yaml_path: &std::path::Path) -> std::io::Result<serde_yaml::Value> {
//     serde_yaml::from_reader(std::fs::File::open(yaml_path)?)
//         .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
// }

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
