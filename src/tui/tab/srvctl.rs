use super::dev::*;
use crate::config::CoreType;
#[cfg(unix)]
use libc;
use ratatui::style::Color;
use ratatui::widgets::ListItem;

newtype_tab!(CoreSrvCtlTab(Tab<SrvCtlContent>));

mod_agent!(
    SrvCtlKey,
    [
        ([KeyCode::Enter], SrvCtlKey::Execute, "Execute"),
        ([KeyCode::Esc], SrvCtlKey::Esc, "Back"),
        ([KeyCode::Up], SrvCtlKey::MoveUp, "Move up"),
        ([KeyCode::Down], SrvCtlKey::MoveDown, "Move down"),
        ([KeyCode::Char('k')], SrvCtlKey::MoveUp, "Move up"),
        ([KeyCode::Char('j')], SrvCtlKey::MoveDown, "Move down"),
    ]
);

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub(crate) enum SrvCtlKey {
    Execute,
    MoveUp,
    MoveDown,
    Esc,
}

impl TryFrom<&crate::tui::Key> for SrvCtlKey {
    type Error = ();

    fn try_from(ev: &crate::tui::Key) -> Result<Self, Self::Error> {
        let agent = agent();
        if !agent.is_empty() {
            return agent.get(ev).map(|k| *k).ok_or(());
        }
        Ok(match ev.code {
            KeyCode::Enter => Self::Execute,
            KeyCode::Esc => Self::Esc,
            KeyCode::Up | KeyCode::Char('k') => Self::MoveUp,
            KeyCode::Down | KeyCode::Char('j') => Self::MoveDown,
            _ => return Err(()),
        })
    }
}

macro_rules! tri {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                crate::tui::widget::popmsg::Confirm::err(e);
                return do_nothing();
            }
        }
    };
    ($e:expr, or_cancel) => {
        match $e {
            Ok(v) => v,
            Err(_) => return do_nothing(),
        }
    };
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SrvCtlOp {
    Stop,
    Restart,
    SwitchCore,
    StopAll,
    #[cfg(windows)]
    Install,
    #[cfg(windows)]
    Uninstall,
    #[cfg(windows)]
    ToggleSysProxy,
}

impl SrvCtlOp {
    fn as_str(&self) -> &str {
        match self {
            Self::Stop => "Stop Service",
            Self::Restart => "Start Service",
            Self::SwitchCore => "Switch Core",
            Self::StopAll => "Stop All Services",
            #[cfg(windows)]
            Self::Install => "Install Srv",
            #[cfg(windows)]
            Self::Uninstall => "Uninstall Srv",
            #[cfg(windows)]
            Self::ToggleSysProxy => "Toggle SysProxy",
        }
    }
    fn all() -> Vec<Self> {
        let mut ops = vec![Self::Stop, Self::Restart, Self::SwitchCore, Self::StopAll];
        #[cfg(windows)]
        {
            ops.push(Self::Install);
            ops.push(Self::Uninstall);
            ops.push(Self::ToggleSysProxy);
        }
        ops
    }
}

#[derive(Default)]
struct SrvCtlContent {
    ops: Vec<SrvCtlOp>,
    service_name: String,
    bin_path: String,
    is_user: bool,
    status: String,
    core_label: String,
    mihomo_status: String,
    singbox_status: String,
    mihomo_service_name: String,
    singbox_service_name: String,
    mihomo_is_user: bool,
    singbox_is_user: bool,
    #[cfg(windows)]
    proxy_enabled: bool,
}

impl SrvCtlContent {
    fn spawn_status_check(&self, task_set: &mut FutureSet<Self>, target: CoreType) {
        let (service_name, is_user, csc) = match target {
            CoreType::Mihomo => (
                self.mihomo_service_name.clone(),
                self.mihomo_is_user,
                crate::config::CONFIG.cfg_file.mihomo.core_service.clone(),
            ),
            CoreType::Singbox => (
                self.singbox_service_name.clone(),
                self.singbox_is_user,
                crate::config::CONFIG.cfg_file.singbox.core_service.clone(),
            ),
        };
        let controller = crate::config::ServiceController::from_config(&csc);
        async move {
            let status = match controller {
                crate::config::ServiceController::Launchd => launchd_status(&service_name, is_user),
                #[cfg(windows)]
                crate::config::ServiceController::Nssm => {
                    crate::functions::command::nssm_status(&service_name)
                }
                crate::config::ServiceController::OpenRc => {
                    let mut args = vec![service_name.as_str(), "status"];
                    let bin = controller.bin_name();
                    let output = std::process::Command::new(bin).args(&args).output();
                    match output {
                        Ok(o) => {
                            let stdout = String::from_utf8_lossy(&o.stdout);
                            if stdout.contains("started") {
                                "active".to_owned()
                            } else if stdout.contains("stopped") {
                                "inactive".to_owned()
                            } else {
                                stdout.trim().to_owned()
                            }
                        }
                        Err(_) => "?".to_owned(),
                    }
                }
                _ => {
                    let mut args = vec!["is-active"];
                    if is_user {
                        args.push("--user");
                    }
                    args.push(&service_name);
                    let output = std::process::Command::new("systemctl").args(&args).output();
                    match output {
                        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_owned(),
                        Err(_) => "?".to_owned(),
                    }
                }
            };
            wrapper(move |c: &mut SrvCtlContent| match target {
                CoreType::Mihomo => c.mihomo_status = status,
                CoreType::Singbox => c.singbox_status = status,
            })
        }
        .spawn_at(task_set);
    }
    fn spawn_current_status_check(&self, task_set: &mut FutureSet<Self>) {
        let service_name = self.service_name.clone();
        let is_user = self.is_user;
        let csc = match crate::config::CONFIG.core_type() {
            CoreType::Mihomo => crate::config::CONFIG.cfg_file.mihomo.core_service.clone(),
            CoreType::Singbox => crate::config::CONFIG.cfg_file.singbox.core_service.clone(),
        };
        let controller = crate::config::ServiceController::from_config(&csc);
        async move {
            let status = match controller {
                crate::config::ServiceController::Launchd => launchd_status(&service_name, is_user),
                #[cfg(windows)]
                crate::config::ServiceController::Nssm => {
                    crate::functions::command::nssm_status(&service_name)
                }
                crate::config::ServiceController::OpenRc => {
                    let mut args = vec![service_name.as_str(), "status"];
                    let bin = controller.bin_name();
                    let output = std::process::Command::new(bin).args(&args).output();
                    match output {
                        Ok(o) => {
                            let stdout = String::from_utf8_lossy(&o.stdout);
                            if stdout.contains("started") {
                                "active".to_owned()
                            } else if stdout.contains("stopped") {
                                "inactive".to_owned()
                            } else {
                                stdout.trim().to_owned()
                            }
                        }
                        Err(_) => "?".to_owned(),
                    }
                }
                _ => {
                    let mut args = vec!["is-active"];
                    if is_user {
                        args.push("--user");
                    }
                    args.push(&service_name);
                    let output = std::process::Command::new("systemctl").args(&args).output();
                    match output {
                        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_owned(),
                        Err(_) => "?".to_owned(),
                    }
                }
            };
            wrapper(move |c: &mut SrvCtlContent| {
                c.status = status.clone();
                match crate::config::CONFIG.core_type() {
                    CoreType::Mihomo => c.mihomo_status = status,
                    CoreType::Singbox => c.singbox_status = status,
                }
            })
        }
        .spawn_at(task_set);
    }
}

#[cfg(not(unix))]
fn launchd_status(_service_name: &str, _is_user: bool) -> String {
    "?".to_owned()
}

#[cfg(unix)]
fn launchd_status(service_name: &str, is_user: bool) -> String {
    if is_user {
        let uid = unsafe { libc::getuid() };
        let output = std::process::Command::new("launchctl")
            .args(["print", &format!("gui/{uid}/{service_name}")])
            .output();
        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                if stdout.contains("state = running") {
                    "active".to_owned()
                } else {
                    "loaded".to_owned()
                }
            }
            _ => "not loaded".to_owned(),
        }
    } else {
        let output = std::process::Command::new("sudo")
            .args([
                "-n",
                "launchctl",
                "print",
                &format!("system/{service_name}"),
            ])
            .output();
        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                if stdout.contains("state = running") {
                    "active".to_owned()
                } else {
                    "loaded".to_owned()
                }
            }
            _ => "not loaded".to_owned(),
        }
    }
}

impl BasicTabContent for SrvCtlContent {
    type Key = SrvCtlKey;

    type State = ListState;

    const TITLE: &str = "CoreSrvCtl";

    fn all_shortcuts() -> &'static [(KeyCombo, Self::Key, &'static str)] {
        agent::all_shortcuts()
    }

    fn on_enter(&mut self, task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        self.spawn_status_check(task_set, CoreType::Mihomo);
        self.spawn_status_check(task_set, CoreType::Singbox);
    }
}

impl TabContent for SrvCtlContent {
    fn init(&mut self, task_set: &mut FutureSet<Self>, state: &mut Self::State) {
        self.ops = SrvCtlOp::all();
        let cfg = &crate::config::CONFIG.cfg_file;

        self.mihomo_service_name = cfg.mihomo.core_service.service_name.clone();
        if self.mihomo_service_name.is_empty() {
            self.mihomo_service_name = "clashtui_mihomo".to_owned();
        }
        self.mihomo_is_user = cfg.mihomo.core_service.is_user;

        self.singbox_service_name = cfg.singbox.core_service.service_name.clone();
        if self.singbox_service_name.is_empty() {
            self.singbox_service_name = "clashtui_singbox".to_owned();
        }
        self.singbox_is_user = cfg.singbox.core_service.is_user;

        match crate::config::CONFIG.core_type() {
            CoreType::Mihomo => {
                self.service_name = self.mihomo_service_name.clone();
                self.bin_path = cfg.mihomo.core.bin_path.clone();
                if self.bin_path.is_empty() {
                    self.bin_path = "/usr/bin/mihomo".to_owned();
                }
                self.is_user = self.mihomo_is_user;
                self.core_label = "mihomo".to_owned();
            }
            CoreType::Singbox => {
                self.service_name = self.singbox_service_name.clone();
                self.bin_path = cfg.singbox.core.bin_path.clone();
                if self.bin_path.is_empty() {
                    self.bin_path = "/usr/bin/sing-box".to_owned();
                }
                self.is_user = self.singbox_is_user;
                self.core_label = "sing-box".to_owned();
            }
        }
        self.status = "...".to_owned();
        self.mihomo_status = "...".to_owned();
        self.singbox_status = "...".to_owned();
        #[cfg(windows)]
        {
            self.proxy_enabled =
                crate::functions::command::get_system_proxy_state().unwrap_or(false);
        }
        if !self.ops.is_empty() {
            state.select(Some(0));
        }
        self.spawn_status_check(task_set, CoreType::Mihomo);
        self.spawn_status_check(task_set, CoreType::Singbox);
    }

    fn handle_key_event(
        &mut self,
        key: SrvCtlKey,
        task_set: &mut FutureSet<Self>,
        state: &mut Self::State,
    ) {
        // ---- main list routing ----
        match key {
            SrvCtlKey::MoveUp => {
                let i = state.selected().unwrap_or(0);
                state.select(Some(i.saturating_sub(1)));
            }
            SrvCtlKey::MoveDown => {
                let i = state.selected().unwrap_or(0);
                if i + 1 < self.ops.len() {
                    state.select(Some(i + 1));
                }
            }
            SrvCtlKey::Execute => {
                let Some(idx) = state.selected() else { return };
                let Some(op) = self.ops.get(idx) else { return };
                let op = *op;

                let _bin_path = self.bin_path.clone();
                let needs_sudo = !self.is_user;

                async move {
                    let password =
                        match crate::functions::command::resolve_sudo_password(needs_sudo).await {
                            Ok(pw) => pw,
                            Err(_) => return do_nothing(),
                        };

                    macro_rules! handle {
                        ($result:expr, $new_status:expr) => {
                            match $result {
                                Ok(out) => {
                                    if out.starts_with("Error") {
                                        crate::tui::widget::popmsg::Confirm::err(out);
                                        do_nothing()
                                    } else {
                                        crate::tui::widget::popmsg::Confirm::title("OK".to_owned())
                                            .with_prompt(out)
                                            .build_and_send();
                                        wrapper(move |c: &mut SrvCtlContent| {
                                            let s = $new_status.to_owned();
                                            c.status = s.clone();
                                            match crate::config::CONFIG.core_type() {
                                                CoreType::Mihomo => c.mihomo_status = s,
                                                CoreType::Singbox => c.singbox_status = s,
                                            }
                                        })
                                    }
                                }
                                Err(e) => {
                                    crate::tui::widget::popmsg::Confirm::err(e);
                                    do_nothing()
                                }
                            }
                        };
                        ($result:expr) => {
                            match $result {
                                Ok(out) => {
                                    if out.starts_with("Error") {
                                        crate::tui::widget::popmsg::Confirm::err(out);
                                    } else {
                                        crate::tui::widget::popmsg::Confirm::title("OK".to_owned())
                                            .with_prompt(out)
                                            .build_and_send();
                                    }
                                    do_nothing()
                                }
                                Err(e) => {
                                    crate::tui::widget::popmsg::Confirm::err(e);
                                    do_nothing()
                                }
                            }
                        };
                    }

                    let pw_ref = password.as_deref();
                    match op {
                        SrvCtlOp::Stop => {
                            handle!(crate::functions::command::stop_service(pw_ref), "inactive")
                        }
                        SrvCtlOp::Restart => {
                            handle!(crate::functions::command::restart_service(pw_ref), "active")
                        }
                        SrvCtlOp::StopAll => {
                            handle!(
                                crate::functions::command::stop_all_services(pw_ref),
                                "inactive"
                            )
                        }
                        SrvCtlOp::SwitchCore => {
                            let old_type = crate::config::CONFIG.core_type();
                            let new_type = match old_type {
                                CoreType::Mihomo => CoreType::Singbox,
                                CoreType::Singbox => CoreType::Mihomo,
                            };
                            let new_label = match new_type {
                                CoreType::Mihomo => "mihomo",
                                CoreType::Singbox => "sing-box",
                            };

                            // stop all core services first
                            let stop_result = crate::functions::command::stop_all_services(pw_ref);

                            match (|| -> anyhow::Result<()> {
                                crate::config::CONFIG.data.lock().unwrap().core_type = new_type;
                                crate::config::CONFIG.save()
                            })() {
                                Ok(()) => {
                                    // start the target core
                                    let start_result =
                                        crate::functions::command::start_core_service(
                                            pw_ref, new_type,
                                        );

                                    let status_msg = format!(
                                        "Core switched to {new_label}\n\n\
                                         Stop all services: {stop}\n\
                                         Start {new_label}: {start}",
                                        stop = stop_result
                                            .as_ref()
                                            .map(|s| s.as_str())
                                            .unwrap_or("?")
                                            .trim(),
                                        start = start_result
                                            .as_ref()
                                            .map(|s| s.as_str())
                                            .unwrap_or("?")
                                            .trim(),
                                    );

                                    let update_label = wrapper(move |c: &mut SrvCtlContent| {
                                        c.core_label = new_label.to_owned();
                                        if start_result.is_ok() {
                                            match new_type {
                                                CoreType::Mihomo => {
                                                    c.mihomo_status = "active".to_owned();
                                                    c.singbox_status = "inactive".to_owned();
                                                }
                                                CoreType::Singbox => {
                                                    c.mihomo_status = "inactive".to_owned();
                                                    c.singbox_status = "active".to_owned();
                                                }
                                            }
                                        }
                                        c.status = match new_type {
                                            CoreType::Mihomo => c.mihomo_status.clone(),
                                            CoreType::Singbox => c.singbox_status.clone(),
                                        };
                                    });

                                    let _ = crate::tui::widget::popmsg::Confirm::dismiss_any(
                                        "Core Switched".to_owned(),
                                    )
                                    .with_prompt(status_msg)
                                    .build_and_send()
                                    .await;

                                    crate::tui::app::QUIT
                                        .store(true, std::sync::atomic::Ordering::Relaxed);
                                    update_label
                                }
                                Err(e) => {
                                    crate::tui::widget::popmsg::Confirm::err(e);
                                    do_nothing()
                                }
                            }
                        }
                        #[cfg(windows)]
                        SrvCtlOp::Install => {
                            handle!(
                                crate::functions::command::install_core_service(
                                    pw_ref,
                                    crate::config::CONFIG.core_type(),
                                ),
                                "installed"
                            )
                        }
                        #[cfg(windows)]
                        SrvCtlOp::Uninstall => {
                            handle!(
                                crate::functions::command::uninstall_core_service(
                                    pw_ref,
                                    crate::config::CONFIG.core_type(),
                                ),
                                "uninstalled"
                            )
                        }
                        #[cfg(windows)]
                        SrvCtlOp::ToggleSysProxy => {
                            let result = async {
                                let enabled = crate::functions::command::get_system_proxy_state()?;
                                if enabled {
                                    crate::functions::command::disable_system_proxy()?;
                                    Ok::<bool, anyhow::Error>(false)
                                } else {
                                    let port = crate::functions::command::get_mixed_port()?;
                                    crate::functions::command::enable_system_proxy(port)?;
                                    Ok::<bool, anyhow::Error>(true)
                                }
                            };
                            match result.await {
                                Ok(new_state) => {
                                    let label = if new_state { "enabled" } else { "disabled" };
                                    crate::tui::widget::popmsg::Confirm::title("OK".to_owned())
                                        .with_prompt(format!("System proxy {label}"))
                                        .build_and_send();
                                    wrapper(move |c: &mut SrvCtlContent| {
                                        c.proxy_enabled = new_state;
                                    })
                                }
                                Err(e) => {
                                    crate::tui::widget::popmsg::Confirm::err(e);
                                    do_nothing()
                                }
                            }
                        }
                    }
                }
                .spawn_at(task_set);
            }
            _ => {}
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State) {
        let user_tag = if self.is_user { " (user)" } else { "" };
        let theme = Theme::get();
        let section = theme.section("srvctl");

        let current_core_label = format!(
            "► {}: {}",
            match crate::config::CONFIG.core_type() {
                CoreType::Mihomo => "mihomo",
                CoreType::Singbox => "sing-box",
            },
            match crate::config::CONFIG.core_type() {
                CoreType::Mihomo => &self.mihomo_status,
                CoreType::Singbox => &self.singbox_status,
            }
        );
        let other_core_label = format!(
            "  {}: {}",
            match crate::config::CONFIG.core_type() {
                CoreType::Mihomo => "sing-box",
                CoreType::Singbox => "mihomo",
            },
            match crate::config::CONFIG.core_type() {
                CoreType::Mihomo => &self.singbox_status,
                CoreType::Singbox => &self.mihomo_status,
            }
        );

        #[cfg(windows)]
        let proxy_label = format!("  Proxy: {}", if self.proxy_enabled { "ON" } else { "OFF" });

        let title_line = format!(
            "{} — {} (core: {}){}",
            Self::TITLE,
            self.service_name,
            self.core_label,
            user_tag
        );

        let block = Block::bordered()
            .border_style(section.border)
            .title(title_line)
            .title_bottom({
                let mut spans = vec![
                    ratatui::text::Span::styled(
                        format!(" {} ", current_core_label),
                        section.border,
                    ),
                    ratatui::text::Span::styled(
                        format!(" {} ", other_core_label),
                        section.border.fg(Color::Rgb(100, 100, 100)),
                    ),
                ];
                #[cfg(windows)]
                spans.push(ratatui::text::Span::styled(
                    format!(" {} ", proxy_label),
                    section.border.fg(Color::Rgb(100, 100, 100)),
                ));
                ratatui::text::Line::from(spans).right_aligned()
            });

        let items: Vec<ListItem> = self
            .ops
            .iter()
            .map(|op| ListItem::new(format!("  {}", op.as_str())))
            .collect();

        let highlight_style = section.highlight;
        let list = List::new(items)
            .block(block)
            .highlight_style(highlight_style);

        f.render_stateful_widget(list, area, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_key(code: KeyCode) -> crate::tui::Key {
        crate::tui::Key {
            code,
            shift: matches!(code, KeyCode::Char(c) if c.is_ascii_uppercase()),
            ctrl: false,
            alt: false,
            super_: false,
        }
    }

    #[test]
    fn srvctl_key_agent_contains_expected() {
        let a = agent();
        assert!(a.contains_key(&mk_key(KeyCode::Enter)));
        assert!(a.contains_key(&mk_key(KeyCode::Esc)));
        assert!(a.contains_key(&mk_key(KeyCode::Up)));
        assert!(a.contains_key(&mk_key(KeyCode::Down)));
        assert!(a.contains_key(&mk_key(KeyCode::Char('k'))));
        assert!(a.contains_key(&mk_key(KeyCode::Char('j'))));
    }

    #[test]
    fn srvctl_key_try_from_correct_actions() {
        assert!(matches!(
            SrvCtlKey::try_from(&mk_key(KeyCode::Enter)),
            Ok(SrvCtlKey::Execute)
        ));
        assert!(matches!(
            SrvCtlKey::try_from(&mk_key(KeyCode::Esc)),
            Ok(SrvCtlKey::Esc)
        ));
        assert!(matches!(
            SrvCtlKey::try_from(&mk_key(KeyCode::Up)),
            Ok(SrvCtlKey::MoveUp)
        ));
        assert!(matches!(
            SrvCtlKey::try_from(&mk_key(KeyCode::Down)),
            Ok(SrvCtlKey::MoveDown)
        ));
    }

    #[test]
    fn srvctl_key_try_from_unknown_is_err() {
        assert!(SrvCtlKey::try_from(&mk_key(KeyCode::Char('x'))).is_err());
    }

    #[test]
    fn srvctl_op_all_is_non_empty() {
        let ops = SrvCtlOp::all();
        assert!(!ops.is_empty());
        assert!(ops.contains(&SrvCtlOp::Stop));
        assert!(ops.contains(&SrvCtlOp::Restart));
        assert!(ops.contains(&SrvCtlOp::SwitchCore));
        assert!(ops.contains(&SrvCtlOp::StopAll));
    }

    #[test]
    fn srvctl_op_as_str_returns_non_empty() {
        for op in [
            SrvCtlOp::Stop,
            SrvCtlOp::Restart,
            SrvCtlOp::SwitchCore,
            SrvCtlOp::StopAll,
        ] {
            assert!(!op.as_str().is_empty());
        }
    }

    #[test]
    fn srvctl_op_as_str_unique() {
        let ops = SrvCtlOp::all();
        let names: Vec<&str> = ops.iter().map(|op| op.as_str()).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(
            names.len(),
            unique.len(),
            "all SrvCtlOp names should be unique"
        );
    }
}
