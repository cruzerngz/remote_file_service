//! Dispatcher side implementations.
//!
//! This module contains implementations of various dispatchers.
#![allow(unused)]

use crate::middleware::{hash_primary, MiddlewareData, ERROR_HEADER, MIDDLWARE_HEADER};
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

/// A faulty dispatcher. That is, a dispatcher that occasionally drops requests and does not send a response.
pub struct FaultyDispatcher<H: Debug + PayloadHandler> {
    socket: UdpSocket,

    // Inner data structure that implements logic for remote interfaces
    handler: H,
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

        log::debug!("dipatcher running on {:?}", protocol);

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

            match self.socket.recv_from(&mut buf).await {
                Ok((bytes, addr)) => {
                    log::debug!("received {} bytes from {}", bytes, addr);

                    // connection packets have zero length
                    if bytes == 0 {
                        continue;
                    }

                    log::debug!("packet has stuff");
                    // let header = buf.iter().take(20).map(|num| *num).collect::<Vec<_>>();
                    // log::debug!("packet header {:?}", std::str::from_utf8(&header));

                    let copy = &buf[..bytes];
                    log::debug!("packet: {:?}", copy);

                    // send an ack back
                    T::send_ack(&self.socket, addr, copy).await;

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
                    };

                    let serialized_response =
                        super::serialize_primary(&middlware_response).unwrap();

                    // send the result and await an ack
                    let sent_bytes = T::send_bytes(
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

impl<H> FaultyDispatcher<H>
where
    H: Debug + PayloadHandler,
{
    /// Create a new dispatcher from the handler and a listening IP
    pub async fn new(addr: SocketAddrV4, handler: H) -> Self {
        let socket = UdpSocket::bind(addr)
            .await
            .expect("failed to bind to specified address");

        Self { socket, handler }
    }

    /// Runs the dispatcher indefinitely.
    pub async fn dispatch(&mut self) {
        let mut buf = [0; BYTE_BUF_SIZE];

        loop {
            if hash_to_boolean(std::time::Instant::now()) {
                continue;
            }

            match self.socket.recv_from(&mut buf).await {
                Ok((bytes, addr)) => {
                    log::debug!("received {} bytes from {}", bytes, addr);

                    // connection packets have zero length
                    if bytes == 0 {
                        continue;
                    }

                    log::debug!("packet has stuff");
                    let copy = &buf[..bytes];

                    if copy.starts_with(MIDDLWARE_HEADER) {
                        self.handle_middleware_data(copy, addr)
                            .await
                            .expect("failed to send back to source");
                        continue;
                    }

                    // to be spawned as a separate task
                    let bytes = match self.handler.handle_payload(copy).await {
                        Ok(res) => {
                            log::debug!("payload header: {:?}", &res[..20]);

                            self.socket
                                .send_to(&res, addr)
                                .await
                                .expect("failed to send back to source")
                        }
                        Err(e) => {
                            log::error!("Invoke error: {:?}", e);

                            let mut data = ERROR_HEADER.to_vec();
                            data.extend(ser_de::serialize_packed(&e).unwrap());

                            self.socket
                                .send_to(&data, addr)
                                .await
                                .expect("failed to send error back to source")
                        }
                    };

                    log::debug!("sent {} bytes to {}", bytes, addr);
                }
                // log the error
                Err(e) => {
                    log::error!("Receive error: {}", e);
                }
            }
        }
    }

    /// Handle inter-middleware comms
    async fn handle_middleware_data(
        &self,
        middleware_data: &[u8],
        reply_addr: SocketAddr,
    ) -> io::Result<()> {
        let data: MiddlewareData =
            ser_de::deserialize_packed_with_header(&middleware_data, MIDDLWARE_HEADER).unwrap();

        log::debug!("middleware: {:?}", data);

        let res = match data {
            MiddlewareData::Ping => MiddlewareData::Ping,
            _ => todo!(),
        };

        self.socket
            .send_to(
                &ser_de::serialize_packed_with_header(&res, MIDDLWARE_HEADER).unwrap(),
                reply_addr,
            )
            .await?;
        Ok(())
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
