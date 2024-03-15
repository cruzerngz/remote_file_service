#![allow(unused)]

mod args;
mod server;

use std::{
    net::{Ipv4Addr, SocketAddrV4},
    str::FromStr,
};

use clap::Parser;
use futures::FutureExt;
use rfs::middleware::{DefaultProto, Dispatcher, HandshakeProto, TransmissionProtocol};

use crate::{args::ServerArgs, server::RfsServer};

#[tokio::main]
async fn main() {
    match std::env::var("RUST_LOG") {
        Ok(_) => (),
        Err(_) => std::env::set_var("RUST_LOG", "DEBUG"),
    }

    pretty_env_logger::formatted_timed_builder()
        .parse_filters(&std::env::var("RUST_LOG").expect("RUST_LOG environment variable not set"))
        .init();

    let args = ServerArgs::parse();
    let server = RfsServer::from_path(args.directory);
    let addr = SocketAddrV4::new(args.address, args.port);

    log::info!("server listening on {}", addr);

    let mut dispatcher = Dispatcher::new(
        addr,
        server,
        HandshakeProto {},
        args.sequential,
        args.request_timeout.into(),
        rfs::defaults::DEFAULT_RETRIES,
        !args.allow_duplicates,
    )
    .await;

    let x: bool;

    // let mut d: Box<Dispatcher<RfsServer, dyn TransmissionProtocol>> = match x {
    //     true => Box::new(Dispatcher::new(
    //         addr,
    //         server,
    //         HandshakeProto {},
    //         args.sequential,
    //         args.request_timeout.into(),
    //         rfs::defaults::DEFAULT_RETRIES,
    //     ).await),
    //     false => Box::new(Dispatcher::new(
    //         addr,
    //         server,
    //         DefaultProto,
    //         args.sequential,
    //         args.request_timeout.into(),
    //         rfs::defaults::DEFAULT_RETRIES,
    //     ).await),
    // };

    dispatcher.dispatch().await;

    // max_udp_tx_rx().await;

    return;
}

async fn max_udp_tx_rx() {
    let data = [1_u8; 100_000];
    let source = tokio::net::UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let sink = tokio::net::UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();

    source.connect(sink.local_addr().unwrap()).await.unwrap();

    source.send(&data).await.unwrap();

    let mut buf = [0_u8; 110_000];

    let (size, addr) = sink.recv_from(&mut buf).await.unwrap();

    println!("sent {} bytes", data.len());
    println!("recv {} bytes", size);
}
