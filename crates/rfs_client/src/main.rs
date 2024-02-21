use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
};

use rfs_core::middleware::ContextManager;
use rfs_methods::*;

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "DEBUG");
    pretty_env_logger::init();

    let manager = ContextManager::new(
        Ipv4Addr::LOCALHOST,
        SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3333),
    )
    .expect("failed to initialize context manager");

    // let res = SimpleOpsClient::compute_fib(&manager, 10).await;
    // log::info!("{:?}", res);

    log::debug!("creating file on the remote");
    let f = rfs_methods::fs::VirtFile::create(manager, "remote_file.txt")
        .await
        .expect("file creation error");

    println!("Hello, world!");
}
