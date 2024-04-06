//! App module. Contains application state
//!
//! For simplicity, only single key events are handled here (no modifiers).

use std::{collections::HashMap, default, io};

use crossterm::event::KeyEvent;
use rfs::fs::VirtFile;
use rfs::fsm::TransitableState;
use rfs::interfaces::FileUpdate;
use rfs::{fs::VirtReadDir, middleware::ContextManager, state_transitions};

use super::tui::Tui;

/// Application state
#[derive(Debug)]
pub struct App {
    exit: bool,

    ctx: ContextManager,

    // stack of open filesystem dirs
    fs_dirs: FixedSizeStack<(String, VirtReadDir)>,

    // current selection idx in the filsystem
    filesystem_pos: usize,

    /// Current open virtual file in the content window
    v_file: Option<VirtFile>,

    /// Contents from the virtual file
    content: Option<String>,

    /// Cursor position in the contents widget
    contents_pos: Option<usize>,

    /// A continuous unbroken string sequence that has not been written to the file.
    ///
    /// Offset is taken from contents_pos
    unsaved_buf: String,

    /// App state
    state: AppState,

    /// State history. Not sure if this is required.
    state_stack: FixedSizeStack<AppState>,
}

/// An (optionally) fixed size stack of elements
#[derive(Clone, Debug)]
pub struct FixedSizeStack<T> {
    size: Option<usize>,
    stack: Vec<T>,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum AppState {
    /// User is on the content widget
    #[default]
    OnContent,

    /// User has entered cursor in content widget.
    ///
    /// Actions like paste and data input are performed here
    InContent,

    /// User is on the filessystem widget
    OnFileSystem,

    /// User has entered the filesystem widget.
    ///
    /// Stuff that can be done: navigate thru and open files/dirs
    InFileSystem,
    // Error,
}

/// App events are a subset of [KeyEvent]
pub enum AppEvents {
    EnterKey,
    EscKey,
    LeftArrowKey,
    RightArrowKey,
}

state_transitions! {
    type State = AppState;
    type Event = AppEvents;

    OnContent + EnterKey => InContent;
    InContent + EscKey => OnContent;

    OnContent + LeftArrowKey => OnFileSystem;
    OnFileSystem + RightArrowKey => OnContent;

    OnFileSystem + EnterKey => InFileSystem;
    InFileSystem + EscKey => OnFileSystem;

    // error ack
    // Error + EnterKey => OnFileSystem;
}

// if key event can be translated into a state event, then handle the state event.
//
// State events are a subset of key events.
impl TryFrom<KeyEvent> for AppEvents {
    type Error = ();

    fn try_from(value: KeyEvent) -> Result<Self, Self::Error> {
        match value.code {
            crossterm::event::KeyCode::Enter => Ok(Self::EnterKey),
            crossterm::event::KeyCode::Left => Ok(Self::LeftArrowKey),
            crossterm::event::KeyCode::Right => Ok(Self::RightArrowKey),
            crossterm::event::KeyCode::Esc => Ok(Self::EscKey),
            _ => Err(()),
        }
    }
}

impl App {
    pub fn new(ctx: ContextManager, tick_rate: f64, frame_rate: f64) -> Self {
        Self {
            exit: false,
            ctx,
            fs_dirs: FixedSizeStack::new(None),
            filesystem_pos: todo!(),
            v_file: None,
            content: None,
            contents_pos: None,
            unsaved_buf: Default::default(),
            state: Default::default(),
            state_stack: {
                let mut stack = FixedSizeStack::new(Some(10));
                stack.push(Default::default());
                stack
            },
        }
    }

    /// This is the main application loop.
    /// A [Tui] is instantiated here and used to render the UI.
    pub async fn run(&mut self) -> io::Result<()> {
        let mut tui = Tui::new(60.0, 4.0)?;

        tui.enter()?;
        tui.start();

        // tui.draw(f);
        while let Some(event) = tui.next().await {
            match event {
                super::tui::AppEvent::Init => {
                    // asd
                    self.init();

                    // render the result
                    tui.event_tx.send(super::tui::AppEvent::Render).unwrap();
                }
                super::tui::AppEvent::Quit => {
                    tui.stop();
                    tui.exit()?;
                    break;
                }
                super::tui::AppEvent::Error => todo!(),
                super::tui::AppEvent::Closed => break,
                super::tui::AppEvent::Tick
                | super::tui::AppEvent::Render
                | super::tui::AppEvent::Resize(_, _) => tui.draw_to_screen().await?,
                super::tui::AppEvent::FocusGained => (),
                super::tui::AppEvent::FocusLost => (),
                super::tui::AppEvent::Paste(paste_str) => {
                    log::debug!("paste not yet handled")
                }
                super::tui::AppEvent::Key(key_event) => {
                    self.handle_key_event(key_event, &mut tui).await
                }
                super::tui::AppEvent::Mouse(_) => (),
            }
        }

        Ok(())
    }

    /// Initialize the app by performing some initial queries to the remote
    pub fn init(&mut self) {}

    // main entry point for keyboard interactions
    async fn handle_key_event(&mut self, key_event: KeyEvent, tui: &mut Tui) {
        match self.state {
            AppState::OnContent | AppState::InContent => {
                self.handle_content_key_event(key_event, tui).await
            }
            AppState::OnFileSystem | AppState::InFileSystem => {
                self.handle_fs_tree_key_event(key_event, tui).await
            }
        }
    }

    /// Handle key events when in or on the fstree widget
    async fn handle_fs_tree_key_event(&mut self, key_event: KeyEvent, tui: &mut Tui) {
        match self.state {
            AppState::OnFileSystem => {}
            AppState::InFileSystem => {}
            _ => unimplemented!(),
        }
    }

    /// Handle key events when in or on the content widget
    async fn handle_content_key_event(&mut self, key_event: KeyEvent, tui: &mut Tui) {
        let app_ev = AppEvents::try_from(key_event).ok();

        match self.state {
            // only enter key can transition state
            AppState::OnContent => {
                tui.fs_widget.select(None);

                match (&mut self.v_file, self.contents_pos) {
                    (None, None) => (),
                    (None, Some(_)) => self.contents_pos = None,
                    (Some(_), None) => (),
                    // write contents to file
                    (Some(v_f), Some(pos)) => {
                        let update =
                            FileUpdate::Insert((pos, self.unsaved_buf.as_bytes().to_vec()));

                        match v_f.write_bytes(update).await {
                            Ok(num_bytes) => {
                                self.unsaved_buf.clear();
                                // self.contents_pos = None;
                            }
                            Err(e) => {
                                log::error!("error writing to file: {:?}", e);
                                todo!()
                            }
                        };
                    }
                }
            }
            AppState::InContent => {
                tui.fs_widget.select(Some(self.filesystem_pos));

                let c = key_event.code;
                match c {
                    crossterm::event::KeyCode::Backspace => {
                        self.unsaved_buf.pop();
                    }
                    crossterm::event::KeyCode::Enter => self.unsaved_buf.push('\n'),
                    // no navi when in insert mode
                    crossterm::event::KeyCode::Left
                    | crossterm::event::KeyCode::Right
                    | crossterm::event::KeyCode::Up
                    | crossterm::event::KeyCode::Down => (),
                    crossterm::event::KeyCode::Delete => {
                        self.unsaved_buf.pop();
                    }
                    crossterm::event::KeyCode::Char(c) => self.unsaved_buf.push(c),
                    // no handler for the rest
                    _ => (),
                }
            }
            _ => unimplemented!(),
        }

        // perform any state transitions
        if let Some(ev) = app_ev {
            self.state.ingest(ev);
        }
    }
}

impl<T> FixedSizeStack<T> {
    pub fn new(size: Option<usize>) -> Self {
        Self {
            size,
            stack: Vec::new(),
        }
    }

    pub fn push(&mut self, item: T) {
        match self.size {
            Some(size) => {
                if self.stack.len() == size {
                    self.stack.remove(0);
                }
            }
            None => (),
        }

        self.stack.push(item);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.stack.pop()
    }
}
