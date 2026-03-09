use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use tui_tree_widget::Tree;

use crate::tui::app::App;
use crate::tui::tree_converter::convert_to_tui_tree;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(f, chunks[0], app);
    render_tree(f, chunks[1], app);
    render_footer(f, chunks[2]);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let title = if app
        .tree_state
        .as_ref()
        .is_some_and(|s| !s.search_query.is_empty())
    {
        format!(
            "🌲 Dependency Tree - Search: {}",
            app.tree_state.as_ref().unwrap().search_query
        )
    } else {
        "🌲 Dependency Tree".to_string()
    };

    let header = Paragraph::new(title)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(header, area);
}

fn render_tree(f: &mut Frame, area: Rect, app: &mut App) {
    if let (Some(tree_data), Some(tree_state)) = (&app.tree_data, &mut app.tree_state) {
        let items = convert_to_tui_tree(tree_data);

        match Tree::new(&items) {
            Ok(tree_widget) => {
                let tree_widget = tree_widget
                    .block(Block::default().borders(Borders::ALL).title("Dependencies"))
                    .highlight_style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol(">> ");
                f.render_stateful_widget(tree_widget, area, &mut tree_state.tui_state);
            }
            Err(e) => {
                let error = Paragraph::new(format!("Tree error: {}", e))
                    .block(Block::default().borders(Borders::ALL).title("Error"));
                f.render_widget(error, area);
            }
        }
    } else {
        let placeholder =
            Paragraph::new("Loading tree...").block(Block::default().borders(Borders::ALL));
        f.render_widget(placeholder, area);
    }
}

fn render_footer(f: &mut Frame, area: Rect) {
    let help = vec![
        Span::raw("↑↓: Navigate | "),
        Span::raw("Space: Toggle | "),
        Span::raw("Esc: Back"),
    ];

    let footer = Paragraph::new(Line::from(help))
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(footer, area);
}
