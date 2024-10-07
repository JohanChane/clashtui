use super::CliCmds;

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
                eprintln!("Try use --shell to specify");
                return;
            }
        }
    };
    clap_complete::generate(
        shell,
        &mut CliCmds::command(),
        "clashtui",
        &mut std::io::stdout(),
    )
}
