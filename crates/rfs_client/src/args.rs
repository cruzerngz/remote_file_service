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

    /// Whether to simulate a faulty network
    #[clap(long)]
    pub simulate_ommisions: bool,
}

#[derive(Clone, Debug, clap::ValueEnum)]
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
