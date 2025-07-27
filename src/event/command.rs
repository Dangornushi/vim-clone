use crate::app::App;
use crate::app::Mode;
use crossterm::event::KeyCode;
use std::io;

pub fn handle_command_mode_event(app: &mut App, key_code: KeyCode) -> io::Result<Option<()>> {
    match key_code {
        KeyCode::Char(c) => {
            app.command_buffer.push(c);
        }
        KeyCode::Backspace => {
            app.command_buffer.pop();
        }
        KeyCode::Enter => {
            let command = app.command_buffer.trim().to_string();
            match command.as_str() {
                "w" => {
                    let current_window = app.current_window_mut();
                    current_window.save_file()?;
                    app.status_message = format!("\"{}\" written", current_window.filename().as_deref().unwrap_or("Untitled"));
                }
                "q" => {
                    let active_pane_id = app.pane_manager.get_active_pane_id();
                    if !app.pane_manager.close_pane(active_pane_id) {
                        // ルートペインを閉じようとした場合、アプリを終了
                        return Ok(Some(()));
                    }
                }
                "wq" => {
                    let current_window = app.current_window_mut();
                    current_window.save_file()?;
                    app.status_message = format!("\"{}\" written", current_window.filename().as_deref().unwrap_or("Untitled"));
                    return Ok(Some(()));
                }
                "r" | "reload" => {
                    let current_window = app.current_window_mut();
                    match current_window.reload_file() {
                        Ok(()) => {
                            app.status_message = format!("\"{}\" reloaded", current_window.filename().as_deref().unwrap_or("Untitled"));
                        }
                        Err(e) => {
                            app.status_message = format!("Failed to reload file: {}", e);
                        }
                    }
                }
                "e" | "edit" => {
                    // 引数なしの場合は現在のファイルを再読み込み
                    let current_window = app.current_window_mut();
                    match current_window.reload_file() {
                        Ok(()) => {
                            app.status_message = format!("\"{}\" reloaded", current_window.filename().as_deref().unwrap_or("Untitled"));
                        }
                        Err(e) => {
                            app.status_message = format!("Failed to reload file: {}", e);
                        }
                    }
                }
                "config" | "conf" => {
                    // 設定ファイルを再読み込み
                    match app.reload_config() {
                        Ok(()) => {
                            app.status_message = "Configuration reloaded successfully".to_string();
                        }
                        Err(e) => {
                            app.status_message = format!("Failed to reload config: {}", e);
                        }
                    }
                }
                "source" => {
                    // 設定ファイルを再読み込み（vimライクなコマンド）
                    match app.reload_config() {
                        Ok(()) => {
                            app.status_message = "Configuration sourced successfully".to_string();
                        }
                        Err(e) => {
                            app.status_message = format!("Failed to source config: {}", e);
                        }
                    }
                }
                "editconfig" | "econfig" => {
                    // 設定ファイルを編集用に開く
                    app.open_file("config.json");
                }
                "showconfig" | "sconfig" => {
                    // 現在の設定を表示
                    app.show_current_config();
                }
                "resetconfig" | "rconfig" => {
                    // 設定をデフォルトにリセット
                    app.reset_config_to_default();
                }
                cmd if cmd.starts_with("set ") => {
                    // 設定値を変更: :set key=value
                    let setting_part = &cmd[4..]; // "set " を除去
                    if let Some(eq_pos) = setting_part.find('=') {
                        let key = setting_part[..eq_pos].trim().to_string();
                        let value = setting_part[eq_pos + 1..].trim().to_string();
                        app.set_config_value(&key, &value);
                    } else {
                        app.status_message = "Usage: :set key=value".to_string();
                    }
                }
                _ => {
                    // ファイル名が指定された場合の処理
                    if command.starts_with("e ") || command.starts_with("edit ") {
                        let parts: Vec<&str> = command.split_whitespace().collect();
                        if parts.len() >= 2 {
                            let filename = parts[1..].join(" ");
                            app.open_file(&filename);
                        }
                    } else {
                        app.status_message = format!("Not a command: {}", command);
                    }
                }
            }
            app.mode = Mode::Normal;
        }
        _ => {}
    }
    Ok(None)
}