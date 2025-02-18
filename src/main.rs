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

/// The clashtui config dir
struct DataDir;

fn main() {
    if is_root::is_root() {
        println!("{}", consts::ROOT_WARNING)
    }
    let Ok((infos, verbose_level)) = commands::parse_args() else {
        return;
    };
    let backend = match BuildConfig::load_config() {
        Ok(v) => BackEnd::build(v),
        Err(e) => reinit_config_dir(e),
    };
    utils::setup_logging(verbose_level);
    // handle commands
    if let Err(e) = match infos {
        Some(command) => commands::handle_cli(command, backend),
        #[cfg(not(feature = "tui"))]
        None => {
            eprintln!("use `--help/-h` for help");
            Ok(())
        }
        #[cfg(feature = "tui")]
        None => {
            println!("Entering TUI...");
            start_tui(backend)
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
    // load global theme
    tui::Theme::load();
    // enter raw mode
    setup::setup()?;
    // and recovery from it when panic
    setup::set_panic_hook();
    let frontend = tui::FrontEnd::new();
    let (backend_tx, app_rx) = tokio::sync::mpsc::channel(5);
    let (app_tx, backend_rx) = tokio::sync::mpsc::channel(5);
    let backend = tokio::spawn(backend.run(backend_tx, backend_rx));
    let frontend = tokio::spawn(frontend.run(app_tx, app_rx));
    let (frontend, backend) = tokio::try_join!(frontend, backend)?;
    setup::restore()?;
    // clear the result, save profiles to disk
    frontend?;
    backend?.to_file()?;
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
        .append_prompt("if you are upgrading from old version, please run `clashtui migrate`")
        .append_prompt("program will try to init default config")
        .append_prompt(format!(
            "WARNING! THIS WILL DELETE ALL FILE UNDER {}",
            DataDir::get().display()
        ))
        .append_prompt("Are you sure to continue?")
        .interact()
        .expect("Unable write/read from stdio")
    {
        // accept 'y' only
        if let Err(e) = BuildConfig::init_config() {
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
