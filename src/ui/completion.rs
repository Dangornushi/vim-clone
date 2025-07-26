use crate::app::App;
use crate::constants::editor;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub fn draw_completion_popup(f: &mut Frame, app: &mut App, editor_rect: Rect) {
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