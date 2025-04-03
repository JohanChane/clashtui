use Ra::{Constraint, Layout, Rect};
use ratatui::prelude as Ra;

/// Create a centered [Rect] from `raw`
pub fn centered_rect(width: Constraint, height: Constraint, raw: Rect) -> Rect {
    let he = Layout::vertical([Constraint::Min(0), height, Constraint::Min(0)]).split(raw);
    Layout::horizontal([Constraint::Min(0), width, Constraint::Min(0)]).split(he[1])[1]
}
