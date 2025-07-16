use std::{
    env,
    fs,
    io::{self, Write},
    path::PathBuf,
};
use arboard::Clipboard;
use crate::{pane::PaneManager, config::{Config, EditorConfig, UiConfig, KeyBindings}};
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
struct WindowState {
    buffer: Vec<String>,
    cursor_x: usize,
    cursor_y: usize,
}

pub struct Window {
    buffer: Vec<String>,
    cursor_x: usize,
    cursor_y: usize,
    scroll_y: usize,
    scroll_x: usize,
    filename: Option<String>,
    visual_start: Option<(usize, usize)>,
    yanked_text: String,
    undo_stack: Vec<WindowState>,
    redo_stack: Vec<WindowState>,
    insert_mode_start_state: Option<WindowState>,
    needs_syntax_update: bool,
    last_modified_line: Option<usize>,
    matching_bracket: Option<(usize, usize)>,
}

impl Window {
    pub fn buffer(&self) -> &Vec<String> {
        &self.buffer
    }
    pub fn buffer_mut(&mut self) -> &mut Vec<String> {
        &mut self.buffer
    }
    pub fn cursor_x(&self) -> usize {
        self.cursor_x
    }
    pub fn cursor_x_mut(&mut self) -> &mut usize {
        &mut self.cursor_x
    }
    pub fn cursor_y(&self) -> usize {
        self.cursor_y
    }
    pub fn cursor_y_mut(&mut self) -> &mut usize {
        &mut self.cursor_y
    }
    pub fn scroll_y(&self) -> usize {
        self.scroll_y
    }
    pub fn scroll_y_mut(&mut self) -> &mut usize {
        &mut self.scroll_y
    }
    pub fn scroll_x(&self) -> usize {
        self.scroll_x
    }
    pub fn scroll_x_mut(&mut self) -> &mut usize {
        &mut self.scroll_x
    }
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }
    pub fn visual_start(&self) -> Option<(usize, usize)> {
        self.visual_start
    }
    pub fn visual_start_mut(&mut self) -> &mut Option<(usize, usize)> {
        &mut self.visual_start
    }
    pub fn matching_bracket(&self) -> Option<(usize, usize)> {
        self.matching_bracket
    }
    pub fn matching_bracket_mut(&mut self) -> &mut Option<(usize, usize)> {
        &mut self.matching_bracket
    }
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
        
        Self {
            buffer,
            cursor_x: 0,
            cursor_y: 0,
            scroll_y: 0,
            scroll_x: 0,
            filename,
            visual_start: None,
            yanked_text: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            insert_mode_start_state: None,
            needs_syntax_update: true,
            last_modified_line: None,
            matching_bracket: None,
        }
    }

    pub fn save_file(&mut self) -> io::Result<()> {
        if let Some(filename) = &self.filename {
            let mut file = fs::File::create(filename)?;
            for line in &self.buffer {
                writeln!(file, "{}", line)?;
            }
            // 括弧の状態更新は不要になったため削除
            Ok(())
        } else {
            Err(io::Error::other("No file name"))
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
                    
                    // 括弧の状態更新は不要になったため削除
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            Err(io::Error::other("No file name to reload"))
        }
    }

    // update_unmatched_brackets 関数は不要になったため削除

    /// 特定の行が変更されたときに呼び出す
    pub fn mark_line_modified(&mut self, line_index: usize) {
        self.last_modified_line = Some(line_index);
        self.needs_syntax_update = true;
        // 括弧の状態更新は不要になったため削除
    }

    /// 文字が挿入されたときに呼び出す
    pub fn on_char_inserted(&mut self, line_index: usize, _char_index: usize, _ch: char) {
        self.mark_line_modified(line_index);
    }

    /// 文字が削除されたときに呼び出す
    pub fn on_char_deleted(&mut self, line_index: usize, _char_index: usize, _ch: char) {
        self.mark_line_modified(line_index);
    }

    /// 行が挿入されたときに呼び出す
    pub fn on_line_inserted(&mut self, line_index: usize) {
        self.mark_line_modified(line_index);
        // 括弧の状態更新は不要になったため削除
    }

    /// 行が削除されたときに呼び出す
    pub fn on_line_deleted(&mut self, line_index: usize) {
        self.mark_line_modified(line_index);
        // 括弧の状態更新は不要になったため削除
    }

    /// シンタックスハイライトの更新完了をマーク
    pub fn mark_syntax_updated(&mut self) {
        self.needs_syntax_update = false;
        self.last_modified_line = None;
    }

    pub fn find_matching_bracket(&mut self) {
        self.matching_bracket = None;
        if self.cursor_y >= self.buffer.len() || self.cursor_x >= self.buffer[self.cursor_y].len() {
            return;
        }

        let ch = self.buffer[self.cursor_y].chars().nth(self.cursor_x).unwrap();
        let (open_bracket, close_bracket) = match ch {
            '(' => ('(', ')'),
            ')' => ('(', ')'),
            '[' => ('[', ']'),
            ']' => ('[', ']'),
            '{' => ('{', '}'),
            '}' => ('{', '}'),
            _ => return,
        };

        let is_forward = ch == open_bracket;
        let mut stack = Vec::new();
        let current_y = self.cursor_y;

        if is_forward {
            for y in current_y..self.buffer.len() {
                let line = &self.buffer[y];
                let start_x = if y == current_y { self.cursor_x } else { 0 };
                for (x, c) in line.chars().enumerate().skip(start_x) {
                    if c == open_bracket {
                        stack.push(c);
                    } else if c == close_bracket {
                        stack.pop();
                        if stack.is_empty() {
                            self.matching_bracket = Some((x, y));
                            return;
                        }
                    }
                }
            }
        } else {
            for y in (0..=current_y).rev() {
                let line = &self.buffer[y];
                let end_x = if y == current_y { self.cursor_x + 1 } else { line.len() };
                let line_chars: Vec<(usize, char)> = line.chars().enumerate().take(end_x).collect();
                for (x, c) in line_chars.into_iter().rev() {
                    if c == close_bracket {
                        stack.push(c);
                    } else if c == open_bracket {
                        stack.pop();
                        if stack.is_empty() {
                            self.matching_bracket = Some((x, y));
                            return;
                        }
                    }
                }
            }
        }
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
            
            true
        } else {
            false
        }
    }

    pub fn scroll_to_cursor(&mut self, height: usize, width: usize, show_line_numbers: bool) {
        // Vertical scroll - 基本的なスクロール処理
        if self.cursor_y < self.scroll_y {
            self.scroll_y = self.cursor_y;
        } else if self.cursor_y >= self.scroll_y + height {
            self.scroll_y = self.cursor_y - height + 1;
        }

        // Horizontal scroll - 基本的なスクロール処理
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
    pub directory_scroll_offset: usize,
    pub show_directory: bool,
    pub config: Config,
    pub show_completion: bool,
    pub completions: Vec<String>,
    pub selected_completion: usize,
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
            directory_scroll_offset: 0,
            show_directory: true,
            config,
            show_completion: false,
            completions: Vec::new(),
            selected_completion: 0,
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
                    if (10..=100).contains(&val) {
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
            "directory_pane_floating" | "dirfloat" => {
                match value.to_lowercase().as_str() {
                    "true" | "1" | "on" | "yes" => {
                        self.config.ui.directory_pane_floating = true;
                        App::save_config(&self.config);
                        self.status_message = "Directory pane floating enabled".to_string();
                    }
                    "false" | "0" | "off" | "no" => {
                        self.config.ui.directory_pane_floating = false;
                        App::save_config(&self.config);
                        self.status_message = "Directory pane floating disabled".to_string();
                    }
                    _ => {
                        self.status_message = "Invalid value for directory_pane_floating (use true/false)".to_string();
                    }
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
        self.directory_scroll_offset = 0;
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

    pub fn current_window_mut(&mut self) -> &mut Window {
        let index = self.get_active_window_index();
        &mut self.windows[index]
    }

    pub fn current_window(&self) -> &Window {
        let index = self.get_active_window_index();
        &self.windows[index]
    }

    pub fn set_yanked_text(&mut self, text: String) {
        self.current_window_mut().yanked_text = text.clone();
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

    pub fn update_completions(&mut self) {
        let (start, end) = self.get_current_word_bounds();
        let window = self.current_window();
        let current_line = &window.buffer[window.cursor_y];
        let current_word = &current_line[start..end];

        if current_word.is_empty() {
            self.show_completion = false;
            return;
        }

        let mut completions = std::collections::HashSet::new();
        for line in &window.buffer {
            for word in line.split(|c: char| !c.is_alphanumeric() && c != '_') {
                if word.starts_with(current_word) && word != current_word {
                    completions.insert(word.to_string());
                }
            }
        }

        self.completions = completions.into_iter().collect();
        self.completions.sort();

        if self.completions.is_empty() {
            self.show_completion = false;
        } else {
            self.show_completion = true;
            self.selected_completion = 0;
        }
    }

    pub fn apply_completion(&mut self) {
        if self.show_completion && !self.completions.is_empty() {
            let completion = &self.completions[self.selected_completion].clone();
            let (start, end) = self.get_current_word_bounds();
            let window = self.current_window_mut();
            let line = &mut window.buffer[window.cursor_y];
            line.replace_range(start..end, completion);
            window.cursor_x = start + completion.len();
            self.show_completion = false;
        }
    }

    fn get_current_word_bounds(&self) -> (usize, usize) {
        let window = self.current_window();
        let line = &window.buffer[window.cursor_y];
        let cursor_x = window.cursor_x;

        let start = line[..cursor_x]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map_or(0, |i| i + 1);

        let end = line[cursor_x..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map_or(line.len(), |i| cursor_x + i);

        (start, end)
    }

    pub fn move_directory_selection_up(&mut self, visible_height: usize) {
        if self.selected_directory_index > 0 {
            self.selected_directory_index -= 1;
            self.update_directory_scroll(visible_height);
        }
    }

    pub fn move_directory_selection_down(&mut self, visible_height: usize) {
        if self.directory_files.is_empty() {
            return;
        }
        let last_index = self.directory_files.len().saturating_sub(1);
        let relative = self.selected_directory_index - self.directory_scroll_offset;
        if self.selected_directory_index < last_index {
            if relative < visible_height - 1 {
                // まだウィンドウの一番下でなければインデックスのみ進める
                self.selected_directory_index += 1;
            } else if self.directory_scroll_offset + visible_height <= last_index {
                // 一番下に到達したときだけスクロール。インデックスはそのまま
                self.directory_scroll_offset += 1;
            }
        }
    }

    pub fn update_directory_scroll(&mut self, visible_height: usize) {
        // リスト全体がウィンドウ内に収まる場合はスクロール不要
        if self.directory_files.len() <= visible_height {
            self.directory_scroll_offset = 0;
            return;
        }
        // 選択項目が表示範囲の上にある場合
        if self.selected_directory_index < self.directory_scroll_offset {
            self.directory_scroll_offset = self.selected_directory_index;
        }
        // 選択項目が表示範囲の下にある場合
        else if self.selected_directory_index >= self.directory_scroll_offset + visible_height {
            self.directory_scroll_offset = self.selected_directory_index.saturating_sub(visible_height - 1);
        }
    }

}