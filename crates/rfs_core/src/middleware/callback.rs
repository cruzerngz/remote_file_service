#![allow(unused)]

use std::net::SocketAddrV4;

use serde::Serialize;
use tokio::net::UdpSocket;

use crate::middleware::MiddlewareData;

/// A type representing a pending callback on the remote.
///
/// This callback is triggered by some condition on the remote.
/// The callback data, will then be sent back to the callee.
#[derive(Debug)]
pub struct RemoteCallback<T: Serialize> {
    /// The return address that the client is awaiting at.
    return_address: SocketAddrV4,
    /// The payload to return to the client.
    return_payload: Option<T>,
}

impl<T: Serialize> RemoteCallback<T> {
    /// Create a new instance of `self`
    pub fn new(return_address: SocketAddrV4) -> Self {
        Self {
            return_address,
            return_payload: None,
        }
    }

    /// Loads the callback with some data
    pub fn load(&mut self, data: T) {
        self.return_payload = Some(data);
    }

    /// Returns the callback data to the callee.
    ///
    /// This method consumes `self`.
    pub async fn send(self, socket: &UdpSocket) -> bool {
        if let None = self.return_payload {
            return false;
        }

        let value = match &self.return_payload {
            Some(value) => value,
            None => return false,
        };

        // serialize and wrap data into middleware payload
        let serialized = super::serialize_primary(value).unwrap();
        let payload = MiddlewareData::Callback(serialized);
        let ser_payload = super::serialize_primary(&payload).unwrap();

        match socket.send_to(&ser_payload, self.return_address).await {
            Ok(_) => todo!(),
            Err(_) => todo!(),
        }

        todo!()

        // todo!()
    }
}
