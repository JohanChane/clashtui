use super::dev::*;
use crate::{
    config::CoreType,
    scripts::{self, ActionRequest, StatusReport, config::ActionDefinition},
    tui::commands::{CommandEvent, Commands},
};
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
}

impl Document for Action {
    fn get_doc(&self) -> &'static str {
        use crate::tui::key::consts::*;
        match self {
            Self::Execute => "Execute",
            Self::MoveUp => MOVE_UP,
            Self::MoveDown => MOVE_DOWN,
        }
    }
}

#[derive(Default)]
struct SrvCtlContent {
    entries: Vec<&'static ActionDefinition>,
    core: CoreType,
    statuses: [Option<Result<StatusReport, String>>; 2],
    error: Option<String>,
}
impl BasicTabContent for SrvCtlContent {
    type Action = Action;
    type State = ListState;
    const TITLE: &str = "CoreSrvCtl";
    fn command_done(&mut self, event: &CommandEvent, _: &mut FutureSet<Self>) {
        match event {
            CommandEvent::Status { core, result, .. } => {
                self.statuses[core.index()] = Some(
                    result
                        .as_ref()
                        .cloned()
                        .map_err(super::super::commands::error_text),
                )
            }
            CommandEvent::Script { target, .. } if target.changes_service => {
                self.statuses[target.core.index()] = None
            }
            _ => (),
        }
    }
}
impl TabContent for SrvCtlContent {
    fn init(&mut self, _: &mut FutureSet<Self>, state: &mut ListState) {
        self.core = crate::config::CONFIG.core_type();
        match scripts::get() {
            Ok(config) => self.entries = config.menu(self.core).collect(),
            Err(error) => self.error = Some(format!("{error:#}")),
        }
        state.select((!self.entries.is_empty()).then_some(0));
    }
    fn handle_action(
        &mut self,
        action: Action,
        _: &mut FutureSet<Self>,
        state: &mut ListState,
        commands: &mut Commands,
    ) {
        match action {
            Action::MoveUp => {
                if !self.entries.is_empty() {
                    state.select(Some(state.selected().unwrap_or(0).saturating_sub(1)));
                }
            }
            Action::MoveDown => {
                if state.selected().unwrap_or(0) + 1 < self.entries.len() {
                    state.select(Some(state.selected().unwrap_or(0) + 1));
                }
            }
            Action::Execute => {
                if let Some(entry) = state.selected().and_then(|index| self.entries.get(index))
                    && let Err(error) =
                        commands.submit(self.core, ActionRequest::Run(entry.id.clone()))
                {
                    Confirm::err(format!("{error:#}"));
                }
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
                    .as_ref()
                    .map_or("...", |report| {
                        report
                            .as_ref()
                            .map_or("unknown", |report| report.state.as_str())
                    });
                format!(
                    "{}{}: {label}",
                    if core == self.core { "► " } else { "" },
                    core
                )
            })
            .join("   ");
        let block = Block::bordered()
            .border_style(section.border)
            .title(format!("{} — {}", Self::TITLE, self.core))
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
                let report = self.statuses[core.index()].as_ref()?;
                let text = match report {
                    Ok(report) => report.diagnostic.as_deref()?,
                    Err(error) => error.as_str(),
                };
                Some(format!("{core}: {text}"))
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
                ListItem::new(format!("  {}", entry.label.as_deref().unwrap_or(&entry.id)))
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
