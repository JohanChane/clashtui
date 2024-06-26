use std::path::{Path, PathBuf};
mod impl_clashsrv;
mod impl_net;
mod impl_profile;

use crate::utils::config::{load_app_config, Config};
use api::ClashUtil;

pub struct ClashBackend {
    pub home_dir: PathBuf,
    pub clash_api: ClashUtil,
    pub cfg: Config,
}

// Misc
impl ClashBackend {
    pub fn new(clashtui_dir: &Path, is_inited: bool) -> (Self, Vec<anyhow::Error>) {
        let ( clash_api, cfg,warning) = load_app_config(clashtui_dir, is_inited)
            .expect("fatal error, fix them before continuing");

        let mut err_track = vec![];
        if let Some(e) = warning {
            err_track.push(e);
        }
        if let Err(e) = clash_api.version() {
            err_track.push(anyhow::anyhow!(
                "Fail to load config from clash core. Is it Running?"
            ));
            log::warn!("Fail to connect to clash:{e:?}")
        }
        (
            Self {
                home_dir: clashtui_dir.to_path_buf(),
                clash_api,
                cfg,
            },
            err_track,
        )
    }
}
