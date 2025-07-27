use std::{
    fs,
    io::{self, Write},
};

// Define the editor modes
#[derive(Copy, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    Command,
    Visual,
    RightPanelInput,
}

#[derive(Clone)]
pub struct WindowState {
    pub buffer: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
}

pub struct Window {
    buffer: Vec<String>,
    cursor_x: usize,
    cursor_y: usize,
    scroll_y: usize,
    scroll_x: usize,
    filename: Option<String>,
    visual_start: Option<(usize, usize)>,
    pub yanked_text: String,
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
    pub fn scroll_x(&self) -> usize {
        self.scroll_x
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
            Ok(())
        } else {
            Err(io::Error::other("No file name"))
        }
    }

    pub fn reload_file(&mut self) -> io::Result<()> {
        if let Some(filename) = &self.filename {
            match fs::read_to_string(filename) {
                Ok(content) => {
                    self.buffer = if content.is_empty() {
                        vec![String::new()]
                    } else {
                        content.lines().map(String::from).collect()
                    };
                    
                    if self.cursor_y >= self.buffer.len() {
                        self.cursor_y = self.buffer.len().saturating_sub(1);
                    }
                    
                    let current_line_len = self.buffer.get(self.cursor_y).map_or(0, |line| line.len());
                    if self.cursor_x > current_line_len {
                        self.cursor_x = current_line_len;
                    }
                    
                    if self.scroll_y >= self.buffer.len() {
                        self.scroll_y = self.buffer.len().saturating_sub(1);
                    }
                    
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            Err(io::Error::other("No file name to reload"))
        }
    }

    pub fn mark_line_modified(&mut self, line_index: usize) {
        self.last_modified_line = Some(line_index);
        self.needs_syntax_update = true;
    }

    pub fn on_char_inserted(&mut self, line_index: usize, _char_index: usize, _ch: char) {
        self.mark_line_modified(line_index);
    }

    pub fn on_char_deleted(&mut self, line_index: usize, _char_index: usize, _ch: char) {
        self.mark_line_modified(line_index);
    }

    pub fn on_line_inserted(&mut self, line_index: usize) {
        self.mark_line_modified(line_index);
    }

    pub fn on_line_deleted(&mut self, line_index: usize) {
        self.mark_line_modified(line_index);
    }

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
        
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        
        self.redo_stack.clear();
    }

    pub fn start_insert_mode(&mut self) {
        self.insert_mode_start_state = Some(WindowState {
            buffer: self.buffer.clone(),
            cursor_x: self.cursor_x,
            cursor_y: self.cursor_y,
        });
    }

    pub fn end_insert_mode(&mut self) {
        if let Some(start_state) = self.insert_mode_start_state.take() {
            self.undo_stack.push(start_state);
            
            if self.undo_stack.len() > 100 {
                self.undo_stack.remove(0);
            }
            
            self.redo_stack.clear();
        }
    }

    pub fn undo(&mut self) -> bool {
        if let Some(state) = self.undo_stack.pop() {
            let current_state = WindowState {
                buffer: self.buffer.clone(),
                cursor_x: self.cursor_x,
                cursor_y: self.cursor_y,
            };
            self.redo_stack.push(current_state);
            
            self.buffer = state.buffer;
            self.cursor_x = state.cursor_x;
            self.cursor_y = state.cursor_y;
            
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
            let current_state = WindowState {
                buffer: self.buffer.clone(),
                cursor_x: self.cursor_x,
                cursor_y: self.cursor_y,
            };
            self.undo_stack.push(current_state);
            
            self.buffer = state.buffer;
            self.cursor_x = state.cursor_x;
            self.cursor_y = state.cursor_y;
            
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
        if self.cursor_y < self.scroll_y {
            self.scroll_y = self.cursor_y;
        } else if self.cursor_y >= self.scroll_y + height {
            self.scroll_y = self.cursor_y - height + 1;
        }

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
    pub fn move_to_screen_top(&mut self) {
        self.cursor_y = self.scroll_y;
        if self.cursor_y < self.buffer.len() {
            let line_len = self.buffer[self.cursor_y].len();
            if self.cursor_x > line_len {
                self.cursor_x = line_len;
            }
        }
    }

    pub fn move_to_screen_bottom(&mut self, visible_height: usize) {
        let last_visible_line = (self.scroll_y + visible_height.saturating_sub(1))
            .min(self.buffer.len().saturating_sub(1));
        self.cursor_y = last_visible_line;
        if self.cursor_y < self.buffer.len() {
            let line_len = self.buffer[self.cursor_y].len();
            if self.cursor_x > line_len {
                self.cursor_x = line_len;
            }
        }
    }
}
