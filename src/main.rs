mod cli;
mod config;
mod functions;
mod tui;

fn main() {
    #[cfg(target_os = "linux")]
    nix::sys::stat::umask(nix::sys::stat::Mode::from_bits_truncate(0o002));;

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

    tui::App::serve().unwrap();

    tui::restore().unwrap();

    config::CONFIG.save().unwrap();
}
