pub fn concat_update_profile_result(result: (Vec<String>, Vec<String>)) -> Vec<String> {
    let suc = result.0.iter().map(|v| format!("Updated: {v}"));
    result
        .1
        .iter()
        .map(|v| format!("Not Updated: {v}"))
        .chain(suc)
        .collect()
}

pub fn get_file_names(dir: &std::path::Path) -> Result<Vec<String>, std::io::Error> {
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
pub fn is_yaml(path: &std::path::Path) -> bool {
    std::fs::File::open(path).is_ok_and(|f| {
        serde_yaml::from_reader::<std::fs::File, serde_yaml::Value>(f).is_ok_and(|v| v.is_mapping())
    })
}
