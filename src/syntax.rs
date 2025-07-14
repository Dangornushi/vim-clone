use super::config::{SyntaxTheme, Theme};
use ratatui::{style::{Color, Style}, text::Span};
use std::collections::HashSet;

/// かっこの入れ子状態を追跡する構造体
#[derive(Debug, Clone)]
pub struct BracketState {
    pub stack: Vec<(char, usize)>, // (かっこの文字, 位置)
    pub unmatched_positions: Vec<usize>, // 対応しないかっこの位置
}

impl BracketState {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            unmatched_positions: Vec::new(),
        }
    }
    
    pub fn current_level(&self) -> usize {
        self.stack.len()
    }
    
    pub fn is_unmatched(&self, position: usize) -> bool {
        self.unmatched_positions.contains(&position)
    }
}

// HashSetを使用してキーワード検索を高速化
lazy_static::lazy_static! {
    static ref RUST_KEYWORDS: HashSet<&'static str> = {
        let keywords = [
            "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
            "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
            "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
            "where", "while", "async", "await", "dyn",
        ];
        keywords.iter().copied().collect()
    };
}


// 事前に計算されたスペース文字列を使用してメモリアロケーションを削減
const INDENT_SPACES: &str = "    ";

/// インデント部分のスペース数を計算する関数
#[inline]
pub fn count_leading_spaces(line: &str) -> usize {
    line.chars().take_while(|&ch| ch == ' ').count()
}

/// インデント部分のスパンを生成する関数
pub fn create_indent_spans(line: &str, indent_width: usize, theme: &Theme) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let space_count = count_leading_spaces(line);

    if indent_width == 0 || space_count == 0 {
        return spans;
    }

    let full_indents = space_count / indent_width;
    let remaining_spaces = space_count % indent_width;

    let indent_colors: Vec<Color> = theme.ui.indent_colors.iter().cloned().map(Into::into).collect();

    // 各インデントレベルに対応する背景色付きスペースを追加
    for i in 0..full_indents {
        let color = indent_colors[i % indent_colors.len()];
        spans.push(Span::styled(INDENT_SPACES.to_string(), Style::default().bg(color)));
    }

    // 残りのスペースがあれば追加（背景色なし）
    if remaining_spaces > 0 {
        spans.push(Span::from(line[full_indents * indent_width..space_count].to_string()));
    }

    spans
}

/// トークンの種類を表す列挙型
#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    Keyword,
    String,
    Number,
    Comment,
    Function,
    Macro,
    Type,
    Identifier,
    Operator,
    Symbol, // 記号（セミコロン、カンマ、ドットなど）
    Whitespace,
    Bracket(usize), // かっこ（入れ子レベル付き）
    UnmatchedBracket, // 対応しないかっこ
}

/// トークンを表す構造体
#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub content: String,
    pub token_type: TokenType,
    pub start: usize,
    pub end: usize,
}

/// 文字列をトークンに分割する関数（かっこの入れ子レベルを追跡）
pub fn tokenize(content: &str) -> Vec<Token> {
    let mut bracket_state = BracketState::new();
    tokenize_with_state(content, &mut bracket_state)
}

/// かっこの状態を保持しながらトークンに分割する関数
pub fn tokenize_with_state(content: &str, bracket_state: &mut BracketState) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;
    let mut in_string = false;
    
    while i < chars.len() {
        let start = i;
        
        // コメント
        if !in_string && i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
            while i < chars.len() {
                i += 1;
            }
            tokens.push(Token {
                content: chars[start..i].iter().collect(),
                token_type: TokenType::Comment,
                start,
                end: i,
            });
            continue;
        }
        
        // 文字列
        if chars[i] == '"' {
            in_string = !in_string;
            i += 1;
            tokens.push(Token {
                content: "\"".to_string(),
                token_type: TokenType::String,
                start,
                end: i,
            });
            continue;
        }
        
        if in_string {
            while i < chars.len() && chars[i] != '"' {
                i += 1;
            }
            if start < i {
                tokens.push(Token {
                    content: chars[start..i].iter().collect(),
                    token_type: TokenType::String,
                    start,
                    end: i,
                });
            }
            continue;
        }
        
        // かっこの処理
        if !in_string {
            match chars[i] {
                '(' | '[' | '{' => {
                    bracket_state.stack.push((chars[i], start));
                    let level = bracket_state.stack.len() - 1;
                    i += 1;
                    tokens.push(Token {
                        content: chars[start..i].iter().collect(),
                        token_type: TokenType::Bracket(level),
                        start,
                        end: i,
                    });
                    continue;
                }
                ')' | ']' | '}' => {
                    // 対応する開きかっこをチェック
                    let expected_open = match chars[i] {
                        ')' => '(',
                        ']' => '[',
                        '}' => '{',
                        _ => unreachable!(),
                    };
                    
                    // スタックから対応する開きかっこを探す
                    if let Some(&(last_bracket, _)) = bracket_state.stack.last() {
                        if last_bracket == expected_open {
                            bracket_state.stack.pop();
                        }
                    }
                    
                    let token_type = TokenType::Bracket(bracket_state.stack.len());
                    
                    i += 1;
                    tokens.push(Token {
                        content: chars[start..i].iter().collect(),
                        token_type,
                        start,
                        end: i,
                    });
                    continue;
                }
                _ => {}
            }
        }
        
        // 数値
        if chars[i].is_ascii_digit() {
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            tokens.push(Token {
                content: chars[start..i].iter().collect(),
                token_type: TokenType::Number,
                start,
                end: i,
            });
            continue;
        }
        
        // 演算子 :: (記号処理より前に配置)
        if i + 1 < chars.len() && chars[i] == ':' && chars[i + 1] == ':' {
            i += 2;
            tokens.push(Token {
                content: "::".to_string(),
                token_type: TokenType::Operator,
                start,
                end: i,
            });
            continue;
        }
        
        // 識別子・キーワード・マクロ
        if chars[i].is_alphanumeric() || chars[i] == '_' {
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            
            // マクロの場合は!も含める
            let mut word = chars[start..i].iter().collect::<String>();
            let mut token_type = classify_word(&word, &chars, i);
            
            // 次の文字が!の場合はマクロとして扱う
            if i < chars.len() && chars[i] == '!' {
                word.push('!');
                i += 1;
                token_type = TokenType::Macro;
            }
            
            tokens.push(Token {
                content: word,
                token_type,
                start,
                end: i,
            });
            continue;
        }
        
        // 空白
        if chars[i].is_whitespace() {
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            tokens.push(Token {
                content: chars[start..i].iter().collect(),
                token_type: TokenType::Whitespace,
                start,
                end: i,
            });
            continue;
        }
        
        // 記号の処理
        match chars[i] {
            ';' | ',' | '.' | '=' | '+' | '-' | '*' | '/' | '%' | 
            '&' | '|' | '^' | '!' | '?' | '<' | '>' | '#' | '@' | '$' => {
                i += 1;
                tokens.push(Token {
                    content: chars[start..i].iter().collect(),
                    token_type: TokenType::Symbol,
                    start,
                    end: i,
                });
                continue;
            }
            ':' => {
                // 単独の : (:: は既に演算子として処理済み)
                i += 1;
                tokens.push(Token {
                    content: chars[start..i].iter().collect(),
                    token_type: TokenType::Symbol,
                    start,
                    end: i,
                });
                continue;
            }
            _ => {}
        }
        
        // その他の文字
        i += 1;
        tokens.push(Token {
            content: chars[start..i].iter().collect(),
            token_type: TokenType::Identifier,
            start,
            end: i,
        });
    }
    
    // 注意: 複数行にわたるかっこの場合、ここでは未対応として扱わない
    // 未対応のかっこの検出は、ファイル全体の処理が完了した後に行う
    
    tokens
}

/// 単語の種類を分類する関数
fn classify_word(word: &str, chars: &[char], current_pos: usize) -> TokenType {
    if RUST_KEYWORDS.contains(word) {
        return TokenType::Keyword;
    }
    
    // 次の文字をチェック（関数呼び出しの検出用）
    let mut next_char_idx = current_pos;
    while next_char_idx < chars.len() && chars[next_char_idx].is_whitespace() {
        next_char_idx += 1;
    }
    
    // 関数呼び出しの検出（!が続く場合はマクロなので除外）
    if next_char_idx < chars.len() && chars[next_char_idx] == '(' {
        return TokenType::Function;
    }
    
    if word.chars().next().map_or(false, |c| c.is_ascii_uppercase()) {
        return TokenType::Type;
    }
    
    TokenType::Identifier
}

/// トークンをスパンに変換する関数
pub fn token_to_span(token: &Token, theme: &SyntaxTheme) -> Span<'static> {
    let bracket_colors: Vec<Color> = theme.bracket_colors.iter().cloned().map(Into::into).collect();
    let style = match &token.token_type {
        TokenType::Keyword => Style::default().fg(theme.keyword.clone().into()),
        TokenType::String => Style::default().fg(theme.string.clone().into()),
        TokenType::Number => Style::default().fg(theme.number.clone().into()),
        TokenType::Comment => Style::default().fg(theme.comment.clone().into()),
        TokenType::Function => Style::default().fg(theme.function.clone().into()),
        TokenType::Macro => Style::default().fg(theme.r#macro.clone().into()),
        TokenType::Type => Style::default().fg(theme.r#type.clone().into()),
        TokenType::Identifier => Style::default().fg(theme.identifier.clone().into()),
        TokenType::Operator => Style::default().fg(theme.operator.clone().into()),
        TokenType::Symbol => Style::default().fg(theme.symbol.clone().into()),
        TokenType::Whitespace => Style::default(),
        TokenType::Bracket(level) => {
            let color = bracket_colors[*level % bracket_colors.len()];
            Style::default().fg(color)
        }
        TokenType::UnmatchedBracket => Style::default()
            .fg(theme.unmatched_bracket_fg.clone().into())
            .bg(theme.unmatched_bracket_bg.clone().into()),
    };
    
    Span::styled(token.content.clone(), style)
}

/// メインのシンタックスハイライト関数
pub fn highlight_syntax(line_str: &str, indent_width: usize, theme: &Theme) -> Vec<Span<'static>> {
    let mut bracket_state = BracketState::new();
    highlight_syntax_with_state(line_str, indent_width, &mut bracket_state, theme)
}

/// ファイル全体の未対応かっこを検出する関数
pub fn detect_unmatched_brackets_in_file(lines: &[String]) -> Vec<(usize, usize)> {
    let mut bracket_state = BracketState::new();
    let mut unmatched_positions = Vec::new();
    
    // 全ての行を処理して、未対応のかっこを検出
    for (line_index, line_str) in lines.iter().enumerate() {
        let space_count = count_leading_spaces(line_str);
        let content_part = &line_str[space_count..];
        
        if !content_part.is_empty() {
            // この行のトークンを処理
            let chars: Vec<char> = content_part.chars().collect();
            let mut i = 0;
            let mut in_string = false;
            
            while i < chars.len() {
                // コメントの処理
                if !in_string && i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                    // 行コメントの場合、この行の残りは無視
                    break;
                }
                
                // 文字列の処理
                if chars[i] == '"' {
                    in_string = !in_string;
                    i += 1;
                    continue;
                }
                
                if in_string {
                    i += 1;
                    continue;
                }
                
                // かっこの処理
                match chars[i] {
                    '(' | '[' | '{' => {
                        bracket_state.stack.push((chars[i], line_index));
                    }
                    ')' | ']' | '}' => {
                        let expected_open = match chars[i] {
                            ')' => '(',
                            ']' => '[',
                            '}' => '{',
                            _ => unreachable!(),
                        };
                        
                        let mut matched = false;
                        if let Some(&(last_bracket, _)) = bracket_state.stack.last() {
                            if last_bracket == expected_open {
                                bracket_state.stack.pop();
                                matched = true;
                            }
                        }
                        
                        // 対応しない閉じかっこ
                        if !matched {
                            unmatched_positions.push((line_index, space_count + i));
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        }
    }
    
    // 最後に残っている開きかっこは未対応
    for &(_, line_index) in &bracket_state.stack {
        unmatched_positions.push((line_index, 0)); // 簡略化のため位置は0
    }
    
    unmatched_positions
}

/// かっこの状態を保持しながらシンタックスハイライトを行う関数
pub fn highlight_syntax_with_state(line_str: &str, indent_width: usize, bracket_state: &mut BracketState, theme: &Theme) -> Vec<Span<'static>> {
    highlight_syntax_with_unmatched(line_str, indent_width, bracket_state, &[], theme)
}

/// 未対応かっこの情報を含むシンタックスハイライト関数
pub fn highlight_syntax_with_unmatched(
    line_str: &str,
    indent_width: usize,
    bracket_state: &mut BracketState,
    unmatched_positions: &[(usize, usize)],
    theme: &Theme,
) -> Vec<Span<'static>> {
    if line_str.is_empty() {
        return vec![Span::from("")];
    }

    let mut spans = Vec::with_capacity(16);

    // インデント部分を処理
    let indent_spans = create_indent_spans(line_str, indent_width, theme);
    spans.extend(indent_spans);

    // コンテンツ部分を処理
    let space_count = count_leading_spaces(line_str);
    let content_part = &line_str[space_count..];

    if content_part.is_empty() {
        return spans;
    }

    // トークン化してスパンに変換（かっこの状態を保持）
    let mut tokens = tokenize_with_state(content_part, bracket_state);

    // 未対応のかっこをチェックして、該当するトークンを更新
    for token in &mut tokens {
        if let TokenType::Bracket(_) = token.token_type {
            let absolute_position = space_count + token.start;
            if unmatched_positions.iter().any(|(_, pos)| *pos == absolute_position) {
                token.token_type = TokenType::UnmatchedBracket;
            }
        }
    }

    for token in tokens {
        spans.push(token_to_span(&token, &theme.syntax));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_leading_spaces() {
        assert_eq!(count_leading_spaces(""), 0);
        assert_eq!(count_leading_spaces("hello"), 0);
        assert_eq!(count_leading_spaces("    hello"), 4);
        assert_eq!(count_leading_spaces("        world"), 8);
        assert_eq!(count_leading_spaces("   "), 3);
    }

    #[test]
    fn test_create_indent_spans() {
        let theme = Theme::default();
        let spans = create_indent_spans("    hello", 4, &theme);
        assert_eq!(spans.len(), 1);
        
        let spans = create_indent_spans("        hello", 4, &theme);
        assert_eq!(spans.len(), 2);
        
        let spans = create_indent_spans("      hello", 4, &theme);
        assert_eq!(spans.len(), 2); // 4スペース + 2スペース
        
        let spans = create_indent_spans("hello", 4, &theme);
        assert_eq!(spans.len(), 0);
    }

    #[test]
    fn test_tokenize_simple() {
        let tokens = tokenize("fn main()");
        assert_eq!(tokens.len(), 5); // fn, space, main, (, )
        assert_eq!(tokens[0].content, "fn");
        assert_eq!(tokens[0].token_type, TokenType::Keyword);
        assert_eq!(tokens[1].content, " ");
        assert_eq!(tokens[1].token_type, TokenType::Whitespace);
        assert_eq!(tokens[2].content, "main");
        assert_eq!(tokens[2].token_type, TokenType::Function);
        assert_eq!(tokens[3].content, "(");
        assert_eq!(tokens[3].token_type, TokenType::Bracket(0));
        assert_eq!(tokens[4].content, ")");
        assert_eq!(tokens[4].token_type, TokenType::Bracket(0));
    }

    #[test]
    fn test_tokenize_string() {
        let tokens = tokenize("\"hello world\"");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].content, "\"");
        assert_eq!(tokens[0].token_type, TokenType::String);
        assert_eq!(tokens[1].content, "hello world");
        assert_eq!(tokens[1].token_type, TokenType::String);
        assert_eq!(tokens[2].content, "\"");
        assert_eq!(tokens[2].token_type, TokenType::String);
    }

    #[test]
    fn test_tokenize_comment() {
        let tokens = tokenize("// this is a comment");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].content, "// this is a comment");
        assert_eq!(tokens[0].token_type, TokenType::Comment);
    }

    #[test]
    fn test_tokenize_numbers() {
        let tokens = tokenize("let x = 42;");
        let number_token = tokens.iter().find(|t| t.token_type == TokenType::Number);
        assert!(number_token.is_some());
        assert_eq!(number_token.unwrap().content, "42");
    }

    #[test]
    fn test_classify_word() {
        let chars: Vec<char> = "fn main() {}".chars().collect();
        assert_eq!(classify_word("fn", &chars, 2), TokenType::Keyword);
        assert_eq!(classify_word("main", &chars, 7), TokenType::Function);
        
        let chars: Vec<char> = "String::new".chars().collect();
        assert_eq!(classify_word("String", &chars, 6), TokenType::Type);
    }

    #[test]
    fn test_macro_detection() {
        let tokens = tokenize("println!(\"Hello\")");
        let macro_token = tokens.iter().find(|t| t.token_type == TokenType::Macro);
        assert!(macro_token.is_some());
        assert_eq!(macro_token.unwrap().content, "println!");
        
        let tokens = tokenize("vec![1, 2, 3]");
        let macro_token = tokens.iter().find(|t| t.token_type == TokenType::Macro);
        assert!(macro_token.is_some());
        assert_eq!(macro_token.unwrap().content, "vec!");
        
        let tokens = tokenize("format!(\"test {}\", value)");
        let macro_token = tokens.iter().find(|t| t.token_type == TokenType::Macro);
        assert!(macro_token.is_some());
        assert_eq!(macro_token.unwrap().content, "format!");
    }

    #[test]
    fn test_function_vs_macro_distinction() {
        // 関数呼び出し
        let tokens = tokenize("some_function()");
        let func_token = tokens.iter().find(|t| t.token_type == TokenType::Function);
        assert!(func_token.is_some());
        assert_eq!(func_token.unwrap().content, "some_function");
        
        // マクロ呼び出し
        let tokens = tokenize("some_macro!()");
        let macro_token = tokens.iter().find(|t| t.token_type == TokenType::Macro);
        assert!(macro_token.is_some());
        assert_eq!(macro_token.unwrap().content, "some_macro!");
    }

    #[test]
    fn test_highlight_syntax_empty() {
        let theme = Theme::default();
        let spans = highlight_syntax("", 4, &theme);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "");
    }

    #[test]
    fn test_highlight_syntax_with_indent() {
        let theme = Theme::default();
        let spans = highlight_syntax("    fn main()", 4, &theme);
        assert!(spans.len() > 1);
        // 最初のスパンはインデント
        // 後続のスパンはシンタックスハイライト
    }

    #[test]
    fn test_rust_keywords_contains() {
        assert!(RUST_KEYWORDS.contains("fn"));
        assert!(RUST_KEYWORDS.contains("let"));
        assert!(RUST_KEYWORDS.contains("mut"));
        assert!(!RUST_KEYWORDS.contains("hello"));
        assert!(!RUST_KEYWORDS.contains("world"));
    }

    #[test]
    fn test_bracket_nesting() {
        let tokens = tokenize("((()))");
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].token_type, TokenType::Bracket(0)); // (
        assert_eq!(tokens[1].token_type, TokenType::Bracket(1)); // (
        assert_eq!(tokens[2].token_type, TokenType::Bracket(2)); // (
        assert_eq!(tokens[3].token_type, TokenType::Bracket(2)); // )
        assert_eq!(tokens[4].token_type, TokenType::Bracket(1)); // )
        assert_eq!(tokens[5].token_type, TokenType::Bracket(0)); // )
    }

    #[test]
    fn test_mixed_brackets() {
        let tokens = tokenize("([{}])");
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].token_type, TokenType::Bracket(0)); // (
        assert_eq!(tokens[1].token_type, TokenType::Bracket(1)); // [
        assert_eq!(tokens[2].token_type, TokenType::Bracket(2)); // {
        assert_eq!(tokens[3].token_type, TokenType::Bracket(2)); // }
        assert_eq!(tokens[4].token_type, TokenType::Bracket(1)); // ]
        assert_eq!(tokens[5].token_type, TokenType::Bracket(0)); // )
    }

    #[test]
    fn test_bracket_state() {
        let mut state = BracketState::new();
        assert_eq!(state.current_level(), 0);
        
        state.stack.push(('(', 0));
        assert_eq!(state.current_level(), 1);
        
        state.stack.push(('[', 1));
        assert_eq!(state.current_level(), 2);
        
        state.stack.pop();
        assert_eq!(state.current_level(), 1);
        
        state.stack.pop();
        assert_eq!(state.current_level(), 0);
    }

    #[test]
    fn test_symbol_detection() {
        let tokens = tokenize("let x = 42;");
        let symbol_tokens: Vec<_> = tokens.iter().filter(|t| t.token_type == TokenType::Symbol).collect();
        assert_eq!(symbol_tokens.len(), 2); // = と ;
        assert_eq!(symbol_tokens[0].content, "=");
        assert_eq!(symbol_tokens[1].content, ";");
        
        let tokens = tokenize("vec![1, 2, 3]");
        let symbol_tokens: Vec<_> = tokens.iter().filter(|t| t.token_type == TokenType::Symbol).collect();
        assert_eq!(symbol_tokens.len(), 2); // , と ,
        assert_eq!(symbol_tokens[0].content, ",");
        assert_eq!(symbol_tokens[1].content, ",");
    }

    #[test]
    fn test_operator_vs_symbol() {
        let tokens = tokenize("String::new()");
        let operator_token = tokens.iter().find(|t| t.token_type == TokenType::Operator);
        assert!(operator_token.is_some());
        assert_eq!(operator_token.unwrap().content, "::");
        
        let tokens = tokenize("let x: i32 = 42;");
        let symbol_tokens: Vec<_> = tokens.iter().filter(|t| t.token_type == TokenType::Symbol).collect();
        let colon_token = symbol_tokens.iter().find(|t| t.content == ":");
        assert!(colon_token.is_some());
    }

    #[test]
    fn test_bracket_matching_basic() {
        // 現在は行ごとの処理では未対応のかっこを検出しない
        // 代わりに、かっこの入れ子レベルが正しく計算されることをテスト
        
        // 正しく対応している場合
        let tokens = tokenize("([{}])");
        let bracket_tokens: Vec<_> = tokens.iter().filter(|t| matches!(t.token_type, TokenType::Bracket(_))).collect();
        assert_eq!(bracket_tokens.len(), 6);
        
        // 開きかっこが多い場合でも、エラーにはならない（複数行の可能性があるため）
        let tokens = tokenize("((())");
        let bracket_tokens: Vec<_> = tokens.iter().filter(|t| matches!(t.token_type, TokenType::Bracket(_))).collect();
        assert_eq!(bracket_tokens.len(), 5);
        
        // 閉じかっこが多い場合も同様
        let tokens = tokenize("(()))");
        let bracket_tokens: Vec<_> = tokens.iter().filter(|t| matches!(t.token_type, TokenType::Bracket(_))).collect();
        assert_eq!(bracket_tokens.len(), 5);
    }

    #[test]
    fn test_multiline_bracket_state() {
        // 複数行にわたるかっこの状態をテスト
        let mut bracket_state = BracketState::new();
        
        // 1行目: if true {
        let tokens1 = tokenize_with_state("if true {", &mut bracket_state);
        let unmatched1: Vec<_> = tokens1.iter().filter(|t| t.token_type == TokenType::UnmatchedBracket).collect();
        assert_eq!(unmatched1.len(), 0); // まだエラーではない
        
        // 2行目: println!("Hello");
        let tokens2 = tokenize_with_state("    println!(\"Hello\");", &mut bracket_state);
        let unmatched2: Vec<_> = tokens2.iter().filter(|t| t.token_type == TokenType::UnmatchedBracket).collect();
        assert_eq!(unmatched2.len(), 0); // 正常
        
        // 3行目: }
        let tokens3 = tokenize_with_state("}", &mut bracket_state);
        let unmatched3: Vec<_> = tokens3.iter().filter(|t| t.token_type == TokenType::UnmatchedBracket).collect();
        assert_eq!(unmatched3.len(), 0); // 正常に閉じられた
        
        // 最終的にスタックは空であるべき
        assert_eq!(bracket_state.stack.len(), 0);
    }

    #[test]
    fn test_detect_unmatched_brackets_in_file() {
        // 正常なファイル
        let normal_lines = vec![
            "fn main() {".to_string(),
            "    println!(\"Hello\");".to_string(),
            "}".to_string(),
        ];
        let unmatched = detect_unmatched_brackets_in_file(&normal_lines);
        assert_eq!(unmatched.len(), 0);
        
        // 開きかっこが多いファイル
        let missing_close = vec![
            "fn main() {".to_string(),
            "    if true {".to_string(),
            "        println!(\"Hello\");".to_string(),
            "    }".to_string(),
            "// } が不足".to_string(),
        ];
        
        let unmatched = detect_unmatched_brackets_in_file(&missing_close);
        assert!(unmatched.len() > 0);
        
        // 閉じかっこが多いファイル
        let extra_close = vec![
            "fn main() {".to_string(),
            "    println!(\"Hello\");".to_string(),
            "}".to_string(),
            "}".to_string(), // 余分な }
        ];
        let unmatched = detect_unmatched_brackets_in_file(&extra_close);
        println!("Extra close brackets: {:?}", unmatched);
        assert!(unmatched.len() > 0);
    }
}