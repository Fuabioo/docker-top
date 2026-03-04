use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::theme::{self, MUTED, PRIMARY, SUCCESS, WARNING};

pub fn render(frame: &mut Frame, area: Rect, connected: bool, last_poll_secs: u64) {
    let docker_icon = Span::styled(
        format!("{} ", theme::ICON_DOCKER),
        ratatui::style::Style::default()
            .fg(PRIMARY)
            .add_modifier(Modifier::BOLD),
    );

    let title = Span::styled(
        "docker-top  ",
        ratatui::style::Style::default()
            .fg(PRIMARY)
            .add_modifier(Modifier::BOLD),
    );

    let status = if connected {
        Span::styled("Connected", ratatui::style::Style::default().fg(SUCCESS))
    } else {
        Span::styled("Disconnected", ratatui::style::Style::default().fg(WARNING))
    };

    let poll_text = format!("  Last poll: {}s ago", last_poll_secs);
    let poll = Span::styled(poll_text, ratatui::style::Style::default().fg(MUTED));

    let line = Line::from(vec![docker_icon, title, status, poll]);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}
