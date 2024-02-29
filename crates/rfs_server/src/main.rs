#![allow(unused)]

mod args;
mod server;

use std::{
    net::{Ipv4Addr, SocketAddrV4},
    str::FromStr,
};

// use rfs_core::middleware::{Dispatcher, RequestServer};

use clap::Parser;
use rfs::middleware::Dispatcher;

use crate::{args::ServerArgs, server::RfsServer};

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "DEBUG");
    pretty_env_logger::formatted_timed_builder()
    .parse_filters(&std::env::var("RUST_LOG").unwrap_or_default())
    .init();

    let args = ServerArgs::parse();
    let server = RfsServer::from_path(args.path);
    let addr = SocketAddrV4::new(args.address, args.port);

    log::info!("server listening on {}", addr);

    let mut dispatcher = Dispatcher::new(addr, server);

    dispatcher.dispatch().await;

    return;
}
