pub(super) const HELP: &str = r#"## Common
j/k/h/l OR Up/Down/Left/Right: Scroll
Enter: Action
Esc: Close popup
Tab: Switch

## Profile Tab
p: Switch to profile
t: Switch to template

## Profile Window
Enter: Select
u: Update proxy-providers only
a: Update all network resources in profile
i: Import
d: Delete
s: Test
e: Edit
v: Preview
n: Info

## Tempalte
Enter: Create yaml
e: Edit
v: Preview

## ClashSrvCtl
Enter: Action

## Global
q: Quit
R: Restart clash core
L: Show recent log
I: Show informations
H: Locate app home path
G: Locate clash config dir
1,2,...,9 OR Tab: Switch tabs
?: Help"#;

pub(crate) const DEFAULT_BASIC_CLASH_CFG_CONTENT: &str = r#"mixed-port: 7890
mode: rule
log-level: info
external-controller: 127.0.0.1:9090"#;

pub(super) const PROFILE: &str = "Profile";
pub(super) const TEMPALTE: &str = "Template";
pub(super) const CLASHSRVCTL: &str = "ClashSrvCtl";
