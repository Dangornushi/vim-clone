use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ratatui::style::Color;
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum SerializableColor {
    Name(String),
    Rgb([u8; 3]),
    Indexed(u8),
}

impl From<SerializableColor> for Color {
    fn from(sc: SerializableColor) -> Self {
        match sc {
            SerializableColor::Name(name) => match name.to_lowercase().as_str() {
                "black" => Color::Black,
                "red" => Color::Red,
                "green" => Color::Green,
                "yellow" => Color::Yellow,
                "blue" => Color::Blue,
                "magenta" => Color::Magenta,
                "cyan" => Color::Cyan,
                "gray" => Color::Gray,
                "darkgray" => Color::DarkGray,
                "lightred" => Color::LightRed,
                "lightgreen" => Color::LightGreen,
                "lightblue" => Color::LightBlue,
                "lightmagenta" => Color::LightMagenta,
                "lightcyan" => Color::LightCyan,
                "white" => Color::White,
                _ => Color::Reset,
            },
            SerializableColor::Rgb(rgb) => Color::Rgb(rgb[0], rgb[1], rgb[2]),
            SerializableColor::Indexed(i) => Color::Indexed(i),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SyntaxTheme {
    pub keyword: SerializableColor,
    pub string: SerializableColor,
    pub number: SerializableColor,
    pub comment: SerializableColor,
    pub function: SerializableColor,
    #[serde(rename = "macro")]
    pub r#macro: SerializableColor,
    #[serde(rename = "type")]
    pub r#type: SerializableColor,
    pub identifier: SerializableColor,
    pub operator: SerializableColor,
    pub symbol: SerializableColor,
    pub bracket_colors: Vec<SerializableColor>,
    pub unmatched_bracket_fg: SerializableColor,
    pub unmatched_bracket_bg: SerializableColor,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiTheme {
    pub active_pane_border: SerializableColor,
    pub selection_background: SerializableColor,
    pub status_bar_background: SerializableColor,
    pub line_number: SerializableColor,
    pub visual_selection_background: SerializableColor,
    pub indent_colors: Vec<SerializableColor>,
    pub completion_background: SerializableColor,
    pub completion_foreground: SerializableColor,
    pub completion_selection_background: SerializableColor,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[derive(Default)]
pub struct Theme {
    pub syntax: SyntaxTheme,
    pub ui: UiTheme,
}

impl Theme {
    pub fn load(name: &str) -> Self {
        let path_str = format!("themes/{}.json", name);
        let path = Path::new(&path_str);
        if let Ok(file_content) = fs::read_to_string(path) {
            match serde_json::from_str(&file_content) {
                Ok(theme) => return theme,
                Err(e) => {
                    eprintln!("Failed to parse theme file: {}, error: {}", path.display(), e);
                }
            }
        } else {
            eprintln!("Failed to read theme file: {}", path.display());
        }
        // フォールバックとしてデフォルトテーマを返す
        Theme::default()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyBindings {
    pub normal: HashMap<String, String>,
    pub ctrl: HashMap<String, String>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut normal = HashMap::new();
        normal.insert("h".to_string(), "move_left".to_string());
        normal.insert("j".to_string(), "move_down".to_string());
        normal.insert("k".to_string(), "move_up".to_string());
        normal.insert("l".to_string(), "move_right".to_string());
        normal.insert("i".to_string(), "mode_insert".to_string());
        normal.insert("v".to_string(), "mode_visual".to_string());
        normal.insert(":".to_string(), "mode_command".to_string());
        normal.insert("p".to_string(), "paste".to_string());
        normal.insert("x".to_string(), "delete_char".to_string());
        normal.insert("a".to_string(), "append".to_string());
        normal.insert("u".to_string(), "undo".to_string());
        normal.insert("o".to_string(), "open_new_line".to_string());
        
        let mut ctrl = HashMap::new();
        ctrl.insert("f".to_string(), "toggle_directory".to_string());
        ctrl.insert("b".to_string(), "toggle_right_panel".to_string());
        ctrl.insert("r".to_string(), "redo".to_string());
        
        Self { normal, ctrl }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EditorConfig {
    pub indent_width: usize,
    pub show_line_numbers: bool,
    pub line_number_width: usize,
    pub tab_size: usize,
    pub auto_indent: bool,
    pub word_wrap: bool,
    pub cursor_style: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EditorMargins {
    pub vertical: u16,
    pub horizontal: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiConfig {
    pub theme: String,
    pub directory_pane_width: u16,
    pub status_bar_height: u16,
    pub show_directory_pane: bool,
    pub directory_pane_floating: bool,
    pub editor_margins: EditorMargins,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[derive(Default)]
pub struct Config {
    pub editor: EditorConfig,
    pub ui: UiConfig,
    pub key_bindings: KeyBindings,
    #[serde(skip)]
    pub theme: Theme,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            indent_width: 4,
            show_line_numbers: true,
            line_number_width: 4,
            tab_size: 4,
            auto_indent: true,
            word_wrap: false,
            cursor_style: "block".to_string(),
        }
    }
}

impl Default for EditorMargins {
    fn default() -> Self {
        Self {
            vertical: 1,
            horizontal: 1,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            directory_pane_width: 30,
            status_bar_height: 1,
            show_directory_pane: false,
            directory_pane_floating: false,
            editor_margins: EditorMargins::default(),
        }
    }
}

impl Default for SyntaxTheme {
    fn default() -> Self {
        Self {
            keyword: SerializableColor::Name("Yellow".to_string()),
            string: SerializableColor::Name("Green".to_string()),
            number: SerializableColor::Name("Magenta".to_string()),
            comment: SerializableColor::Indexed(244),
            function: SerializableColor::Name("LightBlue".to_string()),
            r#macro: SerializableColor::Rgb([255, 165, 0]),
            r#type: SerializableColor::Name("LightCyan".to_string()),
            identifier: SerializableColor::Name("White".to_string()),
            operator: SerializableColor::Name("Yellow".to_string()),
            symbol: SerializableColor::Rgb([200, 200, 200]),
            bracket_colors: vec![
                SerializableColor::Name("White".to_string()),
                SerializableColor::Rgb([255, 100, 100]),
                SerializableColor::Rgb([100, 255, 100]),
                SerializableColor::Rgb([100, 100, 255]),
                SerializableColor::Rgb([255, 255, 100]),
                SerializableColor::Rgb([255, 100, 255]),
                SerializableColor::Rgb([100, 255, 255]),
                SerializableColor::Rgb([255, 200, 100]),
            ],
            unmatched_bracket_fg: SerializableColor::Name("Red".to_string()),
            unmatched_bracket_bg: SerializableColor::Rgb([80, 0, 0]),
        }
    }
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            active_pane_border: SerializableColor::Name("Blue".to_string()),
            selection_background: SerializableColor::Name("Blue".to_string()),
            status_bar_background: SerializableColor::Name("Gray".to_string()),
            line_number: SerializableColor::Name("DarkGray".to_string()),
            visual_selection_background: SerializableColor::Name("Blue".to_string()),
            indent_colors: vec![
                SerializableColor::Rgb([60, 50, 50]),
                SerializableColor::Rgb([50, 60, 50]),
                SerializableColor::Rgb([50, 50, 60]),
                SerializableColor::Rgb([60, 60, 50]),
                SerializableColor::Rgb([60, 50, 60]),
                SerializableColor::Rgb([50, 60, 60]),
                SerializableColor::Rgb([55, 55, 55]),
                SerializableColor::Rgb([65, 55, 50]),
            ],
            completion_background: SerializableColor::Name("DarkGray".to_string()),
            completion_foreground: SerializableColor::Name("White".to_string()),
            completion_selection_background: SerializableColor::Name("Blue".to_string()),
        }
    }
}


impl Config {
    pub fn with_theme(mut self) -> Self {
        self.theme = Theme::load(&self.ui.theme);
        self
    }
}

