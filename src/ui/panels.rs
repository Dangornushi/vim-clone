use crate::app::{App, FocusedPanel};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Helper function to create a centered rect using up certain percentage of the available rect `r`
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

    
pub fn draw_directory_panel(f: &mut Frame, app: &mut App, main_chunks: &[Rect], is_floating: bool) {
        let directory_title = if app.focused_panel == FocusedPanel::Directory {
        format!("Directory: {} [FOCUSED]", app.current_path.to_string_lossy())
    } else {
        format!("Directory: {}", app.current_path.to_string_lossy())
    };
        let directory_block = Block::default().borders(Borders::ALL).title(directory_title.clone());

    if is_floating {
        let area = centered_rect(60, 80, f.size());
        // f.render_widget(Clear, area); // this clears the background

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
    

pub struct ChatPanelData {
    pub items: Vec<String>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub input: String,
    pub focused: bool,
    pub ai_status: String, // AI状態表示用
    pub input_cursor: usize,
}

pub fn draw_chat_panel(
    f: &mut Frame,
    main_chunks: &[Rect],
    show_directory: bool,
    data: &mut ChatPanelData,
) {
    let right_panel_index = if show_directory { 2 } else { 1 };
    let right_panel_area = main_chunks[right_panel_index];

    // 右側パネルを上下に分割（上: リスト、下: 入力欄）
    let right_panel_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3), // 入力欄の高さ
        ])
        .split(right_panel_area);

    // AI状態表示（チャットリストの上部に表示）
    let status_area = right_panel_chunks[0];
    let status_paragraph = Paragraph::new(format!("AI状態: {}", data.ai_status))
        .style(Style::default().fg(ratatui::style::Color::Yellow))
        .block(Block::default().borders(Borders::NONE));
    // 高さ1行分だけ上部に描画
    let status_rect = Rect {
        x: status_area.x,
        y: status_area.y,
        width: status_area.width,
        height: 1,
    };
    f.render_widget(status_paragraph, status_rect);

    // 右側パネルのリスト部分
    let visible_height = right_panel_chunks[0].height.saturating_sub(3) as usize; // 状態表示分高さ減
    if data.selected_index < data.scroll_offset {
        data.scroll_offset = data.selected_index;
    } else if data.selected_index >= data.scroll_offset + visible_height {
        data.scroll_offset = data.selected_index - visible_height + 1;
    }

    let panel_width = right_panel_chunks[0].width as usize;
    let mut right_panel_list: Vec<Line> = Vec::new();
    for (i, item) in data.items.iter().enumerate().skip(data.scroll_offset).take(visible_height) {
        let actual_index = i + data.scroll_offset;
        let is_selected = actual_index == data.selected_index;
        // unicode-widthで表示幅を計算し、収まりきらない場合のみ分割
        use unicode_width::UnicodeWidthChar;
        let mut line = String::new();
        let mut width = 0;
        for c in item.chars() {
            let cw = UnicodeWidthChar::width(c).unwrap_or(1);
            line.push(c);
            width += cw;
            // 句読点のみで改行（スペースは改行しない）
            if width >= panel_width || c == '。' || c == '、' {
                let style = if is_selected {
                    Style::default()
                        .bg(ratatui::style::Color::Rgb(100, 100, 100))
                        .fg(ratatui::style::Color::Rgb(255, 255, 255))
                } else {
                    Style::default().fg(ratatui::style::Color::Rgb(200, 200, 200))
                };
                right_panel_list.push(Line::from(Span::styled(line.clone(), style)));
                line.clear();
                width = 0;
            }
        }
        if !line.is_empty() {
            let style = if is_selected {
                Style::default()
                    .bg(ratatui::style::Color::Rgb(100, 100, 100))
                    .fg(ratatui::style::Color::Rgb(255, 255, 255))
            } else {
                Style::default().fg(ratatui::style::Color::Rgb(200, 200, 200))
            };
            right_panel_list.push(Line::from(Span::styled(line, style)));
        }
    }

    let chat_panel_block = Block::default()
        .borders(Borders::ALL)
        .title(if data.focused {
            "チャット欄 [FOCUSED]"
        } else {
            "チャット欄"
        });
    let chat_panel_paragraph = Paragraph::new(right_panel_list)
        .block(chat_panel_block);
    // 状態表示の下にリストを描画
    let list_rect = Rect {
        x: right_panel_chunks[0].x,
        y: right_panel_chunks[0].y + 1,
        width: right_panel_chunks[0].width,
        height: right_panel_chunks[0].height - 1,
    };
    f.render_widget(chat_panel_paragraph, list_rect);

    // 入力欄
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title("メッセージ入力");
    let input_paragraph = Paragraph::new(data.input.clone()).block(input_block);
    f.render_widget(input_paragraph, right_panel_chunks[1]);
}