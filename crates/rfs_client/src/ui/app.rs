//! App module. Contains application state

use std::{collections::HashMap, default, io};

use rfs::fsm::TransitableState;
use rfs::{fs::VirtReadDir, middleware::ContextManager, state_transitions};

use super::tui::Tui;

/// Application state
#[derive(Debug)]
pub struct App {
    exit: bool,

    ctx: ContextManager,

    // open filesystem directories
    fs_dirs: HashMap<String, VirtReadDir>,

    // current selection in the filsystem
    filesystem_pos: (String, usize),

    content: Option<String>,

    contents_pos: Option<usize>,

    /// App state
    state: AppState,
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

    Error,

    OnFileSystem,
}

/// Keyboard/other events
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

    // error ack
    Error + EnterKey => OnFileSystem;
}

impl App {
    pub fn new(ctx: ContextManager, tick_rate: f64, frame_rate: f64) -> Self {
        Self {
            exit: false,
            ctx,
            fs_dirs: todo!(),
            filesystem_pos: todo!(),
            content: None,
            contents_pos: None,
            state: Default::default(),
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

                    // render the result
                    tui.event_tx.send(super::tui::AppEvent::Render).unwrap();
                }
                super::tui::AppEvent::Quit => {
                    tui.stop();
                    tui.exit()?;
                    break;
                }
                super::tui::AppEvent::Error => todo!(),
                super::tui::AppEvent::Closed => todo!(),
                super::tui::AppEvent::Tick
                | super::tui::AppEvent::Render
                | super::tui::AppEvent::Resize(_, _) => tui.draw_to_screen().await?,
                super::tui::AppEvent::FocusGained => todo!(),
                super::tui::AppEvent::FocusLost => todo!(),
                super::tui::AppEvent::Paste(_) => todo!(),
                super::tui::AppEvent::Key(_) => todo!(),
                super::tui::AppEvent::Mouse(_) => todo!(),
            }
        }

        Ok(())
    }

    /// Initialize the app by performing some initial queries to the remote
    pub fn init(&mut self) {}
}
