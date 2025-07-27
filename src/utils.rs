use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use std::{fs, path::PathBuf};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;


#[derive(Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub key: String,
}

#[derive(Deserialize)]
pub struct AppConfig {
    pub agent: AgentConfig,
}

pub fn load_agent_config(path: &str) -> Option<AgentConfig> {
    let data = fs::read_to_string(path).ok()?;
    let config: AppConfig = serde_json::from_str(&data).ok()?;
    Some(config.agent)
}

// ユーザー入力内容をAPIリクエストに反映する関数
pub async fn send_gemini_greeting_with_input(
    config_path: &str,
    input: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let agent = load_agent_config(config_path).ok_or("Agent config not found")?;
    let endpoint = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        agent.name, agent.key
    );
    let client = reqwest::Client::new();
    let body = format!(
        r#"{{"contents": [{{"parts": [{{"text": "{}"}}]}}]}}"#,
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
        Ok(json) => json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("No response")
            .to_string(),
        Err(_) => "No response".to_string(),
    };
    Ok(reply)
}

pub fn get_display_cursor_x(input: &str, cursor_grapheme: usize) -> u16 {
    input
        .graphemes(true)
        .take(cursor_grapheme)
        .map(|g| g.width())
        .sum::<usize>() as u16
}

pub fn list_directory(path: &PathBuf) -> Result<Vec<String>, std::io::Error> {
    let mut entries = Vec::new();
    if path.is_dir() {
        if let Ok(read_dir) = std::fs::read_dir(path) {
            for entry in read_dir.filter_map(|e| e.ok()) {
                let mut name = entry.file_name().to_string_lossy().to_string();
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    name.push('/');
                }
                entries.push(name);
            }
        }
    }
    entries.sort();
    if path.parent().is_some() {
        entries.insert(0, "../".to_string());
    }
    Ok(entries)
}