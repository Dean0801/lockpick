pub mod menu;
pub mod progress;
pub mod results;
pub mod settings;
pub mod tree;

use ratatui::Frame;

use crate::tui::app::{App, Screen};

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    match app.screen {
        Screen::Menu => menu::render(f, app, area),
        Screen::Scanning => progress::render(f, area, &app.scan_status, app.scan_progress),
        Screen::Results => results::render(f, area, app),
        Screen::Settings => settings::render(f, area),
        Screen::Tree => tree::render(f, app, area),
    }
}
