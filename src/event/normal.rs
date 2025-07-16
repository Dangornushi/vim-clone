use crate::app::{App, Mode};
use crossterm::event::{KeyCode, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;

pub fn handle_normal_mode_event(app: &mut App, key_code: KeyCode, key_modifiers: KeyModifiers) {
    if let KeyCode::Char(c) = key_code {
        if let Some(action) = app.config.key_bindings.normal.get(&c.to_string()) {
            match action.as_str() {
                "move_left" => {
                    if key_modifiers == KeyModifiers::CONTROL {
                        app.activate_left_pane();
                    } else {
                        let current_window = app.current_window();
                        if current_window.cursor_x > 0 {
                            current_window.cursor_x -= 1;
                        }
                    }
                }
                "move_down" => {
                    if key_modifiers == KeyModifiers::CONTROL {
                        app.pane_manager.move_to_down_pane();
                    } else if app.show_directory {
                        if !app.directory_files.is_empty() {
                            app.selected_directory_index = (app.selected_directory_index + 1).min(app.directory_files.len() - 1);
                        }
                    } else {
                        let current_window = app.current_window();
                        if current_window.cursor_y < current_window.buffer.len() - 1 {
                            current_window.cursor_y += 1;
                            let current_line_len_graphemes = current_window.buffer[current_window.cursor_y].graphemes(true).count();
                            current_window.cursor_x = current_window.cursor_x.min(current_line_len_graphemes);
                        }
                    }
                }
                "move_up" => {
                    if key_modifiers == KeyModifiers::CONTROL {
                        app.pane_manager.move_to_up_pane();
                    } else if app.show_directory {
                        if !app.directory_files.is_empty() {
                            app.selected_directory_index = app.selected_directory_index.saturating_sub(1);
                        }
                    } else {
                        let current_window = app.current_window();
                        if current_window.cursor_y > 0 {
                            current_window.cursor_y -= 1;
                            let current_line_len_graphemes = current_window.buffer[current_window.cursor_y].graphemes(true).count();
                            current_window.cursor_x = current_window.cursor_x.min(current_line_len_graphemes);
                        }
                    }
                }
                "move_right" => {
                    if key_modifiers == KeyModifiers::CONTROL {
                        app.activate_right_pane();
                    } else {
                        let current_window = app.current_window();
                        let current_line = &current_window.buffer[current_window.cursor_y];
                        let grapheme_count = current_line.graphemes(true).count();
                        if current_window.cursor_x < grapheme_count.saturating_sub(1) {
                            current_window.cursor_x += 1;
                        }
                    }
                }
                "mode_visual" => {
                    if app.show_directory {
                        app.vsplit_selected_item();
                    } else {
                        let cursor_x = app.current_window().cursor_x;
                        let cursor_y = app.current_window().cursor_y;
                        app.mode = Mode::Visual;
                        app.current_window().visual_start = Some((cursor_x, cursor_y));
                    }
                }
                "hsplit" => {
                    if app.show_directory {
                        app.hsplit_selected_item();
                    }
                }
                "delete_char" => {
                    let current_window = app.current_window();
                    current_window.save_state(); // 変更前の状態を保存
                    let mut graphemes: Vec<String> = current_window.buffer[current_window.cursor_y].graphemes(true).map(String::from).collect();
                    if current_window.cursor_x < graphemes.len() {
                        let deleted_char = graphemes[current_window.cursor_x].chars().next().unwrap_or(' ');
                        graphemes.remove(current_window.cursor_x);
                        current_window.buffer[current_window.cursor_y] = graphemes.join("");
                        if current_window.cursor_x >= graphemes.len() && !graphemes.is_empty() {
                            current_window.cursor_x = graphemes.len().saturating_sub(1);
                        } else if graphemes.is_empty() {
                            current_window.cursor_x = 0;
                        }
                        current_window.on_char_deleted(current_window.cursor_y, current_window.cursor_x, deleted_char);
                    }
                }
                "mode_insert" => {
                    let current_window = app.current_window();
                    current_window.start_insert_mode(); // 挿入モード開始時に状態を保存
                    app.mode = Mode::Insert;
                }
                "append" => {
                    let current_window_ref = app.current_window();
                    let grapheme_count = current_window_ref.buffer[current_window_ref.cursor_y].graphemes(true).count();
                    if current_window_ref.cursor_x < grapheme_count {
                        current_window_ref.cursor_x += 1;
                    }
                    current_window_ref.start_insert_mode(); // 挿入モード開始時に状態を保存
                    app.mode = Mode::Insert;
                }
                "mode_command" => {
                    app.mode = Mode::Command;
                    app.command_buffer.clear();
                }
                "paste" => {
                    let text_to_paste = app.get_clipboard_text();
                    if let Ok(text) = text_to_paste {
                        let current_window = app.current_window();
                        if !text.is_empty() {
                            current_window.save_state(); // 変更前の状態を保存
                            if text.contains('\n') {
                                let mut lines: Vec<String> = text.lines().map(String::from).collect();
                                let current_line_ref = &mut current_window.buffer[current_window.cursor_y];
                                let byte_index = current_line_ref.grapheme_indices(true).nth(current_window.cursor_x).map(|(i, _)| i).unwrap_or(current_line_ref.len());
                                let rest_of_current_line = current_line_ref.split_off(byte_index);
                                current_line_ref.push_str(&lines[0]);
                                let last_line_index = lines.len() - 1;
                                lines[last_line_index].push_str(&rest_of_current_line);
                                for (i, line) in lines.iter().skip(1).enumerate() {
                                    current_window.buffer.insert(current_window.cursor_y + 1 + i, line.clone());
                                    current_window.on_line_inserted(current_window.cursor_y + 1 + i);
                                }
                                current_window.mark_line_modified(current_window.cursor_y);
                            } else {
                                if !current_window.buffer[current_window.cursor_y].is_empty() {
                                    current_window.cursor_x += 1;
                                }
                                let current_line_ref = &mut current_window.buffer[current_window.cursor_y];
                                let byte_index = current_line_ref.grapheme_indices(true).nth(current_window.cursor_x).map(|(i, _)| i).unwrap_or(current_line_ref.len());
                                current_line_ref.insert_str(byte_index, &text);
                                current_window.cursor_x += text.graphemes(true).count();
                                current_window.mark_line_modified(current_window.cursor_y);
                            }
                        }
                    }
                }
                "undo" => {
                    let current_window = app.current_window();
                    if current_window.undo() {
                        app.status_message = "Undone".to_string();
                    } else {
                        app.status_message = "Nothing to undo".to_string();
                    }
                }
                "open_new_line" => {
                    app.status_message = "o key pressed".to_string();
                    let current_window = app.current_window();
                    current_window.open_new_line();
                    current_window.start_insert_mode();
                    app.mode = Mode::Insert;
                }
                _ => {}
            }
        }
    } else if let KeyCode::Enter = key_code {
        if app.show_directory {
            app.open_selected_item();
        }
    } else if key_code == KeyCode::Char('r') && key_modifiers == KeyModifiers::CONTROL {
        // Ctrl+R for redo
        let current_window = app.current_window();
        if current_window.redo() {
            app.status_message = "Redone".to_string();
        } else {
            app.status_message = "Nothing to redo".to_string();
        }
    }
}