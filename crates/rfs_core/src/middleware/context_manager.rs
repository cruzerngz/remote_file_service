//! The client-side middleware module

use crate::{middleware::MiddlewareData, RemotelyInvocable};
use std::{
    fmt::Debug,
    io,
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
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
#[derive(Debug, Clone)]
pub struct ContextManager
where
// T: TransmissionProtocol,
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
    protocol: Arc<dyn TransmissionProtocol + Send + Sync>,
}

impl ContextManager
// where
// T: TransmissionProtocol + std::marker::Send + std::marker::Sync,
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
        protocol: Arc<dyn TransmissionProtocol + Send + Sync>,
    ) -> std::io::Result<Self> {
        let mut s = Self {
            source_ip: source,
            target_ip: target,
            timeout,
            retries,
            protocol,
        };

        // Ok(s)

        let sock = s.generate_socket().await?;
        println!("{:?}", sock);

        log::debug!("establishing initial conn with remote from {:?}", sock);

        let payload = MiddlewareData::Ping;
        let ser_payload = crate::serialize(&payload).expect("serialization must not fail");

        let payload_size = s
            .protocol
            .send_bytes(&sock, target, &ser_payload, timeout, retries)
            .await?;

        assert_eq!(payload_size, ser_payload.len());

        let (_addr, data) = s.protocol.recv_bytes(&sock, timeout, retries).await?;

        let resp: MiddlewareData = crate::deserialize(&data).unwrap();

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
    pub async fn invoke<P: RemotelyInvocable + Debug>(
        &mut self,
        payload: P,
    ) -> Result<P, InvokeError> {
        log::info!("invoking: {:?}", payload);

        // send to server and wait for a reply
        let data = payload.invoke_bytes();

        // for now, bind and connect on every invocation
        let source = self.generate_socket().await?;

        log::debug!("connected to {}", self.target_ip);

        let middleware_payload = MiddlewareData::Payload(data);
        let serialized_payload =
            crate::serialize(&middleware_payload).expect("serialization must not fail");

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

        log::debug!("awaiting remote response on {:?}", source);
        let (_addr, resp) = self
            .protocol
            .recv_bytes(&source, self.timeout, self.retries)
            .await?;

        let middleware_resp: MiddlewareData =
            crate::deserialize(&resp).map_err(|_| InvokeError::DeserializationFailed)?;

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
