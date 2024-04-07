//! Various UI widgets

use std::{
    borrow::Cow,
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

use super::{tui::FocusedWidget, Ui};

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

    /// Render the widget with a brighter border when focused
    focused: bool,

    /// Dialogue contents and error flag
    dialogue: Option<(String, bool)>,
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

    /// Cursor position in the file: (x_col, y_row)
    ///
    /// This will highlight the current line and character in the file.
    cursor_pos: Option<(u16, u16)>,

    /// Area of text to highlight, from an offset and a length.
    ///
    /// This item supercedes the cursor position.
    /// If this is Some(_), the cursor is not rendered.
    ///
    /// The region of text includes newlines.
    highlight: Option<(usize, usize)>,

    /// Notifications are displayed over the main contents like a pop-up.
    /// Errors take precedence over notifications.
    notification: Option<String>,

    /// Error messages are displayed over the main contents like a pop-up.
    error_message: Option<String>,

    /// Render the widget with a brighter border when focused
    focused: bool,
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
                        Span::raw(en.path().to_str().expect("invalid path").to_string())
                    } else {
                        Span::styled(
                            en.path().to_str().expect("invalid path").to_string(),
                            Style::new().green().bold(),
                        )
                    };

                    Line::from(contents)
                })
                .collect::<Vec<_>>(),

            (Some(dirs), Some(mut selection)) => {
                // let all_lines = dirs
                //     .iter()
                //     .enumerate()
                //     .map(|(idx, line)| {
                //         let mut contents = if line.is_file() {
                //             Span::raw(line.path().to_str().expect("invalid path").to_string())
                //         } else {
                //             Span::styled(
                //                 line.path().to_str().expect("invalid path").to_string(),
                //                 Style::new().green().bold(),
                //             )
                //         };

                //         // highlight selection
                //         if selection == idx {
                //             contents = contents.reversed()
                //         }

                //         Line::from(contents)
                //     })
                //     .collect::<Vec<_>>();

                // let shown_lines =
                //     fit_lines_to_window(all_lines, selection as u16, area.height, true, 2);

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

                // lines.collect::<Vec<_>>()
            }
        };

        let para = Paragraph::new(lines)
            .block(
                DEFAULT_BLOCK
                    .title(
                        Title::from(
                            self.parent_dir
                                .to_str()
                                .expect("invalid path")
                                .bold()
                                .gray(),
                        )
                        .alignment(ratatui::layout::Alignment::Left),
                    )
                    .border_style(match self.focused {
                        true => Style::new().white(),
                        false => Style::new().gray(),
                    }),
            )
            .wrap(Wrap { trim: false });

        para.render(area, buf);

        if let Some((entry, err)) = self.dialogue {
            let popup_style = match err {
                true => Style::new().red(),
                false => Style::new().white(),
            };

            let popup_contents = [
                Line::from(entry),
                Line::from(""),
                Line::from(""),
                Line::from("enter").style(popup_style),
            ]
            .into_iter()
            .collect::<Vec<_>>();

            // error message takes up half the screen in each dimension
            let dialogue_rect = fixed_width_rect(85, popup_contents.len() as u16 + 2, area);

            Clear.render(dialogue_rect, buf);

            let popup = Paragraph::new(popup_contents)
                .block(
                    DEFAULT_BLOCK
                        .borders(Borders::ALL)
                        .border_style(popup_style)
                        .title("create")
                        .title_alignment(ratatui::layout::Alignment::Center),
                )
                .alignment(ratatui::layout::Alignment::Center);

            popup.render(dialogue_rect, buf)
        }
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

fn fixed_width_rect(x_percent: u16, y_max_lines: u16, rect: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Max(y_max_lines),
        Constraint::Fill(1),
    ])
    .split(rect);

    Layout::horizontal([
        Constraint::Percentage((100 - x_percent) / 2),
        Constraint::Percentage(x_percent),
        Constraint::Percentage((100 - x_percent) / 2),
    ])
    .split(popup_layout[1])[1]
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

/// Formats the line number with padding and an indicator.
fn line_number(num: usize, padding: usize, indicator: char) -> String {
    format!("{:<padding$} {} ", num, indicator, padding = padding)
}

impl Widget for ContentWindow {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // clear the area first
        Clear.render(area, buf);

        let border_style = match self.focused {
            true => Style::new().white(),
            false => Style::new().gray(),
        };
        let border = DEFAULT_BLOCK.border_style(border_style);

        let main_para = match (self.contents, self.cursor_pos, self.highlight) {
            // render a blank screen
            (None, _, _) => Paragraph::default().block(border),

            // render contents w/ line numbers
            (Some(contents), None, None) => {
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
                                    line_number(line_num + 1, num_line_digits as usize, '|'),
                                    Style::new().bold(),
                                ),
                                Span::raw(line.to_owned()),
                            ])
                        })
                        .collect::<Vec<_>>(),
                )
                .block(border)
            }
            // highlight some text
            (Some(contents), cursor_opt, Some((h_start, h_len))) => {
                let lines = contents.split('\n').collect::<Vec<_>>();

                // let res = highlight_text(contents, h_start, h_len);
                let res = highlight_text(
                    contents,
                    h_start,
                    h_len,
                    self.cursor_pos.and_then(|(_, line)| Some(line)),
                );

                let rendered_lines = match cursor_opt {
                    Some((_, other)) => fit_lines_to_window(res, other, area.height, true, 2),

                    None => Box::new(res.into_iter()),
                };

                Paragraph::new(rendered_lines.collect::<Vec<_>>()).block(border)
                // todo!()
            }

            // render contents w/ scrolling
            (Some(contents), Some((cursor_x, cursor_y)), None) => {
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
                                    line_number(line_num + 1, num_line_digits as usize, '>'),
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
                                    line_number(line_num + 1, num_line_digits as usize, '|'),
                                    Style::new().bold(),
                                ),
                                Span::raw(contents.to_owned()),
                            ])
                        }
                    })
                    .collect::<Vec<_>>();

                // filter to fit the area
                // let rendered_lines: Box<dyn Iterator<Item = _>> = match lines.len()
                //     > area.height as usize - FRAME_BORDER_LINES
                //     && (cursor_y as usize + 1 + 2) > area.height as usize - FRAME_BORDER_LINES
                // {
                //     true => Box::new(
                //         lines
                //             .iter()
                //             .skip(
                //                 (cursor_y as usize + 1 + 2).saturating_sub(area.height as usize)
                //                     + FRAME_BORDER_LINES,
                //             )
                //             .take(area.height as usize - FRAME_BORDER_LINES)
                //             .cloned(),
                //     ),
                //     false => Box::new(lines.into_iter()),
                // };

                let rendered_lines = fit_lines_to_window(lines, cursor_y, area.height, true, 2);

                Paragraph::new(rendered_lines.collect::<Vec<_>>())
                    .block(border)
                    .wrap(Wrap { trim: false })
            }
        };

        main_para.render(area, buf);

        // notifications are written to the title border and override the border colour
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

            let popup = Paragraph::new(err_msg.bold().light_cyan().reversed())
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

impl FocusedWidget for FsTree {
    fn focus(&mut self, selected: bool) {
        self.focused = selected;
    }
}

impl FocusedWidget for ContentWindow {
    fn focus(&mut self, selected: bool) {
        self.focused = selected;
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
            focused: false,
            dialogue: None,
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

    /// Show a dialogue box with a message. Used for file/dir creation
    pub fn dialogue_box<T: ToString>(&mut self, message: Option<T>, error: bool) {
        self.dialogue = match message {
            Some(m) => Some((m.to_string(), error)),
            None => None,
        };
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
    ///
    /// ```ignore
    /// let mut commands = AvailableCommands::new();
    ///
    /// commands.add([("q", "quit"), ("h", "help")]);
    /// ```
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
            focused: false,
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

    /// Highlight a section of text in the file. The highlighted range is inclusive.
    ///
    /// This method performs some checks to ensure that the start and end positions are valid.
    // pub fn set_highlight(&mut self, highlight: Option<((u16, u16), (u16, u16))>) {
    //     self.highlight = match highlight {
    //         // compare y and x values
    //         Some((start, end)) => match (end.1.cmp(&start.1), end.0.cmp(&start.0)) {
    //             // end row is less than start row, do not assign
    //             (std::cmp::Ordering::Less, _) => None,

    //             (
    //                 std::cmp::Ordering::Equal,
    //                 std::cmp::Ordering::Greater | std::cmp::Ordering::Equal,
    //             ) => Some((start, end)),
    //             // start and end in the same line, but end char is lte start char
    //             (std::cmp::Ordering::Equal, _) => None,

    //             (std::cmp::Ordering::Greater, _) => Some((start, end)),
    //             // _ => todo!(),
    //         },
    //         None => None,
    //     };
    // }

    /// Highlight a section of text given a starting point and a length.
    ///
    /// Depending on the contents of the file, the highlighted region can span multiple lines.
    /// Whitespaces are ignored in the calculation of the length.
    /// This can span multiple lines.
    pub fn set_highlight(&mut self, offset: usize, len: usize) {
        self.highlight = Some((offset, len));
    }

    /// Clears highlighting
    pub fn clear_highlight(&mut self) {
        self.highlight = None;
    }

    pub fn pos(&self) -> Option<(u16, u16)> {
        self.cursor_pos
    }

    /// Returns the current cursor position in the file relative to the entire block of text.
    pub fn cursor_offset(&self) -> Option<usize> {
        let contents = self.contents.as_deref()?;
        let (cursor_x, cursor_y) = self.cursor_pos?;

        let lines = contents.split('\n').collect::<Vec<_>>();
        let num_full_lines = (cursor_y as usize).saturating_sub(1);

        // count all chars (incl whitespace) for all full lines
        let full_line_char_count = lines
            .iter()
            .take(cursor_y as usize)
            .map(|l| l.len())
            .sum::<usize>()
            + cursor_y as usize;

        let last_line_char_count = match lines.get(cursor_y as usize) {
            Some(l) => {
                cursor_x as usize
                    + match num_full_lines {
                        0 => 0,
                        _ => 1,
                    }
            } // newline
            None => 0,
        };

        Some(full_line_char_count + last_line_char_count)
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
fn highlight_line_section(
    line: String,
    line_num: usize,
    line_num_padding: usize,
    start: u16,
    end: u16,
    style: Style,
    selected: bool,
) -> Vec<Span<'static>> {
    let chars = line.chars().collect::<Vec<_>>();

    // println!("start: {}, end: {}, len: {}", start, end, chars.len());

    match (
        start < end,
        start < chars.len() as u16,
        end <= chars.len() as u16,
    ) {
        // we are happy
        (true, true, true) => {
            let (left, center) = chars.split_at(start as usize);

            let (to_highlight, right) = center.split_at((end - start) as usize);

            vec![
                match selected {
                    true => Span::styled(
                        line_number(line_num, line_num_padding, '>'),
                        Style::new().bold().white(),
                    ),
                    false => Span::styled(
                        line_number(line_num, line_num_padding, '|'),
                        Style::new().bold(),
                    ),
                },
                Span::raw(left.iter().collect::<String>()),
                Span::styled(to_highlight.iter().collect::<String>(), style),
                Span::raw(right.iter().collect::<String>()),
            ]
        }

        // no highlighting
        _ => {
            // unimplemented!()
            vec![Span::raw(line.to_string())]
        }
    }
}

/// Highlight a block of text based on the char offset and length.
///
/// Line numbers are rendered as well.
///
/// Whitespaces are included in the count, but are not highlighted.
fn highlight_text(
    text: String,
    offset: usize,
    len: usize,
    cursor_line: Option<u16>,
) -> Vec<Line<'static>> {
    let text_lines = text.split('\n').map(|l| l.to_string()).collect::<Vec<_>>();

    let mut remaining_offset = offset;
    let mut remaining_len = len;
    let mut collected_lines = Vec::new();
    let line_padding = match text_lines.len() {
        0 => 1,
        n => n.ilog10() + 1,
    };

    for (idx, line) in text_lines.into_iter().enumerate() {
        // to push raw lines
        if remaining_offset > 0 {
            match line.len().cmp(&remaining_offset) {
                // push line as unformatted text
                std::cmp::Ordering::Less | std::cmp::Ordering::Equal => {
                    remaining_offset -= line.len();

                    let new_line = Line::from(vec![
                        cursor_line
                            .and_then(|l| {
                                if l == idx as u16 {
                                    Some(Span::styled(
                                        line_number(idx + 1, line_padding as usize, '>'),
                                        Style::new().bold().white(),
                                    ))
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(Span::styled(
                                line_number(idx + 1, line_padding as usize, '|'),
                                Style::new().bold(),
                            )),
                        Span::raw(line),
                    ]);

                    collected_lines.push(new_line);
                    remaining_offset = remaining_offset.saturating_sub(1); // account for newline
                    continue;
                }
                // some part of the line will be highlighted
                std::cmp::Ordering::Greater => {
                    // calc highlight indices
                    let highlight_start = remaining_offset as u16;
                    let (highlight_end, used_len) =
                        match line.len().cmp(&(remaining_offset + remaining_len)) {
                            // highlight entire line starting from offset
                            std::cmp::Ordering::Less => {
                                (line.len() as u16, line.len() - remaining_offset)
                            }
                            // highlight line from offset -> end
                            std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => {
                                (line.len() as u16, line.len() - remaining_offset)
                            }
                        };

                    let new_spans = highlight_line_section(
                        line,
                        idx + 1,
                        line_padding as usize,
                        highlight_start,
                        highlight_end,
                        Style::new().bold().light_cyan().reversed(),
                        cursor_line
                            .and_then(|l| Some(l == idx as u16))
                            .unwrap_or(false),
                    );

                    collected_lines.push(Line::from(new_spans));
                    remaining_offset = 0;
                    remaining_len = remaining_len.saturating_sub(used_len + 1); // account for newline
                    continue;
                }
            }
        }

        if remaining_len > 0 {
            let highlight_len = match line.len().cmp(&remaining_len) {
                // highlight entire line
                std::cmp::Ordering::Less | std::cmp::Ordering::Equal => line.len(),
                std::cmp::Ordering::Greater => remaining_len,
            } as u16;

            let new_spans = highlight_line_section(
                line,
                idx + 1,
                line_padding as usize,
                0,
                highlight_len,
                Style::new().bold().light_cyan().reversed(),
                cursor_line
                    .and_then(|l| Some(l == idx as u16))
                    .unwrap_or(false),
            );

            collected_lines.push(Line::from(new_spans));
            remaining_len = remaining_len.saturating_sub(highlight_len as usize + 1); // account for newline
            continue;
        }

        // no more highlighting to do
        collected_lines.push(Line::from(vec![
            cursor_line
                .and_then(|l| {
                    if l == idx as u16 {
                        Some(Span::styled(
                            line_number(idx + 1, line_padding as usize, '>'),
                            Style::new().bold().white(),
                        ))
                    } else {
                        None
                    }
                })
                .unwrap_or(Span::styled(
                    line_number(idx + 1, line_padding as usize, '|'),
                    Style::new().bold(),
                )),
            Span::raw(line),
        ]))
    }

    collected_lines
}

/// Given a formatted vector of lines, the viewport dims, the selected line, determine which lines to render.
fn fit_lines_to_window(
    content_lines: Vec<Line<'static>>,
    selected_idx: u16,
    viewport_height: u16,
    // if widget has borders enabled
    has_borders: bool,
    // extra lines to show below selected line
    bottom_padding: u16,
) -> Box<dyn Iterator<Item = Line<'static>>> {
    // asd

    let border_padding = match has_borders {
        true => 2,
        false => 0,
    };

    match content_lines.len() > viewport_height as usize - border_padding
        && (selected_idx as usize + 1 + bottom_padding as usize)
            > viewport_height as usize - border_padding
    {
        true => Box::new(
            content_lines
                .into_iter()
                .skip(
                    (selected_idx as usize + 1 + bottom_padding as usize)
                        .saturating_sub(viewport_height as usize)
                        + border_padding,
                )
                .take(viewport_height as usize - border_padding),
        ),
        false => Box::new(content_lines.into_iter()),
    }
}
#[cfg(test)]
mod tests {

    use std::{io, time::Duration};

    use crossterm::event;
    use ratatui::{backend::CrosstermBackend, Terminal};

    use super::*;

    #[test]
    fn test_highlight_line_section() {
        let line = "hello world";
        let style = Style::new().reversed();

        let spans = highlight_line_section(line.to_owned(), 1, 2, 2, 6, style, true);

        println!("spans: {:#?}", spans);

        assert_eq!(spans.len(), 4);
    }

    #[test]
    fn test_highlight_text() -> io::Result<()> {
        let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

        let contents = "0123456789\n0123456789\n0123456789\n".to_string();
        let mut c_window = ContentWindow::new();

        let lines = highlight_text(contents, 3, 10, None);

        println!("lines: {:#?}", lines);

        Ok(())
    }

    #[test]
    fn test_cursor_offset() {
        let mut content_widget = ContentWindow::new();
        content_widget.set_contents(Some("1234\n1234\n123"));
        assert_eq!(content_widget.cursor_offset(), None);

        content_widget.set_cursor_pos(Some((0, 0)));
        assert_eq!(content_widget.cursor_offset(), Some(0));

        content_widget.set_cursor_pos(Some((1, 0)));
        assert_eq!(content_widget.cursor_offset(), Some(1));

        content_widget.set_cursor_pos(Some((0, 1)));
        assert_eq!(content_widget.cursor_offset(), Some(5));

        content_widget.set_cursor_pos(Some((1, 1)));
        assert_eq!(content_widget.cursor_offset(), Some(6));

        content_widget.set_cursor_pos(Some((0, 2)));
        assert_eq!(content_widget.cursor_offset(), Some(11));
    }

    // / Test the set_highlight() method
    // #[test]
    // fn test_set_highlight_input() {
    //     let mut content = ContentWindow::new();

    //     // start and end is the same point, valid
    //     content.set_highlight(Some(((0, 0), (0, 0))));
    //     assert_eq!(content.highlight, Some(((0, 0), (0, 0))));

    //     // valid start, valid end
    //     content.set_highlight(Some(((0, 0), (1, 1))));
    //     assert_eq!(content.highlight, Some(((0, 0), (1, 1))));

    //     // valid start, invalid end
    //     content.set_highlight(Some(((1, 1), (0, 0))));
    //     assert_eq!(content.highlight, None);

    //     // start and end on the same line, valid
    //     content.set_highlight(Some(((1, 1), (2, 1))));
    //     assert_eq!(content.highlight, Some(((1, 1), (2, 1))));

    //     // start and end on the same line, invalid
    //     content.set_highlight(Some(((1, 1), (0, 1))));
    //     assert_eq!(content.highlight, None);
    // }
}
