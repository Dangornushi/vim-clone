use crate::app::{App, Mode};
use crossterm::event::{KeyCode, KeyEvent};
use unicode_segmentation::UnicodeSegmentation;

pub fn handle_right_panel_input_mode_event(app: &mut App, key_event: KeyEvent) {
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Enter, _) => {
            let input = app.right_panel_input.clone();
            if !input.is_empty() {
                // 入力内容もチャット欄に表示
                app.right_panel_items.push(format!("ユーザー: {}", input));
                app.ai_status = "回答生成中".to_string(); // 送信時に状態変更
                if let Some(sender) = app.ai_response_sender.as_ref() {
                    let sender = sender.clone();
                    tokio::spawn(async move {
                        // ユーザー入力内容をAPIに渡す
                        let reply = match crate::utils::send_gemini_greeting_with_input("config.json", &input).await {
                            Ok(r) => r,
                            Err(e) => format!("Gemini APIエラー: {}", e),
                        };
                        let _ = sender.send(reply).await;
                    });
                }
                app.right_panel_input.clear();
                app.right_panel_input_cursor = 0;
            }
            app.mode = Mode::RightPanelInput;
        }
        (KeyCode::Backspace, _) => {
            if app.right_panel_input_cursor > 0 {
                let graphemes: Vec<&str> = app.right_panel_input.graphemes(true).collect();
                if app.right_panel_input_cursor <= graphemes.len() {
                    let byte_index = app.right_panel_input
                        .grapheme_indices(true)
                        .nth(app.right_panel_input_cursor - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let next_byte_index = app.right_panel_input
                        .grapheme_indices(true)
                        .nth(app.right_panel_input_cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(app.right_panel_input.len());
                    app.right_panel_input.drain(byte_index..next_byte_index);
                    app.right_panel_input_cursor -= 1;
                }
            }
        }
        (KeyCode::Left, _) => {
            if app.right_panel_input_cursor > 0 {
                app.right_panel_input_cursor -= 1;
            }
        }
        (KeyCode::Right, _) => {
            let grapheme_count = app.right_panel_input.graphemes(true).count();
            if app.right_panel_input_cursor < grapheme_count {
                app.right_panel_input_cursor += 1;
            }
        }
        (KeyCode::Home, _) => {
            app.right_panel_input_cursor = 0;
        }
        (KeyCode::End, _) => {
            app.right_panel_input_cursor = app.right_panel_input.graphemes(true).count();
        }
        (KeyCode::Char(c), _) => {
            let byte_index = app.right_panel_input
                .grapheme_indices(true)
                .nth(app.right_panel_input_cursor)
                .map(|(i, _)| i)
                .unwrap_or(app.right_panel_input.len());
            app.right_panel_input.insert(byte_index, c);
            app.right_panel_input_cursor += 1;
        }
        _ => {}
    }
}