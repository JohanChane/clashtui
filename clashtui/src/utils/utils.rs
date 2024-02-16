pub fn concat_update_profile_result(
    result: (Vec<(String, String)>, Vec<(String, String)>),
) -> Vec<String> {
    let (updated_res, not_updated_res) = result;
    let mut concatenated_result = Vec::new();

    for (url, path) in not_updated_res {
        concatenated_result.push(format!("Not Updated: {} -> {}", url, path));
    }

    for (url, path) in updated_res {
        concatenated_result.push(format!("Updated: {} -> {}", url, path));
    }

    concatenated_result
}

pub fn get_file_names(dir: &std::path::Path) -> Result<Vec<String>, std::io::Error> {
    let mut file_names: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name() {
                if let Some(file_name_str) = file_name.to_str() {
                    file_names.push(file_name_str.to_string());
                }
            }
        }
    }

    Ok(file_names)
}
