mod command;
mod insert;
mod normal;
mod visual;
mod right_panel_input;

use crate::App;
use crate::app::Mode;
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
                
                if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('b') {
                    app.show_right_panel = !app.show_right_panel;
                    if app.show_right_panel {
                        app.focused_panel = crate::app::FocusedPanel::RightPanel;
                    } else {
                        app.focused_panel = crate::app::FocusedPanel::Editor;
                    }
                terminal.draw(|f| crate::ui::ui(f, &mut app))?;
                    continue;
                
                }
                
                if key.code == KeyCode::Tab {
                    use crate::app::FocusedPanel;
                    if app.show_directory && app.show_right_panel {
                        app.focused_panel = match app.focused_panel {
                            FocusedPanel::Directory => FocusedPanel::RightPanel,
                            FocusedPanel::RightPanel => FocusedPanel::Editor,
                            FocusedPanel::Editor => FocusedPanel::Directory,
                        };
                    } else if app.show_directory {
                        app.focused_panel = match app.focused_panel {
                            FocusedPanel::Directory => FocusedPanel::Editor,
                            _ => FocusedPanel::Directory,
                        };
                    } else if app.show_right_panel {
                        app.focused_panel = match app.focused_panel {
                            FocusedPanel::RightPanel => FocusedPanel::Editor,
                            _ => FocusedPanel::RightPanel,
                        };
                    }
                    continue;
                }

                match app.mode {
                    Mode::Normal => normal::handle_normal_mode_event(&mut app, key.code, key.modifiers),
                    Mode::Insert => insert::handle_insert_mode_event(&mut app, key.code),
                    Mode::Visual => visual::handle_visual_mode_event(&mut app, key.code),
                    Mode::RightPanelInput => right_panel_input::handle_right_panel_input_mode_event(&mut app, key.code),
                    Mode::Command => {
                        if (command::handle_command_mode_event(&mut app, key.code)?).is_some() {
                            return Ok(());
                        }
                    }
                }
                app.current_window_mut().find_matching_bracket();
            }
        }
    }
}