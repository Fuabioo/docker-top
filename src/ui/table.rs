use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::config::{ColumnVisibility, SortColumn, SortConfig};
use crate::model::{self, ComposeProject, ProjectStatus};

use super::theme;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    projects: &[ComposeProject],
    table_state: &mut TableState,
    columns: &ColumnVisibility,
    sort: &SortConfig,
) {
    if projects.is_empty() {
        let msg = Paragraph::new("No Docker Compose projects found")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme::MUTED))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(theme::PRIMARY)),
            );
        frame.render_widget(msg, area);
        return;
    }

    let header_cells = build_header_cells(columns, sort);
    let header = Row::new(header_cells)
        .style(theme::style_header())
        .height(1);

    let rows: Vec<Row> = projects
        .iter()
        .map(|p| build_row(p, columns))
        .collect();

    let widths = build_widths(columns);

    let table = Table::new(rows, &widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(theme::PRIMARY)),
        )
        .row_highlight_style(theme::style_selected())
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(table, area, table_state);
}

fn build_header_cells<'a>(columns: &ColumnVisibility, sort: &SortConfig) -> Vec<Cell<'a>> {
    let mut cells = vec![sort_header("PROJECT", SortColumn::Name, sort)];

    if columns.ctrs {
        cells.push(sort_header("CTRS", SortColumn::Ctrs, sort));
    }
    if columns.cpu {
        cells.push(sort_header(
            &format!("{} CPU", theme::ICON_CPU),
            SortColumn::Cpu,
            sort,
        ));
    }
    if columns.mem {
        cells.push(sort_header(
            &format!("{} MEM", theme::ICON_MEMORY),
            SortColumn::Mem,
            sort,
        ));
    }
    if columns.net {
        cells.push(sort_header(
            &format!("{} NET RX/TX", theme::ICON_NETWORK),
            SortColumn::NetRx,
            sort,
        ));
    }
    if columns.io {
        cells.push(sort_header(
            &format!("{} IO R/W", theme::ICON_DISK),
            SortColumn::IoRead,
            sort,
        ));
    }
    if columns.time_up {
        cells.push(sort_header(
            &format!("{} TIME-UP", theme::ICON_CLOCK),
            SortColumn::TimeUp,
            sort,
        ));
    }
    if columns.last_chg {
        cells.push(sort_header(
            &format!("{} LAST-CHG", theme::ICON_CLOCK),
            SortColumn::LastChg,
            sort,
        ));
    }

    cells
}

fn sort_header<'a>(label: &str, col: SortColumn, sort: &SortConfig) -> Cell<'a> {
    if sort.column == col {
        let icon = if sort.ascending {
            theme::ICON_SORT_ASC
        } else {
            theme::ICON_SORT_DESC
        };
        Cell::from(format!("{} {}", label, icon))
    } else {
        Cell::from(label.to_string())
    }
}

fn build_row<'a>(project: &ComposeProject, columns: &ColumnVisibility) -> Row<'a> {
    let (is_running, is_partial, is_dead) = match project.status {
        ProjectStatus::Running => (true, false, false),
        ProjectStatus::Partial => (false, true, false),
        ProjectStatus::Dead => (false, false, true),
        ProjectStatus::Stopped => (false, false, false),
    };

    let status_icon = theme::status_icon(is_running, is_partial, is_dead);
    let icon_style = theme::style_status(is_running, is_partial, is_dead);

    let name_line = Line::from(vec![
        Span::styled(format!("{} ", status_icon), icon_style),
        Span::raw(project.name.clone()),
    ]);

    let mut cells: Vec<Cell> = vec![Cell::from(name_line)];

    if columns.ctrs {
        cells.push(Cell::from(format!("{}", project.container_count())));
    }
    if columns.cpu {
        let cpu_str = format!("{:.1}%", project.total_cpu);
        cells.push(Cell::from(cpu_str).style(theme::style_cpu(project.total_cpu)));
    }
    if columns.mem {
        let mem_str = format!(
            "{} / {}",
            theme::fmt_bytes(project.total_mem),
            theme::fmt_bytes(project.mem_limit),
        );
        cells.push(Cell::from(mem_str).style(theme::style_mem(project.mem_percent())));
    }
    if columns.net {
        let net_str = format!(
            "{} / {}",
            theme::fmt_bytes(project.total_net_rx),
            theme::fmt_bytes(project.total_net_tx),
        );
        cells.push(Cell::from(net_str));
    }
    if columns.io {
        let io_str = format!(
            "{} / {}",
            theme::fmt_bytes(project.total_blk_read),
            theme::fmt_bytes(project.total_blk_write),
        );
        cells.push(Cell::from(io_str));
    }
    if columns.time_up {
        let time_str = project
            .oldest_started_at
            .map(model::uptime_str)
            .unwrap_or_else(|| "-".to_string());
        cells.push(Cell::from(time_str));
    }
    if columns.last_chg {
        let chg_str = project
            .newest_started_at
            .map(model::uptime_str)
            .unwrap_or_else(|| "-".to_string());
        cells.push(Cell::from(chg_str));
    }

    Row::new(cells)
}

fn build_widths(columns: &ColumnVisibility) -> Vec<Constraint> {
    let mut widths = vec![Constraint::Fill(1)]; // PROJECT gets all remaining space

    if columns.ctrs {
        widths.push(Constraint::Length(6));
    }
    if columns.cpu {
        widths.push(Constraint::Length(9));
    }
    if columns.mem {
        widths.push(Constraint::Length(17));
    }
    if columns.net {
        widths.push(Constraint::Length(21));
    }
    if columns.io {
        widths.push(Constraint::Length(15));
    }
    if columns.time_up {
        widths.push(Constraint::Length(10));
    }
    if columns.last_chg {
        widths.push(Constraint::Length(10));
    }

    widths
}
