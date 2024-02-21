use std::net::{Ipv4Addr, SocketAddrV4};

use futures::AsyncWriteExt;
use rfs_core::middleware::ContextManager;
use rfs::*;

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
    let mut remote_file = rfs::fs::VirtFile::create(manager, "remote_file.txt")
        .await
        .expect("file creation error");

    remote_file
        .write("hello world asdlkmasldkmalskd\n".as_bytes())
        .await
        .expect("failed to write to file");

    println!("Hello, world!");
}
