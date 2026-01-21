pub mod controller;

use crate::core::state::{AppState, Config, KeyCombo, KeybindAction, MenuTab, Mode};

#[derive(Clone, Copy, Debug)]
pub struct UiState {
    pub mode: Mode,
    pub active_tab: MenuTab,
    pub dropdown_idx: usize,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            mode: Mode::Editing,
            active_tab: MenuTab::Re,
            dropdown_idx: 0,
        }
    }
}

pub fn load_custom_keybinds(app: &mut AppState, config: &Config) {
    for (combo_str, action_str) in &config.custom_keybinds {
        if let (Some(combo), Some(action)) = (
            KeyCombo::from_string(combo_str),
            action_str.parse::<KeybindAction>().ok(),
        ) {
            app.keybind_state.custom_binds.insert(combo, action);
        }
    }
}

pub use controller::handle_key_event;
