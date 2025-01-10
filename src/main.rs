#![warn(clippy::all)]
#![deny(unsafe_code)]
#![warn(keyword_idents_2024)]
mod clash;
mod commands;
#[cfg(feature = "tui")]
mod tui;
mod utils;

use utils::{consts, BackEnd, BuildConfig};

static HOME_DIR: std::sync::LazyLock<std::path::PathBuf> = std::sync::LazyLock::new(load_home_dir);

fn main() -> anyhow::Result<()> {
    if is_root::is_root() {
        println!("{}", consts::ROOT_WARNING)
    }
    // Err here means to generate completion
    if let Ok(infos) = commands::parse_args() {
        // home dir is inited here
        let log_file = HOME_DIR.join(consts::LOG_FILE);
        setup_logging(&log_file);
        let buildconfig = match BuildConfig::load_config(HOME_DIR.as_path()) {
            Ok(v) => v,
            Err(e) => {
                // we don't really do so, as it can be dangerous
                if commands::Confirm::default()
                    .append_prompt(format!("failed to load config: {e}"))
                    .append_prompt("program will try to init default config")
                    .append_prompt(format!(
                        "WARNING! THIS WILL DELETE ALL FILE UNDER {}",
                        HOME_DIR.display()
                    ))
                    .append_prompt("Are you sure to continue?")
                    .interact()?
                {
                    // accept 'y' only
                    if let Err(e) = BuildConfig::init_config(HOME_DIR.as_path()) {
                        eprint!("init config failed: {e}");
                        std::process::exit(-1);
                    } else {
                        println!(
                            "Config Inited, please modify them to have clashtui work properly"
                        );
                    };
                } else {
                    println!("Abort");
                }
                std::process::exit(0);
            }
        };
        // build backend
        let backend = BackEnd::build(buildconfig).expect("failed to build Backend");
        // handle args
        if let Some(command) = infos {
            match commands::handle_cli(command, backend) {
                Ok(v) => {
                    println!("{v}")
                }
                Err(e) => {
                    eprintln!("clashtui encounter some error:{e}");
                    log::error!("Cli:{e:?}");
                    std::process::exit(-1)
                }
            }
        } else {
            #[cfg(feature = "tui")]
            {
                println!("Entering TUI...");
                if let Err(e) = start_tui(backend) {
                    eprintln!("clashtui encounter some error:{e}");
                    log::error!("Tui:{e:?}");
                    std::process::exit(-1)
                }
            }
            #[cfg(not(feature = "tui"))]
            eprintln!("use `--help/-h` for help")
        }
    } else {
        eprint!("generate completion success");
    }
    std::process::exit(0)
}
#[cfg(feature = "tui")]
// run a single thread, since there is no high-cpu-usage task
#[tokio::main(flavor = "current_thread")]
async fn start_tui(backend: BackEnd) -> anyhow::Result<()> {
    use tui::setup;
    if let Err(e) = tui::Theme::load(None) {
        anyhow::bail!("Theme: {e}")
    };
    let app = tui::FrontEnd::new();
    setup::setup()?;
    // make terminal restore after panic
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

fn load_home_dir() -> std::path::PathBuf {
    use std::{env, path};
    let data_dir = env::current_exe()
        .expect("Err loading exe_file_path")
        .parent()
        .expect("Err finding exe_dir")
        .join("data");
    if data_dir.exists() && data_dir.is_dir() {
        // portable mode
        data_dir
    } else {
        if cfg!(target_os = "linux") {
            env::var_os("XDG_CONFIG_HOME")
                .map(path::PathBuf::from)
                .or(env::var_os("HOME").map(|h| path::PathBuf::from(h).join(".config")))
        } else if cfg!(target_os = "windows") {
            env::var_os("APPDATA").map(path::PathBuf::from)
        } else if cfg!(target_os = "macos") {
            env::var_os("HOME").map(|h| path::PathBuf::from(h).join(".config"))
        } else {
            unimplemented!("Not supported platform")
        }
        .map(|c| c.join("clashtui"))
        .expect("failed to load home dir")
    }
}

fn setup_logging(log_file: &std::path::Path) {
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Root};
    use log4rs::encode::pattern::PatternEncoder;
    #[cfg(debug_assertions)]
    let _ = std::fs::remove_file(log_file); // auto rm old log for debug
    let flag = if std::fs::File::open(log_file)
        .and_then(|f| f.metadata())
        .is_ok_and(|m| m.len() > 1024 * 1024)
    {
        let _ = std::fs::remove_file(log_file);
        true
    } else {
        false
    };
    let log_level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    let file_appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%H:%M:%S)} [{l}] {t} - {m}{n}",
        ))) // Having a timestamp would be better.
        .build(log_file)
        .expect("Err opening log file");

    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(file_appender)))
        .build(Root::builder().appender("file").build(log_level))
        .expect("Err building log config");

    log4rs::init_config(config).expect("Err initing log service");

    log::info!("Start Log, level: {}", log_level);
    if flag {
        log::info!("Log file too large, clear")
    }
}
