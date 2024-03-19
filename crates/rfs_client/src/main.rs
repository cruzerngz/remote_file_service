mod args;
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
use futures::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use ratatui::{backend::CrosstermBackend, style::Stylize, widgets, Terminal};
use rfs::{
    fs::VirtFile,
    interfaces::{PrimitiveFsOpsClient, SimpleOpsClient},
    middleware::*,
};

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
    let mut manager = ContextManager::new(
        args.listen_address,
        SocketAddrV4::new(args.target, args.port),
        args.request_timeout.into(),
        args.num_retries,
        Arc::new(HandshakeProto {}),
    )
    .await?;

    log::info!("starting remote invocations");
    let _ = SimpleOpsClient::say_hello(&mut manager, "new configurtation".to_string())
        .await
        .unwrap();

    let mut file = VirtFile::open(manager.clone(), "remote_file.txt").await?;

    println!("file opened: {:#?}", file);

    // let contents = rfs::fs::read_to_string(manager, "red_chips_v3.json").await?;

    // println!("file contents: {}", contents);
    // // file.write_all(b"hello world").await?;
    let mut contents = String::new();
    let data = file.read_bytes().await?;

    println!("bytes read: {:?}", std::str::from_utf8(&data));

    let _ = file
        .write_bytes(
            "hello world hello world\n".as_bytes(),
            rfs::interfaces::FileWriteMode::Insert(3),
        )
        .await?;

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
