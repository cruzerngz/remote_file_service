//! Tui module. Handles rendering/// Terminal ui struct.

use std::{
    borrow::Borrow,
    io::{self, Read},
    ops::{Deref, DerefMut},
    sync::Arc,
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
    style::{Style, Stylize},
    widgets::{block::title, Block, Borders, Clear, Widget},
    Frame, Terminal,
};
use rfs::interfaces::FileUpdate;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use super::widgets::{
    AvailableCommands, ContentWindow, FsTree, StderrLogs, TitleBar, DEFAULT_BLOCK,
};
/// This is instantiated and run inside app::run().

/// This is the main terminal type used inside main
pub type Ui = Terminal<CrosstermBackend<std::io::Stdout>>;

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

    // stderr pipe
    pub stderr_pipe: Arc<std::sync::Mutex<dyn io::Read + Send + 'static>>,

    // widgets
    pub title_widget: TitleBar,
    pub fs_widget: FsTree,
    pub logs_widget: StderrLogs,
    pub commands_widget: AvailableCommands,
    pub content_widget: ContentWindow,
}

/// Various rectangles rendered on the screen
#[derive(Debug)]
pub struct UIWindows {
    /// Application title
    pub title: Rect,

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
    /// First event sent is init
    Init,
    Quit,
    Error(Option<String>),
    Closed,
    Tick,
    Render,
    FocusGained,
    FocusLost,
    Paste(String),
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),

    /// notif
    SetContentNotification(Option<String>),

    /// Highlight stuff in the content window.
    ///
    /// tuple contains `(offset, len)`
    HighlightContent(Option<(usize, usize)>),

    /// file update event
    FileUpdate {
        path: String,
        upd: FileUpdate,
    },
}

/// If a widget can be in focus, it should implement this trait.
///
/// Focused widget should visually distinguish itself from other widgets.
/// In the case for all widgets implementting this trait, their borders will be bolded.
pub trait FocusedWidget {
    /// Toggle focus for a widget
    fn focus(&mut self, selected: bool);
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
    pub fn new(
        tick_rate: f64,
        frame_rate: f64,
        sh: Arc<std::sync::Mutex<dyn io::Read + Send + 'static>>,
    ) -> io::Result<Self> {
        let mut terminal = ratatui::Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
        // terminal.clear()?;
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let cancellation_token = CancellationToken::new();
        let task = tokio::spawn(async {}); // placeholder task
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

            stderr_pipe: sh,

            title_widget: TitleBar::new(),
            fs_widget: FsTree::new(),
            logs_widget: StderrLogs::new(),
            commands_widget: AvailableCommands::new(),
            content_widget: ContentWindow::new(),
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

    /// Start key events capture task
    pub fn start(&mut self) {
        let tick_delay = std::time::Duration::from_secs_f64(1.0 / self.tick_rate);
        let render_delay = std::time::Duration::from_secs_f64(1.0 / self.frame_rate);
        self.cancel();
        self.cancellation_token = CancellationToken::new();
        let _cancellation_token = self.cancellation_token.clone();
        let _event_tx = self.event_tx.clone();

        // spawn the keyboard events task
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
                            Some(Err(e)) => {
                                _event_tx.send(AppEvent::Error(Some(e.to_string()))).unwrap();
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
        crossterm::execute!(std::io::stdout(), EnterAlternateScreen, cursor::Hide)?;
        crossterm::terminal::enable_raw_mode()?;
        if self.mouse {
            crossterm::execute!(std::io::stderr(), EnableMouseCapture)?;
        }
        if self.paste {
            crossterm::execute!(std::io::stderr(), EnableBracketedPaste)?;
        }
        // self.start();
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
            crossterm::execute!(std::io::stdout(), LeaveAlternateScreen, cursor::Show)?;
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

    /// Draw the UI and it's current state to the screen.
    /// This method should be called inside [`super::App`]
    pub async fn draw_to_screen(&mut self) -> io::Result<()> {
        // update piped stderr logs
        // stderr logs are automatically updated every draw call
        let mut new_logs = String::new();
        self.stderr_pipe
            .lock()
            .unwrap()
            .read_to_string(&mut new_logs)?;
        self.logs_widget.push(new_logs);

        let title_widget = self.title_widget.clone();
        let fs_widget = self.fs_widget.clone();
        let commands_widget = self.commands_widget.clone();
        let logs_widget = self.logs_widget.clone();
        let content_widget = self.content_widget.clone();

        self.draw(|f| {
            let windows = UIWindows::from(f.borrow());

            f.render_widget(title_widget, windows.title);
            f.render_widget(fs_widget, windows.filesystem);
            f.render_widget(content_widget, windows.content);
            f.render_widget(commands_widget, windows.commands);
            f.render_widget(logs_widget, windows.logs);
        })?;

        Ok(())
    }

    pub fn on_filesystem(&mut self) {
        self.fs_widget.focus(true);
        self.content_widget.focus(false);
        self.commands_widget.clear();
        self.commands_widget.add([
            ("ESC", "exit"),
            ("ENTER", "enter filesystem browse"),
            ("RIGHT", "go to content"),
        ]);
    }

    pub fn on_content(&mut self) {
        self.content_widget.focus(true);
        self.fs_widget.focus(false);
        self.commands_widget.clear();
        self.commands_widget
            .add([("ENTER", "enter content"), ("LEFT", "filesystem tree")]);
    }

    pub fn in_filesystem(&mut self) {
        self.content_widget.focus(false);
        self.fs_widget.focus(true);
        self.commands_widget.clear();
        self.commands_widget.add([
            ("ESC", "exit filesystem browse"),
            ("ENTER", "enter file/dir"),
            ("BACKSPACE", "go to parent dir"),
            ("UP/DOWN", "navigate"),
            ("f", "create file"),
            ("d", "create directory"),
            ("x", "delete file/dir"),
        ]);
    }

    pub fn in_content_navi(&mut self) {
        self.fs_widget.focus(false);
        self.content_widget.focus(true);
        self.commands_widget.clear();
        self.commands_widget.add([
            ("ESC", "exit content"),
            ("ENTER", "enter insert mode"),
            ("DEL", "delete a character"),
            ("arrow keys", "navigate"),
            ("w", "watch file for changes"),
        ]);
    }

    pub fn in_content_insert(&mut self) {
        self.fs_widget.focus(false);
        self.content_widget.focus(true);
        self.commands_widget.clear();
        self.commands_widget
            .add([("ESC", "exit insert mode and save changes")]);
    }

    pub fn in_filesystem_create(&mut self, title: &str) {
        self.fs_widget.focus(true);
        self.content_widget.focus(false);
        self.commands_widget.clear();
        self.commands_widget
            .add([("ESC", "cancel"), ("ENTER", "create file/dir")]);
        self.fs_widget.dialogue_box(Some((title, "", false)));
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
            .constraints(vec![
                Constraint::Max(1),
                Constraint::Percentage(80),
                Constraint::Max(20),
            ])
            .split(value.size());

        // left layout for filesystem, right layout for content
        let filesystem_layout = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints(vec![Constraint::Max(100), Constraint::Percentage(75)])
            .split(main_layout[1]);

        // top layout for content, bottom for available commands
        let content_layout = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints(vec![Constraint::Percentage(90), Constraint::Min(3)])
            .split(filesystem_layout[1]);

        Self {
            title: main_layout[0],
            logs: main_layout[2],
            filesystem: filesystem_layout[0],
            content: content_layout[0],
            commands: content_layout[1],
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        borrow::Borrow,
        hash::{DefaultHasher, Hash, Hasher},
        io::Read,
    };

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

        let title = {
            let mut t = TitleBar::new();
            t.set_title(Some("tui test title"));
            t
        };

        let mut logs = StderrLogs::new();

        let handle = std::thread::spawn(|| {
            let mut count = 0;
            loop {
                std::thread::sleep(Duration::from_millis(500));
                count += 1;
                log::info!("this is log line {}", count);
            }
        });

        let (mut fs_tree, num_entries) = {
            // we are manually creating virt objects here
            const BASE_PATH: &str = ".";

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

            let mut tree = FsTree::new();
            tree.push(virt_rd, &cur_dir.path);

            (tree, num_entries)
        };

        fs_tree.dialogue_box(Some(("create file!!!!", "file_name", false)));

        let mut fs_selection = 0;

        // manually seeding some commands
        let mut commands = AvailableCommands::new();
        commands.add([('q', "quit"), ('h', "help")]);

        let mut content_widget = ContentWindow::new();
        content_widget.set_cursor_pos(Some((0, 0)));
        // content_widget.set_notification(Some("hello world from the notifications!"));
        content_widget.set_error_message(Some("this is an error message AHHHHH"));
        let mut content_highlight_offset = 0;

        let mut focus_toggle = false;

        loop {
            // wait for a crossterm keypress
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    break;
                }
            } else {
                let mut new_logs = String::new();
                sh.read_to_string(&mut new_logs)?;
                logs.push(new_logs);

                if fs_selection == num_entries.saturating_sub(1) {
                    fs_selection = 0;
                } else {
                    fs_selection += 1;
                }

                fs_tree.select(Some(fs_selection));

                // toggle select/deselect on fs and content widgets
                focus_toggle = !focus_toggle;
                fs_tree.focus(focus_toggle);
                content_widget.focus(!focus_toggle);

                terminal.draw(|frame| {
                    let windows = UIWindows::from(frame.borrow());

                    let content = format!(
                        "main window: {:#?}, ui windows: {:#?}",
                        frame.size(),
                        windows
                    );
                    let num_content_chars = content.chars().count();
                    content_widget.set_contents(Some(content));

                    let hash_time = {
                        let mut hasher = DefaultHasher::new();
                        std::time::Instant::now().hash(&mut hasher);
                        hasher.finish()
                    };

                    // content_widget.cursor_down();

                    if content_highlight_offset < num_content_chars - 1 {
                        content_highlight_offset += 1;
                    } else {
                        content_highlight_offset = 0;
                    }

                    content_widget.set_highlight(content_highlight_offset, 20);
                    // content_widget.clear_highlight();

                    match hash_time % 4 {
                        0 => content_widget.cursor_down(),
                        1 => content_widget.cursor_up(),
                        2 => content_widget.cursor_right(),
                        3 => content_widget.cursor_left(),
                        // 4 => content_widget.clear_highlight(),
                        // 4 => {
                        //     let pos = content_widget.pos().unwrap();
                        //     content_widget.set_highlight(pos, 20);
                        //     log::info!("highlight {} chars from: {:?}", 20, pos)
                        // }
                        // 5 => {
                        //     log::info!("clearing hightlighting");
                        //     content_widget.clear_highlight()
                        // }
                        _ => unimplemented!(),
                    }

                    frame.render_widget(title.clone(), windows.title);

                    // frame.render_widget(
                    //     Paragraph::new(format!(
                    //         "main window: {:#?}, ui windows: {:#?}",
                    //         frame.size(),
                    //         windows
                    //     ))
                    //     .block(Block::new().borders(Borders::ALL)),
                    //     windows.content,
                    // );

                    frame.render_widget(content_widget.clone(), windows.content);

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
