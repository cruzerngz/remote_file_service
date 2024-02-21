use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};

use crate::{
    middleware::ERROR_HEADER,
    ser_de::{self, ByteViewer},
    RemotelyInvocable,
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
    /// The target address and port
    source_ip: Ipv4Addr,
    target_ip: SocketAddrV4,
}

impl ContextManager {
    /// Create a new context manager, along with a target IP and port.
    ///
    /// TODO: bind and wait for server to become online.
    pub fn new(source: Ipv4Addr, target: SocketAddrV4) -> std::io::Result<Self> {
        // OS to assign outgoing port
        let addr = SocketAddrV4::new(source, 0);

        // let sock = UdpSocket::bind(SocketAddrV4::new(source, 0))
        //     .map_err(|_| InvokeError::RemoteConnectionFailed)
        //     .unwrap();

        // sock.connect(target).unwrap();

        // sock.send(&[1,2,3,4,5,6,7,8,9,10]).unwrap();

        Ok(Self {
            source_ip: source,
            target_ip: target,
        })
    }

    /// Send an invocation over the network, and returns the result.
    pub async fn invoke<P: RemotelyInvocable>(&self, payload: P) -> Result<P, InvokeError> {
        // send to server and wait for a reply
        let data = &payload.invoke_bytes()[..40];

        // for now, bind and connect on every invocation

        let source = UdpSocket::bind(SocketAddrV4::new(self.source_ip, 0))
            .map_err(|_| InvokeError::RemoteConnectionFailed)?;

        source
            .connect(self.target_ip)
            .map_err(|_| InvokeError::RemoteConnectionFailed)?;

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
}
