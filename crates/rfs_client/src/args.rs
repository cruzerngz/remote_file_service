//! Command-line args for client

use std::{net::Ipv4Addr, path::PathBuf};

use clap::Parser;

#[derive(Parser)]
pub struct ClientArgs {

    /// The IPv4 address of the client.
    #[clap(default_value_t = Ipv4Addr::LOCALHOST)]
    pub listen_address: Ipv4Addr,

    /// The IPv4 address of the server.
    #[clap(default_value_t = Ipv4Addr::LOCALHOST)]
    pub target: Ipv4Addr,

    /// The server port to connect to.
    #[clap(default_value_t = rfs::defaults::DEFAULT_PORT)]
    pub port: u16,
}
