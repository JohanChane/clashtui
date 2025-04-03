use crate::consts::PKG_NAME;

use super::CliCmds;

pub fn gen_complete(shell: Option<clap_complete::Shell>) {
    use clap::CommandFactory;
    let Some(shell) = shell
        .inspect(|target| eprintln!("Target Shell is {}", target))
        .or(clap_complete::Shell::from_env())
        .inspect(|detected| eprintln!("Detected Shell is {}", detected))
    else {
        eprintln!("Unable to determine which shell you are running");
        eprintln!("Try use --generate-shell-completion=<your shell> to specify");
        eprintln!("run '{} --help' to get possible values", PKG_NAME);
        return;
    };
    clap_complete::generate(
        shell,
        &mut CliCmds::command(),
        PKG_NAME,
        &mut std::io::stdout(),
    )
}
