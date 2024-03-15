//! CLI args

use std::{
    net::Ipv4Addr,
    path::{Path, PathBuf},
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
    #[clap(long)]
    #[clap(default_value_t = true)]
    pub allow_duplicates: bool,
}
