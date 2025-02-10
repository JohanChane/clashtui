use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{prelude as Ra, widgets as Raw};

use super::tools;
use crate::tui::misc::EventState;
use crate::tui::{Drawable, Theme};

pub struct Browser {
    cwd: PathBuf,
    items: Vec<Box<dyn Fp>>,
    selected: usize,
}

impl Drawable for Browser {
    fn render(&mut self, f: &mut ratatui::Frame, _: ratatui::layout::Rect, _: bool) {
        let area = tools::centered_rect(Ra::Constraint::Fill(2), Ra::Constraint::Fill(2), f.area());

        let mut state = Raw::ListState::default().with_selected(Some(self.selected));

        let list = Raw::List::new(self.items.iter().map(|file| {
            Ra::Text::raw(file.name()).style(Ra::Style::default().fg(if file.is_dir() {
                Ra::Color::LightCyan
            } else {
                Ra::Color::default()
            }))
        }))
        .scroll_padding((area.height - 3).div_ceil(2) as usize)
        .highlight_style(
            Ra::Style::default()
                .bg(Theme::get().list_hl_bg_fouced)
                .add_modifier(Ra::Modifier::BOLD),
        )
        .block(
            Raw::Block::bordered()
                .border_style(Ra::Style::default().fg(Theme::get().list_block_fouced_fg))
                .title_top(format!("Browser: {}", self.cwd.display()))
                .title_bottom(Ra::Line::raw("Press 'O' to open selected dir/file").right_aligned()),
        );

        f.render_widget(Raw::Clear, area);
        f.render_stateful_widget(list, area, &mut state);
    }

    fn handle_key_event(&mut self, ev: &crossterm::event::KeyEvent) -> EventState {
        if ev.kind != KeyEventKind::Press {
            return EventState::NotConsumed;
        }
        match ev.code {
            KeyCode::Right | KeyCode::Enter => {
                if self.items[self.selected].is_dir() {
                    self.cwd = self.items.swap_remove(self.selected).path();
                    self.update();
                } else {
                    return EventState::Yes;
                }
            }
            KeyCode::Left | KeyCode::Backspace => {
                if self.items[0].name() == UPPER_DIR {
                    self.cwd = self.items.swap_remove(0).path();
                    self.update();
                }
            }

            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => self.selected = (self.selected + 1).min(self.items.len() - 1),

            KeyCode::Home => self.selected = 0,
            KeyCode::End => self.selected = self.items.len() - 1,

            KeyCode::Char('O') | KeyCode::Char('o') => return EventState::Yes,
            KeyCode::Esc => return EventState::Cancel,
            _ => (),
        };
        EventState::WorkDone
    }
}

impl Browser {
    pub fn new(path: &std::path::Path) -> Self {
        let mut instance = Self {
            cwd: path.to_path_buf(),
            items: vec![],
            selected: 0,
        };
        instance.update();
        instance
    }
    pub fn path(mut self) -> std::path::PathBuf {
        self.items.swap_remove(self.selected).path()
    }
    fn update(&mut self) {
        if let Err(e) = self.get_and_set_files() {
            let err = format!("Failed to open {}: {e}", self.cwd.display());
            log::error!("{err}");
            self.items = vec![Box::new(FsErr(err))];
        };
        self.selected = 0
    }
    fn get_and_set_files(&mut self) -> std::io::Result<()> {
        let (mut dirs, mut none_dirs): (Vec<_>, Vec<_>) = std::fs::read_dir(&self.cwd)?
            .filter_map(|entry| entry.ok())
            .map(|e| {
                let path = e.path();
                Box::new(FileOrDir(path)).to_dyn()
            })
            .partition(|file| file.is_dir());

        dirs.sort_unstable_by(|f1, f2| f1.name().cmp(&f2.name()));
        none_dirs.sort_unstable_by(|f1, f2| f1.name().cmp(&f2.name()));

        if let Some(parent) = self.cwd.parent() {
            let mut files: Vec<Box<dyn Fp>> = Vec::with_capacity(1 + dirs.len() + none_dirs.len());

            files.push(Box::new(UpperDir(parent.to_path_buf())));

            files.extend(dirs);
            files.extend(none_dirs);

            self.items = files
        } else {
            let mut files = Vec::with_capacity(dirs.len() + none_dirs.len());

            files.extend(dirs);
            files.extend(none_dirs);

            self.items = files;
        };

        Ok(())
    }
}

const UPPER_DIR: &str = "../";
trait Fp: Send {
    fn name(&self) -> std::borrow::Cow<'_, str>;
    fn is_dir(&self) -> bool;
    fn path(self: Box<Self>) -> std::path::PathBuf;
    #[inline]
    fn to_dyn(self: Box<Self>) -> Box<dyn Fp>
    where
        Self: Sized + 'static,
    {
        self
    }
}
#[derive(Debug)]
struct FileOrDir(PathBuf);
impl Fp for FileOrDir {
    /// the path is built from [std::fs::DirEntry]
    /// thus it will never end in '.','..'
    fn name(&self) -> std::borrow::Cow<'_, str> {
        self.0.file_name().unwrap().to_string_lossy()
    }
    fn is_dir(&self) -> bool {
        self.0.is_dir()
    }
    fn path(self: Box<FileOrDir>) -> std::path::PathBuf {
        self.0
    }
}
struct UpperDir(PathBuf);
impl Fp for UpperDir {
    fn name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed(UPPER_DIR)
    }
    fn is_dir(&self) -> bool {
        self.0.is_dir()
    }
    fn path(self: Box<UpperDir>) -> std::path::PathBuf {
        self.0
    }
}
struct FsErr(String);
impl Fp for FsErr {
    fn name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed(&self.0)
    }
    fn is_dir(&self) -> bool {
        true
    }
    /// current, just redirect to temp dir
    fn path(self: Box<Self>) -> std::path::PathBuf {
        std::env::temp_dir()
    }
}
