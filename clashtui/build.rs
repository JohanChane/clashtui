use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn get_git_version() -> String {
    let version = env::var("CARGO_PKG_VERSION").unwrap().to_string();

    let child = Command::new("git")
        .args(&["describe", "--always"])
        .output();
    match child {
        Ok(child) => {
            let buf = String::from_utf8(child.stdout).expect("failed to read stdout");
            buf
        },
        Err(err) => {
            eprintln!("`git describe` err: {}", err);
            version
        }
    }
}

fn main() {
    let version = get_git_version();
    println!("{version}");
    let mut f = File::create(
        Path::new(&env::var("OUT_DIR").unwrap())
            .join("VERSION")).unwrap();
    f.write_all(version.trim().as_bytes()).unwrap();
}