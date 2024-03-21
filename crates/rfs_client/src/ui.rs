//! User interface module
//!
#![allow(unused)]

mod app;
mod contents;
mod tui;

use std::{
    borrow::Borrow,
    io::{self, stdout, Stdout},
    time::Duration,
};

use crossterm::{
    event::{Event, KeyEvent, KeyEventKind, MouseEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::{FutureExt, SinkExt, StreamExt};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout, Rect},
    Frame, Terminal,
};
use rfs::fs::VirtReadDir;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

/// This is the main terminal type used inside main
pub type Ui = Terminal<CrosstermBackend<Stdout>>;

/// UI refreshes at this frequency
pub const TICK_PERIOD: Duration = Duration::from_millis(1000 / 60);



#[derive(Debug)]
pub enum SelectedScreen {
    /// Main content window
    ContentWindow,

    /// Filesystem widget
    FilesystemTree,

    /// Logs redirected from stderr
    StderrLogs, // this should be unselectable, but ill leave it here for now
}

/// Spawns the UI event handler.
/// This converts crossterm events into our own custom events
pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
}

/// Events that change the selected screen
#[derive(Debug)]
pub enum SelectedScreenEvents {}


pub async fn run() -> io::Result<()> {
    // let mut term = init()?;
    // let mut app = App::new(60.0, 60.0);

    loop {
        // if app.exit {
        //     break;
        // }
    }

    Ok(())
}

impl EventHandler {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        // spawn the event handler
        tokio::spawn(async move {
            loop {
                if crossterm::event::poll(TICK_PERIOD).expect("poll failed") {
                    if let Ok(ev) = crossterm::event::read() {
                        tx.send(ev).expect("event send failed");
                    }
                }
            }
        });

        Self { rx }
    }

    /// Block and wait for the next event
    pub async fn next(&mut self) -> io::Result<Event> {
        self.rx.recv().await.ok_or(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "failed to get event from channel",
        ))
    }
}
