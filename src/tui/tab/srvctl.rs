use super::dev::*;
use crate::config::CoreType;
use ratatui::style::Color;
use ratatui::widgets::ListItem;
#[cfg(unix)]
use libc;

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
}

impl SrvCtlOp {
    fn as_str(&self) -> &str {
        match self {
            Self::Stop => "Stop Service",
            Self::Restart => "Start Service",
            Self::SwitchCore => "Switch Core",
            Self::StopAll => "Stop All Services",
        }
    }
    fn all() -> Vec<Self> {
        vec![Self::Stop, Self::Restart, Self::SwitchCore, Self::StopAll]
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
}

impl SrvCtlContent {
    fn spawn_status_check(&self, task_set: &mut FutureSet<Self>, target: CoreType) {
        let (service_name, is_user) = match target {
            CoreType::Mihomo => (self.mihomo_service_name.clone(), self.mihomo_is_user),
            CoreType::Singbox => (self.singbox_service_name.clone(), self.singbox_is_user),
        };
        let controller = crate::config::ServiceController::default();
        async move {
            let status = match controller {
                crate::config::ServiceController::Launchd => {
                    launchd_status(&service_name, is_user)
                }
                _ => {
                    let mut args = vec!["is-active"];
                    if is_user {
                        args.push("--user");
                    }
                    args.push(&service_name);
                    let output = std::process::Command::new("systemctl")
                        .args(&args)
                        .output();
                    match output {
                        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_owned(),
                        Err(_) => "?".to_owned(),
                    }
                }
            };
            wrapper(move |c: &mut SrvCtlContent| {
                match target {
                    CoreType::Mihomo => c.mihomo_status = status,
                    CoreType::Singbox => c.singbox_status = status,
                }
            })
        }
        .spawn_at(task_set);
    }
    fn spawn_current_status_check(&self, task_set: &mut FutureSet<Self>) {
        let service_name = self.service_name.clone();
        let is_user = self.is_user;
        let controller = crate::config::ServiceController::default();
        async move {
            let status = match controller {
                crate::config::ServiceController::Launchd => {
                    launchd_status(&service_name, is_user)
                }
                _ => {
                    let mut args = vec!["is-active"];
                    if is_user {
                        args.push("--user");
                    }
                    args.push(&service_name);
                    let output = std::process::Command::new("systemctl")
                        .args(&args)
                        .output();
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
            .args(["launchctl", "print", &format!("system/{service_name}")])
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
                    let password = match crate::functions::command::resolve_sudo_password(needs_sudo).await {
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
                                        crate::tui::widget::popmsg::Confirm::title(
                                            "OK".to_owned(),
                                        )
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
                                        crate::tui::widget::popmsg::Confirm::title(
                                            "OK".to_owned(),
                                        )
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
                            handle!(
                                crate::functions::command::stop_service(pw_ref),
                                "inactive"
                            )
                        }
                        SrvCtlOp::Restart => {
                            handle!(
                                crate::functions::command::restart_service(pw_ref),
                                "active"
                            )
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

                                    crate::tui::app::QUIT.store(
                                        true,
                                        std::sync::atomic::Ordering::Relaxed,
                                    );
                                    update_label
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

        let block = Block::bordered()
            .border_style(section.border)
            .title(format!(
                "{} — {} (core: {}){}",
                Self::TITLE, self.service_name, self.core_label, user_tag
            ))
            .title_bottom(
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(
                        format!(" {} ", current_core_label),
                        section.border,
                    ),
                    ratatui::text::Span::styled(
                        format!(" {} ", other_core_label),
                        section.border.fg(Color::Rgb(100, 100, 100)),
                    ),
                ])
                .right_aligned(),
            );

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
