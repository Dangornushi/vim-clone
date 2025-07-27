use crate::config::load_agent_config;
use reqwest::header::CONTENT_TYPE;

pub async fn send_gemini_greeting(config_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let agent = crate::config::load_agent_config(config_path).ok_or("Agent config not found")?;
    let endpoint = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        agent.name, agent.key
    );
    let client = reqwest::Client::new();
    let body = r#"{
        "contents": [
            { "parts": [ { "text": "挨拶して" } ] }
        ]
    }"#;
    let res = client
        .post(&endpoint)
        .header(CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await?;
    let text = res.text().await?;
    // textフィールドのみ抽出
    let reply = match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(json) => json["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or("No response").to_string(),
        Err(_) => "No response".to_string(),
    };
    Ok(reply)
}

use unicode_width::UnicodeWidthStr;
pub fn get_display_cursor_x(input: &str, cursor_grapheme: usize) -> u16 {
    use unicode_segmentation::UnicodeSegmentation;
    use unicode_width::UnicodeWidthStr;
    input
        .graphemes(true)
        .take(cursor_grapheme)
        .map(|g| g.width())
        .sum::<usize>() as u16
}
// ユーザー入力内容をAPIリクエストに反映する関数
pub async fn send_gemini_greeting_with_input(config_path: &str, input: &str) -> Result<String, Box<dyn std::error::Error>> {
    let agent = crate::config::load_agent_config(config_path).ok_or("Agent config not found")?;
    let endpoint = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        agent.name, agent.key
    );
    let client = reqwest::Client::new();
    let body = format!(
        r#"{{
            "contents": [
                {{ "parts": [ {{ "text": "{}" }} ] }}
            ]
        }}"#,
        input
    );
    let res = client
        .post(&endpoint)
        .header(CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await?;
    let text = res.text().await?;
    let reply = match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(json) => json["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or("No response").to_string(),
        Err(_) => "No response".to_string(),
    };
    Ok(reply)
}

/// カーソル移動の統一処理
pub fn move_cursor(cursor_x: &mut usize, cursor_y: &mut usize, direction: Direction, line: Option<&str>, buffer_len: usize) {
    use unicode_width::UnicodeWidthStr;
    match direction {
        Direction::Left => {
            if let Some(line) = line {
                use unicode_segmentation::UnicodeSegmentation;
                let graphemes: Vec<&str> = line.graphemes(true).collect();
                if *cursor_x > 0 {
                    // 左に移動する場合、前の文字の表示幅分だけ減算
                    let prev = graphemes.get(*cursor_x - 1).unwrap_or(&"");
                    let prev_width = UnicodeWidthStr::width(*prev);
                    *cursor_x = cursor_x.saturating_sub(prev_width.max(1));
                }
            } else if *cursor_x > 0 {
                *cursor_x -= 1;
            }
        }
        Direction::Right => {
            if let Some(line) = line {
                use unicode_segmentation::UnicodeSegmentation;
                let graphemes: Vec<&str> = line.graphemes(true).collect();
                let grapheme_count = graphemes.len();
                if *cursor_x < grapheme_count {
                    // 右に移動する場合、次の文字の表示幅分だけ加算
                    let next = graphemes.get(*cursor_x).unwrap_or(&"");
                    let next_width = UnicodeWidthStr::width(*next);
                    *cursor_x += next_width.max(1);
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
    use unicode_segmentation::UnicodeSegmentation;
    let byte_index = line.grapheme_indices(true).nth(x).map(|(i, _)| i).unwrap_or(line.len());
    line.insert(byte_index, c);
}

/// 指定位置の文字を削除
pub fn delete_char(line: &mut String, x: usize) -> Option<char> {
    use unicode_segmentation::UnicodeSegmentation;
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
    use unicode_segmentation::UnicodeSegmentation;
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
