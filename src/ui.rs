use crate::state::{AppState, Config, MenuTab, Mode, Palette, PromptType, APP_NAME};
use chrono::Local;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute, queue,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{size, Clear, ClearType},
};
use std::collections::HashSet;
use std::io::{Stdout, Write};
use std::sync::LazyLock;

static RUST_KEYWORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut set = HashSet::new();
    set.extend([
        "fn", "let", "mut", "pub", "crate", "mod", "use", "impl", "trait", "struct", "enum",
        "type", "const", "static", "async", "await", "return", "if", "else", "match", "loop",
        "while", "for", "in", "break", "continue", "self", "Self", "super", "as", "true", "false",
        "None", "Some", "Ok", "Err",
    ]);
    set
}); 

static PYTHON_KEYWORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut set = HashSet::new();
    set.extend([
        "def", "class", "import", "from", "as", "return", "if", "elif", "else", "for", "while",
        "in", "and", "or", "not", "True", "False", "None", "lambda", "with", "try", "except",
        "finally", "raise", "pass", "async", "await",
    ]);
    set
});

static BASH_KEYWORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut set = HashSet::new();
    set.extend([
        "if", "then", "else", "fi", "for", "in", "do", "done", "while", "until", "case", "esac",
        "function",
    ]);
    set
});

pub fn redraw_all(
    stdout: &mut Stdout,
    mode: Mode,
    config: &Config,
    state: &AppState,
    active_tab: MenuTab,
    dropdown_idx: usize,
) -> std::io::Result<()> {
    let palette = state.current_palette;
    execute!(stdout, Hide)?;

    let (term_w, term_h) = size().unwrap_or((80, 24));

    let header_height = if config.show_header { 1 } else { 0 };
    let tab_bar_height = if config.show_tab_bar && state.buffers.len() > 1 {
        1
    } else {
        0
    };
    let menu_height = if mode == Mode::Menu { 1 } else { 0 };
    let status_height = if config.show_status_bar { 1 } else { 0 };

    let editor_start_y = header_height + tab_bar_height + menu_height;
    let editor_height = term_h.saturating_sub(editor_start_y + status_height);

    queue!(
        stdout,
        SetBackgroundColor(palette.bg),
        Clear(ClearType::Purge),
        MoveTo(0, 0)
    )?;

    if config.show_header {
        draw_header(stdout, term_w, state, palette)?;
    }

    if config.show_tab_bar && state.buffers.len() > 1 {
        draw_tab_bar(stdout, header_height, term_w, state, palette)?;
    }

    let buf = state.current_buffer();
    let gutter_width = if config.show_line_numbers {
        let max_lines = buf.lines.len().max(1);
        (max_lines.to_string().len() + 2) as u16
    } else {
        0
    };

    let viewport_offset_y = buf.viewport_offset_y;
    let viewport_offset_x = buf.viewport_offset_x;
    let editor_width = term_w.saturating_sub(gutter_width) as usize;

    for i in 0..editor_height {
        let screen_y = editor_start_y + i;
        let line_idx = viewport_offset_y + i as usize;

        queue!(
            stdout,
            MoveTo(0, screen_y),
            SetBackgroundColor(palette.bg),
            Clear(ClearType::UntilNewLine)
        )?;

        if line_idx < buf.lines.len() {
            if config.show_line_numbers {
                let line_num = line_idx + 1;
                let num_str = format!(
                    " {:>width$} ",
                    line_num,
                    width = (gutter_width - 2) as usize
                );
                queue!(stdout, SetForegroundColor(palette.accent), Print(num_str))?;
            }

            draw_line_with_selection(
                stdout,
                state,
                config,
                line_idx,
                viewport_offset_x,
                editor_width,
                palette,
            )?;
        }
    }

    if mode == Mode::Explorer {
        draw_explorer(stdout, state, editor_start_y, editor_height, palette)?;
    }
    if mode == Mode::Help {
        draw_help_overlay(stdout, term_w, term_h, state, palette)?;
    }
    if mode == Mode::Settings {
        draw_settings_overlay(stdout, term_w, term_h, config, state.settings_idx, palette)?;
    }
    if mode == Mode::ColorEditor {
        draw_color_editor_overlay(stdout, term_w, term_h, state, palette)?;
    }
    if mode == Mode::KeyRebind {
        draw_key_rebind_overlay(stdout, term_w, term_h, state, palette)?;
    }
    if mode == Mode::ConfirmWipe {
        draw_confirm_wipe(stdout, term_w, term_h, palette)?;
    }
    if state.input_mode {
        draw_input_prompt(stdout, term_w, term_h, state, palette)?;
    }
    if state.confirm_mode.is_some() {
        draw_confirm_close_tab(stdout, term_w, term_h, state, palette)?;
    }

    if mode == Mode::Menu {
        draw_menu_bar(stdout, active_tab, header_height + tab_bar_height, palette)?;
        draw_current_dropdown(
            stdout,
            active_tab,
            dropdown_idx,
            header_height + tab_bar_height + 1,
            palette,
        )?;
    }

    if config.show_status_bar {
        draw_status_bar(stdout, term_w, term_h, mode, state, config, palette)?;
    }

    if mode == Mode::Editing && !state.input_mode {
        let cursor_x = gutter_width + (buf.cursor_x.saturating_sub(viewport_offset_x)) as u16;
        let cursor_y = editor_start_y + (buf.cursor_y.saturating_sub(viewport_offset_y)) as u16;

        if cursor_y < term_h.saturating_sub(status_height) {
            queue!(stdout, MoveTo(cursor_x, cursor_y))?;
        }
    }

    execute!(stdout, Show)?;
    stdout.flush()
}

fn draw_line_with_selection(
    stdout: &mut Stdout,
    state: &AppState,
    config: &Config,
    line_idx: usize,
    viewport_offset_x: usize,
    editor_width: usize,
    palette: Palette,
) -> std::io::Result<()> {
    let buf = state.current_buffer();
    let line = &buf.lines[line_idx];
    let file_ext = buf.filename.rsplit('.').next().unwrap_or("");

    let keywords = if config.syntax_highlight {
        match file_ext {
            "rs" => Some(&*RUST_KEYWORDS),
            "py" => Some(&*PYTHON_KEYWORDS),
            "sh" | "bash" => Some(&*BASH_KEYWORDS),
            _ => None,
        }
    } else {
        None
    };

    let start = viewport_offset_x.min(line.len());
    let end = (start + editor_width).min(line.len());

    if let Some(sel) = &state.selection {
        let (sx, sy, ex, ey) = sel.normalized();
        if line_idx >= sy && line_idx <= ey {
            let sel_start = if line_idx == sy { sx } else { 0 };
            let sel_end = if line_idx == ey { ex } else { line.len() };

            let overlap_start = sel_start.max(start);
            let overlap_end = sel_end.min(end);

            if overlap_start < overlap_end {
                if start < overlap_start {
                    render_text(
                        stdout,
                        &line[start..overlap_start],
                        palette.text,
                        palette.bg,
                        keywords,
                        &palette,
                    )?;
                }

                render_text(
                    stdout,
                    &line[overlap_start..overlap_end],
                    palette.bg,
                    palette.highlight,
                    None,
                    &palette,
                )?;

                if overlap_end < end {
                    render_text(
                        stdout,
                        &line[overlap_end..end],
                        palette.text,
                        palette.bg,
                        keywords,
                        &palette,
                    )?;
                }

                return Ok(());
            }
        }
    }

    if start < end {
        render_text(
            stdout,
            &line[start..end],
            palette.text,
            palette.bg,
            keywords,
            &palette,
        )
    } else {
        Ok(())
    }
}

fn render_text(
    stdout: &mut Stdout,
    text: &str,
    fg: Color,
    bg: Color,
    keywords: Option<&HashSet<&'static str>>,
    palette: &Palette,
) -> std::io::Result<()> {
    if text.is_empty() {
        return Ok(());
    }

    if keywords.is_none() {
        queue!(
            stdout,
            SetForegroundColor(fg),
            SetBackgroundColor(bg),
            Print(text)
        )?;
        return Ok(());
    }

    let keywords = keywords.unwrap();
    let keyword_color = palette.primary;

    let mut last_end = 0;
    for (start, end) in word_boundaries(text) {
        if start > last_end {
            queue!(
                stdout,
                SetForegroundColor(fg),
                SetBackgroundColor(bg),
                Print(&text[last_end..start])
            )?;
        }

        let word = &text[start..end];
        let color = if keywords.contains(word) {
            keyword_color
        } else {
            fg
        };

        queue!(
            stdout,
            SetForegroundColor(color),
            SetBackgroundColor(bg),
            Print(word)
        )?;

        last_end = end;
    }

    if last_end < text.len() {
        queue!(
            stdout,
            SetForegroundColor(fg),
            SetBackgroundColor(bg),
            Print(&text[last_end..])
        )?;
    }

    Ok(())
}

fn word_boundaries(s: &str) -> Vec<(usize, usize)> {
    let mut bounds = Vec::new();
    let mut start = None;

    for (i, c) in s.char_indices() {
        if c.is_alphanumeric() || c == '_' {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(st) = start {
            bounds.push((st, i));
            start = None;
        }
    }

    if let Some(st) = start {
        bounds.push((st, s.len()));
    }

    bounds
}

fn draw_tab_bar(
    stdout: &mut Stdout,
    y: u16,
    w: u16,
    state: &AppState,
    palette: Palette,
) -> std::io::Result<()> {
    queue!(
        stdout,
        MoveTo(0, y),
        SetBackgroundColor(palette.panel),
        Clear(ClearType::UntilNewLine),
        Print(" ".repeat(w as usize))
    )?;

    let mut x = 2u16;
    let active_idx = state.active_buffer;

    for (i, buf) in state.buffers.iter().enumerate() {
        let marker = if buf.modified { "● " } else { "  " };
        let name = format!("{}{}", marker, buf.filename);
        let tab_len = name.len() as u16 + 2;

        if x + tab_len > w.saturating_sub(5) {
            queue!(
                stdout,
                MoveTo(x, y),
                SetForegroundColor(palette.accent),
                Print("...")
            )?;
            break;
        }

        queue!(stdout, MoveTo(x, y))?;

        if i == active_idx {
            queue!(
                stdout,
                SetBackgroundColor(palette.accent),
                SetForegroundColor(palette.bg),
                Print(format!(" {} ", name))
            )?;
        } else {
            queue!(
                stdout,
                SetBackgroundColor(palette.panel),
                SetForegroundColor(palette.text),
                Print(format!(" {} ", name))
            )?;
        }

        x += tab_len + 1;
    }

    Ok(())
}

fn draw_header(
    stdout: &mut Stdout,
    w: u16,
    state: &AppState,
    palette: Palette,
) -> std::io::Result<()> {
    let buf = state.current_buffer();
    let filename = &buf.filename;

    queue!(
        stdout,
        MoveTo(0, 0),
        SetBackgroundColor(palette.panel),
        Clear(ClearType::UntilNewLine),
        Print(" ".repeat(w as usize))
    )?;

    queue!(
        stdout,
        MoveTo(2, 0),
        SetForegroundColor(palette.primary),
        Print(format!("{} ", APP_NAME)),
        SetForegroundColor(palette.highlight),
        Print(format!("v0.2.0 — {}", filename))
    )?;

    let time = Local::now().format("%H:%M").to_string();
    queue!(
        stdout,
        MoveTo(w.saturating_sub(time.len() as u16 + 2), 0),
        SetForegroundColor(palette.accent),
        Print(time)
    )?;

    Ok(())
}

pub fn draw_menu_bar(
    stdout: &mut Stdout,
    active_tab: MenuTab,
    y: u16,
    palette: Palette,
) -> std::io::Result<()> {
    let (term_w, _) = size().unwrap_or((80, 24));

    queue!(
        stdout,
        MoveTo(0, y),
        SetBackgroundColor(palette.accent),
        Clear(ClearType::UntilNewLine),
        Print(" ".repeat(term_w as usize))
    )?;

    let tabs = [
        (" FERO ", MenuTab::Re),
        (" FILE ", MenuTab::File),
        (" EDIT ", MenuTab::Edit),
        (" VIEW ", MenuTab::View),
    ];

    let mut x = 2u16;
    for &(name, tab) in &tabs {
        queue!(stdout, MoveTo(x, y))?;

        if tab == active_tab {
            queue!(
                stdout,
                SetBackgroundColor(palette.primary),
                SetForegroundColor(palette.bg),
                Print(name)
            )?;
        } else {
            queue!(
                stdout,
                SetBackgroundColor(palette.accent),
                SetForegroundColor(palette.text),
                Print(name)
            )?;
        }

        x += name.len() as u16 + 2;
    }

    Ok(())
}

pub fn draw_current_dropdown(
    stdout: &mut Stdout,
    active: MenuTab,
    idx: usize,
    y_offset: u16,
    palette: Palette,
) -> std::io::Result<()> {
    let (items, x_off, max_item_len) = match active {
        MenuTab::Re => (vec![" Config ", " Help ", " Exit ", " Save & Exit "], 2, 14),
        MenuTab::File => (
            vec![
                " New Tab ",
                " Open ",
                " Close Tab ",
                " Next Tab ",
                " Prev Tab ",
                " Save As ",
            ],
            10,
            14,
        ),
        MenuTab::Edit => (
            vec![" Find ", " Replace ", " Go To Line ", " Wipe Buffer "],
            18,
            15,
        ),
        MenuTab::View => (
            vec![" Header ", " Status ", " Lines ", " Tabs ", " Syntax "],
            26,
            12,
        ),
    };

    let dropdown_height = items.len() as u16 + 2;
    let box_width = max_item_len + 4;

    for dy in 0..dropdown_height {
        queue!(
            stdout,
            MoveTo(x_off, y_offset + dy),
            SetBackgroundColor(palette.panel),
            Print(" ".repeat(box_width as usize))
        )?;
    }

    let active_idx = idx % items.len();

    for (i, item) in items.iter().enumerate() {
        let y = y_offset + 1 + i as u16;
        queue!(stdout, MoveTo(x_off + 2, y))?;

        if i == active_idx {
            queue!(
                stdout,
                SetBackgroundColor(palette.primary),
                SetForegroundColor(palette.bg),
                Print(item)
            )?;
        } else {
            queue!(
                stdout,
                SetBackgroundColor(palette.panel),
                SetForegroundColor(palette.text),
                Print(item)
            )?;
        }
    }

    Ok(())
}

fn draw_input_prompt(
    stdout: &mut Stdout,
    w: u16,
    h: u16,
    state: &AppState,
    palette: Palette,
) -> std::io::Result<()> {
    let title = match state.prompt_type {
        PromptType::SaveAs => "SAVE AS",
        PromptType::Find => "FIND TEXT",
        PromptType::GoToLine => "GO TO LINE",
        _ => "INPUT",
    };

    let box_w = 50;
    let box_h = 7;
    let start_x = (w.saturating_sub(box_w)) / 2;
    let start_y = (h.saturating_sub(box_h)) / 2;

    for i in 0..box_h {
        queue!(
            stdout,
            MoveTo(start_x, start_y + i),
            SetBackgroundColor(palette.panel),
            Print(" ".repeat(box_w as usize))
        )?;
    }

    queue!(
        stdout,
        MoveTo(start_x + 2, start_y + 1),
        SetForegroundColor(palette.primary),
        Print(title)
    )?;

    if state.prompt_type == PromptType::SaveAs {
        let path_display = if state.input_buffer.is_empty() {
            format!("{}", state.current_dir.display())
        } else {
            let p = state.current_dir.join(&state.input_buffer);
            format!("{}", p.display())
        };

        queue!(
            stdout,
            MoveTo(start_x + 2, start_y + 2),
            SetForegroundColor(palette.accent),
            Print(format!("Path: {}", path_display))
        )?;
    }

    queue!(
        stdout,
        MoveTo(start_x + 2, start_y + 3),
        SetForegroundColor(palette.text),
        SetBackgroundColor(palette.panel),
        Clear(ClearType::UntilNewLine),
        Print("> "),
        Print(&state.input_buffer),
    )?;

    queue!(
        stdout,
        MoveTo(start_x + 2, start_y + 5),
        SetForegroundColor(palette.accent),
        Print("[Enter confirm • Esc cancel]")
    )?;

    Ok(())
}

fn draw_status_bar(
    stdout: &mut Stdout,
    w: u16,
    h: u16,
    mode: Mode,
    state: &AppState,
    config: &Config,
    palette: Palette,
) -> std::io::Result<()> {
    let y = h.saturating_sub(1);
    let buf = state.current_buffer();

    queue!(
        stdout,
        MoveTo(0, y),
        SetBackgroundColor(palette.panel),
        Clear(ClearType::UntilNewLine),
        Print(" ".repeat(w as usize))
    )?;

    let mode_str = if let Some(flash) = &state.status_flash {
        format!(" {} ", flash)
    } else {
        format!(" {} ", format!("{:?}", mode).to_uppercase())
    };

    let pos_str = format!(" L{},C{} ", buf.cursor_y + 1, buf.cursor_x + 1);
    let modified = if buf.modified { " ●" } else { "" };
    let auto_save = if config.auto_save { " AS" } else { "" };
    let undo_redo = format!(" U:{} R:{}", state.undo_stack.len(), state.redo_stack.len());

    let right_str = format!("{}{}{}{}", pos_str, auto_save, modified, undo_redo);
    let right_len = right_str.len() as u16;

    queue!(
        stdout,
        MoveTo(2, y),
        SetForegroundColor(if state.status_flash.is_some() {
            palette.warning
        } else {
            palette.primary
        }),
        Print(&mode_str)
    )?;

    queue!(
        stdout,
        MoveTo(w.saturating_sub(right_len + 2), y),
        SetForegroundColor(palette.text),
        Print(&right_str)
    )?;

    Ok(())
}

fn draw_help_overlay(
    stdout: &mut Stdout,
    w: u16,
    h: u16,
    state: &AppState,
    palette: Palette,
) -> std::io::Result<()> {
    let box_w = 60;
    let box_h = 22;
    let x = (w.saturating_sub(box_w)) / 2;
    let y = (h.saturating_sub(box_h)) / 2;

    for i in 0..box_h {
        queue!(
            stdout,
            MoveTo(x, y + i),
            SetBackgroundColor(palette.panel),
            Print(" ".repeat(box_w as usize))
        )?;
    }

    queue!(
        stdout,
        MoveTo(x + 2, y + 1),
        SetForegroundColor(palette.primary),
        Print("FERO HELP - KEYBINDINGS")
    )?;

    let bindings = [
        ("Esc", "Menu / Close overlay"),
        ("Arrows", "Move cursor"),
        ("Shift+Arrows", "Select text"),
        ("Ctrl+A", "Select all"),
        ("Ctrl+C/X/V", "Copy / Cut / Paste"),
        ("Enter", "New line"),
        ("Tab", "Indent"),
        ("Ctrl+Tab", "Switch tab"),
        ("Ctrl+S", "Save"),
        ("Ctrl+Z/Y", "Undo / Redo"),
        ("Ctrl+Up/Down", "Jump top/bottom"),
    ];

    for (i, (key, desc)) in bindings.iter().enumerate() {
        queue!(stdout, MoveTo(x + 3, y + 3 + i as u16))?;
        queue!(stdout, SetForegroundColor(palette.accent), Print(key))?;
        queue!(stdout, MoveTo(x + 18, y + 3 + i as u16))?;
        queue!(stdout, SetForegroundColor(palette.text), Print(desc))?;
    }

    if !state.keybind_state.custom_binds.is_empty() {
        queue!(stdout, MoveTo(x + 3, y + 16))?;
        queue!(
            stdout,
            SetForegroundColor(palette.warning),
            Print("CUSTOM BINDS")
        )?;

        let mut y_line = y + 17;
        for (combo, action) in state.keybind_state.custom_binds.iter().take(3) {
            let key_str = combo.to_string();
            queue!(stdout, MoveTo(x + 5, y_line))?;
            queue!(
                stdout,
                SetForegroundColor(palette.highlight),
                Print(&key_str)
            )?;
            queue!(stdout, MoveTo(x + 25, y_line))?;
            queue!(
                stdout,
                SetForegroundColor(palette.text),
                Print(format!("{:?}", action))
            )?;
            y_line += 1;
        }
    }

    queue!(
        stdout,
        MoveTo(x + 2, y + box_h - 1),
        SetForegroundColor(palette.accent),
        Print("Press Esc or Enter to close")
    )?;

    Ok(())
}

fn draw_settings_overlay(
    stdout: &mut Stdout,
    w: u16,
    h: u16,
    config: &Config,
    idx: usize,
    palette: Palette,
) -> std::io::Result<()> {
    let box_w = 44;
    let box_h = 11;
    let x = (w.saturating_sub(box_w)) / 2;
    let y = (h.saturating_sub(box_h)) / 2;

    for i in 0..box_h {
        queue!(
            stdout,
            MoveTo(x, y + i),
            SetBackgroundColor(palette.panel),
            Print(" ".repeat(box_w as usize))
        )?;
    }

    queue!(
        stdout,
        MoveTo(x + 2, y + 1),
        SetForegroundColor(palette.primary),
        Print("SETTINGS")
    )?;

    let options = [
        format!("Auto Save: {}", if config.auto_save { "ON" } else { "OFF" }),
        format!("Tab Size: {}", config.tab_size),
        "Edit Colors".to_string(),
        "Rebind Keys".to_string(),
        "Close Settings".to_string(),
    ];

    for (i, opt) in options.iter().enumerate() {
        let y_pos = y + 3 + i as u16;
        queue!(stdout, MoveTo(x + 3, y_pos))?;

        if i == idx {
            queue!(
                stdout,
                SetBackgroundColor(palette.accent),
                SetForegroundColor(palette.highlight),
                Print(format!(" > {}", opt))
            )?;
        } else {
            queue!(
                stdout,
                SetBackgroundColor(palette.panel),
                SetForegroundColor(palette.text),
                Print(format!("   {}", opt))
            )?;
        }
    }

    Ok(())
}

fn draw_key_rebind_overlay(
    stdout: &mut Stdout,
    w: u16,
    h: u16,
    state: &AppState,
    palette: Palette,
) -> std::io::Result<()> {
    let box_w = 52;
    let box_h = 20;
    let x = (w.saturating_sub(box_w)) / 2;
    let y = (h.saturating_sub(box_h)) / 2;

    for i in 0..box_h {
        queue!(
            stdout,
            MoveTo(x, y + i),
            SetBackgroundColor(palette.panel),
            Print(" ".repeat(box_w as usize))
        )?;
    }

    queue!(
        stdout,
        MoveTo(x + 2, y + 1),
        SetForegroundColor(palette.primary),
        Print("KEY REBINDING")
    )?;

    let actions = [
        "Open Menu",
        "Save",
        "Undo",
        "Redo",
        "New Tab",
        "Close Tab",
        "Next Tab",
        "Prev Tab",
        "Select All",
        "Copy",
        "Cut",
        "Paste",
        "Find",
        "Go To Line",
        "Wipe Buffer",
        "Reset to Default",
    ];

    let kb = &state.keybind_state;
    let visible_lines = 14;
    let start = kb.scroll_offset;
    let end = (start + visible_lines).min(actions.len());

    for (i, action) in actions[start..end].iter().enumerate() {
        let global_i = start + i;
        let y_pos = y + 3 + i as u16;

        queue!(stdout, MoveTo(x + 2, y_pos))?;

        if global_i == kb.selected_action && !kb.waiting_for_key && !kb.confirming_reset {
            queue!(
                stdout,
                SetBackgroundColor(palette.accent),
                SetForegroundColor(palette.highlight),
                Print("▶ ")
            )?;
        } else {
            queue!(stdout, Print("  "))?;
        }

        queue!(
            stdout,
            SetBackgroundColor(palette.panel),
            SetForegroundColor(palette.text),
            Print(action)
        )?;
    }

    let bottom_y = y + 18;
    queue!(stdout, MoveTo(x + 2, bottom_y))?;

    if kb.confirming_reset {
        queue!(
            stdout,
            SetForegroundColor(palette.warning),
            Print("RESET ALL? Press Y to confirm")
        )?;
    } else if kb.waiting_for_key {
        queue!(
            stdout,
            SetForegroundColor(palette.warning),
            Print("PRESS NEW KEY • Esc to cancel")
        )?;
    } else {
        queue!(
            stdout,
            SetForegroundColor(palette.accent),
            Print("↑↓ navigate • Enter rebind • Esc exit")
        )?;
    }

    Ok(())
}

fn draw_confirm_wipe(stdout: &mut Stdout, w: u16, h: u16, palette: Palette) -> std::io::Result<()> {
    let box_w = 50;
    let box_h = 9;
    let x = (w.saturating_sub(box_w)) / 2;
    let y = (h.saturating_sub(box_h)) / 2;

    for i in 0..box_h {
        queue!(
            stdout,
            MoveTo(x, y + i),
            SetBackgroundColor(palette.panel),
            Print(" ".repeat(box_w as usize))
        )?;
    }

    queue!(
        stdout,
        MoveTo(x + 2, y + 1),
        SetForegroundColor(palette.warning),
        Print("WARNING: IRREVERSIBLE ACTION")
    )?;

    queue!(
        stdout,
        MoveTo(x + 2, y + 3),
        SetForegroundColor(palette.text),
        Print("This will permanently delete all text.")
    )?;

    queue!(
        stdout,
        MoveTo(x + 2, y + 5),
        SetForegroundColor(palette.primary),
        Print("Press Y to confirm, any other key to cancel")
    )?;

    Ok(())
}

fn draw_explorer(
    stdout: &mut Stdout,
    state: &AppState,
    y_start: u16,
    height: u16,
    palette: Palette,
) -> std::io::Result<()> {
    let width = 40;

    queue!(
        stdout,
        MoveTo(0, y_start),
        SetBackgroundColor(palette.panel),
        SetForegroundColor(palette.primary),
        Print(" FILE EXPLORER ")
    )?;

    for i in 1..height {
        queue!(
            stdout,
            MoveTo(0, y_start + i),
            SetBackgroundColor(palette.panel),
            Print(" ".repeat(width as usize))
        )?;

        let file_idx = state.explorer_offset + (i as usize - 1);
        if file_idx < state.explorer_files.len() {
            let name = &state.explorer_files[file_idx];
            queue!(stdout, MoveTo(2, y_start + i))?;

            if file_idx == state.explorer_idx {
                queue!(
                    stdout,
                    SetBackgroundColor(palette.accent),
                    SetForegroundColor(palette.highlight),
                    Print(format!("> {}", name))
                )?;
            } else {
                queue!(
                    stdout,
                    SetBackgroundColor(palette.panel),
                    SetForegroundColor(palette.text),
                    Print(format!("  {}", name))
                )?;
            }
        }
    }

    Ok(())
}

fn draw_color_editor_overlay(
    stdout: &mut Stdout,
    w: u16,
    h: u16,
    state: &AppState,
    palette: Palette,
) -> std::io::Result<()> {
    let box_w = 64;
    let box_h = 16;
    let start_x = (w.saturating_sub(box_w)) / 2;
    let start_y = (h.saturating_sub(box_h)) / 2;

    for i in 0..box_h {
        queue!(
            stdout,
            MoveTo(start_x, start_y + i),
            SetBackgroundColor(palette.panel),
            Print(" ".repeat(box_w as usize))
        )?;
    }

    queue!(
        stdout,
        MoveTo(start_x + 2, start_y + 1),
        SetForegroundColor(palette.primary),
        Print("COLOR THEME EDITOR")
    )?;

    queue!(
        stdout,
        MoveTo(start_x + 2, start_y + 2),
        SetForegroundColor(palette.accent),
        Print("↑↓ navigate • Enter edit • Ctrl+S save • Esc exit")
    )?;

    for (i, entry) in state.color_entries.iter().enumerate() {
        let y = start_y + 4 + i as u16;
        queue!(stdout, MoveTo(start_x + 3, y))?;

        let prefix = if i == state.color_editor_idx {
            "▶ "
        } else {
            "  "
        };
        let name_fg = if i == state.color_editor_idx {
            palette.highlight
        } else {
            palette.text
        };
        let hex_fg = if i == state.color_editor_idx && state.editing_hex {
            palette.primary
        } else {
            palette.text
        };

        queue!(
            stdout,
            SetForegroundColor(name_fg),
            Print(format!("{}{:9}", prefix, entry.name))
        )?;
        queue!(
            stdout,
            MoveTo(start_x + 15, y),
            SetForegroundColor(hex_fg),
            Print(&entry.current_hex)
        )?;
    }

    let input_y = if state.prompt_type == PromptType::SaveAs {
        start_y + 3
    } else {
        start_y + 3
    };
    queue!(
        stdout,
        MoveTo(start_x + 2, input_y),
        SetForegroundColor(palette.text),
        Print(format!("> {}", state.input_buffer))
    )?;

    Ok(())
}

fn draw_confirm_close_tab(
    stdout: &mut Stdout,
    w: u16,
    h: u16,
    state: &AppState,
    palette: Palette,
) -> std::io::Result<()> {
    let box_w = 50;
    let box_h = 10;
    let x = (w.saturating_sub(box_w)) / 2;
    let y = (h.saturating_sub(box_h)) / 2;

    for i in 0..box_h {
        queue!(
            stdout,
            MoveTo(x, y + i),
            SetBackgroundColor(palette.panel),
            Print(" ".repeat(box_w as usize))
        )?;
    }

    queue!(
        stdout,
        MoveTo(x + 2, y + 1),
        SetForegroundColor(palette.warning),
        Print("UNSAVED CHANGES")
    )?;

    queue!(
        stdout,
        MoveTo(x + 2, y + 3),
        SetForegroundColor(palette.text),
        Print("Save before closing?")
    )?;

    let options = ["No (discard)", "Yes (save)", "Cancel"];
    for (i, opt) in options.iter().enumerate() {
        let selected = state.confirm_choice as usize == i;
        queue!(
            stdout,
            MoveTo(x + 8, y + 5 + i as u16),
            SetBackgroundColor(if selected {
                palette.accent
            } else {
                palette.panel
            }),
            SetForegroundColor(if selected {
                palette.highlight
            } else {
                palette.text
            }),
            Print(if selected { "▶ " } else { "  " }),
            Print(opt)
        )?;
    }

    queue!(
        stdout,
        MoveTo(x + 2, y + 8),
        SetForegroundColor(palette.accent),
        Print("↑↓ navigate • Enter confirm • Esc cancel")
    )?;

    Ok(())
}
