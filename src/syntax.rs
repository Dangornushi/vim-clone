use super::config::{SyntaxTheme, Theme};
use ratatui::{style::{Color, Style}, text::Span};
use std::collections::HashSet;
use std::iter::Peekable;
use std::str::CharIndices;

/// かっこの入れ子状態を追跡する構造体
#[derive(Debug, Clone, Default)]
pub struct BracketState {
    pub stack: Vec<(char, usize, usize)>, // (かっこの文字, 行番号, 列番号)
}

impl BracketState {
    pub fn new() -> Self {
        Self::default()
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
    Symbol,
    Whitespace,
    Bracket { level: usize, is_matched: bool },
}

/// トークンを表す構造体
#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub content: String,
    pub token_type: TokenType,
    pub start: usize,
    pub end: usize,
}

/// トークナイザの状態を管理する構造体
struct Tokenizer<'a> {
    content: &'a str,
    chars: Peekable<CharIndices<'a>>,
    line_idx: usize,
    content_start_col: usize,
    bracket_state: &'a mut BracketState,
}

impl<'a> Tokenizer<'a> {
    fn new(
        content: &'a str,
        line_idx: usize,
        content_start_col: usize,
        bracket_state: &'a mut BracketState,
    ) -> Self {
        Self {
            content,
            chars: content.char_indices().peekable(),
            line_idx,
            content_start_col,
            bracket_state,
        }
    }

    fn run(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while self.chars.peek().is_some() {
            tokens.push(self.next_token());
        }
        tokens
    }

    fn next_token(&mut self) -> Token {
        let (start, ch) = self.peek_char_and_index().unwrap();
        match ch {
            '/' if self.peek_next_char() == Some('/') => self.tokenize_comment(start),
            '"' => self.tokenize_quoted_string(start, '"'),
            '\'' => self.tokenize_char_literal_or_lifetime(start),
            '(' | '[' | '{' => self.tokenize_open_bracket(start, ch),
            ')' | ']' | '}' => self.tokenize_close_bracket(start, ch),
            c if c.is_ascii_digit() => self.tokenize_number(start),
            c if c.is_alphanumeric() || c == '_' => self.tokenize_identifier(start),
            c if c.is_whitespace() => self.tokenize_whitespace(start),
            ':' if self.peek_next_char() == Some(':') => self.tokenize_operator(start, 2),
            _ => self.tokenize_symbol(start),
        }
    }

    fn peek_char_and_index(&mut self) -> Option<(usize, char)> {
        self.chars.peek().cloned()
    }

    fn peek_next_char(&mut self) -> Option<char> {
        let mut iter = self.chars.clone();
        iter.next();
        iter.peek().map(|&(_, c)| c)
    }

    fn advance(&mut self) -> Option<(usize, char)> {
        self.chars.next()
    }

    fn take_while<F>(&mut self, mut predicate: F) -> usize
    where
        F: FnMut(char) -> bool,
    {
        while let Some((_, ch)) = self.peek_char_and_index() {
            if predicate(ch) {
                self.advance();
            } else {
                break;
            }
        }
        self.peek_char_and_index().map_or(self.content.len(), |(i, _)| i)
    }

    fn tokenize_comment(&mut self, start: usize) -> Token {
        let end = self.take_while(|_| true);
        Token {
            content: self.content[start..end].to_string(),
            token_type: TokenType::Comment,
            start,
            end,
        }
    }

    fn tokenize_quoted_string(&mut self, start: usize, quote_char: char) -> Token {
        self.advance(); // Consume opening quote
        let mut escaped = false;
        let _end = self.take_while(|c| {
            if escaped {
                escaped = false;
                return true;
            }
            if c == '\\' {
                escaped = true;
                return true;
            }
            c != quote_char
        });
        self.advance(); // Consume closing quote
        let final_end = self.peek_char_and_index().map_or(self.content.len(), |(i, _)| i);
        Token {
            content: self.content[start..final_end].to_string(),
            token_type: TokenType::String,
            start,
            end: final_end,
        }
    }

    fn tokenize_char_literal_or_lifetime(&mut self, start: usize) -> Token {
        let mut iter = self.chars.clone();
        iter.next(); // consume '\''

        if let Some(&(_, c1)) = iter.peek() {
            if c1.is_alphanumeric() || c1 == '_' {
                iter.next(); // consume c1
                if iter.peek().map_or(true, |&(_, c2)| c2 != '\'') {
                    // It's a lifetime
                    self.advance(); // consume '\''
                    let end = self.take_while(|c| c.is_alphanumeric() || c == '_');
                    return Token {
                        content: self.content[start..end].to_string(),
                        token_type: TokenType::Identifier,
                        start,
                        end,
                    };
                }
            }
        }
        // It's a char literal
        self.tokenize_quoted_string(start, '\'')
    }

    fn tokenize_open_bracket(&mut self, start: usize, ch: char) -> Token {
        self.advance();
        let level = self.bracket_state.stack.len();
        let col = self.content_start_col + start;
        self.bracket_state.stack.push((ch, self.line_idx, col));
        Token {
            content: ch.to_string(),
            token_type: TokenType::Bracket { level, is_matched: true },
            start,
            end: start + 1,
        }
    }

    fn tokenize_close_bracket(&mut self, start: usize, ch: char) -> Token {
        self.advance();
        let expected_open = match ch {
            ')' => '(',
            ']' => '[',
            '}' => '{',
            _ => unreachable!(),
        };
        let mut is_matched = false;
        let mut level = self.bracket_state.stack.len();
        if let Some(&(last_bracket, _, _)) = self.bracket_state.stack.last() {
            if last_bracket == expected_open {
                self.bracket_state.stack.pop();
                is_matched = true;
                level = self.bracket_state.stack.len();
            }
        }
        Token {
            content: ch.to_string(),
            token_type: TokenType::Bracket { level, is_matched },
            start,
            end: start + 1,
        }
    }

    fn tokenize_number(&mut self, start: usize) -> Token {
        let end = self.take_while(|c| c.is_ascii_digit());
        Token {
            content: self.content[start..end].to_string(),
            token_type: TokenType::Number,
            start,
            end,
        }
    }

    fn tokenize_identifier(&mut self, start: usize) -> Token {
        let end = self.take_while(|c| c.is_alphanumeric() || c == '_');
        let mut content = self.content[start..end].to_string();
        let mut token_type = classify_word(&content, self.peek_char_and_index().map(|(_, c)| c));

        if self.peek_char_and_index().map(|(_, c)| c) == Some('!') {
            self.advance();
            content.push('!');
            token_type = TokenType::Macro;
        }
        let final_end = self.peek_char_and_index().map_or(self.content.len(), |(i, _)| i);

        Token { content, token_type, start, end: final_end }
    }

    fn tokenize_whitespace(&mut self, start: usize) -> Token {
        let end = self.take_while(|c| c.is_whitespace());
        Token {
            content: self.content[start..end].to_string(),
            token_type: TokenType::Whitespace,
            start,
            end,
        }
    }

    fn tokenize_operator(&mut self, start: usize, len: usize) -> Token {
        for _ in 0..len {
            self.advance();
        }
        Token {
            content: self.content[start..start + len].to_string(),
            token_type: TokenType::Operator,
            start,
            end: start + len,
        }
    }

    fn tokenize_symbol(&mut self, start: usize) -> Token {
        let ch = self.advance().unwrap().1;
        Token {
            content: ch.to_string(),
            token_type: TokenType::Symbol,
            start,
            end: start + ch.len_utf8(),
        }
    }
}

/// かっこの状態を保持しながらトークンに分割する関数
pub fn tokenize_with_state(
    content: &str,
    line_idx: usize,
    content_start_col: usize,
    bracket_state: &mut BracketState,
) -> Vec<Token> {
    if content.is_empty() {
        return Vec::new();
    }
    let mut tokenizer = Tokenizer::new(content, line_idx, content_start_col, bracket_state);
    tokenizer.run()
}

/// 単語の種類を分類する関数
fn classify_word(word: &str, next_char: Option<char>) -> TokenType {
    if RUST_KEYWORDS.contains(word) {
        return TokenType::Keyword;
    }
    if next_char == Some('(') {
        return TokenType::Function;
    }
    if word.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
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
        TokenType::Bracket { level, is_matched } => {
            let color = bracket_colors[*level % bracket_colors.len()];
            let mut style = Style::default().fg(color);
            if !is_matched {
                style = style
                    .fg(theme.unmatched_bracket_fg.clone().into())
                    .bg(theme.unmatched_bracket_bg.clone().into());
            }
            style
        }
    };
    Span::styled(token.content.clone(), style)
}

/// かっこの状態を保持しながらシンタックスハイライトを行う関数
pub fn highlight_syntax_with_state(
    line_str: &str,
    line_idx: usize,
    indent_width: usize,
    bracket_state: &mut BracketState,
    theme: &Theme,
    unmatched_brackets: &HashSet<(usize, usize)>,
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

    // トークン化してスパンに変換
    let tokens = tokenize_with_state(content_part, line_idx, space_count, bracket_state);

    for token in tokens {
        let mut span_style = token_to_span(&token, &theme.syntax).style;
        if let TokenType::Bracket { is_matched, .. } = token.token_type {
            let col = space_count + token.start;
            if !is_matched || unmatched_brackets.contains(&(line_idx, col)) {
                span_style = span_style
                    .fg(theme.syntax.unmatched_bracket_fg.clone().into())
                    .bg(theme.syntax.unmatched_bracket_bg.clone().into());
            }
        }
        spans.push(Span::styled(token.content.clone(), span_style));
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
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("fn main()", 0, 0, &mut bracket_state);
        assert_eq!(tokens.len(), 5); // fn, space, main, (, )
        assert_eq!(tokens[0].content, "fn");
        assert_eq!(tokens[0].token_type, TokenType::Keyword);
        assert_eq!(tokens[1].content, " ");
        assert_eq!(tokens[1].token_type, TokenType::Whitespace);
        assert_eq!(tokens[2].content, "main");
        assert_eq!(tokens[2].token_type, TokenType::Function);
        assert_eq!(tokens[3].content, "(");
        assert_eq!(tokens[3].token_type, TokenType::Bracket { level: 0, is_matched: true });
        assert_eq!(tokens[4].content, ")");
        assert_eq!(tokens[4].token_type, TokenType::Bracket { level: 0, is_matched: true });
    }

    #[test]
    fn test_tokenize_string() {
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("\"hello world\"", 0, 0, &mut bracket_state);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].content, "\"hello world\"");
        assert_eq!(tokens[0].token_type, TokenType::String);
    }

    #[test]
    fn test_tokenize_string_with_escape() {
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state(r#""hello \"world\"""#, 0, 0, &mut bracket_state);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].content, r#""hello \"world\"""#);
        assert_eq!(tokens[0].token_type, TokenType::String);
    }

    #[test]
    fn test_tokenize_comment() {
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("// this is a comment", 0, 0, &mut bracket_state);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].content, "// this is a comment");
        assert_eq!(tokens[0].token_type, TokenType::Comment);
    }

    #[test]
    fn test_tokenize_numbers() {
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("let x = 42;", 0, 0, &mut bracket_state);
        let number_token = tokens.iter().find(|t| t.token_type == TokenType::Number);
        assert!(number_token.is_some());
        assert_eq!(number_token.unwrap().content, "42");
    }

    #[test]
    fn test_classify_word() {
        assert_eq!(classify_word("fn", Some(' ')), TokenType::Keyword);
        assert_eq!(classify_word("main", Some('(')), TokenType::Function);
        assert_eq!(classify_word("String", Some(':')), TokenType::Type);
    }

    #[test]
    fn test_macro_detection() {
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("println!(\"Hello\")", 0, 0, &mut bracket_state);
        let macro_token = tokens.iter().find(|t| t.token_type == TokenType::Macro);
        assert!(macro_token.is_some());
        assert_eq!(macro_token.unwrap().content, "println!");
        
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("vec![1, 2, 3]", 0, 0, &mut bracket_state);
        let macro_token = tokens.iter().find(|t| t.token_type == TokenType::Macro);
        assert!(macro_token.is_some());
        assert_eq!(macro_token.unwrap().content, "vec!");
        
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("format!(\"test {}\", 1)", 0, 0, &mut bracket_state);
        let macro_token = tokens.iter().find(|t| t.token_type == TokenType::Macro);
        assert!(macro_token.is_some());
        assert_eq!(macro_token.unwrap().content, "format!");
    }

    #[test]
    fn test_function_vs_macro_distinction() {
        // 関数呼び出し
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("some_function()", 0, 0, &mut bracket_state);
        let func_token = tokens.iter().find(|t| t.token_type == TokenType::Function);
        assert!(func_token.is_some());
        assert_eq!(func_token.unwrap().content, "some_function");
        
        // マクロ呼び出し
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("some_macro!()", 0, 0, &mut bracket_state);
        let macro_token = tokens.iter().find(|t| t.token_type == TokenType::Macro);
        assert!(macro_token.is_some());
        assert_eq!(macro_token.unwrap().content, "some_macro!");
    }

    #[test]
    fn test_highlight_syntax_empty() {
        let theme = Theme::default();
        let mut bracket_state = BracketState::new();
        let spans = highlight_syntax_with_state("", 0, 4, &mut bracket_state, &theme, &HashSet::new());
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "");
    }

    #[test]
    fn test_highlight_syntax_with_indent() {
        let theme = Theme::default();
        let mut bracket_state = BracketState::new();
        let spans = highlight_syntax_with_state("    fn main()", 0, 4, &mut bracket_state, &theme, &HashSet::new());
        assert!(spans.len() > 1);
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
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("((()))", 0, 0, &mut bracket_state);
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].token_type, TokenType::Bracket { level: 0, is_matched: true }); // (
        assert_eq!(tokens[1].token_type, TokenType::Bracket { level: 1, is_matched: true }); // (
        assert_eq!(tokens[2].token_type, TokenType::Bracket { level: 2, is_matched: true }); // (
        assert_eq!(tokens[3].token_type, TokenType::Bracket { level: 2, is_matched: true }); // )
        assert_eq!(tokens[4].token_type, TokenType::Bracket { level: 1, is_matched: true }); // )
        assert_eq!(tokens[5].token_type, TokenType::Bracket { level: 0, is_matched: true }); // )
    }

    #[test]
    fn test_mixed_brackets() {
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("([{}])", 0, 0, &mut bracket_state);
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].token_type, TokenType::Bracket { level: 0, is_matched: true }); // (
        assert_eq!(tokens[1].token_type, TokenType::Bracket { level: 1, is_matched: true }); // [
        assert_eq!(tokens[2].token_type, TokenType::Bracket { level: 2, is_matched: true }); // {
        assert_eq!(tokens[3].token_type, TokenType::Bracket { level: 2, is_matched: true }); // }
        assert_eq!(tokens[4].token_type, TokenType::Bracket { level: 1, is_matched: true }); // ]
        assert_eq!(tokens[5].token_type, TokenType::Bracket { level: 0, is_matched: true }); // )
    }

    #[test]
    fn test_bracket_state() {
        let mut state = BracketState::new();
        assert_eq!(state.stack.len(), 0);
        
        state.stack.push(('(', 0, 0));
        assert_eq!(state.stack.len(), 1);
        
        state.stack.push(('[', 0, 1));
        assert_eq!(state.stack.len(), 2);
        
        state.stack.pop();
        assert_eq!(state.stack.len(), 1);
        
        state.stack.pop();
        assert_eq!(state.stack.len(), 0);
    }

    #[test]
    fn test_symbol_detection() {
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("let x = 42;", 0, 0, &mut bracket_state);
        let symbol_tokens: Vec<_> = tokens.iter().filter(|t| t.token_type == TokenType::Symbol).collect();
        assert_eq!(symbol_tokens.len(), 2); // = と ;
        assert_eq!(symbol_tokens[0].content, "=");
        assert_eq!(symbol_tokens[1].content, ";");
        
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("vec![1, 2, 3]", 0, 0, &mut bracket_state);
        let symbol_tokens: Vec<_> = tokens.iter().filter(|t| t.token_type == TokenType::Symbol).collect();
        assert_eq!(symbol_tokens.len(), 2); // , と ,
        assert_eq!(symbol_tokens[0].content, ",");
        assert_eq!(symbol_tokens[1].content, ",");
    }

    #[test]
    fn test_operator_vs_symbol() {
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("String::new()", 0, 0, &mut bracket_state);
        let operator_token = tokens.iter().find(|t| t.token_type == TokenType::Operator);
        assert!(operator_token.is_some());
        assert_eq!(operator_token.unwrap().content, "::");
        
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("let x: i32 = 42;", 0, 0, &mut bracket_state);
        let symbol_tokens: Vec<_> = tokens.iter().filter(|t| t.token_type == TokenType::Symbol).collect();
        let colon_token = symbol_tokens.iter().find(|t| t.content == ":");
        assert!(colon_token.is_some());
    }

    #[test]
    fn test_bracket_matching_basic() {
        // 正しく対応している場合
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("([{}])", 0, 0, &mut bracket_state);
        let bracket_tokens: Vec<_> = tokens.iter().filter(|t| matches!(t.token_type, TokenType::Bracket { .. })).collect();
        assert_eq!(bracket_tokens.len(), 6);
        assert!(bracket_tokens.iter().all(|t| {
            if let TokenType::Bracket { is_matched, .. } = t.token_type {
                is_matched
            } else {
                false
            }
        }));
        
        // 閉じかっこが多い場合
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("(()))", 0, 0, &mut bracket_state);
        let bracket_tokens: Vec<_> = tokens.iter().filter(|t| matches!(t.token_type, TokenType::Bracket { .. })).collect();
        assert_eq!(bracket_tokens.len(), 5);
        // 最後の閉じ括弧が未対応としてマークされることを確認
        assert!(!matches!(bracket_tokens[4].token_type, TokenType::Bracket { is_matched: true, .. }));
    }

    #[test]
    fn test_multiline_bracket_state() {
        // 複数行にわたるかっこの状態をテスト
        let mut bracket_state = BracketState::new();
        
        // 1行目: if true {
        let tokens1 = tokenize_with_state("if true {", 0, 0, &mut bracket_state);
        let bracket_token1 = tokens1.iter().find(|t| matches!(t.token_type, TokenType::Bracket { .. }));
        assert!(matches!(bracket_token1.unwrap().token_type, TokenType::Bracket { is_matched: true, .. }));
        
        // 2行目: println!("Hello");
        let tokens2 = tokenize_with_state("    println!(\"Hello\");", 1, 4, &mut bracket_state);
        let bracket_tokens2: Vec<_> = tokens2.iter().filter(|t| matches!(t.token_type, TokenType::Bracket{..})).collect();
        assert!(bracket_tokens2.iter().all(|t| matches!(t.token_type, TokenType::Bracket { is_matched: true, .. })));

        // 3行目: }
        let tokens3 = tokenize_with_state("}", 2, 0, &mut bracket_state);
        let bracket_token3 = tokens3.iter().find(|t| matches!(t.token_type, TokenType::Bracket { .. }));
        assert!(matches!(bracket_token3.unwrap().token_type, TokenType::Bracket { is_matched: true, .. }));
        
        // 最終的にスタックは空であるべき
        assert_eq!(bracket_state.stack.len(), 0);
    }

    #[test]
    fn test_unmatched_bracket_detection_single_line() {
        // 余分な閉じ括弧のテスト
        let mut bracket_state = BracketState::new();
        let tokens = tokenize_with_state("())", 0, 0, &mut bracket_state);
        let unmatched_brackets: Vec<_> = tokens.iter().filter(|t| {
            if let TokenType::Bracket { is_matched, .. } = t.token_type {
                !is_matched
            } else {
                false
            }
        }).collect();
        assert_eq!(unmatched_brackets.len(), 1);
        assert_eq!(unmatched_brackets[0].content, ")"); // 余分な閉じ括弧が未対応
    }

    #[test]
    fn test_unmatched_bracket_highlight_multiline() {
        let lines = vec!["fn main() {", "    let x = 1;"];
        let theme = Theme::default();
        
        // 1パス目: ファイル全体をスキャンして未対応の括弧を特定
        let mut scan_state = BracketState::new();
        let mut all_unmatched_brackets: HashSet<(usize, usize)> = HashSet::new();
        for (i, line) in lines.iter().enumerate() {
            let space_count = count_leading_spaces(line);
            let content_part = &line[space_count..];
            let tokens = tokenize_with_state(content_part, i, space_count, &mut scan_state);
            for token in tokens {
                if let TokenType::Bracket { is_matched: false, .. } = token.token_type {
                    if token.content == ")" || token.content == "]" || token.content == "}" {
                        all_unmatched_brackets.insert((i, space_count + token.start));
                    }
                }
            }
        }
        for &(_, line, col) in &scan_state.stack {
            all_unmatched_brackets.insert((line, col));
        }
        let unmatched_brackets = all_unmatched_brackets;

        // `{` が未対応として検出されていることを確認
        assert!(unmatched_brackets.contains(&(0, 10)));

        // 2パス目: ハイライト処理
        let mut highlight_state = BracketState::new();
        let spans = highlight_syntax_with_state(lines[0], 0, 4, &mut highlight_state, &theme, &unmatched_brackets);

        let bracket_span = spans.iter().find(|s| s.content == "{").unwrap();
        assert_eq!(bracket_span.style.fg, Some(theme.syntax.unmatched_bracket_fg.clone().into()));
        assert_eq!(bracket_span.style.bg, Some(theme.syntax.unmatched_bracket_bg.clone().into()));
    }
}
