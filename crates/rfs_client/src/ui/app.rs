//! App module. Contains application state
//!
//! For simplicity, only single key events are handled here (no modifiers).

use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, default, io};

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use rfs::fs::VirtFile;
use rfs::fsm::TransitableState;
use rfs::interfaces::FileUpdate;
use rfs::{fs::VirtReadDir, middleware::ContextManager, state_transitions};
use tokio::sync::Mutex;

use super::tui::{AppEvent, FocusedWidget, Tui};

const FS_CREATE_FILE: char = 'f';
const FS_CREATE_DIR: char = 'd';
const FS_DELETE: char = 'x';

// feature not impl'd
const FS_RENAME: char = 'r';

/// Trait for handling application state.
///
/// ```ignore
/// #[async_trait]
/// impl HandleStateEvent for AppState {
///     async fn handle_event(&mut self, event: KeyEvent, app_data: &mut AppData, tui: &mut Tui) {
///         todo!()
///     }
/// }
/// ```
#[async_trait]
trait HandleStateEvent {
    async fn handle_event(&mut self, event: KeyEvent, app_data: &mut AppData, tui: &mut Tui);
}

/// Application state
// #[derive(Debug)]
pub struct App {
    exit: bool,

    /// All app-related data lives in here
    data: AppData,

    sh: Arc<std::sync::Mutex<shh::ShhStderr>>,

    // ctx: ContextManager,

    // // stack of open filesystem dirs
    // fs_dirs: FixedSizeStack<(String, VirtReadDir)>,

    // // current selection idx in the filsystem
    // filesystem_pos: usize,

    // /// Current open virtual file in the content window
    // v_file: Option<VirtFile>,

    // /// Contents from the virtual file
    // content: Option<String>,

    // /// Cursor position in the contents widget
    // contents_pos: Option<usize>,

    // /// A continuous unbroken string sequence that has not been written to the file.
    // ///
    // /// Offset is taken from contents_pos
    // unsaved_buf: String,

    // /// Error message to overlay on the screen
    // err_msg: Option<String>,
    /// App state
    state: AppState,

    /// State history. Not sure if this is required.
    state_stack: FixedSizeStack<AppState>,
}

// q: how can I have a struct field be a reference to another field in the same struct?
// a: you can't. You can use a `Rc` or `Arc` to share ownership between fields.
/// App-specific data
#[derive(Debug)]
pub struct AppData {
    ctx: ContextManager,

    // stack of open filesystem dirs
    fs_dirs: FixedSizeStack<(String, VirtReadDir)>,

    // current selection idx in the filsystem
    filesystem_pos: usize,

    /// Previously opened virtual files
    v_file_history: HashMap<String, Arc<Mutex<VirtFile>>>,

    /// Current open virtual file in the content window
    v_file: Option<Arc<Mutex<VirtFile>>>,

    /// Contents from the virtual file
    content: Option<String>,

    /// Cursor position in the contents widget
    contents_pos: Option<usize>,

    /// A continuous unbroken string sequence that has not been written to the file.
    ///
    /// Offset is taken from contents_pos
    unsaved_buf: String,

    /// Error message to overlay on the screen
    err_msg: Option<String>,
}

/// An (optionally) fixed size stack of elements
#[derive(Clone, Debug)]
pub struct FixedSizeStack<T> {
    size: Option<usize>,
    stack: Vec<T>,
}

#[derive(Clone, Debug, Default)]
pub enum AppState {
    /// User is on the content widget
    #[default]
    OnContent,

    /// User has entered cursor in content widget.
    ///
    /// Actions like paste and data input are performed here
    InContent(ContentState),

    /// User is on the filessystem widget
    OnFileSystem,

    /// User has entered the filesystem widget.
    ///
    /// Stuff that can be done: navigate thru and open files/dirs
    InFileSystem(FsState),
    // Error,
}

/// Content widget state
#[derive(Clone, Debug, Default)]
pub enum ContentState {
    /// Arrow key navigation
    #[default]
    Navigate,

    /// Content insert
    Insert,

    /// File watch
    Watch,
}

/// Filesystem inner state
#[derive(Clone, Debug, Default)]
pub enum FsState {
    #[default]
    Navigate,

    CreateFile(String),

    CreateDir(String),
}

/// App events are a subset of [KeyEvent]
#[derive(Clone, Debug)]
pub enum AppEvents {
    EnterKey,
    EscKey,
    LeftArrowKey,
    RightArrowKey,
    Char(char),
}

// impl TransitableState for AppState {
//     type Event = AppEvents;

//     fn ingest(&mut self, event: Self::Event) {
//         match (&self, event) {
//             (Self::OnContent, AppEvents::EnterKey) => {
//                 *self = Self::InContent(ContentState::default())
//             }
//             (Self::InContent(_), AppEvents::EscKey) => *self = Self::OnContent,

//             (Self::OnContent, AppEvents::LeftArrowKey) => *self = Self::OnFileSystem,
//             (Self::OnFileSystem, AppEvents::RightArrowKey) => *self = Self::OnContent,

//             (Self::OnFileSystem, AppEvents::EnterKey) => {
//                 *self = Self::InFileSystem(FsState::default())
//             }
//             (Self::InFileSystem(_), AppEvents::EscKey) => *self = Self::OnFileSystem,

//             _ => (),
//         }
//     }
// }

// impl TransitableState for ContentState {
//     type Event = AppEvents;

//     fn ingest(&mut self, event: Self::Event) {
//         match (self, event) {
//             (Self::Insert, AppEvents::EnterKey) => (),
//             (Self::Insert, AppEvents::EscKey) => (),
//             _ => (),
//         }
//     }
// }

// state_transitions! {
//     type State = AppState;
//     type Event = AppEvents;

//     OnContent + EnterKey => InContent;
//     InContent + EscKey => OnContent;

//     OnContent + LeftArrowKey => OnFileSystem;
//     OnFileSystem + RightArrowKey => OnContent;

//     OnFileSystem + EnterKey => InFileSystem;
//     InFileSystem + EscKey => OnFileSystem;

//     // error ack
//     // Error + EnterKey => OnFileSystem;
// }

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
    pub fn new(ctx: ContextManager, tick_rate: f64, frame_rate: f64, shh: shh::ShhStderr) -> Self {
        Self {
            exit: false,
            data: AppData::new(ctx),
            sh: Arc::new(std::sync::Mutex::new(shh)),
            // ctx,
            // fs_dirs: FixedSizeStack::new(None),
            // filesystem_pos: todo!(),
            // v_file: None,
            // content: None,
            // contents_pos: None,
            // unsaved_buf: Default::default(),
            // err_msg: None,
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
        let mut tui = Tui::new(60.0, 4.0, self.sh.clone())?;
        tui.enter()?;
        tui.start();

        // tui.draw(f);
        while let Some(event) = tui.next().await {
            match event {
                AppEvent::Init => {
                    // asd
                    self.init(&mut tui).await;

                    // render the result
                    tui.event_tx.send(AppEvent::Render).unwrap();
                }
                AppEvent::Quit => {
                    tui.stop();
                    tui.exit()?;
                    break;
                }
                AppEvent::Error(e) => todo!(),
                AppEvent::Closed => break,
                AppEvent::Tick | AppEvent::Render | AppEvent::Resize(_, _) => {
                    // tui.logs_widget.update_logs();
                    tui.draw_to_screen().await?
                }
                AppEvent::FocusGained => (),
                AppEvent::FocusLost => (),
                AppEvent::Paste(paste_str) => {
                    log::debug!("paste not yet handled")
                }
                AppEvent::Key(key_event) => {
                    self.data
                        .handle_app_state(&mut self.state, key_event, &mut tui)
                        .await
                }
                AppEvent::Mouse(_) => (),
                AppEvent::SetContentNotification(notif) => {
                    tui.content_widget.set_notification(notif);
                }
                AppEvent::HighlightContent(content) => match content {
                    Some((offset, len)) => {
                        tui.content_widget.set_highlight(offset, len);
                    }
                    None => tui.content_widget.clear_highlight(),
                },
                AppEvent::FileUpdate { path, upd } => {
                    log::debug!("file update event for: {:?}", path);
                    //
                    let v_file = match &self.data.v_file {
                        Some(vf) => vf,
                        // ignore
                        None => continue,
                    };

                    log::debug!("acquiring lock on current vfile");
                    let mut lock = v_file.lock().await;
                    let upd_dur = Duration::from_secs(2);
                    match &lock.as_path() == &path {
                        // curr file is being updated
                        true => {
                            match &upd {
                                FileUpdate::Append(data) => Self::show_highlight(
                                    lock.local_cache().len(),
                                    data.len(),
                                    upd_dur,
                                    &tui,
                                ),
                                FileUpdate::Insert((offset, data)) => {
                                    Self::show_highlight(*offset, data.len(), upd_dur, &tui)
                                }
                                FileUpdate::Overwrite(data) => {
                                    Self::show_highlight(0, data.len(), upd_dur, &tui)
                                }
                            }

                            lock.update_bytes(upd);
                        }
                        // search for other files in lookup and update it
                        false => match self.data.v_file_history.get(&path) {
                            Some(vf) => {
                                log::debug!("updating file in history");
                                let mut map_lock = vf.lock().await;

                                Self::show_notification(
                                    format!("{} updated", &path),
                                    Duration::from_secs(2),
                                    &tui,
                                );

                                map_lock.update_bytes(upd);
                            }
                            None => (),
                        },
                    }

                    // possible race condition: file watch for previous file completes
                    // while new file is still being watched
                }
            }
        }

        Ok(())
    }

    /// Initialize the app by populating the curdir and setting the initial state
    pub async fn init(&mut self, tui: &mut Tui) {
        self.state = AppState::OnFileSystem;

        let start_dir_entry = rfs::fs::read_dir(self.data.ctx.clone(), ".").await.unwrap();

        self.data
            .fs_dirs
            .push((".".to_string(), start_dir_entry.clone()));

        tui.fs_widget.push(start_dir_entry, ".");
    }

    // main entry point for keyboard interaction
    // no longer used, see `impl AppData`.
    async fn handle_key_event(&mut self, key_event: KeyEvent, tui: &mut Tui) {
        // match self.state {
        //     AppState::OnContent | AppState::InContent => {
        //         self.handle_content_key_event(key_event, tui).await
        //     }
        //     AppState::OnFileSystem | AppState::InFileSystem => {
        //         self.handle_fs_tree_key_event(key_event, tui).await
        //     }
        // }
    }

    /// Handle key events when in or on the fstree widget
    async fn handle_fs_tree_key_event(&mut self, key_event: KeyEvent, tui: &mut Tui) {
        let app_ev = AppEvents::try_from(key_event).ok();

        match self.state {
            // only enter key can trainsition this state
            AppState::OnFileSystem => {}

            // AppState::InFileSystem => {
            //     let dir = match self.fs_dirs.top() {
            //         Some(d) => d,
            //         None => return,
            //     };

            //     match key_event.code {
            //         KeyCode::Enter => {
            //             let dir_entry = match dir.1.get(self.filesystem_pos) {
            //                 Some(entry) => entry,
            //                 None => {
            //                     self.filesystem_pos = 0;
            //                     return;
            //                 }
            //             };
            //         }

            //         KeyCode::Up => {
            //             self.filesystem_pos = self.filesystem_pos.saturating_sub(1);
            //         }
            //         KeyCode::Down => {
            //             // saturating add against dir index
            //             self.filesystem_pos = match dir.1.get(self.filesystem_pos + 1) {
            //                 Some(_) => self.filesystem_pos + 1,
            //                 None => self.filesystem_pos,
            //             };
            //         }

            //         KeyCode::Char(FS_CREATE_FILE) => {
            //             let x = FS_CREATE_DIR;
            //         }

            //         KeyCode::Char(FS_CREATE_DIR) => {
            //             let x = FS_CREATE_DIR;
            //         }

            //         _ => todo!(),
            //     }
            // }
            _ => unimplemented!(),
        }

        if let Some(ev) = app_ev {
            // log::debug!("ingesting event: {:?}", ev);
            // self.state.ingest(ev);
        }
    }

    /// Handle key events when in or on the content widget
    async fn handle_content_key_event(&mut self, key_event: KeyEvent, tui: &mut Tui) {
        let app_ev = AppEvents::try_from(key_event).ok();

        // match self.state {
        //     // only enter key can transition state
        //     AppState::OnContent => {
        //         tui.fs_widget.select(None);

        //         match (&mut self.v_file, self.contents_pos) {
        //             (None, None) => (),
        //             (None, Some(_)) => self.contents_pos = None,
        //             (Some(_), None) => (),
        //             // write contents to file
        //             (Some(v_f), Some(pos)) => {
        //                 let update =
        //                     FileUpdate::Insert((pos, self.unsaved_buf.as_bytes().to_vec()));

        //                 match v_f.write_bytes(update).await {
        //                     Ok(num_bytes) => {
        //                         self.unsaved_buf.clear();
        //                         Self::show_notification(
        //                             format!("wrote {} bytes to {}", num_bytes, v_f.as_path()),
        //                             Duration::from_secs(1),
        //                             tui,
        //                         );
        //                         // self.contents_pos = None;
        //                     }
        //                     Err(e) => {
        //                         log::error!("error writing to file: {:?}", e);
        //                         tui.event_tx
        //                             .send(crate::ui::tui::AppEvent::Error(e.to_string()))
        //                             .unwrap();
        //                     }
        //                 };
        //             }
        //         }
        //     }
        //     AppState::InContent => {
        //         tui.fs_widget.select(Some(self.filesystem_pos));

        //         match key_event.code {
        //             crossterm::event::KeyCode::Backspace => {
        //                 self.unsaved_buf.pop();
        //             }
        //             crossterm::event::KeyCode::Enter => self.unsaved_buf.push('\n'),
        //             // no navi when in insert mode
        //             crossterm::event::KeyCode::Left
        //             | crossterm::event::KeyCode::Right
        //             | crossterm::event::KeyCode::Up
        //             | crossterm::event::KeyCode::Down => (),
        //             crossterm::event::KeyCode::Delete => {
        //                 self.unsaved_buf.pop();
        //             }
        //             crossterm::event::KeyCode::Char(c) => self.unsaved_buf.push(c),
        //             // no handler for the rest
        //             _ => (),
        //         }
        //     }
        //     _ => unimplemented!("state not handled in this method"),
        // }

        // // perform any state transitions
        // if let Some(ev) = app_ev {
        //     log::debug!("ingesting event: {:?}", ev);
        //     self.state.ingest(ev);
        // }
    }

    /// Show a notification message on the content window for a specified duration,
    /// and then toggle it off.
    fn show_notification<M: ToString>(msg: M, dur: Duration, tui: &Tui) {
        let ev_chan = tui.event_tx.clone();
        let message = msg.to_string();

        tokio::spawn(async move {
            ev_chan
                .send(super::tui::AppEvent::SetContentNotification(Some(
                    message.to_string(),
                )))
                .unwrap();

            tokio::time::sleep(dur).await;

            ev_chan
                .send(super::tui::AppEvent::SetContentNotification(None))
                .unwrap();
        });
    }

    /// Show a highlight on the content widget for a specified duration,
    /// and then toggle it off.
    fn show_highlight(offset: usize, len: usize, dur: Duration, tui: &Tui) {
        let ev_chan = tui.event_tx.clone();

        tokio::spawn(async move {
            ev_chan
                .send(AppEvent::HighlightContent(Some((offset, len))))
                .unwrap();

            tokio::time::sleep(dur).await;

            ev_chan.send(AppEvent::HighlightContent(None))
        });
    }
}

impl AppData {
    pub fn new(ctx: ContextManager) -> Self {
        Self {
            ctx,
            fs_dirs: FixedSizeStack::new(None),
            filesystem_pos: 0,
            v_file_history: Default::default(),
            v_file: None,
            content: None,
            contents_pos: None,
            unsaved_buf: Default::default(),
            err_msg: None,
        }
    }

    /// Top-level state handelr
    pub async fn handle_app_state(
        &mut self,
        app_state: &mut AppState,
        app_ev: KeyEvent,
        tui: &mut Tui,
    ) {
        match app_state {
            AppState::OnContent => {
                tui.fs_widget.focus(false);
                tui.content_widget.focus(true);
                tui.commands_widget.clear();
                tui.commands_widget.add([
                    ("ENTER", "enter insert mode"),
                    ("LEFT", "filesystem tree"),
                    ("arrow keys", "navigate"),
                    ("w", "watch file for changes"),
                ]);
                match app_ev.code {
                    KeyCode::Enter => {
                        *app_state = AppState::InContent(ContentState::default());
                    }
                    KeyCode::Left => {
                        *app_state = AppState::OnFileSystem;
                    }
                    _ => (),
                }
            }
            AppState::InContent(_) => {
                tui.fs_widget.focus(false);
                tui.content_widget.focus(true);
                tui.commands_widget.clear();
                tui.commands_widget
                    .add([("ESC", "exit insert mode and save changes")]);
                self.handle_content_state(app_state, app_ev, tui).await;
            }
            AppState::OnFileSystem => {
                tui.content_widget.focus(false);
                tui.fs_widget.focus(true);
                tui.commands_widget.clear();
                tui.commands_widget.add([
                    ("ESC", "exit"),
                    ("ENTER", "enter filesystem browse"),
                    ("RIGHT", "go to content"),
                ]);

                match app_ev.code {
                    KeyCode::Esc => {
                        tui.event_tx.send(AppEvent::Quit).unwrap();
                    }
                    KeyCode::Enter => {
                        *app_state = AppState::InFileSystem(FsState::default());
                    }
                    KeyCode::Right => {
                        *app_state = AppState::OnContent;
                    }
                    _ => (),
                }
            }
            AppState::InFileSystem(_) => {
                tui.content_widget.focus(false);
                tui.fs_widget.focus(true);
                tui.commands_widget.clear();
                tui.commands_widget.add([
                    ("ESC", "exit filesystem browse"),
                    ("ENTER", "enter file/dir"),
                    ("BACKSPACE", "go to parent dir"),
                    ("UP/DOWN", "navigate"),
                    ("f", "create file"),
                    ("d", "create directory"),
                    ("x", "delete file/dir"),
                ]);
                self.handle_fs_state(app_state, app_ev, tui).await;
            }
        }
    }

    pub async fn handle_fs_state(
        &mut self,
        app_state: &mut AppState,
        app_ev: KeyEvent,
        tui: &mut Tui,
    ) {
        let fs_state = if let AppState::InFileSystem(s) = app_state {
            s
        } else {
            return;
        };

        match fs_state {
            FsState::Navigate => match app_ev.code {
                KeyCode::Esc => {
                    *app_state = AppState::OnFileSystem;
                }
                KeyCode::Enter => {
                    let top_dir_entry = self.fs_dirs.top().cloned();

                    let dir_entry = match &top_dir_entry {
                        Some((dir, read_dir)) => match read_dir.get(self.filesystem_pos) {
                            Some(entry) => entry,
                            None => return,
                        },
                        None => return,
                    };

                    match dir_entry.is_file() {
                        // open file
                        true => {

                            //
                        }
                        // read dir and recurse
                        false => {
                            let path = dir_entry.path.clone();
                            let read_dir = match rfs::fs::read_dir(self.ctx.clone(), &path).await {
                                Ok(rd) => rd,
                                Err(e) => {
                                    log::error!("Read dir error: {:?}", e);
                                    return;
                                }
                            };

                            let entry = (path, read_dir.clone());
                            self.fs_dirs.push(entry);
                            self.filesystem_pos = 0;
                            tui.fs_widget
                                .push(read_dir, dir_entry.path().file_name().unwrap_or_default());
                            tui.fs_widget.select(Some(self.filesystem_pos));
                        }
                    }
                }
                /// Go up one dir (if possible)
                KeyCode::Backspace => {
                    match self.fs_dirs.depth() > 1 {
                        true => {
                            self.fs_dirs.pop();
                            self.filesystem_pos = 0;
                            tui.fs_widget.pop();

                            // TODO: perform a read dir call
                            tui.fs_widget.select(Some(self.filesystem_pos));
                        }
                        false => (),
                    }
                }
                KeyCode::Up => {
                    self.filesystem_pos = self.filesystem_pos.saturating_sub(1);
                    tui.fs_widget.select(Some(self.filesystem_pos));
                }
                KeyCode::Down => {
                    self.filesystem_pos = match self.fs_dirs.top() {
                        Some(dir) => match dir.1.get(self.filesystem_pos + 1) {
                            Some(_) => self.filesystem_pos + 1,
                            None => self.filesystem_pos,
                        },
                        None => 0,
                    };

                    tui.fs_widget.select(Some(self.filesystem_pos));
                }
                KeyCode::Char(FS_CREATE_FILE) => {}
                KeyCode::Char(FS_CREATE_DIR) => {}
                KeyCode::Char(FS_DELETE) => {}

                _ => (),
            },
            FsState::CreateFile(buf) => {
                match app_ev.code {
                    KeyCode::Enter => {
                        if is_valid_fs_path_segment(&buf) {
                            // construct actual path to file
                            let path = match self.fs_dirs.top() {
                                Some((dir, _)) => format!("{}/{}", dir, buf),
                                None => buf.clone(),
                            };

                            match self.v_file_history.get(path.as_str()) {
                                Some(v_file) => {
                                    self.v_file = Some(v_file.clone());
                                }
                                None => {
                                    // create a new file
                                    let v_file =
                                        match VirtFile::create(self.ctx.clone(), &path).await {
                                            Ok(vf) => Arc::new(Mutex::new(vf)),
                                            Err(e) => {
                                                log::error!("virtual file creation error: {:?}", e);
                                                return;
                                            }
                                        };

                                    self.v_file = Some(v_file.clone());
                                    self.v_file_history.insert(path, v_file);
                                }
                            }
                        }

                        // jump into the file
                        *app_state = AppState::InContent(Default::default());
                        // clear dialogue
                        tui.fs_widget.dialogue_box(Option::<&str>::None, false);
                        self.enqueue_render(tui);
                        return;
                    }
                    KeyCode::Backspace => {
                        buf.pop();
                        tui.fs_widget.dialogue_box(Some(&buf), false);
                    }
                    KeyCode::Char(c) => {
                        buf.push(c);
                        tui.fs_widget.dialogue_box(Some(&buf), false);
                    }
                    _ => (),
                }

                // update the dialogue box
                tui.fs_widget
                    .dialogue_box(Some(&buf), is_valid_fs_path_segment(&buf));
            }
            FsState::CreateDir(buf) => {
                match app_ev.code {
                    KeyCode::Esc => {
                        // clear dialogue
                        tui.fs_widget.dialogue_box(Option::<&str>::None, false);
                        // self.enqueue_render(tui);
                        return;
                    }
                    KeyCode::Enter => {
                        if is_valid_fs_path_segment(&buf) {
                            // construct actual path to file
                            let path = match self.fs_dirs.top() {
                                Some((dir, _)) => format!("{}/{}", dir, buf),
                                None => buf.clone(),
                            };

                            match rfs::fs::create_dir(self.ctx.clone(), &path).await {
                                Ok(_) => (),
                                Err(e) => todo!(),
                            }

                            let read_dir = match rfs::fs::read_dir(self.ctx.clone(), &path).await {
                                Ok(rd) => rd,
                                Err(_) => todo!(),
                            };

                            self.fs_dirs.push((path, read_dir));
                        } else {
                            return;
                        }

                        // jump into the file
                        *app_state = AppState::InFileSystem(Default::default());
                        // clear dialogue
                        tui.fs_widget.dialogue_box(Option::<&str>::None, false);
                        // self.enqueue_render(tui);
                        return;
                    }
                    KeyCode::Backspace => {
                        buf.pop();
                        tui.fs_widget.dialogue_box(Some(&buf), false);
                    }
                    KeyCode::Char(c) => {
                        buf.push(c);
                        tui.fs_widget.dialogue_box(Some(&buf), false);
                    }

                    _ => (),
                }

                // update the dialogue box
                tui.fs_widget
                    .dialogue_box(Some(&buf), is_valid_fs_path_segment(&buf));
            }

            _ => todo!(),
        }
    }

    pub async fn handle_content_state(
        &mut self,
        app_state: &mut AppState,
        app_ev: KeyEvent,
        tui: &mut Tui,
    ) {
        let cont_state = if let AppState::InContent(inner) = app_state {
            inner
        } else {
            return;
        };

        match cont_state {
            ContentState::Navigate => {
                match app_ev.code {
                    // navi
                    KeyCode::Up => {
                        tui.content_widget.cursor_up();
                    }
                    KeyCode::Down => {
                        tui.content_widget.cursor_down();
                    }
                    KeyCode::Left => {
                        tui.content_widget.cursor_left();
                    }
                    KeyCode::Right => {
                        tui.content_widget.cursor_right();
                    }
                    KeyCode::Enter => {
                        // toggle insert mode
                        *cont_state = ContentState::Insert;

                        self.unsaved_buf.clear();
                    }

                    _ => (),
                }

                self.contents_pos = tui.content_widget.cursor_offset();
            }
            ContentState::Insert => {
                match app_ev.code {
                    // every escape updates the file
                    KeyCode::Esc => {
                        let v_file = match &self.v_file {
                            Some(vf) => vf.clone(),
                            None => return,
                        };

                        match self.contents_pos {
                            Some(offset) => {
                                let mut lock = v_file.lock().await;

                                let update = FileUpdate::Insert((
                                    offset,
                                    self.unsaved_buf.as_bytes().to_vec(),
                                ));

                                // TODO: handle err here
                                lock.write_bytes(update).await.unwrap();
                            }
                            None => (),
                        }

                        // toggle insert mode
                        *cont_state = ContentState::Navigate;
                        // self.enqueue_render(tui);
                    }

                    KeyCode::Char(c) => {
                        // insert char
                        self.unsaved_buf.push(c);
                        tui.content_widget.set_contents(Some(&self.unsaved_buf));
                    }

                    KeyCode::Enter => {
                        // insert newline
                        self.unsaved_buf.push('\n');
                        tui.content_widget.set_contents(Some(&self.unsaved_buf));
                    }

                    KeyCode::Backspace => {
                        self.unsaved_buf.pop();
                        tui.content_widget.set_contents(Some(&self.unsaved_buf));
                    }

                    _ => (),
                }
            }
            // spawns a watch channel
            ContentState::Watch => {
                //a ad

                let v_f = match &self.v_file {
                    Some(vf) => vf.clone(),
                    None => return,
                };

                let ev_tx = tui.event_tx.clone();

                tokio::spawn(async move {
                    let mut update_channel = match v_f.lock().await.watch_chan().await {
                        Ok(ch) => ch,
                        Err(_) => return,
                    };

                    match update_channel.recv().await {
                        Some(Ok((path, update_data))) => {
                            // update the content widget
                            ev_tx
                                .send(AppEvent::FileUpdate {
                                    path,
                                    upd: update_data,
                                })
                                .unwrap();
                        }
                        _ => return,
                    };
                });
            }

            _ => unimplemented!(),
        }
    }

    /// Enqueue a render event to the event channel
    pub fn enqueue_render(&self, tui: &Tui) {
        tui.event_tx.send(super::tui::AppEvent::Render).unwrap();
    }
}

#[async_trait]
impl HandleStateEvent for AppState {
    async fn handle_event(&mut self, event: KeyEvent, app_data: &mut AppData, tui: &mut Tui) {
        match self {
            AppState::OnContent => todo!(),
            AppState::InContent(inner) => inner.handle_event(event, app_data, tui).await,
            AppState::OnFileSystem => todo!(),
            AppState::InFileSystem(inner) => inner.handle_event(event, app_data, tui).await,
        }
    }
}

#[async_trait]
impl HandleStateEvent for ContentState {
    async fn handle_event(&mut self, event: KeyEvent, app_data: &mut AppData, tui: &mut Tui) {
        match self {
            _ => unimplemented!(),
        }
    }
}

#[async_trait]
impl HandleStateEvent for FsState {
    async fn handle_event(&mut self, event: KeyEvent, app_data: &mut AppData, tui: &mut Tui) {
        match self {
            _ => unimplemented!(),
        }
    }
}

// impl HandleStateEvent for AppData {

//     async fn handle_event(&mut self, state: &mut Self::State, event: KeyEvent, tui: &mut Tui) {
//         match state {
//             AppState::OnContent => todo!(),
//             AppState::InContent(inner) => todo!(),
//             AppState::OnFileSystem => todo!(),
//             AppState::InFileSystem(inner) => todo!(),
//         }
//     }
// }

impl<T> FixedSizeStack<T> {
    pub fn new(size: Option<usize>) -> Self {
        Self {
            size,
            stack: Vec::new(),
        }
    }

    /// Push an item onto the stack
    ///
    /// If the stack is full, the oldest item is removed
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

    /// Pop the top element from the stack
    pub fn pop(&mut self) -> Option<T> {
        self.stack.pop()
    }

    /// Get the top element of the stack
    pub fn top(&self) -> Option<&T> {
        self.stack.last()
    }

    /// Get the current depth of the stack
    /// (same as the number of elements in the stack)
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

/// Checks if a string is a valid path segment (filename or directory name)
fn is_valid_fs_path_segment(s: &str) -> bool {
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
    // && match s.chars().next() {
    //     Some(c) => !c.is_ascii_digit(),
    //     None => true,
    // }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_validate_filesystem_string() {
        assert!(is_valid_fs_path_segment("valid_string.txt"));
        assert!(is_valid_fs_path_segment("0valid_string1"));

        assert!(!is_valid_fs_path_segment("invalid_string%.asd"));
        assert!(!is_valid_fs_path_segment("invalid_string>.asd"));
        assert!(!is_valid_fs_path_segment("invalid_string<.asd"));
        assert!(!is_valid_fs_path_segment("invalid_string|.asd"));
        assert!(!is_valid_fs_path_segment("invalid_string*.asd"));
        assert!(!is_valid_fs_path_segment("invalid_string?.asd"));
        assert!(!is_valid_fs_path_segment("invalid_string:.asd"));
        assert!(!is_valid_fs_path_segment("invalid_string\".asd"));
        assert!(!is_valid_fs_path_segment("invalid_string\\.asd"));
    }
}
