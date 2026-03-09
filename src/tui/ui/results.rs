use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::tui::app::App;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(f, chunks[0]);
    render_details(f, chunks[1], app);
    render_footer(f, chunks[2]);
}

fn render_header(f: &mut Frame, area: Rect) {
    let header = Paragraph::new("📊 Scan Results")
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, area);
}

fn render_details(f: &mut Frame, area: Rect, app: &App) {
    let Some(result) = &app.result else {
        return;
    };

    let mut items = Vec::new();
    let mut section_indices = Vec::new(); // 记录每个section的起始索引
    let mut current_line = 0;

    // Unused Dependencies
    if let Some(ref u) = result.unused {
        let count = u.unused.len();
        section_indices.push(current_line);
        let is_selected = app.results_selected == section_indices.len() - 1;
        let is_expanded = app.results_expanded.contains(&(section_indices.len() - 1));

        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if count > 0 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        };

        let prefix = if count > 0 {
            if is_expanded { "▼" } else { "▶" }
        } else {
            " "
        };
        items.push(
            ListItem::new(format!("{} 📦 Unused Dependencies: {}", prefix, count)).style(style),
        );
        current_line += 1;

        if is_expanded {
            for dep in &u.unused {
                items.push(
                    ListItem::new(format!("    • {}", dep.name))
                        .style(Style::default().fg(Color::Gray)),
                );
                current_line += 1;
            }
        }
    }

    // Vulnerabilities
    if let Some(ref v) = result.vulns {
        let count: usize = v.iter().map(|r| r.vulns.len()).sum();
        section_indices.push(current_line);
        let is_selected = app.results_selected == section_indices.len() - 1;
        let is_expanded = app.results_expanded.contains(&(section_indices.len() - 1));

        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if count > 0 {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Green)
        };

        let prefix = if count > 0 {
            if is_expanded { "▼" } else { "▶" }
        } else {
            " "
        };
        items
            .push(ListItem::new(format!("{} 🛡️  Vulnerabilities: {}", prefix, count)).style(style));
        current_line += 1;

        if is_expanded {
            for vr in v {
                for vuln in &vr.vulns {
                    let sev_color = match vuln.severity {
                        crate::Severity::Critical => Color::Red,
                        crate::Severity::High => Color::LightRed,
                        crate::Severity::Medium => Color::Yellow,
                        crate::Severity::Low => Color::Gray,
                    };
                    items.push(
                        ListItem::new(format!(
                            "    • {} - {} ({:?})",
                            vr.package, vuln.id, vuln.severity
                        ))
                        .style(Style::default().fg(sev_color)),
                    );
                    current_line += 1;
                }
            }
        }
    }

    // Duplicates
    if let Some(ref d) = result.duplicates {
        let count = d.duplicates.len();
        section_indices.push(current_line);
        let is_selected = app.results_selected == section_indices.len() - 1;
        let is_expanded = app.results_expanded.contains(&(section_indices.len() - 1));

        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if count > 0 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        };

        let prefix = if count > 0 {
            if is_expanded { "▼" } else { "▶" }
        } else {
            " "
        };
        items.push(ListItem::new(format!("{} 🔄 Duplicates: {}", prefix, count)).style(style));
        current_line += 1;

        if is_expanded {
            for dup in &d.duplicates {
                items.push(
                    ListItem::new(format!(
                        "    • {} ({} versions)",
                        dup.name,
                        dup.versions.len()
                    ))
                    .style(Style::default().fg(Color::Gray)),
                );
                current_line += 1;
            }
        }
    }

    // Outdated
    if let Some(ref o) = result.outdated {
        let count = o.total_outdated;
        section_indices.push(current_line);
        let is_selected = app.results_selected == section_indices.len() - 1;
        let is_expanded = app.results_expanded.contains(&(section_indices.len() - 1));

        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if count > 0 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        };

        let prefix = if count > 0 {
            if is_expanded { "▼" } else { "▶" }
        } else {
            " "
        };
        items.push(ListItem::new(format!("{} 📊 Outdated: {}", prefix, count)).style(style));
        current_line += 1;

        if is_expanded {
            for entry in &o.entries {
                items.push(
                    ListItem::new(format!(
                        "    • {} {} → {}",
                        entry.name, entry.current, entry.latest
                    ))
                    .style(Style::default().fg(Color::Gray)),
                );
                current_line += 1;
            }
        }
    }

    // Supply Chain
    if let Some(ref sc) = result.supply_chain {
        let count = sc.risks.len();
        section_indices.push(current_line);
        let is_selected = app.results_selected == section_indices.len() - 1;
        let is_expanded = app.results_expanded.contains(&(section_indices.len() - 1));

        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if count > 0 {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Green)
        };

        let prefix = if count > 0 {
            if is_expanded { "▼" } else { "▶" }
        } else {
            " "
        };
        items.push(
            ListItem::new(format!("{} 🔗 Supply Chain Risks: {}", prefix, count)).style(style),
        );

        if is_expanded {
            for risk in &sc.risks {
                items.push(
                    ListItem::new(format!("    • {} - {:?}", risk.package, risk.risk_type))
                        .style(Style::default().fg(Color::Gray)),
                );
            }
        }
    }

    // 计算选中section的行号
    let selected_line = if app.results_selected < section_indices.len() {
        section_indices[app.results_selected]
    } else {
        0
    };

    let visible_height = area.height.saturating_sub(2) as usize;

    // 如果启用自动滚动（键盘导航），确保选中项可见
    let scroll = if app.results_auto_scroll {
        if selected_line < app.results_scroll {
            selected_line
        } else if selected_line >= app.results_scroll + visible_height {
            selected_line.saturating_sub(visible_height - 1)
        } else {
            app.results_scroll
        }
    } else {
        app.results_scroll
    };

    let visible_items: Vec<_> = items
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect();

    let list =
        List::new(visible_items).block(Block::default().borders(Borders::ALL).title("Details"));
    f.render_widget(list, area);
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
