use std::collections::HashSet;
use vim_editor::config::Theme;
use vim_editor::syntax::{highlight_syntax_with_state, count_leading_spaces, create_indent_spans, BracketState};

#[test]
fn test_syntax_highlighting_integration() {
    // Rustコードの例
    let code_lines = ["fn main() {",
        "    let x = 42;",
        "    if x > 0 {",
        r#"        println!("Hello, world!");"#,
        "    }",
        "}"];
    
    let theme = Theme::default();
    let unmatched_brackets = HashSet::new();
    for (i, line) in code_lines.iter().enumerate() {
        let spans = highlight_syntax_with_state(line, i, 4, &mut BracketState::new(), &theme, &unmatched_brackets);
        assert!(!spans.is_empty(), "Line {} should have spans", i);
        
        // 各行の内容をチェック
        match i {
            0 => {
                // "fn main() {"
                assert!(spans.iter().any(|s| s.content.contains("fn")));
                assert!(spans.iter().any(|s| s.content.contains("main")));
            },
            1 => {
                // "    let x = 42;"
                assert!(spans.len() >= 2); // インデント + コンテンツ
                assert!(spans.iter().any(|s| s.content.contains("let")));
                assert!(spans.iter().any(|s| s.content.contains("42")));
            },
            3 => {
                // "        println!("Hello, world!");"
                assert!(spans.len() >= 3); // 2レベルインデント + コンテンツ
                assert!(spans.iter().any(|s| s.content.contains("println!")));
                assert!(spans.iter().any(|s| s.content.contains("Hello, world!")));
            },
            _ => {}
        }
    }
}

#[test]
fn test_indent_levels() {
    let test_cases = vec![
        ("", 0),
        ("hello", 0),
        ("    hello", 4),
        ("        hello", 8),
        ("            hello", 12),
    ];
    
    for (input, expected) in test_cases {
        assert_eq!(count_leading_spaces(input), expected);
    }
}

#[test]
fn test_indent_spans_creation() {
    // 1レベルインデント
    let theme = Theme::default();
    let spans = create_indent_spans("    hello", 4, &theme);
    assert_eq!(spans.len(), 1);
    
    // 2レベルインデント
    let theme = Theme::default();
    let spans = create_indent_spans("        hello", 4, &theme);
    assert_eq!(spans.len(), 2);
    
    // 不完全なインデント
    let theme = Theme::default();
    let spans = create_indent_spans("      hello", 4, &theme);
    assert_eq!(spans.len(), 2); // 4スペース + 2スペース
    
    // インデントなし
    let theme = Theme::default();
    let spans = create_indent_spans("hello", 4, &theme);
    assert_eq!(spans.len(), 0);
}

#[test]
fn test_string_handling() {
    let code = r#"let msg = "Hello, \"world\"!";"#;
    let theme = Theme::default();
    let spans = highlight_syntax_with_state(code, 0, 0, &mut BracketState::new(), &theme, &HashSet::new());
    
    // 文字列部分が正しく処理されているかチェック
    assert!(spans.iter().any(|s| s.content.contains("Hello")));
    assert!(spans.iter().any(|s| s.content == r#""Hello, \"world\"!""#));
}

#[test]
fn test_comment_handling() {
    let code = "let x = 5; // this is a comment";
    let theme = Theme::default();
    let spans = highlight_syntax_with_state(code, 0, 0, &mut BracketState::new(), &theme, &HashSet::new());
    
    // コメント部分が正しく処理されているかチェック
    assert!(spans.iter().any(|s| s.content.contains("this is a comment")));
}

#[test]
fn test_empty_and_whitespace_lines() {
    let theme = Theme::default();
    let unmatched_brackets = &HashSet::new();
    // 空行
    let spans = highlight_syntax_with_state("", 0, 0, &mut BracketState::new(), &theme, unmatched_brackets);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content, "");
    
    // 空白のみの行
    let spans = highlight_syntax_with_state("    ", 0, 4, &mut BracketState::new(), &theme, unmatched_brackets);
    assert_eq!(spans.len(), 1); // 4スペースのインデントスパン
    assert_eq!(spans[0].content, "    ");
    
    // タブ混在（スペースのみをインデントとして扱う）
    let spans = highlight_syntax_with_state("\t    hello", 0, 0, &mut BracketState::new(), &theme, unmatched_brackets);
    assert!(!spans.is_empty());
}

#[test]
fn test_complex_rust_code() {
    let complex_code = vec![
        "use std::collections::HashMap;",
        "",
        "fn process_data(data: &[i32]) -> Result<HashMap<String, i32>, Box<dyn std::error::Error>> {",
        "    let mut map = HashMap::new();",
        "    for (index, &value) in data.iter().enumerate() {",
        "        if value > 0 {",
        r#"            map.insert(format!("item_{}", index), value);"#,
        "        } else {",
        r#"            eprintln!("Warning: negative value at index {}", index);"#,
        "        }",
        "    }",
        "    Ok(map)",
        "}",
    ];
    
    let theme = Theme::default();
    let unmatched_brackets = HashSet::new();
    for (line_num, line) in complex_code.iter().enumerate() {
        let spans = highlight_syntax_with_state(line, line_num, 4, &mut BracketState::new(), &theme, &unmatched_brackets);
        
        // 各行が適切に処理されているかチェック
        if !line.trim().is_empty() {
            assert!(!spans.is_empty(), "Line {} should not be empty", line_num);
        }
        
        // 特定の行の詳細チェック
        match line_num {
            0 => {
                // use文
                assert!(spans.iter().any(|s| s.content.contains("use")));
                assert!(spans.iter().any(|s| s.content.contains("std")));
            },
            2 => {
                // 関数定義
                assert!(spans.iter().any(|s| s.content.contains("fn")));
                assert!(spans.iter().any(|s| s.content.contains("process_data")));
            },
            6 => {
                // format!マクロ
                assert!(spans.iter().any(|s| s.content.contains("format!")));
            },
            _ => {}
        }
    }
}