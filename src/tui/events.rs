use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::time::Duration;

pub enum AppEvent {
    Quit,
    Up,
    Down,
    Select,
    Back,
    None,
}

pub fn poll_event() -> std::io::Result<AppEvent> {
    if event::poll(Duration::from_millis(100))?
        && let Event::Key(key) = event::read()?
    {
        return Ok(handle_key(key));
    }
    Ok(AppEvent::None)
}

fn handle_key(key: KeyEvent) -> AppEvent {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => AppEvent::Quit,
        KeyCode::Up | KeyCode::Char('k') => AppEvent::Up,
        KeyCode::Down | KeyCode::Char('j') => AppEvent::Down,
        KeyCode::Enter => AppEvent::Select,
        KeyCode::Backspace => AppEvent::Back,
        _ => AppEvent::None,
    }
}
