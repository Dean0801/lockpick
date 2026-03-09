use crossterm::event::{self, Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use std::time::Duration;

pub enum AppEvent {
    Quit,
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    ScrollUp,
    ScrollDown,
    None,
}

pub fn poll_event() -> std::io::Result<AppEvent> {
    if event::poll(Duration::from_millis(100))? {
        match event::read()? {
            Event::Key(key) => return Ok(handle_key(key)),
            Event::Mouse(mouse) => return Ok(handle_mouse(mouse)),
            _ => {}
        }
    }
    Ok(AppEvent::None)
}

fn handle_mouse(mouse: MouseEvent) -> AppEvent {
    match mouse.kind {
        MouseEventKind::ScrollUp => AppEvent::ScrollUp,
        MouseEventKind::ScrollDown => AppEvent::ScrollDown,
        _ => AppEvent::None,
    }
}

fn handle_key(key: KeyEvent) -> AppEvent {
    match key.code {
        KeyCode::Char('q') => AppEvent::Quit,
        KeyCode::Esc => AppEvent::Back,
        KeyCode::Up | KeyCode::Char('k') => AppEvent::Up,
        KeyCode::Down | KeyCode::Char('j') => AppEvent::Down,
        KeyCode::Left | KeyCode::Char('h') => AppEvent::Left,
        KeyCode::Right | KeyCode::Char('l') => AppEvent::Right,
        KeyCode::Enter | KeyCode::Char(' ') => AppEvent::Select,
        KeyCode::Backspace => AppEvent::Back,
        _ => AppEvent::None,
    }
}
