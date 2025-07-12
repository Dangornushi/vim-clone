mod app;
mod event;
mod ui;

use crate::app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
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
    env,
    error::Error,
    io,
};

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Clear the screen
    execute!(terminal.backend_mut(), Clear(ClearType::All))?;

    // create app and run it
    let args: Vec<String> = env::args().collect();
    let filename = args.get(1).cloned();
    let app = App::new(filename);
    let res = event::run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
