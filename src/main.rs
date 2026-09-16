mod app;
mod event;
mod scanner;
mod theme;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use crossterm::event::{KeyCode, KeyModifiers};
use event::{Event, EventHandler};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, path::PathBuf};

#[tokio::main]
async fn main() -> Result<()> {
    let root: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(root);
    app.start_scan();

    let mut events = EventHandler::new(100);

    loop {
        app.poll_scan();
        terminal.draw(|f| ui::render(f, &mut app))?;

        match events.next().await? {
            Event::Tick => {}
            Event::Key(key) => {
                handle_key(&mut app, key.code, key.modifiers);
                if app.quit_confirm && matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode, mods: KeyModifiers) {
    // ── Quit confirmation popup ────────────────────────────────────────────────
    if app.quit_confirm {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => { /* handled in loop — will break */ }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.quit_confirm = false;
            }
            _ => {}
        }
        return;
    }

    // ── Help popup ────────────────────────────────────────────────────────────
    if app.show_help {
        match code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
                app.show_help = false;
            }
            _ => {}
        }
        return;
    }

    // ── Search / filter mode ──────────────────────────────────────────────────
    if app.is_searching {
        match code {
            KeyCode::Esc => {
                app.is_searching = false;
                app.search_query.clear();
                app.update_search_filter();
            }
            KeyCode::Enter => {
                app.is_searching = false;
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.update_search_filter();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.update_search_filter();
            }
            _ => {}
        }
        return;
    }

    // ── Normal mode ───────────────────────────────────────────────────────────
    match (code, mods) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            // Handled outside — set quit_confirm so the loop breaks cleanly
            app.quit_confirm = true;
        }
        (KeyCode::Char('q'), _) => app.quit_confirm = true,
        (KeyCode::Char('?'), _) => app.show_help = true,

        (KeyCode::Down,  _) | (KeyCode::Char('j'), _) => app.select_next(),
        (KeyCode::Up,    _) | (KeyCode::Char('k'), _) => app.select_prev(),

        (KeyCode::Enter, _) | (KeyCode::Char('l'), _) => app.enter(),

        (KeyCode::Backspace, _) | (KeyCode::Char('h'), _) | (KeyCode::Left, _) => {
            app.go_up();
        }

        (KeyCode::Char('/'), _) => {
            app.is_searching = true;
            app.search_query.clear();
            app.update_search_filter();
        }

        (KeyCode::Char('r'), _) => {
            app.status = "Rescanning…".to_string();
            app.start_scan();
        }

        (KeyCode::Char('.'), _) => app.toggle_hidden(),
        (KeyCode::Char('t'), _) => app.reload_theme(),

        _ => {}
    }
}
