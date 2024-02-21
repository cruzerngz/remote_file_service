#![allow(unused)]

use std::{
    net::{Ipv4Addr, SocketAddrV4},
    str::FromStr,
};

use rfs_core::middleware::{Dispatcher, RequestServer};

use crate::server::RfsServer;

mod server;

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "DEBUG");
    pretty_env_logger::init();

    let server = RfsServer::default();

    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3333);

    log::info!("server listening on {}", addr);

    let mut dispatcher = Dispatcher::new(addr, server);

    dispatcher.dispatch().await;

    return;
}
