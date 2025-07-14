use std::{
    env,
    fs,
    io::{self, Write},
    path::PathBuf,
};
use arboard::Clipboard;
use crate::{pane::PaneManager, config::{Config, EditorConfig, UiConfig, KeyBindings}, syntax::{BracketState, detect_unmatched_brackets_in_file}};
use serde::Serialize;

// Define the editor modes
#[derive(Copy, Clone)]
pub enum Mode {
    Normal,
    Insert,
    Command,
    Visual,
}

// App holds the state of the application
#[derive(Clone)]
pub struct WindowState {
    pub buffer: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
}

pub struct Window {
    pub buffer: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub scroll_y: usize,
    pub scroll_x: usize,
    pub filename: Option<String>,
    pub visual_start: Option<(usize, usize)>,
    pub yanked_text: String,
    pub bracket_state: BracketState,
    pub unmatched_brackets: Vec<(usize, usize)>, // (行番号, 位置)
    pub undo_stack: Vec<WindowState>,
    pub redo_stack: Vec<WindowState>,
    pub insert_mode_start_state: Option<WindowState>, // 挿入モード開始時の状態
}

impl Window {
    pub fn new(filename: Option<String>) -> Self {
        let buffer = if let Some(path) = &filename {
            fs::read_to_string(path)
                .map(|content| content.lines().map(String::from).collect())
                .unwrap_or_else(|_| vec![String::new()])
        } else {
            vec![String::new()]
        };

        let unmatched_brackets = detect_unmatched_brackets_in_file(&buffer);
        
        Self {
            buffer,
            cursor_x: 0,
            cursor_y: 0,
            scroll_y: 0,
            scroll_x: 0,
            filename,
            visual_start: None,
            yanked_text: String::new(),
            bracket_state: BracketState::new(),
            unmatched_brackets,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            insert_mode_start_state: None,
        }
    }

    pub fn save_file(&mut self) -> io::Result<()> {
        if let Some(filename) = &self.filename {
            let mut file = fs::File::create(filename)?;
            for line in &self.buffer {
                writeln!(file, "{}", line)?;
            }
            self.update_unmatched_brackets();
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "No file name"))
        }
    }

    pub fn reload_file(&mut self) -> io::Result<()> {
        if let Some(filename) = &self.filename {
            match fs::read_to_string(filename) {
                Ok(content) => {
                    // ファイル内容を再読み込み
                    self.buffer = if content.is_empty() {
                        vec![String::new()]
                    } else {
                        content.lines().map(String::from).collect()
                    };
                    
                    // カーソル位置を安全な範囲に調整
                    if self.cursor_y >= self.buffer.len() {
                        self.cursor_y = self.buffer.len().saturating_sub(1);
                    }
                    
                    let current_line_len = self.buffer.get(self.cursor_y).map_or(0, |line| line.len());
                    if self.cursor_x > current_line_len {
                        self.cursor_x = current_line_len;
                    }
                    
                    // スクロール位置も調整
                    if self.scroll_y >= self.buffer.len() {
                        self.scroll_y = self.buffer.len().saturating_sub(1);
                    }
                    
                    self.update_unmatched_brackets();
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "No file name to reload"))
        }
    }

    pub fn update_unmatched_brackets(&mut self) {
        self.unmatched_brackets = detect_unmatched_brackets_in_file(&self.buffer);
    }

    pub fn save_state(&mut self) {
        let state = WindowState {
            buffer: self.buffer.clone(),
            cursor_x: self.cursor_x,
            cursor_y: self.cursor_y,
        };
        self.undo_stack.push(state);
        
        // アンドゥスタックのサイズ制限（メモリ使用量を制御）
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        
        // 新しい変更が行われたらリドゥスタックをクリア
        self.redo_stack.clear();
    }

    // 挿入モード開始時に呼ぶ
    pub fn start_insert_mode(&mut self) {
        self.insert_mode_start_state = Some(WindowState {
            buffer: self.buffer.clone(),
            cursor_x: self.cursor_x,
            cursor_y: self.cursor_y,
        });
    }

    // 挿入モード終了時に呼ぶ（Escキー押下時）
    pub fn end_insert_mode(&mut self) {
        if let Some(start_state) = self.insert_mode_start_state.take() {
            // 挿入モード開始時の状態をアンドゥスタックに保存
            self.undo_stack.push(start_state);
            
            // アンドゥスタックのサイズ制限
            if self.undo_stack.len() > 100 {
                self.undo_stack.remove(0);
            }
            
            // 新しい変更が行われたらリドゥスタックをクリア
            self.redo_stack.clear();
        }
    }

    pub fn undo(&mut self) -> bool {
        if let Some(state) = self.undo_stack.pop() {
            // 現在の状態をリドゥスタックに保存
            let current_state = WindowState {
                buffer: self.buffer.clone(),
                cursor_x: self.cursor_x,
                cursor_y: self.cursor_y,
            };
            self.redo_stack.push(current_state);
            
            // 前の状態を復元
            self.buffer = state.buffer;
            self.cursor_x = state.cursor_x;
            self.cursor_y = state.cursor_y;
            
            // カーソル位置を安全な範囲に調整
            if self.cursor_y >= self.buffer.len() {
                self.cursor_y = self.buffer.len().saturating_sub(1);
            }
            if self.cursor_y < self.buffer.len() {
                let line_len = self.buffer[self.cursor_y].len();
                if self.cursor_x > line_len {
                    self.cursor_x = line_len;
                }
            }
            
            self.update_unmatched_brackets();
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(state) = self.redo_stack.pop() {
            // 現在の状態をアンドゥスタックに保存
            let current_state = WindowState {
                buffer: self.buffer.clone(),
                cursor_x: self.cursor_x,
                cursor_y: self.cursor_y,
            };
            self.undo_stack.push(current_state);
            
            // 次の状態を復元
            self.buffer = state.buffer;
            self.cursor_x = state.cursor_x;
            self.cursor_y = state.cursor_y;
            
            // カーソル位置を安全な範囲に調整
            if self.cursor_y >= self.buffer.len() {
                self.cursor_y = self.buffer.len().saturating_sub(1);
            }
            if self.cursor_y < self.buffer.len() {
                let line_len = self.buffer[self.cursor_y].len();
                if self.cursor_x > line_len {
                    self.cursor_x = line_len;
                }
            }
            
            self.update_unmatched_brackets();
            true
        } else {
            false
        }
    }

    pub fn scroll_to_cursor(&mut self, height: usize, width: usize, show_line_numbers: bool) {
        // Vertical scroll
        if self.cursor_y < self.scroll_y {
            self.scroll_y = self.cursor_y;
        } else if self.cursor_y >= self.scroll_y + height {
            self.scroll_y = self.cursor_y - height + 1;
        }

        // Horizontal scroll
        let line_number_width = if show_line_numbers { 4 } else { 0 };
        let separator_width = if show_line_numbers { 1 } else { 0 };
        let available_width = width.saturating_sub(line_number_width + separator_width);

        if self.cursor_x < self.scroll_x {
            self.scroll_x = self.cursor_x;
        } else if self.cursor_x >= self.scroll_x + available_width {
            self.scroll_x = self.cursor_x - available_width + 1;
        }
    }

    pub fn open_new_line(&mut self) {
        self.save_state();
        let new_line_y = self.cursor_y + 1;
        self.buffer.insert(new_line_y, String::new());
        self.cursor_y = new_line_y;
        self.cursor_x = 0;
        self.update_unmatched_brackets();
    }
}

pub struct App {
    pub windows: Vec<Window>,
    pub pane_manager: PaneManager,
    pub mode: Mode,
    pub command_buffer: String,
    pub status_message: String,
    clipboard: Clipboard,
    pub current_path: PathBuf,
    pub directory_files: Vec<String>,
    pub selected_directory_index: usize,
    pub show_directory: bool,
    pub config: Config,
}

impl App {
    pub fn new(filename: Option<String>) -> Self {
        let config = App::load_config();
        let initial_window = Window::new(filename.clone());
        let path = if let Some(f) = &filename {
            PathBuf::from(f)
                .parent()
                .map_or_else(|| env::current_dir().unwrap(), |p| p.to_path_buf())
        } else {
            env::current_dir().unwrap()
        };

        let mut app = Self {
            windows: vec![initial_window],
            pane_manager: PaneManager::new(0),
            mode: Mode::Normal,
            command_buffer: String::new(),
            status_message: String::new(),
            clipboard: Clipboard::new().unwrap(),
            current_path: path,
            directory_files: vec![],
            selected_directory_index: 0,
            show_directory: true,
            config,
        };
        app.update_directory_files();
        app
    }

    fn load_config() -> Config {
        let config_path = PathBuf::from("config.json");
        let config = if let Ok(file) = fs::File::open(&config_path) {
            serde_json::from_reader(file).unwrap_or_else(|e| {
                eprintln!("Failed to parse config.json: {}. Using default config.", e);
                let default_config = Config::default();
                App::save_config(&default_config);
                default_config
            })
        } else {
            eprintln!("config.json not found. Creating a default one.");
            let default_config = Config::default();
            App::save_config(&default_config);
            default_config
        };
        config.with_theme()
    }

    pub fn reload_config(&mut self) -> Result<(), String> {
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

    pub fn show_current_config(&mut self) {
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
            self.config.ui.editor_margins.vertical,
            self.config.ui.editor_margins.horizontal
        );
        
        self.status_message = "Configuration displayed (check terminal output)".to_string();
        // 実際のアプリケーションでは、これを別のペインやポップアップで表示することもできます
        println!("{}", config_summary);
    }

    pub fn reset_config_to_default(&mut self) {
        self.config = Config::default();
        App::save_config(&self.config);
        self.status_message = "Configuration reset to default values".to_string();
    }

    pub fn set_config_value(&mut self, key: &str, value: &str) {
        match key {
            // Editor settings
            "indent_width" | "indentwidth" => {
                if let Ok(val) = value.parse::<usize>() {
                    if val > 0 && val <= 16 {
                        self.config.editor.indent_width = val;
                        App::save_config(&self.config);
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
                        App::save_config(&self.config);
                        self.status_message = "Line numbers enabled".to_string();
                    }
                    "false" | "0" | "off" | "no" => {
                        self.config.editor.show_line_numbers = false;
                        App::save_config(&self.config);
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
                        App::save_config(&self.config);
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
                        App::save_config(&self.config);
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
                        App::save_config(&self.config);
                        self.status_message = "Auto indent enabled".to_string();
                    }
                    "false" | "0" | "off" | "no" => {
                        self.config.editor.auto_indent = false;
                        App::save_config(&self.config);
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
                        App::save_config(&self.config);
                        self.status_message = "Word wrap enabled".to_string();
                    }
                    "false" | "0" | "off" | "no" => {
                        self.config.editor.word_wrap = false;
                        App::save_config(&self.config);
                        self.status_message = "Word wrap disabled".to_string();
                    }
                    _ => {
                        self.status_message = "Invalid value for word_wrap (use true/false)".to_string();
                    }
                }
            }
            // UI settings
            "directory_pane_width" | "dirwidth" => {
                if let Ok(val) = value.parse::<u16>() {
                    if val >= 10 && val <= 100 {
                        self.config.ui.directory_pane_width = val;
                        App::save_config(&self.config);
                        self.status_message = format!("directory_pane_width set to {}", val);
                    } else {
                        self.status_message = "directory_pane_width must be between 10 and 100".to_string();
                    }
                } else {
                    self.status_message = "Invalid value for directory_pane_width (must be a number)".to_string();
                }
            }
            "theme" => {
                self.config.ui.theme = value.to_string();
                App::save_config(&self.config);
                self.status_message = format!("Theme set to '{}'", value);
            }
            _ => {
                self.status_message = format!("Unknown setting: {}", key);
            }
        }
    }

    fn save_config(config: &Config) {
        // Create a temporary config without the theme for serialization
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

    fn update_directory_files(&mut self) {
        self.directory_files.clear();
        if self.current_path.parent().is_some() {
            self.directory_files.push("../".to_string());
        }
        if let Ok(entries) = fs::read_dir(&self.current_path) {
            let mut files = vec![];
            let mut dirs = vec![];

            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        dirs.push(format!("{}/", file_name));
                    } else {
                        files.push(file_name);
                    }
                }
            }
            dirs.sort();
            files.sort();
            self.directory_files.extend(dirs);
            self.directory_files.extend(files);
        }
        self.selected_directory_index = 0;
    }

    pub fn open_selected_item(&mut self) {
        if let Some(selected_item) = self.directory_files.get(self.selected_directory_index).cloned() {
            let item_name = selected_item.trim_end_matches('/');

            if item_name == ".." {
                if let Some(parent) = self.current_path.parent() {
                    self.current_path = parent.to_path_buf();
                    self.update_directory_files();
                }
                return;
            }

            let new_path = self.current_path.join(item_name);

            if new_path.is_dir() {
                self.current_path = new_path;
                self.update_directory_files();
            } else if new_path.is_file() {
                let file_path_str = new_path.to_str().unwrap().to_string();
                let window_index = if let Some(index) = self.windows.iter().position(|w| w.filename.as_deref() == Some(&file_path_str)) {
                    index
                } else {
                    let new_window = Window::new(Some(file_path_str));
                    self.windows.push(new_window);
                    self.windows.len() - 1
                };
                
                // 現在のアクティブペインのウィンドウインデックスを更新
                let active_pane_id = self.pane_manager.get_active_pane_id();
                if let Some(pane) = self.pane_manager.get_pane_mut(active_pane_id) {
                    pane.window_index = window_index;
                }
                self.show_directory = false;
            }
        }
    }

    pub fn vsplit_selected_item(&mut self) {
        if let Some(selected_item) = self.directory_files.get(self.selected_directory_index).cloned() {
            let item_name = selected_item.trim_end_matches('/');
            let new_path = self.current_path.join(item_name);

            if new_path.is_file() {
                let file_path_str = new_path.to_str().unwrap().to_string();
                let window_index = if let Some(index) = self.windows.iter().position(|w| w.filename.as_deref() == Some(&file_path_str)) {
                    index
                } else {
                    let new_window = Window::new(Some(file_path_str));
                    self.windows.push(new_window);
                    self.windows.len() - 1
                };

                // 現在のアクティブペインを垂直分割
                let active_pane_id = self.pane_manager.get_active_pane_id();
                if let Some(new_pane_id) = self.pane_manager.vsplit(active_pane_id, window_index) {
                    self.pane_manager.set_active_pane(new_pane_id);
                }
                self.show_directory = false;
            }
        }
    }

    pub fn hsplit_selected_item(&mut self) {
        if let Some(selected_item) = self.directory_files.get(self.selected_directory_index).cloned() {
            let item_name = selected_item.trim_end_matches('/');
            let new_path = self.current_path.join(item_name);

            if new_path.is_file() {
                let file_path_str = new_path.to_str().unwrap().to_string();
                let window_index = if let Some(index) = self.windows.iter().position(|w| w.filename.as_deref() == Some(&file_path_str)) {
                    index
                } else {
                    let new_window = Window::new(Some(file_path_str));
                    self.windows.push(new_window);
                    self.windows.len() - 1
                };

                // 現在のアクティブペインを水平分割
                let active_pane_id = self.pane_manager.get_active_pane_id();
                if let Some(new_pane_id) = self.pane_manager.hsplit(active_pane_id, window_index) {
                    self.pane_manager.set_active_pane(new_pane_id);
                }
                self.show_directory = false;
            }
        }
    }

    pub fn current_window(&mut self) -> &mut Window {
        let index = self.get_active_window_index();
        &mut self.windows[index]
    }

    pub fn set_yanked_text(&mut self, text: String) {
        self.current_window().yanked_text = text.clone();
        if let Err(e) = self.clipboard.set_text(text) {
            self.status_message = format!("Failed to set clipboard: {}", e);
        }
    }

    pub fn get_clipboard_text(&mut self) -> Result<String, arboard::Error> {
        self.clipboard.get_text()
    }

    fn get_active_window_index(&self) -> usize {
        if let Some(active_pane) = self.pane_manager.get_active_pane() {
            active_pane.window_index
        } else {
            0 // フォールバック
        }
    }

    pub fn activate_left_pane(&mut self) {
        self.pane_manager.move_to_left_pane();
    }

    pub fn activate_right_pane(&mut self) {
        self.pane_manager.move_to_right_pane();
    }

    pub fn open_file(&mut self, filename: &str) {
        let file_path = if filename.starts_with('/') {
            // 絶対パス
            PathBuf::from(filename)
        } else {
            // 相対パス
            self.current_path.join(filename)
        };

        let file_path_str = file_path.to_string_lossy().to_string();
        
        // 既に開いているファイルかチェック
        if let Some(window_index) = self.windows.iter().position(|w| {
            w.filename.as_deref() == Some(&file_path_str)
        }) {
            // 既に開いている場合は、そのウィンドウをアクティブにする
            let active_pane_id = self.pane_manager.get_active_pane_id();
            if let Some(pane) = self.pane_manager.get_pane_mut(active_pane_id) {
                pane.window_index = window_index;
            }
            self.status_message = format!("Switched to \"{}\"", filename);
        } else {
            // 新しいファイルを開く
            let new_window = Window::new(Some(file_path_str.clone()));
            
            // ファイルが存在するかチェック
            if file_path.exists() {
                self.windows.push(new_window);
                let window_index = self.windows.len() - 1;
                
                // 現在のアクティブペインのウィンドウインデックスを更新
                let active_pane_id = self.pane_manager.get_active_pane_id();
                if let Some(pane) = self.pane_manager.get_pane_mut(active_pane_id) {
                    pane.window_index = window_index;
                }
                self.status_message = format!("\"{}\" opened", filename);
            } else {
                // ファイルが存在しない場合は新規作成
                self.windows.push(new_window);
                let window_index = self.windows.len() - 1;
                
                let active_pane_id = self.pane_manager.get_active_pane_id();
                if let Some(pane) = self.pane_manager.get_pane_mut(active_pane_id) {
                    pane.window_index = window_index;
                }
                self.status_message = format!("\"{}\" [New File]", filename);
            }
        }
    }

}