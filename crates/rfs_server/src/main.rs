#![allow(unused)]

mod args;
mod server;

use std::{
    net::{Ipv4Addr, SocketAddrV4},
    str::FromStr,
    sync::Arc,
};

use clap::Parser;
use futures::{lock::Mutex, FutureExt};
use rfs::middleware::{
    DefaultProto, Dispatcher, FaultyDefaultProto, FaultyHandshakeProto, FaultyRequestAckProto,
    HandshakeProto, RequestAckProto, TransmissionProtocol,
};

use crate::{
    args::ServerArgs,
    server::{RegisteredFileUpdates, RfsServer, FILE_UPDATE_CALLBACKS},
};

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

    let mut dispatcher: Dispatcher<RfsServer> =
        match (args.invocation_semantics, args.simulate_ommisions) {
            (args::InvocationSemantics::Maybe, Some(frac)) => {
                Dispatcher::new(
                    addr,
                    server,
                    Arc::new(FaultyDefaultProto::from_frac(frac)),
                    args.sequential,
                    args.request_timeout.into(),
                    rfs::defaults::DEFAULT_RETRIES,
                    false,
                )
                .await
            }
            (args::InvocationSemantics::Maybe, None) => {
                Dispatcher::new(
                    addr,
                    server,
                    Arc::new(DefaultProto),
                    args.sequential,
                    args.request_timeout.into(),
                    rfs::defaults::DEFAULT_RETRIES,
                    false,
                )
                .await
            }
            (args::InvocationSemantics::AtLeastOnce, Some(frac)) => {
                Dispatcher::new(
                    addr,
                    server,
                    Arc::new(FaultyRequestAckProto::from_frac(frac)),
                    args.sequential,
                    args.request_timeout.into(),
                    rfs::defaults::DEFAULT_RETRIES,
                    false,
                )
                .await
            }
            (args::InvocationSemantics::AtLeastOnce, None) => {
                Dispatcher::new(
                    addr,
                    server,
                    Arc::new(RequestAckProto),
                    args.sequential,
                    args.request_timeout.into(),
                    rfs::defaults::DEFAULT_RETRIES,
                    false,
                )
                .await
            }
            (args::InvocationSemantics::AtMostOnce, Some(frac)) => {
                Dispatcher::new(
                    addr,
                    server,
                    Arc::new(FaultyHandshakeProto::from_frac(frac)),
                    args.sequential,
                    args.request_timeout.into(),
                    rfs::defaults::DEFAULT_RETRIES,
                    true,
                )
                .await
            }
            (args::InvocationSemantics::AtMostOnce, None) => {
                Dispatcher::new(
                    addr,
                    server,
                    Arc::new(HandshakeProto),
                    args.sequential,
                    args.request_timeout.into(),
                    rfs::defaults::DEFAULT_RETRIES,
                    true,
                )
                .await
            }
        };

    // initialize callback stuffs
    FILE_UPDATE_CALLBACKS.get_or_init(|| {
        Arc::new(Mutex::new(RegisteredFileUpdates {
            bind_addr: args.address,
            lookup: Default::default(),
            proto: dispatcher.protocol.clone(),
            timeout: args.request_timeout.into(),
            retries: rfs::defaults::DEFAULT_RETRIES,
        }))
    });

    dispatcher.dispatch().await;

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
