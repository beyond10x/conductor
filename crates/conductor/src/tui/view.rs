//! What the terminal UI draws (`story:tui-readonly`): the instances, the sessions of the selected
//! instance and its settings side by side, and a footer with the status line and the keys. It draws
//! into any ratatui backend, so the tests draw into a test backend's buffer.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{Pane, Screen};

/// The keys the footer lists.
pub const KEYS: &str =
    "Enter attach  w watch  l log  ctrl-] d back  Tab pane  j/k move  r refresh  q quit";

/// Draws `screen` into `frame`.
pub fn draw(frame: &mut Frame<'_>, screen: &Screen) {
    let [top, footer] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(4)])
        .areas(frame.area());
    let [instances, sessions, settings] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(42),
            Constraint::Percentage(30),
        ])
        .areas(top);
    let selected = screen.instance().unwrap_or("-").to_owned();

    let lines: Vec<Line<'_>> = screen
        .instances
        .iter()
        .enumerate()
        .map(|(at, row)| {
            let decisions = row
                .decisions
                .map_or_else(|| "?".to_owned(), |count| count.to_string());
            pick(
                format!(
                    "{:<10} {:<11} {decisions:>2} dec  {}",
                    row.name, row.state, row.kpis
                ),
                at == screen.selected,
                screen.focus == Pane::Instances,
            )
        })
        .collect();
    pane(
        frame,
        instances,
        format!(" instances on {} ", screen.daemon),
        lines,
        "no instance",
    );

    let lines: Vec<Line<'_>> = screen
        .sessions()
        .iter()
        .enumerate()
        .map(|(at, row)| {
            pick(
                format!(
                    "{:<12} {:<10} {:>6}  {}  {}",
                    row.role, row.state, row.context, row.work, row.name
                ),
                at == screen.session,
                screen.focus == Pane::Sessions,
            )
        })
        .collect();
    pane(
        frame,
        sessions,
        format!(" sessions of {selected} "),
        lines,
        "no session",
    );

    let lines: Vec<Line<'_>> = screen
        .settings
        .iter()
        .map(|(path, value)| Line::from(format!("{path:<28} {value}")))
        .collect();
    pane(
        frame,
        settings,
        format!(" settings of {selected} (read-only) "),
        lines,
        "no settings",
    );

    let status = if screen.status.is_empty() {
        Line::from("")
    } else {
        Line::from(screen.status.clone()).style(Style::default().add_modifier(Modifier::BOLD))
    };
    frame.render_widget(
        Paragraph::new(vec![status, Line::from(KEYS)])
            .block(Block::default().borders(Borders::ALL)),
        footer,
    );
}

/// One row, `> ` before the selected one, reversed when its pane has the focus.
fn pick(text: String, selected: bool, focused: bool) -> Line<'static> {
    let line = Line::from(format!("{}{text}", if selected { "> " } else { "  " }));
    if selected && focused {
        line.style(Style::default().add_modifier(Modifier::REVERSED))
    } else {
        line
    }
}

/// A bordered pane titled `title` holding `lines`, or `empty` when there are none.
fn pane(frame: &mut Frame<'_>, area: Rect, title: String, lines: Vec<Line<'_>>, empty: &str) {
    let lines = if lines.is_empty() {
        vec![Line::from(format!("  {empty}"))]
    } else {
        lines
    };
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title)),
        area,
    );
}
