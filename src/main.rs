mod config;
mod editor;
mod state;
mod ui;

use crate::state::{
    AppState, Buffer, ColorEntry, Config, ConfirmChoice, ConfirmType, KeyCombo, KeybindAction,
    MenuTab, Mode, Palette, PromptType, Selection, UndoState, APP_NAME,
};
use crate::ui::redraw_all;

use crossterm::{
    event::{poll, read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use std::{
    env, fs,
    io::{stdout, Stdout},
    time::Duration,
};

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

    let mut mode = Mode::Editing;
    let mut active_tab = MenuTab::Re;
    let mut dropdown_idx: usize = 0;

    redraw_all(&mut stdout, mode, &config, &app, active_tab, dropdown_idx)?;

    loop {
        let mut needs_redraw = false;

        if poll(Duration::from_millis(100))? {
            match read()? {
                Event::Resize(_, _) => {
                    needs_redraw = true;
                }
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    handle_key_event(
                        &mut app,
                        key,
                        &mut mode,
                        &mut active_tab,
                        &mut dropdown_idx,
                        &mut config,
                        &mut stdout,
                    )?;
                    needs_redraw = true;
                }
                _ => {}
            }
        }

        if app.tick_flash() {
            needs_redraw = true;
        }

        if needs_redraw {
            update_viewport(&mut app, &config);
            redraw_all(&mut stdout, mode, &config, &app, active_tab, dropdown_idx)?;
        }
    }
}

fn handle_key_event(
    app: &mut AppState,
    key: KeyEvent,
    mode: &mut Mode,
    active_tab: &mut MenuTab,
    dropdown_idx: &mut usize,
    config: &mut Config,
    stdout: &mut Stdout,
) -> std::io::Result<()> {
    match *mode {
        Mode::ConfirmWipe => {
            if matches!(key.code, KeyCode::Char('y' | 'Y')) {
                app.push_undo();
                let buf = app.current_buffer_mut();
                buf.lines = vec![String::new()];
                buf.cursor_x = 0;
                buf.cursor_y = 0;
                buf.modified = true;
                app.flash_status("BUFFER WIPED".to_string());
                *mode = Mode::Editing;
            } else {
                *mode = Mode::Editing;
            }
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
                    *mode = Mode::Editing;
                }
                KeyCode::Esc => {
                    app.confirm_mode = None;
                    *mode = Mode::Editing;
                }
                _ => needs_redraw = false,
            }

            if save_and_close {
                save_and_close_tab(app);
            } else if close_tab {
                close_current_tab(app);
            }
            if needs_redraw {
                redraw_all(stdout, *mode, config, app, *active_tab, *dropdown_idx)?;
            }
        }

        Mode::Help => {
            if key.code == KeyCode::Esc || key.code == KeyCode::Enter {
                *mode = Mode::Editing;
            }
        }

        Mode::Settings => match key.code {
            KeyCode::Esc => *mode = Mode::Editing,
            KeyCode::Up => app.settings_idx = app.settings_idx.saturating_sub(1),
            KeyCode::Down => {
                if app.settings_idx < 4 {
                    app.settings_idx += 1
                }
            }
            KeyCode::Enter => match app.settings_idx {
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
                    let p = app.current_palette;
                    app.color_entries = vec![
                        ColorEntry {
                            name: "EDITOR BG".to_string(),
                            current_hex: Palette::to_hex(p.editor_bg),
                        },
                        ColorEntry {
                            name: "EDITOR FG".to_string(),
                            current_hex: Palette::to_hex(p.editor_fg),
                        },
                        ColorEntry {
                            name: "UI BG".to_string(),
                            current_hex: Palette::to_hex(p.ui_bg),
                        },
                        ColorEntry {
                            name: "UI FG".to_string(),
                            current_hex: Palette::to_hex(p.ui_fg),
                        },
                        ColorEntry {
                            name: "KEYWORD".to_string(),
                            current_hex: Palette::to_hex(p.keyword),
                        },
                        ColorEntry {
                            name: "SELECTION BG".to_string(),
                            current_hex: Palette::to_hex(p.selection_bg),
                        },
                        ColorEntry {
                            name: "ACCENT PRIMARY".to_string(),
                            current_hex: Palette::to_hex(p.accent_primary),
                        },
                        ColorEntry {
                            name: "ACCENT SECONDARY".to_string(),
                            current_hex: Palette::to_hex(p.accent_secondary),
                        },
                        ColorEntry {
                            name: "WARNING".to_string(),
                            current_hex: Palette::to_hex(p.warning),
                        },
                    ];
                    app.color_editor_idx = 0;
                    app.editing_hex = false;
                    *mode = Mode::ColorEditor;
                }
                3 => {
                    *mode = Mode::KeyRebind;
                    let kb = &mut app.keybind_state;
                    kb.in_rebind_mode = true;
                    kb.selected_action = 0;
                    kb.waiting_for_key = false;
                    kb.pending_action = None;
                    kb.scroll_offset = 0;
                    kb.confirming_reset = false;
                }
                4 => *mode = Mode::Editing,
                _ => {}
            },
            _ => {}
        },

        Mode::KeyRebind => {
            let mut flash_msg: Option<String> = None;
            {
                let kb = &mut app.keybind_state;
                let total_actions = 16;

                if kb.waiting_for_key {
                    if let Some(index) = kb.pending_action {
                        if let Some(action) = KeybindAction::from_index(index) {
                            let combo = KeyCombo {
                                code: key.code,
                                modifiers: key.modifiers,
                            };
                            kb.custom_binds.insert(combo, action);

                            save_keybind_to_config(config, &combo, action);
                            let _ = config::save_config(config);

                            let mod_str = if key.modifiers.contains(KeyModifiers::CONTROL) {
                                "Ctrl+"
                            } else if key.modifiers.contains(KeyModifiers::SHIFT) {
                                "Shift+"
                            } else if key.modifiers.contains(KeyModifiers::ALT) {
                                "Alt+"
                            } else {
                                ""
                            };
                            let key_str = format!("{:?}", key.code);
                            flash_msg = Some(format!(
                                "{} BOUND & SAVED",
                                format!("{mod_str}{key_str}").trim()
                            ));
                        }
                    }
                    kb.waiting_for_key = false;
                    kb.pending_action = None;
                } else if kb.confirming_reset {
                    if matches!(key.code, KeyCode::Char('y' | 'Y')) {
                        kb.custom_binds.clear();
                        config.custom_keybinds.clear();
                        let _ = config::save_config(config);
                        flash_msg = Some("ALL KEYBINDS RESET & SAVED".to_string());
                    } else {
                        flash_msg = Some("RESET CANCELLED".to_string());
                    }
                    kb.confirming_reset = false;
                } else {
                    match key.code {
                        KeyCode::Esc => {
                            *mode = Mode::Settings;
                            kb.in_rebind_mode = false;
                        }
                        KeyCode::Up => {
                            if kb.selected_action > 0 {
                                kb.selected_action -= 1;
                                if kb.selected_action < kb.scroll_offset {
                                    kb.scroll_offset = kb.selected_action;
                                }
                            }
                        }
                        KeyCode::Down => {
                            if kb.selected_action < total_actions - 1 {
                                kb.selected_action += 1;
                                if kb.selected_action >= kb.scroll_offset + 14 {
                                    kb.scroll_offset = kb.selected_action.saturating_sub(13);
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if kb.selected_action == 15 {
                                kb.confirming_reset = true;
                                flash_msg = Some(
                                    "RESET ALL BINDS? Press Y to confirm, anything else cancel"
                                        .to_string(),
                                );
                            } else if KeybindAction::from_index(kb.selected_action).is_some() {
                                kb.waiting_for_key = true;
                                kb.pending_action = Some(kb.selected_action);
                                flash_msg = Some("PRESS NEW KEY • Esc to cancel".to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            if let Some(m) = flash_msg {
                app.flash_status(m);
            }
        }

        Mode::ColorEditor => match key.code {
            KeyCode::Esc => *mode = Mode::Settings,
            KeyCode::Up => {
                app.color_editor_idx = app.color_editor_idx.saturating_sub(1);
                app.editing_hex = false;
            }
            KeyCode::Down => {
                if app.color_editor_idx < app.color_entries.len() - 1 {
                    app.color_editor_idx += 1;
                }
                app.editing_hex = false;
            }
            KeyCode::Enter => app.editing_hex = !app.editing_hex,
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Ok(new_palette) = parse_palette_from_entries(&app.color_entries) {
                    app.current_palette = new_palette;
                    config.palette = new_palette.to_config();
                    let _ = config::save_config(config);
                    app.flash_status("COLORS SAVED".to_string());
                }
                *mode = Mode::Settings;
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
            }
            KeyCode::Down => {
                if app.explorer_idx < app.explorer_files.len().saturating_sub(1) {
                    app.explorer_idx += 1;
                    let (_, term_h) = size().unwrap_or((80, 24));
                    let visible = term_h.saturating_sub(4) as usize;
                    if app.explorer_idx >= app.explorer_offset + visible {
                        app.explorer_offset = app.explorer_idx - visible + 1;
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(selected) = app.explorer_files.get(app.explorer_idx).cloned() {
                    let clean_name = selected.trim_start_matches("📁 ").trim_start_matches("📄 ");
                    let full_path = app.current_dir.join(clean_name);
                    if full_path.is_dir() {
                        app.current_dir = full_path;
                        let _ = refresh_explorer(app);
                    } else {
                        match editor::load_from_file(full_path.to_str().unwrap_or(&selected)) {
                            Ok(lines) => {
                                let mut new_buffer = Buffer::new(clean_name.to_string());
                                new_buffer.lines = lines;
                                app.buffers.push(new_buffer);
                                app.active_buffer = app.buffers.len() - 1;
                                *mode = Mode::Editing;
                            }
                            Err(e) => {
                                app.flash_status(format!("OPEN FAILED: {}", e));
                            }
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(parent) = app.current_dir.parent() {
                    app.current_dir = parent.to_path_buf();
                    let _ = refresh_explorer(app);
                }
            }
            KeyCode::Esc => *mode = Mode::Menu,
            _ => {}
        },

        Mode::Editing => {
            if app.input_mode {
                handle_prompt_input(app, key.code, mode, config);
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
                    redraw_all(stdout, *mode, config, app, *active_tab, *dropdown_idx)?;
                    return Ok(());
                }

                handle_editing_input(app, key, config, mode);
            }
        }

        Mode::Menu => match key.code {
            KeyCode::Esc => *mode = Mode::Editing,
            KeyCode::Left => {
                *active_tab = prev_tab(*active_tab);
                *dropdown_idx = 0;
            }
            KeyCode::Right => {
                *active_tab = next_tab(*active_tab);
                *dropdown_idx = 0;
            }
            KeyCode::Up => *dropdown_idx = dropdown_idx.saturating_sub(1),
            KeyCode::Down => *dropdown_idx += 1,
            KeyCode::Enter => {
                let (exit, next_mode) =
                    handle_menu_selection(*active_tab, *dropdown_idx, config, app)?;
                if exit {
                    let _ = execute!(stdout, LeaveAlternateScreen);
                    let _ = disable_raw_mode();
                    std::process::exit(0);
                }
                *mode = next_mode;
                if *mode == Mode::Explorer {
                    let _ = refresh_explorer(app);
                }
            }
            _ => {}
        },
    }

    Ok(())
}

fn load_custom_keybinds(app: &mut AppState, config: &Config) {
    for (combo_str, action_str) in &config.custom_keybinds {
        if let (Some(combo), Some(action)) = (
            KeyCombo::from_string(combo_str),
            action_str.parse::<KeybindAction>().ok(),
        ) {
            app.keybind_state.custom_binds.insert(combo, action);
        }
    }
}

fn save_keybind_to_config(config: &mut Config, combo: &KeyCombo, action: KeybindAction) {
    let combo_str = combo.to_string();
    let action_str = action.to_str().to_string();

    config.custom_keybinds.retain(|(k, _)| k != &combo_str);
    config.custom_keybinds.push((combo_str, action_str));
}

fn save_and_close_tab(app: &mut AppState) {
    let filename = app.current_buffer().filename.clone();
    let lines = app.current_buffer().lines.clone();
    if let Err(e) = editor::save_to_file(&lines, &filename) {
        app.flash_status(format!("SAVE FAILED: {}", e));
        return;
    }
    close_current_tab(app);
}

fn close_current_tab(app: &mut AppState) {
    if app.buffers.len() > 1 {
        app.buffers.remove(app.active_buffer);
        if app.active_buffer >= app.buffers.len() {
            app.active_buffer = app.buffers.len() - 1;
        }
    }
}

fn handle_editing_input(app: &mut AppState, key: KeyEvent, config: &Config, mode: &mut Mode) {
    let code = key.code;
    let modifiers = key.modifiers;

    let combo = KeyCombo { code, modifiers };

    if let Some(&action) = app.keybind_state.custom_binds.get(&combo) {
        perform_keybind_action(app, action, config, mode);
        return;
    }

    if let Some(sel) = app.selection.as_ref() {
        let (sx, sy, ex, ey) = sel.normalized();
        let selected_text = extract_selected_text(app.current_buffer(), sx, sy, ex, ey);

        match code {
            KeyCode::Backspace | KeyCode::Delete => {
                app.push_undo();
                delete_selection(app.current_buffer_mut(), sx, sy, ex, ey);
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
                delete_selection(app.current_buffer_mut(), sx, sy, ex, ey);
                app.selection = None;
                app.current_buffer_mut().modified = true;
                return;
            }
            KeyCode::Char('a') if modifiers.contains(KeyModifiers::CONTROL) => {
                let buf = app.current_buffer();
                if !buf.lines.is_empty() {
                    app.selection = Some(Selection {
                        start_x: 0,
                        start_y: 0,
                        end_x: buf.lines[buf.lines.len() - 1].len(),
                        end_y: buf.lines.len() - 1,
                    });
                }
                return;
            }
            _ => {}
        }
    }

    if code == KeyCode::Char('v') && modifiers.contains(KeyModifiers::CONTROL) {
        if !app.clipboard.is_empty() {
            app.push_undo();
            let paste_text = app.clipboard.clone();
            let buf = app.current_buffer_mut();
            for ch in paste_text.chars().rev() {
                if ch == '\n' {
                    let remaining = buf.lines[buf.cursor_y].split_off(buf.cursor_x);
                    buf.lines.insert(buf.cursor_y + 1, remaining);
                    buf.cursor_y += 1;
                    buf.cursor_x = 0;
                } else {
                    buf.lines[buf.cursor_y].insert(buf.cursor_x, ch);
                    buf.cursor_x += 1;
                }
            }
            buf.modified = true;
            app.selection = None;
        }
        return;
    }

    match code {
        KeyCode::Char('a') if modifiers.contains(KeyModifiers::CONTROL) => {
            let buf = app.current_buffer();
            if !buf.lines.is_empty() {
                app.selection = Some(Selection {
                    start_x: 0,
                    start_y: 0,
                    end_x: buf.lines[buf.lines.len() - 1].len(),
                    end_y: buf.lines.len() - 1,
                });
            }
        }
        KeyCode::Up if modifiers.contains(KeyModifiers::CONTROL) => {
            let buf = app.current_buffer_mut();
            buf.cursor_y = 0;
            buf.cursor_x = 0;
            update_viewport(app, config);
            app.selection = None;
        }
        KeyCode::Down if modifiers.contains(KeyModifiers::CONTROL) => {
            let buf = app.current_buffer_mut();
            buf.cursor_y = buf.lines.len().saturating_sub(1);
            buf.cursor_x = buf.lines[buf.cursor_y].len();
            update_viewport(app, config);
            app.selection = None;
        }
        KeyCode::Tab => {
            app.push_undo();
            let buf = app.current_buffer_mut();
            let spaces = " ".repeat(config.tab_size);
            buf.lines[buf.cursor_y].insert_str(buf.cursor_x, &spaces);
            buf.cursor_x += config.tab_size;
            buf.modified = true;
        }
        KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => {
            let filename = app.current_buffer().filename.clone();
            let lines = app.current_buffer().lines.clone();
            if let Err(e) = editor::save_to_file(&lines, &filename) {
                app.flash_status(format!("SAVE FAILED: {}", e));
            } else {
                app.current_buffer_mut().modified = false;
                app.flash_status("SAVED".to_string());
            }
        }
        KeyCode::Char('z') if modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(undo) = app.undo_stack.pop() {
                let current = app.current_buffer();
                let redo_state = UndoState {
                    lines: current.lines.clone(),
                    cursor_x: current.cursor_x,
                    cursor_y: current.cursor_y,
                };
                let buf = app.current_buffer_mut();
                buf.lines = undo.lines;
                buf.cursor_x = undo.cursor_x;
                buf.cursor_y = undo.cursor_y;
                buf.modified = true;
                app.redo_stack.push(redo_state);
            }
        }
        KeyCode::Char('y') if modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(redo) = app.redo_stack.pop() {
                let current = app.current_buffer();
                let undo_state = UndoState {
                    lines: current.lines.clone(),
                    cursor_x: current.cursor_x,
                    cursor_y: current.cursor_y,
                };
                let buf = app.current_buffer_mut();
                buf.lines = redo.lines;
                buf.cursor_x = redo.cursor_x;
                buf.cursor_y = redo.cursor_y;
                buf.modified = true;
                app.undo_stack.push(undo_state);
            }
        }
        KeyCode::Esc => {
            app.selection = None;
            *mode = Mode::Menu;
        }
        KeyCode::Char(c) => {
            app.push_undo();
            let buf = app.current_buffer_mut();
            buf.lines[buf.cursor_y].insert(buf.cursor_x, c);
            buf.cursor_x += 1;
            buf.modified = true;
            app.selection = None;
        }
        KeyCode::Enter => {
            app.push_undo();
            let buf = app.current_buffer_mut();
            let remaining = buf.lines[buf.cursor_y].split_off(buf.cursor_x);
            buf.lines.insert(buf.cursor_y + 1, remaining);
            buf.cursor_y += 1;
            buf.cursor_x = 0;
            buf.modified = true;
            app.selection = None;
        }
        KeyCode::Backspace => {
            app.push_undo();
            let buf = app.current_buffer_mut();
            if buf.cursor_x > 0 {
                buf.cursor_x -= 1;
                buf.lines[buf.cursor_y].remove(buf.cursor_x);
            } else if buf.cursor_y > 0 {
                let current_line = buf.lines.remove(buf.cursor_y);
                buf.cursor_y -= 1;
                buf.cursor_x = buf.lines[buf.cursor_y].len();
                buf.lines[buf.cursor_y].push_str(&current_line);
            }
            buf.modified = true;
            app.selection = None;
        }
        KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
            let old_x = app.current_buffer().cursor_x;
            let old_y = app.current_buffer().cursor_y;

            if modifiers.contains(KeyModifiers::SHIFT) && app.selection.is_none() {
                app.selection = Some(Selection::new(old_x, old_y));
            }

            {
                let buf = app.current_buffer_mut();
                match code {
                    KeyCode::Up if buf.cursor_y > 0 => {
                        buf.cursor_y -= 1;
                        buf.cursor_x = buf.cursor_x.min(buf.lines[buf.cursor_y].len());
                    }
                    KeyCode::Down if buf.cursor_y < buf.lines.len() - 1 => {
                        buf.cursor_y += 1;
                        buf.cursor_x = buf.cursor_x.min(buf.lines[buf.cursor_y].len());
                    }
                    KeyCode::Left if buf.cursor_x > 0 => buf.cursor_x -= 1,
                    KeyCode::Right if buf.cursor_x < buf.lines[buf.cursor_y].len() => {
                        buf.cursor_x += 1
                    }
                    _ => {}
                }
            }

            update_viewport(app, config);

            let new_x = app.current_buffer().cursor_x;
            let new_y = app.current_buffer().cursor_y;

            if modifiers.contains(KeyModifiers::SHIFT) {
                if let Some(sel) = &mut app.selection {
                    sel.end_x = new_x;
                    sel.end_y = new_y;
                }
            } else {
                app.selection = None;
            }
        }
        _ => {}
    }
}

fn perform_keybind_action(
    app: &mut AppState,
    action: KeybindAction,
    _config: &Config,
    mode: &mut Mode,
) {
    match action {
        KeybindAction::Menu => *mode = Mode::Menu,
        KeybindAction::Save => {
            let filename = app.current_buffer().filename.clone();
            let lines = app.current_buffer().lines.clone();
            if let Err(e) = editor::save_to_file(&lines, &filename) {
                app.flash_status(format!("SAVE FAILED: {}", e));
            } else {
                app.current_buffer_mut().modified = false;
                app.flash_status("SAVED".to_string());
            }
        }
        KeybindAction::Undo => {
            if let Some(undo) = app.undo_stack.pop() {
                let current = app.current_buffer();
                let redo_state = UndoState {
                    lines: current.lines.clone(),
                    cursor_x: current.cursor_x,
                    cursor_y: current.cursor_y,
                };
                let buf = app.current_buffer_mut();
                buf.lines = undo.lines;
                buf.cursor_x = undo.cursor_x;
                buf.cursor_y = undo.cursor_y;
                buf.modified = true;
                app.redo_stack.push(redo_state);
            }
        }
        KeybindAction::Redo => {
            if let Some(redo) = app.redo_stack.pop() {
                let current = app.current_buffer();
                let undo_state = UndoState {
                    lines: current.lines.clone(),
                    cursor_x: current.cursor_x,
                    cursor_y: current.cursor_y,
                };
                let buf = app.current_buffer_mut();
                buf.lines = redo.lines;
                buf.cursor_x = redo.cursor_x;
                buf.cursor_y = redo.cursor_y;
                buf.modified = true;
                app.undo_stack.push(undo_state);
            }
        }
        KeybindAction::NewTab => {
            app.buffers.push(Buffer::new("unsaved.txt".to_string()));
            app.active_buffer = app.buffers.len() - 1;
        }
        KeybindAction::CloseTab => {
            if app.current_buffer().modified {
                app.confirm_mode = Some(ConfirmType::CloseTab);
                app.confirm_choice = ConfirmChoice::No;
            } else {
                close_current_tab(app);
            }
        }
        KeybindAction::NextTab => {
            if app.buffers.len() > 1 {
                app.active_buffer = (app.active_buffer + 1) % app.buffers.len();
            }
        }
        KeybindAction::PrevTab => {
            if app.buffers.len() > 1 {
                app.active_buffer = if app.active_buffer == 0 {
                    app.buffers.len() - 1
                } else {
                    app.active_buffer - 1
                };
            }
        }
        KeybindAction::SelectAll => {
            let buf = app.current_buffer();
            if !buf.lines.is_empty() {
                app.selection = Some(Selection {
                    start_x: 0,
                    start_y: 0,
                    end_x: buf.lines[buf.lines.len() - 1].len(),
                    end_y: buf.lines.len() - 1,
                });
            }
        }
        KeybindAction::Copy => {
            if let Some(sel) = app.selection.as_ref() {
                let (sx, sy, ex, ey) = sel.normalized();
                app.clipboard = extract_selected_text(app.current_buffer(), sx, sy, ex, ey);
                app.flash_status("COPIED".to_string());
            }
        }
        KeybindAction::Cut => {
            if let Some(sel) = app.selection.as_ref() {
                let (sx, sy, ex, ey) = sel.normalized();
                app.clipboard = extract_selected_text(app.current_buffer(), sx, sy, ex, ey);
                app.push_undo();
                delete_selection(app.current_buffer_mut(), sx, sy, ex, ey);
                app.selection = None;
                app.current_buffer_mut().modified = true;
                app.flash_status("CUT".to_string());
            }
        }
        KeybindAction::Paste => {
            if !app.clipboard.is_empty() {
                app.push_undo();
                let paste_text = app.clipboard.clone();
                let buf = app.current_buffer_mut();
                for ch in paste_text.chars().rev() {
                    if ch == '\n' {
                        let remaining = buf.lines[buf.cursor_y].split_off(buf.cursor_x);
                        buf.lines.insert(buf.cursor_y + 1, remaining);
                        buf.cursor_y += 1;
                        buf.cursor_x = 0;
                    } else {
                        buf.lines[buf.cursor_y].insert(buf.cursor_x, ch);
                        buf.cursor_x += 1;
                    }
                }
                buf.modified = true;
                app.selection = None;
            }
        }
        KeybindAction::Find => {
            app.input_mode = true;
            app.prompt_type = PromptType::Find;
            app.input_buffer.clear();
        }
        KeybindAction::GoToLine => {
            app.input_mode = true;
            app.prompt_type = PromptType::GoToLine;
            app.input_buffer.clear();
        }
        KeybindAction::WipeBuffer => *mode = Mode::ConfirmWipe,
        KeybindAction::ResetToDefault => {}
    }
}

fn extract_selected_text(buf: &Buffer, sx: usize, sy: usize, ex: usize, ey: usize) -> String {
    let mut text = String::new();
    for y in sy..=ey {
        let line = &buf.lines[y];
        let start = if y == sy { sx } else { 0 };
        let end = if y == ey { ex } else { line.len() };
        if start < end {
            text.push_str(&line[start..end]);
        }
        if y < ey {
            text.push('\n');
        }
    }
    text
}

fn delete_selection(buf: &mut Buffer, sx: usize, sy: usize, ex: usize, ey: usize) {
    if sy == ey {
        buf.lines[sy].drain(sx..ex);
        buf.cursor_x = sx;
    } else {
        let mut new_line = buf.lines[sy][..sx].to_string();
        new_line.push_str(&buf.lines[ey][ex..]);
        buf.lines.splice(sy..=ey, std::iter::once(new_line));
        buf.cursor_y = sy;
        buf.cursor_x = sx;
    }
}

fn update_viewport(app: &mut AppState, config: &Config) {
    let (term_w, term_h) = size().unwrap_or((80, 24));

    let buf = app.current_buffer_mut();
    let sidebar_width = if config.show_line_numbers { 6 } else { 0 };
    let available_width = term_w.saturating_sub(sidebar_width) as usize;
    let available_height = term_h.saturating_sub(
        if config.show_header { 1 } else { 0 }
            + if config.show_tab_bar { 1 } else { 0 }
            + if config.show_status_bar { 1 } else { 0 },
    ) as usize;

    if buf.cursor_x < buf.viewport_offset_x {
        buf.viewport_offset_x = buf.cursor_x;
    } else if buf.cursor_x >= buf.viewport_offset_x + available_width {
        buf.viewport_offset_x = buf.cursor_x.saturating_sub(available_width - 1);
    }

    if buf.cursor_y < buf.viewport_offset_y {
        buf.viewport_offset_y = buf.cursor_y;
    } else if buf.cursor_y >= buf.viewport_offset_y + available_height {
        buf.viewport_offset_y = buf.cursor_y.saturating_sub(available_height - 1);
    }
}

fn handle_prompt_input(app: &mut AppState, code: KeyCode, _mode: &mut Mode, config: &Config) {
    match code {
        KeyCode::Esc => {
            app.input_mode = false;
            app.input_buffer.clear();
        }
        KeyCode::Enter => {
            let input = app.input_buffer.clone();
            match app.prompt_type {
                PromptType::SaveAs => {
                    app.input_buffer.clear();
                    app.input_mode = false;
                    let buf = app.current_buffer_mut();
                    buf.filename = input.clone();
                    if let Err(e) = editor::save_to_file(&buf.lines, &input) {
                        app.flash_status(format!("SAVE FAILED: {}", e));
                    } else {
                        buf.modified = false;
                        app.flash_status("SAVED".to_string());
                    }
                }
                PromptType::GoToLine => {
                    if let Ok(num) = input.parse::<usize>() {
                        let target = num.saturating_sub(1);
                        let buf = app.current_buffer_mut();
                        if target < buf.lines.len() {
                            buf.cursor_y = target;
                            buf.cursor_x = 0;
                            update_viewport(app, config);
                        }
                    }
                }
                PromptType::Find => {
                    let buf = app.current_buffer();
                    for (i, line) in buf.lines.iter().enumerate().skip(buf.cursor_y) {
                        if let Some(pos) = line.find(&input) {
                            let buf = app.current_buffer_mut();
                            buf.cursor_y = i;
                            buf.cursor_x = pos;
                            update_viewport(app, config);

                            break;
                        }
                    }
                }
                _ => {}
            }
            app.input_mode = false;
            app.input_buffer.clear();
        }
        KeyCode::Char(c) => app.input_buffer.push(c),
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        _ => {}
    }
}

fn refresh_explorer(app: &mut AppState) -> std::io::Result<()> {
    let mut files = Vec::new();
    for entry in fs::read_dir(&app.current_dir)?.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if entry.path().is_dir() {
            files.push(format!("📁 {}", name));
        } else {
            files.push(format!("📄 {}", name));
        }
    }
    files.sort_by(|a, b| {
        let a_dir = a.starts_with("📁");
        let b_dir = b.starts_with("📁");
        (!a_dir)
            .cmp(&!b_dir)
            .then(a.to_lowercase().cmp(&b.to_lowercase()))
    });
    app.explorer_files = files;
    app.explorer_idx = 0;
    app.explorer_offset = 0;
    Ok(())
}

fn parse_palette_from_entries(entries: &[ColorEntry]) -> Result<Palette, ()> {
    let mut p = Palette::default();
    for e in entries {
        if e.current_hex.len() == 7 && e.current_hex.starts_with('#') {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&e.current_hex[1..3], 16),
                u8::from_str_radix(&e.current_hex[3..5], 16),
                u8::from_str_radix(&e.current_hex[5..7], 16),
            ) {
                let color = crossterm::style::Color::Rgb { r, g, b };
                match e.name.as_str() {
                    "EDITOR BG" => p.editor_bg = color,
                    "EDITOR FG" => p.editor_fg = color,
                    "UI BG" => p.ui_bg = color,
                    "UI FG" => p.ui_fg = color,
                    "KEYWORD" => p.keyword = color,
                    "SELECTION BG" => p.selection_bg = color,
                    "ACCENT PRIMARY" => p.accent_primary = color,
                    "ACCENT SECONDARY" => p.accent_secondary = color,
                    "WARNING" => p.warning = color,
                    _ => {}
                }
            }
        }
    }
    Ok(p)
}

fn handle_menu_selection(
    tab: MenuTab,
    idx: usize,
    config: &mut Config,
    app: &mut AppState,
) -> std::io::Result<(bool, Mode)> {
    match tab {
        MenuTab::Re => match idx % 4 {
            0 => Ok((false, Mode::Settings)),
            1 => Ok((false, Mode::Help)),
            2 => Ok((true, Mode::Editing)),
            3 => {
                let _ = config::save_config(config);
                Ok((true, Mode::Editing))
            }
            _ => Ok((false, Mode::Editing)),
        },
        MenuTab::File => match idx % 6 {
            0 => {
                app.buffers.push(Buffer::new("unsaved.txt".to_string()));
                app.active_buffer = app.buffers.len() - 1;
                Ok((false, Mode::Editing))
            }
            1 => Ok((false, Mode::Explorer)),
            2 => {
                if app.current_buffer().modified {
                    app.confirm_mode = Some(ConfirmType::CloseTab);
                    app.confirm_choice = ConfirmChoice::No;
                    Ok((false, Mode::Editing))
                } else {
                    close_current_tab(app);
                    Ok((false, Mode::Editing))
                }
            }
            3 => {
                if app.buffers.len() > 1 {
                    app.active_buffer = (app.active_buffer + 1) % app.buffers.len();
                }
                Ok((false, Mode::Editing))
            }
            4 => {
                if app.buffers.len() > 1 {
                    app.active_buffer = if app.active_buffer == 0 {
                        app.buffers.len() - 1
                    } else {
                        app.active_buffer - 1
                    };
                }
                Ok((false, Mode::Editing))
            }
            5 => {
                let buf = app.current_buffer();
                app.input_buffer = format!("./{}", buf.filename);
                app.prompt_type = PromptType::SaveAs;
                app.input_mode = true;

                Ok((false, Mode::Editing))
            }
            _ => Ok((false, Mode::Editing)),
        },
        MenuTab::Edit => match idx % 4 {
            0 => {
                app.input_mode = true;
                app.prompt_type = PromptType::Find;
                app.input_buffer.clear();
                Ok((false, Mode::Editing))
            }
            1 => Ok((false, Mode::Editing)),
            2 => {
                app.input_mode = true;
                app.prompt_type = PromptType::GoToLine;
                app.input_buffer.clear();
                Ok((false, Mode::Editing))
            }
            3 => Ok((false, Mode::ConfirmWipe)),
            _ => Ok((false, Mode::Editing)),
        },
        MenuTab::View => {
            match idx % 5 {
                0 => config.show_header = !config.show_header,
                1 => config.show_status_bar = !config.show_status_bar,
                2 => config.show_line_numbers = !config.show_line_numbers,
                3 => {
                    config.show_tab_bar = !config.show_tab_bar;
                    let _ = config::save_config(config);
                    app.flash_status(format!(
                        "TAB BAR {}",
                        if config.show_tab_bar {
                            "SHOWN"
                        } else {
                            "HIDDEN"
                        }
                    ));
                }
                4 => {
                    config.syntax_highlight = !config.syntax_highlight;
                    let _ = config::save_config(config);
                    app.flash_status(format!(
                        "SYNTAX {}",
                        if config.syntax_highlight {
                            "ENABLED"
                        } else {
                            "DISABLED"
                        }
                    ));
                }
                _ => {}
            }
            Ok((false, Mode::Editing))
        }
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
