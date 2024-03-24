mod args;
mod data_collection;
mod test;
mod ui;

use std::{io, net::SocketAddrV4, sync::Arc};

use args::ClientArgs;
use clap::Parser;
use rfs::middleware::*;

#[tokio::main]
async fn main() -> io::Result<()> {
    match std::env::var("RUST_LOG") {
        Ok(_) => (),
        Err(_) => std::env::set_var("RUST_LOG", "DEBUG"),
    }

    pretty_env_logger::formatted_timed_builder()
        .parse_filters(&std::env::var("RUST_LOG").expect("RUST_LOG environment variable not set"))
        .init();

    let args = ClientArgs::parse();
    let manager = match (args.invocation_semantics, args.simulate_ommisions) {
        (args::InvocationSemantics::Maybe, true) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(FaultyDefaultProto::from_frac(
                    rfs::defaults::DEFAULT_FAILURE_RATE,
                )),
            )
            .await?
        }
        (args::InvocationSemantics::Maybe, false) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(DefaultProto),
            )
            .await?
        }
        (args::InvocationSemantics::AtLeastOnce, true) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(FaultyRequestAckProto::from_frac(
                    rfs::defaults::DEFAULT_FAILURE_RATE,
                )),
            )
            .await?
        }
        (args::InvocationSemantics::AtLeastOnce, false) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(RequestAckProto),
            )
            .await?
        }
        (args::InvocationSemantics::AtMostOnce, true) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(FaultyHandshakeProto::from_frac(
                    rfs::defaults::DEFAULT_FAILURE_RATE,
                )),
            )
            .await?
        }
        (args::InvocationSemantics::AtMostOnce, false) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(HandshakeProto),
            )
            .await?
        }
    };

    match args.test {
        true => {
            test::test_mode(manager).await?;
            return Ok(());
        }
        false => {
            let mut app = ui::App::new(manager, 60.0, 4.0);
            app.run().await?;
        }
    }

    return Ok(());
}
