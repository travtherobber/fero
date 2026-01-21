mod app;
mod config;
mod core;
mod ui;

use crate::app::{handle_key_event, load_custom_keybinds, UiState};
use crate::core::snapshot::build_snapshot;
use crate::core::state::{AppState, Palette, APP_NAME};
use crossterm::{
    event::{poll, read, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{env, io::stdout, time::Duration};

fn main() -> std::io::Result<()> {
    let reset_colors = env::args().any(|arg| arg == "--reset-colors");

    std::panic::set_hook(Box::new(|info| {
        let mut stdout = stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
        let _ = disable_raw_mode();
        eprintln!("{} FATAL ERROR: {:?}", APP_NAME, info);
    }));

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let mut config = config::load_config();

    let mut app = AppState::new();
    app.current_palette = Palette::from_config(&config.palette);

    if reset_colors {
        app.current_palette = Palette::default();
        config.palette = app.current_palette.to_config();
        let _ = config::save_config(&config);
    }

    load_custom_keybinds(&mut app, &config);

    let mut ui_state = UiState::new();
    let mut needs_redraw = true;

    loop {
        let mut should_exit = false;

        if poll(Duration::from_millis(100))? {
            match read()? {
                Event::Resize(_, _) => {
                    needs_redraw = true;
                }
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let (term_w, term_h) = size().unwrap_or((80, 24));
                    let outcome =
                        handle_key_event(&mut app, key, &mut ui_state, &mut config, term_w, term_h)?;
                    needs_redraw = needs_redraw || outcome.needs_redraw;
                    should_exit = outcome.should_exit;
                }
                _ => {}
            }
        }

        if app.tick_flash() {
            needs_redraw = true;
        }

        if should_exit {
            break;
        }

        if needs_redraw {
            let (term_w, term_h) = size().unwrap_or((80, 24));
            app.ensure_cursor_visible(
                term_w,
                term_h,
                config.show_line_numbers,
                config.show_status_bar,
                config.show_header,
                config.show_tab_bar,
            );
            let snapshot = build_snapshot(
                &app,
                &config,
                ui_state.mode,
                ui_state.active_tab,
                ui_state.dropdown_idx,
            );
            ui::redraw_all(&mut stdout, &snapshot, term_w, term_h)?;
            needs_redraw = false;
        }
    }

    execute!(stdout, LeaveAlternateScreen)?;
    disable_raw_mode()
}
