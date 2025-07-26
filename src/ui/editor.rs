use crate::app::App;
use crate::window::Mode;
use crate::syntax::{highlight_syntax_with_state, BracketState};
use crate::constants::{editor, ui as ui_constants, file};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use unicode_segmentation::UnicodeSegmentation;

pub fn draw_editor_pane(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect, window_index: usize, is_active: bool) {
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