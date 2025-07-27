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
                app.right_panel_input_cursor = 0;
            }
            app.mode = Mode::RightPanelInput;
        }
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.focused_panel = crate::app::FocusedPanel::Editor;
        }
        KeyCode::Backspace => {
            if app.right_panel_input_cursor > 0 {
                use unicode_segmentation::UnicodeSegmentation;
                let graphemes: Vec<&str> = app.right_panel_input.graphemes(true).collect();
                if app.right_panel_input_cursor <= graphemes.len() {
                    // カーソル位置の前の文字を削除
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
        KeyCode::Left => {
            if app.right_panel_input_cursor > 0 {
                app.right_panel_input_cursor -= 1;
            }
        }
        KeyCode::Right => {
            use unicode_segmentation::UnicodeSegmentation;
            let grapheme_count = app.right_panel_input.graphemes(true).count();
            if app.right_panel_input_cursor < grapheme_count {
                app.right_panel_input_cursor += 1;
            }
        }
        KeyCode::Home => {
            app.right_panel_input_cursor = 0;
        }
        KeyCode::End => {
            use unicode_segmentation::UnicodeSegmentation;
            app.right_panel_input_cursor = app.right_panel_input.graphemes(true).count();
        }
        KeyCode::Char('b') => {
            // Ctrl + b でEditorにフォーカスを戻す
            app.focused_panel = crate::app::FocusedPanel::Editor;
        }
        KeyCode::Char(c) => {
            use unicode_segmentation::UnicodeSegmentation;
            // カーソル位置に文字を挿入
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
