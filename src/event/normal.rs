use crate::app::{App, Mode};
use crossterm::event::{KeyCode, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;

pub fn handle_normal_mode_event(app: &mut App, key_code: KeyCode, key_modifiers: KeyModifiers) {
    let _show_line_numbers = app.config.editor.show_line_numbers;
    if let KeyCode::Char(c) = key_code {
        if let Some(action) = app.config.key_bindings.normal.get(&c.to_string()) {
            let visible_height = if app.show_directory && app.config.ui.directory_pane_floating {
                20
            } else { 0 };
            match action.as_str() {
                "move_left" => {
                    if key_modifiers == KeyModifiers::CONTROL {
                        app.activate_left_pane();
                    } else {
                        let current_window = app.current_window_mut();
                        if *current_window.cursor_x_mut() > 0 {
                            *current_window.cursor_x_mut() -= 1;
                            // スクロール処理を即座に実行
                        }
                    }
                }
                "move_down" => {
                    if key_modifiers == KeyModifiers::CONTROL {
                        app.pane_manager.move_to_down_pane();
                    } else if app.show_directory {
                        app.move_directory_selection_down(visible_height);
                    } else {
                        let current_window = app.current_window_mut();
                        let len = current_window.buffer().len();
                        let cy = *current_window.cursor_y_mut();
                        if cy < len - 1 {
                            *current_window.cursor_y_mut() += 1;
                            let cy2 = *current_window.cursor_y_mut();
                            let current_line_len_graphemes = current_window.buffer()[cy2].graphemes(true).count();
                            let cx = *current_window.cursor_x_mut();
                            *current_window.cursor_x_mut() = cx.min(current_line_len_graphemes);
                            // スクロール処理を即座に実行
                        }
                    }
                }
                "move_up" => {
                    if key_modifiers == KeyModifiers::CONTROL {
                        app.pane_manager.move_to_up_pane();
                    } else if app.show_directory {
                        app.move_directory_selection_up(visible_height);
                    } else {
                        let current_window = app.current_window_mut();
                        let cy = *current_window.cursor_y_mut();
                        if cy > 0 {
                            *current_window.cursor_y_mut() -= 1;
                            let cy2 = *current_window.cursor_y_mut();
                            let current_line_len_graphemes = current_window.buffer()[cy2].graphemes(true).count();
                            let cx = *current_window.cursor_x_mut();
                            *current_window.cursor_x_mut() = cx.min(current_line_len_graphemes);
                            // スクロール処理を即座に実行
                        }
                    }
                }
                "move_right" => {
                    if key_modifiers == KeyModifiers::CONTROL {
                        app.activate_right_pane();
                    } else {
                        let current_window = app.current_window_mut();
                        let cy = *current_window.cursor_y_mut();
                        let current_line = &current_window.buffer()[cy];
                        let grapheme_count = current_line.graphemes(true).count();
                        let cx = *current_window.cursor_x_mut();
                        if cx < grapheme_count.saturating_sub(1) {
                            *current_window.cursor_x_mut() += 1;
                            // スクロール処理を即座に実行
                        }
                    }
                }
                "mode_visual" => {
                    if app.show_directory {
                        app.vsplit_selected_item();
                    } else {
                        let cursor_x = *app.current_window_mut().cursor_x_mut();
                        let cursor_y = *app.current_window_mut().cursor_y_mut();
                        app.mode = Mode::Visual;
                        *app.current_window_mut().visual_start_mut() = Some((cursor_x, cursor_y));
                    }
                }
                "hsplit" => {
                    if app.show_directory {
                        app.hsplit_selected_item();
                    }
                }
                "delete_char" => {
                    let current_window = app.current_window_mut();
                    current_window.save_state(); // 変更前の状態を保存
                    let cy = *current_window.cursor_y_mut();
                    let mut graphemes: Vec<String> = current_window.buffer()[cy].graphemes(true).map(String::from).collect();
                    let cx = *current_window.cursor_x_mut();
                    if cx < graphemes.len() {
                        let deleted_char = graphemes[cx].chars().next().unwrap_or(' ');
                        graphemes.remove(cx);
                        current_window.buffer_mut()[cy] = graphemes.join("");
                        let new_cx = if cx >= graphemes.len() && !graphemes.is_empty() {
                            graphemes.len().saturating_sub(1)
                        } else if graphemes.is_empty() {
                            0
                        } else {
                            cx
                        };
                        *current_window.cursor_x_mut() = new_cx;
                        current_window.on_char_deleted(cy, new_cx, deleted_char);
                    }
                }
                "mode_insert" => {
                    let current_window = app.current_window_mut();
                    current_window.start_insert_mode(); // 挿入モード開始時に状態を保存
                    app.mode = Mode::Insert;
                }
                "append" => {
                    let current_window_ref = app.current_window_mut();
                    let cy = *current_window_ref.cursor_y_mut();
                    let grapheme_count = current_window_ref.buffer()[cy].graphemes(true).count();
                    let cx = *current_window_ref.cursor_x_mut();
                    if cx < grapheme_count {
                        *current_window_ref.cursor_x_mut() += 1;
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
                        let current_window = app.current_window_mut();
                        if !text.is_empty() {
                            current_window.save_state(); // 変更前の状態を保存
                            let cy = *current_window.cursor_y_mut();
                            let mut cx = *current_window.cursor_x_mut();
                            if text.contains('\n') {
                                let mut lines: Vec<String> = text.lines().map(String::from).collect();
                                let current_line_ref = &mut current_window.buffer_mut()[cy];
                                let byte_index = current_line_ref.grapheme_indices(true).nth(cx).map(|(i, _)| i).unwrap_or(current_line_ref.len());
                                let rest_of_current_line = current_line_ref.split_off(byte_index);
                                current_line_ref.push_str(&lines[0]);
                                let last_line_index = lines.len() - 1;
                                lines[last_line_index].push_str(&rest_of_current_line);
                                for (i, line) in lines.iter().skip(1).enumerate() {
                                    current_window.buffer_mut().insert(cy + 1 + i, line.clone());
                                    current_window.on_line_inserted(cy + 1 + i);
                                }
                                current_window.mark_line_modified(cy);
                            } else {
                                if !current_window.buffer()[cy].is_empty() {
                                    cx += 1;
                                }
                                let current_line_ref = &mut current_window.buffer_mut()[cy];
                                let byte_index = current_line_ref.grapheme_indices(true).nth(cx).map(|(i, _)| i).unwrap_or(current_line_ref.len());
                                current_line_ref.insert_str(byte_index, &text);
                                *current_window.cursor_x_mut() = cx + text.graphemes(true).count();
                                current_window.mark_line_modified(cy);
                            }
                        }
                    }
                }
                "undo" => {
                    let current_window = app.current_window_mut();
                    if current_window.undo() {
                        app.status_message = "Undone".to_string();
                    } else {
                        app.status_message = "Nothing to undo".to_string();
                    }
                }
                "open_new_line" => {
                    app.status_message = "o key pressed".to_string();
                    let current_window = app.current_window_mut();
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
        let current_window = app.current_window_mut();
        if current_window.redo() {
            app.status_message = "Redone".to_string();
        } else {
            app.status_message = "Nothing to redo".to_string();
        }
    }
}