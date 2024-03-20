use std::path::{Path, PathBuf};

pub(super) fn get_file_names<P>(dir: P) -> std::io::Result<Vec<String>>
where
    P: AsRef<std::path::Path>,
{
    let mut file_names: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name() {
                file_names.push(file_name.to_string_lossy().to_string());
            }
        }
    }
    Ok(file_names)
}
/// Judging by format
pub(super) fn is_yaml(path: &std::path::Path) -> bool {
    std::fs::File::open(path).is_ok_and(|f| {
        serde_yaml::from_reader::<std::fs::File, serde_yaml::Value>(f).is_ok_and(|v| v.is_mapping())
    })
}

pub(super) fn parse_yaml(yaml_path: &std::path::Path) -> std::io::Result<serde_yaml::Value> {
    serde_yaml::from_reader(std::fs::File::open(yaml_path)?)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

pub fn get_mtime<P>(file_path: P) -> std::io::Result<std::time::SystemTime>
where
    P: AsRef<std::path::Path>,
{
    let file = std::fs::metadata(file_path)?;
    if file.is_file() {
        file.modified()
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Not a file?",
        ))
    }
}

pub fn str_duration(t: std::time::Duration) -> String {
    use std::time::Duration;
    if t.is_zero() {
        "Just Now".to_string()
    } else if t < Duration::from_secs(60 * 59) {
        let min = t.as_secs() / 60;
        format!("{}m", min + 1)
    } else if t < Duration::from_secs(3600 * 24) {
        let hou = t.as_secs() / 3600;
        format!("{hou}h")
    } else {
        let day = t.as_secs() / (3600 * 24);
        format!("{day}d")
    }
}