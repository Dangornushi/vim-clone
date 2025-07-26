pub mod syntax;
pub mod constants;
pub mod config;
pub mod window;
pub mod app_config;

// 公開API
pub use syntax::{
    highlight_syntax_with_state,
    tokenize_with_state,
    count_leading_spaces,
    create_indent_spans,
    Token,
    TokenType,
    BracketState,
};

pub use window::{
    Window,
    WindowState,
    Mode,
};

pub use app_config::{
    ConfigManager,
    AppConfigManager,
};

pub use constants::{
    editor,
    ui,
    file,
};