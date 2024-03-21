//! Various UI widgets

use std::{collections::VecDeque, path::PathBuf};

use ratatui::{
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{block::Title, Block, Borders, Paragraph, Widget, Wrap},
};
use rfs::{
    fs::{VirtDirEntry, VirtReadDir},
    ser_de::de,
};

use crate::ui::tui::DEFAULT_BLOCK;

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
