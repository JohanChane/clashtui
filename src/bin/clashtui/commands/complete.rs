use super::CliCmds;

pub fn gen_complete(shell: Option<clap_complete::Shell>) {
    use clap::CommandFactory;
    let gen = if let Some(gen) = shell {
        eprintln!("Target Shell: {gen}");
        gen
    } else {
        match clap_complete::shells::Shell::from_env() {
            Some(gen) => {
                eprintln!("Current Shell: {gen}");
                gen
            }
            None => {
                eprintln!("Unable to determine what shell this is");
                eprintln!("Try use --shell to specify");
                return;
            }
        }
    };
    clap_complete::generate(
        gen,
        &mut CliCmds::command(),
        // gen bin name by argv[0]
        std::env::args().next().unwrap(),
        &mut std::io::stdout(),
    )
}
