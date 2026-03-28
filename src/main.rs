mod app;
mod event;
mod launchd;
mod schedule;
mod ui;

use app::App;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use event::{AppEvent, EventHandler};
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let events = EventHandler::new(Duration::from_secs(2));
    let mut app = App::new();

    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        match events.next()? {
            AppEvent::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.should_quit = true;
                } else if app.show_action_menu {
                    handle_action_menu_key(&mut app, key.code);
                } else {
                    handle_key(&mut app, key.code);
                }
            }
            AppEvent::Tick => {
                app.refresh_status();
            }
        }

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => app.next_tab(),
        KeyCode::Left | KeyCode::Char('h') => app.prev_tab(),
        KeyCode::Down | KeyCode::Char('j') => app.move_cursor_down(),
        KeyCode::Up | KeyCode::Char('k') => app.move_cursor_up(),
        KeyCode::Enter => {
            if app.selected_service().is_some() {
                app.show_action_menu = true;
            }
        }
        _ => {}
    }
}

fn handle_action_menu_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('l') => {
            if let Some(svc) = app.selected_service() {
                let path = svc.plist_path.clone();
                let _ = launchd::load_service(&path);
            }
            app.show_action_menu = false;
            app.refresh_status();
        }
        KeyCode::Char('u') => {
            if let Some(svc) = app.selected_service() {
                let path = svc.plist_path.clone();
                let _ = launchd::unload_service(&path);
            }
            app.show_action_menu = false;
            app.refresh_status();
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.show_action_menu = false;
        }
        _ => {}
    }
}
