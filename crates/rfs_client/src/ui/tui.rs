//! Tui module. Handles rendering/// Terminal ui struct.

use std::io;

use crossterm::{
    event::{self, Event, KeyEvent, KeyEventKind, MouseEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::{FutureExt, StreamExt};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout, Rect},
    Frame, Terminal,
};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
/// This is instantiated and run inside app::run().

/// This is the main terminal type used inside main
pub type Ui = Terminal<CrosstermBackend<std::io::Stdout>>;

#[derive(Debug)]
pub struct Tui {
    pub terminal: Ui,
    pub task: JoinHandle<()>,
    pub cancellation_token: CancellationToken,
    pub event_rx: UnboundedReceiver<AppEvent>,
    pub event_tx: UnboundedSender<AppEvent>,
    pub frame_rate: f64,
    pub tick_rate: f64,
}

/// Various rectangles rendered on the screen
#[derive(Debug)]
pub struct UIWindows {
    /// Window for logs
    pub logs: Rect,

    /// Window for filesystem
    pub filesystem: Rect,

    /// Window for content
    pub content: Rect,

    /// Window for commands
    pub commands: Rect,
}

#[derive(Clone, Debug)]
pub enum AppEvent {
    Init,
    Quit,
    Error,
    Closed,
    Tick,
    Render,
    FocusGained,
    FocusLost,
    Paste(String),
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

/// Initialize the terminal
pub fn init_terminal() -> io::Result<Ui> {
    execute!(std::io::stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    Terminal::new(CrosstermBackend::new(std::io::stdout()))
}

/// Restores the terminal to its previous state
pub fn restore_terminal() -> io::Result<()> {
    execute!(std::io::stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}

impl Tui {
    pub fn new(frame_rate: f64, tick_rate: f64) -> io::Result<Self> {
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

        Ok(Self {
            terminal: Terminal::new(CrosstermBackend::new(std::io::stdout()))?,
            task: tokio::spawn(async {}),
            cancellation_token: CancellationToken::new(),
            event_rx,
            event_tx,
            frame_rate,
            tick_rate,
        })
    }

    pub fn start(&mut self) {
        let tick_delay = std::time::Duration::from_secs_f64(1.0 / self.tick_rate);
        let render_delay = std::time::Duration::from_secs_f64(1.0 / self.frame_rate);
        let _event_tx = self.event_tx.clone();
        self.task = tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_delay);
            let mut render_interval = tokio::time::interval(render_delay);
            loop {
                let tick_delay = tick_interval.tick();
                let render_delay = render_interval.tick();
                let crossterm_event = reader.next().fuse();
                tokio::select! {
                  maybe_event = crossterm_event => {
                    match maybe_event {
                      Some(Ok(evt)) => {
                        match evt {
                          Event::Key(key) => {
                            if key.kind == KeyEventKind::Press {
                              _event_tx.send(AppEvent::Key(key)).unwrap();
                            }
                          },
                          _ => todo!("handle other events here")
                        }
                      }
                      Some(Err(_)) => {
                        _event_tx.send(AppEvent::Error).unwrap();
                      }
                      None => {},
                    }
                  },
                  _ = tick_delay => {
                      _event_tx.send(AppEvent::Tick).unwrap();
                  },
                  _ = render_delay => {
                      _event_tx.send(AppEvent::Render).unwrap();
                  },
                }
            }
        });
    }

    /// Block until the next event is received
    pub async fn next(&mut self) -> Option<AppEvent> {
        self.event_rx.recv().await
    }
}

// generate the window bounds from a frame.
// there are 4 windows, logs, content, filesystem, commands.
impl From<&Frame<'_>> for UIWindows {
    fn from(value: &Frame<'_>) -> Self {
        // top layout for main UI, bottom layout for redirected stderr
        let main_layout = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints(vec![Constraint::Percentage(80), Constraint::Max(20)])
            .split(value.size());

        // left layout for filesystem, right layout for content
        let filesystem_layout = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints(vec![Constraint::Max(100), Constraint::Percentage(75)])
            .split(main_layout[0]);

        // top layout for content, bottom for available commands
        let content_layout = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints(vec![Constraint::Percentage(90), Constraint::Min(3)])
            .split(filesystem_layout[1]);

        Self {
            logs: main_layout[1],
            filesystem: filesystem_layout[0],
            content: content_layout[0],
            commands: content_layout[1],
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;

    use ratatui::{
        style::Stylize,
        widgets::{block::Title, Block, BorderType, Borders, Paragraph},
    };

    // use ratatui::Terminal;
    use super::*;

    #[test]
    fn test_render_boxes() -> io::Result<()> {
        init_terminal()?;

        let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

        // terminal.draw(|frame| {
        //     let windows = UIWindows::from(frame.borrow());

        //     frame.render_widget(
        //         Paragraph::new("hello world from the main content window"),
        //         windows.content,
        //     );

        //     frame.render_widget(
        //         Paragraph::new("hello world from the logs window"),
        //         windows.logs,
        //     );
        // });

        loop {
            // wait for a crossterm keypress
            if event::poll(std::time::Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    break;
                }
            } else {
                terminal.draw(|frame| {
                    let windows = UIWindows::from(frame.borrow());

                    frame.render_widget(
                        Paragraph::default().block(Block::new().borders(Borders::ALL)),
                        frame.size(),
                    );

                    frame.render_widget(
                        Paragraph::new(format!(
                            "main window: {:#?}, ui windows: {:#?}",
                            frame.size(),
                            windows
                        ))
                        .block(Block::new().borders(Borders::ALL)),
                        windows.content,
                    );

                    frame.render_widget(
                        Paragraph::new("")
                            .wrap(ratatui::widgets::Wrap { trim: false })
                            .block(
                                Block::new().borders(Borders::ALL).title(
                                    Title::from("virtual fs".gray().bold())
                                        .alignment(ratatui::layout::Alignment::Center),
                                ),
                            ),
                        windows.filesystem,
                    );

                    frame.render_widget(
                        Paragraph::new("This is the commands box").block(
                            Block::new().borders(Borders::ALL).title(
                                Title::from("commands".gray().bold())
                                    .alignment(ratatui::layout::Alignment::Center),
                            ),
                        ),
                        windows.commands,
                    );

                    frame.render_widget(
                        Paragraph::new("This is the stderr logs box").block(
                            Block::new().borders(Borders::ALL).title(
                                Title::from("logs".gray().bold())
                                    .alignment(ratatui::layout::Alignment::Center),
                            ),
                        ),
                        windows.logs,
                    )
                });
            }
        }

        restore_terminal()?;

        Ok(())
    }
}
