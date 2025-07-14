use vim_editor::config::Theme;
use vim_editor::syntax::{highlight_syntax, count_leading_spaces, create_indent_spans, tokenize, TokenType};

#[test]
fn test_syntax_highlighting_integration() {
    // Rustコードの例
    let code_lines = vec![
        "fn main() {",
        "    let x = 42;",
        "    if x > 0 {",
        "        println!(\"Hello, world!\");",
        "    }",
        "}",
    ];
    
    for (i, line) in code_lines.iter().enumerate() {
        let theme = Theme::default();
        let spans = highlight_syntax(line, 4, &theme);
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
                // "        println!(\"Hello, world!\");"
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
fn test_tokenization_comprehensive() {
    let code = "fn main() -> Result<(), Box<dyn Error>> {";
    let tokens = tokenize(code);
    
    // トークンの種類をチェック
    let keyword_tokens: Vec<_> = tokens.iter().filter(|t| t.token_type == TokenType::Keyword).collect();
    assert!(!keyword_tokens.is_empty());
    assert!(keyword_tokens.iter().any(|t| t.content == "fn"));
    
    let function_tokens: Vec<_> = tokens.iter().filter(|t| t.token_type == TokenType::Function).collect();
    assert!(!function_tokens.is_empty());
    assert!(function_tokens.iter().any(|t| t.content == "main"));
    
    let type_tokens: Vec<_> = tokens.iter().filter(|t| t.token_type == TokenType::Type).collect();
    assert!(!type_tokens.is_empty());
    assert!(type_tokens.iter().any(|t| t.content == "Result"));
    assert!(type_tokens.iter().any(|t| t.content == "Box"));
    assert!(type_tokens.iter().any(|t| t.content == "Error"));
}

#[test]
fn test_string_handling() {
    let code = "let msg = \"Hello, \\\"world\\\"\";";
    let theme = Theme::default();
    let spans = highlight_syntax(code, 4, &theme);
    
    // 文字列部分が正しく処理されているかチェック
    assert!(spans.iter().any(|s| s.content.contains("Hello")));
    assert!(spans.iter().any(|s| s.content == "\""));
}

#[test]
fn test_comment_handling() {
    let code = "let x = 5; // this is a comment";
    let theme = Theme::default();
    let spans = highlight_syntax(code, 4, &theme);
    
    // コメント部分が正しく処理されているかチェック
    assert!(spans.iter().any(|s| s.content.contains("this is a comment")));
}

#[test]
fn test_macro_detection() {
    let code = "println!(\"test\"); vec![1, 2, 3];";
    let tokens = tokenize(code);
    
    let macro_tokens: Vec<_> = tokens.iter().filter(|t| t.token_type == TokenType::Macro).collect();
    assert!(!macro_tokens.is_empty());
    assert!(macro_tokens.iter().any(|t| t.content == "println!"));
    assert!(macro_tokens.iter().any(|t| t.content == "vec!"));
}

#[test]
fn test_number_detection() {
    let code = "let nums = [1, 42, 999];";
    let tokens = tokenize(code);
    
    let number_tokens: Vec<_> = tokens.iter().filter(|t| t.token_type == TokenType::Number).collect();
    assert_eq!(number_tokens.len(), 3);
    assert!(number_tokens.iter().any(|t| t.content == "1"));
    assert!(number_tokens.iter().any(|t| t.content == "42"));
    assert!(number_tokens.iter().any(|t| t.content == "999"));
}

#[test]
fn test_operator_detection() {
    let code = "std::collections::HashMap";
    let tokens = tokenize(code);
    
    let operator_tokens: Vec<_> = tokens.iter().filter(|t| t.token_type == TokenType::Operator).collect();
    assert_eq!(operator_tokens.len(), 2);
    assert!(operator_tokens.iter().all(|t| t.content == "::"));
}

#[test]
fn test_empty_and_whitespace_lines() {
    // 空行
    let theme = Theme::default();
    let spans = highlight_syntax("", 4, &theme);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content, "");
    
    // 空白のみの行
    let theme = Theme::default();
    let spans = highlight_syntax("    ", 4, &theme);
    assert_eq!(spans.len(), 1); // インデントスパンのみ
    
    // タブ混在（スペースのみをインデントとして扱う）
    let theme = Theme::default();
    let spans = highlight_syntax("\t    hello", 4, &theme);
    assert!(spans.len() >= 1);
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
        "            map.insert(format!(\"item_{}\", index), value);",
        "        } else {",
        "            eprintln!(\"Warning: negative value at index {}\", index);",
        "        }",
        "    }",
        "    Ok(map)",
        "}",
    ];
    
    for (line_num, line) in complex_code.iter().enumerate() {
        let theme = Theme::default();
        let spans = highlight_syntax(line, 4, &theme);
        
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