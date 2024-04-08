//! Command-line args for client

use std::{fmt::Display, net::Ipv4Addr};

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

    /// Invocation semantics (transmission protocol) to use
    #[clap(long)]
    #[clap(default_value_t = InvocationSemantics::AtMostOnce)]
    pub invocation_semantics: InvocationSemantics,

    /// Whether to simulate a faulty network.
    ///
    /// The client will simulate a transmission failure every 1 in N attempts.
    #[clap(long, value_name = "N")]
    pub simulate_ommisions: Option<u32>,

    /// Client local cache lifetime
    #[clap(long)]
    #[clap(default_value = "1m")]
    pub freshness_interval: humantime::Duration,

    /// Start the client in test mode.
    /// This mode checks for general runtime stability and
    /// the reliability of each transmission protocol.
    ///
    /// This mode sends multiple dummy requests and responses from the remote,
    /// and returns failure statistics.
    #[clap(long)]
    pub test: bool,

    /// Send logs to a log file.
    #[clap(long)]
    pub log_to_file: bool,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum InvocationSemantics {
    /// A request is sent only once, and the receipt is not guaranteed.
    Maybe,

    /// A request is sent until a response is received.
    AtLeastOnce,

    /// Duplicate requests will be processed at most once.
    AtMostOnce,
}

impl Display for InvocationSemantics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", camel_to_snake_case(&format!("{:?}", self)))
    }
}

pub fn camel_to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i != 0 {
                result.push('-');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}
