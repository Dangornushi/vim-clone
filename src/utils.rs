use unicode_segmentation::UnicodeSegmentation;

/// 指定方向にカーソルを移動（行・列範囲を自動調整）
pub fn move_cursor_left(cursor_x: &mut usize) {
    if *cursor_x > 0 {
        *cursor_x -= 1;
    }
}

pub fn move_cursor_right(cursor_x: &mut usize, line: &str) {
    let grapheme_count = line.graphemes(true).count();
    if *cursor_x < grapheme_count.saturating_sub(1) {
        *cursor_x += 1;
    }
}

pub fn move_cursor_up(cursor_y: &mut usize) {
    if *cursor_y > 0 {
        *cursor_y -= 1;
    }
}

pub fn move_cursor_down(cursor_y: &mut usize, buffer_len: usize) {
    if *cursor_y < buffer_len.saturating_sub(1) {
        *cursor_y += 1;
    }
}

/// 指定位置に文字を挿入
pub fn insert_char(line: &mut String, x: usize, c: char) {
    let byte_index = line.grapheme_indices(true).nth(x).map(|(i, _)| i).unwrap_or(line.len());
    line.insert(byte_index, c);
}

/// 指定位置の文字を削除
pub fn delete_char(line: &mut String, x: usize) -> Option<char> {
    let byte_index = line.grapheme_indices(true).nth(x).map(|(i, _)| i)?;
    let removed = line[byte_index..].chars().next()?;
    line.drain(byte_index..byte_index + removed.len_utf8());
    Some(removed)
}

/// インデント幅分のスペースを取得
pub fn get_indent(line: &str, indent_width: usize) -> String {
    line.chars().take_while(|&ch| ch == ' ').collect::<String>()
        + &" ".repeat(indent_width)
}

/// 行の分割（改行時）
pub fn split_line(line: &mut String, x: usize) -> String {
    let byte_index = line.grapheme_indices(true).nth(x).map(|(i, _)| i).unwrap_or(line.len());
    line.split_off(byte_index)
}

/// 行の結合（Backspaceで行頭時）
pub fn join_lines(prev_line: &mut String, next_line: String) {
    prev_line.push_str(&next_line);
}