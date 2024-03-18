//! CLI args

use std::{
    fmt::Display, net::Ipv4Addr, path::{Path, PathBuf}
};

use clap::Parser;

/// Remote file service server arguments
#[derive(Parser)]
pub(crate) struct ServerArgs {
    /// The IPv4 address for the server to bind to.
    #[clap(short, long)]
    #[clap(default_value_t = Ipv4Addr::LOCALHOST)]
    pub address: Ipv4Addr,

    /// The port number for the server to listen on.
    #[clap(short, long)]
    #[clap(default_value_t = rfs::defaults::DEFAULT_PORT)]
    pub port: u16,

    /// The starting directory the server will attach itself to.
    #[clap(short, long)]
    #[clap(default_value = PathBuf::from(std::env::current_dir().unwrap()).into_os_string())]
    pub directory: PathBuf,

    /// The timeout duration
    #[clap(short, long)]
    #[clap(default_value = rfs::defaults::DEFAULT_TIMEOUT)]
    pub request_timeout: humantime::Duration,

    /// Process requests sequentially instead of in parallel.
    #[clap(long)]
    pub sequential: bool,

    /// Do not filter duplicate requests
    /// TODO: remove this option, the 2 fields below will handle this case
    #[clap(long)]
    #[clap(default_value_t = true)]
    pub allow_duplicates: bool,

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

