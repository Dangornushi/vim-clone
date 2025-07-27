use crate::app::{App, FocusedPanel};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthChar;

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
        let inner_area = area.inner(&Margin { vertical: 1, horizontal: 1 });
        let visible_height = inner_area.height as usize;

        app.update_directory_scroll(visible_height);

        let directory_list: Vec<Line> = app.directory_files
            .iter()
            .enumerate()
            .skip(app.directory_scroll_offset)
            .take(visible_height)
            .map(|(i, file)| {
                let style = if i == app.selected_directory_index {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default()
                };
                Line::from(Span::styled(file.clone(), style))
            })
            .collect();
        let directory_paragraph = Paragraph::new(directory_list).block(directory_block.clone());
        f.render_widget(Clear, area);
        f.render_widget(directory_paragraph, area);
    } else {
        let directory_list: Vec<Line> = app.directory_files.iter().enumerate().map(|(i, file)| {
            let style = if i == app.selected_directory_index {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };
            Line::from(Span::styled(file.clone(), style))
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
    pub ai_status: String,
}

pub fn draw_chat_panel(
    f: &mut Frame,
    main_chunks: &[Rect],
    show_directory: bool,
    data: &mut ChatPanelData,
) {
    let right_panel_index = if show_directory { 2 } else { 1 };
    let right_panel_area = main_chunks[right_panel_index];

    let right_panel_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(right_panel_area);

    let status_area = right_panel_chunks[0];
    let status_paragraph = Paragraph::new(format!("AI Status: {}", data.ai_status))
        .style(Style::default().fg(Color::Yellow));
    let status_rect = Rect {
        x: status_area.x,
        y: status_area.y,
        width: status_area.width,
        height: 1,
    };
    f.render_widget(status_paragraph, status_rect);

    let visible_height = right_panel_chunks[0].height.saturating_sub(3) as usize;
    if data.selected_index < data.scroll_offset {
        data.scroll_offset = data.selected_index;
    } else if data.selected_index >= data.scroll_offset + visible_height {
        data.scroll_offset = data.selected_index - visible_height + 1;
    }

    let panel_width = right_panel_chunks[0].width as usize;
    let mut right_panel_list: Vec<Line> = Vec::new();
    for (i, item) in data.items.iter().enumerate().skip(data.scroll_offset).take(visible_height) {
        let is_selected = i == data.selected_index;
        let mut line = String::new();
        let mut width = 0;
        for c in item.chars() {
            let cw = c.width().unwrap_or(1);
            line.push(c);
            width += cw;
            if width >= panel_width || c == '。' || c == '、' {
                let style = if is_selected {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default()
                };
                right_panel_list.push(Line::from(Span::styled(line.clone(), style)));
                line.clear();
                width = 0;
            }
        }
        if !line.is_empty() {
            let style = if is_selected {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };
            right_panel_list.push(Line::from(Span::styled(line, style)));
        }
    }

    let chat_panel_block = Block::default()
        .borders(Borders::ALL)
        .title(if data.focused {
            "Chat [FOCUSED]"
        } else {
            "Chat"
        });
    let chat_panel_paragraph = Paragraph::new(right_panel_list).block(chat_panel_block);
    let list_rect = Rect {
        x: right_panel_chunks[0].x,
        y: right_panel_chunks[0].y + 1,
        width: right_panel_chunks[0].width,
        height: right_panel_chunks[0].height - 1,
    };
    f.render_widget(chat_panel_paragraph, list_rect);

    let input_block = Block::default().borders(Borders::ALL).title("Input");
    let input_paragraph = Paragraph::new(data.input.clone()).block(input_block);
    f.render_widget(input_paragraph, right_panel_chunks[1]);
}
