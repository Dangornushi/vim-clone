mod command;
mod insert;
mod normal;
mod visual;

use crate::app::{App, Mode};
use crossterm::{
    cursor::SetCursorStyle,
    event::{self, Event, KeyEventKind},
    execute,
};
use ratatui::backend::Backend;
use ratatui::Terminal;
use std::io;

pub fn run_app<B: Backend + std::io::Write>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        match app.mode {
            Mode::Insert => {
                execute!(terminal.backend_mut(), SetCursorStyle::SteadyBar)?;
            }
            _ => {
                execute!(terminal.backend_mut(), SetCursorStyle::SteadyBlock)?;
            }
        }
        terminal.draw(|f| crate::ui::ui(f, &mut app))?;

        use crossterm::event::{KeyCode, KeyModifiers};

// ... (rest of the file)

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('f') {
                    app.show_directory = !app.show_directory;
                    continue;
                }

                match app.mode {
                    Mode::Normal => normal::handle_normal_mode_event(&mut app, key.code, key.modifiers),
                    Mode::Insert => insert::handle_insert_mode_event(&mut app, key.code),
                    Mode::Visual => visual::handle_visual_mode_event(&mut app, key.code),
                    Mode::Command => {
                        if (command::handle_command_mode_event(&mut app, key.code)?).is_some() {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}