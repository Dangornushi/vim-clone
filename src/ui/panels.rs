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
    

pub fn draw_right_panel(f: &mut Frame, app: &mut App, main_chunks: &[Rect]) {
        let right_panel_index = if app.show_directory { 2 } else { 1 };
    let right_panel_area = main_chunks[right_panel_index];
    
    // 右側パネルを上下に分割（上: リスト、下: 入力欄）
        let right_panel_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3), // 入力欄の高さ
        ])
        .split(right_panel_area);

    // 右側パネルのリスト部分
    let visible_height = right_panel_chunks[0].height.saturating_sub(2) as usize; // ボーダーを除いた高さ
    app.update_right_panel_scroll(visible_height);
    
    let right_panel_list: Vec<Line> = app.right_panel_items
        .iter()
        .enumerate()
        .skip(app.right_panel_scroll_offset)
        .take(visible_height)
        .map(|(i, item)| {
            let actual_index = i + app.right_panel_scroll_offset;
            if actual_index == app.selected_right_panel_index {
                Line::from(Span::styled(
                    item.clone(),
                    Style::default()
                        .bg(ratatui::style::Color::Rgb(100, 100, 100))
                        .fg(ratatui::style::Color::Rgb(255, 255, 255))
                ))
            } else {
                Line::from(Span::styled(
                    item.clone(),
                    Style::default().fg(ratatui::style::Color::Rgb(200, 200, 200))
                ))
            }
        })
        .collect();

    let right_panel_block = Block::default()
        .borders(Borders::ALL)
        .title(if app.focused_panel == FocusedPanel::RightPanel {
            "Right Panel [FOCUSED]"
        } else {
            "Right Panel"
        });
    let right_panel_paragraph = Paragraph::new(right_panel_list).block(right_panel_block);
    f.render_widget(right_panel_paragraph, right_panel_chunks[0]);

    // 入力欄
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title("Input");
    let input_paragraph = Paragraph::new(app.right_panel_input.clone()).block(input_block);
    f.render_widget(input_paragraph, right_panel_chunks[1]);
}