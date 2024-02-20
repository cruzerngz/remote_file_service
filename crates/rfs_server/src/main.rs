#![allow(unused)]

use std::{net::SocketAddrV4, str::FromStr};

use rfs_core::middleware::{Dispatcher, RequestServer};

use crate::server::RfsServer;

mod server;

#[tokio::main]
async fn main() {
    let server = RfsServer {
        home: Default::default(),
    };

    let mut dispatcher = Dispatcher::from_handler(server);

    dispatcher.dispatch().await;

    return;
}
