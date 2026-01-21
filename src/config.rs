use crate::core::state::Config;
use std::fs;
use std::path::PathBuf;

pub fn get_config_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    path.pop();
    path.push("config.toml");
    path
}

pub fn load_config() -> Config {
    let path = get_config_path();
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(config) = toml::from_str(&content) {
            return config;
        }
    }
    Config::default()
}

pub fn save_config(config: &Config) -> std::io::Result<()> {
    let path = get_config_path();
    let content = toml::to_string_pretty(config).unwrap_or_default();
    fs::write(path, content)
}
 
