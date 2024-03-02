//! The client-side middleware module

use crate::{
    middleware::{MiddlewareData, MIDDLWARE_HEADER},
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
    const TIMEOUT: Duration = Duration::from_secs(15);

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

        sock.send_to(
            &super::serialize_primary(&MiddlewareData::Ping).unwrap(),
            s.target_ip,
        )
        .await?;

        let mut buf = [0; 1000];
        let recv_bytes = sock.recv(&mut buf).await.unwrap();

        let revc_data: MiddlewareData = super::deserialize_primary(&buf[..recv_bytes]).unwrap();

        match revc_data == payload {
            true => {
                log::debug!("handshake successful");

                Ok(s)
            }
            false => {
                log::debug!("handshake unsuccessful");

                Err(io::Error::new(io::ErrorKind::Other, "you are a failure"))
            }
        }
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

        let size = source
            .send(&serialized_payload)
            .await
            .map_err(|_| InvokeError::DataTransmissionFailed)?;

        log::debug!("request sent: {} bytes", size);

        let mut recv_buf = [0; 10_000];
        let num_bytes = source
            .recv(&mut recv_buf)
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
