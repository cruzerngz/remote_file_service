//! Callback definitions are found here.
#![allow(unused)]

use std::net::Ipv4Addr;

use serde::Serialize;

/// Callback type related to filesystem stuffs
#[derive(Debug)]
pub struct FileSystemCallback {}

/// A type representing a pending callback on the remote.
///
/// This callback
#[derive(Debug)]
pub struct RegisteredCallback<T: Serialize> {
    /// The return address that the client is awaiting at.
    return_address: Ipv4Addr,
    /// The payload to return to the client.
    return_payload: T,
}
