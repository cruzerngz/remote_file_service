//! Dispatcher side implementations.
//!
//! This module contains implementations of various dispatchers.
#![allow(unused)]

use crate::middleware::{hash_primary, MiddlewareData};
use crate::ser_de::{self, ser};

use super::{PayloadHandler, TransmissionProtocol, BYTE_BUF_SIZE};
use std::collections::btree_map;
use std::fmt::Debug;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::net::{SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use std::{io, marker};
use tokio::net::{ToSocketAddrs, UdpSocket};

/// The dispatcher for remote invocations.
///
/// The dispatcher routes the contents of remote invocations to their
/// appropriate handlers.
#[derive(Debug)]
pub struct Dispatcher<H, T>
where
    H: Debug + PayloadHandler,
    T: TransmissionProtocol,
{
    socket: Arc<UdpSocket>,
    timeout: Duration,
    retries: u8,

    /// Inner data structure that implements logic for remote interfaces
    handler: H,
    /// Message passing protocol. Acts as a transport layer.
    ///
    /// We only need the trait associated methods, so a struct instance is not required.
    protocol: T,
}

impl<H, T> Dispatcher<H, T>
where
    H: Debug + PayloadHandler,
    T: TransmissionProtocol + Debug + std::marker::Send + std::marker::Sync,
{
    /// Create a new dispatcher from the handler and a listening IP.
    ///
    /// Choose a transmission protocol that implmements [`TransmissionProtocol`]
    pub async fn new<A: ToSocketAddrs>(
        addr: A,
        handler: H,
        protocol: T,
        timeout: Duration,
        retries: u8,
    ) -> Self {
        let socket = UdpSocket::bind(addr)
            .await
            .expect("failed to bind to specified address");

        log::info!("dipatcher using {:?}", protocol);

        Self {
            socket: Arc::new(socket),
            handler,
            protocol,
            timeout,
            retries,
        }
    }

    /// Runs the dispatcher indefinitely.
    pub async fn dispatch(&mut self) {
        let mut buf = [0; BYTE_BUF_SIZE];

        loop {
            // buf.clear();

            match self
                .protocol
                .recv_bytes(&self.socket, self.timeout, self.retries)
                .await
            {
                Ok((addr, bytes)) => {
                    log::debug!("received {} bytes from {}", bytes.len(), addr);

                    // connection packets have zero length
                    if bytes.len() == 0 {
                        continue;
                    }

                    log::debug!("packet has stuff");
                    // let header = buf.iter().take(20).map(|num| *num).collect::<Vec<_>>();
                    // log::debug!("packet header {:?}", std::str::from_utf8(&header));

                    // send an ack back
                    // T::send_ack(&self.socket, addr, copy).await;

                    let data: MiddlewareData = match super::deserialize_primary(&buf) {
                        Ok(d) => d,
                        Err(e) => {
                            log::error!("deserialization failed: {:?}", e);

                            continue;
                        }
                    };

                    let middlware_response = match data {
                        MiddlewareData::Ping => handle_ping().await,
                        MiddlewareData::Payload(payload) => {
                            handle_payload(&mut self.handler, &payload).await
                        }
                        MiddlewareData::Callback(call) => handle_callback(&call).await,

                        // errors are client-side only
                        // dispatcher should not be receiving errors directly from a client
                        MiddlewareData::Error(e) => {
                            log::info!("stray error: {:?}", e);
                            continue;
                        }

                        // acks are checked for right after sending
                        MiddlewareData::Ack(h) => {
                            log::info!("stray ack: {}", h);
                            continue;
                        }

                        _ => todo!(),
                    };

                    let serialized_response =
                        super::serialize_primary(&middlware_response).unwrap();

                    // send the result and await an ack
                    let sent_bytes = self
                        .protocol
                        .send_bytes(
                            &self.socket,
                            addr,
                            &serialized_response,
                            self.timeout,
                            self.retries,
                        )
                        .await;

                    log::debug!("sent {:?} bytes to {}", sent_bytes, addr);
                }

                // log the error
                Err(e) => {
                    log::error!("Receive error: {}", e);
                }
            }
        }
    }
}

/// Handle a ping request
async fn handle_ping() -> MiddlewareData {
    MiddlewareData::Ping
}

/// Handle remote invocations
async fn handle_payload<H: PayloadHandler>(handler: &mut H, payload: &[u8]) -> MiddlewareData {
    match handler.handle_payload(payload).await {
        Ok(res) => MiddlewareData::Payload(res),
        Err(e) => MiddlewareData::Error(e),
    }
}

/// Handle callbacks (not used atm)
async fn handle_callback(call: &[u8]) -> MiddlewareData {
    todo!()
}

/// Hash an object and derive a boolean from it in a determinstic way
fn hash_to_boolean<H: Hash>(item: H) -> bool {
    let mut hasher = DefaultHasher::new();
    item.hash(&mut hasher);

    let value = hasher.finish();

    let first = value & 0b11111;
    let last = (value >> 56) & 0b11111;

    match (value >> first) & 0b1 ^ (value >> (63 - last)) & 0b1 {
        0 => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_to_bool() {
        for _ in 0..100 {
            let res = hash_to_boolean(std::time::Instant::now());

            println!("{:?}", res);
        }
    }
}
