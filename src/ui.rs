use crate::app::{App, Mode};
use crate::syntax::{highlight_syntax_with_state, BracketState};
use crate::constants::{editor, ui as ui_constants, file};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Clear},
    Frame,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub fn ui(f: &mut Frame, app: &mut App) {
    let app_mode = app.mode;
    let app_status_message = app.status_message.clone();
    let app_command_buffer = app.command_buffer.clone();

    let is_floating = app.config.ui.directory_pane_floating;

    let main_chunks = if app.show_directory && !is_floating {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(app.config.ui.directory_pane_width), Constraint::Min(0)].as_ref())
            .split(f.size())
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0)].as_ref())
            .split(f.size())
    };

    let editor_chunk_index = if app.show_directory && !is_floating { 1 } else { 0 };
    let editor_area = main_chunks[editor_chunk_index];

    // ペインマネージャーを使用してレイアウトを計算
    app.pane_manager.calculate_layout(editor_area);

    // すべてのリーフペインの情報を取得
    let pane_info: Vec<(usize, usize, ratatui::layout::Rect, bool)> = {
        let leaf_panes = app.pane_manager.get_leaf_panes();
        let active_pane_id = app.pane_manager.get_active_pane_id();
        
        leaf_panes.iter()
            .filter_map(|pane| {
                pane.rect.map(|rect| {
                    (pane.id, pane.window_index, rect, pane.id == active_pane_id)
                })
            })
            .collect()
    };
    
    // ペインを描画
    for (_, window_index, rect, is_active) in pane_info {
        draw_editor_pane(f, app, rect, window_index, is_active);
    }

    if app.show_directory {
        let directory_title = format!("Directory: {}", app.current_path.to_string_lossy());
        let directory_block = Block::default().borders(Borders::ALL).title(directory_title);
        let directory_list: Vec<Line> = app.directory_files.iter().enumerate().map(|(i, file)| {
            if i == app.selected_directory_index {
                Line::from(Span::styled(file, Style::default().bg(app.config.theme.ui.selection_background.clone().into())))
            } else {
                Line::from(file.as_str())
            }
        }).collect();
        let directory_paragraph = Paragraph::new(directory_list).block(directory_block);

        if is_floating {
            let area = centered_rect(60, 80, f.size());
            f.render_widget(Clear, area); // this clears the background
            f.render_widget(directory_paragraph, area);
        } else {
            f.render_widget(directory_paragraph, main_chunks[0]);
        }
    }

    // ステータスバーの描画
    let status_bar_text = match app_mode {
        Mode::Normal => {
            let w = app.current_window();
            format!(
                "NORMAL | {}:{} | {}",
                w.cursor_y + 1,
                w.cursor_x + 1,
                app_status_message
            )
        },
        Mode::Insert => "INSERT".to_string(),
        Mode::Visual => "VISUAL".to_string(),
        Mode::Command => format!(":{}", app_command_buffer),
    };
    let status_bar_chunk = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(app.config.ui.status_bar_height)].as_ref())
        .split(f.size())[1];
    let status_bar = Paragraph::new(status_bar_text).style(Style::default().bg(app.config.theme.ui.status_bar_background.clone().into()));
    f.render_widget(status_bar, status_bar_chunk);

    // カーソルの描画
    if !app.show_directory {
        if let Some(active_pane) = app.pane_manager.get_active_pane() {
            if let Some(rect) = active_pane.rect {
                let show_line_numbers = app.config.editor.show_line_numbers;
                let horizontal_margin = app.config.ui.editor_margins.horizontal;
                let current_window = app.current_window();
                let cursor_width = current_window.buffer[current_window.cursor_y]
                    .graphemes(true)
                    .take(current_window.cursor_x)
                    .map(|g| g.width())
                    .sum::<usize>();
                let line_number_width = if show_line_numbers { editor::DEFAULT_LINE_NUMBER_WIDTH } else { 0 };
                let separator_width = if show_line_numbers { editor::LINE_NUMBER_SEPARATOR_WIDTH } else { 0 };
                let text_start_x_offset = horizontal_margin as usize + line_number_width + separator_width;
                f.set_cursor(
                    rect.x + text_start_x_offset as u16 + (cursor_width - current_window.scroll_x) as u16,
                    rect.y + 1 + (current_window.cursor_y - current_window.scroll_y) as u16,
                )
            }
        }
    }
}

/// Helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_editor_pane(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect, window_index: usize, is_active: bool) {
    let window = &mut app.windows[window_index];
    let app_mode = app.mode;
    let config = &app.config;
    
    // シンタックスハイライトの更新完了をマーク
    window.mark_syntax_updated();

    let border_style = if is_active { Style::default().fg(config.theme.ui.active_pane_border.clone().into()) } else { Style::default() };
    let editor_block = Block::default().borders(Borders::ALL).title(window.filename.as_deref().unwrap_or(file::DEFAULT_FILENAME)).border_style(border_style);
    f.render_widget(editor_block, area);
    let editor_area = area.inner(&Margin { 
        vertical: config.ui.editor_margins.vertical, 
        horizontal: config.ui.editor_margins.horizontal 
    });

    window.scroll_to_cursor(editor_area.height as usize, editor_area.width as usize, config.editor.show_line_numbers);

    let line_number_width = if config.editor.show_line_numbers { config.editor.line_number_width } else { 0 };
    let separator_width = if config.editor.show_line_numbers { editor::LINE_NUMBER_SEPARATOR_WIDTH } else { 0 };

    let editor_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(line_number_width as u16),
            Constraint::Length(separator_width as u16),
            Constraint::Min(0),
        ])
        .split(editor_area);

    if config.editor.show_line_numbers {
        let line_numbers: Vec<Line> = (window.scroll_y..window.scroll_y + editor_area.height as usize)
            .map(|i| {
                if i < window.buffer.len() {
                    Line::from(Span::styled(
                        format!("{:>width$}", i + 1, width = line_number_width), 
                        Style::default().fg(config.theme.ui.line_number.clone().into())
                    ))
                } else {
                    Line::from(Span::styled(
                        format!("{:>width$}", ui_constants::EMPTY_LINE_MARKER, width = line_number_width), 
                        Style::default().fg(config.theme.ui.line_number.clone().into())
                    ))
                }
            })
            .collect();
        let line_numbers_paragraph = Paragraph::new(line_numbers).alignment(Alignment::Right);
        f.render_widget(line_numbers_paragraph, editor_chunks[0]);

        let space_paragraph = Paragraph::new(" ");
        f.render_widget(space_paragraph, editor_chunks[1]);
    }

    // 1. ファイル全体をスキャンし、閉じられていない開き括弧を特定し、
    //    同時に各行の開始時点での BracketState をキャッシュする
    let mut states_by_line = Vec::with_capacity(window.buffer.len() + 1);
    states_by_line.push(BracketState::new());
    let mut current_state = BracketState::new();
    let empty_unmatched = std::collections::HashSet::new();
    for (i, line_str) in window.buffer.iter().enumerate() {
        let space_count = crate::syntax::count_leading_spaces(line_str);
        let content_part = &line_str[space_count..];
        // このループでは状態の更新とキャッシュが目的なので、unmatched_brackets は空のセットを渡す
        let _ = crate::syntax::tokenize_with_state(content_part, i, space_count, &mut current_state, &empty_unmatched);
        states_by_line.push(current_state.clone());
    }
    // スキャン完了後、スタックに残っているものが閉じられていない開き括弧
    let unmatched_brackets: std::collections::HashSet<(usize, usize)> = 
        current_state.stack.iter().map(|&(_, line, col)| (line, col)).collect();

    // 2. 表示範囲の行をレンダリングする
    let text: Vec<Line> = window
        .buffer
        .iter()
        .enumerate()
        .skip(window.scroll_y)
        .take(editor_area.height as usize)
        .map(|(i, line_str)| {
            // キャッシュした状態を使ってハイライト
            let mut bracket_state = states_by_line[i].clone();

            if let (Mode::Visual, Some(start)) = (&app_mode, window.visual_start) {
                if is_active {
                    let (start_x, start_y) = start;
                    let (end_x, end_y) = (window.cursor_x, window.cursor_y);

                    let ((sel_start_y, sel_start_x), (sel_end_y, sel_end_x)) =
                        if (start_y, start_x) <= (end_y, end_x) {
                            ((start_y, start_x), (end_y, end_x))
                        } else {
                            ((end_y, end_x), (start_y, start_x))
                        };

                    if i >= sel_start_y && i <= sel_end_y {
                        let graphemes: Vec<&str> = line_str.graphemes(true).collect();
                        let line_len = graphemes.len();

                        let highlight_start = if i == sel_start_y { sel_start_x } else { 0 };
                        let highlight_end = if i == sel_end_y { sel_end_x + 1 } else { line_len };

                        let highlight_start = highlight_start.min(line_len);
                        let highlight_end = highlight_end.min(line_len);

                        let mut spans = Vec::new();
                        if highlight_start > 0 {
                            let s = graphemes[0..highlight_start].join("");
                            spans.extend(highlight_syntax_with_state(&s, i, config.editor.indent_width, &mut bracket_state, &config.theme, &unmatched_brackets));
                        }
                        if highlight_start < highlight_end {
                            let selected_text = graphemes[highlight_start..highlight_end].join("");
                            let highlighted_selected_spans = highlight_syntax_with_state(&selected_text, i, config.editor.indent_width, &mut bracket_state, &config.theme, &unmatched_brackets)
                                .into_iter()
                                .map(|mut span| {
                                    span.style = span.style.bg(config.theme.ui.visual_selection_background.clone().into());
                                    span
                                })
                                .collect::<Vec<Span<'static>>>();
                            spans.extend(highlighted_selected_spans);
                        }
                        if highlight_end < line_len {
                            let s = graphemes[highlight_end..line_len].join("");
                            spans.extend(highlight_syntax_with_state(&s, i, config.editor.indent_width, &mut bracket_state, &config.theme, &unmatched_brackets));
                        }
                        return Line::from(spans);
                    }
                }
            }
            Line::from(highlight_syntax_with_state(line_str, i, config.editor.indent_width, &mut bracket_state, &config.theme, &unmatched_brackets))
        })
        .collect();
    let editor_paragraph = Paragraph::new(text).scroll((0, window.scroll_x as u16));
    f.render_widget(editor_paragraph, editor_chunks[2]);
}
