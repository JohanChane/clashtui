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

    if let Err(e) = config::init(cmd.config_dir) {
        eprintln!("Failed to load Config\n{e}");
        return;
    }

    tui::init().unwrap();

    tui::App::serve().unwrap();

    tui::restore().unwrap();

    config::CONFIG.save().unwrap();
}
