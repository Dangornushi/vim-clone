use crate::app::App;
use crate::app::Mode;
use crossterm::event::KeyCode;

pub fn handle_right_panel_input_mode_event(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Enter => {
            // 入力欄からアイテムを追加
            if !app.right_panel_input.is_empty() {
                app.add_right_panel_item(app.right_panel_input.clone());
                app.right_panel_input.clear();
                app.status_message = "Item added to right panel".to_string();
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Esc => {
            // 入力モードを終了
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            // 文字を削除
            app.right_panel_input.pop();
        }
        KeyCode::Char(c) => {
            // 文字を追加
            app.right_panel_input.push(c);
        }
        _ => {}
    }
}
