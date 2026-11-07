use super::dev::*;
use crate::config::CoreType;
use crate::tui::{
    business::{Business, BusinessView},
    core::ServiceOperation,
};
use ratatui::style::Color;
use ratatui::widgets::ListItem;

newtype_tab!(CoreSrvCtlTab(Tab<SrvCtlContent>), Srvctl);

key_map!(
    Srvctl,
    Action,
    DefaultBindings::new().with_keys([
        (KeyCode::Enter, Action::Execute),
        (KeyCode::Up, Action::MoveUp),
        (KeyCode::Down, Action::MoveDown),
        (KeyCode::Char('k'), Action::MoveUp),
        (KeyCode::Char('j'), Action::MoveDown),
    ])
);

#[derive_aliases::derive(..Action, Debug)]
pub(in crate::tui) enum Action {
    Execute,
    MoveUp,
    MoveDown,
    // Retain dev's configurable Esc action, which does not change the service view.
    Esc,
}

impl Document for Action {
    fn get_doc(&self) -> &'static str {
        use crate::tui::key::consts::*;
        match self {
            Self::Execute => "Execute",
            Self::MoveUp => MOVE_UP,
            Self::MoveDown => MOVE_DOWN,
            Self::Esc => "Back",
        }
    }
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
        #[cfg_attr(not(windows), allow(unused_mut))]
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
    is_user: bool,
    core_label: String,
    mihomo_status: String,
    singbox_status: String,
    #[cfg(windows)]
    proxy_enabled: bool,
}
impl SrvCtlContent {
    fn read_state(&mut self, view: BusinessView<'_>) {
        self.mihomo_status = view.core.state.services[0].description.clone();
        self.singbox_status = view.core.state.services[1].description.clone();
    }
}
impl BasicTabContent for SrvCtlContent {
    type Action = Action;
    type State = ListState;
    const TITLE: &str = "CoreSrvCtl";
    fn sync_from(&mut self, view: BusinessView<'_>, _: &mut ListState) {
        self.read_state(view);
    }
    fn on_enter(&mut self, _tasks: &mut FutureSet<Self>, _: &mut ListState, cx: &mut Business) {
        cx.check_services();
        self.read_state(cx.view());
        #[cfg(windows)]
        async {
            let enabled =
                tokio::task::spawn_blocking(crate::functions::command::get_system_proxy_state)
                    .await;
            wrapper(move |content: &mut Self| {
                if let Ok(Ok(enabled)) = enabled {
                    content.proxy_enabled = enabled;
                }
            })
        }
        .spawn_at(_tasks);
    }
}
impl TabContent for SrvCtlContent {
    fn init(&mut self, _: &mut FutureSet<Self>, state: &mut ListState) {
        self.ops = SrvCtlOp::all();
        let core = crate::config::CONFIG.core_type();
        let config = match core {
            CoreType::Mihomo => &crate::config::CONFIG.cfg_file.mihomo.core_service,
            CoreType::Singbox => &crate::config::CONFIG.cfg_file.singbox.core_service,
        };
        self.service_name = if config.service_name.is_empty() {
            match core {
                CoreType::Mihomo => "clashtui_mihomo",
                CoreType::Singbox => "clashtui_singbox",
            }
            .into()
        } else {
            config.service_name.clone()
        };
        self.is_user = config.is_user;
        self.core_label = core.to_string();
        self.mihomo_status = "Not checked".into();
        self.singbox_status = "Not checked".into();
        if !self.ops.is_empty() {
            state.select(Some(0));
        }
    }
    fn handle_action(
        &mut self,
        action: Action,
        _tasks: &mut FutureSet<Self>,
        state: &mut ListState,
        cx: &mut Business,
    ) {
        // ---- main list routing ----
        match action {
            Action::Esc => {}
            Action::MoveUp => {
                let i = state.selected().unwrap_or(0);
                state.select(Some(i.saturating_sub(1)));
            }
            Action::MoveDown => {
                let i = state.selected().unwrap_or(0);
                if i + 1 < self.ops.len() {
                    state.select(Some(i + 1));
                }
            }
            Action::Execute => {
                let Some(op) = state.selected().and_then(|i| self.ops.get(i)).copied() else {
                    return;
                };
                let target = crate::config::CONFIG.core_type();
                match op {
                    SrvCtlOp::Stop => {
                        cx.start_service(target, ServiceOperation::Stop);
                    }
                    SrvCtlOp::Restart => {
                        cx.start_service(target, ServiceOperation::Start);
                    }
                    SrvCtlOp::StopAll => {
                        cx.start_service(target, ServiceOperation::StopAll);
                    }
                    SrvCtlOp::SwitchCore => {
                        let next = match target {
                            CoreType::Mihomo => CoreType::Singbox,
                            CoreType::Singbox => CoreType::Mihomo,
                        };
                        cx.start_service(next, ServiceOperation::SwitchCore);
                    }
                    #[cfg(windows)]
                    SrvCtlOp::Install => {
                        cx.start_service(target, ServiceOperation::Install);
                    }
                    #[cfg(windows)]
                    SrvCtlOp::Uninstall => {
                        cx.start_service(target, ServiceOperation::Uninstall);
                    }
                    #[cfg(windows)]
                    SrvCtlOp::ToggleSysProxy => {
                        let work = cx.toggle_system_proxy();
                        async move {
                            let result = work.await;
                            match result {
                                Ok(Ok(enabled)) => {
                                    Confirm::message_async(
                                        "System proxy".to_owned(),
                                        if enabled { "Enabled" } else { "Disabled" }.to_owned(),
                                    )
                                    .await;
                                    wrapper(move |content: &mut Self| {
                                        content.proxy_enabled = enabled
                                    })
                                }
                                other => {
                                    Confirm::err_async(format!("System proxy failed: {other:?}"))
                                        .await;
                                    do_nothing()
                                }
                            }
                        }
                        .spawn_at(_tasks);
                    }
                }
            }
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State, _view: BusinessView<'_>) {
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
                #[cfg_attr(not(windows), allow(unused_mut))]
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
