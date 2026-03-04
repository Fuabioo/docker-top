use ratatui::style::{Color, Modifier, Style};

// ── Nerd Font Icons ──────────────────────────────────────────────────────────

pub const ICON_DOCKER: &str = "\u{f308}";
pub const ICON_RUNNING: &str = "\u{f111}";
pub const ICON_STOPPED: &str = "\u{f28d}";
pub const ICON_DEAD: &str = "\u{f071}";
pub const ICON_CPU: &str = "\u{f2db}";
pub const ICON_MEMORY: &str = "\u{efc5}";
pub const ICON_NETWORK: &str = "\u{f0ac}";
pub const ICON_DISK: &str = "\u{f0a0}";
pub const ICON_CLOCK: &str = "\u{f017}";
pub const ICON_SORT_ASC: &str = "\u{f0de}";
pub const ICON_SORT_DESC: &str = "\u{f0dd}";

// ── Color Palette ────────────────────────────────────────────────────────────

pub const PRIMARY: Color = Color::Cyan;
pub const SUCCESS: Color = Color::Green;
pub const WARNING: Color = Color::Yellow;
pub const DANGER: Color = Color::Red;
pub const MUTED: Color = Color::DarkGray;
pub const BG_SELECTED: Color = Color::Indexed(236); // Dark gray
pub const BG_HEADER: Color = Color::Indexed(236);

// ── Style Helpers ────────────────────────────────────────────────────────────

pub fn style_header() -> Style {
    Style::default()
        .fg(PRIMARY)
        .bg(BG_HEADER)
        .add_modifier(Modifier::BOLD)
}

pub fn style_selected() -> Style {
    Style::default()
        .bg(BG_SELECTED)
        .add_modifier(Modifier::BOLD)
}

pub fn style_normal() -> Style {
    Style::default()
}

pub fn style_muted() -> Style {
    Style::default().fg(MUTED)
}

pub fn style_status(running: bool, partial: bool, dead: bool) -> Style {
    if dead {
        Style::default().fg(DANGER)
    } else if partial {
        Style::default().fg(WARNING)
    } else if running {
        Style::default().fg(SUCCESS)
    } else {
        Style::default().fg(MUTED)
    }
}

/// Color-coded style for CPU percentage.
pub fn style_cpu(percent: f64) -> Style {
    if percent > 80.0 {
        Style::default()
            .fg(DANGER)
            .add_modifier(Modifier::BOLD)
    } else if percent > 50.0 {
        Style::default().fg(WARNING)
    } else {
        Style::default()
    }
}

/// Color-coded style for memory usage percentage.
pub fn style_mem(percent: f64) -> Style {
    if percent > 90.0 {
        Style::default()
            .fg(DANGER)
            .add_modifier(Modifier::BOLD)
    } else if percent > 70.0 {
        Style::default().fg(WARNING)
    } else {
        Style::default()
    }
}

/// Get status icon for a project.
pub fn status_icon(running: bool, partial: bool, dead: bool) -> &'static str {
    if dead {
        ICON_DEAD
    } else if partial {
        ICON_RUNNING // yellow dot handled by caller
    } else if running {
        ICON_RUNNING
    } else {
        ICON_STOPPED
    }
}

/// Format bytes as human-readable with bytesize.
pub fn fmt_bytes(b: u64) -> String {
    bytesize::ByteSize(b).to_string()
}
