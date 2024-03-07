use std::env;
use std::process::Command;

fn get_git_version() -> String {
    let version = env::var("CARGO_PKG_VERSION").unwrap();

    let child = Command::new("git").args(["describe", "--always"]).output();
    match child {
        Ok(child) => String::from_utf8(child.stdout).expect("failed to read stdout"),
        Err(err) => {
            eprintln!("`git describe` err: {}", err);
            version
        }
    }
}

fn main() {
    let version = get_git_version();
    let build_type: bool = env::var("DEBUG").unwrap().parse().unwrap();
    let mut version = version.trim_end().to_owned();
    version.push_str(if build_type { "-debug" } else { "-release" });
    use std::io::Write;
    let io = std::io::stdout();
    writeln!(
        &io,
        "{}",
        format!("cargo:rustc-env=CLASHTUI_VERSION={}", version)
    )
    .unwrap();
    writeln!(&io, "cargo:rerun-if-changed=../.git/HEAD").unwrap();
    writeln!(&io, "cargo:rerun-if-changed=../.git/refs/heads/dev").unwrap();
    writeln!(&io, "cargo:rerun-if-changed=build.rs",).unwrap();
}
