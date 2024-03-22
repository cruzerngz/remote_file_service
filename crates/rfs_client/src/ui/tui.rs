//! Tui module. Handles rendering/// Terminal ui struct.

use std::{
    io,
    ops::{Deref, DerefMut},
    time::Duration,
};

use crossterm::{
    cursor,
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyEvent, KeyEventKind, MouseEvent,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::{FutureExt, StreamExt};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders},
    Frame, Terminal,
};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
/// This is instantiated and run inside app::run().

/// This is the main terminal type used inside main
pub type Ui = Terminal<CrosstermBackend<std::io::Stdout>>;

/// Default block for the UI
pub const DEFAULT_BLOCK: Block = Block::new().borders(Borders::ALL);

pub struct Tui {
    pub terminal: ratatui::Terminal<CrosstermBackend<std::io::Stdout>>,
    pub task: JoinHandle<()>,
    pub cancellation_token: CancellationToken,
    pub event_rx: UnboundedReceiver<AppEvent>,
    pub event_tx: UnboundedSender<AppEvent>,
    pub frame_rate: f64,
    pub tick_rate: f64,
    pub mouse: bool,
    pub paste: bool,
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
    pub fn new(tick_rate: f64, frame_rate: f64) -> io::Result<Self> {
        let terminal = ratatui::Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let cancellation_token = CancellationToken::new();
        let task = tokio::spawn(async {});
        let mouse = false;
        let paste = false;
        Ok(Self {
            terminal,
            task,
            cancellation_token,
            event_rx,
            event_tx,
            frame_rate,
            tick_rate,
            mouse,
            paste,
        })
    }

    pub fn tick_rate(mut self, tick_rate: f64) -> Self {
        self.tick_rate = tick_rate;
        self
    }

    pub fn frame_rate(mut self, frame_rate: f64) -> Self {
        self.frame_rate = frame_rate;
        self
    }

    pub fn mouse(mut self, mouse: bool) -> Self {
        self.mouse = mouse;
        self
    }

    pub fn paste(mut self, paste: bool) -> Self {
        self.paste = paste;
        self
    }

    pub fn start(&mut self) {
        let tick_delay = std::time::Duration::from_secs_f64(1.0 / self.tick_rate);
        let render_delay = std::time::Duration::from_secs_f64(1.0 / self.frame_rate);
        self.cancel();
        self.cancellation_token = CancellationToken::new();
        let _cancellation_token = self.cancellation_token.clone();
        let _event_tx = self.event_tx.clone();
        self.task = tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_delay);
            let mut render_interval = tokio::time::interval(render_delay);
            _event_tx.send(AppEvent::Init).unwrap();
            loop {
                let tick_delay = tick_interval.tick();
                let render_delay = render_interval.tick();
                let crossterm_event = reader.next().fuse();
                tokio::select! {
                  _ = _cancellation_token.cancelled() => {
                    break;
                  }
                  maybe_event = crossterm_event => {
                    match maybe_event {
                      Some(Ok(evt)) => {
                        match evt {
                          Event::Key(key) => {
                            if key.kind == KeyEventKind::Press {
                              _event_tx.send(AppEvent::Key(key)).unwrap();
                            }
                          },
                          Event::Mouse(mouse) => {
                            _event_tx.send(AppEvent::Mouse(mouse)).unwrap();
                          },
                          Event::Resize(x, y) => {
                            _event_tx.send(AppEvent::Resize(x, y)).unwrap();
                          },
                          Event::FocusLost => {
                            _event_tx.send(AppEvent::FocusLost).unwrap();
                          },
                          Event::FocusGained => {
                            _event_tx.send(AppEvent::FocusGained).unwrap();
                          },
                          Event::Paste(s) => {
                            _event_tx.send(AppEvent::Paste(s)).unwrap();
                          },
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

    pub fn stop(&self) -> io::Result<()> {
        self.cancel();
        let mut counter = 0;
        while !self.task.is_finished() {
            std::thread::sleep(Duration::from_millis(1));
            counter += 1;
            if counter > 50 {
                self.task.abort();
            }
            if counter > 100 {
                log::error!("Failed to abort task in 100 milliseconds for unknown reason");
                break;
            }
        }
        Ok(())
    }

    pub fn enter(&mut self) -> io::Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stderr(), EnterAlternateScreen, cursor::Hide)?;
        if self.mouse {
            crossterm::execute!(std::io::stderr(), EnableMouseCapture)?;
        }
        if self.paste {
            crossterm::execute!(std::io::stderr(), EnableBracketedPaste)?;
        }
        self.start();
        Ok(())
    }

    pub fn exit(&mut self) -> io::Result<()> {
        self.stop()?;
        if crossterm::terminal::is_raw_mode_enabled()? {
            self.flush()?;
            if self.paste {
                crossterm::execute!(std::io::stderr(), DisableBracketedPaste)?;
            }
            if self.mouse {
                crossterm::execute!(std::io::stderr(), DisableMouseCapture)?;
            }
            crossterm::execute!(std::io::stderr(), LeaveAlternateScreen, cursor::Show)?;
            crossterm::terminal::disable_raw_mode()?;
        }
        Ok(())
    }

    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    pub fn suspend(&mut self) -> io::Result<()> {
        self.exit()?;
        Ok(())
    }

    pub fn resume(&mut self) -> io::Result<()> {
        self.enter()?;
        Ok(())
    }

    pub async fn next(&mut self) -> Option<AppEvent> {
        self.event_rx.recv().await
    }
}

impl Deref for Tui {
    type Target = ratatui::Terminal<CrosstermBackend<std::io::Stdout>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for Tui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        self.exit().unwrap();
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
    use std::{borrow::Borrow, io::Read};

    use ratatui::{
        style::Stylize,
        widgets::{block::Title, Block, BorderType, Borders, Paragraph},
    };
    use rfs::fs::{VirtDirEntry, VirtReadDir};

    use crate::ui::widgets::{AvailableCommands, FsTree, StderrLogs};

    use super::*;

    /// Check out the look of the UI and box sizes
    // #[ignore = "this test is for manual testing only"]
    #[test]
    fn test_render_boxes() -> io::Result<()> {
        let mut sh = shh::stderr()?;

        pretty_env_logger::formatted_builder()
            .parse_filters("debug")
            .init();

        init_terminal()?;

        let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

        let mut logs = StderrLogs::new();

        let handle = std::thread::spawn(|| {
            let mut count = 0;
            loop {
                std::thread::sleep(Duration::from_millis(500));
                count += 1;
                log::info!("this is log line {}", count);
            }
        });

        // we are manually creating virt objects here
        const BASE_PATH: &str = "../..";

        let cur_dir = VirtDirEntry {
            path: BASE_PATH.to_string(),
            file: false,
        };

        let entries = std::fs::read_dir(BASE_PATH)?;
        let virt: Vec<_> = entries
            .into_iter()
            .filter_map(|entry| Some(entry.ok()?))
            .filter_map(|entry| VirtDirEntry::from_dir_entry(entry, BASE_PATH))
            .collect();

        let num_entries = virt.len();
        let virt_rd = VirtReadDir::from(virt);

        let mut fs_tree = FsTree::new();
        fs_tree.push(virt_rd, cur_dir);

        let mut fs_selection = 0;

        // manually seeding some commands
        let mut commands = AvailableCommands::new();
        commands.add([('q', "quit"), ('h', "help")]);

        loop {
            // wait for a crossterm keypress
            if event::poll(std::time::Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    break;
                }
            } else {
                let mut new_logs = String::new();
                sh.read_to_string(&mut new_logs)?;
                logs.push(new_logs);

                if fs_selection == num_entries - 1 {
                    fs_selection = 0;
                } else {
                    fs_selection += 1;
                }

                fs_tree.select(Some(fs_selection));

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

                    frame.render_widget(fs_tree.clone(), windows.filesystem);

                    frame.render_widget(commands.clone(), windows.commands);

                    frame.render_widget(logs.clone(), windows.logs)
                });
            }
        }

        restore_terminal()?;

        Ok(())
    }
}
