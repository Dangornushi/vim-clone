mod command;
mod insert;
mod normal;
mod visual;
mod right_panel_input;

use crate::app::{App, Mode};
use crossterm::{
    cursor::SetCursorStyle,
    event::{self, Event, KeyEventKind, KeyCode, KeyModifiers},
    execute,
};
use ratatui::backend::Backend;
use ratatui::Terminal;
use std::io;

pub async fn run_app<B: Backend + std::io::Write>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> io::Result<()> {
    loop {
        // AIレスポンス受信ポーリング
        if let Some(receiver) = app.ai_response_receiver.as_mut() {
            let mut msgs = Vec::new();
            while let Ok(msg) = receiver.try_recv() {
                msgs.push(msg);
            }
            for msg in msgs {
                let is_error = msg.starts_with("Gemini APIエラー");
                app.add_right_panel_item(msg.clone());
                if is_error {
                    app.ai_status = msg;
                } else {
                    app.ai_status = "完了".to_string();
                }
                app.status_message = "Geminiからの返答を追加しました".to_string();
            }
        }

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

                if key.code == KeyCode::Esc {
                    // どのモードでもEscでノーマルモードに戻る
                    // ただし、特殊な状態（ビジュアルモードなど）のクリーンアップが必要な場合がある
                    if app.mode == Mode::Visual {
                        *app.current_window_mut().visual_start_mut() = None;
                    }
                    if app.mode == Mode::Insert {
                        app.current_window_mut().end_insert_mode();
                    }
                    app.mode = Mode::Normal;
                    continue;
                }

                match app.mode {
                    Mode::Normal => normal::handle_normal_mode_event(&mut app, key.code, key.modifiers),
                    Mode::Insert => insert::handle_insert_mode_event(&mut app, key.code),
                    Mode::Visual => visual::handle_visual_mode_event(&mut app, key.code),
                    // 非同期AIリクエストはbg関数で処理
                    Mode::RightPanelInput => right_panel_input::handle_right_panel_input_mode_event(&mut app, key),
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
    
    match (key_modifiers, key_code) {
        (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
            app.show_directory = !app.show_directory;
            app.focused_panel = if app.show_directory {
                crate::app::FocusedPanel::Directory
            } else {
                crate::app::FocusedPanel::Editor
            };
            app.status_message = format!("Directory panel {}", 
                if app.show_directory { "opened" } else { "closed" });
            true
        }
        (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
            app.show_right_panel = !app.show_right_panel;
            if app.show_right_panel {
                app.focused_panel = crate::app::FocusedPanel::RightPanel;
            } else {
                app.focused_panel = crate::app::FocusedPanel::Editor;
                if app.mode == Mode::RightPanelInput {
                    app.mode = Mode::Normal;
                }
            }
            true
        }
        // Ctrl+h/j/k/l でのパネル間移動（全パネル対応）
        (KeyModifiers::CONTROL, KeyCode::Char('h')) => {
            handle_panel_focus(app, "focus_left_panel");
            true
        }
        (KeyModifiers::CONTROL, KeyCode::Char('j')) => {
            handle_panel_focus(app, "focus_down_panel");
            true
        }
        (KeyModifiers::CONTROL, KeyCode::Char('k')) => {
            handle_panel_focus(app, "focus_up_panel");
            true
        }
        (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
            handle_panel_focus(app, "focus_right_panel");
            true
        }
        _ => false,
    }
}

/// パネルフォーカス処理
fn handle_panel_focus(app: &mut App, action: &str) {
    
    match action {
        "focus_left_panel" => {
            app.move_to_next_left_panel();
        }
        "focus_right_panel" => {
            app.move_to_next_right_panel();
        }
        "focus_up_panel" => {
            app.move_to_next_up_panel();
        }
        "focus_down_panel" => {
            app.move_to_next_down_panel();
        }
        _ => {}
    }
}

/// フォーカスの循環切り替えを処理
fn handle_focus_cycling(app: &mut App, key_code: KeyCode) -> bool {
    if key_code != KeyCode::Tab {
        return false;
    }
    
    
    app.focused_panel = match (app.show_directory, app.show_right_panel, &app.focused_panel) {
        (true, true, crate::app::FocusedPanel::Directory) => crate::app::FocusedPanel::RightPanel,
        (true, true, crate::app::FocusedPanel::RightPanel) => crate::app::FocusedPanel::Editor,
        (true, true, crate::app::FocusedPanel::Editor) => crate::app::FocusedPanel::Directory,
        (true, false, crate::app::FocusedPanel::Directory) => crate::app::FocusedPanel::Editor,
        (true, false, _) => crate::app::FocusedPanel::Directory,
        (false, true, crate::app::FocusedPanel::RightPanel) => crate::app::FocusedPanel::Editor,
        (false, true, _) => crate::app::FocusedPanel::RightPanel,
        _ => app.focused_panel.clone(),
    };
    
    true
}