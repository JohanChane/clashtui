use std::env;
use std::process::Command;

fn get_version() -> String {
    let branch_name = match Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
    {
        Ok(v) => String::from_utf8(v.stdout)
            .expect("failed to read stdout")
            .trim_end()
            .to_string(),
        Err(err) => {
            eprintln!("`git rev-parse` err: {}", err);
            "".to_string()
        }
    };

    let git_describe = match Command::new("git").args(["describe", "--always"]).output() {
        Ok(v) => String::from_utf8(v.stdout)
            .expect("failed to read stdout")
            .trim_end()
            .to_string(),
        Err(err) => {
            eprintln!("`git describe` err: {}", err);
            "".to_string()
        }
    };

    let cargo_pkg_version = env::var("CARGO_PKG_VERSION").unwrap();

    let build_type = if env::var("DEBUG").unwrap().parse().unwrap() {
        "-debug"
    } else {
        ""
    };

    let git_status = match Command::new("git").args(["status", "--short"]).output() {
        Ok(v) => {
            if v.stdout.is_empty() {
                "".to_owned()
            } else {
                "-dirty".to_owned()
            }
        }
        Err(e) => {
            eprintln!("`git status --short` err: {e}");
            "".to_owned()
        }
    };

    format!("v{cargo_pkg_version}-{branch_name}-{git_describe}{build_type}{git_status}")
}

fn main() {
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=build.rs",);

    if env::var("CLASHTUI_VERSION").is_err() {
        println!("cargo:rustc-env=CLASHTUI_VERSION={}", get_version());
    }
}
