use super::dev::*;
use crate::config::CoreType;
use crate::config::scripts::ScriptConfig;
use crate::functions::service::{self, CORES, ServiceState, StatusReport, core_index};
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum Operation {
    Script {
        id: String,
        label: String,
    },
    SwitchCore,
    StopAll,
    Unavailable {
        id: &'static str,
        label: String,
        reason: String,
    },
}

impl Operation {
    fn key(&self) -> &str {
        match self {
            Self::Script { id, .. } => id,
            Self::SwitchCore => ":switch_core",
            Self::StopAll => ":stop_all",
            Self::Unavailable { id, .. } => id,
        }
    }
    fn label(&self) -> &str {
        match self {
            Self::Script { label, .. } => label,
            Self::SwitchCore => "Switch Core",
            Self::StopAll => "Stop All Services",
            Self::Unavailable { label, .. } => label,
        }
    }
}

fn menu(scripts: &ScriptConfig, core: CoreType) -> Vec<Operation> {
    let mut ops: Vec<_> = scripts
        .actions_for(core)
        .map(|action| Operation::Script {
            id: action.id.clone(),
            label: action.label.clone(),
        })
        .collect();
    if !ops.is_empty() {
        let missing_stops: Vec<_> = CORES
            .iter()
            .filter(|core| scripts.find(**core, "stop_core").is_none())
            .map(|core| format!("{core}/stop_core"))
            .collect();
        let other = match core {
            CoreType::Mihomo => CoreType::Singbox,
            CoreType::Singbox => CoreType::Mihomo,
        };
        let mut missing_switch = missing_stops.clone();
        if scripts.find(other, "start_core").is_none() {
            missing_switch.push(format!("{other}/start_core"));
        }
        let capability = |op: Operation, missing: Vec<String>| {
            if missing.is_empty() {
                op
            } else {
                Operation::Unavailable {
                    id: if op == Operation::SwitchCore {
                        ":switch_core"
                    } else {
                        ":stop_all"
                    },
                    label: format!("{} (unsupported: {})", op.label(), missing.join(", ")),
                    reason: format!("Missing actions: {}", missing.join(", ")),
                }
            }
        };
        ops.push(capability(Operation::SwitchCore, missing_switch));
        ops.push(capability(Operation::StopAll, missing_stops));
    }
    ops
}

#[derive(Default)]
struct SrvCtlContent {
    ops: Vec<Operation>,
    scripts: Option<Arc<ScriptConfig>>,
    root: PathBuf,
    core: CoreType,
    generation: u64,
    statuses: [Option<StatusReport>; 2],
    pending: [Cell<bool>; 2],
    refresh: [Cell<bool>; 2],
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
                    .and_then(|index| self.ops.get(index))
                    .map(|op| op.key().to_owned())
            })
            .flatten();
        self.core = core;
        self.generation = self.generation.wrapping_add(1);
        self.revision = service::revision();
        self.statuses = [None, None];
        // Clear before parsing: invalid edits must never leave executable stale actions.
        self.scripts = None;
        self.ops.clear();
        self.error = None;
        match ScriptConfig::load(&self.root) {
            Ok(scripts) => {
                self.ops = menu(&scripts, core);
                self.scripts = Some(Arc::new(scripts));
            }
            Err(error) => self.error = Some(format!("{error:#}")),
        }
        let index = selected
            .and_then(|id| self.ops.iter().position(|op| op.key() == id))
            .or_else(|| (!self.ops.is_empty()).then_some(0));
        state.select(index);
        for refresh in &self.refresh {
            refresh.set(self.scripts.is_some());
        }
    }

    fn spawn_status_checks(&self, tasks: &mut FutureSet<Self>) {
        if !self.visible || self.busy {
            return;
        }
        let Some(scripts) = &self.scripts else {
            return;
        };
        for core in CORES {
            let index = core_index(core);
            if self.pending[index].get() || !self.refresh[index].get() {
                continue;
            }
            self.pending[index].set(true);
            self.refresh[index].set(false);
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
                    content.pending[index].set(false);
                    if content.generation == generation {
                        content.statuses[index] = Some(report);
                    }
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

    fn refresh_after_operation(&mut self, revision: u64) {
        if self.revision != revision {
            self.revision = revision;
            self.generation = self.generation.wrapping_add(1);
            self.statuses = [None, None];
            for refresh in &self.refresh {
                refresh.set(true);
            }
        }
        for (index, status) in self.statuses.iter().enumerate() {
            if status
                .as_ref()
                .is_some_and(|report| report.state == ServiceState::Checking)
            {
                self.refresh[index].set(true);
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
                if !self.ops.is_empty() {
                    state.select(Some(state.selected().unwrap_or(0).saturating_sub(1)));
                }
            }
            Action::MoveDown => {
                let index = state.selected().unwrap_or(0);
                if index + 1 < self.ops.len() {
                    state.select(Some(index + 1));
                }
            }
            Action::Execute => {
                let Some(op) = state
                    .selected()
                    .and_then(|index| self.ops.get(index))
                    .cloned()
                else {
                    return;
                };
                if let Operation::Unavailable { reason, .. } = &op {
                    Confirm::err(reason);
                    return;
                }
                let Some(scripts) = self.scripts.clone() else {
                    return;
                };
                let root = self.root.clone();
                let core = self.core;
                self.busy = true;
                let switching = op == Operation::SwitchCore;
                async move {
                    let result = tokio::task::spawn_blocking(move || match op {
                        Operation::Script { id, .. } => {
                            service::execute(&scripts, &root, core, &id)
                        }
                        Operation::StopAll => service::stop_all(&scripts, &root),
                        Operation::Unavailable { .. } => unreachable!("disabled action"),
                        Operation::SwitchCore => service::switch_core(
                            &scripts,
                            &root,
                            match core {
                                CoreType::Mihomo => CoreType::Singbox,
                                CoreType::Singbox => CoreType::Mihomo,
                            },
                        ),
                    })
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
                        content.revision = service::revision();
                        content.generation = content.generation.wrapping_add(1);
                        content.statuses = [None, None];
                        for refresh in &content.refresh {
                            refresh.set(true);
                        }
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
        let status = CORES
            .map(|core| {
                let label = self.statuses[core_index(core)]
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
        if self.ops.is_empty() {
            frame.render_widget(
                Paragraph::new("未配置服务动作 / No service actions configured").block(block),
                area,
            );
            return;
        }
        let diagnostics = CORES
            .iter()
            .filter_map(|core| {
                self.statuses[core_index(*core)]
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
            .ops
            .iter()
            .map(|op| {
                let item = ListItem::new(format!("  {}", op.label()));
                if matches!(op, Operation::Unavailable { .. }) {
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
            .map(|op| op.label().to_owned())
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
        assert!(content.scripts.is_none() && content.ops.is_empty() && content.error.is_some());
        assert_eq!(state.selected(), None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn global_operation_invalidates_status_and_busy_query_gets_retried() {
        let mut content = SrvCtlContent::default();
        content.statuses[0] = Some(StatusReport {
            state: ServiceState::Checking,
            diagnostic: None,
        });
        content.refresh_after_operation(0);
        assert!(content.refresh[0].get());
        assert!(!content.refresh[1].get());
        content.statuses[0] = Some(StatusReport {
            state: ServiceState::Active,
            diagnostic: None,
        });
        content.refresh[0].set(false);
        let generation = content.generation;
        content.refresh_after_operation(1);
        assert!(content.statuses.iter().all(Option::is_none));
        assert!(content.refresh.iter().all(Cell::get));
        assert_ne!(content.generation, generation);
    }
}
