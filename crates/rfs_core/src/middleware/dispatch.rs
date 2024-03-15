//! Dispatcher side implementations.
//!
//! This module contains implementations of various dispatchers.
#![allow(unused)]

use crate::middleware::{hash_primary, MiddlewareData};
use crate::ser_de::{self, ser};

use super::{PayloadHandler, TransmissionProtocol, BYTE_BUF_SIZE};
use futures::lock::Mutex;
use std::borrow::{Borrow, BorrowMut};
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
    sequential: bool,

    /// Inner data structure that implements logic for remote interfaces
    handler: Arc<Mutex<H>>,
    /// Message passing protocol. Acts as a transport layer.
    ///
    /// We only need the trait associated methods, so a struct instance is not required.
    protocol: T,
}

impl<H, T> Dispatcher<H, T>
where
    H: Debug + PayloadHandler + std::marker::Send + std::marker::Sync + 'static,
    T: TransmissionProtocol + Debug + std::marker::Send + std::marker::Sync + 'static,
{
    /// Create a new dispatcher from the handler and a listening IP.
    ///
    /// Choose a transmission protocol that implmements [`TransmissionProtocol`]
    pub async fn new<A: ToSocketAddrs>(
        addr: A,
        handler: H,
        protocol: T,
        sequential: bool,
        timeout: Duration,
        retries: u8,
    ) -> Self {
        let socket = UdpSocket::bind(addr)
            .await
            .expect("failed to bind to specified address");

        log::info!("dipatcher using {:?}", protocol);

        Self {
            socket: Arc::new(socket),
            handler: Arc::new(Mutex::new(handler)),
            sequential,
            protocol,
            timeout,
            retries,
        }
    }

    /// Runs the dispatcher indefinitely.
    pub async fn dispatch(&mut self) {
        let mut buf = [0; BYTE_BUF_SIZE];

        let mut request_num = 0;

        loop {
            log::info!("awaiting request #{}", request_num);

            // create new response socket
            // so we don't intercepts requests to the main dispatch socket
            let mut resp_addr = self
                .socket
                .local_addr()
                .expect("failed to get local address");
            resp_addr.set_port(0);

            let resp_sock = UdpSocket::bind(resp_addr)
                .await
                .expect("failed to bind response socket");

            match self
                .protocol
                .recv_bytes(&self.socket, self.timeout, self.retries)
                .await
            {
                // spawn resp in separate thread
                Ok((addr, bytes)) => {
                    log::info!("dispatch received request #{} from {}", request_num, addr);
                    log::debug!("response will be sent from {:?}", resp_sock);

                    // let socket = self.socket.clone();
                    let handler = self.handler.clone();
                    let proto = self.protocol.clone(); // proto cannot be shared
                    let timeout = self.timeout.clone();
                    let retries = self.retries.clone();

                    // tasks can run for an arbitrary amount of time
                    let handle = tokio::spawn(async move {
                        Self::execute_handler(
                            addr, &bytes, resp_sock, handler, proto, timeout, retries,
                        )
                        .await
                    });

                    // if we are processing sequentially, we wait on each task every loop iter
                    if self.sequential {
                        handle.await.expect("thread join error");
                    }
                }

                // log the error
                Err(e) => {
                    log::error!("Receive error: {}", e);
                }
            }

            request_num += 1;
        }
    }

    /// Executes the handler
    async fn execute_handler(
        address: SocketAddrV4,
        data: &[u8],
        socket: UdpSocket,
        handler: Arc<Mutex<H>>,
        mut protocol: T,
        timeout: Duration,
        retries: u8,
    ) {
        log::debug!("received {} bytes from {}", data.len(), address);

        // connection packets have zero length
        if data.len() == 0 {
            return;
        }

        log::debug!("packet has stuff");
        log::debug!("packet contents: {:?}", data);

        // send an ack back
        // T::send_ack(&self.socket, addr, copy).await;

        let data: MiddlewareData = match crate::deserialize(&data) {
            Ok(d) => d,
            Err(e) => {
                log::error!("deserialization failed: {:?}", e);

                return;
            }
        };

        let mut handler_lock = handler.lock().await;

        let middlware_response = match data {
            MiddlewareData::Ping => handle_ping().await,
            MiddlewareData::Payload(payload) => match handler_lock.handle_payload(&payload).await {
                Ok(res) => MiddlewareData::Payload(res),
                Err(e) => MiddlewareData::Error(e),
            },
            MiddlewareData::Callback(call) => handle_callback(&call).await,

            // errors are client-side only
            // dispatcher should not be receiving errors directly from a client
            MiddlewareData::Error(e) => {
                log::info!("stray error: {:?}", e);
                return;
            }

            // acks are checked for right after sending
            MiddlewareData::Ack(h) => {
                log::info!("stray ack: {}", h);
                return;
            }

            _ => todo!(),
        };

        drop(handler_lock);

        let serialized_response = crate::serialize(&middlware_response).unwrap();

        log::debug!("dispatch sending response to {}", address);

        // send the result
        let sent_bytes = protocol
            .send_bytes(&socket, address, &serialized_response, timeout, retries)
            .await;

        log::debug!("sent {:?} bytes to {}", sent_bytes, address);
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
