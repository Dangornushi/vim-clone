use crate::app::App;
use crate::app::Mode;
use crossterm::event::KeyCode;
use unicode_segmentation::UnicodeSegmentation;

pub fn handle_visual_mode_event(app: &mut App, key_code: KeyCode) {
    let current_window = app.current_window_mut();
    match key_code {
        KeyCode::Char('h') => {
            if current_window.cursor_x() > 0 {
                *current_window.cursor_x_mut() -= 1;
            }
        }
        KeyCode::Char('j') => {
            let y = current_window.cursor_y();
            if y < current_window.buffer_mut().len() - 1 {
                *current_window.cursor_y_mut() += 1;
                let x = current_window.cursor_x();
                let line_len = current_window.buffer_mut()[y + 1].graphemes(true).count();
                *current_window.cursor_x_mut() = x.min(line_len);
            }
        }
        KeyCode::Char('k') => {
            let y = current_window.cursor_y();
            if y > 0 {
                *current_window.cursor_y_mut() -= 1;
                let x = current_window.cursor_x();
                let line_len = current_window.buffer_mut()[y - 1].graphemes(true).count();
                *current_window.cursor_x_mut() = x.min(line_len);
            }
        }
        KeyCode::Char('l') => {
            let y = current_window.cursor_y();
            let current_line = &current_window.buffer_mut()[y];
            let grapheme_count = current_line.graphemes(true).count();
            let x = current_window.cursor_x();
            if x < grapheme_count.saturating_sub(1) {
                *current_window.cursor_x_mut() += 1;
            }
        }
        KeyCode::Char('d') | KeyCode::Char('y') => {
            let mut yanked_text = String::new();
            let new_mode = Mode::Normal; // 新しいモードを保持する変数

            if let Some(start) = current_window.visual_start() {
                if key_code == KeyCode::Char('d') {
                    current_window.save_state(); // 削除前の状態を保存
                }
                let (start_x, start_y) = start;
                let (end_x, end_y) = (current_window.cursor_x(), current_window.cursor_y());

                // Normalize selection direction
                let ((sel_start_y, sel_start_x), (sel_end_y, sel_end_x)) =
                    if (start_y, start_x) <= (end_y, end_x) {
                        ((start_y, start_x), (end_y, end_x))
                    } else {
                        ((end_y, end_x), (start_y, start_x))
                    };

                if sel_start_y == sel_end_y {
                    // Single line
                    let line = &current_window.buffer_mut()[sel_start_y];
                    let start_byte = line.grapheme_indices(true).nth(sel_start_x).map(|(i, _)| i).unwrap_or(line.len());
                    let end_byte = line.grapheme_indices(true).nth(sel_end_x + 1).map(|(i, _)| i).unwrap_or(line.len());
                    if start_byte < end_byte {
                        yanked_text.push_str(&line[start_byte..end_byte]);
                    }
                } else {
                    // Multi-line
                    let start_line = &current_window.buffer_mut()[sel_start_y];
                    let start_byte = start_line.grapheme_indices(true).nth(sel_start_x).map(|(i, _)| i).unwrap_or(start_line.len());
                    yanked_text.push_str(&start_line[start_byte..]);
                    yanked_text.push('\n');
                    for y in (sel_start_y + 1)..sel_end_y {
                        yanked_text.push_str(&current_window.buffer_mut()[y]);
                        yanked_text.push('\n');
                    }
                    let end_line = &current_window.buffer_mut()[sel_end_y];
                    let end_byte = end_line.grapheme_indices(true).nth(sel_end_x + 1).map(|(i, _)| i).unwrap_or(end_line.len());
                    yanked_text.push_str(&end_line[..end_byte]);
                }

                if key_code == KeyCode::Char('d') {
                    if sel_start_y == sel_end_y {
                        // Single line deletion
                        let line = &mut current_window.buffer_mut()[sel_start_y];
                        let start_byte = line.grapheme_indices(true).nth(sel_start_x).map(|(i, _)| i).unwrap_or(line.len());
                        let end_byte = line.grapheme_indices(true).nth(sel_end_x + 1).map(|(i, _)| i).unwrap_or(line.len());
                        if start_byte < end_byte {
                            line.drain(start_byte..end_byte);
                        }
                    } else {
                        // Multi-line deletion
                        let end_line = &current_window.buffer_mut()[sel_end_y];
                        let split_point_byte = end_line.grapheme_indices(true).nth(sel_end_x + 1).map(|(i, _)| i).unwrap_or(end_line.len());
                        let end_line_suffix = end_line[split_point_byte..].to_string();

                        let start_line = &mut current_window.buffer_mut()[sel_start_y];
                        let start_byte = start_line.grapheme_indices(true).nth(sel_start_x).map(|(i, _)| i).unwrap_or(start_line.len());
                        start_line.truncate(start_byte);
                        start_line.push_str(&end_line_suffix);

                        let start_of_removal = sel_start_y + 1;
                        if start_of_removal <= sel_end_y {
                            current_window.buffer_mut().drain(start_of_removal..=sel_end_y);
                        }
                    }
                }

                // Set cursor position
                *current_window.cursor_x_mut() = sel_start_x;
                *current_window.cursor_y_mut() = sel_start_y;

                if current_window.buffer_mut().is_empty() {
                    current_window.buffer_mut().push(String::new());
                    *current_window.cursor_y_mut() = 0;
                    *current_window.cursor_x_mut() = 0;
                } else {
                    let y = current_window.cursor_y();
                    if y >= current_window.buffer_mut().len() {
                        *current_window.cursor_y_mut() = current_window.buffer_mut().len().saturating_sub(1);
                    }
                    let x = current_window.cursor_x();
                    let y2 = current_window.cursor_y();
                    let line_len = current_window.buffer_mut()[y2].graphemes(true).count();
                    if x > line_len {
                        *current_window.cursor_x_mut() = line_len;
                    }
                }
                *current_window.visual_start_mut() = None;
            }
            app.set_yanked_text(yanked_text);
            app.mode = new_mode;
        }
        KeyCode::Esc => {
            *current_window.visual_start_mut() = None; // visual_startを先にクリア
            app.mode = Mode::Normal; // モード変更を後に
        }
        _ => {}
    }
}