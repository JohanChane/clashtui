use std::rc::Rc;

pub type SharedSymbols = Rc<Symbols>;

pub struct Symbols {
    pub profile: String,
    pub template: String,
    pub clashsrvctl: String,

    pub help: String,
}

impl Default for Symbols {
    fn default() -> Self {
        let help = r#"## Profile
                        p: Switch profile
                        enter: Select
                        u: Update proxy-providers only
                        U: Update all network resources in profile
                        i: Import
                        D: Delete
                        e: Edit
                        T: Test

                        ## Tempalte
                        t: Switch template
                        enter: Create yaml
                        e: Edit

                        ## ClashSrvCtl
                        enter: Action

                        ## Scroll
                        j/k/h/l OR Up/Down/Left/Right: Scroll

                        ## Global
                        R: Start clash service
                        S: Stop clash service
                        P: Preview
                        H: Locate app home path
                        L: show recent log
                        1,2,...,9: Switch tab
                        Esc: Close popup
                        q: Quit
                        ?: help"#
            .to_string();
        Self {
            profile: "Profile".to_string(),
            template: "Template".to_string(),
            clashsrvctl: "ClashSrvCtl".to_string(),

            help,
        }
    }
}
