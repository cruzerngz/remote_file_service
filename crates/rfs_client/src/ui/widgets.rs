//! Various UI widgets

use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
};

use crossterm::event::KeyCode;
use ratatui::{
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{block::Title, Block, Borders, Paragraph, Widget, Wrap},
};
use rfs::{
    fs::{VirtDirEntry, VirtReadDir},
    ser_de::de,
};

/// Default block used for UI elements
pub const DEFAULT_BLOCK: Block = Block::new().borders(Borders::ALL);

/// Title bar
#[derive(Clone, Debug)]
pub struct TitleBar {
    title: Option<String>,
}

/// Filesystem tree widgets
#[derive(Clone, Debug)]
pub struct FsTree {
    /// The relative path to the current directory
    parent_dir: PathBuf,

    /// Entries in the current directory
    entries: Vec<VirtReadDir>,

    /// Current selection, if any
    selection: Option<usize>,
}

/// Error log widget.
///
/// Logs are taken from [shh] and pushed into `self`.
/// This struct implements [Widget], so it can be rendered to the terminal.
#[derive(Clone, Debug)]
pub struct StderrLogs {
    pub logs: VecDeque<String>,
}

/// Widget that displays available commands.
#[derive(Clone, Debug)]
pub struct AvailableCommands {
    /// A command key and it's description
    commands: HashMap<String, String>,
}

/// This widget is used to display file contents, as well as any error messages.
#[derive(Clone, Debug)]
pub struct ContentWindow {
    /// File contents to display.
    ///
    /// Multiple lines should be separated by '\n'.
    contents: Option<String>,

    /// Cursor position in the file: (row, col)
    ///
    /// This will highlight the current line and character in the file.
    cursor_pos: Option<(u16, u16)>,

    /// Notifications are displayed over the main contents like a pop-up.
    /// Errors take precedence over notifications.
    notification: Option<String>,

    /// Error messages are displayed over the main contents like a pop-up.
    error_message: Option<String>,
}

impl Widget for TitleBar {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let block = match self.title {
            Some(t) => DEFAULT_BLOCK
                .borders(Borders::TOP)
                .title(t)
                .title_alignment(ratatui::layout::Alignment::Center)
                .style(Style::new().bold()),
            None => DEFAULT_BLOCK.borders(Borders::TOP),
        };

        block.render(area, buf)
    }
}

impl Widget for FsTree {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // a block takes up 1 line at the top and bottom.
        // if this widget renders with a border, this const needs to be set to 2.
        // if this widget renders without a border, this const needs to be set to 0.
        const FRAME_BORDER_LINES: usize = 2;

        let lines = match (self.entries.last(), self.selection) {
            (None, None) => Vec::new(),
            (None, Some(_)) => Vec::new(),
            (Some(dirs), None) => dirs
                .iter()
                .enumerate()
                .map(|(idx, en)| {
                    let mut contents = if en.is_file() {
                        Span::raw(en.path().to_str().expect("invalid path"))
                    } else {
                        Span::styled(
                            en.path().to_str().expect("invalid path"),
                            Style::new().green().bold(),
                        )
                    };

                    Line::from(contents)
                })
                .collect::<Vec<_>>(),

            (Some(dirs), Some(mut selection)) => {
                let lines: Box<dyn Iterator<Item = _>> = match dirs.len()
                    > area.height as usize - FRAME_BORDER_LINES
                    && selection + 1 > area.height as usize - FRAME_BORDER_LINES
                {
                    true => {
                        let new_iter = Box::new(
                            dirs.iter()
                                .take(selection + 1)
                                .rev()
                                .take(area.height as usize - FRAME_BORDER_LINES)
                                .rev(),
                        );
                        selection = area.height as usize - 1 - FRAME_BORDER_LINES;
                        new_iter
                    }
                    false => Box::new(dirs.iter()),
                };

                lines
                    .enumerate()
                    .map(|(idx, en)| {
                        let mut contents = if en.is_file() {
                            Span::raw(en.path().to_str().expect("invalid path"))
                        } else {
                            Span::styled(
                                en.path().to_str().expect("invalid path"),
                                Style::new().green().bold(),
                            )
                        };

                        // highlight selection
                        if selection == idx {
                            contents = contents.reversed()
                        }

                        Line::from(contents)
                    })
                    .collect::<Vec<_>>()
            }
        };

        let para = Paragraph::new(lines)
            .block(
                DEFAULT_BLOCK.title(
                    Title::from(
                        self.parent_dir
                            .to_str()
                            .expect("invalid path")
                            .bold()
                            .gray(),
                    )
                    .alignment(ratatui::layout::Alignment::Left),
                ),
            )
            .wrap(Wrap { trim: false });

        para.render(area, buf)
    }
}

impl Widget for StderrLogs {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // we need to take the last N lines from the logs that fit in the rect
        let lines = self
            .logs
            .iter()
            .rev()
            .take(area.height as usize)
            .map(|log| Line::from(vec![Span::raw(log)]))
            .rev()
            .collect::<Vec<_>>();

        let para = Paragraph::new(lines)
            .block(DEFAULT_BLOCK.title(
                Title::from("logs".bold().gray()).alignment(ratatui::layout::Alignment::Center),
            ))
            .wrap(Wrap { trim: false });

        para.render(area, buf)
    }
}

impl Widget for AvailableCommands {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let mut sorted = self.commands.iter().collect::<Vec<_>>();
        sorted.sort();

        let instrs = sorted
            .into_iter()
            .map(|(key, desc)| {
                vec![
                    Span::styled(key, Style::new().bold().green()),
                    Span::raw(": "),
                    Span::styled(desc, Style::new().underlined()),
                    Span::raw("  "),
                ]
            })
            .collect::<Vec<_>>()
            .concat();

        let line = Line::from(instrs);

        let para = Paragraph::new(line)
            .block(DEFAULT_BLOCK.title(
                Title::from("commands".bold().gray()).alignment(ratatui::layout::Alignment::Center),
            ))
            .wrap(Wrap { trim: false });

        para.render(area, buf)
    }
}

impl Widget for ContentWindow {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        match (self.contents, self.cursor_pos) {
            // render a blank screen
            (None, _) => todo!(),
            // render contents
            (Some(_), None) => todo!(),
            // render contents w/ scrolling
            (Some(_), Some(_)) => todo!(),
        }

        // notifications and error messages are overlaid on top of the contents
        // notifications are displayed on the top border???
        match (self.error_message, self.notification) {
            (Some(err_msg), _) => todo!(),
            (None, Some(notif)) => todo!(),
            _ => (),
        }

        todo!()
    }
}

impl TitleBar {
    pub fn new() -> Self {
        Self { title: None }
    }

    /// Set the title of the title bar
    pub fn set_title<T: ToString>(&mut self, title: Option<T>) {
        self.title = title.and_then(|t| Some(t.to_string()));
    }
}

impl FsTree {
    pub fn new() -> Self {
        Self {
            parent_dir: PathBuf::new(),
            entries: Vec::new(),
            selection: None,
        }
    }

    /// Push a virtual directory into the stack.
    ///
    /// This should be called when entering directories
    pub fn push(&mut self, entries: VirtReadDir, dir: VirtDirEntry) {
        self.entries.push(entries);
        self.parent_dir.push(dir.path());
    }

    /// Pop the last virtual directory from the stack
    ///
    /// This should be called when leaving directories
    pub fn pop(&mut self) {
        self.entries.pop();
        self.parent_dir.pop();
    }

    /// Select an entry by its index
    pub fn select(&mut self, idx: Option<usize>) {
        self.selection = match idx {
            Some(offset) => {
                if let Some(read_dir) = self.entries.last() {
                    if offset < read_dir.len() {
                        Some(offset)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            None => None,
        }
    }
}

impl StderrLogs {
    pub fn new() -> Self {
        Self {
            logs: VecDeque::new(),
        }
    }

    /// Push additional logs to the ring buffer
    pub fn push(&mut self, log: String) {
        // strip empty lines
        // logs always have something
        let lines = log
            .split("\n")
            .filter_map(|l| match l.len() {
                0 => None,
                _ => Some(l.to_owned()),
            })
            .collect::<Vec<_>>();

        if self.logs.len() + lines.len() > 100 {
            self.logs.drain(0..(self.logs.len() - (100 - lines.len())));
        }

        self.logs.extend(lines);
    }
}

impl AvailableCommands {
    pub fn new() -> Self {
        Self {
            commands: Default::default(),
        }
    }

    /// Add a bunch of commands to the list
    pub fn add<C: IntoIterator<Item = (K, V)>, K: ToString, V: ToString>(&mut self, commands: C) {
        let modified = commands
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()));

        self.commands.extend(modified)
    }

    /// Clears the current list of commands
    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

impl ContentWindow {
    pub fn new() -> Self {
        Self {
            contents: None,
            cursor_pos: None,
            notification: None,
            error_message: None,
        }
    }
}
