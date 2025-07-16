use crate::app::{App, Mode};
use crossterm::event::KeyCode;
use unicode_segmentation::UnicodeSegmentation;

pub fn handle_visual_mode_event(app: &mut App, key_code: KeyCode) {
    let current_window = app.current_window();
    match key_code {
        KeyCode::Char('h') => {
            if current_window.cursor_x > 0 {
                current_window.cursor_x -= 1;
            }
        }
        KeyCode::Char('j') => {
            if current_window.cursor_y < current_window.buffer.len() - 1 {
                current_window.cursor_y += 1;
                let current_line_len_graphemes = current_window.buffer[current_window.cursor_y].graphemes(true).count();
                current_window.cursor_x = current_window.cursor_x.min(current_line_len_graphemes);
            }
        }
        KeyCode::Char('k') => {
            if current_window.cursor_y > 0 {
                current_window.cursor_y -= 1;
                let current_line_len_graphemes = current_window.buffer[current_window.cursor_y].graphemes(true).count();
                current_window.cursor_x = current_window.cursor_x.min(current_line_len_graphemes);
            }
        }
        KeyCode::Char('l') => {
            let current_line = &current_window.buffer[current_window.cursor_y];
            let grapheme_count = current_line.graphemes(true).count();
            if current_window.cursor_x < grapheme_count.saturating_sub(1) {
                current_window.cursor_x += 1;
            }
        }
        KeyCode::Char('d') | KeyCode::Char('y') => {
            let mut yanked_text = String::new();
            let new_mode = Mode::Normal; // 新しいモードを保持する変数

            if let Some(start) = current_window.visual_start {
                if key_code == KeyCode::Char('d') {
                    current_window.save_state(); // 削除前の状態を保存
                }
                let (start_x, start_y) = start;
                let (end_x, end_y) = (current_window.cursor_x, current_window.cursor_y);

                // Normalize selection direction
                let ((sel_start_y, sel_start_x), (sel_end_y, sel_end_x)) =
                    if (start_y, start_x) <= (end_y, end_x) {
                        ((start_y, start_x), (end_y, end_x))
                    } else {
                        ((end_y, end_x), (start_y, start_x))
                    };

                if sel_start_y == sel_end_y {
                    // Single line
                    let line = &current_window.buffer[sel_start_y];
                    let start_byte = line.grapheme_indices(true).nth(sel_start_x).map(|(i, _)| i).unwrap_or(line.len());
                    let end_byte = line.grapheme_indices(true).nth(sel_end_x + 1).map(|(i, _)| i).unwrap_or(line.len());
                    if start_byte < end_byte {
                        yanked_text.push_str(&line[start_byte..end_byte]);
                    }
                } else {
                    // Multi-line
                    let start_byte = current_window.buffer[sel_start_y].grapheme_indices(true).nth(sel_start_x).map(|(i, _)| i).unwrap_or(current_window.buffer[sel_start_y].len());
                    yanked_text.push_str(&current_window.buffer[sel_start_y][start_byte..]);
                    yanked_text.push('\n');
                    for y in (sel_start_y + 1)..sel_end_y {
                        yanked_text.push_str(&current_window.buffer[y]);
                        yanked_text.push('\n');
                    }
                    let end_line = &current_window.buffer[sel_end_y];
                    let end_byte = end_line.grapheme_indices(true).nth(sel_end_x + 1).map(|(i, _)| i).unwrap_or(end_line.len());
                    yanked_text.push_str(&end_line[..end_byte]);
                }

                if key_code == KeyCode::Char('d') {
                    if sel_start_y == sel_end_y {
                        // Single line deletion
                        let line = &mut current_window.buffer[sel_start_y];
                        let start_byte = line.grapheme_indices(true).nth(sel_start_x).map(|(i, _)| i).unwrap_or(line.len());
                        let end_byte = line.grapheme_indices(true).nth(sel_end_x + 1).map(|(i, _)| i).unwrap_or(line.len());
                        if start_byte < end_byte {
                            line.drain(start_byte..end_byte);
                        }
                    } else {
                        // Multi-line deletion
                        let end_line = &current_window.buffer[sel_end_y];
                        let split_point_byte = end_line.grapheme_indices(true).nth(sel_end_x + 1).map(|(i, _)| i).unwrap_or(end_line.len());
                        let end_line_suffix = end_line[split_point_byte..].to_string();

                        let start_byte = current_window.buffer[sel_start_y].grapheme_indices(true).nth(sel_start_x).map(|(i, _)| i).unwrap_or(current_window.buffer[sel_start_y].len());
                        current_window.buffer[sel_start_y].truncate(start_byte);
                        current_window.buffer[sel_start_y].push_str(&end_line_suffix);

                        let start_of_removal = sel_start_y + 1;
                        if start_of_removal <= sel_end_y {
                            current_window.buffer.drain(start_of_removal..=sel_end_y);
                        }
                    }
                }

                // Set cursor position
                current_window.cursor_x = sel_start_x;
                current_window.cursor_y = sel_start_y;

                if current_window.buffer.is_empty() {
                    current_window.buffer.push(String::new());
                    current_window.cursor_y = 0;
                    current_window.cursor_x = 0;
                } else {
                    if current_window.cursor_y >= current_window.buffer.len() {
                        current_window.cursor_y = current_window.buffer.len().saturating_sub(1);
                    }
                    if current_window.cursor_x > current_window.buffer[current_window.cursor_y].graphemes(true).count() {
                        current_window.cursor_x = current_window.buffer[current_window.cursor_y].graphemes(true).count();
                    }
                }
                current_window.visual_start = None;
            }
            app.set_yanked_text(yanked_text);
            app.mode = new_mode;
        }
        KeyCode::Esc => {
            current_window.visual_start = None; // visual_startを先にクリア
            app.mode = Mode::Normal; // モード変更を後に
        }
        _ => {}
    }
}