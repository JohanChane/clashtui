#![warn(clippy::all)]
#![deny(unsafe_code)]
#![warn(keyword_idents_2024)]
mod backend;
mod clash;
mod commands;
#[cfg(feature = "tui")]
mod tui;
mod utils;

use backend::BackEnd;
use utils::{consts, BuildConfig};

static HOME_DIR: std::sync::LazyLock<std::path::PathBuf> = std::sync::LazyLock::new(|| {
    if let Some(data_dir) = commands::_HOME_DIR.get() {
        if data_dir.exists() && data_dir.is_dir() {
            match std::path::absolute(data_dir) {
                Ok(dir) => return dir,
                Err(e) => {
                    log::error!("Cannot locate absolute path:{e}");
                    log::error!("Update profile may not work");
                    return data_dir.clone();
                }
            }
        }
    };
    utils::load_home_dir()
});

fn main() {
    if is_root::is_root() {
        println!("{}", consts::ROOT_WARNING)
    }
    let Ok((infos, verbose)) = commands::parse_args() else {
        return;
    };
    let backend = match BuildConfig::load_config(&HOME_DIR) {
        Ok(v) => BackEnd::build(v),
        Err(e) => reinit_config_dir(e),
    };
    utils::setup_logging(&HOME_DIR.join(consts::LOG_FILE), verbose);
    // handle commands
    if let Err(e) = match infos {
        Some(command) => commands::handle_cli(command, backend),
        None => {
            #[cfg(feature = "tui")]
            {
                println!("Entering TUI...");
                start_tui(backend)
            }
            #[cfg(not(feature = "tui"))]
            {
                eprintln!("use `--help/-h` for help");
                Ok(())
            }
        }
    } {
        eprintln!("clashtui encounter some error: {e}");
        log::error!("Err: {e}");
        std::process::exit(-1)
    };
}

/// running in single thread, since there is no high-cpu-usage task
#[cfg(feature = "tui")]
#[tokio::main(flavor = "current_thread")]
async fn start_tui(backend: BackEnd) -> anyhow::Result<()> {
    use tui::setup;
    if let Err(e) = tui::Theme::load(None) {
        anyhow::bail!("Theme loading: {e}")
    };
    setup::setup()?;
    let app = tui::FrontEnd::new();
    // make terminal restorable after panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = setup::restore();
        original_hook(panic);
    }));
    let (backend_tx, app_rx) = tokio::sync::mpsc::channel(5);
    let (app_tx, backend_rx) = tokio::sync::mpsc::channel(5);
    let backend = tokio::spawn(backend.run(backend_tx, backend_rx));
    let app = tokio::spawn(app.run(app_tx, app_rx));
    let (a, b) = tokio::try_join!(app, backend)?;
    setup::restore()?;
    a?;
    b?.to_file(HOME_DIR.join(consts::DATA_FILE))?;
    Ok(())
}

/// function to handle error when loading config
/// and init default config
///
/// it never returns
///
/// # Panics
/// if unable to write/read from stdio
fn reinit_config_dir(err: impl std::fmt::Display) -> ! {
    // we don't really do so, as it can be dangerous
    if commands::Confirm::default()
        .append_prompt(format!("failed to load config: {err}"))
        .append_prompt(if cfg!(feature = "migration") {
            "if you are upgrading from old version, please run `clashtui migrate`"
        } else {
            ""
        })
        .append_prompt("program will try to init default config")
        .append_prompt(format!(
            "WARNING! THIS WILL DELETE ALL FILE UNDER {}",
            HOME_DIR.display()
        ))
        .append_prompt("Are you sure to continue?")
        .interact()
        .expect("Unable write/read from stdio")
    {
        // accept 'y' only
        if let Err(e) = BuildConfig::init_config(HOME_DIR.as_path()) {
            eprint!("init config failed: {e}");
            std::process::exit(-1);
        } else {
            println!("Config Inited, please modify them to have clashtui work properly");
        };
    } else {
        println!("Abort");
    }
    std::process::exit(0);
}
