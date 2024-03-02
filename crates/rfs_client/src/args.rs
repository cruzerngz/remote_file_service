//! Command-line args for client

use std::net::Ipv4Addr;

use clap::Parser;

#[derive(Parser)]
pub struct ClientArgs {
    /// The IPv4 address of the client.
    #[clap(short, long)]
    #[clap(default_value_t = Ipv4Addr::LOCALHOST)]
    pub listen_address: Ipv4Addr,

    /// The IPv4 address of the server.
    #[clap(short, long)]
    #[clap(default_value_t = Ipv4Addr::LOCALHOST)]
    pub target: Ipv4Addr,

    /// The server port to connect to.
    #[clap(short, long)]
    #[clap(default_value_t = rfs::defaults::DEFAULT_PORT)]
    pub port: u16,

    /// The timeout duration
    #[clap(short, long)]
    #[clap(default_value = rfs::defaults::DEFAULT_TIMEOUT)]
    pub request_timeout: humantime::Duration,

    /// The number of retries before returning an error
    #[clap(short, long)]
    #[clap(default_value_t = rfs::defaults::DEFAULT_RETRIES)]
    pub num_retries: u8,
}
