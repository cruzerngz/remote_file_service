//! App module. Contains application state

use std::{collections::HashMap, io};

use rfs::{fs::VirtReadDir, middleware::ContextManager};

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
}

impl App {
    pub fn new(ctx: ContextManager, tick_rate: f64, frame_rate: f64) -> Self {
        Self {
            exit: false,
            ctx,
            fs_dirs: todo!(),
            filesystem_pos: todo!(),
        }
    }

    /// This is the main application loop.
    /// A [Tui] is instantiated here and used to render the UI.
    pub async fn run(&mut self) -> io::Result<()> {
        let mut tui = Tui::new(60.0, 4.0)?;

        tui.start();

        while let Some(event) = tui.next().await {
            match event {
                super::tui::AppEvent::Init => todo!(),
                super::tui::AppEvent::Quit => break,
                super::tui::AppEvent::Error => todo!(),
                super::tui::AppEvent::Closed => todo!(),
                super::tui::AppEvent::Tick => todo!(),
                super::tui::AppEvent::Render => todo!(),
                super::tui::AppEvent::FocusGained => todo!(),
                super::tui::AppEvent::FocusLost => todo!(),
                super::tui::AppEvent::Paste(_) => todo!(),
                super::tui::AppEvent::Key(_) => todo!(),
                super::tui::AppEvent::Mouse(_) => todo!(),
                super::tui::AppEvent::Resize(_, _) => todo!(),
                // _ => todo!(),
            }
        }

        Ok(())
    }
}
