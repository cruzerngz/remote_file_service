mod args;

use std::net::{Ipv4Addr, SocketAddrV4};

use clap::Parser;
use futures::{AsyncReadExt, AsyncWriteExt};
// use rfs_core::middleware::ContextManager;
use rfs::{interfaces::*, middleware::ContextManager};

use crate::args::ClientArgs;

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "DEBUG");
    pretty_env_logger::init();

    let args = ClientArgs::parse();

    let manager = ContextManager::new(
        Ipv4Addr::LOCALHOST,
        SocketAddrV4::new(args.target, args.port),
    )
    .expect("failed to initialize context manager");

    // let res = SimpleOpsClient::compute_fib(&manager, 10).await;
    // log::info!("{:?}", res);

    // log::debug!("creating file on the remote");
    // let mut remote_file = rfs::fs::VirtFile::create(manager, "remote_file.txt")
    //     .await
    //     .expect("file creation error");
    // remote_file
    // .write("hello world asdlkmasldkmalskd\n".as_bytes())
    // .await
    // .expect("failed to write to file");

    // let contents = rfs::fs::read_to_string(manager, "remote_file.txt")
    //     .await
    //     .expect("failed to read file");
    // println!("contents: {}", contents);

    PrimitiveFsOpsClient::write_append_bytes(
        &manager,
        "remote_file.txt".to_owned(),
        "new line\n".as_bytes().to_vec(),
    )
    .await
    .unwrap();

    // let req = PrimitiveFsOpsClient::Request {
    //     path: "remote_file.txt".to_string(),
    //     bytes: "new line".as_bytes().to_vec(),
    // };

    return;
}
