//! The client-side middleware module

// TODO: refac all UDP to tokio's UDP
// we're going to use the select! macro baby

use std::{
    io,
    net::{Ipv4Addr, SocketAddrV4, UdpSocket},
    time::Duration,
};

use crate::{
    middleware::{MiddlewareData, ERROR_HEADER, MIDDLWARE_HEADER},
    ser_de, RemotelyInvocable,
};

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
}

impl ContextManager {
    /// Timeout for sending requests to the remote
    const TIMEOUT: Duration = Duration::from_secs(15);

    /// Create a new context manager, along with a target IP and port.
    ///
    /// TODO: bind and wait for server to become online.
    pub fn new(source: Ipv4Addr, target: SocketAddrV4) -> std::io::Result<Self> {
        let s = Self {
            source_ip: source,
            target_ip: target,
        };

        let sock = s.generate_socket();
        println!("{:?}", sock);
        let sock = sock?;

        log::debug!("establishing initial conn with remote...");
        sock.connect(s.target_ip)?;
        log::debug!("connection established, establishing handshake...");

        let payload = MiddlewareData::Ping;

        sock.send_to(
            &ser_de::serialize_packed_with_header(&payload, MIDDLWARE_HEADER).unwrap(),
            s.target_ip,
        )?;

        let mut buf = [0; 1000];
        let recv_bytes = sock.recv(&mut buf).unwrap();

        let revc_data: MiddlewareData =
            ser_de::deserialize_packed_with_header(&buf[..recv_bytes], MIDDLWARE_HEADER).unwrap();

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
        let data = &payload.invoke_bytes();

        // for now, bind and connect on every invocation

        let source = self.connect_to_remote()?;

        log::debug!("connected to {}", self.target_ip);

        let size = source
            .send(&data)
            .map_err(|_| InvokeError::DataTransmissionFailed)?;

        log::debug!("request sent: {} bytes", size);

        let mut recv_buf = [0; 10_000];
        source
            .recv(&mut recv_buf)
            .map_err(|_| InvokeError::DataTransmissionFailed)?;

        // check for an error header, and process the remote error
        if recv_buf.starts_with(ERROR_HEADER) {
            let error: InvokeError = ser_de::deserialize_packed(&recv_buf[ERROR_HEADER.len()..])
                .expect("failed to deserialize error packet");

            Err(error)
        } else {
            P::process_invocation(&recv_buf)
        }
    }

    /// Create and bind to a new socket, with an arbitary port
    fn generate_socket(&self) -> io::Result<UdpSocket> {
        let sock = UdpSocket::bind(SocketAddrV4::new(self.source_ip, 0))?;

        // sock.connect(self.target_ip)?;
        Ok(sock)
    }

    /// Connects a UDP socket to the remote and returns it
    fn connect_to_remote(&self) -> Result<UdpSocket, InvokeError> {
        let sock = self
            .generate_socket()
            .map_err(|_| InvokeError::RemoteConnectionFailed)?;

        sock.connect(self.target_ip)
            .map_err(|_| InvokeError::RemoteConnectionFailed)?;

        Ok(sock)
    }

    /// Ping the remote and waits for a response
    fn ping_remote(&self) -> Result<(), InvokeError> {
        let sock = self.connect_to_remote()?;

        sock.send(
            &ser_de::serialize_packed_with_header(&MiddlewareData::Ping, MIDDLWARE_HEADER).unwrap(),
        )
        .unwrap();

        Ok(())
    }
}
