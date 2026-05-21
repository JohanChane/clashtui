use super::dev::*;
use crate::config::CONFIG;
use ratatui::{
    layout::{Constraint, Layout},
    text::{Line, Span},
    widgets::{Clear, ListItem, Paragraph},
};
use strum::VariantArray;

newtype_tab!(SettingsTab(Tab<SettingsContent>));

mod_agent!(
    SettingsKey,
    [
        ([KeyCode::Enter], SettingsKey::Execute, "Apply"),
        ([KeyCode::Esc], SettingsKey::Esc, "Back"),
        ([KeyCode::Up], SettingsKey::MoveUp, "Move up"),
        ([KeyCode::Down], SettingsKey::MoveDown, "Move down"),
        ([KeyCode::Char('k')], SettingsKey::MoveUp, "Move up"),
        ([KeyCode::Char('j')], SettingsKey::MoveDown, "Move down"),
    ]
);

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub(crate) enum SettingsKey {
    Execute,
    MoveUp,
    MoveDown,
    Esc,
}

impl TryFrom<&crate::tui::Key> for SettingsKey {
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
}

impl SettingsOp {
    fn all() -> Vec<Self> {
        let mut ops = vec![Self::SwitchMode, Self::AllowLan];
        if CONFIG.core_type() == CoreType::Mihomo {
            ops.push(Self::TunEnable);
            ops.push(Self::TunStackOp);
            ops.push(Self::FlushFakeIP);
            ops.push(Self::FlushDNSCache);
        }
        ops
    }
}

#[derive(Default)]
struct SettingsContent {
    ops: Vec<SettingsOp>,
    current_mode: String,
    allow_lan: bool,
    tun_enable: bool,
    tun_stack: String,
    mode_selector_state: ListState,
    mode_selector_visible: bool,
    tun_selector_state: ListState,
    tun_selector_visible: bool,
    modes: Vec<Mode>,
    tun_stacks: Vec<TunStack>,
}

impl BasicTabContent for SettingsContent {
    type Key = SettingsKey;

    type State = ListState;

    const TITLE: &str = "Settings";

    fn all_shortcuts() -> &'static [(KeyCombo, Self::Key, &'static str)] {
        agent::all_shortcuts()
    }

    fn on_enter(&mut self, _task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        if crate::config::is_core_mismatch() {
            self.current_mode = "core mismatch".to_owned();
            self.allow_lan = false;
            self.tun_enable = false;
            self.tun_stack = "core mismatch".to_owned();
        }
    }
}

impl TabContent for SettingsContent {
    fn init(&mut self, task_set: &mut FutureSet<Self>, state: &mut Self::State) {
        self.ops = SettingsOp::all();
        self.modes = Mode::VARIANTS.to_vec();
        self.tun_stacks = TunStack::VARIANTS.to_vec();
        self.mode_selector_state.select(Some(0));
        self.tun_selector_state.select(Some(0));
        if !self.ops.is_empty() {
            state.select(Some(0));
        }
        if crate::config::is_core_mismatch() {
            self.current_mode = "core mismatch".to_owned();
            self.allow_lan = false;
            self.tun_enable = false;
            self.tun_stack = "core mismatch".to_owned();
            return;
        }

        async move {
            let result = tokio::task::spawn_blocking(|| crate::functions::restful::config::fetch())
                .await
                .unwrap();
            match result {
                Ok(config) => {
                    let mode = config.mode.to_string();
                    let allow_lan = config.allow_lan.unwrap_or(false);
                    let tun_enable = config.tun.as_ref().map(|t| t.enable).unwrap_or(false);
                    let tun_stack = config
                        .tun
                        .as_ref()
                        .map(|t| t.stack.to_string())
                        .unwrap_or_else(|| "Mixed".to_owned());
                    wrapper(move |c: &mut SettingsContent| {
                        c.current_mode = mode;
                        c.allow_lan = allow_lan;
                        c.tun_enable = tun_enable;
                        c.tun_stack = tun_stack;
                    })
                }
                Err(e) => {
                    crate::tui::widget::popmsg::Confirm::err(e);
                    do_nothing()
                }
            }
        }
        .spawn_at(task_set);
    }

    fn handle_key_event(
        &mut self,
        key: SettingsKey,
        task_set: &mut FutureSet<Self>,
        state: &mut Self::State,
    ) {
        if self.mode_selector_visible {
            match key {
                SettingsKey::MoveUp => {
                    let i = self.mode_selector_state.selected().unwrap_or(0);
                    self.mode_selector_state.select(Some(i.saturating_sub(1)));
                }
                SettingsKey::MoveDown => {
                    let i = self.mode_selector_state.selected().unwrap_or(0);
                    if i + 1 < self.modes.len() {
                        self.mode_selector_state.select(Some(i + 1));
                    }
                }
                SettingsKey::Esc => {
                    self.mode_selector_visible = false;
                }
                SettingsKey::Execute => {
                    let idx = self.mode_selector_state.selected().unwrap_or(0);
                    if let Some(mode) = self.modes.get(idx) {
                        let mode = *mode;
                        self.mode_selector_visible = false;
                        async move {
                            if crate::config::is_core_mismatch() {
                                return do_nothing();
                            }
                            let payload = serde_json::json!({"mode": mode.to_string()}).to_string();
                            let result = tokio::task::spawn_blocking(move || {
                                crate::functions::restful::config::patch(payload)
                            })
                            .await
                            .unwrap();
                            match result {
                                Ok(_) => {
                                    let new_val = mode.to_string();
                                    wrapper(move |c: &mut SettingsContent| {
                                        c.current_mode = new_val;
                                    })
                                }
                                Err(e) => {
                                    crate::tui::widget::popmsg::Confirm::err(e);
                                    do_nothing()
                                }
                            }
                        }
                        .spawn_at(task_set);
                    }
                }
            }
            return;
        }

        if self.tun_selector_visible {
            match key {
                SettingsKey::MoveUp => {
                    let i = self.tun_selector_state.selected().unwrap_or(0);
                    self.tun_selector_state.select(Some(i.saturating_sub(1)));
                }
                SettingsKey::MoveDown => {
                    let i = self.tun_selector_state.selected().unwrap_or(0);
                    if i + 1 < self.tun_stacks.len() {
                        self.tun_selector_state.select(Some(i + 1));
                    }
                }
                SettingsKey::Esc => {
                    self.tun_selector_visible = false;
                }
                SettingsKey::Execute => {
                    let idx = self.tun_selector_state.selected().unwrap_or(0);
                    if let Some(stack) = self.tun_stacks.get(idx) {
                        let stack = *stack;
                        self.tun_selector_visible = false;
                        async move {
                            if crate::config::is_core_mismatch() {
                                return do_nothing();
                            }
                            let payload = serde_json::json!({"tun": {"stack": stack.to_string()}})
                                .to_string();
                            let result = tokio::task::spawn_blocking(move || {
                                crate::functions::restful::config::patch(payload)
                            })
                            .await
                            .unwrap();
                            match result {
                                Ok(_) => {
                                    let new_val = stack.to_string();
                                    wrapper(move |c: &mut SettingsContent| {
                                        c.tun_stack = new_val;
                                    })
                                }
                                Err(e) => {
                                    crate::tui::widget::popmsg::Confirm::err(e);
                                    do_nothing()
                                }
                            }
                        }
                        .spawn_at(task_set);
                    }
                }
            }
            return;
        }

        match key {
            SettingsKey::MoveUp => {
                let i = state.selected().unwrap_or(0);
                state.select(Some(i.saturating_sub(1)));
            }
            SettingsKey::MoveDown => {
                let i = state.selected().unwrap_or(0);
                if i + 1 < self.ops.len() {
                    state.select(Some(i + 1));
                }
            }
            SettingsKey::Execute => {
                let Some(idx) = state.selected() else { return };
                let Some(op) = self.ops.get(idx) else { return };
                match op {
                    SettingsOp::SwitchMode => {
                        self.mode_selector_visible = true;
                    }
                    SettingsOp::AllowLan => {
                        let new_val = !self.allow_lan;
                        self.allow_lan = new_val;
                        async move {
                            if crate::config::is_core_mismatch() {
                                return do_nothing();
                            }
                            let payload = serde_json::json!({"allow-lan": new_val}).to_string();
                            let result = tokio::task::spawn_blocking(move || {
                                crate::functions::restful::config::patch(payload)
                            })
                            .await
                            .unwrap();
                            match result {
                                Ok(_) => wrapper(move |c: &mut SettingsContent| {
                                    c.allow_lan = new_val;
                                }),
                                Err(e) => {
                                    crate::tui::widget::popmsg::Confirm::err(e);
                                    wrapper(move |c: &mut SettingsContent| {
                                        c.allow_lan = !new_val;
                                    })
                                }
                            }
                        }
                        .spawn_at(task_set);
                    }
                    SettingsOp::TunEnable => {
                        let new_val = !self.tun_enable;
                        self.tun_enable = new_val;
                        async move {
                            if crate::config::is_core_mismatch() {
                                return do_nothing();
                            }
                            let payload =
                                serde_json::json!({"tun": {"enable": new_val}}).to_string();
                            let result = tokio::task::spawn_blocking(move || {
                                crate::functions::restful::config::patch(payload)
                            })
                            .await
                            .unwrap();
                            match result {
                                Ok(_) => wrapper(move |c: &mut SettingsContent| {
                                    c.tun_enable = new_val;
                                }),
                                Err(e) => {
                                    crate::tui::widget::popmsg::Confirm::err(e);
                                    wrapper(move |c: &mut SettingsContent| {
                                        c.tun_enable = !new_val;
                                    })
                                }
                            }
                        }
                        .spawn_at(task_set);
                    }
                    SettingsOp::TunStackOp => {
                        self.tun_selector_visible = true;
                    }
                    SettingsOp::FlushFakeIP => {
                        async move {
                            if crate::config::is_core_mismatch() {
                                return do_nothing();
                            }
                            let result = tokio::task::spawn_blocking(|| {
                                crate::functions::restful::cache::flush_fakeip()
                            })
                            .await
                            .unwrap();
                            match result {
                                Ok(_) => do_nothing(),
                                Err(e) => {
                                    crate::tui::widget::popmsg::Confirm::err(e);
                                    do_nothing()
                                }
                            }
                        }
                        .spawn_at(task_set);
                    }
                    SettingsOp::FlushDNSCache => {
                        async move {
                            if crate::config::is_core_mismatch() {
                                return do_nothing();
                            }
                            let result = tokio::task::spawn_blocking(|| {
                                crate::functions::restful::cache::flush_dns()
                            })
                            .await
                            .unwrap();
                            match result {
                                Ok(_) => do_nothing(),
                                Err(e) => {
                                    crate::tui::widget::popmsg::Confirm::err(e);
                                    do_nothing()
                                }
                            }
                        }
                        .spawn_at(task_set);
                    }
                }
            }
            _ => {}
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State) {
        let block = Block::bordered()
            .border_style(Theme::get().section("settings").border)
            .title(Self::TITLE);

        if crate::config::is_core_mismatch() {
            let widget = Paragraph::new("API data mismatch with configured core").block(block);
            f.render_widget(widget, area);
            return;
        }

        let value_style = Theme::get().section("settings").muted;

        let items: Vec<ListItem> = self
            .ops
            .iter()
            .map(|op| {
                let (name, current) = match op {
                    SettingsOp::SwitchMode => ("Mode", self.current_mode.as_str()),
                    SettingsOp::AllowLan => {
                        let val = if self.allow_lan { "Yes" } else { "No" };
                        ("Allow LAN", val)
                    }
                    SettingsOp::TunEnable => {
                        let val = if self.tun_enable { "Yes" } else { "No" };
                        ("TUN", val)
                    }
                    SettingsOp::TunStackOp => ("TUN Stack", self.tun_stack.as_str()),
                    SettingsOp::FlushFakeIP => ("Flush Fake-IP", ""),
                    SettingsOp::FlushDNSCache => ("Flush DNS Cache", ""),
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
    fn settings_key_agent_contains_expected() {
        let a = agent();
        assert!(a.contains_key(&mk_key(KeyCode::Enter)));
        assert!(a.contains_key(&mk_key(KeyCode::Esc)));
        assert!(a.contains_key(&mk_key(KeyCode::Up)));
        assert!(a.contains_key(&mk_key(KeyCode::Down)));
        assert!(a.contains_key(&mk_key(KeyCode::Char('k'))));
        assert!(a.contains_key(&mk_key(KeyCode::Char('j'))));
    }

    #[test]
    fn settings_key_try_from_correct_actions() {
        assert!(matches!(
            SettingsKey::try_from(&mk_key(KeyCode::Enter)),
            Ok(SettingsKey::Execute)
        ));
        assert!(matches!(
            SettingsKey::try_from(&mk_key(KeyCode::Esc)),
            Ok(SettingsKey::Esc)
        ));
        assert!(matches!(
            SettingsKey::try_from(&mk_key(KeyCode::Up)),
            Ok(SettingsKey::MoveUp)
        ));
        assert!(matches!(
            SettingsKey::try_from(&mk_key(KeyCode::Down)),
            Ok(SettingsKey::MoveDown)
        ));
        assert!(matches!(
            SettingsKey::try_from(&mk_key(KeyCode::Char('k'))),
            Ok(SettingsKey::MoveUp)
        ));
        assert!(matches!(
            SettingsKey::try_from(&mk_key(KeyCode::Char('j'))),
            Ok(SettingsKey::MoveDown)
        ));
    }

    #[test]
    fn settings_key_try_from_unknown_key_is_err() {
        assert!(SettingsKey::try_from(&mk_key(KeyCode::Char('x'))).is_err());
        assert!(SettingsKey::try_from(&mk_key(KeyCode::Backspace)).is_err());
    }

    #[test]
    fn settings_op_all_is_non_empty() {
        let ops = SettingsOp::all();
        assert!(!ops.is_empty());
        assert!(ops.contains(&SettingsOp::SwitchMode));
        assert!(ops.contains(&SettingsOp::AllowLan));
    }

    #[test]
    fn settings_content_default_has_empty_ops() {
        let c = SettingsContent::default();
        assert!(c.ops.is_empty());
        assert!(!c.mode_selector_visible);
        assert!(!c.tun_selector_visible);
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
