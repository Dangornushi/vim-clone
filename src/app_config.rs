use std::{fs, path::PathBuf};
use serde::Serialize;
use crate::config::{Config, EditorConfig, UiConfig, KeyBindings};

pub trait ConfigManager {
    fn load_config() -> Config;
    fn reload_config(&mut self) -> Result<(), String>;
    fn show_current_config(&mut self);
    fn reset_config_to_default(&mut self);
    fn set_config_value(&mut self, key: &str, value: &str);
    fn save_config(config: &Config);
}

pub struct AppConfigManager {
    pub config: Config,
    pub status_message: String,
}

impl AppConfigManager {
    pub fn new() -> Self {
        Self {
            config: Self::load_config(),
            status_message: String::new(),
        }
    }
}

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

    fn reload_config(&mut self) -> Result<(), String> {
        let config_path = PathBuf::from("config.json");
        match fs::File::open(&config_path) {
            Ok(file) => {
                match serde_json::from_reader::<_, Config>(file) {
                    Ok(new_config) => {
                        self.config = new_config.with_theme();
                        Ok(())
                    }
                    Err(e) => {
                        Err(format!("Failed to parse config.json: {}", e))
                    }
                }
            }
            Err(e) => {
                Err(format!("Failed to open config.json: {}", e))
            }
        }
    }

    fn show_current_config(&mut self) {
        let config_summary = format!(
            "Current Configuration:\n\
            Editor:\n\
            - Indent width: {}\n\
            - Show line numbers: {}\n\
            - Line number width: {}\n\
            - Tab size: {}\n\
            - Auto indent: {}\n\
            - Word wrap: {}\n\
            - Cursor style: {}\n\
            UI:\n\
            - Theme: {}\n\
            - Directory pane width: {}\n\
            - Status bar height: {}\n\
            - Show directory pane: {}\n\
             - Directory pane floating: {}\n\
            - Editor margins: vertical={}, horizontal={}",
            self.config.editor.indent_width,
            self.config.editor.show_line_numbers,
            self.config.editor.line_number_width,
            self.config.editor.tab_size,
            self.config.editor.auto_indent,
            self.config.editor.word_wrap,
            self.config.editor.cursor_style,
            self.config.ui.theme,
            self.config.ui.directory_pane_width,
            self.config.ui.status_bar_height,
            self.config.ui.show_directory_pane,
            self.config.ui.directory_pane_floating,
            self.config.ui.editor_margins.vertical,
            self.config.ui.editor_margins.horizontal
        );
        
        self.status_message = "Configuration displayed (check terminal output)".to_string();
        println!("{}", config_summary);
    }

    fn reset_config_to_default(&mut self) {
        self.config = Config::default();
        Self::save_config(&self.config);
        self.status_message = "Configuration reset to default values".to_string();
    }

    fn set_config_value(&mut self, key: &str, value: &str) {
        match key {
            "indent_width" | "indentwidth" => {
                if let Ok(val) = value.parse::<usize>() {
                    if val > 0 && val <= 16 {
                        self.config.editor.indent_width = val;
                        Self::save_config(&self.config);
                        self.status_message = format!("indent_width set to {}", val);
                    } else {
                        self.status_message = "indent_width must be between 1 and 16".to_string();
                    }
                } else {
                    self.status_message = "Invalid value for indent_width (must be a number)".to_string();
                }
            }
            "show_line_numbers" | "number" | "nu" => {
                match value.to_lowercase().as_str() {
                    "true" | "1" | "on" | "yes" => {
                        self.config.editor.show_line_numbers = true;
                        Self::save_config(&self.config);
                        self.status_message = "Line numbers enabled".to_string();
                    }
                    "false" | "0" | "off" | "no" => {
                        self.config.editor.show_line_numbers = false;
                        Self::save_config(&self.config);
                        self.status_message = "Line numbers disabled".to_string();
                    }
                    _ => {
                        self.status_message = "Invalid value for show_line_numbers (use true/false)".to_string();
                    }
                }
            }
            "line_number_width" | "numberwidth" | "nuw" => {
                if let Ok(val) = value.parse::<usize>() {
                    if val > 0 && val <= 10 {
                        self.config.editor.line_number_width = val;
                        Self::save_config(&self.config);
                        self.status_message = format!("line_number_width set to {}", val);
                    } else {
                        self.status_message = "line_number_width must be between 1 and 10".to_string();
                    }
                } else {
                    self.status_message = "Invalid value for line_number_width (must be a number)".to_string();
                }
            }
            "tab_size" | "tabsize" | "ts" => {
                if let Ok(val) = value.parse::<usize>() {
                    if val > 0 && val <= 16 {
                        self.config.editor.tab_size = val;
                        Self::save_config(&self.config);
                        self.status_message = format!("tab_size set to {}", val);
                    } else {
                        self.status_message = "tab_size must be between 1 and 16".to_string();
                    }
                } else {
                    self.status_message = "Invalid value for tab_size (must be a number)".to_string();
                }
            }
            "auto_indent" | "autoindent" | "ai" => {
                match value.to_lowercase().as_str() {
                    "true" | "1" | "on" | "yes" => {
                        self.config.editor.auto_indent = true;
                        Self::save_config(&self.config);
                        self.status_message = "Auto indent enabled".to_string();
                    }
                    "false" | "0" | "off" | "no" => {
                        self.config.editor.auto_indent = false;
                        Self::save_config(&self.config);
                        self.status_message = "Auto indent disabled".to_string();
                    }
                    _ => {
                        self.status_message = "Invalid value for auto_indent (use true/false)".to_string();
                    }
                }
            }
            "word_wrap" | "wrap" => {
                match value.to_lowercase().as_str() {
                    "true" | "1" | "on" | "yes" => {
                        self.config.editor.word_wrap = true;
                        Self::save_config(&self.config);
                        self.status_message = "Word wrap enabled".to_string();
                    }
                    "false" | "0" | "off" | "no" => {
                        self.config.editor.word_wrap = false;
                        Self::save_config(&self.config);
                        self.status_message = "Word wrap disabled".to_string();
                    }
                    _ => {
                        self.status_message = "Invalid value for word_wrap (use true/false)".to_string();
                    }
                }
            }
            "directory_pane_width" | "dirwidth" => {
                if let Ok(val) = value.parse::<u16>() {
                    if (10..=100).contains(&val) {
                        self.config.ui.directory_pane_width = val;
                        Self::save_config(&self.config);
                        self.status_message = format!("directory_pane_width set to {}", val);
                    } else {
                        self.status_message = "directory_pane_width must be between 10 and 100".to_string();
                    }
                } else {
                    self.status_message = "Invalid value for directory_pane_width (must be a number)".to_string();
                }
            }
            "directory_pane_floating" | "dirfloat" => {
                match value.to_lowercase().as_str() {
                    "true" | "1" | "on" | "yes" => {
                        self.config.ui.directory_pane_floating = true;
                        Self::save_config(&self.config);
                        self.status_message = "Directory pane floating enabled".to_string();
                    }
                    "false" | "0" | "off" | "no" => {
                        self.config.ui.directory_pane_floating = false;
                        Self::save_config(&self.config);
                        self.status_message = "Directory pane floating disabled".to_string();
                    }
                    _ => {
                        self.status_message = "Invalid value for directory_pane_floating (use true/false)".to_string();
                    }
                }
            }
            "theme" => {
                self.config.ui.theme = value.to_string();
                Self::save_config(&self.config);
                self.status_message = format!("Theme set to '{}'", value);
            }
            _ => {
                self.status_message = format!("Unknown setting: {}", key);
            }
        }
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