#[allow(unused)]
mod config;

use config::CtCfg;

use crate::backend::ProfileType;

pub fn migrate() -> anyhow::Result<()> {
    use crate::DataDir;
    let CtCfg {
        clash_cfg_dir,
        clash_bin_path,
        clash_cfg_path,
        clash_srv_name,
        is_user,
        timeout,
        edit_cmd,
        open_dir_cmd,
    } = CtCfg::load(&DataDir::get().join("config.yaml"))
        .map_err(|e| anyhow::anyhow!("Loading v0.2.3 config file: {e}"))?;
    let basic = super::Basic {
        clash_config_dir: clash_cfg_dir,
        clash_bin_path,
        clash_config_path: clash_cfg_path,
    };
    let service = super::Service {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        clash_service_name: clash_srv_name,
        #[cfg(target_os = "linux")]
        is_user,
    };
    let config = super::ConfigFile {
        basic,
        service,
        timeout,
        edit_cmd,
        open_dir_cmd,
        ..Default::default()
    };
    let pm = collect_profiles(DataDir::get())
        .map_err(|e| anyhow::anyhow!("Collecting profiles: {e}"))?;
    let basic_map = std::fs::read_to_string(DataDir::get().join("basic_clash_config.yaml"))
        .map_err(|e| anyhow::anyhow!("Loading basic clash config: {e}"))?;

    std::fs::remove_dir_all(DataDir::get())
        .map_err(|e| anyhow::anyhow!("Removing config dir: {e}"))?;
    super::BuildConfig::init_config().map_err(|e| anyhow::anyhow!("Initing config file: {e}"))?;

    use crate::consts::BASIC_PATH;
    config
        .to_file()
        .map_err(|e| anyhow::anyhow!("Saving new config file: {e}"))?;
    pm.to_file()
        .map_err(|e| anyhow::anyhow!("Saving new profiles database: {e}"))?;
    std::fs::write(BASIC_PATH.as_path(), basic_map)
        .map_err(|e| anyhow::anyhow!("Saving basic clash config: {e}"))?;
    Ok(())
}

/// collect profiles under `config_dir/profiles`,
/// and set the current profile to the one specified in `config_dir/data.yml`
///
/// 'no_pp' is ignored, all file under `config_dir/profiles` are treated as profile(url)
fn collect_profiles(config_dir: &std::path::Path) -> anyhow::Result<super::ProfileManager> {
    let profiles = super::ProfileManager::default();
    for entry in std::fs::read_dir(config_dir.join("profiles"))
        .map_err(|e| anyhow::anyhow!("iter /profiles: {e}"))?
    {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let path = entry.path();
            let name = entry.file_name();
            let content = std::fs::read_to_string(&path)?;
            profiles.insert(name.to_str().unwrap(), ProfileType::Url(content));
        }
    }
    let data_yml = std::fs::read_to_string(config_dir.join("data.yaml"))?;
    let data_yml: serde_yml::Mapping = serde_yml::from_str(&data_yml)?;
    profiles.set_current(
        profiles
            .get(data_yml.get("current_profile").unwrap().as_str().unwrap())
            .ok_or(anyhow::anyhow!(
                "current_profile not found in loaded profiles: {:?}",
                profiles.all()
            ))?,
    );
    Ok(profiles)
}
