use std::env;
use std::process::Command;

fn get_version() -> String {
    let cargo_pkg_version = env::var("CARGO_PKG_VERSION").unwrap();

    let git_short_hash = match Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        Ok(v) => String::from_utf8(v.stdout)
            .expect("failed to read stdout")
            .trim_end()
            .to_string(),
        Err(err) => {
            eprintln!("`git rev-parse` err: {}", err);
            "unknown".to_string()
        }
    };

    let dirty = match Command::new("git")
        .args(["status", "--short"])
        .output()
    {
        Ok(v) => {
            if v.stdout.is_empty() {
                String::new()
            } else {
                "-dirty".to_string()
            }
        }
        Err(e) => {
            eprintln!("`git status --short` err: {e}");
            String::new()
        }
    };

    format!("{cargo_pkg_version}-{git_short_hash}{dirty}")
}

fn main() {
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=build.rs",);

    if env::var("CLASHTUI_VERSION").is_err() {
        println!("cargo:rustc-env=CLASHTUI_VERSION={}", get_version());
    }
}
