use std::rc::Rc;

pub type SharedSymbols = Rc<Symbols>;

pub struct Symbols {
    pub profile: String,
    pub template: String,
    pub clashsrvctl: String,

    pub help: String,
    pub default_clash_cfg_content: String,
    pub default_basic_clash_cfg_content: String,
}

impl Default for Symbols {
    fn default() -> Self {
        let help = r#"## Profile
                        p: Switch to profile
                        enter: Select
                        u: Update proxy-providers only
                        U: Update all network resources in profile
                        i: Import
                        D: Delete
                        e: Edit
                        T: Test
                        P: Preview

                        ## Tempalte
                        t: Switch to template
                        enter: Create yaml
                        e: Edit
                        P: Preview

                        ## ClashSrvCtl
                        enter: Action

                        ## Scroll
                        j/k/h/l OR Up/Down/Left/Right: Scroll

                        ## Global
                        E: (rE)start clash service
                        R: Restart clash core
                        S: Stop clash service
                        H: locate app Home path
                        G: Locate clash config dir
                        L: show recent Log
                        1,2,...,9: Switch tab
                        Esc: Close popup
                        q: Quit
                        ?: help"#
            .to_string();
        let default_clash_cfg_content = r#"[default]
clash_core_path = ""
clash_cfg_dir = ""
clash_cfg_path = ""
clash_srv_name = "mihomo""#
            .to_string();
        let default_basic_clash_cfg_content = r#"mixed-port: 7890
mode: rule
log-level: info
external-controller: 127.0.0.1:9090"#
            .to_string();
        Self {
            profile: "Profile".to_string(),
            template: "Template".to_string(),
            clashsrvctl: "ClashSrvCtl".to_string(),

            help,
            default_clash_cfg_content,
            default_basic_clash_cfg_content,
        }
    }
}
