pub const HELP: &str = r#"## Profile
p: Switch to profile
enter: Select
u: Update proxy-providers only
U: Update all network resources in profile
i: Import
d: Delete
e: Edit
t: Test
p: Preview

## Tempalte
T: Switch to template
enter: Create yaml
e: Edit
p: Preview

## ClashSrvCtl
enter: Action

## Scroll
j/k/h/l OR Up/Down/Left/Right: Scroll

## Global
R: Restart clash core
H: Locate app home path
G: Locate clash config dir
L: show recent log
1,2,...,9 OR Tab: Switch tabs
Esc: Close popup
Q: Quit
?: help"#;

pub const DEFAULT_BASIC_CLASH_CFG_CONTENT: &str = r#"mixed-port: 7890
mode: rule
log-level: info
external-controller: 127.0.0.1:9090"#;

pub const PROFILE: &str = "Profile";
pub const TEMPALTE: &str = "Template";
pub const CLASHSRVCTL: &str = "ClashSrvCtl";
pub const CONFIG: &str = "Config";
