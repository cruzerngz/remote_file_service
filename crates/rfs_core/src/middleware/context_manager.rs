//! The client-side middleware module

use crate::{
    middleware::{send_ack, send_timeout, MiddlewareData, MIDDLWARE_HEADER},
    ser_de, RemotelyInvocable,
};
use std::{
    io,
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use tokio::net::UdpSocket;

use super::InvokeError;

/// The context manager for the client.
///
/// The context manager handles the transmission of data to its server-side counterpart,
/// the dispatcher.
///
/// Integrity checks, validation, etc. are performed here.
#[derive(Debug, Clone, Copy)]
pub struct ContextManager {
    /// The client's IP
    source_ip: Ipv4Addr,
    /// The server's IP
    target_ip: SocketAddrV4,

    /// Request timeout
    timeout: Duration,

    /// Number of retries
    retries: u8,
}

impl ContextManager {
    /// Timeout for sending requests to the remote

    /// Create a new context manager, along with a target IP and port.
    ///
    /// TODO: bind and wait for server to become online.
    pub async fn new(
        source: Ipv4Addr,
        target: SocketAddrV4,
        timeout: Duration,
        retries: u8,
    ) -> std::io::Result<Self> {
        let s = Self {
            source_ip: source,
            target_ip: target,
            timeout,
            retries,
        };

        let sock = s.generate_socket().await?;
        println!("{:?}", sock);

        log::debug!("establishing initial conn with remote...");
        sock.connect(s.target_ip).await?;
        log::debug!("connection established, establishing handshake...");

        let payload = MiddlewareData::Ping;

        let res = send_timeout(
            &sock,
            target,
            &super::serialize_primary(&payload).unwrap(),
            timeout,
            retries,
        )
        .await?;

        log::debug!("handshake established");

        Ok(s)
    }

    /// Send an invocation over the network, and returns the result.
    pub async fn invoke<P: RemotelyInvocable>(&self, payload: P) -> Result<P, InvokeError> {
        // send to server and wait for a reply
        let data = payload.invoke_bytes();

        // for now, bind and connect on every invocation

        let source = self.connect_to_remote().await?;

        log::debug!("connected to {}", self.target_ip);

        let middleware_payload = MiddlewareData::Payload(data);
        let serialized_payload = super::serialize_primary(&middleware_payload).unwrap();

        let size = send_timeout(
            &source,
            self.target_ip,
            &serialized_payload,
            self.timeout,
            self.retries,
        )
        .await
        .map_err(|_| InvokeError::RequestTimedOut)?;

        log::debug!("request sent: {} bytes", size);

        let mut recv_buf = [0; 10_000];
        let num_bytes = source
            .recv(&mut recv_buf)
            .await
            .map_err(|_| InvokeError::DataTransmissionFailed)?;

        // ack back to remote
        send_ack(&source, self.target_ip, &recv_buf[..num_bytes])
            .await
            .map_err(|_| InvokeError::DataTransmissionFailed)?;

        let middleware_resp: MiddlewareData =
            super::deserialize_primary(&recv_buf[..num_bytes]).unwrap();

        match middleware_resp {
            MiddlewareData::Payload(p) => P::process_invocation(&p),
            MiddlewareData::Error(e) => Err(e),
            _ => unimplemented!(),
        }
    }

    /// Create and bind to a new socket, with an arbitary port
    async fn generate_socket(&self) -> io::Result<UdpSocket> {
        let sock = UdpSocket::bind(SocketAddrV4::new(self.source_ip, 0)).await?;

        // sock.connect(self.target_ip)?;
        Ok(sock)
    }

    /// Connects a UDP socket to the remote and returns it
    async fn connect_to_remote(&self) -> Result<UdpSocket, InvokeError> {
        let sock = self
            .generate_socket()
            .await
            .map_err(|_| InvokeError::RemoteConnectionFailed)?;

        sock.connect(self.target_ip)
            .await
            .map_err(|_| InvokeError::RemoteConnectionFailed)?;

        Ok(sock)
    }

    /// Ping the remote and waits for a response
    async fn ping_remote(&self) -> Result<(), InvokeError> {
        let sock = self.connect_to_remote().await?;

        sock.send(
            &ser_de::serialize_packed_with_header(&MiddlewareData::Ping, MIDDLWARE_HEADER).unwrap(),
        )
        .await
        .unwrap();

        Ok(())
    }
}
