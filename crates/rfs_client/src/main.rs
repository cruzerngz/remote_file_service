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
        (args::InvocationSemantics::Maybe, Some(frac)) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(FaultyDefaultProto::from_frac(frac)),
            )
            .await?
        }
        (args::InvocationSemantics::Maybe, None) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(DefaultProto),
            )
            .await?
        }
        (args::InvocationSemantics::AtLeastOnce, Some(frac)) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(FaultyRequestAckProto::from_frac(frac)),
            )
            .await?
        }
        (args::InvocationSemantics::AtLeastOnce, None) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(RequestAckProto),
            )
            .await?
        }
        (args::InvocationSemantics::AtMostOnce, Some(frac)) => {
            ContextManager::new(
                args.listen_address,
                SocketAddrV4::new(args.target, args.port),
                args.request_timeout.into(),
                args.num_retries,
                Arc::new(FaultyHandshakeProto::from_frac(frac)),
            )
            .await?
        }
        (args::InvocationSemantics::AtMostOnce, None) => {
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
            // test::test_mode(manager).await?;

            let inv_prob = match args.simulate_ommisions {
                Some(frac) => frac,
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "test mode requires specifying simulated ommisions",
                    ))
                }
            };

            let _ = data_collection::test(
                args.invocation_semantics.clone(),
                inv_prob,
                args.listen_address,
                args.target,
                args.port,
                args.request_timeout.into(),
                args.num_retries,
            )
            .await?;

            return Ok(());
        }
        false => {
            let mut app = ui::App::new(manager, 60.0, 4.0);
            app.run().await?;
        }
    }

    return Ok(());
}
