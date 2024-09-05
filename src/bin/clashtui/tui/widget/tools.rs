use ratatui::prelude as Ra;
use Ra::{Constraint, Direction, Layout, Rect};
#[deprecated = "use `centered_rect` instand"]
/// Create a centered rect using up certain percentage of the available rect `r`
pub fn centered_percent_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    centered_rect(
        Constraint::Percentage(percent_x),
        Constraint::Percentage(percent_y),
        r,
    )
}

/// Create a centered [Rect] from `raw`
pub fn centered_rect(width: Constraint, height: Constraint, raw: Rect) -> Rect {
    let he = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), height, Constraint::Min(0)])
        .split(raw);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), width, Constraint::Min(0)].as_ref())
        .split(he[1])[1]
}

#[deprecated = "use `centered_rect` instand"]
/// Create a centered rect using specific lengths for width and height
pub fn centered_lenght_rect(width: u16, height: u16, container: Rect) -> Rect {
    centered_rect(
        Constraint::Length(width),
        Constraint::Length(height),
        container,
    )
}
