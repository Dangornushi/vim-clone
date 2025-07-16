mod app;
mod event;
mod ui;
mod pane;
mod config;
mod syntax;
mod constants;
use crate::app::App;
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, Clear,
        ClearType,
    },
};

use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{
    error::Error,
    io,
};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File to open
    file: Option<String>,
    #[command(subcommand)]
    command: Option<Subcommands>,
}

#[derive(Parser, Debug)]
enum Subcommands {
    /// Create a new file
    New {
        /// Name of the file to create
        name: String,
    },
    /// Display version information
    Version,
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    let args = Args::parse();

    // If a subcommand is used, we don't need to enter raw mode or run the TUI
    let filename = if let Some(file) = args.file {
        Some(file)
    } else if let Some(Subcommands::New { name }) = args.command {
        println!("Creating new file: {}", name);
        // ここでファイル作成ロジックを追加する
        // 例: std::fs::File::create(&name)?;
        return Ok(()); // 新規作成の場合はTUIを起動しない
    } else if let Some(Subcommands::Version) = args.command {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    } else {
        None
    };

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
        )
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Clear the screen
    execute!(terminal.backend_mut(), Clear(ClearType::All))?;

    // create app and run it
    let app = App::new(filename);
    let res = event::run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        PopKeyboardEnhancementFlags
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
