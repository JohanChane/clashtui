use super::dev::*;
use crossterm::event::KeyEventKind;
use ratatui::{text::Line, widgets::ListItem};

newtype_tab!(SrvCtlTab(Tab<SrvCtlContent>));

mod_agent!(
    SrvCtlKey,
    [
        ([KeyCode::Enter], SrvCtlKey::Execute, "Execute selected operation"),
        ([KeyCode::Up], SrvCtlKey::MoveUp, ""),
        ([KeyCode::Down], SrvCtlKey::MoveDown, ""),
        ([KeyCode::Char('k')], SrvCtlKey::MoveUp, ""),
        ([KeyCode::Char('j')], SrvCtlKey::MoveDown, ""),
    ]
);

#[derive(Clone, Copy, serde::Deserialize)]
pub(super) enum SrvCtlKey {
    Execute,
    MoveUp,
    MoveDown,
}

impl TryFrom<&KeyEvent> for SrvCtlKey {
    type Error = ();

    fn try_from(ev: &KeyEvent) -> Result<Self, Self::Error> {
        let agent = agent();
        if !agent.is_empty() {
            return agent.get(ev).map(|k| *k).ok_or(());
        }
        if ev.kind != KeyEventKind::Press {
            return Err(());
        }
        Ok(match ev.code {
            KeyCode::Enter => Self::Execute,
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
    SetPermission,
    FixFilePermissions,
}

impl SrvCtlOp {
    fn as_str(&self) -> &str {
        match self {
            Self::Stop => "Stop Service",
            Self::Restart => "Start Service",
            Self::SetPermission => "Set Permission",
            Self::FixFilePermissions => "Fix File Permissions",
        }
    }
    fn all() -> Vec<Self> {
        vec![Self::Stop, Self::Restart, Self::SetPermission, Self::FixFilePermissions]
    }
}

#[derive(Default)]
struct SrvCtlContent {
    ops: Vec<SrvCtlOp>,
    service_name: String,
    bin_path: String,
    is_user: bool,
    status: String,
}

impl SrvCtlContent {
    fn spawn_status_check(&self, task_set: &mut FutureSet<Self>) {
        let service_name = self.service_name.clone();
        let is_user = self.is_user;
        async move {
            let mut args = vec!["is-active"];
            if is_user {
                args.push("--user");
            }
            args.push(&service_name);
            let output = std::process::Command::new("systemctl")
                .args(&args)
                .output();
            let status = match output {
                Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_owned(),
                Err(_) => "?".to_owned(),
            };
            wrapper(move |c: &mut SrvCtlContent| {
                c.status = status;
            })
        }
        .spawn_at(task_set);
    }
}

impl BasicTabContent for SrvCtlContent {
    type Key = SrvCtlKey;

    type State = ListState;

    const TITLE: &str = "ClashSrvCtl";

    fn all_shortcuts() -> &'static [(KeyCombo, Self::Key, &'static str)] {
        agent::all_shortcuts()
    }
}

impl TabContent for SrvCtlContent {
    fn init(&mut self, task_set: &mut FutureSet<Self>, state: &mut Self::State) {
        self.ops = SrvCtlOp::all();
        self.service_name = crate::config::CONFIG.cfg_file.service.clash_service_name.clone();
        if self.service_name.is_empty() {
            self.service_name = "clashtui_mihomo".to_owned();
        }
        self.bin_path = crate::config::CONFIG.cfg_file.basic.clash_bin_path.clone();
        if self.bin_path.is_empty() {
            self.bin_path = "/usr/bin/mihomo".to_owned();
        }
        self.is_user = crate::config::CONFIG.cfg_file.service.is_user;
        self.status = "...".to_owned();
        if !self.ops.is_empty() {
            state.select(Some(0));
        }
        self.spawn_status_check(task_set);
    }

    fn handle_key_event(
        &mut self,
        key: SrvCtlKey,
        task_set: &mut FutureSet<Self>,
        state: &mut Self::State,
    ) {
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
                let bin_path = self.bin_path.clone();
                let config_dir = crate::config::CONFIG.cfg_file.basic.clash_config_dir.clone();
                let needs_sudo = !self.is_user;

                async move {
                    let password = if needs_sudo && op != SrvCtlOp::FixFilePermissions {
                        let pw = tri!(
                            InputMasked::new()
                                .with_title("Sudo Password".to_owned())
                                .with_prompt("Sudo password (empty if NOPASSWD):".to_owned())
                                .build_and_send()
                                .await,
                            or_cancel
                        );
                        Some(pw)
                    } else {
                        None
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
                                            c.status = $new_status.to_owned();
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
                        SrvCtlOp::SetPermission => {
                            handle!(crate::functions::command::set_permission(
                                &bin_path, pw_ref,
                            ))
                        }
                        SrvCtlOp::FixFilePermissions => {
                            let dir = std::path::Path::new(&config_dir);
                            let group = crate::functions::command::get_dir_group_name(dir);
                            let Some(group) = group else {
                                crate::tui::widget::popmsg::Confirm::err(anyhow::anyhow!(
                                    "Cannot determine group of '{}'. Does the directory exist?",
                                    config_dir
                                ));
                                return do_nothing();
                            };
                            handle!(
                                crate::functions::command::repair_file_permissions(dir, &group)
                            )
                        }
                    }
                }
                .spawn_at(task_set);
            }
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State) {
        let user_tag = if self.is_user { " (user)" } else { "" };
        let block = Block::bordered()
            .border_style(Theme::get().tab.tab_focused)
            .title(format!("{} — {}{}", Self::TITLE, self.service_name, user_tag))
            .title_bottom(
                Line::raw(format!(" {} ", self.status))
                    .right_aligned()
                    .reversed(),
            );

        let items: Vec<ListItem> = self
            .ops
            .iter()
            .map(|op| ListItem::new(format!("  {}", op.as_str())))
            .collect();

        let highlight_style = Theme::get().tab.item_highlighted;
        let list = List::new(items)
            .block(block)
            .highlight_style(highlight_style);

        f.render_stateful_widget(list, area, state);
    }
}
