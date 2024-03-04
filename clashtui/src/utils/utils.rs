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

pub(super) fn parse_yaml(yaml_path: &std::path::Path) -> std::io::Result<serde_yaml::Value> {
    let mut file = std::fs::File::open(yaml_path)?;
    let mut yaml_content = String::new();
    std::io::Read::read_to_string(&mut file, &mut yaml_content)?;
    let parsed_yaml_content: serde_yaml::Value =
        serde_yaml::from_str(yaml_content.as_str()).unwrap();
    Ok(parsed_yaml_content)
}
