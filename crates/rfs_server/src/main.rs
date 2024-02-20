use std::{net::SocketAddrV4, str::FromStr};

use rfs_core::middleware::RequestServer;

use crate::server::RfsServer;

mod server;

#[tokio::main]
async fn main() {
    let mut server = RfsServer {
        home: Default::default(),
    };

    // server
    //     .serve(SocketAddrV4::from_str("127.0.0.1:2222").unwrap())
    //     .await;

    println!("Hello, world!")
}
