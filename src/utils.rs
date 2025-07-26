use unicode_segmentation::UnicodeSegmentation;

/// カーソル移動の統一処理
pub fn move_cursor(cursor_x: &mut usize, cursor_y: &mut usize, direction: Direction, line: Option<&str>, buffer_len: usize) {
    match direction {
        Direction::Left => {
            if *cursor_x > 0 {
                *cursor_x -= 1;
            }
        }
        Direction::Right => {
            if let Some(line) = line {
                let grapheme_count = line.graphemes(true).count();
                if *cursor_x < grapheme_count.saturating_sub(1) {
                    *cursor_x += 1;
                }
            }
        }
        Direction::Up => {
            if *cursor_y > 0 {
                *cursor_y -= 1;
            }
        }
        Direction::Down => {
            if *cursor_y < buffer_len.saturating_sub(1) {
                *cursor_y += 1;
            }
        }
    }
}

#[derive(Clone, Copy)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
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

/// 行の分割と結合の統一処理
pub fn split_line(line: &mut String, x: usize) -> String {
    let byte_index = line.grapheme_indices(true).nth(x).map(|(i, _)| i).unwrap_or(line.len());
    line.split_off(byte_index)
}

pub fn join_lines(prev_line: &mut String, next_line: String) {
    prev_line.push_str(&next_line);
}

// 従来の個別関数は互換性のために残す（deprecated）
#[deprecated(note = "Use move_cursor with Direction::Left instead")]
pub fn move_cursor_left(cursor_x: &mut usize) {
    move_cursor(cursor_x, &mut 0, Direction::Left, None, 0);
}

#[deprecated(note = "Use move_cursor with Direction::Right instead")]
pub fn move_cursor_right(cursor_x: &mut usize, line: &str) {
    move_cursor(cursor_x, &mut 0, Direction::Right, Some(line), 0);
}

#[deprecated(note = "Use move_cursor with Direction::Up instead")]
pub fn move_cursor_up(cursor_y: &mut usize) {
    move_cursor(&mut 0, cursor_y, Direction::Up, None, 0);
}
#[deprecated(note = "Use move_cursor with Direction::Down instead")]
pub fn move_cursor_down(cursor_y: &mut usize, buffer_len: usize) {
    move_cursor(&mut 0, cursor_y, Direction::Down, None, buffer_len);
}
