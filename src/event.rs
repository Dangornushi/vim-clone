use crate::app::{App, Mode};
use crossterm::event::{self, Event, KeyCode};
use ratatui::backend::Backend;
use ratatui::Terminal;
use std::io;

pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| crate::ui::ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match app.mode {
                Mode::Normal => match key.code {
                    KeyCode::Char('h') => {
                        if app.cursor_x > 0 {
                            app.cursor_x -= 1;
                        }
                    }
                    KeyCode::Char('j') => {
                        if app.cursor_y < app.buffer.len() - 1 {
                            app.cursor_y += 1;
                            app.cursor_x = app.cursor_x.min(app.buffer[app.cursor_y].len());
                        }
                    }
                    KeyCode::Char('k') => {
                        if app.cursor_y > 0 {
                            app.cursor_y -= 1;
                            app.cursor_x = app.cursor_x.min(app.buffer[app.cursor_y].len());
                        }
                    }
                    KeyCode::Char('l') => {
                        if app.cursor_x < app.buffer[app.cursor_y].len() {
                            app.cursor_x += 1;
                        }
                    }
                    KeyCode::Char('x') => {
                        if app.cursor_x < app.buffer[app.cursor_y].len() {
                            app.buffer[app.cursor_y].remove(app.cursor_x);
                        }
                    }
                    KeyCode::Char('i') => {
                        app.mode = Mode::Insert;
                    }
                    KeyCode::Char(':') => {
                        app.mode = Mode::Command;
                        app.command_buffer.clear();
                    }
                    KeyCode::Char('v') => {
                        app.mode = Mode::Visual;
                        app.visual_start = Some((app.cursor_x, app.cursor_y));
                    }
                    KeyCode::Char('p') => {
                        if let Ok(text) = app.get_clipboard_text() {
                            if text.is_empty() {
                                // do nothing
                            } else if text.contains('\n') {
                                // Multi-line paste
                                let mut lines: Vec<String> = text.lines().map(String::from).collect();
                                let rest_of_current_line = app.buffer[app.cursor_y].split_off(app.cursor_x);
                                app.buffer[app.cursor_y].push_str(&lines[0]);

                                let last_line_index = lines.len() - 1;
                                lines[last_line_index].push_str(&rest_of_current_line);

                                for (i, line) in lines.iter().skip(1).enumerate() {
                                    app.buffer.insert(app.cursor_y + 1 + i, line.clone());
                                }
                                // Cursor remains at the start of the pasted block
                            } else {
                                // Single-line (character-wise) paste
                                if !app.buffer[app.cursor_y].is_empty() {
                                    app.cursor_x += 1;
                                }
                                if app.cursor_x > app.buffer[app.cursor_y].len() {
                                    app.cursor_x = app.buffer[app.cursor_y].len();
                                }
                                app.buffer[app.cursor_y].insert_str(app.cursor_x, &text);
                                app.cursor_x += text.len() - 1;
                            }
                        }
                    }
                    _ => {}
                },
                Mode::Visual => match key.code {
                    KeyCode::Char('h') => {
                        if app.cursor_x > 0 {
                            app.cursor_x -= 1;
                        }
                    }
                    KeyCode::Char('j') => {
                        if app.cursor_y < app.buffer.len() - 1 {
                            app.cursor_y += 1;
                            app.cursor_x = app.cursor_x.min(app.buffer[app.cursor_y].len());
                        }
                    }
                    KeyCode::Char('k') => {
                        if app.cursor_y > 0 {
                            app.cursor_y -= 1;
                            app.cursor_x = app.cursor_x.min(app.buffer[app.cursor_y].len());
                        }
                    }
                    KeyCode::Char('l') => {
                        if app.cursor_x < app.buffer[app.cursor_y].len() {
                            app.cursor_x += 1;
                        }
                    }
                    KeyCode::Char('d') | KeyCode::Char('y') => {
                        if let Some(start) = app.visual_start {
                            let (start_x, start_y) = start;
                            let (end_x, end_y) = (app.cursor_x, app.cursor_y);

                            // Normalize selection direction
                            let ((sel_start_y, sel_start_x), (sel_end_y, sel_end_x)) =
                                if (start_y, start_x) <= (end_y, end_x) {
                                    ((start_y, start_x), (end_y, end_x))
                                } else {
                                    ((end_y, end_x), (start_y, start_x))
                                };

                            let mut yanked_text = String::new();
                            if sel_start_y == sel_end_y {
                                // Single line
                                let line = &app.buffer[sel_start_y];
                                let end = (sel_end_x + 1).min(line.len());
                                if sel_start_x < end {
                                    yanked_text.push_str(&line[sel_start_x..end]);
                                }
                            } else {
                                // Multi-line
                                yanked_text.push_str(&app.buffer[sel_start_y][sel_start_x..]);
                                yanked_text.push('\n');
                                for y in (sel_start_y + 1)..sel_end_y {
                                    yanked_text.push_str(&app.buffer[y]);
                                    yanked_text.push('\n');
                                }
                                let end_line = &app.buffer[sel_end_y];
                                let end = (sel_end_x + 1).min(end_line.len());
                                yanked_text.push_str(&end_line[..end]);
                            }
                            app.set_yanked_text(yanked_text);

                            if key.code == KeyCode::Char('d') {
                                if sel_start_y == sel_end_y {
                                    // Single line deletion
                                    let line = &mut app.buffer[sel_start_y];
                                    let end = (sel_end_x + 1).min(line.len());
                                    if sel_start_x < end {
                                        line.drain(sel_start_x..end);
                                    }
                                } else {
                                    // Multi-line deletion
                                    let end_line = &app.buffer[sel_end_y];
                                    let split_point = (sel_end_x + 1).min(end_line.len());
                                    let end_line_suffix = end_line[split_point..].to_string();

                                    app.buffer[sel_start_y].truncate(sel_start_x);
                                    app.buffer[sel_start_y].push_str(&end_line_suffix);

                                    let start_of_removal = sel_start_y + 1;
                                    if start_of_removal <= sel_end_y {
                                        app.buffer.drain(start_of_removal..=sel_end_y);
                                    }
                                }
                            }

                            app.cursor_x = sel_start_x;
                            app.cursor_y = sel_start_y;

                            if app.buffer.is_empty() {
                                app.buffer.push(String::new());
                                app.cursor_y = 0;
                                app.cursor_x = 0;
                            } else {
                                if app.cursor_y >= app.buffer.len() {
                                    app.cursor_y = app.buffer.len() - 1;
                                }
                                if app.cursor_x > app.buffer[app.cursor_y].len() {
                                    app.cursor_x = app.buffer[app.cursor_y].len();
                                }
                            }

                            app.mode = Mode::Normal;
                            app.visual_start = None;
                        }
                    }
                    KeyCode::Esc => {
                        app.mode = Mode::Normal;
                        app.visual_start = None;
                    }
                    _ => {}
                },
                Mode::Insert => match key.code {
                    KeyCode::Char(c) => {
                        app.buffer[app.cursor_y].insert(app.cursor_x, c);
                        app.cursor_x += 1;
                    }
                    KeyCode::Backspace => {
                        if app.cursor_x > 0 {
                            app.cursor_x -= 1;
                            app.buffer[app.cursor_y].remove(app.cursor_x);
                        } else if app.cursor_y > 0 {
                            let prev_line_len = app.buffer[app.cursor_y - 1].len();
                            let current_line = app.buffer.remove(app.cursor_y);
                            app.buffer[app.cursor_y - 1].push_str(&current_line);
                            app.cursor_y -= 1;
                            app.cursor_x = prev_line_len;
                        }
                    }
                    KeyCode::Enter => {
                        let current_line = &mut app.buffer[app.cursor_y];
                        let new_line = current_line.split_off(app.cursor_x);
                        app.buffer.insert(app.cursor_y + 1, new_line);
                        app.cursor_y += 1;
                        app.cursor_x = 0;
                    }
                    KeyCode::Esc => {
                        app.mode = Mode::Normal;
                    }
                    _ => {}
                },
                Mode::Command => match key.code {
                    KeyCode::Char(c) => {
                        app.command_buffer.push(c);
                    }
                    KeyCode::Backspace => {
                        app.command_buffer.pop();
                    }
                    KeyCode::Enter => {
                        let command = app.command_buffer.trim();
                        match command {
                            "w" => {
                                app.save_file()?;
                            }
                            "q" => {
                                return Ok(());
                            }
                            "wq" => {
                                app.save_file()?;
                                return Ok(());
                            }
                            _ => {
                                app.status_message = format!("Not a command: {}", command);
                            }
                        }
                        app.mode = Mode::Normal;
                    }
                    KeyCode::Esc => {
                        app.mode = Mode::Normal;
                    }
                    _ => {}
                },
            }
        }
    }
}