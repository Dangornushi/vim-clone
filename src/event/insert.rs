use crate::app::{App, Mode};
use crossterm::event::KeyCode;
use unicode_segmentation::UnicodeSegmentation;

pub fn handle_insert_mode_event(app: &mut App, key_code: KeyCode) {
    let indent_width = app.config.editor.indent_width;
    let tab_size = app.config.editor.tab_size;
    let current_window = app.current_window();
    match key_code {
        KeyCode::Char(c) => {
            if c == '\n' || c == '\r' {
                // 挿入モード中は状態を保存しない
                let current_line_ref = &mut current_window.buffer[current_window.cursor_y];
                let byte_index = current_line_ref
                    .grapheme_indices(true)
                    .nth(current_window.cursor_x)
                    .map(|(i, _)| i)
                    .unwrap_or(current_line_ref.len());
                let new_line = current_line_ref.split_off(byte_index);
                
                // 前の行の先頭のスペースを取得
                let mut indent = current_line_ref.chars()
                    .take_while(|&ch| ch == ' ')
                    .collect::<String>();

                // カーソル位置の直前の文字を取得
                let _char_before_cursor = current_line_ref.graphemes(true).nth(current_window.cursor_x.saturating_sub(1));

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
                current_window.buffer.insert(current_window.cursor_y + 1, indented_new_line);
                current_window.cursor_y += 1;
                current_window.cursor_x = indent.len();
                current_window.update_unmatched_brackets();
            } else {
                // 挿入モード中は状態を保存しない
                let mut graphemes: Vec<String> =
                    current_window.buffer[current_window.cursor_y].graphemes(true).map(String::from).collect();

                let is_closing_char = matches!(c, ')' | ']' | '}' | '"' | '\'');
                let char_at_cursor = graphemes.get(current_window.cursor_x).map(|s| s.as_str());

                if is_closing_char && char_at_cursor == Some(c.to_string().as_str()) {
                    // If the typed character is a closing char and it matches the char at the cursor,
                    // just move the cursor forward without inserting anything.
                    current_window.cursor_x += 1;
                } else {
                    let opening_char_auto_close = match c {
                        '(' => Some(')'),
                        '[' => Some(']'),
                        '{' => Some('}'),
                        '"' => Some('"'),
                        '\'' => Some('\''),
                        _ => None,
                    };

                    if let Some(closing) = opening_char_auto_close {
                        graphemes.insert(current_window.cursor_x, c.to_string());
                        graphemes.insert(current_window.cursor_x + 1, closing.to_string());
                        current_window.buffer[current_window.cursor_y] = graphemes.join("");
                        current_window.cursor_x += 1;
                        current_window.update_unmatched_brackets();
                    } else {
                        let char_str = c.to_string();
                        graphemes.insert(current_window.cursor_x, char_str);
                        current_window.buffer[current_window.cursor_y] = graphemes.join("");
                        current_window.cursor_x += 1;
                        current_window.update_unmatched_brackets();
                    }
                }
            }
        }
        KeyCode::Backspace => {
            // 挿入モード中は状態を保存しない
            let mut graphemes: Vec<String> = current_window.buffer[current_window.cursor_y].graphemes(true).map(String::from).collect();
            if current_window.cursor_x > 0 {
                let removed_grapheme_index = current_window.cursor_x - 1;
                let removed_grapheme = &graphemes[removed_grapheme_index].clone();

                let closing_grapheme = match removed_grapheme.as_str() {
                    "(" => Some(")"),
                    "[" => Some("]"),
                    "{" => Some("}"),
                    "\"" => Some("\""),
                    "'" => Some("'"),
                    _ => None,
                };

                if let Some(closing) = closing_grapheme {
                    if current_window.cursor_x < graphemes.len() && graphemes[current_window.cursor_x] == closing {
                        graphemes.remove(current_window.cursor_x);
                    }
                }

                current_window.cursor_x -= 1;
                graphemes.remove(removed_grapheme_index);
                current_window.buffer[current_window.cursor_y] = graphemes.join("");
                current_window.update_unmatched_brackets();
            } else if current_window.cursor_y > 0 {
                let prev_line_len_graphemes = current_window.buffer[current_window.cursor_y - 1].graphemes(true).count();
                let current_line_content = current_window.buffer.remove(current_window.cursor_y);
                current_window.buffer[current_window.cursor_y - 1].push_str(&current_line_content);
                current_window.cursor_y -= 1;
                current_window.cursor_x = prev_line_len_graphemes;
                current_window.update_unmatched_brackets();
            }
        }
        KeyCode::Enter => {
            // Enterキーで改行処理
            // 挿入モード中は状態を保存しない
            let current_line_ref = &mut current_window.buffer[current_window.cursor_y];
            let byte_index = current_line_ref
                .grapheme_indices(true)
                .nth(current_window.cursor_x)
                .map(|(i, _)| i)
                .unwrap_or(current_line_ref.len());
            let new_line = current_line_ref.split_off(byte_index);
            
            // 前の行の先頭のスペースを取得
            let mut indent = current_line_ref.chars()
                .take_while(|&ch| ch == ' ')
                .collect::<String>();

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
            current_window.buffer.insert(current_window.cursor_y + 1, indented_new_line);
            current_window.cursor_y += 1;
            current_window.cursor_x = indent.len();
            current_window.update_unmatched_brackets();
        }
        KeyCode::Tab => {
            // 挿入モード中は状態を保存しない
            let mut graphemes: Vec<String> = current_window.buffer[current_window.cursor_y].graphemes(true).map(String::from).collect();
            for _ in 0..tab_size {
                graphemes.insert(current_window.cursor_x, " ".to_string());
                current_window.cursor_x += 1;
            }
            current_window.buffer[current_window.cursor_y] = graphemes.join("");
            current_window.update_unmatched_brackets();
        }
        KeyCode::Esc => {
            let current_window = app.current_window();
            current_window.end_insert_mode(); // 挿入モード終了時に状態を保存
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}