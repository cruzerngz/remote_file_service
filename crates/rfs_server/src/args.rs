//! CLI args

use std::{
    net::Ipv4Addr,
    path::{Path, PathBuf},
};

use clap::Parser;

/// Remote file service init arguments
#[derive(Parser)]
pub(crate) struct ServerArgs {
    /// The IPv4 address for the server to bind to.
    #[clap(default_value_t = Ipv4Addr::LOCALHOST)]
    pub address: Ipv4Addr,

    /// The port number for the server to listen on.
    #[clap(default_value_t = rfs::defaults::DEFAULT_PORT)]
    pub port: u16,

    /// The starting directory the server will attach itself to.
    #[clap(default_value = PathBuf::from(std::env::current_dir().unwrap()).into_os_string())]
    pub path: PathBuf,
}
