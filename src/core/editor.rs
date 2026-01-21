use crate::core::io;
use crate::core::state::{
    AppState, Buffer, ColorEntry, Config, ConfirmChoice, ConfirmType, KeyCombo, KeybindAction,
    MenuTab, Mode, Palette, PaletteConfig, PromptType, Selection, UndoState,
};
use crossterm::event::{KeyCode, KeyModifiers};
use std::fs;

pub struct MenuSelection {
    pub exit: bool,
    pub next_mode: Mode,
}

pub fn save_and_close_tab(app: &mut AppState) -> std::io::Result<()> {
    let filename = app.current_buffer().filename.clone();
    let lines = app.current_buffer().lines.clone();
    if let Err(e) = io::save_to_file(&lines, &filename) {
        app.flash_status(format!("SAVE FAILED: {}", e));
        return Ok(());
    }
    close_current_tab(app);
    Ok(())
}

pub fn close_current_tab(app: &mut AppState) {
    if app.buffers.len() > 1 {
        app.buffers.remove(app.active_buffer);
        if app.active_buffer >= app.buffers.len() {
            app.active_buffer = app.buffers.len() - 1;
        }
    }
}

pub fn handle_keybind_capture(
    app: &mut AppState,
    config: &mut Config,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Option<String> {
    let kb = &mut app.keybind_state;
    if kb.waiting_for_key {
        if let Some(index) = kb.pending_action {
            if let Some(action) = KeybindAction::from_index(index) {
                let combo = KeyCombo { code, modifiers };
                kb.custom_binds.insert(combo, action);

                save_keybind_to_config(config, &combo, action);
                let _ = crate::config::save_config(config);

                let mod_str = if modifiers.contains(KeyModifiers::CONTROL) {
                    "Ctrl+"
                } else if modifiers.contains(KeyModifiers::SHIFT) {
                    "Shift+"
                } else if modifiers.contains(KeyModifiers::ALT) {
                    "Alt+"
                } else {
                    ""
                };
                let key_str = format!("{:?}", code);
                let msg = format!("{} BOUND & SAVED", format!("{mod_str}{key_str}").trim());
                kb.waiting_for_key = false;
                kb.pending_action = None;
                return Some(msg);
            }
        }
        kb.waiting_for_key = false;
        kb.pending_action = None;
        return None;
    }

    if kb.confirming_reset {
        if matches!(code, KeyCode::Char('y' | 'Y')) {
            kb.custom_binds.clear();
            config.custom_keybinds.clear();
            let _ = crate::config::save_config(config);
            kb.confirming_reset = false;
            return Some("ALL KEYBINDS RESET & SAVED".to_string());
        }
        kb.confirming_reset = false;
        return Some("RESET CANCELLED".to_string());
    }

    None
}

pub fn handle_keybind_navigation(app: &mut AppState, code: KeyCode) -> Option<String> {
    let kb = &mut app.keybind_state;
    let total_actions = 16;

    match code {
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
                return Some(
                    "RESET ALL BINDS? Press Y to confirm, anything else cancel".to_string(),
                );
            }
            if KeybindAction::from_index(kb.selected_action).is_some() {
                kb.waiting_for_key = true;
                kb.pending_action = Some(kb.selected_action);
                return Some("PRESS NEW KEY • Esc to cancel".to_string());
            }
        }
        _ => {}
    }

    None
}

pub fn parse_palette_from_entries(entries: &[ColorEntry]) -> Result<Palette, ()> {
    let mut config = PaletteConfig::default();
    for e in entries {
        if e.current_hex.len() == 7 && e.current_hex.starts_with('#') {
            let hex_val = e.current_hex.clone();
            match e.name.as_str() {
                "ui_background" => config.ui_background = hex_val,
                "ui_foreground" => config.ui_foreground = hex_val,
                "ui_border" => config.ui_border = hex_val,
                "status_bar_bg" => config.status_bar_bg = hex_val,
                "status_bar_fg" => config.status_bar_fg = hex_val,
                "header_bg" => config.header_bg = hex_val,
                "header_fg" => config.header_fg = hex_val,
                "editor_background" => config.editor_background = hex_val,
                "editor_foreground" => config.editor_foreground = hex_val,
                "line_number_bg" => config.line_number_bg = hex_val,
                "line_number_fg" => config.line_number_fg = hex_val,
                "cursor" => config.cursor = hex_val,
                "selection_bg" => config.selection_bg = hex_val,
                "selection_fg" => config.selection_fg = hex_val,
                "syntax_keyword" => config.syntax_keyword = hex_val,
                "syntax_string" => config.syntax_string = hex_val,
                "syntax_comment" => config.syntax_comment = hex_val,
                "syntax_function" => config.syntax_function = hex_val,
                "syntax_type" => config.syntax_type = hex_val,
                "syntax_constant" => config.syntax_constant = hex_val,
                "accent_primary" => config.accent_primary = hex_val,
                "accent_secondary" => config.accent_secondary = hex_val,
                "match_highlight" => config.match_highlight = hex_val,
                "error" => config.error = hex_val,
                "warning" => config.warning = hex_val,
                _ => {}
            }
        }
    }
    Ok(Palette::from_config(&config))
}

pub fn refresh_explorer(app: &mut AppState) -> std::io::Result<()> {
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

pub fn handle_prompt_input(
    app: &mut AppState,
    code: KeyCode,
    config: &Config,
    term_w: u16,
    term_h: u16,
) {
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
                    if let Err(e) = io::save_to_file(&buf.lines, &input) {
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
                            update_viewport(app, config, term_w, term_h);
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
                            update_viewport(app, config, term_w, term_h);

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

pub fn handle_menu_selection(
    tab: MenuTab,
    idx: usize,
    config: &mut Config,
    app: &mut AppState,
) -> std::io::Result<MenuSelection> {
    match tab {
        MenuTab::Re => match idx % 4 {
            0 => Ok(MenuSelection {
                exit: false,
                next_mode: Mode::Settings,
            }),
            1 => Ok(MenuSelection {
                exit: false,
                next_mode: Mode::Help,
            }),
            2 => Ok(MenuSelection {
                exit: true,
                next_mode: Mode::Editing,
            }),
            3 => {
                let _ = crate::config::save_config(config);
                Ok(MenuSelection {
                    exit: true,
                    next_mode: Mode::Editing,
                })
            }
            _ => Ok(MenuSelection {
                exit: false,
                next_mode: Mode::Editing,
            }),
        },
        MenuTab::File => match idx % 6 {
            0 => {
                app.buffers.push(Buffer::new("unsaved.txt".to_string()));
                app.active_buffer = app.buffers.len() - 1;
                Ok(MenuSelection {
                    exit: false,
                    next_mode: Mode::Editing,
                })
            }
            1 => Ok(MenuSelection {
                exit: false,
                next_mode: Mode::Explorer,
            }),
            2 => {
                if app.current_buffer().modified {
                    app.confirm_mode = Some(ConfirmType::CloseTab);
                    app.confirm_choice = ConfirmChoice::No;
                    Ok(MenuSelection {
                        exit: false,
                        next_mode: Mode::Editing,
                    })
                } else {
                    close_current_tab(app);
                    Ok(MenuSelection {
                        exit: false,
                        next_mode: Mode::Editing,
                    })
                }
            }
            3 => {
                if app.buffers.len() > 1 {
                    app.active_buffer = (app.active_buffer + 1) % app.buffers.len();
                }
                Ok(MenuSelection {
                    exit: false,
                    next_mode: Mode::Editing,
                })
            }
            4 => {
                if app.buffers.len() > 1 {
                    app.active_buffer = if app.active_buffer == 0 {
                        app.buffers.len() - 1
                    } else {
                        app.active_buffer - 1
                    };
                }
                Ok(MenuSelection {
                    exit: false,
                    next_mode: Mode::Editing,
                })
            }
            5 => {
                let buf = app.current_buffer();
                app.input_buffer = format!("./{}", buf.filename);
                app.prompt_type = PromptType::SaveAs;
                app.input_mode = true;

                Ok(MenuSelection {
                    exit: false,
                    next_mode: Mode::Editing,
                })
            }
            _ => Ok(MenuSelection {
                exit: false,
                next_mode: Mode::Editing,
            }),
        },
        MenuTab::Edit => match idx % 4 {
            0 => {
                app.input_mode = true;
                app.prompt_type = PromptType::Find;
                app.input_buffer.clear();
                Ok(MenuSelection {
                    exit: false,
                    next_mode: Mode::Editing,
                })
            }
            1 => Ok(MenuSelection {
                exit: false,
                next_mode: Mode::Editing,
            }),
            2 => {
                app.input_mode = true;
                app.prompt_type = PromptType::GoToLine;
                app.input_buffer.clear();
                Ok(MenuSelection {
                    exit: false,
                    next_mode: Mode::Editing,
                })
            }
            3 => Ok(MenuSelection {
                exit: false,
                next_mode: Mode::ConfirmWipe,
            }),
            _ => Ok(MenuSelection {
                exit: false,
                next_mode: Mode::Editing,
            }),
        },
        MenuTab::View => {
            match idx % 5 {
                0 => config.show_header = !config.show_header,
                1 => config.show_status_bar = !config.show_status_bar,
                2 => config.show_line_numbers = !config.show_line_numbers,
                3 => {
                    config.show_tab_bar = !config.show_tab_bar;
                    let _ = crate::config::save_config(config);
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
                    let _ = crate::config::save_config(config);
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
            Ok(MenuSelection {
                exit: false,
                next_mode: Mode::Editing,
            })
        }
    }
}

pub fn update_viewport(app: &mut AppState, config: &Config, term_w: u16, term_h: u16) {
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
        buf.viewport_offset_x = buf.cursor_x.saturating_sub(available_width.saturating_sub(1));
    }

    if buf.cursor_y < buf.viewport_offset_y {
        buf.viewport_offset_y = buf.cursor_y;
    } else if buf.cursor_y >= buf.viewport_offset_y + available_height {
        buf.viewport_offset_y = buf
            .cursor_y
            .saturating_sub(available_height.saturating_sub(1));
    }
}

pub fn handle_ctrl_move(app: &mut AppState, code: KeyCode) {
    let buf = app.current_buffer_mut();
    match code {
        KeyCode::Up => {
            buf.cursor_y = 0;
            buf.cursor_x = 0;
        }
        KeyCode::Down => {
            buf.cursor_y = buf.lines.len().saturating_sub(1);
            buf.cursor_x = buf.lines[buf.cursor_y].len();
        }
        _ => {}
    }
}

pub fn insert_tab(app: &mut AppState, tab_size: usize) {
    let buf = app.current_buffer_mut();
    let spaces = " ".repeat(tab_size);
    buf.lines[buf.cursor_y].insert_str(buf.cursor_x, &spaces);
    buf.cursor_x += tab_size;
    buf.modified = true;
}

pub fn insert_char(app: &mut AppState, c: char) {
    let buf = app.current_buffer_mut();
    buf.lines[buf.cursor_y].insert(buf.cursor_x, c);
    buf.cursor_x += 1;
    buf.modified = true;
}

pub fn insert_newline(app: &mut AppState) {
    let buf = app.current_buffer_mut();
    let remaining = buf.lines[buf.cursor_y].split_off(buf.cursor_x);
    buf.lines.insert(buf.cursor_y + 1, remaining);
    buf.cursor_y += 1;
    buf.cursor_x = 0;
    buf.modified = true;
}

pub fn backspace(app: &mut AppState) {
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
}

pub fn move_cursor(app: &mut AppState, code: KeyCode) {
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
        KeyCode::Right if buf.cursor_x < buf.lines[buf.cursor_y].len() => buf.cursor_x += 1,
        _ => {}
    }
}

pub fn apply_selection_move(app: &mut AppState, shift: bool, old_x: usize, old_y: usize) {
    if shift && app.selection.is_none() {
        app.selection = Some(Selection::new(old_x, old_y));
    }
}

pub fn finalize_selection_move(app: &mut AppState, shift: bool) {
    let new_x = app.current_buffer().cursor_x;
    let new_y = app.current_buffer().cursor_y;

    if shift {
        if let Some(sel) = &mut app.selection {
            sel.end_x = new_x;
            sel.end_y = new_y;
        }
    } else {
        app.selection = None;
    }
}

pub fn clear_selection(app: &mut AppState) {
    app.selection = None;
}

pub fn select_all(app: &mut AppState) {
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

pub fn paste_clipboard(app: &mut AppState) {
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

pub fn undo(app: &mut AppState) {
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

pub fn redo(app: &mut AppState) {
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

pub fn perform_keybind_action(app: &mut AppState, action: KeybindAction, mode: &mut Mode) {
    match action {
        KeybindAction::Menu => *mode = Mode::Menu,
        KeybindAction::Save => {
            let filename = app.current_buffer().filename.clone();
            let lines = app.current_buffer().lines.clone();
            if let Err(e) = io::save_to_file(&lines, &filename) {
                app.flash_status(format!("SAVE FAILED: {}", e));
            } else {
                app.current_buffer_mut().modified = false;
                app.flash_status("SAVED".to_string());
            }
        }
        KeybindAction::Undo => undo(app),
        KeybindAction::Redo => redo(app),
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
        KeybindAction::SelectAll => select_all(app),
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
                paste_clipboard(app);
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

pub fn extract_selected_text(buf: &Buffer, sx: usize, sy: usize, ex: usize, ey: usize) -> String {
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

pub fn delete_selection(buf: &mut Buffer, sx: usize, sy: usize, ex: usize, ey: usize) {
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

fn save_keybind_to_config(config: &mut Config, combo: &KeyCombo, action: KeybindAction) {
    let combo_str = combo.to_string();
    let action_str = action.to_str().to_string();

    config.custom_keybinds.retain(|(k, _)| k != &combo_str);
    config.custom_keybinds.push((combo_str, action_str));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_character_at_cursor() {
        let mut app = AppState::new();
        insert_char(&mut app, 'x');

        let buf = app.current_buffer();
        assert_eq!(buf.lines[0], "x");
        assert_eq!(buf.cursor_x, 1);
    }

    #[test]
    fn inserts_tab_as_spaces() {
        let mut app = AppState::new();
        insert_tab(&mut app, 2);

        let buf = app.current_buffer();
        assert_eq!(buf.lines[0], "  ");
        assert_eq!(buf.cursor_x, 2);
    }

    #[test]
    fn splits_line_on_enter() {
        let mut app = AppState::new();
        app.current_buffer_mut().lines[0] = "hello".to_string();
        app.current_buffer_mut().cursor_x = 2;
        insert_newline(&mut app);

        let buf = app.current_buffer();
        assert_eq!(buf.lines, vec!["he".to_string(), "llo".to_string()]);
        assert_eq!(buf.cursor_y, 1);
        assert_eq!(buf.cursor_x, 0);
    }

    #[test]
    fn backspace_merges_lines() {
        let mut app = AppState::new();
        app.current_buffer_mut().lines = vec!["one".to_string(), "two".to_string()];
        app.current_buffer_mut().cursor_y = 1;
        app.current_buffer_mut().cursor_x = 0;
        backspace(&mut app);

        let buf = app.current_buffer();
        assert_eq!(buf.lines, vec!["onetwo".to_string()]);
        assert_eq!(buf.cursor_y, 0);
        assert_eq!(buf.cursor_x, 3);
    }

    #[test]
    fn deletes_selection_range() {
        let mut app = AppState::new();
        app.current_buffer_mut().lines[0] = "hello world".to_string();
        app.selection = Some(Selection {
            start_x: 0,
            start_y: 0,
            end_x: 5,
            end_y: 0,
        });
        delete_selection(app.current_buffer_mut(), 0, 0, 5, 0);

        let buf = app.current_buffer();
        assert_eq!(buf.lines[0], " world");
    }

    #[test]
    fn undo_restores_previous_state() {
        let mut app = AppState::new();
        app.push_undo();
        insert_char(&mut app, 'a');
        undo(&mut app);

        let buf = app.current_buffer();
        assert_eq!(buf.lines[0], "");
    }

    #[test]
    fn redo_restores_after_undo() {
        let mut app = AppState::new();
        app.push_undo();
        insert_char(&mut app, 'a');
        undo(&mut app);
        redo(&mut app);

        let buf = app.current_buffer();
        assert_eq!(buf.lines[0], "a");
    }

    #[test]
    fn select_all_marks_entire_buffer() {
        let mut app = AppState::new();
        app.current_buffer_mut().lines = vec!["one".to_string(), "two".to_string()];
        select_all(&mut app);

        let selection = app.selection.expect("selection");
        assert_eq!(selection.start_x, 0);
        assert_eq!(selection.start_y, 0);
        assert_eq!(selection.end_y, 1);
        assert_eq!(selection.end_x, 3);
    }
}
