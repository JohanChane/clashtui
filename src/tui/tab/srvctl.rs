use super::dev::*;
use crate::config::CoreType;
use crate::config::scripts::ScriptConfig;
use crate::functions::service::{self, Operation, ServiceState, StatusReport};
use ratatui::widgets::ListItem;
use std::{cell::Cell, path::PathBuf, sync::Arc};

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

#[derive(Clone, Debug)]
struct MenuEntry {
    operation: Operation,
    label: String,
    unavailable_reason: Option<String>,
}

impl MenuEntry {
    fn new(operation: Operation, label: impl Into<String>, scripts: &ScriptConfig) -> Self {
        let missing = operation.missing_actions(scripts);
        let mut label = label.into();
        let unavailable_reason = if missing.is_empty() {
            None
        } else {
            let missing = missing.join(", ");
            label.push_str(&format!(" (unsupported: {missing})"));
            Some(format!("Missing actions: {missing}"))
        };
        Self {
            operation,
            label,
            unavailable_reason,
        }
    }

    fn key(&self) -> &str {
        match &self.operation {
            Operation::Script { id, .. } => id,
            Operation::SwitchCore(_) => ":switch_core",
            Operation::StopAll => ":stop_all",
        }
    }
}

fn menu(scripts: &ScriptConfig, core: CoreType) -> Vec<MenuEntry> {
    let mut entries: Vec<_> = scripts
        .actions_for(core)
        .map(|action| MenuEntry {
            operation: Operation::Script {
                core,
                id: action.id.clone(),
            },
            label: action.label.clone(),
            unavailable_reason: None,
        })
        .collect();
    if !entries.is_empty() {
        entries.push(MenuEntry::new(
            Operation::SwitchCore(core.other()),
            "Switch Core",
            scripts,
        ));
        entries.push(MenuEntry::new(
            Operation::StopAll,
            "Stop All Services",
            scripts,
        ));
    }
    entries
}

#[derive(Default)]
struct StatusQuery {
    report: Option<StatusReport>,
    pending: Cell<bool>,
    refresh: Cell<bool>,
}

#[derive(Default)]
struct SrvCtlContent {
    entries: Vec<MenuEntry>,
    scripts: Option<Arc<ScriptConfig>>,
    root: PathBuf,
    core: CoreType,
    generation: u64,
    statuses: [StatusQuery; 2],
    error: Option<String>,
    visible: bool,
    busy: bool,
    revision: u64,
    watch_pending: Cell<bool>,
}

impl SrvCtlContent {
    fn reload(&mut self, core: CoreType, state: &mut ListState) {
        let selected = (self.core == core)
            .then(|| {
                state
                    .selected()
                    .and_then(|index| self.entries.get(index))
                    .map(|entry| entry.key().to_owned())
            })
            .flatten();
        self.core = core;
        // Clear before parsing: invalid edits must never leave executable stale actions.
        self.scripts = None;
        self.entries.clear();
        self.error = None;
        match ScriptConfig::load(&self.root) {
            Ok(scripts) => {
                self.entries = menu(&scripts, core);
                self.scripts = Some(Arc::new(scripts));
            }
            Err(error) => self.error = Some(format!("{error:#}")),
        }
        let index = selected
            .and_then(|id| self.entries.iter().position(|entry| entry.key() == id))
            .or_else(|| (!self.entries.is_empty()).then_some(0));
        state.select(index);
        self.invalidate_statuses(service::revision());
    }

    fn spawn_status_checks(&self, tasks: &mut FutureSet<Self>) {
        if !self.visible || self.busy {
            return;
        }
        let Some(scripts) = &self.scripts else {
            return;
        };
        for core in CoreType::ALL {
            let index = core.index();
            let status = &self.statuses[index];
            if status.pending.get() || !status.refresh.get() {
                continue;
            }
            status.pending.set(true);
            status.refresh.set(false);
            let scripts = Arc::clone(scripts);
            let root = self.root.clone();
            let generation = self.generation;
            async move {
                let report = tokio::task::spawn_blocking(move || {
                    service::query_status(&scripts, &root, core)
                })
                .await
                .unwrap_or_else(StatusReport::unknown);
                wrapper(move |content: &mut Self| {
                    content.complete_status(core, generation, report);
                })
            }
            .spawn_at(tasks);
        }
    }

    fn watch_operations(&self, tasks: &mut FutureSet<Self>) {
        if !self.visible || self.watch_pending.replace(true) {
            return;
        }
        async {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            wrapper(|content: &mut Self| {
                content.watch_pending.set(false);
                if !content.visible {
                    return;
                }
                content.refresh_after_operation(service::revision());
            })
        }
        .spawn_at(tasks);
    }

    fn invalidate_statuses(&mut self, revision: u64) {
        self.revision = revision;
        self.generation = self.generation.wrapping_add(1);
        for status in &mut self.statuses {
            status.report = None;
            status.refresh.set(self.scripts.is_some());
        }
    }

    fn complete_status(&mut self, core: CoreType, generation: u64, report: StatusReport) {
        let status = &mut self.statuses[core.index()];
        status.pending.set(false);
        if self.generation == generation {
            status.report = Some(report);
        }
    }

    fn refresh_after_operation(&mut self, revision: u64) {
        if self.revision != revision {
            self.invalidate_statuses(revision);
        }
        for status in &self.statuses {
            if status
                .report
                .as_ref()
                .is_some_and(|report| report.state == ServiceState::Checking)
            {
                status.refresh.set(true);
            }
        }
    }
}

impl BasicTabContent for SrvCtlContent {
    type Action = Action;

    type State = ListState;
    const TITLE: &str = "CoreSrvCtl";

    fn on_enter(&mut self, tasks: &mut FutureSet<Self>, state: &mut Self::State) {
        self.visible = true;
        self.reload(crate::config::CONFIG.core_type(), state);
        self.spawn_status_checks(tasks);
        self.watch_operations(tasks);
    }

    fn on_leave(&mut self, _tasks: &mut FutureSet<Self>, _state: &mut Self::State) {
        self.visible = false;
    }

    fn after_sync(&self, tasks: &mut FutureSet<Self>) {
        self.spawn_status_checks(tasks);
        self.watch_operations(tasks);
    }
}

impl TabContent for SrvCtlContent {
    fn init(&mut self, _tasks: &mut FutureSet<Self>, _state: &mut Self::State) {
        self.root = crate::config::config_dir_path();
    }

    fn handle_action(
        &mut self,
        action: Action,
        tasks: &mut FutureSet<Self>,
        state: &mut Self::State,
    ) {
        if self.busy {
            return;
        }
        match action {
            Action::Esc => {}
            Action::MoveUp => {
                if !self.entries.is_empty() {
                    state.select(Some(state.selected().unwrap_or(0).saturating_sub(1)));
                }
            }
            Action::MoveDown => {
                let index = state.selected().unwrap_or(0);
                if index + 1 < self.entries.len() {
                    state.select(Some(index + 1));
                }
            }
            Action::Execute => {
                let Some(entry) = state
                    .selected()
                    .and_then(|index| self.entries.get(index))
                    .cloned()
                else {
                    return;
                };
                if let Some(reason) = &entry.unavailable_reason {
                    Confirm::err(reason);
                    return;
                }
                let Some(scripts) = self.scripts.clone() else {
                    return;
                };
                let root = self.root.clone();
                self.busy = true;
                let operation = entry.operation;
                let switching = matches!(operation, Operation::SwitchCore(_));
                async move {
                    let result =
                        tokio::task::spawn_blocking(move || operation.execute(&scripts, &root))
                            .await
                            .unwrap_or_else(|error| Err(error.into()));
                    if let Err(error) = &result {
                        Confirm::err(format!("{error:#}"));
                    }
                    if switching && result.is_ok() {
                        crate::tui::app::QUIT.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    wrapper(move |content: &mut Self| {
                        content.busy = false;
                        content.invalidate_statuses(service::revision());
                    })
                }
                .spawn_at(tasks);
            }
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, state: &mut Self::State) {
        use ratatui::{
            layout::{Constraint, Layout},
            widgets::Paragraph,
        };
        let theme = Theme::get();
        let section = theme.section("srvctl");
        let status = CoreType::ALL
            .map(|core| {
                let label = self.statuses[core.index()]
                    .report
                    .as_ref()
                    .map_or("...", |report| report.state.as_str());
                format!(
                    "{}{}: {label}",
                    if core == self.core { "► " } else { "" },
                    core
                )
            })
            .join("   ");
        let block = Block::bordered()
            .border_style(section.border)
            .title(format!(
                "{} — {}{}",
                Self::TITLE,
                self.core,
                if self.busy { " (running)" } else { "" }
            ))
            .title_bottom(status);
        if let Some(error) = &self.error {
            frame.render_widget(
                Paragraph::new(error.as_str())
                    .wrap(ratatui::widgets::Wrap { trim: false })
                    .block(block),
                area,
            );
            return;
        }
        if self.entries.is_empty() {
            frame.render_widget(
                Paragraph::new("未配置服务动作 / No service actions configured").block(block),
                area,
            );
            return;
        }
        let diagnostics = CoreType::ALL
            .iter()
            .filter_map(|core| {
                self.statuses[core.index()]
                    .report
                    .as_ref()
                    .and_then(|report| {
                        report
                            .diagnostic
                            .as_ref()
                            .map(|text| format!("{core}: {text}"))
                    })
            })
            .collect::<Vec<_>>()
            .join("\n");
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let [list_area, info_area] = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(if diagnostics.is_empty() { 0 } else { 3 }),
        ])
        .areas(inner);
        let items: Vec<_> = self
            .entries
            .iter()
            .map(|entry| {
                let item = ListItem::new(format!("  {}", entry.label));
                if entry.unavailable_reason.is_some() {
                    item.fg(Color::DarkGray)
                } else {
                    item
                }
            })
            .collect();
        frame.render_stateful_widget(
            List::new(items).highlight_style(section.highlight),
            list_area,
            state,
        );
        frame.render_widget(
            Paragraph::new(diagnostics).wrap(ratatui::widgets::Wrap { trim: false }),
            info_area,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scripts(value: serde_json::Value) -> ScriptConfig {
        let config: ScriptConfig = serde_json::from_value(value).unwrap();
        config.validate().unwrap();
        config
    }

    #[test]
    fn menu_preserves_order_and_filters_core_commands() {
        let config = scripts(serde_json::json!({"root":".","actions":[
            {"id":"custom","label":"First","cores":["singbox"],"cmd":{"program":"tool","args":[]}},
            {"id":"start_core","label":"Second","cmd":{"program":"tool","args":[]}},
            {"id":"custom","label":"Third","cores":["mihomo"],"cmd":{"program":"tool","args":[]}}
        ]}));
        let labels: Vec<_> = menu(&config, CoreType::Mihomo)
            .iter()
            .map(|entry| entry.label.clone())
            .collect();
        assert_eq!(&labels[..2], ["Second", "Third"]);
        assert!(labels[2].contains("unsupported: mihomo/stop_core, sing-box/stop_core"));
        assert!(labels[3].contains("unsupported:"));
        let empty = scripts(serde_json::json!({"root":".","actions":[]}));
        assert!(menu(&empty, CoreType::Mihomo).is_empty());
    }

    #[test]
    fn reload_keeps_identity_and_invalid_config_disables_old_commands() {
        let root = std::env::temp_dir().join(format!(
            "clashtui-reload-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        std::fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let path = root.join("scripts.json");
        let action =
            |id: &str| serde_json::json!({"id":id,"label":id,"cmd":{"program":"tool","args":[]}});
        let write = |actions| {
            std::fs::write(
                &path,
                serde_json::json!({"root":".","actions":actions}).to_string(),
            )
            .unwrap()
        };
        write(vec![action("first"), action("second")]);
        let mut content = SrvCtlContent {
            root: root.clone(),
            ..Default::default()
        };
        let mut state = ListState::default();
        content.reload(CoreType::Mihomo, &mut state);
        state.select(Some(1));
        write(vec![action("second"), action("first")]);
        content.reload(CoreType::Mihomo, &mut state);
        assert_eq!(state.selected(), Some(0));
        std::fs::write(&path, "bad json").unwrap();
        content.reload(CoreType::Mihomo, &mut state);
        assert!(content.scripts.is_none() && content.entries.is_empty() && content.error.is_some());
        assert_eq!(state.selected(), None);
        std::fs::remove_file(&path).unwrap();
        content.reload(CoreType::Mihomo, &mut state);
        assert!(content.scripts.is_none() && content.entries.is_empty());
        assert!(
            content
                .error
                .as_ref()
                .unwrap()
                .contains("run the installer")
        );
        assert_eq!(state.selected(), None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn global_operation_invalidates_status_and_busy_query_gets_retried() {
        let mut content = SrvCtlContent {
            scripts: Some(Arc::new(scripts(
                serde_json::json!({"root":".","actions":[]}),
            ))),
            ..Default::default()
        };
        content.statuses[0].report = Some(StatusReport {
            state: ServiceState::Checking,
            diagnostic: None,
        });
        content.refresh_after_operation(0);
        assert!(content.statuses[0].refresh.get());
        assert!(!content.statuses[1].refresh.get());
        content.statuses[0].report = Some(StatusReport {
            state: ServiceState::Active,
            diagnostic: None,
        });
        content.statuses[0].refresh.set(false);
        let generation = content.generation;
        content.refresh_after_operation(1);
        assert!(
            content
                .statuses
                .iter()
                .all(|status| status.report.is_none())
        );
        assert!(content.statuses.iter().all(|status| status.refresh.get()));
        assert_ne!(content.generation, generation);
    }

    #[test]
    fn invalidation_waits_for_old_queries_and_discards_their_results() {
        for configured in [true, false] {
            let mut content = SrvCtlContent {
                scripts: configured.then(|| {
                    Arc::new(scripts(serde_json::json!({
                        "root": ".", "actions": []
                    })))
                }),
                ..Default::default()
            };
            let generation = content.generation;
            for status in &content.statuses {
                status.pending.set(true);
            }
            content.invalidate_statuses(1);
            for core in CoreType::ALL {
                assert!(content.statuses[core.index()].pending.get());
                content.complete_status(
                    core,
                    generation,
                    StatusReport {
                        state: ServiceState::Active,
                        diagnostic: None,
                    },
                );
                let status = &content.statuses[core.index()];
                assert!(!status.pending.get());
                assert!(status.report.is_none());
                assert_eq!(status.refresh.get(), configured);
            }
        }
    }
}
