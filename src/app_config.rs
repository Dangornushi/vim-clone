use std::{fs, path::PathBuf};
use serde::Serialize;
use crate::config::{Config, EditorConfig, UiConfig, KeyBindings};

pub trait ConfigManager {
    fn load_config() -> Config;
    fn save_config(config: &Config);
}

pub struct AppConfigManager;

impl AppConfigManager {}

impl ConfigManager for AppConfigManager {
    fn load_config() -> Config {
        let config_path = PathBuf::from("config.json");
        let config = if let Ok(file) = fs::File::open(&config_path) {
            serde_json::from_reader(file).unwrap_or_else(|e| {
                eprintln!("Failed to parse config.json: {}. Using default config.", e);
                let default_config = Config::default();
                Self::save_config(&default_config);
                default_config
            })
        } else {
            eprintln!("config.json not found. Creating a default one.");
            let default_config = Config::default();
            Self::save_config(&default_config);
            default_config
        };
        config.with_theme()
    }

    fn save_config(config: &Config) {
        #[derive(Serialize)]
        struct SerializableConfig<'a> {
            editor: &'a EditorConfig,
            ui: &'a UiConfig,
            key_bindings: &'a KeyBindings,
        }

        let serializable_config = SerializableConfig {
            editor: &config.editor,
            ui: &config.ui,
            key_bindings: &config.key_bindings,
        };

        if let Ok(file) = fs::File::create("config.json") {
            serde_json::to_writer_pretty(file, &serializable_config).ok();
        }
    }
}