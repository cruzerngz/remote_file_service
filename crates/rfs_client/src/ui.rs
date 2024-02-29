//! User interface module
//!
#![allow(unused)]

use std::io::{self, stdout, Stdout};

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};

/// This is the main terminal type used inside main
pub type Ui = Terminal<CrosstermBackend<Stdout>>;

/// Main UI application
#[derive(Debug, Default)]
pub struct App {
    exit: bool,
}

/// Initialize the terminal
pub fn init() -> io::Result<Ui> {
    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    Terminal::new(CrosstermBackend::new(stdout()))
}

/// Restores the terminal to its previous state
pub fn restore() -> io::Result<()> {
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}

impl App {
    /// This is the main render loop
    pub async fn run(&mut self, terminal: &mut Ui) -> io::Result<()> {
        while !self.exit {
            // do stuff
            terminal.draw(|fr| self.render_frame(fr))?;
            self.handle_events().await?;
        }

        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame<'_>) {
        todo!()
    }

    // event handling logic here (file opening, discovery, etc.s)
    async fn handle_events(&mut self) -> io::Result<()> {
        todo!()
    }
}
