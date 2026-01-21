use crate::app::UiState;
use crate::config;
use crate::core::editor;
use crate::core::io;
use crate::core::state::{
    AppState, Buffer, Config, ConfirmChoice, ConfirmType, KeyCombo, MenuTab, Mode,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct ControllerOutcome {
    pub needs_redraw: bool,
    pub should_exit: bool,
}

impl ControllerOutcome {
    fn new() -> Self {
        Self {
            needs_redraw: false,
            should_exit: false,
        }
    }
}

pub fn handle_key_event(
    app: &mut AppState,
    key: KeyEvent,
    ui: &mut UiState,
    config: &mut Config,
    term_w: u16,
    term_h: u16,
) -> std::io::Result<ControllerOutcome> {
    let mut outcome = ControllerOutcome::new();

    match ui.mode {
        Mode::ConfirmWipe => {
            if matches!(key.code, KeyCode::Char('y' | 'Y')) {
                app.push_undo();
                let buf = app.current_buffer_mut();
                buf.lines = vec![String::new()];
                buf.cursor_x = 0;
                buf.cursor_y = 0;
                buf.modified = true;
                app.flash_status("BUFFER WIPED".to_string());
            }
            ui.mode = Mode::Editing;
            outcome.needs_redraw = true;
        }

        Mode::Confirm(ConfirmType::CloseTab) => {
            let mut close_tab = false;
            let mut save_and_close = false;
            let mut needs_redraw = true;

            match key.code {
                KeyCode::Up | KeyCode::Left => {
                    app.confirm_choice = match app.confirm_choice {
                        ConfirmChoice::No => ConfirmChoice::Cancel,
                        ConfirmChoice::Yes => ConfirmChoice::No,
                        ConfirmChoice::Cancel => ConfirmChoice::Yes,
                    };
                }
                KeyCode::Down | KeyCode::Right => {
                    app.confirm_choice = match app.confirm_choice {
                        ConfirmChoice::No => ConfirmChoice::Yes,
                        ConfirmChoice::Yes => ConfirmChoice::Cancel,
                        ConfirmChoice::Cancel => ConfirmChoice::No,
                    };
                }
                KeyCode::Enter => {
                    match app.confirm_choice {
                        ConfirmChoice::Yes => save_and_close = true,
                        ConfirmChoice::No => close_tab = true,
                        ConfirmChoice::Cancel => {}
                    }
                    app.confirm_mode = None;
                    ui.mode = Mode::Editing;
                }
                KeyCode::Esc => {
                    app.confirm_mode = None;
                    ui.mode = Mode::Editing;
                }
                _ => needs_redraw = false,
            }

            if save_and_close {
                editor::save_and_close_tab(app)?;
            } else if close_tab {
                editor::close_current_tab(app);
            }

            outcome.needs_redraw = needs_redraw;
        }

        Mode::Help => {
            if key.code == KeyCode::Esc || key.code == KeyCode::Enter {
                ui.mode = Mode::Editing;
                outcome.needs_redraw = true;
            }
        }

        Mode::Settings => match key.code {
            KeyCode::Esc => {
                ui.mode = Mode::Editing;
                outcome.needs_redraw = true;
            }
            KeyCode::Up => {
                app.settings_idx = app.settings_idx.saturating_sub(1);
                outcome.needs_redraw = true;
            }
            KeyCode::Down => {
                if app.settings_idx < 4 {
                    app.settings_idx += 1
                }
                outcome.needs_redraw = true;
            }
            KeyCode::Enter => {
                match app.settings_idx {
                    0 => {
                        config.auto_save = !config.auto_save;
                        let _ = config::save_config(config);
                    }
                    1 => {
                        config.tab_size = if config.tab_size >= 8 {
                            2
                        } else {
                            config.tab_size + 2
                        };
                        let _ = config::save_config(config);
                    }
                    2 => {
                        app.populate_color_entries();
                        app.color_editor_idx = 0;
                        app.editing_hex = false;
                        ui.mode = Mode::ColorEditor;
                    }
                    3 => {
                        ui.mode = Mode::KeyRebind;
                        let kb = &mut app.keybind_state;
                        kb.in_rebind_mode = true;
                        kb.selected_action = 0;
                        kb.waiting_for_key = false;
                        kb.pending_action = None;
                        kb.scroll_offset = 0;
                        kb.confirming_reset = false;
                    }
                    4 => ui.mode = Mode::Editing,
                    _ => {}
                }
                outcome.needs_redraw = true;
            }
            _ => {}
        },

        Mode::KeyRebind => {
            let mut flash_msg = editor::handle_keybind_capture(app, config, key.code, key.modifiers);
            if flash_msg.is_none() {
                if key.code == KeyCode::Esc {
                    ui.mode = Mode::Settings;
                    app.keybind_state.in_rebind_mode = false;
                    outcome.needs_redraw = true;
                } else {
                    flash_msg = editor::handle_keybind_navigation(app, key.code);
                }
            }
            if let Some(msg) = flash_msg {
                app.flash_status(msg);
                outcome.needs_redraw = true;
            } else if matches!(key.code, KeyCode::Up | KeyCode::Down | KeyCode::Enter) {
                outcome.needs_redraw = true;
            }
        }

        Mode::ColorEditor => match key.code {
            KeyCode::Esc => {
                ui.mode = Mode::Settings;
                outcome.needs_redraw = true;
            }
            KeyCode::Up => {
                app.color_editor_idx = app.color_editor_idx.saturating_sub(1);
                app.editing_hex = false;
                outcome.needs_redraw = true;
            }
            KeyCode::Down => {
                if app.color_editor_idx < app.color_entries.len() - 1 {
                    app.color_editor_idx += 1;
                }
                app.editing_hex = false;
                outcome.needs_redraw = true;
            }
            KeyCode::Enter => {
                app.editing_hex = !app.editing_hex;
                outcome.needs_redraw = true;
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Ok(new_palette) = editor::parse_palette_from_entries(&app.color_entries) {
                    app.current_palette = new_palette;
                    config.palette = new_palette.to_config();
                    let _ = config::save_config(config);
                    app.flash_status("COLORS SAVED".to_string());
                }
                ui.mode = Mode::Settings;
                outcome.needs_redraw = true;
            }
            _ if app.editing_hex => {
                let entry = &mut app.color_entries[app.color_editor_idx];
                match key.code {
                    KeyCode::Char(c) => {
                        let c = c.to_ascii_uppercase();
                        if c.is_ascii_hexdigit() {
                            if entry.current_hex.len() < 7 {
                                if entry.current_hex.is_empty() {
                                    entry.current_hex.push('#');
                                }
                                entry.current_hex.push(c);
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        entry.current_hex.pop();
                        if entry.current_hex == "#" {
                            entry.current_hex.clear();
                        }
                    }
                    _ => {}
                }
                outcome.needs_redraw = true;
            }
            _ => {}
        },

        Mode::Explorer => match key.code {
            KeyCode::Up => {
                if app.explorer_idx > 0 {
                    app.explorer_idx -= 1;
                    if app.explorer_idx < app.explorer_offset {
                        app.explorer_offset = app.explorer_idx;
                    }
                }
                outcome.needs_redraw = true;
            }
            KeyCode::Down => {
                if app.explorer_idx < app.explorer_files.len().saturating_sub(1) {
                    app.explorer_idx += 1;
                    let visible = term_h.saturating_sub(4) as usize;
                    if app.explorer_idx >= app.explorer_offset + visible {
                        app.explorer_offset = app.explorer_idx - visible + 1;
                    }
                }
                outcome.needs_redraw = true;
            }
            KeyCode::Enter => {
                if let Some(selected) = app.explorer_files.get(app.explorer_idx).cloned() {
                    let clean_name = selected.trim_start_matches("📁 ").trim_start_matches("📄 ");
                    let full_path = app.current_dir.join(clean_name);
                    if full_path.is_dir() {
                        app.current_dir = full_path;
                        let _ = editor::refresh_explorer(app);
                    } else {
                        match io::load_from_file(full_path.to_str().unwrap_or(&selected)) {
                            Ok(lines) => {
                                let mut new_buffer = Buffer::new(clean_name.to_string());
                                new_buffer.lines = lines;
                                app.buffers.push(new_buffer);
                                app.active_buffer = app.buffers.len() - 1;
                                ui.mode = Mode::Editing;
                            }
                            Err(e) => {
                                app.flash_status(format!("OPEN FAILED: {}", e));
                            }
                        }
                    }
                }
                outcome.needs_redraw = true;
            }
            KeyCode::Backspace => {
                if let Some(parent) = app.current_dir.parent() {
                    app.current_dir = parent.to_path_buf();
                    let _ = editor::refresh_explorer(app);
                }
                outcome.needs_redraw = true;
            }
            KeyCode::Esc => {
                ui.mode = Mode::Menu;
                outcome.needs_redraw = true;
            }
            _ => {}
        },

        Mode::Editing => {
            if app.input_mode {
                editor::handle_prompt_input(app, key.code, config, term_w, term_h);
                outcome.needs_redraw = true;
            } else {
                if key.code == KeyCode::Tab && key.modifiers.contains(KeyModifiers::CONTROL) {
                    if app.buffers.len() > 1 {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            app.active_buffer = if app.active_buffer == 0 {
                                app.buffers.len() - 1
                            } else {
                                app.active_buffer - 1
                            };
                        } else {
                            app.active_buffer = (app.active_buffer + 1) % app.buffers.len();
                        }
                        app.flash_status(format!("TAB {}", app.current_buffer().filename));
                    }
                    outcome.needs_redraw = true;
                    return Ok(outcome);
                }

                handle_editing_input(app, key, config, ui, term_w, term_h);
                outcome.needs_redraw = true;
            }
        }

        Mode::Menu => match key.code {
            KeyCode::Esc => {
                ui.mode = Mode::Editing;
                outcome.needs_redraw = true;
            }
            KeyCode::Left => {
                ui.active_tab = prev_tab(ui.active_tab);
                ui.dropdown_idx = 0;
                outcome.needs_redraw = true;
            }
            KeyCode::Right => {
                ui.active_tab = next_tab(ui.active_tab);
                ui.dropdown_idx = 0;
                outcome.needs_redraw = true;
            }
            KeyCode::Up => {
                ui.dropdown_idx = ui.dropdown_idx.saturating_sub(1);
                outcome.needs_redraw = true;
            }
            KeyCode::Down => {
                ui.dropdown_idx += 1;
                outcome.needs_redraw = true;
            }
            KeyCode::Enter => {
                let result =
                    editor::handle_menu_selection(ui.active_tab, ui.dropdown_idx, config, app)?;
                if result.exit {
                    outcome.should_exit = true;
                    return Ok(outcome);
                }
                ui.mode = result.next_mode;
                if ui.mode == Mode::Explorer {
                    let _ = editor::refresh_explorer(app);
                }
                outcome.needs_redraw = true;
            }
            _ => {}
        },
    }

    Ok(outcome)
}

fn handle_editing_input(
    app: &mut AppState,
    key: KeyEvent,
    config: &Config,
    ui: &mut UiState,
    term_w: u16,
    term_h: u16,
) {
    let code = key.code;
    let modifiers = key.modifiers;

    let combo = KeyCombo { code, modifiers };

    if let Some(&action) = app.keybind_state.custom_binds.get(&combo) {
        editor::perform_keybind_action(app, action, &mut ui.mode);
        return;
    }

    if let Some(sel) = app.selection.as_ref() {
        let (sx, sy, ex, ey) = sel.normalized();
        let selected_text = editor::extract_selected_text(app.current_buffer(), sx, sy, ex, ey);

        match code {
            KeyCode::Backspace | KeyCode::Delete => {
                app.push_undo();
                editor::delete_selection(app.current_buffer_mut(), sx, sy, ex, ey);
                app.selection = None;
                app.current_buffer_mut().modified = true;
                return;
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                app.clipboard = selected_text;
                return;
            }
            KeyCode::Char('x') if modifiers.contains(KeyModifiers::CONTROL) => {
                app.clipboard = selected_text;
                app.push_undo();
                editor::delete_selection(app.current_buffer_mut(), sx, sy, ex, ey);
                app.selection = None;
                app.current_buffer_mut().modified = true;
                return;
            }
            KeyCode::Char('a') if modifiers.contains(KeyModifiers::CONTROL) => {
                editor::select_all(app);
                return;
            }
            _ => {}
        }
    }

    if code == KeyCode::Char('v') && modifiers.contains(KeyModifiers::CONTROL) {
        if !app.clipboard.is_empty() {
            app.push_undo();
            editor::paste_clipboard(app);
        }
        return;
    }

    match code {
        KeyCode::Char('a') if modifiers.contains(KeyModifiers::CONTROL) => {
            editor::select_all(app);
        }
        KeyCode::Up if modifiers.contains(KeyModifiers::CONTROL) => {
            editor::handle_ctrl_move(app, KeyCode::Up);
            editor::update_viewport(app, config, term_w, term_h);
            editor::clear_selection(app);
        }
        KeyCode::Down if modifiers.contains(KeyModifiers::CONTROL) => {
            editor::handle_ctrl_move(app, KeyCode::Down);
            editor::update_viewport(app, config, term_w, term_h);
            editor::clear_selection(app);
        }
        KeyCode::Tab => {
            app.push_undo();
            editor::insert_tab(app, config.tab_size);
        }
        KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => {
            let filename = app.current_buffer().filename.clone();
            let lines = app.current_buffer().lines.clone();
            if let Err(e) = io::save_to_file(&lines, &filename) {
                app.flash_status(format!("SAVE FAILED: {}", e));
            } else {
                app.current_buffer_mut().modified = false;
                app.flash_status("SAVED".to_string());
            }
        }
        KeyCode::Char('z') if modifiers.contains(KeyModifiers::CONTROL) => {
            editor::undo(app);
        }
        KeyCode::Char('y') if modifiers.contains(KeyModifiers::CONTROL) => {
            editor::redo(app);
        }
        KeyCode::Esc => {
            app.selection = None;
            ui.mode = Mode::Menu;
        }
        KeyCode::Char(c) => {
            app.push_undo();
            editor::insert_char(app, c);
            editor::clear_selection(app);
        }
        KeyCode::Enter => {
            app.push_undo();
            editor::insert_newline(app);
            editor::clear_selection(app);
        }
        KeyCode::Backspace => {
            app.push_undo();
            editor::backspace(app);
            editor::clear_selection(app);
        }
        KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
            let old_x = app.current_buffer().cursor_x;
            let old_y = app.current_buffer().cursor_y;

            editor::apply_selection_move(app, modifiers.contains(KeyModifiers::SHIFT), old_x, old_y);

            editor::move_cursor(app, code);
            editor::update_viewport(app, config, term_w, term_h);

            editor::finalize_selection_move(app, modifiers.contains(KeyModifiers::SHIFT));
        }
        _ => {}
    }
}

fn next_tab(t: MenuTab) -> MenuTab {
    match t {
        MenuTab::Re => MenuTab::File,
        MenuTab::File => MenuTab::Edit,
        MenuTab::Edit => MenuTab::View,
        MenuTab::View => MenuTab::Re,
    }
}

fn prev_tab(t: MenuTab) -> MenuTab {
    match t {
        MenuTab::Re => MenuTab::View,
        MenuTab::View => MenuTab::Edit,
        MenuTab::Edit => MenuTab::File,
        MenuTab::File => MenuTab::Re,
    }
}
