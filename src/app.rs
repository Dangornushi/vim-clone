use std::{
    env,
    fs,
    path::PathBuf,
};
use arboard::Clipboard;
use crate::{
    pane::PaneManager,
    config::Config,
    window::Window,
    app_config::{ConfigManager, AppConfigManager},
};

// Re-export for other modules
pub use crate::window::Mode;

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
    pub show_right_panel: bool,
    pub right_panel_input: String,
    pub right_panel_items: Vec<String>,
    pub selected_right_panel_index: usize,
    pub right_panel_scroll_offset: usize,
    pub focused_panel: FocusedPanel,
}

#[derive(Clone, PartialEq, Debug)]
pub enum FocusedPanel {
    Editor,
    Directory,
    RightPanel,
}

impl App {
    pub fn new(filename: Option<String>) -> Self {
        let config = AppConfigManager::load_config();
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
            show_right_panel: false,
            right_panel_input: String::new(),
            right_panel_items: vec![
                "Item 1".to_string(),
                "Item 2".to_string(),
                "Item 3".to_string(),
                "Example Task".to_string(),
                "Another Task".to_string(),
                "Development Notes".to_string(),
                "Bug Reports".to_string(),
                "Feature Requests".to_string(),
            ],
            selected_right_panel_index: 0,
            right_panel_scroll_offset: 0,
            focused_panel: FocusedPanel::Directory,
        };
        app.update_directory_files();
        app
    }

    pub fn reload_config(&mut self) -> Result<(), String> {
        let mut config_manager = AppConfigManager::new();
        config_manager.config = self.config.clone();
        match config_manager.reload_config() {
            Ok(()) => {
                self.config = config_manager.config;
                Ok(())
            }
            Err(e) => Err(e)
        }
    }

    pub fn show_current_config(&mut self) {
        let mut config_manager = AppConfigManager::new();
        config_manager.config = self.config.clone();
        config_manager.show_current_config();
        self.status_message = config_manager.status_message;
    }

    pub fn reset_config_to_default(&mut self) {
        let mut config_manager = AppConfigManager::new();
        config_manager.reset_config_to_default();
        self.config = config_manager.config;
        self.status_message = config_manager.status_message;
    }

    pub fn set_config_value(&mut self, key: &str, value: &str) {
        let mut config_manager = AppConfigManager::new();
        config_manager.config = self.config.clone();
        config_manager.set_config_value(key, value);
        self.config = config_manager.config;
        self.status_message = config_manager.status_message;
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
                let window_index = if let Some(index) = self.windows.iter().position(|w| w.filename() == Some(&file_path_str)) {
                    index
                } else {
                    let new_window = Window::new(Some(file_path_str));
                    self.windows.push(new_window);
                    self.windows.len() - 1
                };
                
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
                let window_index = if let Some(index) = self.windows.iter().position(|w| w.filename() == Some(&file_path_str)) {
                    index
                } else {
                    let new_window = Window::new(Some(file_path_str));
                    self.windows.push(new_window);
                    self.windows.len() - 1
                };

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
                let window_index = if let Some(index) = self.windows.iter().position(|w| w.filename() == Some(&file_path_str)) {
                    index
                } else {
                    let new_window = Window::new(Some(file_path_str));
                    self.windows.push(new_window);
                    self.windows.len() - 1
                };

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
            0
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
            PathBuf::from(filename)
        } else {
            self.current_path.join(filename)
        };

        let file_path_str = file_path.to_string_lossy().to_string();
        
        if let Some(window_index) = self.windows.iter().position(|w| {
            w.filename() == Some(&file_path_str)
        }) {
            let active_pane_id = self.pane_manager.get_active_pane_id();
            if let Some(pane) = self.pane_manager.get_pane_mut(active_pane_id) {
                pane.window_index = window_index;
            }
            self.status_message = format!("Switched to \"{}\"", filename);
        } else {
            let new_window = Window::new(Some(file_path_str.clone()));
            
            if file_path.exists() {
                self.windows.push(new_window);
                let window_index = self.windows.len() - 1;
                
                let active_pane_id = self.pane_manager.get_active_pane_id();
                if let Some(pane) = self.pane_manager.get_pane_mut(active_pane_id) {
                    pane.window_index = window_index;
                }
                self.status_message = format!("\"{}\" opened", filename);
            } else {
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
        let current_line = &window.buffer()[window.cursor_y()];
        let current_word = &current_line[start..end];

        if current_word.is_empty() {
            self.show_completion = false;
            return;
        }

        let mut completions = std::collections::HashSet::new();
        for line in window.buffer() {
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
            let completion = self.completions[self.selected_completion].clone();
            let (start, end) = self.get_current_word_bounds();
            let window = self.current_window_mut();
            let cursor_y = window.cursor_y();
            let line = &mut window.buffer_mut()[cursor_y];
            line.replace_range(start..end, &completion);
            *window.cursor_x_mut() = start + completion.len();
            self.show_completion = false;
        }
    }

    fn get_current_word_bounds(&self) -> (usize, usize) {
        let window = self.current_window();
        let line = &window.buffer()[window.cursor_y()];
        let cursor_x = window.cursor_x();

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
        let relative = self.selected_directory_index.saturating_sub(self.directory_scroll_offset);
        if self.selected_directory_index < last_index {
            if relative < visible_height.saturating_sub(1) {
                self.selected_directory_index += 1;
            } else if self.directory_scroll_offset + visible_height <= last_index {
                self.directory_scroll_offset += 1;
            }
        }
    }

    pub fn update_directory_scroll(&mut self, visible_height: usize) {
        if self.directory_files.len() <= visible_height {
            self.directory_scroll_offset = 0;
            return;
        }
        if self.selected_directory_index < self.directory_scroll_offset {
            self.directory_scroll_offset = self.selected_directory_index;
        }
        else if self.selected_directory_index >= self.directory_scroll_offset + visible_height {
            self.directory_scroll_offset = self.selected_directory_index.saturating_sub(visible_height.saturating_sub(1));
        }
    }

    pub fn move_right_panel_selection_up(&mut self, visible_height: usize) {
        if self.selected_right_panel_index > 0 {
            self.selected_right_panel_index -= 1;
            self.update_right_panel_scroll(visible_height);
        }
    }

    pub fn move_right_panel_selection_down(&mut self, visible_height: usize) {
        if self.right_panel_items.is_empty() {
            return;
        }
        let last_index = self.right_panel_items.len().saturating_sub(1);
        if self.selected_right_panel_index < last_index {
            self.selected_right_panel_index += 1;
            self.update_right_panel_scroll(visible_height);
        }
    }

    pub fn update_right_panel_scroll(&mut self, visible_height: usize) {
        if self.right_panel_items.len() <= visible_height {
            self.right_panel_scroll_offset = 0;
            return;
        }
        if self.selected_right_panel_index < self.right_panel_scroll_offset {
            self.right_panel_scroll_offset = self.selected_right_panel_index;
        }
        else if self.selected_right_panel_index >= self.right_panel_scroll_offset + visible_height {
            self.right_panel_scroll_offset = self.selected_right_panel_index.saturating_sub(visible_height.saturating_sub(1));
        }
    }

    pub fn add_right_panel_item(&mut self, item: String) {
        self.right_panel_items.push(item);
    }

    pub fn remove_selected_right_panel_item(&mut self) {
        if !self.right_panel_items.is_empty() && self.selected_right_panel_index < self.right_panel_items.len() {
            self.right_panel_items.remove(self.selected_right_panel_index);
            if self.selected_right_panel_index >= self.right_panel_items.len() && !self.right_panel_items.is_empty() {
                self.selected_right_panel_index = self.right_panel_items.len() - 1;
            }
        }
    }
}