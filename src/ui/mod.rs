pub mod editor;
pub mod completion;
pub mod panels;

use crate::app::App;
use crate::utils::get_display_cursor_x;
use crate::window::Mode;
use crate::constants::editor as editor_constants;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::Paragraph,
    Frame,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub use editor::draw_editor_pane;
pub use completion::draw_completion_popup;
pub use panels::{draw_directory_panel, draw_chat_panel, ChatPanelData};

pub fn ui(f: &mut Frame, app: &mut App) {
    let app_mode = app.mode;
    let app_status_message = app.status_message.clone();
    let app_command_buffer = app.command_buffer.clone();

    let is_floating = app.config.ui.directory_pane_floating;

    let main_chunks = if (app.show_directory || app.show_right_panel) && !is_floating {
        let mut constraints = vec![];
        
        // 左側ディレクトリパネル
        if app.show_directory {
            constraints.push(Constraint::Length(app.config.ui.directory_pane_width));
        }
        
        // 中央のエディタ部分
        constraints.push(Constraint::Min(0));
        
        // 右側パネル
        if app.show_right_panel && !is_floating {
            constraints.push(Constraint::Length(app.config.ui.directory_pane_width));
        }
        
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
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
        draw_directory_panel(f, app, &main_chunks, is_floating);
    }

    // 右側パネルの描画
    if app.show_right_panel && !is_floating {
        use crate::ui::panels::ChatPanelData;
        let mut chat_panel_data = ChatPanelData {
            items: app.right_panel_items.clone(),
            selected_index: app.selected_right_panel_index,
            scroll_offset: app.right_panel_scroll_offset,
            input: app.right_panel_input.clone(),
            focused: app.focused_panel == crate::app::FocusedPanel::RightPanel,
            ai_status: app.ai_status.clone(), // AI状態をAppから取得
            input_cursor: app.right_panel_input_cursor,
        };
        draw_chat_panel(
            f,
            &main_chunks,
            app.show_directory,
            &mut chat_panel_data,
        );
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
        Mode::RightPanelInput => "RIGHT PANEL INPUT".to_string(),
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

    // カーソルの描画 - フォーカスされているパネルに表示
    use crate::app::FocusedPanel;
    match app.focused_panel {
        FocusedPanel::RightPanel if app.show_right_panel && !is_floating => {
            if app.mode == Mode::RightPanelInput {
                // 右側パネルの入力欄にカーソルを表示
                let right_panel_index = if app.show_directory { 2 } else { 1 };
                let right_panel_area = main_chunks[right_panel_index];
                let right_panel_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(0),
                        Constraint::Length(3),
                    ])
                    .split(right_panel_area);
                
                let input_area = right_panel_chunks[1].inner(&ratatui::layout::Margin { vertical: 1, horizontal: 1 });
                let cursor_x = get_display_cursor_x(&app.right_panel_input, app.right_panel_input_cursor);
                f.set_cursor(
                    input_area.x + cursor_x,
                    input_area.y,
                );
            }
        }
        FocusedPanel::Directory if app.show_directory => {
            if is_floating {
                // フローティングディレクトリパネルにカーソルを表示
                let area = panels::centered_rect(60, 80, f.size());
                let inner_area = area.inner(&ratatui::layout::Margin { vertical: 1, horizontal: 1 });
                let cursor_y = (app.selected_directory_index - app.directory_scroll_offset).min(inner_area.height.saturating_sub(1) as usize);
                f.set_cursor(inner_area.x, inner_area.y + cursor_y as u16);
            } else {
                // 非フローティングディレクトリパネルにカーソルを表示
                let directory_area = main_chunks[0].inner(&ratatui::layout::Margin { vertical: 1, horizontal: 1 });
                f.set_cursor(directory_area.x, directory_area.y + app.selected_directory_index as u16);
            }
        }
        FocusedPanel::Editor => {
            if let Some(active_pane) = app.pane_manager.get_active_pane() {
                if let Some(rect) = active_pane.rect {
                    let show_line_numbers = app.config.editor.show_line_numbers;
                    let horizontal_margin = app.config.ui.editor_margins.horizontal;
                    let current_window = app.current_window_mut();
                    let cursor_width = if current_window.buffer().is_empty() || current_window.cursor_y() >= current_window.buffer().len() {
                        0
                    } else {
                        current_window.buffer()[current_window.cursor_y()]
                            .graphemes(true)
                            .take(current_window.cursor_x())
                            .map(|g| g.width())
                            .sum::<usize>()
                    };
                    let line_number_width = if show_line_numbers { editor_constants::DEFAULT_LINE_NUMBER_WIDTH } else { 0 };
                    let separator_width = if show_line_numbers { editor_constants::LINE_NUMBER_SEPARATOR_WIDTH } else { 0 };
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
        _ => {} // その他の場合は何もしない
    }
}