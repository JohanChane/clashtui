use ratatui::prelude as Ra;
use Ra::{Constraint, Direction, Layout, Rect};

/// Create a centered rect using up certain percentage of the available rect `r`
pub fn centered_percent_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

/// Create a centered rect using specific lengths for width and height
pub fn centered_lenght_rect(width: u16, height: u16, container: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((container.height - height) / 2),
            Constraint::Length(height),
            Constraint::Length((container.height - height) / 2),
        ])
        .split(container);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((container.width - width) / 2),
            Constraint::Length(width),
            Constraint::Length((container.width - width) / 2),
        ])
        .split(popup_layout[1])[1]
}
