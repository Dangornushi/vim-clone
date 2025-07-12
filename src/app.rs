use std::{
    env,
    fs,
    io::{self, Write},
};
use arboard::Clipboard;

// Define the editor modes
pub enum Mode {
    Normal,
    Insert,
    Command,
    Visual,
}

// App holds the state of the application
pub struct App {
    pub buffer: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub mode: Mode,
    pub filename: Option<String>,
    pub command_buffer: String,
    pub status_message: String,
    pub visual_start: Option<(usize, usize)>,
    pub yanked_text: String,
    clipboard: Clipboard,
}

impl App {
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
            mode: Mode::Normal,
            filename,
            command_buffer: String::new(),
            status_message: String::new(),
            visual_start: None,
            yanked_text: String::new(),
            clipboard: Clipboard::new().unwrap(),
        }
    }

    pub fn save_file(&mut self) -> io::Result<()> {
        if let Some(filename) = &self.filename {
            let mut file = fs::File::create(filename)?;
            for line in &self.buffer {
                writeln!(file, "{}", line)?;
            }
            self.status_message = format!("\"{}\" written", filename);
        } else {
            self.status_message = "No file name".to_string();
        }
        Ok(())
    }

    pub fn set_yanked_text(&mut self, text: String) {
        self.yanked_text = text.clone();
        if let Err(e) = self.clipboard.set_text(text) {
            self.status_message = format!("Failed to set clipboard: {}", e);
        }
    }

    pub fn get_clipboard_text(&mut self) -> Result<String, arboard::Error> {
        self.clipboard.get_text()
    }
}