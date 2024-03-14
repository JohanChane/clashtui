const MIHOMO: &str = "https://api.github.com/repos/MetaCubeX/mihomo/releases/latest";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

fn dl<S: Into<minreq::URL>>(url: S) -> minreq::ResponseLazy {
    minreq::get(url)
        .with_header("user-agent", USER_AGENT)
        .with_timeout(120)
        .send_lazy()
        .unwrap()
}
fn dl_mihomo<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<std::path::PathBuf> {
    let apinfo: super::GithubApi = serde_json::from_reader(dl(MIHOMO))?;

    let name = {
        // it should be ok to do at compile-time, since 64bit platform can run 32bit software
        #[cfg(target_os = "linux")]
        let os = "linux";
        #[cfg(target_os = "windows")]
        let os = "windows";
        #[cfg(target_arch = "x86_64")]
        let arch = "amd64";
        #[cfg(target_arch = "x86")]
        let arch = "386";
        let compat = false;
        let oldgo = false;
        let tag_name = apinfo.tag_name;
        let mut s = format!("mihomo-{os}-{arch}");
        if compat {
            s.push_str("-compatible")
        }
        if oldgo {
            s.push_str("-go120")
        }
        // mihomo-linux-amd64-compatible-go120-v1.18.1.gz
        format!("{s}-{tag_name}.gz")
    };
    let name = name.as_str();
    let path = {
        let path = std::path::Path::new(path.as_ref());
        if path.is_dir() {
            path.join(name)
        } else {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "not dir!"));
        }
    };

    if let Some(v) = apinfo.assets.iter().find(|s| s.name == name) {
        let mut fp = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&path)?;
        std::io::copy(&mut dl(&v.browser_download_url), &mut fp)?;
    };
    Ok(path.join(name))
}
#[test]
fn doit() {
    let cur = std::env::current_dir().unwrap();
    println!("{cur:?}");
    let worksapce = cur.parent().unwrap().parent().unwrap();
    let program = dl_mihomo(worksapce).unwrap();
    // TODO:unzip the file and chmod
    std::process::Command::new(program)
        .args([
            "-d",
            worksapce.join("Example").to_str().unwrap(),
            "-f",
            worksapce
                .join("Example")
                .join("basic_clash_config.yaml")
                .to_str()
                .unwrap(),
        ])
        .spawn()
        .unwrap();
}
