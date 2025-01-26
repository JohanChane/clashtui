use profile::ProfileType;

use super::*;
#[cfg(feature = "tui")]
use crate::tui::tabs::profile::TemplateOp;
use crate::{
    utils::consts::{PROFILE_PATH, TEMPLATE_PATH},
    HOME_DIR,
};

impl BackEnd {
    pub fn get_all_templates(&self) -> std::io::Result<Vec<String>> {
        let dir_path = HOME_DIR.join(TEMPLATE_PATH);
        Ok(std::fs::read_dir(dir_path)?
            .collect::<std::io::Result<Vec<std::fs::DirEntry>>>()?
            .into_iter()
            .map(|p| {
                p.file_name()
                    .into_string()
                    .unwrap_or("Containing non UTF-8 char".to_owned())
            })
            .collect())
    }
    pub fn create_template(&self, path: String) -> anyhow::Result<Option<String>> {
        let path = std::path::PathBuf::from(path);
        let file = std::fs::File::open(&path)?;
        let map: serde_yml::Mapping = serde_yml::from_reader(file)?;
        match map
            .get("clashtui_template_version")
            .and_then(|v| v.as_u64())
        {
            None => {
                todo!("fallback")
            }
            Some(ver) if ver <= 1 => {
                // file is opened, so file_name should exist
                let name_maybe_with_ext = path.file_name().unwrap().to_str().unwrap();
                let name = name_maybe_with_ext
                    // remove the last one only
                    // e.g. this.tar.gz => this.tar
                    .rsplit_once('.')
                    .unwrap_or((name_maybe_with_ext, ""))
                    .0;
                std::fs::copy(&path, HOME_DIR.join(TEMPLATE_PATH).join(name))?;
                Ok(Some(format!(
                    "Name:{} Added\nClashtui Template Version {}",
                    // path from a String, should be UTF-8
                    name,
                    ver
                )))
            }
            Some(_) => unimplemented!(),
        }
    }
    pub fn apply_template(&self, name: String) -> anyhow::Result<()> {
        let path = HOME_DIR.join(TEMPLATE_PATH).join(&name);
        let file = std::fs::File::open(&path)
            .inspect_err(|e| log::debug!("Founding template {name}:{e}"))?;
        let map: serde_yml::Mapping = serde_yml::from_reader(file)?;
        match map
            .get("clashtui_template_version")
            .and_then(|v| v.as_u64())
        {
            None => {
                todo!("fallback")
            }
            Some(1) => {
                let gened = template_ver1(map, &name)?;
                let gened_name = format!("{name}.clashtui_generated");
                let path = HOME_DIR.join(PROFILE_PATH).join(&gened_name);
                serde_yml::to_writer(std::fs::File::create(path)?, &gened)?;
                self.pm.insert(gened_name, ProfileType::Generated(name));
            }
            Some(_) => unimplemented!(),
        }
        Ok(())
    }
}

fn template_ver1(
    mut tpl: serde_yml::Mapping,
    tpl_name: &str,
) -> anyhow::Result<serde_yml::Mapping> {
    macro_rules! expand {
        ($pats:pat, $exprs:expr) => {
            let $pats = $exprs else {
                anyhow::bail!(
                    "Failed to find {} in {}",
                    stringify!($pats),
                    stringify!($exprs)
                )
            };
        };
    }
    let local_urls = vec!["".to_owned()];
    // proxy-providers with proxy-groups
    let mut relation: std::collections::HashMap<serde_yml::Value, Vec<serde_yml::Value>> =
        std::collections::HashMap::new();
    // relationship between proxy-providers and proxy-groups
    {
        expand!(
            Some(serde_yml::Value::Sequence(pg)),
            tpl.remove("proxy-groups")
        );
        //  - name: "Sl"
        //    tpl_param:
        //      providers: ["pvd"]
        //    type: select
        for value in pg {
            if value.get("tpl_param").is_none() {
                relation
                    .entry(serde_yml::Value::Null)
                    .or_default()
                    .push(value);
                continue;
            }
            expand!(serde_yml::Value::Mapping(mut value), value);
            expand!(
                Some(serde_yml::Value::Mapping(mut param)),
                value.remove("tpl_param")
            );
            expand!(
                Some(serde_yml::Value::Sequence(pvds)),
                param.remove("providers")
            );
            for pvd in pvds {
                relation
                    .entry(pvd)
                    .or_default()
                    .push(serde_yml::Value::Mapping(value.clone()));
            }
        }
    }

    // proxy-providers
    {
        expand!(
            Some(serde_yml::Value::Mapping(pp)),
            tpl.remove("proxy-providers")
        );
        let mut extended_proxy_providers = serde_yml::Mapping::new();
        let mut extended_proxy_groups = serde_yml::Sequence::new();
        // proxy_provider_name:
        //   tpl_param:
        //   type: http
        for (key, value) in pp {
            if value.get("tpl_param").is_none() {
                continue;
            }
            // asserts
            expand!(serde_yml::Value::Mapping(mut content), value);
            expand!(serde_yml::Value::String(name), key);
            expand!(
                Some(pgs),
                relation.remove(&serde_yml::Value::String(name.clone()))
            );
            // remove marker
            content.remove("tpl_param");

            for (i, url) in local_urls.iter().enumerate() {
                use serde_yml::Value::String;
                let mut spp = content.clone();
                let proxy_provider_name = format!("{name}-{i}");
                spp.insert(String("url".to_string()), String(url.clone()));
                spp.insert(
                    String("path".to_string()),
                    String(format!(
                        "proxy-providers/tpl/{tpl_name}/{proxy_provider_name}.yaml"
                    )),
                );
                extended_proxy_providers.insert(
                    String(proxy_provider_name.clone()),
                    serde_yml::Value::Mapping(spp),
                );
                // proxy-groups
                for pg in pgs.clone() {
                    expand!(serde_yml::Value::Mapping(mut pg), pg);
                    expand!(Some(serde_yml::Value::String(pg_name)), pg.remove("name"));
                    pg.insert(
                        "name".into(),
                        format!("{pg_name}-{proxy_provider_name}").into(),
                    );
                    pg.insert("use".into(), vec![proxy_provider_name.clone()].into());
                    extended_proxy_groups.push(serde_yml::Value::Mapping(pg));
                }
            }
        }
        expand!(Some(pgs), relation.remove(&serde_yml::Value::Null));
        extended_proxy_groups.extend(pgs);
        for pg in &mut extended_proxy_groups {
            if let Some(providers) = pg.get("use") {
                let mut new_providers = Vec::new();
                for p in providers.as_sequence().unwrap() {
                    let p_str = p.as_str().unwrap();
                    if p_str.starts_with('<') && p_str.ends_with('>') {
                        let trimmed_p_str = p_str.trim_start_matches('<').trim_end_matches('>');
                        new_providers.extend(
                            extended_proxy_providers
                                .iter()
                                .map(|(k, _)| k.as_str().unwrap())
                                .filter(|n| n.starts_with(trimmed_p_str))
                                .map(|s| s.to_owned()),
                        );
                    } else {
                        new_providers.push(p_str.to_string());
                    }
                }
                pg["use"] = serde_yml::Value::Sequence(
                    new_providers
                        .into_iter()
                        .map(serde_yml::Value::String)
                        .collect(),
                );
            }
            if let Some(serde_yml::Value::Sequence(groups)) = pg.get("proxies") {
                let mut new_groups = Vec::new();
                for g in groups {
                    let g_str = g.as_str().unwrap();
                    if g_str.starts_with('<') && g_str.ends_with('>') {
                        let trimmed_p_str = g_str.trim_start_matches('<').trim_end_matches('>');
                        new_groups.extend(
                            extended_proxy_providers
                                .iter()
                                .map(|(k, _)| k.as_str().unwrap())
                                .filter(|n| n.starts_with(trimmed_p_str))
                                .map(|s| s.to_owned()),
                        );
                    } else {
                        new_groups.push(g_str.to_string());
                    }
                }
                pg["proxies"] = serde_yml::Value::Sequence(
                    new_groups
                        .into_iter()
                        .map(serde_yml::Value::String)
                        .collect(),
                );
            }
        }
        tpl.insert(
            "proxy-providers".into(),
            serde_yml::Value::Mapping(extended_proxy_providers),
        );
        tpl.insert(
            "proxy-groups".into(),
            serde_yml::Value::Sequence(extended_proxy_groups),
        );
    }
    Ok(tpl)
}

#[test]
fn ets() {
    let s = r#"proxy-anchor:
  - delay_test: &pa_dt {url: https://www.gstatic.com/generate_204, interval: 300}
  - proxy_provider: &pa_pp {interval: 3600, intehealth-check: {enable: true, url: https://www.gstatic.com/generate_204, interval: 300}}

proxy-groups:
  - name: "Entry"
    type: select
    proxies:
      - <At>                    # 使用 proxy-groups 中的 `At` 模板代理组。
      - <Sl>                    # 与 `<At>` 同理。

  - name: "Sl"                  # 定义名称是 `Sl` (名称可自定义) 的模板代理组。根据模板代理提供者 `pvd`, 会生成 `Sl-pvd0`, `Sl-pvd1`, ...
    tpl_param:
      providers: ["pvd"]        # 表示使用名称是 `pvd` 的模板代理提供者。
    type: select

  - name: "At"                  # 与 `Sl` 同理。
    tpl_param:
      providers: ["pvd"]
    type: url-test
    <<: *pa_dt

  - name: "Entry-RuleMode"        # 类似于黑白名单模式。用于控制有无代理都可以访问的网站使用代理或直连。
    type: select
    proxies:
      - DIRECT
      - Entry

  - name: "Entry-LastMatch"       # 设置不匹配规则的连接的入口。
    type: select
    proxies:
      - Entry
      - DIRECT

proxy-providers:
  pvd:             # 定义名称是 `pvd` (名称可自定义) 的模板代理提供者。会生成 `pvd0`, `pvd1`, ...
    tpl_param:
    type: http    # type 字段要放在此处, 不能放入 pp。原因是要用于更新资源。
    <<: *pa_pp

rules:
  #- IN-TYPE,INNER,DIRECT       # 设置 mihomo 内部的网络连接(比如: 更新 proxy-providers, rule-providers 等)是直连。
  - GEOIP,lan,DIRECT,no-resolve
  - GEOSITE,biliintl,Entry
  - GEOSITE,ehentai,Entry
  - GEOSITE,github,Entry
  - GEOSITE,twitter,Entry
  - GEOSITE,youtube,Entry
  - GEOSITE,google,Entry
  - GEOSITE,telegram,Entry
  - GEOSITE,netflix,Entry
  - GEOSITE,bilibili,Entry-RuleMode
  - GEOSITE,bahamut,Entry
  - GEOSITE,spotify,Entry
  - GEOSITE,geolocation-!cn,Entry
  - GEOIP,google,Entry
  - GEOIP,netflix,Entry
  - GEOIP,telegram,Entry
  - GEOIP,twitter,Entry
  - GEOSITE,pixiv,Entry
  - GEOSITE,CN,Entry-RuleMode
  - GEOIP,CN,Entry-RuleMode
  - MATCH,Entry-LastMatch"#;
    let p = serde_yml::from_str(s).unwrap();
    let p = template_ver1(p, "tpl_name").unwrap();
    println!("{}", serde_yml::to_string(&p).unwrap())
}
#[cfg(feature = "tui")]
impl BackEnd {
    pub(super) fn handle_template_op(&self, op: TemplateOp) -> CallBack {
        match op {
            TemplateOp::GetALL => match self.get_all_templates() {
                Ok(v) => CallBack::TemplateInit(v),
                Err(e) => CallBack::Error(e.to_string()),
            },
            TemplateOp::Add(path) => match self.create_template(path) {
                Ok(Some(str)) => CallBack::TemplateCTL(vec![str]),
                Ok(None) => {
                    CallBack::TemplateCTL(vec!["Not a valid clashtui template".to_string()])
                }
                Err(e) => CallBack::Error(e.to_string()),
            },
            TemplateOp::Remove(name) => {
                let path = HOME_DIR.join(TEMPLATE_PATH).join(&name);
                match std::fs::remove_file(path) {
                    Ok(()) => CallBack::TemplateCTL(vec![format!("{name} Removed")]),
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
            TemplateOp::Generate(name) => match self.apply_template(name) {
                Ok(()) => CallBack::TemplateCTL(vec![]),
                Err(e) => CallBack::Error(e.to_string()),
            },
            TemplateOp::Preview(name) => {
                let path = HOME_DIR.join(TEMPLATE_PATH).join(name);
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        CallBack::Preview(content.lines().map(|s| s.to_owned()).collect())
                    }
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
            TemplateOp::Edit(name) => {
                let path = HOME_DIR.join(TEMPLATE_PATH).join(name);
                match ipc::spawn(
                    "sh",
                    vec![
                        "-c",
                        self.edit_cmd.replace("%s", path.to_str().unwrap()).as_str(),
                    ],
                ) {
                    Ok(()) => CallBack::Edit,
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
        }
    }
}
