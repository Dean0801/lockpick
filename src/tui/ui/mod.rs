pub mod menu;
pub mod progress;
pub mod results;
pub mod settings;

use ratatui::Frame;

use crate::tui::app::{App, Screen};

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    match app.screen {
        Screen::Menu => menu::render(f, app, area),
        Screen::Scanning => progress::render(f, area, &app.scan_status, app.scan_progress),
        Screen::Results => {
            if let Some(ref result) = app.result {
                results::render(f, area, result);
            } else {
                menu::render(f, app, area);
            }
        }
        Screen::Settings => settings::render(f, area),
    }
}
