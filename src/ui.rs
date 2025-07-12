use crate::app::{App, Mode};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
        .split(f.size());

    let text: Vec<Line> = app
        .buffer
        .iter()
        .enumerate()
        .map(|(i, line_str)| {
            if let (Mode::Visual, Some(start)) = (&app.mode, app.visual_start) {
                let (start_x, start_y) = start;
                let (end_x, end_y) = (app.cursor_x, app.cursor_y);

                // Normalize selection direction
                let ((sel_start_y, sel_start_x), (sel_end_y, sel_end_x)) =
                    if (start_y, start_x) <= (end_y, end_x) {
                        ((start_y, start_x), (end_y, end_x))
                    } else {
                        ((end_y, end_x), (start_y, start_x))
                    };

                if i >= sel_start_y && i <= sel_end_y {
                    let chars: Vec<char> = line_str.chars().collect();
                    let line_len = chars.len();

                    let highlight_start = if i == sel_start_y { sel_start_x } else { 0 };
                    let highlight_end = if i == sel_end_y {
                        sel_end_x + 1
                    } else {
                        line_len
                    };

                    let highlight_start = highlight_start.min(line_len);
                    let highlight_end = highlight_end.min(line_len);

                    if highlight_start >= highlight_end {
                        return Line::from(line_str.as_str());
                    }

                    let mut spans = Vec::new();
                    if highlight_start > 0 {
                        spans.push(Span::from(
                            chars[0..highlight_start].iter().collect::<String>(),
                        ));
                    }
                    spans.push(Span::styled(
                        chars[highlight_start..highlight_end]
                            .iter()
                            .collect::<String>(),
                        Style::default().bg(Color::Blue),
                    ));
                    if highlight_end < line_len {
                        spans.push(Span::from(
                            chars[highlight_end..line_len].iter().collect::<String>(),
                        ));
                    }

                    return Line::from(spans);
                }
            }
            Line::from(line_str.as_str())
        })
        .collect();
    let editor_paragraph =
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Editor"));
    f.render_widget(editor_paragraph, chunks[0]);

    let status_bar_text = match app.mode {
        Mode::Normal => format!(
            "NORMAL | {}:{} | {}",
            app.cursor_y + 1,
            app.cursor_x + 1,
            app.status_message
        ),
        Mode::Insert => "INSERT".to_string(),
        Mode::Visual => "VISUAL".to_string(),
        Mode::Command => format!(":{}", app.command_buffer),
    };
    let status_bar = Paragraph::new(status_bar_text).style(Style::default().bg(Color::Gray));
    f.render_widget(status_bar, chunks[1]);

    // Show cursor
    match app.mode {
        Mode::Normal | Mode::Insert | Mode::Visual => {
            f.set_cursor(
                chunks[0].x + app.cursor_x as u16 + 1,
                chunks[0].y + app.cursor_y as u16 + 1,
            )
        }
        Mode::Command => {
            f.set_cursor(
                chunks[1].x + app.command_buffer.len() as u16 + 1,
                chunks[1].y,
            )
        }
    }
    // Clear status message after displaying it once
    if !app.status_message.is_empty() && !matches!(app.mode, Mode::Command) {
        app.status_message.clear();
    }
}