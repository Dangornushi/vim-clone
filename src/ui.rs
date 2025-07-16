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
        let directory_block = Block::default().borders(Borders::ALL).title(directory_title.clone());

        if is_floating {
            let area = centered_rect(60, 80, f.size());
            f.render_widget(Clear, area); // this clears the background

            // フローティングウィンドウの内部エリアを計算
            let inner_area = area.inner(&Margin { vertical: 1, horizontal: 1 });
            let visible_height = inner_area.height as usize;

            // スクロール範囲を計算
            let total_items = app.directory_files.len();
            let scroll_offset = app.directory_scroll_offset;
            let visible_end = (scroll_offset + visible_height).min(total_items);

            // スクロール位置を最新化（スクロール時や選択時に必ず呼ぶ）
            app.update_directory_scroll(visible_height);

            // 表示するアイテムのリストを作成
            let directory_list: Vec<Line> = app.directory_files
                .iter()
                .enumerate()
                .skip(scroll_offset)
                .take(visible_height)
                .map(|(i, file)| {
                    let actual_index = i + scroll_offset;
                    if actual_index == app.selected_directory_index {
                        // より明確な色でハイライト
                        Line::from(Span::styled(
                            file.clone(),
                            Style::default()
                                .bg(ratatui::style::Color::Rgb(100, 100, 100))  // 明確なグレー
                                .fg(ratatui::style::Color::Rgb(255, 255, 255))  // 白文字
                        ))
                    } else {
                        Line::from(Span::styled(
                            file.clone(),
                            Style::default().fg(ratatui::style::Color::Rgb(200, 200, 200))  // 通常の項目も明確に
                        ))
                    }
                })
                .collect();
            let title_with_scroll = if total_items > visible_height {
                format!("Directory: {} ({}/{} items)",
                    app.current_path.to_string_lossy(),
                    visible_end,
                    total_items
                )
            } else {
                directory_title
            };

            let directory_block_with_scroll = Block::default()
                .borders(Borders::ALL)
                .title(title_with_scroll);

            let directory_paragraph = Paragraph::new(directory_list).block(directory_block_with_scroll);
            f.render_widget(directory_paragraph, area);
        } else {
            // 非フローティングモードでも明確な色でハイライト
            let directory_list: Vec<Line> = app.directory_files.iter().enumerate().map(|(i, file)| {
                if i == app.selected_directory_index {
                    Line::from(Span::styled(
                        file.clone(),
                        Style::default()
                            .bg(ratatui::style::Color::Rgb(100, 100, 100))  // 明確なグレー
                            .fg(ratatui::style::Color::Rgb(255, 255, 255))  // 白文字
                    ))
                } else {
                    Line::from(Span::styled(
                        file.clone(),
                        Style::default().fg(ratatui::style::Color::Rgb(200, 200, 200))  // 通常の項目も明確に
                    ))
                }
            }).collect();
            let directory_paragraph = Paragraph::new(directory_list).block(directory_block.clone());
            f.render_widget(directory_paragraph, main_chunks[0]);
        }
    }

    // ステータスバーの描画
    let status_bar_text = match app_mode {
        Mode::Normal => {
            let w = app.current_window_mut();
            format!(
                "NORMAL | {}:{} | {}",
                w.cursor_y() + 1,
                w.cursor_x() + 1,
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

    // 予測変換ポップアップの描画
    if app.show_completion && !app.completions.is_empty() && !app.show_directory {
        if let Some(active_pane) = app.pane_manager.get_active_pane() {
            if let Some(rect) = active_pane.rect {
                draw_completion_popup(f, app, rect);
            }
        }
    }

    // カーソルの描画
    if !app.show_directory {
        if let Some(active_pane) = app.pane_manager.get_active_pane() {
            if let Some(rect) = active_pane.rect {
                let show_line_numbers = app.config.editor.show_line_numbers;
                let horizontal_margin = app.config.ui.editor_margins.horizontal;
                let current_window = app.current_window_mut();
                let cursor_width = current_window.buffer()[current_window.cursor_y()]
                    .graphemes(true)
                    .take(current_window.cursor_x())
                    .map(|g| g.width())
                    .sum::<usize>();
                let line_number_width = if show_line_numbers { editor::DEFAULT_LINE_NUMBER_WIDTH } else { 0 };
                let separator_width = if show_line_numbers { editor::LINE_NUMBER_SEPARATOR_WIDTH } else { 0 };
                let text_start_x_offset = horizontal_margin as usize + line_number_width + separator_width;
                // カーソルが表示範囲内にある場合のみ描画
                if current_window.cursor_y() >= current_window.scroll_y() && 
                   current_window.cursor_y() < current_window.scroll_y() + rect.height.saturating_sub(2) as usize {
                    f.set_cursor(
                        rect.x + text_start_x_offset as u16 + (cursor_width - current_window.scroll_x()) as u16,
                        rect.y + 1 + (current_window.cursor_y() - current_window.scroll_y()) as u16,
                    )
                }
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
    let editor_block = Block::default().borders(Borders::ALL).title(window.filename().unwrap_or(file::DEFAULT_FILENAME)).border_style(border_style);
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
        let line_numbers: Vec<Line> = (window.scroll_y()..window.scroll_y() + editor_area.height as usize)
            .map(|i| {
                if i < window.buffer().len() {
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

    // 1パス目: ファイル全体をスキャンし、未対応の括弧を特定し、
    //          同時に各行の開始時点での BracketState をキャッシュする
    let mut states_by_line = Vec::with_capacity(window.buffer().len() + 1);
    states_by_line.push(BracketState::new());
    let mut current_state = BracketState::new();
    let mut all_unmatched_brackets: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    
    for (i, line_str) in window.buffer().iter().enumerate() {
        let space_count = crate::syntax::count_leading_spaces(line_str);
        let content_part = &line_str[space_count..];
        // 1パス目では、unmatched_brackets は空のセットを渡し、
        // tokenize_with_state は自身のスタックに基づいてis_matchedを決定する
        let tokens = crate::syntax::tokenize_with_state(content_part, i, space_count, &mut current_state);
        
        // この行で未対応とマークされた閉じ括弧を収集
        for token in tokens {
            if let crate::syntax::TokenType::Bracket { is_matched: false, .. } = token.token_type {
                // 閉じ括弧で、かつマッチしていない場合
                if token.content == ")" || token.content == "]" || token.content == "}" {
                    all_unmatched_brackets.insert((i, space_count + token.start));
                }
            }
        }
        states_by_line.push(current_state.clone());
    }
    // スキャン完了後、スタックに残っているものが閉じられていない開き括弧
    for &(_, line, col) in &current_state.stack {
        all_unmatched_brackets.insert((line, col));
    }
    let unmatched_brackets = all_unmatched_brackets; // 名前を合わせる

    // 2. 表示範囲の行をレンダリングする
    let text: Vec<Line> = window
        .buffer()
        .iter()
        .enumerate()
        .skip(window.scroll_y())
        .take(editor_area.height as usize)
        .map(|(i, line_str)| {
            // キャッシュした状態を使ってハイライト
            let mut bracket_state = states_by_line[i].clone();

            if let (Mode::Visual, Some(start)) = (&app_mode, window.visual_start()) {
                if is_active {
                    let (start_x, start_y) = start;
                    let (end_x, end_y) = (window.cursor_x(), window.cursor_y());

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

            let mut spans = highlight_syntax_with_state(line_str, i, config.editor.indent_width, &mut bracket_state, &config.theme, &unmatched_brackets);
            if let Some((bx, by)) = window.matching_bracket() {
                if by == i {
                    let mut current_width = 0;
                    for span in &mut spans {
                        let span_width = span.width();
                        if current_width <= bx && bx < current_width + span_width {
                            span.style = span.style.add_modifier(ratatui::style::Modifier::UNDERLINED);
                            break;
                        }
                        current_width += span_width;
                    }
                }
            }
            Line::from(spans)
        })
        .collect();
    let editor_paragraph = Paragraph::new(text).scroll((0, window.scroll_x() as u16));
    f.render_widget(editor_paragraph, editor_chunks[2]);
}

fn draw_completion_popup(f: &mut Frame, app: &mut App, editor_rect: Rect) {
    let current_window = app.current_window();
    let show_line_numbers = app.config.editor.show_line_numbers;
    let horizontal_margin = app.config.ui.editor_margins.horizontal;
    let line_number_width = if show_line_numbers { editor::DEFAULT_LINE_NUMBER_WIDTH } else { 0 };
    let separator_width = if show_line_numbers { editor::LINE_NUMBER_SEPARATOR_WIDTH } else { 0 };
    
    // カーソル位置を計算
    let cursor_width = current_window.buffer()[current_window.cursor_y()]
        .graphemes(true)
        .take(current_window.cursor_x())
        .map(|g| g.width())
        .sum::<usize>();
    
    let text_start_x_offset = horizontal_margin as usize + line_number_width + separator_width;
    let cursor_x = editor_rect.x + text_start_x_offset as u16 + (cursor_width - current_window.scroll_x()) as u16;
    let cursor_y = editor_rect.y + 1 + (current_window.cursor_y() - current_window.scroll_y()) as u16;
    
    // 予測変換ポップアップのサイズを計算
    let max_items = 10;
    let visible_items = app.completions.len().min(max_items);
    let popup_height = visible_items as u16 + 2; // ボーダー分を追加
    
    // 最大幅を計算（最長の補完候補に基づく）
    let max_width = app.completions.iter()
        .map(|s| s.width())
        .max()
        .unwrap_or(10)
        .max(10) as u16 + 4; // パディング分を追加
    
    // ポップアップの位置を計算（カーソルの下）
    let popup_x = cursor_x.min(f.size().width.saturating_sub(max_width));
    let popup_y = if cursor_y + 1 + popup_height <= f.size().height {
        cursor_y + 1 // カーソルの下
    } else {
        cursor_y.saturating_sub(popup_height) // カーソルの上
    };
    
    let popup_rect = Rect {
        x: popup_x,
        y: popup_y,
        width: max_width,
        height: popup_height,
    };
    
    // 背景をクリア
    f.render_widget(Clear, popup_rect);
    
    // スクロール位置を計算
    let scroll_offset = if app.selected_completion >= max_items {
        app.selected_completion - max_items + 1
    } else {
        0
    };
    
    // 表示する補完候補を準備
    let completion_lines: Vec<Line> = app.completions
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(max_items)
        .map(|(i, completion)| {
            let actual_index = i + scroll_offset;
            if actual_index == app.selected_completion {
                // 選択されている項目
                Line::from(Span::styled(
                    completion.clone(),
                    Style::default()
                        .bg(app.config.theme.ui.completion_selection_background.clone().into())
                        .fg(app.config.theme.ui.completion_foreground.clone().into())
                ))
            } else {
                // 通常の項目
                Line::from(Span::styled(
                    completion.clone(),
                    Style::default()
                        .fg(app.config.theme.ui.completion_foreground.clone().into())
                ))
            }
        })
        .collect();
    
    // ポップアップを描画
    let popup_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(app.config.theme.ui.completion_background.clone().into()));
    
    let popup_paragraph = Paragraph::new(completion_lines)
        .block(popup_block);
    
    f.render_widget(popup_paragraph, popup_rect);
}
