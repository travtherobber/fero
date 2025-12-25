use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

pub const APP_NAME: &str = "FERO";

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Editing,
    Menu,
    Explorer,
    Settings,
    Help,
    ColorEditor,
    ConfirmWipe,
    KeyRebind,
    Confirm(ConfirmType),
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MenuTab {
    Re,
    File,
    Edit,
    View,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum PromptType {
    SaveAs,
    Find,
    Replace,
    GoToLine,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ConfirmType {
    CloseTab,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ConfirmChoice {
    No,
    Yes,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeybindAction {
    Menu,
    Save,
    Undo,
    Redo,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    SelectAll,
    Copy,
    Cut,
    Paste,
    Find,
    GoToLine,
    WipeBuffer,
    ResetToDefault,
}

impl KeybindAction {
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(KeybindAction::Menu),
            1 => Some(KeybindAction::Save),
            2 => Some(KeybindAction::Undo),
            3 => Some(KeybindAction::Redo),
            4 => Some(KeybindAction::NewTab),
            5 => Some(KeybindAction::CloseTab),
            6 => Some(KeybindAction::NextTab),
            7 => Some(KeybindAction::PrevTab),
            8 => Some(KeybindAction::SelectAll),
            9 => Some(KeybindAction::Copy),
            10 => Some(KeybindAction::Cut),
            11 => Some(KeybindAction::Paste),
            12 => Some(KeybindAction::Find),
            13 => Some(KeybindAction::GoToLine),
            14 => Some(KeybindAction::WipeBuffer),
            15 => Some(KeybindAction::ResetToDefault),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeybindState {
    pub in_rebind_mode: bool,
    pub selected_action: usize,
    pub waiting_for_key: bool,
    pub pending_action: Option<usize>,
    pub custom_binds: HashMap<KeyCombo, KeybindAction>,
    pub scroll_offset: usize,
    pub confirming_reset: bool,
}

impl Default for KeybindState {
    fn default() -> Self {
        Self {
            in_rebind_mode: false,
            selected_action: 0,
            waiting_for_key: false,
            pending_action: None,
            custom_binds: HashMap::new(),
            scroll_offset: 0,
            confirming_reset: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub show_line_numbers: bool,
    pub show_status_bar: bool,
    pub show_header: bool,
    pub auto_save: bool,
    pub tab_size: usize,
    #[serde(default)]
    pub palette: PaletteConfig,
    #[serde(default = "default_true")]
    pub syntax_highlight: bool,
    #[serde(default = "default_true")]
    pub show_tab_bar: bool,
    #[serde(default)]
    pub custom_keybinds: Vec<(String, String)>,
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            show_line_numbers: true,
            show_status_bar: true,
            show_header: true,
            auto_save: false,
            tab_size: 4,
            palette: PaletteConfig::default(),
            syntax_highlight: true,
            show_tab_bar: true,
            custom_keybinds: Vec::new(),
        }
    }
}

impl std::fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}|{:?}", self.modifiers, self.code)
    }
}

impl KeyCombo {
    pub fn from_string(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.splitn(2, '|').collect();
        if parts.len() != 2 {
            return None;
        }
        let mods_str = parts[0];
        let code_str = parts[1];

        let mut modifiers = KeyModifiers::empty();
        if mods_str.contains("CONTROL") {
            modifiers |= KeyModifiers::CONTROL;
        }
        if mods_str.contains("SHIFT") {
            modifiers |= KeyModifiers::SHIFT;
        }
        if mods_str.contains("ALT") {
            modifiers |= KeyModifiers::ALT;
        }

        let code = if let Some(char_part) =
            code_str.strip_prefix("Char('").and_then(|s| s.strip_suffix("')"))
        {
            char_part.chars().next().map(KeyCode::Char)
        } else {
            match code_str {
                "Enter" => Some(KeyCode::Enter),
                "Backspace" => Some(KeyCode::Backspace),
                "Tab" => Some(KeyCode::Tab),
                "Left" => Some(KeyCode::Left),
                "Right" => Some(KeyCode::Right),
                "Up" => Some(KeyCode::Up),
                "Down" => Some(KeyCode::Down),
                "Delete" => Some(KeyCode::Delete),
                "Esc" => Some(KeyCode::Esc),
                _ => None,
            }
        };

        code.map(|c| KeyCombo {
            code: c,
            modifiers,
        })
    }
}

impl KeybindAction {
    pub fn to_str(&self) -> &'static str {
        match self {
            KeybindAction::Menu => "Menu",
            KeybindAction::Save => "Save",
            KeybindAction::Undo => "Undo",
            KeybindAction::Redo => "Redo",
            KeybindAction::NewTab => "NewTab",
            KeybindAction::CloseTab => "CloseTab",
            KeybindAction::NextTab => "NextTab",
            KeybindAction::PrevTab => "PrevTab",
            KeybindAction::SelectAll => "SelectAll",
            KeybindAction::Copy => "Copy",
            KeybindAction::Cut => "Cut",
            KeybindAction::Paste => "Paste",
            KeybindAction::Find => "Find",
            KeybindAction::GoToLine => "GoToLine",
            KeybindAction::WipeBuffer => "WipeBuffer",
            KeybindAction::ResetToDefault => "ResetToDefault",
        }
    }
}

impl FromStr for KeybindAction {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Menu" => Ok(KeybindAction::Menu),
            "Save" => Ok(KeybindAction::Save),
            "Undo" => Ok(KeybindAction::Undo),
            "Redo" => Ok(KeybindAction::Redo),
            "NewTab" => Ok(KeybindAction::NewTab),
            "CloseTab" => Ok(KeybindAction::CloseTab),
            "NextTab" => Ok(KeybindAction::NextTab),
            "PrevTab" => Ok(KeybindAction::PrevTab),
            "SelectAll" => Ok(KeybindAction::SelectAll),
            "Copy" => Ok(KeybindAction::Copy),
            "Cut" => Ok(KeybindAction::Cut),
            "Paste" => Ok(KeybindAction::Paste),
            "Find" => Ok(KeybindAction::Find),
            "GoToLine" => Ok(KeybindAction::GoToLine),
            "WipeBuffer" => Ok(KeybindAction::WipeBuffer),
            "ResetToDefault" => Ok(KeybindAction::ResetToDefault),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaletteConfig {
    pub ui_background: String,
    pub ui_foreground: String,
    pub ui_border: String,
    pub status_bar_bg: String,
    pub status_bar_fg: String,
    pub header_bg: String,
    pub header_fg: String,
    pub editor_background: String,
    pub editor_foreground: String,
    pub line_number_bg: String,
    pub line_number_fg: String,
    pub cursor: String,
    pub selection_bg: String,
    pub selection_fg: String,
    pub syntax_keyword: String,
    pub syntax_string: String,
    pub syntax_comment: String,
    pub syntax_function: String,
    pub syntax_type: String,
    pub syntax_constant: String,
    pub accent_primary: String,
    pub accent_secondary: String,
    pub match_highlight: String,
    pub error: String,
    pub warning: String,
}

impl Default for PaletteConfig {
    fn default() -> Self {
        Self {
            ui_background: "#1E1E1E".to_string(),
            ui_foreground: "#D4D4D4".to_string(),
            ui_border: "#3A3A3A".to_string(),
            status_bar_bg: "#007ACC".to_string(),
            status_bar_fg: "#FFFFFF".to_string(),
            header_bg: "#3C3C3C".to_string(),
            header_fg: "#FFFFFF".to_string(),
            editor_background: "#121212".to_string(),
            editor_foreground: "#D4D4D4".to_string(),
            line_number_bg: "#121212".to_string(),
            line_number_fg: "#858585".to_string(),
            cursor: "#FFFFFF".to_string(),
            selection_bg: "#264F78".to_string(),
            selection_fg: "#FFFFFF".to_string(),
            syntax_keyword: "#C586C0".to_string(),
            syntax_string: "#CE9178".to_string(),
            syntax_comment: "#6A9955".to_string(),
            syntax_function: "#DCDCAA".to_string(),
            syntax_type: "#4EC9B0".to_string(),
            syntax_constant: "#B5CEA8".to_string(),
            accent_primary: "#007ACC".to_string(),
            accent_secondary: "#4D4D4D".to_string(),
            match_highlight: "#515C6A".to_string(),
            error: "#F44747".to_string(),
            warning: "#FFD700".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub ui_background: Color,
    pub ui_foreground: Color,
    pub ui_border: Color,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub editor_background: Color,
    pub editor_foreground: Color,
    pub line_number_bg: Color,
    pub line_number_fg: Color,
    pub cursor: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub syntax_keyword: Color,
    pub syntax_string: Color,
    pub syntax_comment: Color,
    pub syntax_function: Color,
    pub syntax_type: Color,
    pub syntax_constant: Color,
    pub accent_primary: Color,
    pub accent_secondary: Color,
    pub match_highlight: Color,
    pub error: Color,
    pub warning: Color,
}

impl Palette {
    pub fn default() -> Self {
        Self::from_config(&PaletteConfig::default())
    }

    pub fn from_config(cfg: &PaletteConfig) -> Self {
        Self {
            ui_background: Self::hex_to_color(&cfg.ui_background),
            ui_foreground: Self::hex_to_color(&cfg.ui_foreground),
            ui_border: Self::hex_to_color(&cfg.ui_border),
            status_bar_bg: Self::hex_to_color(&cfg.status_bar_bg),
            status_bar_fg: Self::hex_to_color(&cfg.status_bar_fg),
            header_bg: Self::hex_to_color(&cfg.header_bg),
            header_fg: Self::hex_to_color(&cfg.header_fg),
            editor_background: Self::hex_to_color(&cfg.editor_background),
            editor_foreground: Self::hex_to_color(&cfg.editor_foreground),
            line_number_bg: Self::hex_to_color(&cfg.line_number_bg),
            line_number_fg: Self::hex_to_color(&cfg.line_number_fg),
            cursor: Self::hex_to_color(&cfg.cursor),
            selection_bg: Self::hex_to_color(&cfg.selection_bg),
            selection_fg: Self::hex_to_color(&cfg.selection_fg),
            syntax_keyword: Self::hex_to_color(&cfg.syntax_keyword),
            syntax_string: Self::hex_to_color(&cfg.syntax_string),
            syntax_comment: Self::hex_to_color(&cfg.syntax_comment),
            syntax_function: Self::hex_to_color(&cfg.syntax_function),
            syntax_type: Self::hex_to_color(&cfg.syntax_type),
            syntax_constant: Self::hex_to_color(&cfg.syntax_constant),
            accent_primary: Self::hex_to_color(&cfg.accent_primary),
            accent_secondary: Self::hex_to_color(&cfg.accent_secondary),
            match_highlight: Self::hex_to_color(&cfg.match_highlight),
            error: Self::hex_to_color(&cfg.error),
            warning: Self::hex_to_color(&cfg.warning),
        }
    }

    pub fn to_config(&self) -> PaletteConfig {
        PaletteConfig {
            ui_background: Self::to_hex(self.ui_background),
            ui_foreground: Self::to_hex(self.ui_foreground),
            ui_border: Self::to_hex(self.ui_border),
            status_bar_bg: Self::to_hex(self.status_bar_bg),
            status_bar_fg: Self::to_hex(self.status_bar_fg),
            header_bg: Self::to_hex(self.header_bg),
            header_fg: Self::to_hex(self.header_fg),
            editor_background: Self::to_hex(self.editor_background),
            editor_foreground: Self::to_hex(self.editor_foreground),
            line_number_bg: Self::to_hex(self.line_number_bg),
            line_number_fg: Self::to_hex(self.line_number_fg),
            cursor: Self::to_hex(self.cursor),
            selection_bg: Self::to_hex(self.selection_bg),
            selection_fg: Self::to_hex(self.selection_fg),
            syntax_keyword: Self::to_hex(self.syntax_keyword),
            syntax_string: Self::to_hex(self.syntax_string),
            syntax_comment: Self::to_hex(self.syntax_comment),
            syntax_function: Self::to_hex(self.syntax_function),
            syntax_type: Self::to_hex(self.syntax_type),
            syntax_constant: Self::to_hex(self.syntax_constant),
            accent_primary: Self::to_hex(self.accent_primary),
            accent_secondary: Self::to_hex(self.accent_secondary),
            match_highlight: Self::to_hex(self.match_highlight),
            error: Self::to_hex(self.error),
            warning: Self::to_hex(self.warning),
        }
    }

    fn hex_to_color(hex: &str) -> Color {
        if hex.len() == 7 && hex.starts_with('#') {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[1..3], 16),
                u8::from_str_radix(&hex[3..5], 16),
                u8::from_str_radix(&hex[5..7], 16),
            ) {
                return Color::Rgb { r, g, b };
            }
        }
        Color::Rgb { r: 0, g: 0, b: 0 }
    }

    pub fn to_hex(color: Color) -> String {
        if let Color::Rgb { r, g, b } = color {
            format!("#{:02X}{:02X}{:02X}", r, g, b)
        } else {
            "#000000".to_string()
        }
    }
}

#[derive(Clone)]
pub struct ColorEntry {
    pub name: String,
    pub current_hex: String,
}

impl AppState {
    pub fn populate_color_entries(&mut self) {
        let config = self.current_palette.to_config();
        self.color_entries = vec![
            ColorEntry { name: "ui_background".to_string(), current_hex: config.ui_background },
            ColorEntry { name: "ui_foreground".to_string(), current_hex: config.ui_foreground },
            ColorEntry { name: "ui_border".to_string(), current_hex: config.ui_border },
            ColorEntry { name: "status_bar_bg".to_string(), current_hex: config.status_bar_bg },
            ColorEntry { name: "status_bar_fg".to_string(), current_hex: config.status_bar_fg },
            ColorEntry { name: "header_bg".to_string(), current_hex: config.header_bg },
            ColorEntry { name: "header_fg".to_string(), current_hex: config.header_fg },
            ColorEntry { name: "editor_background".to_string(), current_hex: config.editor_background },
            ColorEntry { name: "editor_foreground".to_string(), current_hex: config.editor_foreground },
            ColorEntry { name: "line_number_bg".to_string(), current_hex: config.line_number_bg },
            ColorEntry { name: "line_number_fg".to_string(), current_hex: config.line_number_fg },
            ColorEntry { name: "cursor".to_string(), current_hex: config.cursor },
            ColorEntry { name: "selection_bg".to_string(), current_hex: config.selection_bg },
            ColorEntry { name: "selection_fg".to_string(), current_hex: config.selection_fg },
            ColorEntry { name: "syntax_keyword".to_string(), current_hex: config.syntax_keyword },
            ColorEntry { name: "syntax_string".to_string(), current_hex: config.syntax_string },
            ColorEntry { name: "syntax_comment".to_string(), current_hex: config.syntax_comment },
            ColorEntry { name: "syntax_function".to_string(), current_hex: config.syntax_function },
            ColorEntry { name: "syntax_type".to_string(), current_hex: config.syntax_type },
            ColorEntry { name: "syntax_constant".to_string(), current_hex: config.syntax_constant },
            ColorEntry { name: "accent_primary".to_string(), current_hex: config.accent_primary },
            ColorEntry { name: "accent_secondary".to_string(), current_hex: config.accent_secondary },
            ColorEntry { name: "match_highlight".to_string(), current_hex: config.match_highlight },
            ColorEntry { name: "error".to_string(), current_hex: config.error },
            ColorEntry { name: "warning".to_string(), current_hex: config.warning },
        ];
    }
}

#[derive(Clone, Debug)]
pub struct Selection {
    pub start_x: usize,
    pub start_y: usize,
    pub end_x: usize,
    pub end_y: usize,
}

impl Selection {
    pub fn new(x: usize, y: usize) -> Self {
        Self {
            start_x: x,
            start_y: y,
            end_x: x,
            end_y: y,
        }
    }

    pub fn normalized(&self) -> (usize, usize, usize, usize) {
        if self.start_y < self.end_y || (self.start_y == self.end_y && self.start_x <= self.end_x) {
            (self.start_x, self.start_y, self.end_x, self.end_y)
        } else {
            (self.end_x, self.end_y, self.start_x, self.start_y)
        }
    }
}

#[derive(Clone)]
pub struct Buffer {
    pub lines: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,

    pub viewport_offset_y: usize,
    pub viewport_offset_x: usize,

    pub filename: String,
    pub file_path: Option<PathBuf>,
    pub modified: bool,
}

impl Buffer {
    pub fn new(filename: String) -> Self {
        Self {
            lines: vec![String::new()],
            cursor_x: 0,
            cursor_y: 0,
            viewport_offset_y: 0,
            viewport_offset_x: 0,
            filename,
            file_path: None,
            modified: false,
        }
    }
}

#[derive(Clone)]
pub struct UndoState {
    pub lines: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
}

pub struct AppState {
    pub buffers: Vec<Buffer>,
    pub active_buffer: usize,
    pub current_dir: PathBuf,
    pub explorer_files: Vec<String>,
    pub explorer_idx: usize,
    pub explorer_offset: usize,
    pub input_mode: bool,
    pub input_buffer: String,
    pub prompt_type: PromptType,
    pub settings_idx: usize,
    pub current_palette: Palette,
    pub color_entries: Vec<ColorEntry>,
    pub color_editor_idx: usize,
    pub editing_hex: bool,
    pub selection: Option<Selection>,
    pub undo_stack: Vec<UndoState>,
    pub redo_stack: Vec<UndoState>,
    pub status_flash: Option<String>,
    pub status_flash_timer: u8,
    pub confirm_mode: Option<ConfirmType>,
    pub confirm_choice: ConfirmChoice,
    pub clipboard: String,
    pub keybind_state: KeybindState,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            buffers: vec![Buffer::new("unsaved.txt".to_string())],
            active_buffer: 0,
            current_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            explorer_files: Vec::new(),
            explorer_idx: 0,
            explorer_offset: 0,
            input_mode: false,
            input_buffer: String::new(),
            prompt_type: PromptType::Find,
            settings_idx: 0,
            current_palette: Palette::default(),
            color_entries: Vec::new(),
            color_editor_idx: 0,
            editing_hex: false,
            selection: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            status_flash: None,
            status_flash_timer: 0,
            confirm_mode: None,
            confirm_choice: ConfirmChoice::No,
            clipboard: String::new(),
            keybind_state: KeybindState::default(),
        }
    }

    pub fn current_buffer(&self) -> &Buffer {
        &self.buffers[self.active_buffer]
    }

    pub fn current_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.active_buffer]
    }

    pub fn push_undo(&mut self) {
        let lines = self.buffers[self.active_buffer].lines.clone();
        let cursor_x = self.buffers[self.active_buffer].cursor_x;
        let cursor_y = self.buffers[self.active_buffer].cursor_y;

        if self.undo_stack.len() >= 100 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(UndoState {
            lines,
            cursor_x,
            cursor_y,
        });
        self.redo_stack.clear();
    }

    pub fn ensure_cursor_visible(
        &mut self,
        term_w: u16,
        term_h: u16,
        show_line_numbers: bool,
        show_status_bar: bool,
        show_header: bool,
        show_tab_bar: bool,
    ) {
        let buf = self.current_buffer_mut();

        let sidebar_width = if show_line_numbers { 6 } else { 0 };
        let available_width = term_w.saturating_sub(sidebar_width) as usize;
        let available_height = term_h.saturating_sub(
            (if show_header { 1 } else { 0 })
                + (if show_tab_bar { 1 } else { 0 })
                + (if show_status_bar { 1 } else { 0 }),
        ) as usize;

        if buf.cursor_x < buf.viewport_offset_x {
            buf.viewport_offset_x = buf.cursor_x;
        } else if buf.cursor_x >= buf.viewport_offset_x + available_width {
            buf.viewport_offset_x = buf
                .cursor_x
                .saturating_sub(available_width.saturating_sub(1));
        }

        if buf.cursor_y < buf.viewport_offset_y {
            buf.viewport_offset_y = buf.cursor_y;
        } else if buf.cursor_y >= buf.viewport_offset_y + available_height {
            buf.viewport_offset_y = buf
                .cursor_y
                .saturating_sub(available_height.saturating_sub(1));
        }
    }

    pub fn flash_status(&mut self, msg: String) {
        self.status_flash = Some(msg);
        self.status_flash_timer = 20;
    }

    pub fn tick_flash(&mut self) -> bool {
        if self.status_flash_timer > 0 {
            self.status_flash_timer -= 1;
            if self.status_flash_timer == 0 {
                self.status_flash = None;
                return true;
            }
        }
        false
    }
}
  