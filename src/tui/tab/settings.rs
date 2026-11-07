use super::dev::*;
use crate::config::CONFIG;
use crate::tui::{
    business::{Business, BusinessView},
    core::ApiResource,
};
use ratatui::{
    layout::{Constraint, Layout},
    text::{Line, Span},
    widgets::{Clear, ListItem, Paragraph},
};
use strum::VariantArray;

newtype_tab!(SettingsTab(Tab<SettingsContent>), Settings);

key_map!(
    Settings,
    Action,
    DefaultBindings::new().with_keys([
        (KeyCode::Enter, Action::Execute),
        (KeyCode::Esc, Action::Esc),
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

use crate::config::CoreType;
use crate::functions::restful::config_struct::{Mode, TunStack};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsOp {
    SwitchMode,
    AllowLan,
    TunEnable,
    TunStackOp,
    FlushFakeIP,
    FlushDNSCache,
    UpdateGeo,
}

impl SettingsOp {
    fn all() -> Vec<Self> {
        let mut ops = vec![Self::SwitchMode, Self::AllowLan];
        if CONFIG.core_type() == CoreType::Mihomo {
            ops.push(Self::TunEnable);
            ops.push(Self::TunStackOp);
            ops.push(Self::FlushFakeIP);
            ops.push(Self::FlushDNSCache);
            ops.push(Self::UpdateGeo);
        }
        ops
    }
}

#[derive(Default)]
struct SettingsContent {
    ops: Vec<SettingsOp>,
    mode_selector_state: ListState,
    mode_selector_visible: bool,
    tun_selector_state: ListState,
    tun_selector_visible: bool,
    modes: Vec<Mode>,
    tun_stacks: Vec<TunStack>,
}

impl BasicTabContent for SettingsContent {
    type Action = Action;
    type State = ListState;
    const TITLE: &str = "Settings";
    fn sync_from(&mut self, view: BusinessView<'_>, _: &mut ListState) {
        if !view.core.api_allowed() {
            self.mode_selector_visible = false;
            self.tun_selector_visible = false;
        }
    }
    fn on_enter(&mut self, _: &mut FutureSet<Self>, _: &mut ListState, cx: &mut Business) {
        cx.enter(ApiResource::Settings);
    }
    fn on_leave(&mut self, _: &mut FutureSet<Self>, _: &mut ListState, cx: &mut Business) {
        cx.leave(ApiResource::Settings);
    }
}
impl TabContent for SettingsContent {
    fn init(&mut self, _: &mut FutureSet<Self>, state: &mut ListState) {
        self.ops = SettingsOp::all();
        self.modes = Mode::VARIANTS.to_vec();
        self.tun_stacks = TunStack::VARIANTS.to_vec();
        self.mode_selector_state.select(Some(0));
        self.tun_selector_state.select(Some(0));
        if !self.ops.is_empty() {
            state.select(Some(0));
        }
    }
    fn handle_action(
        &mut self,
        action: Action,
        _: &mut FutureSet<Self>,
        state: &mut ListState,
        cx: &mut Business,
    ) {
        if matches!(action, Action::Execute)
            && (!cx.view().core.api_allowed() || cx.view().core.queries.settings.data.is_none())
        {
            return;
        }
        if self.mode_selector_visible {
            match action {
                Action::MoveUp => {
                    let i = self.mode_selector_state.selected().unwrap_or(0);
                    self.mode_selector_state.select(Some(i.saturating_sub(1)));
                }
                Action::MoveDown => {
                    let i = self.mode_selector_state.selected().unwrap_or(0);
                    if i + 1 < self.modes.len() {
                        self.mode_selector_state.select(Some(i + 1));
                    }
                }
                Action::Esc => self.mode_selector_visible = false,
                Action::Execute => {
                    if let Some(mode) = self
                        .modes
                        .get(self.mode_selector_state.selected().unwrap_or(0))
                    {
                        cx.patch_settings(
                            serde_json::json!({"mode": mode.to_string()}).to_string(),
                        );
                    }
                    self.mode_selector_visible = false;
                }
            }
            return;
        }
        if self.tun_selector_visible {
            match action {
                Action::MoveUp => {
                    let i = self.tun_selector_state.selected().unwrap_or(0);
                    self.tun_selector_state.select(Some(i.saturating_sub(1)));
                }
                Action::MoveDown => {
                    let i = self.tun_selector_state.selected().unwrap_or(0);
                    if i + 1 < self.tun_stacks.len() {
                        self.tun_selector_state.select(Some(i + 1));
                    }
                }
                Action::Esc => self.tun_selector_visible = false,
                Action::Execute => {
                    if let Some(stack) = self
                        .tun_stacks
                        .get(self.tun_selector_state.selected().unwrap_or(0))
                    {
                        cx.patch_settings(
                            serde_json::json!({"tun": {"stack": stack.to_string()}}).to_string(),
                        );
                    }
                    self.tun_selector_visible = false;
                }
            }
            return;
        }
        match action {
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
            Action::Esc => {}
            Action::Execute => {
                let Some(op) = state.selected().and_then(|i| self.ops.get(i)).copied() else {
                    return;
                };
                match op {
                    SettingsOp::SwitchMode => self.mode_selector_visible = true,
                    SettingsOp::TunStackOp => self.tun_selector_visible = true,
                    SettingsOp::AllowLan => cx.patch_settings(
                        serde_json::json!({"allow-lan": !cx.view().core.queries.settings.data.as_ref().and_then(|c| c.allow_lan).unwrap_or(false)}).to_string(),
                    ),
                    SettingsOp::TunEnable => cx.patch_settings(
                        serde_json::json!({"tun": {"enable": !cx.view().core.queries.settings.data.as_ref().and_then(|c| c.tun.as_ref()).is_some_and(|t| t.enable)}}).to_string(),
                    ),
                    SettingsOp::FlushFakeIP | SettingsOp::FlushDNSCache | SettingsOp::UpdateGeo => {
                        use crate::tui::business::SettingsMaintenance;
                        cx.maintain_settings(match op {
                            SettingsOp::FlushFakeIP => SettingsMaintenance::FlushFakeIp,
                            SettingsOp::FlushDNSCache => SettingsMaintenance::FlushDns,
                            _ => SettingsMaintenance::UpdateGeo,
                        });
                    }
                }
            }
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State, view: BusinessView<'_>) {
        let block = Block::bordered()
            .border_style(Theme::get().section("settings").border)
            .title(Self::TITLE);

        let error = view
            .core
            .diagnostic()
            .or_else(|| view.core.queries.settings.error.clone());
        if let Some(error) = error {
            let widget = Paragraph::new(error.as_str()).block(block);
            f.render_widget(widget, area);
            return;
        }

        let Some(config) = &view.core.queries.settings.data else {
            f.render_widget(
                Paragraph::new("Loading configuration...").block(block),
                area,
            );
            return;
        };
        let current_mode = config.mode.to_string();
        let allow_lan = config.allow_lan.unwrap_or(false);
        let tun_enable = config.tun.as_ref().is_some_and(|tun| tun.enable);
        let tun_stack = config
            .tun
            .as_ref()
            .map(|tun| tun.stack.to_string())
            .unwrap_or_else(|| "Mixed".into());
        let value_style = Theme::get().section("settings").muted;

        let items: Vec<ListItem> = self
            .ops
            .iter()
            .map(|op| {
                let (name, current) = match op {
                    SettingsOp::SwitchMode => ("Mode", current_mode.as_str()),
                    SettingsOp::AllowLan => {
                        let val = if allow_lan { "Yes" } else { "No" };
                        ("Allow LAN", val)
                    }
                    SettingsOp::TunEnable => {
                        let val = if tun_enable { "Yes" } else { "No" };
                        ("TUN", val)
                    }
                    SettingsOp::TunStackOp => ("TUN Stack", tun_stack.as_str()),
                    SettingsOp::FlushFakeIP => ("Flush Fake-IP", ""),
                    SettingsOp::FlushDNSCache => ("Flush DNS Cache", ""),
                    SettingsOp::UpdateGeo => ("Update GEO", ""),
                };
                ListItem::new(Line::from(vec![
                    Span::raw(format!("  {:<14}", name)),
                    Span::raw(current).style(value_style),
                ]))
            })
            .collect();

        let highlight_style = Theme::get().section("settings").highlight;
        let list = List::new(items)
            .block(block)
            .highlight_style(highlight_style);

        f.render_stateful_widget(list, area, state);

        if self.mode_selector_visible {
            let select_area = centered_rect(60, 30, area);
            let mode_items: Vec<ListItem> = self
                .modes
                .iter()
                .map(|m| ListItem::new(format!("  {}", m)))
                .collect();
            let mode_list = List::new(mode_items)
                .block(
                    Block::bordered()
                        .border_style(Theme::get().section("settings").border)
                        .title("Mode"),
                )
                .highlight_style(highlight_style);
            f.render_widget(Clear, select_area);
            f.render_stateful_widget(
                mode_list,
                select_area,
                &mut self.mode_selector_state.clone(),
            );
        }

        if self.tun_selector_visible {
            let select_area = centered_rect(60, 30, area);
            let tun_items: Vec<ListItem> = self
                .tun_stacks
                .iter()
                .map(|s| ListItem::new(format!("  {}", s)))
                .collect();
            let tun_list = List::new(tun_items)
                .block(
                    Block::bordered()
                        .border_style(Theme::get().section("settings").border)
                        .title("TUN Stack"),
                )
                .highlight_style(highlight_style);
            f.render_widget(Clear, select_area);
            f.render_stateful_widget(tun_list, select_area, &mut self.tun_selector_state.clone());
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
