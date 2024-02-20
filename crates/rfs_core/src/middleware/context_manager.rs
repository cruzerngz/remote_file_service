use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};

use crate::RemotelyInvocable;

use super::InvokeError;

/// The context manager for the client.
///
/// The context manager handles the transmission of data to its server-side counterpart,
/// the dispatcher.
///
/// Integrity checks, validation, etc. are performed here.
#[derive(Debug)]
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

        Ok(Self {
            source_ip: source,
            target_ip: target,
        })
    }

    /// Send an invocation over the network, and returns the result.
    pub async fn invoke<P: RemotelyInvocable>(&self, payload: P) -> Result<P, InvokeError> {
        // send to server and wait for a reply
        let data = payload.invoke_bytes();

        // for now, bind and connect on every invocation

        let source = UdpSocket::bind(SocketAddrV4::new(self.source_ip, 0))
            .map_err(|_| InvokeError::RemoteConnectionFailed)?;

        source
            .connect(self.target_ip)
            .map_err(|_| InvokeError::RemoteConnectionFailed)?;

        source
            .send(&data)
            .map_err(|_| InvokeError::DataTransmissionFailed)?;

        let mut recv_buf = Vec::new();
        source
            .recv(&mut recv_buf)
            .map_err(|_| InvokeError::DataTransmissionFailed)?;

        P::process_invocation(&recv_buf)

        // self.target.connect(addr)
        // self.source_ip.send(&data);

        // let mut recv_buf = Vec::new();
        // self.source_ip.recv_from(&mut recv_buf);

        // todo!()
    }
}
