use crate::core::state::{
    AppState, Buffer, ColorEntry, Config, ConfirmChoice, ConfirmType, KeybindState, MenuTab, Mode,
    Palette, PromptType, Selection,
};
use std::path::PathBuf;

pub struct RenderSnapshot<'a> {
    pub mode: Mode,
    pub active_tab: MenuTab,
    pub dropdown_idx: usize,
    pub config: &'a Config,
    pub palette: Palette,
    pub buffers: &'a [Buffer],
    pub active_buffer: usize,
    pub selection: &'a Option<Selection>,
    pub input_mode: bool,
    pub input_buffer: &'a str,
    pub prompt_type: PromptType,
    pub current_dir: &'a PathBuf,
    pub status_flash: &'a Option<String>,
    pub undo_len: usize,
    pub redo_len: usize,
    pub settings_idx: usize,
    pub color_entries: &'a [ColorEntry],
    pub color_editor_idx: usize,
    pub editing_hex: bool,
    pub confirm_mode: &'a Option<ConfirmType>,
    pub confirm_choice: ConfirmChoice,
    pub explorer_files: &'a [String],
    pub explorer_idx: usize,
    pub explorer_offset: usize,
    pub keybind_state: &'a KeybindState,
}

impl<'a> RenderSnapshot<'a> {
    pub fn current_buffer(&self) -> &Buffer {
        &self.buffers[self.active_buffer]
    }
}

pub fn build_snapshot<'a>(
    app: &'a AppState,
    config: &'a Config,
    mode: Mode,
    active_tab: MenuTab,
    dropdown_idx: usize,
) -> RenderSnapshot<'a> {
    RenderSnapshot {
        mode,
        active_tab,
        dropdown_idx,
        config,
        palette: app.current_palette,
        buffers: &app.buffers,
        active_buffer: app.active_buffer,
        selection: &app.selection,
        input_mode: app.input_mode,
        input_buffer: &app.input_buffer,
        prompt_type: app.prompt_type,
        current_dir: &app.current_dir,
        status_flash: &app.status_flash,
        undo_len: app.undo_stack.len(),
        redo_len: app.redo_stack.len(),
        settings_idx: app.settings_idx,
        color_entries: &app.color_entries,
        color_editor_idx: app.color_editor_idx,
        editing_hex: app.editing_hex,
        confirm_mode: &app.confirm_mode,
        confirm_choice: app.confirm_choice,
        explorer_files: &app.explorer_files,
        explorer_idx: app.explorer_idx,
        explorer_offset: app.explorer_offset,
        keybind_state: &app.keybind_state,
    }
}
