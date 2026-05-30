#[allow(unused)]
mod config;

use config::CtCfg;

use super::database::ProfileType;

pub fn migrate() -> anyhow::Result<()> {
    let old_data_dir = super::util::load_home_dir()?;

    let CtCfg {
        clash_cfg_dir,
        clash_bin_path,
        clash_cfg_path,
        clash_srv_name,
        is_user,
        timeout,
        edit_cmd,
        open_dir_cmd,
    } = CtCfg::load(&old_data_dir.join("config.yaml"))
        .map_err(|e| anyhow::anyhow!("Loading v0.3.0 config file: {e}"))?;

    let basic_map = std::fs::read_to_string(old_data_dir.join("basic_clash_config.yaml"))
        .map_err(|e| anyhow::anyhow!("Loading basic clash config: {e}"))?;

    let pm = collect_profiles(&old_data_dir)
        .map_err(|e| anyhow::anyhow!("Collecting profiles: {e}"))?;

    std::fs::remove_dir_all(&old_data_dir)
        .map_err(|e| anyhow::anyhow!("Removing config dir: {e}"))?;

    super::init(Some(old_data_dir.clone()))?;

    let config = super::core::ConfigFile {
        mihomo: super::core::MihomoSection {
            core: super::core::CoreConfig {
                config_dir: clash_cfg_dir,
                bin_path: clash_bin_path,
                config_path: clash_cfg_path,
            },
            core_service: super::core::CoreServiceConfig {
                service_name: clash_srv_name,
                is_user,
            },
        },
        singbox: super::core::SingboxSection::default(),
        timeout,
        extra: super::core::Extra {
            edit_cmd: (!edit_cmd.is_empty()).then_some(edit_cmd),
            open_dir_cmd: (!open_dir_cmd.is_empty()).then_some(open_dir_cmd),
        },
    };

    config
        .to_file()
        .map_err(|e| anyhow::anyhow!("Saving new config file: {e}"))?;
    pm.to_file()
        .map_err(|e| anyhow::anyhow!("Saving new profiles database: {e}"))?;

    let mihomo_override = super::config_dir_path()
        .join("mihomo")
        .join(super::util::defs::CORE_OVERRIDE_FILE);
    std::fs::write(&mihomo_override, basic_map)
        .map_err(|e| anyhow::anyhow!("Saving basic clash config: {e}"))?;
    Ok(())
}

/// collect profiles under `config_dir/profiles`,
/// and set the current profile to the one specified in `config_dir/data.yml`
///
/// 'no_pp' is ignored, all file under `config_dir/profiles` are treated as profile(url)
fn collect_profiles(config_dir: &std::path::Path) -> anyhow::Result<super::database::ProfileManager> {
    let mut profiles = super::database::ProfileManager::default();
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
