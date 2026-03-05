use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::AnalysisResult;

pub fn render(f: &mut Frame, area: Rect, result: &AnalysisResult) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(f, chunks[0]);
    render_summary(f, chunks[1], result);
    render_footer(f, chunks[2]);
}

fn render_header(f: &mut Frame, area: Rect) {
    let header = Paragraph::new("📊 Scan Results")
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, area);
}

fn render_summary(f: &mut Frame, area: Rect, result: &AnalysisResult) {
    let mut items = Vec::new();

    if let Some(ref u) = result.unused {
        let count = u.unused.len();
        let color = if count > 0 {
            Color::Yellow
        } else {
            Color::Green
        };
        items.push(
            ListItem::new(format!("📦 Unused Dependencies: {}", count))
                .style(Style::default().fg(color)),
        );
    }

    if let Some(ref v) = result.vulns {
        let count: usize = v.iter().map(|r| r.vulns.len()).sum();
        let color = if count > 0 { Color::Red } else { Color::Green };
        items.push(
            ListItem::new(format!("🛡️  Vulnerabilities: {}", count))
                .style(Style::default().fg(color)),
        );
    }

    if let Some(ref d) = result.duplicates {
        let count = d.duplicates.len();
        let color = if count > 0 {
            Color::Yellow
        } else {
            Color::Green
        };
        items.push(
            ListItem::new(format!("🔄 Duplicates: {}", count)).style(Style::default().fg(color)),
        );
    }

    if let Some(ref o) = result.outdated {
        let count = o.total_outdated;
        let color = if count > 0 {
            Color::Yellow
        } else {
            Color::Green
        };
        items.push(
            ListItem::new(format!("📊 Outdated Packages: {}", count))
                .style(Style::default().fg(color)),
        );
    }

    if let Some(ref sc) = result.supply_chain {
        let count = sc.risks.len();
        let color = if count > 0 { Color::Red } else { Color::Green };
        items.push(
            ListItem::new(format!("🔗 Supply Chain Risks: {}", count))
                .style(Style::default().fg(color)),
        );
    }

    let list = List::new(items).block(Block::default().borders(Borders::ALL));
    f.render_widget(list, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new("Press any key to return to menu")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, area);
}
