#![warn(clippy::all)]
mod commands;
mod tui;
mod utils;

use utils::{consts, init_config, load_config, BackEnd, Flag, Flags};

fn main() {
    if is_root::is_root() {
        println!("{}", consts::ROOT_WARNING)
    }
    // Err here means to generate completion
    if let Ok(infos) = commands::parse_args() {
        // store pre-setup flags
        let mut flags = Flags::empty();
        // setup home dir
        let home_dir = load_home_dir(&mut flags);

        setup_logging(home_dir.join(consts::LOG_FILE));
        // pre-setup flags are done here
        let flags = flags;
        log::debug!("Current flags: {:?}", flags);
        let buildconfig = match load_config(&home_dir) {
            Ok(v) => v,
            Err(e) => {
                use std::io::{Read, Write};
                eprintln!("failed to load config: {e}");
                println!("program will try to init default config");
                println!(
                    "WARNING! THIS WILL DELETE ALL FILE UNDER {}",
                    home_dir.to_string_lossy()
                );
                // we don't really do so, as it can be dangerous
                print!("Are you sure to continue[y/n]?");
                std::io::stdout().flush().unwrap();
                let mut buf = [0_u8; 1];
                let rep = std::io::stdin().read(&mut buf);
                if rep.is_ok_and(|l| l != 0) && buf[0] == b'y' {
                    // accept 'y' only
                    if let Err(e) = init_config(&home_dir) {
                        eprint!("init config failed: {e}");
                    };
                } else {
                    println!("Abort");
                    std::process::exit(0);
                }
                load_config(home_dir).unwrap()
            }
        };
        // build backend
        let backend = BackEnd::try_from(buildconfig).expect("failed to build Backend");
        // handle args
        if let Some(command) = infos {
            match commands::handle_cli(command, backend) {
                Ok(v) => {
                    println!("{v}")
                }
                Err(e) => {
                    eprintln!("clashtui encounter some error");
                    eprintln!("{e}");
                    log::error!("Cli:{e:?}");
                    std::process::exit(-1)
                }
            }
        } else {
            println!("Entering TUI...");
            if let Err(e) = start_tui(backend) {
                eprintln!("clashtui encounter some error");
                eprintln!("{e}");
                log::error!("Tui:{e:?}");
                std::process::exit(-1)
            }
        }
    } else {
        eprint!("generate completion success");
    }
    std::process::exit(0)
}
// run a single thread, since there is no high-cpu-usage task
#[tokio::main(flavor = "current_thread")]
async fn start_tui(backend: BackEnd) -> anyhow::Result<()> {
    use tui::setup;
    tui::Theme::load(None)?;
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
    let (b, a) = tokio::try_join!(app, backend)?;
    b?;
    a?;
    setup::restore()?;
    Ok(())
}

fn load_home_dir(flags: &mut Flags<Flag>) -> std::path::PathBuf {
    let config_dir = {
        use std::{env, path::PathBuf};
        let exe_dir = env::current_exe()
            .expect("Err loading exe_file_path")
            .parent()
            .expect("Err finding exe_dir")
            .to_path_buf();
        let data_dir = exe_dir.join("data");
        if data_dir.exists() && data_dir.is_dir() {
            // portable mode
            flags.insert(Flag::PortableMode);
            data_dir
        } else {
            #[cfg(target_os = "linux")]
            let config_dir_str = env::var("XDG_CONFIG_HOME")
                .or_else(|_| env::var("HOME").map(|home| format!("{}/.config/clashtui", home)));
            #[cfg(target_os = "windows")]
            let config_dir_str = env::var("APPDATA").map(|appdata| format!("{}/clashtui", appdata));
            #[cfg(target_os = "macos")]
            let config_dir_str = env::var("HOME").map(|home| format!("{}/.config/clashtui", home));
            PathBuf::from(&config_dir_str.expect("Err loading global config dir"))
        }
    };
    config_dir
}

fn setup_logging<P: AsRef<std::path::Path>>(log_path: P) {
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Root};
    use log4rs::encode::pattern::PatternEncoder;
    #[cfg(debug_assertions)]
    let _ = std::fs::remove_file(&log_path); // auto rm old log for debug
    let flag = if std::fs::File::open(&log_path)
        .and_then(|f| f.metadata())
        .is_ok_and(|m| m.len() > 1024 * 1024)
    {
        let _ = std::fs::remove_file(&log_path);
        true
    } else {
        false
    };
    // No need to change. This is set to auto switch to Info level when build release
    #[allow(unused_variables)]
    let log_level = log::LevelFilter::Info;
    #[cfg(debug_assertions)]
    let log_level = log::LevelFilter::Debug;
    let file_appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%H:%M:%S)} [{l}] {t} - {m}{n}",
        ))) // Having a timestamp would be better.
        .build(log_path)
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
