use super::CliCmds;

// CARGO_BIN_NAME is unknown at coding, but set to CARGO_PKG_NAME when building
const PKG_NAME: &str = env!("CARGO_PKG_NAME");
// const PKG_NAME: &str = env!("CARGO_BIN_NAME");

pub fn gen_complete(shell: Option<clap_complete::Shell>) {
    use clap::CommandFactory;
    let shell = if let Some(shell) = shell {
        eprintln!("Target Shell: {shell}");
        shell
    } else {
        match clap_complete::shells::Shell::from_env() {
            Some(shell) => {
                eprintln!("Current Shell: {shell}");
                shell
            }
            None => {
                eprintln!("Unable to determine what shell this is");
                eprintln!("Try use --generate-shell-completion=<your shell> to specify");
                eprintln!("type '{} --help' to get possible values", PKG_NAME);
                return;
            }
        }
    };
    clap_complete::generate(
        shell,
        &mut CliCmds::command(),
        PKG_NAME,
        &mut std::io::stdout(),
    )
}
