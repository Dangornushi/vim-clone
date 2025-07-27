// 入力された文字列に「, Hello!」を付加して返す関数
pub fn chat_greet(input: &str) -> String {
    format!("{}, Hello!", input)
}
use crate::app::App;
use crate::app::Mode;
use crossterm::event::KeyCode;
use vim_editor::utils;


// Gemini APIリクエストをバックグラウンドで実行するための関数
pub fn handle_right_panel_input_mode_event_bg(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Enter => {
            let input = app.right_panel_input.clone();
            if !input.is_empty() {
                // 入力内容もチャット欄に表示
                app.right_panel_items.push(format!("ユーザー: {}", input));
                app.ai_status = "回答生成中".to_string(); // 送信時に状態変更
                if let Some(sender) = app.ai_response_sender.as_ref() {
                    let sender = sender.clone();
                    tokio::spawn(async move {
                        // ユーザー入力内容をAPIに渡す
                        let reply = match utils::send_gemini_greeting_with_input("config.json", &input).await {
                            Ok(r) => r,
                            Err(e) => format!("Gemini APIエラー: {}", e),
                        };
                        let _ = sender.send(reply).await;
                    });
                }
                app.right_panel_input.clear();
            }
            app.mode = Mode::RightPanelInput;
        }
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.focused_panel = crate::app::FocusedPanel::Editor;
        }
        KeyCode::Backspace => {
            app.right_panel_input.pop();
        }
        KeyCode::Char('b') => {
            // Ctrl + b でEditorにフォーカスを戻す
            app.focused_panel = crate::app::FocusedPanel::Editor;
        }
        KeyCode::Char(c) => {
            app.right_panel_input.push(c);
        }
        _ => {}
    }
}
