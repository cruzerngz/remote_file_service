//! Dispatcher side implementations.
//!
//! This module contains implementations of various dispatchers.

use crate::middleware::{MiddlewareData, ERROR_HEADER, MIDDLWARE_HEADER};
use crate::ser_de;

use super::PayloadHandler;
use std::fmt::Debug;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;
use std::net::{SocketAddr, SocketAddrV4, UdpSocket};

const BYTE_BUF_SIZE: usize = 65535;

/// The dispatcher for remote invocations.
///
/// The dispatcher routes the contents of remote invocations to their
/// appropriate handlers.
///
#[derive(Debug)]
pub struct Dispatcher<H: Debug + PayloadHandler> {
    socket: UdpSocket,

    // Inner data structure that implements logic for remote interfaces
    handler: H,
}

/// A faulty dispatcher. That is, a dispatcher that occasionally drops requests and does not send a response.
pub struct FaultyDispatcher<H: Debug + PayloadHandler> {
    socket: UdpSocket,

    // Inner data structure that implements logic for remote interfaces
    handler: H,
}

impl<H> Dispatcher<H>
where
    H: Debug + PayloadHandler,
{
    /// Create a new dispatcher from the handler and a listening IP
    pub fn new(addr: SocketAddrV4, handler: H) -> Self {
        let socket = UdpSocket::bind(addr).expect("failed to bind to specified address");

        Self { socket, handler }
    }

    /// Runs the dispatcher indefinitely.
    pub async fn dispatch(&mut self) {
        let mut buf = [0; BYTE_BUF_SIZE];

        loop {
            // buf.clear();

            match self.socket.recv_from(&mut buf) {
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
                            .expect("failed to send back to source");
                        continue;
                    }

                    // to be spawned as a separate task
                    let bytes = match self.handler.handle_payload(copy).await {
                        Ok(res) => {
                            log::debug!("payload header: {:?}", &res[..20]);

                            self.socket
                                .send_to(&res, addr)
                                .expect("failed to send back to source")
                        }
                        Err(e) => {
                            log::error!("Invoke error: {:?}", e);

                            let mut data = ERROR_HEADER.to_vec();
                            data.extend(ser_de::serialize_packed(&e).unwrap());

                            self.socket
                                .send_to(&data, addr)
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
    fn handle_middleware_data(
        &self,
        middleware_data: &[u8],
        reply_addr: SocketAddr,
    ) -> io::Result<()> {
        let data: MiddlewareData =
            ser_de::deserialize_packed_with_header(&middleware_data, MIDDLWARE_HEADER).unwrap();

        log::debug!("middleware: {:?}", data);

        let res = match data {
            MiddlewareData::Ping => MiddlewareData::Ping,
        };

        self.socket.send_to(
            &ser_de::serialize_packed_with_header(&res, MIDDLWARE_HEADER).unwrap(),
            reply_addr,
        )?;
        Ok(())
    }
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
    pub fn new(addr: SocketAddrV4, handler: H) -> Self {
        let socket = UdpSocket::bind(addr).expect("failed to bind to specified address");

        Self { socket, handler }
    }

    /// Runs the dispatcher indefinitely.
    pub async fn dispatch(&mut self) {
        let mut buf = [0; BYTE_BUF_SIZE];

        loop {
            if hash_to_boolean(std::time::Instant::now()) {
                continue;
            }

            match self.socket.recv_from(&mut buf) {
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
                            .expect("failed to send back to source");
                        continue;
                    }

                    // to be spawned as a separate task
                    let bytes = match self.handler.handle_payload(copy).await {
                        Ok(res) => {
                            log::debug!("payload header: {:?}", &res[..20]);

                            self.socket
                                .send_to(&res, addr)
                                .expect("failed to send back to source")
                        }
                        Err(e) => {
                            log::error!("Invoke error: {:?}", e);

                            let mut data = ERROR_HEADER.to_vec();
                            data.extend(ser_de::serialize_packed(&e).unwrap());

                            self.socket
                                .send_to(&data, addr)
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
    fn handle_middleware_data(
        &self,
        middleware_data: &[u8],
        reply_addr: SocketAddr,
    ) -> io::Result<()> {
        let data: MiddlewareData =
            ser_de::deserialize_packed_with_header(&middleware_data, MIDDLWARE_HEADER).unwrap();

        log::debug!("middleware: {:?}", data);

        let res = match data {
            MiddlewareData::Ping => MiddlewareData::Ping,
        };

        self.socket.send_to(
            &ser_de::serialize_packed_with_header(&res, MIDDLWARE_HEADER).unwrap(),
            reply_addr,
        )?;
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
