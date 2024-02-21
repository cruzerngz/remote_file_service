use std::net::{Ipv4Addr, SocketAddrV4};

use rfs_core::{middleware::ContextManager, RemotelyInvocable};
use rfs_methods::{self, SimpleOpsComputeFib, SimpleOpsSayHello};

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "DEBUG");
    pretty_env_logger::init();

    let manager = ContextManager::new(
        Ipv4Addr::LOCALHOST,
        SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3333),
    )
    .expect("failed to initialize context manager");

    // let payload = SimpleOpsSayHello::Request {
    //     content: "World".to_string(),
    // };

    // let res = manager.invoke(payload).await.unwrap();

    // println!("{:?}", res);

    let res = rfs_methods::SimpleOpsClient::compute_fib(manager, 10).await;
    log::info!("{:?}", res);

    // let payload = SimpleOpsComputeFib::Request { fib_num: 10 };

    // // log::debug!("outgoing payload: {:?}", payload.invoke_bytes());

    // let res = manager.invoke(payload).await.unwrap();

    // println!("{:?}", res);

    // let res = rfs_methods::SimpleOpsClient::compute_fib(manager, 10).await;
    // println!("{:?}", res);

    println!("Hello, world!");
}
