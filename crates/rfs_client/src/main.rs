mod args;
mod test;
mod ui;

use std::{
    io::{self, Write},
    net::SocketAddrV4,
    sync::Arc,
};

use args::ClientArgs;
use clap::Parser;
// use clap::Parser;
use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    style::Stylize,
    widgets::{self, canvas::Context},
    Terminal,
};
use rfs::middleware::*;

#[tokio::main]
async fn main() -> io::Result<()> {
    match std::env::var("RUST_LOG") {
        Ok(_) => (),
        Err(_) => std::env::set_var("RUST_LOG", "DEBUG"),
    }

    pretty_env_logger::formatted_timed_builder()
        .parse_filters(&std::env::var("RUST_LOG").expect("RUST_LOG environment variable not set"))
        .init();

    let args = ClientArgs::parse();
    let mut manager = match (args.invocation_semantics, args.simulate_ommisions) {
        (args::InvocationSemantics::Maybe, true) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(DefaultProto),
            )
            .await?
        }
        (args::InvocationSemantics::Maybe, false) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(DefaultProto),
            )
            .await?
        }
        (args::InvocationSemantics::AtLeastOnce, true) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(FaultyRequestAckProto::<10>),
            )
            .await?
        }
        (args::InvocationSemantics::AtLeastOnce, false) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(RequestAckProto),
            )
            .await?
        }
        (args::InvocationSemantics::AtMostOnce, true) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(HandshakeProto {}),
            )
            .await?
        }
        (args::InvocationSemantics::AtMostOnce, false) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(HandshakeProto {}),
            )
            .await?
        }
    };

    match args.test {
        true => {
            test::test_mode(manager).await?;
            return Ok(());
        }
        false => (),
    }

    // println!("file contents: {}", contents);

    // let res = PrimitiveFsOpsClient::read_bytes(&mut manager, "remote_file.txt".to_string()).await?;

    // println!("length of contents: {:?}", res.len());

    // let mut buf = [0_u8; 1000];
    // file.poll_read(&mut buf).await?;

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
