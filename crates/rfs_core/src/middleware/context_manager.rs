//! The client-side middleware module

use crate::{middleware::MiddlewareData, RemotelyInvocable};
use std::{
    io,
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use tokio::net::UdpSocket;

use super::{InvokeError, TransmissionProtocol};

/// The context manager for the client.
///
/// The context manager handles the transmission of data to its server-side counterpart,
/// the dispatcher.
///
/// Integrity checks, validation, etc. are performed here.
#[derive(Debug, Clone, Copy)]
pub struct ContextManager<T>
where
    T: TransmissionProtocol,
{
    /// The client's IP
    source_ip: Ipv4Addr,
    /// The server's IP
    target_ip: SocketAddrV4,

    /// Request timeout
    pub(super) timeout: Duration,

    /// Number of retries
    pub(super) retries: u8,

    #[allow(unused)]
    protocol: T,
}

impl<T> ContextManager<T>
where
    T: TransmissionProtocol + std::marker::Send + std::marker::Sync,
{
    /// Timeout for sending requests to the remote

    /// Create a new context manager, along with a target IP and port.
    ///
    /// TODO: bind and wait for server to become online.
    pub async fn new(
        source: Ipv4Addr,
        target: SocketAddrV4,
        timeout: Duration,
        retries: u8,
        protocol: T,
    ) -> std::io::Result<Self> {
        let mut s = Self {
            source_ip: source,
            target_ip: target,
            timeout,
            retries,
            protocol,
        };

        let sock = s.generate_socket().await?;
        println!("{:?}", sock);

        log::debug!("establishing initial conn with remote...");
        sock.connect(s.target_ip).await?;
        log::debug!("establishing handshake...");

        let payload = MiddlewareData::Ping;

        let resp = s
            .protocol
            .send_bytes(
                &sock,
                target,
                &super::serialize_primary(&payload).unwrap(),
                timeout,
                retries,
            )
            .await?;

        let (addr, data) = s.protocol.recv_bytes(&sock, timeout, retries).await?;

        let resp: MiddlewareData = super::deserialize_primary(&data).unwrap();

        match resp == payload {
            true => {
                log::debug!("handshake established");
                Ok(s)
            }
            false => {
                log::debug!("invalid response");
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Expected ping response",
                ))
            }
        }
    }

    /// Send an invocation over the network, and returns the result.
    pub async fn invoke<P: RemotelyInvocable>(&mut self, payload: P) -> Result<P, InvokeError> {
        // send to server and wait for a reply
        let data = payload.invoke_bytes();

        // for now, bind and connect on every invocation

        let source = self.connect_remote().await?;

        log::debug!("connected to {}", self.target_ip);

        let middleware_payload = MiddlewareData::Payload(data);
        let serialized_payload = super::serialize_primary(&middleware_payload).unwrap();

        let _resp = self
            .protocol
            .send_bytes(
                &source,
                self.target_ip,
                &serialized_payload,
                self.timeout,
                self.retries,
            )
            .await
            .map_err(|e| <InvokeError>::from(e))?;

        let (addr, resp) = self
            .protocol
            .recv_bytes(&source, self.timeout, self.retries)
            .await?;

        // response shoud come from the same IP
        assert_eq!(self.target_ip, addr);

        let middleware_resp: MiddlewareData =
            super::deserialize_primary(&resp).map_err(|_| InvokeError::DeserializationFailed)?;

        match middleware_resp {
            MiddlewareData::Payload(p) => P::process_invocation(&p),
            MiddlewareData::Error(e) => Err(e),
            _ => unimplemented!(),
        }
    }

    /// Create and bind to a new socket, with an arbitary port
    async fn generate_socket(&self) -> io::Result<UdpSocket> {
        let sock = UdpSocket::bind(SocketAddrV4::new(self.source_ip, 0)).await?;

        Ok(sock)
    }

    /// Connects a UDP socket to the remote specified in `self`
    /// and returns it.
    async fn connect_remote(&self) -> Result<UdpSocket, InvokeError> {
        let sock = self
            .generate_socket()
            .await
            .map_err(|_| InvokeError::RemoteConnectionFailed)?;

        sock.connect(self.target_ip)
            .await
            .map_err(|_| InvokeError::RemoteConnectionFailed)?;

        Ok(sock)
    }

    // /// Ping the remote and waits for a response
    //     async fn ping_remote(&self) -> Result<(), InvokeError> {
    //         let sock = self.connect_remote().await?;

    //         sock.send(
    //             &ser_de::serialize_packed_with_header(&MiddlewareData::Ping, MIDDLWARE_HEADER).unwrap(),
    //         )
    //         .await
    //         .unwrap();

    //         Ok(())
    //     }
}
