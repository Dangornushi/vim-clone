use crate::app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
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

mod app;
mod event;
mod ui;
mod pane;
mod config;
mod syntax;
mod constants;
mod window;
mod app_config;
mod utils;

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
    let args = Args::parse();

    let filename = if let Some(file) = args.file {
        Some(file)
    } else if let Some(Subcommands::New { name }) = args.command {
        println!("Creating new file: {}", name);
        return Ok(());
    } else if let Some(Subcommands::Version) = args.command {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    } else {
        None
    };

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new(filename);
    let rt = tokio::runtime::Runtime::new()?;
    let res = rt.block_on(event::run_app(&mut terminal, app));

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}
