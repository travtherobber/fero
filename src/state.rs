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
        let mods = parts[0];
        let code = parts[1];

        let mut m = KeyModifiers::empty();
        if mods.contains("CONTROL") {
            m |= KeyModifiers::CONTROL;
        }
        if mods.contains("SHIFT") {
            m |= KeyModifiers::SHIFT;
        }
        if mods.contains("ALT") {
            m |= KeyModifiers::ALT;
        }

        let kc = if code.starts_with("Char(") {
            if let Some(ch_part) = code.strip_prefix("Char('") {
                if let Some(end) = ch_part.find("')") {
                    let chs = &ch_part[..end];
                    chs.chars().next().map_or(KeyCode::Char(' '), KeyCode::Char)
                } else {
                    KeyCode::Char(' ')
                }
            } else {
                KeyCode::Char(' ')
            }
        } else if code.contains("Enter") {
            KeyCode::Enter
        } else if code.contains("Backspace") {
            KeyCode::Backspace
        } else if code.contains("Tab") {
            KeyCode::Tab
        } else if code.contains("Left") {
            KeyCode::Left
        } else if code.contains("Right") {
            KeyCode::Right
        } else if code.contains("Up") {
            KeyCode::Up
        } else if code.contains("Down") {
            KeyCode::Down
        } else if code.contains("Delete") {
            KeyCode::Delete
        } else if code.contains("Esc") || code.contains("Esc") {
            KeyCode::Esc
        } else {
            KeyCode::Char(' ')
        };

        Some(KeyCombo {
            code: kc,
            modifiers: m,
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
    pub bg: String,
    pub primary: String,
    pub panel: String,
    pub accent: String,
    pub highlight: String,
    pub text: String,
    pub warning: String,
}

impl Default for PaletteConfig {
    fn default() -> Self {
        Self {
            bg: "#0A0F0B".to_string(),
            primary: "#18FF6D".to_string(),
            panel: "#0E3B2A".to_string(),
            accent: "#1F7A4A".to_string(),
            highlight: "#9DFFB8".to_string(),
            text: "#F2FFE9".to_string(),
            warning: "#FF8C1A".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg: Color,
    pub primary: Color,
    pub panel: Color,
    pub accent: Color,
    pub highlight: Color,
    pub text: Color,
    pub warning: Color,
}

impl Palette {
    pub fn default() -> Self {
        Self::from_config(&PaletteConfig::default())
    }

    pub fn from_config(cfg: &PaletteConfig) -> Self {
        Self {
            bg: Self::hex_to_color(&cfg.bg),
            primary: Self::hex_to_color(&cfg.primary),
            panel: Self::hex_to_color(&cfg.panel),
            accent: Self::hex_to_color(&cfg.accent),
            highlight: Self::hex_to_color(&cfg.highlight),
            text: Self::hex_to_color(&cfg.text),
            warning: Self::hex_to_color(&cfg.warning),
        }
    }

    pub fn to_config(&self) -> PaletteConfig {
        PaletteConfig {
            bg: Self::to_hex(self.bg),
            primary: Self::to_hex(self.primary),
            panel: Self::to_hex(self.panel),
            accent: Self::to_hex(self.accent),
            highlight: Self::to_hex(self.highlight),
            text: Self::to_hex(self.text),
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

    pub fn tick_flash(&mut self) {
        if self.status_flash_timer > 0 {
            self.status_flash_timer -= 1;
            if self.status_flash_timer == 0 {
                self.status_flash = None;
            }
        }
    }
}
