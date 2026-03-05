use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render(f: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(f, chunks[0]);
    render_options(f, chunks[1]);
    render_footer(f, chunks[2]);
}

fn render_header(f: &mut Frame, area: Rect) {
    let header = Paragraph::new("⚙️  Settings")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, area);
}

fn render_options(f: &mut Frame, area: Rect) {
    let items = vec![
        ListItem::new("📁 Project Path: .").style(Style::default().fg(Color::White)),
        ListItem::new("🔍 Skip Dev Dependencies: No").style(Style::default().fg(Color::White)),
        ListItem::new("💾 Cache Enabled: Yes").style(Style::default().fg(Color::White)),
        ListItem::new("🌐 Language: Auto").style(Style::default().fg(Color::White)),
        ListItem::new("📊 Output Format: Terminal").style(Style::default().fg(Color::White)),
    ];

    let list = List::new(items).block(Block::default().borders(Borders::ALL));
    f.render_widget(list, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new("Press Esc to return to menu")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, area);
}
