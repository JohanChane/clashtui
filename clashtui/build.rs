use std::env;
//use std::process::Command;

/***
fn get_git_version() -> String {
    let version = env::var("CARGO_PKG_VERSION").unwrap();

    let child = Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output();
    match child {
        Ok(child) => String::from_utf8(child.stdout).expect("failed to read stdout"),
        Err(err) => {
            eprintln!("`git describe` err: {}", err);
            version
        }
    }
}
***/

fn main() {
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/heads/dev");
    println!("cargo:rerun-if-changed=build.rs",);

    if let Ok(_) = env::var("CLASHTUI_VERSION") {
    } else {
        // ## Use CARGO_PKG_VERSION as CLASHTUI_VERSION
        println!(
            "cargo:rustc-env=CLASHTUI_VERSION={}",
            env::var("CARGO_PKG_VERSION").unwrap(),
        );

        // ## Use git tag as CLASHTUI_VERSION
        //let version = get_git_version();
        //let mut version = version.trim_end().to_owned();
        ////let build_type: bool = env::var("DEBUG").unwrap().parse().unwrap();
        ////version.push_str(if build_type { "-debug" } else { "-release" });
        //println!("cargo:rustc-env=CLASHTUI_VERSION={}", version);
    }
}
