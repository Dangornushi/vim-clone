use crate::app::App;
use crate::app::Mode;
use crossterm::event::KeyCode;
use unicode_segmentation::UnicodeSegmentation;

pub fn handle_insert_mode_event(app: &mut App, key_code: KeyCode) {
    if app.show_completion {
        match key_code {
            KeyCode::Tab | KeyCode::Enter => {
                app.apply_completion();
                return;
            }
            KeyCode::Up => {
                if !app.completions.is_empty() {
                    app.selected_completion = app.selected_completion.saturating_sub(1);
                }
                return;
            }
            KeyCode::Down => {
                if !app.completions.is_empty() {
                    app.selected_completion = (app.selected_completion + 1).min(app.completions.len() - 1);
                }
                return;
            }
            KeyCode::Esc => {
                app.show_completion = false;
                return;
            }
            _ => {}
        }
    }

    let indent_width = app.config.editor.indent_width;
    let tab_size = app.config.editor.tab_size;
    let _show_line_numbers = app.config.editor.show_line_numbers;
    let current_window = app.current_window_mut();
    match key_code {
        KeyCode::Char(c) => {
            if c == '\n' || c == '\r' {
                // 改行処理
                let y = current_window.cursor_y();
                let x = current_window.cursor_x();
                let current_line_ref = &mut current_window.buffer_mut()[y];
                let byte_index = current_line_ref
                    .grapheme_indices(true)
                    .nth(x)
                    .map(|(i, _)| i)
                    .unwrap_or(current_line_ref.len());
                let new_line = current_line_ref.split_off(byte_index);

                // 前の行の先頭のスペースを取得
                let mut indent = current_line_ref.chars()
                    .take_while(|&ch| ch == ' ')
                    .collect::<String>();

                // カーソル位置の直前の文字を取得
                let _char_before_cursor = current_line_ref.graphemes(true).nth(x.saturating_sub(1));

                // 前の行の末尾が開き括弧の場合、インデントを深くする
                if current_line_ref.ends_with('{') || current_line_ref.ends_with('[') || current_line_ref.ends_with('(') {
                    let indent_spaces = " ".repeat(indent_width);
                    indent.push_str(&indent_spaces);
                } else if new_line.starts_with('}') || new_line.starts_with(')') || new_line.starts_with(']') {
                    // 新しい行の先頭が閉じ括弧の場合、インデントを一段浅くする
                    if indent.len() >= indent_width {
                        indent.truncate(indent.len() - indent_width);
                    }
                }

                let indented_new_line = format!("{}{}", indent, new_line);
                current_window.buffer_mut().insert(y + 1, indented_new_line);
                *current_window.cursor_y_mut() += 1;
                *current_window.cursor_x_mut() = indent.len();
                current_window.on_line_inserted(current_window.cursor_y());
                // スクロール処理を即座に実行
            } else {
                // 通常の文字挿入
                let y = current_window.cursor_y();
                let x = current_window.cursor_x();
                let line = &mut current_window.buffer_mut()[y];
                let byte_index = line.grapheme_indices(true).nth(x).map(|(i, _)| i).unwrap_or(line.len());
                line.insert(byte_index, c);
                *current_window.cursor_x_mut() += 1;
                current_window.on_char_inserted(y, x, c);
            }
        }
        KeyCode::Backspace => {
            let y = current_window.cursor_y();
            let x = current_window.cursor_x();
            if x > 0 {
                let line = &mut current_window.buffer_mut()[y];
                let prev_grapheme = line.grapheme_indices(true).nth(x - 1).map(|(i, _)| i).unwrap_or(0);
                let removed = line[prev_grapheme..].chars().next().unwrap_or('\0');
                line.drain(prev_grapheme..prev_grapheme + removed.len_utf8());
                *current_window.cursor_x_mut() -= 1;
                current_window.on_char_deleted(y, x - 1, removed);
            } else if y > 0 {
                // 行頭なら前の行と結合
                let prev_line_len = current_window.buffer_mut()[y - 1].graphemes(true).count();
                let current_line = current_window.buffer_mut().remove(y);
                let prev_line = &mut current_window.buffer_mut()[y - 1];
                prev_line.push_str(&current_line);
                *current_window.cursor_y_mut() -= 1;
                *current_window.cursor_x_mut() = prev_line_len;
                current_window.on_line_deleted(y);
            }
        }
        KeyCode::Esc => {
            current_window.end_insert_mode();
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}