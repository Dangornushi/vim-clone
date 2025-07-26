mod command;
mod insert;
mod normal;
mod visual;
mod right_panel_input;

use crate::App;
use crate::app::Mode;
use crossterm::{
    cursor::SetCursorStyle,
    event::{self, Event, KeyEventKind, KeyCode, KeyModifiers},
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

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                // パネル切り替えの統一処理
                if handle_panel_toggle(&mut app, key.code, key.modifiers) {
                    continue;
                }
                
                // フォーカス切り替えの統一処理
                if handle_focus_cycling(&mut app, key.code) {
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

/// パネルの表示/非表示を切り替える統一処理
fn handle_panel_toggle(app: &mut App, key_code: KeyCode, key_modifiers: KeyModifiers) -> bool {
    use crate::app::FocusedPanel;
    
    match (key_modifiers, key_code) {
        (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
            app.show_directory = !app.show_directory;
            app.focused_panel = if app.show_directory {
                FocusedPanel::Directory
            } else {
                FocusedPanel::Editor
            };
            app.status_message = format!("Directory panel {}", 
                if app.show_directory { "opened" } else { "closed" });
            true
        }
        (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
            app.show_right_panel = !app.show_right_panel;
            app.focused_panel = if app.show_right_panel {
                FocusedPanel::RightPanel
            } else {
                FocusedPanel::Editor
            };
            true
        }
        _ => false,
    }
}

/// フォーカスの循環切り替えを処理
fn handle_focus_cycling(app: &mut App, key_code: KeyCode) -> bool {
    if key_code != KeyCode::Tab {
        return false;
    }
    
    use crate::app::FocusedPanel;
    
    app.focused_panel = match (app.show_directory, app.show_right_panel, &app.focused_panel) {
        (true, true, FocusedPanel::Directory) => FocusedPanel::RightPanel,
        (true, true, FocusedPanel::RightPanel) => FocusedPanel::Editor,
        (true, true, FocusedPanel::Editor) => FocusedPanel::Directory,
        (true, false, FocusedPanel::Directory) => FocusedPanel::Editor,
        (true, false, _) => FocusedPanel::Directory,
        (false, true, FocusedPanel::RightPanel) => FocusedPanel::Editor,
        (false, true, _) => FocusedPanel::RightPanel,
        _ => app.focused_panel.clone(),
    };
    
    true
}