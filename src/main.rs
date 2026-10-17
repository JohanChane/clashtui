mod cli;
mod config;
mod functions;
mod tui;

fn main() {
    #[cfg(target_os = "linux")]
    nix::sys::stat::umask(nix::sys::stat::Mode::from_bits_truncate(0o002));

    let Some(cmd) = cli::from_env().handle_early_exit() else {
        return;
    };

    if let Err(e) = config::init(cmd.config_dir.clone()) {
        eprintln!("Failed to load Config\n{e}");
        return;
    }

    // Handle CLI subcommands (profile, service, mode, update)
    if cmd.command.is_some() {
        if let Err(e) = cli::handle_cli(cmd) {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
        config::CONFIG.save().unwrap();
        return;
    }

    let log_path = config::config_dir_path().join("clashtui.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("Failed to open log file");
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();

    tui::init().unwrap();

    // Never `unwrap` the TUI entry points below: a lost terminal (EIO) or any
    // other I/O error must not panic the process. With `panic = "abort"` a
    // panic here would abort and could leave the terminal unusable.
    if let Err(e) = tui::App::serve() {
        log::error!("TUI exited with error: {e}");
    }

    if let Err(e) = tui::restore() {
        log::error!("Failed to restore terminal: {e}");
    }

    if let Err(e) = config::CONFIG.save() {
        log::error!("Failed to save config: {e}");
    }
}
