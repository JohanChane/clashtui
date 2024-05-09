use std::env;
use std::process::Command;

fn get_version() -> String {
    let git_describe = Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output();
    let mut version = match git_describe {
        Ok(v) => {
            String::from_utf8(v.stdout).expect("failed to read stdout").trim_end().to_string()
        }
        Err(err) => {
            eprintln!("`git describe` err: {}", err);

            let mut v = String::from("v");
            v.push_str(env::var("CARGO_PKG_VERSION").unwrap().as_str());
            v
        }
    };

    let build_type: bool = env::var("DEBUG").unwrap().parse().unwrap();
    version.push_str(if build_type {"-debug"} else {""});

    version
}

fn main() {
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/heads/dev");
    println!("cargo:rerun-if-changed=build.rs",);

    if let Ok(_) = env::var("CLASHTUI_VERSION") {
    } else {
        println!(
            "cargo:rustc-env=CLASHTUI_VERSION={}",
            get_version()
        );
    }
}
