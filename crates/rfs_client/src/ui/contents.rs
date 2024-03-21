//! This is a bad module name

use ratatui::{style::Stylize, text::Span};
use rfs::fs::VirtReadDir;

/// Render the contents of a type to the terminal
pub trait ToTerminal {
    type Context;

    fn to_terminal(&self, context: Self::Context) -> String;
}

impl ToTerminal for VirtReadDir {
    type Context = ();

    fn to_terminal(&self, context: Self::Context) -> String {
        let lines = self.iter().map(|entry| match entry.is_file() {
            true => entry
                .path()
                .to_str()
                .expect("invalid path")
                .to_owned()
                .white(),
            false => entry
                .path()
                .to_str()
                .expect("invalid path")
                .to_owned()
                .bold()
                .green(),
        });

        todo!()
    }
}
