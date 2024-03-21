//! Various UI widgets

use std::{collections::VecDeque, path::PathBuf};

use ratatui::{
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{block::Title, Block, Borders, Paragraph, Widget, Wrap},
};
use rfs::{fs::VirtReadDir, ser_de::de};

use crate::ui::tui::DEFAULT_BLOCK;

/// Filesystem tree widgets
#[derive(Debug)]
pub struct FsTree {
    /// The relative path to the current directory
    parent_dir: PathBuf,

    /// Entries in the current directory
    entries: VirtReadDir,
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
        let lines = self
            .entries
            .iter()
            .enumerate()
            .map(|(idx, en)| {
                let line_num = Span::styled(format!("{}: ", idx), Style::new().bold().gray());

                if en.is_file() {
                    Line::from(vec![
                        line_num,
                        Span::raw(en.path().to_str().expect("invalid path")),
                    ])
                } else {
                    Line::from(vec![
                        line_num,
                        Span::styled(
                            en.path().to_str().expect("invalid path").to_owned(),
                            Style::new().green().bold(),
                        ),
                    ])
                }
            })
            .collect::<Vec<_>>();

        let para = Paragraph::new(lines)
            .block(
                DEFAULT_BLOCK.title(
                    Title::from(self.parent_dir.to_str().expect("invalid path"))
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

impl StderrLogs {
    pub fn new() -> Self {
        Self {
            logs: VecDeque::new(),
        }
    }

    /// Push additional logs to the ring buffer
    pub fn push(&mut self, log: String) {
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
