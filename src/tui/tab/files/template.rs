use super::*;
use crate::functions::file::template::*;

mod_agent!(
    Key,
    [
        (KeyCode::Left, Key::Switch),
        (KeyCode::Right, Key::Switch),
        (KeyCode::Down, Key::MoveDown),
        (KeyCode::Up, Key::MoveUp),
        (KeyCode::Char('d'), Key::Action(Action::Delete)),
        (KeyCode::Char('p'), Key::Action(Action::Preview)),
        (KeyCode::Enter, Key::Action(Action::Generate)),
    ]
);

#[derive(Clone, Copy, serde::Deserialize)]
pub enum Action {
    Generate,
    Delete,
    Preview,
}

#[derive(Clone, Copy, serde::Deserialize)]
pub enum Key {
    Switch,
    MoveUp,
    MoveDown,
    Select,

    Action(Action),
}

impl TryFrom<&KeyEvent> for Key {
    type Error = ();

    fn try_from(value: &KeyEvent) -> Result<Self, Self::Error> {
        let agent = agent();
        if !agent.is_empty() {
            return agent.get(value).map(|act| *act).ok_or(());
        }

        if value.kind != crossterm::event::KeyEventKind::Press {
            return Err(());
        }
        Ok(match value.code {
            KeyCode::Enter => Self::Select,
            KeyCode::Right | KeyCode::Left => Self::Switch,
            KeyCode::Down => Self::MoveDown,
            KeyCode::Up => Self::MoveUp,

            _ => return Err(()),
        })
    }
}

#[derive(Default)]
pub struct Template {
    items: Vec<String>,
    filter: Option<String>,
}

impl BasicTabContent for Template {
    type Key = Key;
    type State = ListState;

    const TITLE: &str = "Template";
}

impl DualTabContentMate for Template {
    type Mate = super::profile::Profile;

    fn init(&mut self, task_set: &mut FutureSet<(Self::Mate, Self)>, _: &mut Self::State) {
        async {
            let templates = tri!(get_all_templates());
            wrapper(|(_, content): &mut (Self::Mate, Self)| content.items = templates)
        }
        .spawn_at(task_set);
    }

    fn handle_key_event(
        &mut self,
        key: Self::Key,
        task_set: &mut FutureSet<(Self::Mate, Self)>,
        state: &mut Self::State,
    ) -> bool {
        match key {
            Key::Switch => return true,
            Key::MoveDown => state.select_next(),
            Key::MoveUp => state.select_previous(),

            Key::Select => todo!(),

            Key::Action(action) => {
                let name = get_name!(self, state);
                action.act(name).spawn_at(task_set)
            }
        }
        false
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State, is_focused: bool) {
        let block = Block::bordered()
            .border_style(if is_focused {
                Theme::get().tab.tab_focused
            } else {
                Theme::get().tab.dualtab_unfocused
            })
            .title(Self::TITLE);

        let block = if let Some(filter) = self.filter.as_ref() {
            block.title_bottom(Line::raw(format!(" {filter} ")).right_aligned().reversed())
        } else {
            block.title_bottom(Line::raw(format!(" /: Search ")).right_aligned().reversed())
        };

        let iter = self
            .items
            .iter()
            // filter content now
            .filter_map(|value| {
                self.filter
                    .as_deref()
                    .is_none_or(|pat| value.contains(pat))
                    .then_some(value.as_str())
            });
        let widget = List::from_iter(iter)
            .block(block)
            .highlight_style(if is_focused {
                Theme::get().tab.item_highlighted
            } else {
                Theme::get().tab.item_unhighlighted
            });
        f.render_stateful_widget(widget, area, state);
    }
}

mod actions {
    use super::*;

    impl Action {
        pub async fn act(self, name: String) -> CB {
            match self {
                Self::Generate => generate(name).await,
                Self::Delete => delete(name).await,
                Self::Preview => preview(name).await,
            }
        }
    }

    type CB = Box<dyn for<'a> FnOnce(&'a mut C) + Send + 'static>;
    type C = (<Template as DualTabContentMate>::Mate, Template);

    async fn generate(name: String) -> CB {
        tri!(apply_template(name));
        sync!(C)
    }

    async fn delete(name: String) -> CB {
        todo!()
    }

    async fn preview(name: String) -> CB {
        todo!()
    }
}
