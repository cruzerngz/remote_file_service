//! Various UI widgets

use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
};

use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    symbols::line,
    text::{Line, Span},
    widgets::{block::Title, Block, Borders, Clear, Paragraph, Widget, Wrap},
};
use rfs::{
    fs::{VirtDirEntry, VirtReadDir},
    ser_de::de,
};

/// Default block used for UI elements
pub const DEFAULT_BLOCK: Block = Block::new().borders(Borders::ALL);

// a block takes up 1 line at the top and bottom.
// for widgets with full border, this const needs to be set to 2.
// for widgets without a border, this const needs to be set to 0.
const FRAME_BORDER_LINES: usize = 2;

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

    /// Area of text to highlight, from start position to end position.
    ///
    /// This item supercedes the cursor position.
    /// If this is Some(_), the cursor is not rendered.
    highlight: Option<((u16, u16), (u16, u16))>,

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

/// Derive a centered rectangle from a given rectangle, with percentage scale factor.
///
/// Gracefully taken from:
///
/// https://github.com/ratatui-org/ratatui/blob/main/examples/popup.rs#L116
fn centered_rect(x_percent: u16, y_percent: u16, rect: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - y_percent) / 2),
        Constraint::Percentage(y_percent),
        Constraint::Percentage((100 - y_percent) / 2),
    ])
    .split(rect);

    Layout::horizontal([
        Constraint::Percentage((100 - x_percent) / 2),
        Constraint::Percentage(x_percent),
        Constraint::Percentage((100 - x_percent) / 2),
    ])
    .split(popup_layout[1])[1]
}

impl Widget for ContentWindow {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // clear the area first
        Clear.render(area, buf);

        let main_para = match (self.contents, self.cursor_pos) {
            // render a blank screen
            (None, _) => Paragraph::default().block(DEFAULT_BLOCK),
            // render contents w/ line numbers
            (Some(contents), None) => {
                let num_line_digits = match contents.lines().count() {
                    0 => 1,
                    n => n.ilog10() + 1,
                };

                // asd
                Paragraph::new(
                    contents
                        .split('\n')
                        .enumerate()
                        .map(|(line_num, line)| {
                            Line::from(vec![
                                Span::styled(
                                    format!(
                                        "{:<padding$}: ",
                                        line_num + 1,
                                        padding = num_line_digits as usize
                                    ),
                                    Style::new().bold(),
                                ),
                                Span::raw(line.to_owned()),
                            ])
                        })
                        .collect::<Vec<_>>(),
                )
                .block(DEFAULT_BLOCK)
            }
            // render contents w/ scrolling
            (Some(contents), Some((cursor_x, cursor_y))) => {
                let num_line_digits = match contents.lines().count() {
                    0 => 1,
                    n => n.ilog10() + 1,
                };

                let lines = contents
                    .split('\n')
                    .enumerate()
                    .map(|(line_num, contents)| {
                        // highlight current row + selected character
                        if cursor_y as usize == line_num {
                            Line::from({
                                let mut spans = vec![Span::styled(
                                    format!(
                                        "{:<padding$}> ",
                                        line_num + 1,
                                        padding = num_line_digits as usize
                                    ),
                                    Style::new().bold().white(),
                                )];
                                spans.extend(
                                    contents
                                        .chars()
                                        .enumerate()
                                        .map(|(col, c)| match col == cursor_x as usize {
                                            true => {
                                                Span::styled(c.to_string(), Style::new().reversed())
                                            }
                                            false => Span::raw(c.to_string()),
                                        })
                                        .collect::<Vec<_>>(),
                                );

                                spans
                            })
                        } else {
                            Line::from(vec![
                                Span::styled(
                                    format!(
                                        "{:<padding$}: ",
                                        line_num + 1,
                                        padding = num_line_digits as usize
                                    ),
                                    Style::new().bold(),
                                ),
                                Span::raw(contents.to_owned()),
                            ])
                        }
                    })
                    .collect::<Vec<_>>();

                // filter to fit the area
                let rendered_lines: Box<dyn Iterator<Item = _>> = match lines.len()
                    > area.height as usize - FRAME_BORDER_LINES
                    && (cursor_y as usize + 1 + 2) > area.height as usize - FRAME_BORDER_LINES
                {
                    true => Box::new(
                        lines
                            .iter()
                            .skip(
                                (cursor_y as usize + 1 + 2).saturating_sub(area.height as usize)
                                    + FRAME_BORDER_LINES,
                            )
                            .take(area.height as usize - FRAME_BORDER_LINES)
                            .cloned(),
                    ),
                    false => Box::new(lines.into_iter()),
                };

                Paragraph::new(rendered_lines.collect::<Vec<_>>())
                    .block(DEFAULT_BLOCK)
                    .wrap(Wrap { trim: false })
            }
        };

        main_para.render(area, buf);

        // notifications are written to the title border
        if let Some(notif) = self.notification {
            let notif_block = DEFAULT_BLOCK
                .borders(Borders::ALL)
                .border_style(Style::new().light_cyan())
                .title(Title::from("notification".white().bold()))
                .title_alignment(ratatui::layout::Alignment::Left);

            notif_block.render(area, buf);
        }

        // errors are written to an inset pop-up window
        if let Some(err_msg) = self.error_message {
            // error message takes up half the screen in each dimension
            let err_rect = centered_rect(50, 50, area);

            Clear.render(err_rect, buf);

            let popup = Paragraph::new(err_msg)
                .block(
                    DEFAULT_BLOCK
                        .borders(Borders::ALL)
                        .border_style(Style::new().red())
                        .title("error")
                        .title_alignment(ratatui::layout::Alignment::Center),
                )
                .alignment(ratatui::layout::Alignment::Center);

            popup.render(err_rect, buf)
        }
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
            highlight: None,
            notification: None,
            error_message: None,
        }
    }

    /// Set the contents of the content window
    pub fn set_contents<T: ToString>(&mut self, contents: Option<T>) {
        self.contents = contents.and_then(|c| Some(c.to_string()));
    }

    pub fn set_notification<T: ToString>(&mut self, notif: Option<T>) {
        self.notification = notif.and_then(|n| Some(n.to_string()));
    }

    pub fn set_error_message<T: ToString>(&mut self, err: Option<T>) {
        self.error_message = err.and_then(|e| Some(e.to_string()));
    }

    pub fn set_cursor_pos(&mut self, pos: Option<(u16, u16)>) {
        self.cursor_pos = pos;
    }

    /// Highlight a section of text in the file
    pub fn set_highlight(&mut self, highlight: Option<((u16, u16), (u16, u16))>) {
        self.highlight = highlight;
    }

    /// Get the lines and cursor position
    fn lines_and_cursor_position(&self) -> Option<((u16, u16), Vec<&str>)> {
        let (curr_x, curr_y) = self.cursor_pos?;

        let lines = self.contents.as_ref()?.split('\n').collect::<Vec<_>>();

        Some(((curr_x, curr_y), lines))
    }

    /// Attempt to move the cursor left by one character.
    ///
    /// Modifies the x-position of the cursor.
    pub fn cursor_left(&mut self) {
        let ((mut curr_x, mut curr_y), lines) = match self.lines_and_cursor_position() {
            Some(v) => v,
            None => return,
        };

        // gets the selected line. If the line is out of bounds, set to the last line.
        let line = match lines.get(curr_y as usize) {
            Some(line) => *line,
            None => {
                curr_y = lines.len().saturating_sub(1) as u16;
                curr_x = match lines.last() {
                    Some(l) => match curr_x < l.chars().count() as u16 {
                        true => curr_x,
                        false => l.chars().count().saturating_sub(1) as u16,
                    },
                    None => 0,
                };

                self.cursor_pos = Some((curr_x, curr_y));
                return;
            }
        };

        let line_chars = line.chars().collect::<Vec<_>>();
        match (curr_x as usize) < line_chars.len() {
            true => {
                curr_x = curr_x.saturating_sub(1);
            }
            false => curr_x = line_chars.len().saturating_sub(1) as u16,
        }

        self.cursor_pos = Some((curr_x, curr_y));
    }

    /// Attempt to move the cursor right by one character
    pub fn cursor_right(&mut self) {
        let ((mut curr_x, mut curr_y), lines) = match self.lines_and_cursor_position() {
            Some(v) => v,
            None => return,
        };

        // gets the selected line. If the line is out of bounds, set to the last line.
        let line = match lines.get(curr_y as usize) {
            Some(line) => *line,
            None => {
                curr_y = lines.len().saturating_sub(1) as u16;
                curr_x = match lines.last() {
                    Some(l) => match curr_x < l.chars().count() as u16 {
                        true => curr_x,
                        false => l.chars().count().saturating_sub(1) as u16,
                    },
                    None => 0,
                };

                self.cursor_pos = Some((curr_x, curr_y));
                return;
            }
        };

        let line_chars = line.chars().collect::<Vec<_>>();
        match curr_x < (line_chars.len() - 1) as u16 {
            true => {
                curr_x += 1;
            }
            false => curr_x = line_chars.len().saturating_sub(1) as u16,
        }

        self.cursor_pos = Some((curr_x, curr_y));
    }

    /// Attempt to move the cursor up by one line
    pub fn cursor_up(&mut self) {
        let ((mut curr_x, mut curr_y), lines) = match self.lines_and_cursor_position() {
            Some(v) => v,
            None => return,
        };

        // gets the selected line. If the line is out of bounds, set to the last line.
        let line = match lines.get(curr_y as usize) {
            Some(line) => *line,
            None => {
                curr_y = lines.len().saturating_sub(1) as u16;
                curr_x = match lines.last() {
                    Some(l) => match curr_x < l.chars().count() as u16 {
                        true => curr_x,
                        false => l.chars().count().saturating_sub(1) as u16,
                    },
                    None => 0,
                };

                self.cursor_pos = Some((curr_x, curr_y));
                return;
            }
        };

        // get the previous line
        let prev_line = match lines.get(curr_y.saturating_sub(1) as usize) {
            Some(l) => {
                curr_y = curr_y.saturating_sub(1);

                l
            }
            None => {
                self.cursor_pos = Some((curr_x, curr_y));
                return;
            }
        };

        let line_chars = prev_line.chars().collect::<Vec<_>>();

        match line_chars.get(curr_x as usize) {
            Some(_) => (),
            None => {
                curr_x = line_chars.len().saturating_sub(1) as u16;
            }
        }

        self.cursor_pos = Some((curr_x, curr_y));
    }

    /// Attempt to move the cursor down by one line
    pub fn cursor_down(&mut self) {
        let ((mut curr_x, mut curr_y), lines) = match self.lines_and_cursor_position() {
            Some(v) => v,
            None => return,
        };

        // gets the selected line. If the line is out of bounds, set to the last line.
        let line = match lines.get(curr_y as usize) {
            Some(line) => *line,
            None => {
                curr_y = lines.len().saturating_sub(1) as u16;
                curr_x = match lines.last() {
                    Some(l) => match curr_x < l.chars().count() as u16 {
                        true => curr_x,
                        false => l.chars().count().saturating_sub(1) as u16,
                    },
                    None => 0,
                };

                self.cursor_pos = Some((curr_x, curr_y));
                return;
            }
        };

        // get the next line
        let prev_line = match lines.get(curr_y as usize + 1) {
            Some(l) => {
                curr_y += 1;

                l
            }
            None => {
                self.cursor_pos = Some((curr_x, curr_y));
                return;
            }
        };

        // place the x-position at the same character as the previous line
        // or at the end of the line if the previous line is shorter
        let line_chars = prev_line.chars().collect::<Vec<_>>();

        match line_chars.get(curr_x as usize) {
            Some(_) => (),
            None => {
                curr_x = line_chars.len().saturating_sub(1) as u16;
            }
        }

        self.cursor_pos = Some((curr_x, curr_y));
    }
}

/// Highlight a section of the line.
///
/// The end char index is exclusive.
fn highlight_line_section(line: &str, start: u16, end: u16, style: Style) -> Vec<Span> {
    let chars = line.chars().collect::<Vec<_>>();

    match (
        start < end,
        start < chars.len() as u16,
        end < chars.len() as u16,
    ) {
        // we are happy
        (true, true, true) => {
            let (left, center) = chars.split_at(start as usize);

            let (to_highlight, right) = center.split_at((end - start) as usize);

            vec![
                Span::raw(left.iter().collect::<String>()),
                Span::styled(to_highlight.iter().collect::<String>(), style),
                Span::raw(right.iter().collect::<String>()),
            ]
        }

        // no highlighting
        _ => {
            vec![Span::raw(line)]
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_highlight_line_section() {
        let line = "hello world";
        let style = Style::new().reversed();

        let spans = highlight_line_section(line, 2, 6, style);

        assert_eq!(spans.len(), 3);
    }
}
