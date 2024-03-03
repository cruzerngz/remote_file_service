mod args;
mod ui;

use std::{
    io::{self, Write},
    net::SocketAddrV4,
};

use args::ClientArgs;
use clap::Parser;
// use clap::Parser;
use crossterm::event::{self, KeyCode, KeyEventKind};
// use futures::{AsyncReadExt, AsyncWriteExt};
use ratatui::{backend::CrosstermBackend, style::Stylize, widgets, Terminal};
use rfs::{interfaces::SimpleOpsClient, middleware::*};

#[tokio::main]
async fn main() -> io::Result<()> {
    pretty_env_logger::formatted_timed_builder()
        .parse_filters("DEBUG")
        .init();

    let args = ClientArgs::parse();
    let manager = ContextManager::new(
        args.listen_address,
        SocketAddrV4::new(args.target, args.port),
        args.request_timeout.into(),
        args.num_retries,
        SimpleProto,
    )
    .await
    .unwrap();

    let _ = SimpleOpsClient::say_hello(&manager, "new configurtation".to_string())
        .await
        .unwrap();

    // let mut term = ui::init()?;

    // let mut app = ui::App::default();
    // app.run(&mut term).await?;

    // ui::restore()?;

    return Ok(());
}

#[allow(unused)]
async fn render_loop<W: Write>(mut term: Terminal<CrosstermBackend<W>>) -> io::Result<()> {
    loop {
        term.draw(|frame| {
            let area = frame.size();

            frame.render_widget(
                widgets::Paragraph::new("Hello world from ratatui!").white(),
                area,
            )
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press && k.code == KeyCode::Char('q') {
                    break Ok(());
                }
            }
        }
    }
}
