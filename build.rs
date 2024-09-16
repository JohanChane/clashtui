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

    let build_type: bool = env::var("DEBUG").unwrap().parse().unwrap();
    let build_type_str = if build_type { "debug" } else { "" };

    let version = format!("v{cargo_pkg_version}-{branch_name}-{git_describe}-{build_type_str}");

    version
}

fn main() {
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/heads/dev");
    println!("cargo:rerun-if-changed=build.rs",);

    if env::var("CLASHTUI_VERSION").is_err() {
        println!("cargo:rustc-env=CLASHTUI_VERSION={}", get_version());
    }
}
