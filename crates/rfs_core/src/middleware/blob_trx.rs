//! This module contains the structs and related items
//! for transmitting large blobs of data over UDP.
#![allow(unused)]

use std::{
    io, marker,
    net::{Ipv4Addr, SocketAddrV4},
    path::Path,
    time::Duration,
};

use futures::{io::BufReader, AsyncReadExt};
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;

use crate::ser_de::ByteViewer;

use super::{ContextManager, TransmissionProtocol};

/// A binary blob transmitter/receiver.
///
/// This should be used when data payloads exceed the UDP packet limit (65507 bytes).
///
/// This struct relies on using a context manager for initial association.
/// After association with the remote, both ends may change their ports
/// to free up traffic for the context manager/dispatcher.
#[derive(Debug)]
pub struct BlobTransceiver<Mode, T>
where
    T: TransmissionProtocol,
{
    /// Address the transceiver tx/rx from
    bind_addr: Ipv4Addr,

    /// Address of the remote
    remote: SocketAddrV4,

    socket: UdpSocket,

    timeout: Duration,
    retries: u8,

    proto: marker::PhantomData<T>,
    marker: marker::PhantomData<Mode>,
}

/// Transmitter sends stuff to the receiver
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transmitter;

/// Receiver sends stuff to the transmitter
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receiver;

/// The data type sent between `BlobTransceiver`s
#[derive(Clone, Debug, Serialize, Deserialize)]
enum BlobPacket {
    /// Associate the local instance of `Self` with a remote instance.
    Associate,

    Metadata {
        /// Size of the payload to be transferred
        size: usize,

        /// Single transmission size (this should be close to UDP max payload)
        data_size: usize,
    },

    /// Data packet
    Data {
        seq_num: usize,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },

    /// Ack for sequence number
    Ack(usize),
}

impl<T> BlobTransceiver<Transmitter, T>
where
    T: TransmissionProtocol,
{
    /// Create a blob transmitter
    pub async fn transmitter<U: TransmissionProtocol>(
        bind_addr: Ipv4Addr,
        remote: SocketAddrV4,
        ctx: &ContextManager<U>,
    ) -> io::Result<Self> {
        Self::_new(bind_addr, remote, &ctx).await
    }

    /// Ingest any bytes-like data.
    ///
    /// This will not send data until the receiver is ready.
    pub async fn ingest<R: futures::AsyncRead + Unpin>(&mut self, mut data: R) -> io::Result<()> {
        let mut buf = Vec::new();
        data.read_to_end(&mut buf);

        let mut bytes_left = buf.len();
        let mut viewer = ByteViewer::from_slice(&buf);

        let mut seq_num = 0;

        loop {
            let bytes_to_send = match viewer.distance_to_end() {
                65507.. => 65507,
                0 => break,
                // in between 0 and max packet size
                lower => lower,
            };

            let payload = viewer.next_bytes(bytes_to_send, true);

            seq_num += 1;
        }

        Ok(())
    }
}

impl<T> BlobTransceiver<Receiver, T>
where
    T: TransmissionProtocol,
{
    /// Create a blob receiver
    pub async fn receiver<U: TransmissionProtocol>(
        bind_addr: Ipv4Addr,
        remote: SocketAddrV4,
        ctx: &ContextManager<U>,
    ) -> io::Result<Self> {
        Self::_new(bind_addr, remote, &ctx).await
    }
}

impl<Mode, T> BlobTransceiver<Mode, T>
where
    T: TransmissionProtocol,
{
    /// Internal method
    async fn _new<U: TransmissionProtocol>(
        bind_addr: Ipv4Addr,
        remote: SocketAddrV4,
        ctx: &ContextManager<U>,
    ) -> io::Result<Self> {
        // bind address gets an OS-assigned socket
        let socket = UdpSocket::bind(SocketAddrV4::new(bind_addr, 0)).await?;

        Ok(Self {
            bind_addr,
            remote,
            socket,
            timeout: ctx.timeout,
            retries: ctx.retries,
            proto: marker::PhantomData,
            marker: marker::PhantomData,
        })
    }

    /// Associate with it's counterpart at the specified remote address.
    /// This method can be used to re-associate with a new remote.
    ///
    /// This serves as a sanity check if the remote is ready to do stuff.
    pub async fn associate(&mut self, remote: Option<SocketAddrV4>) -> io::Result<()> {
        let assoc_packet = BlobPacket::Associate;

        // let assoc_resp = T::send_with_response(
        //     &self.socket,
        //     remote.unwrap_or(self.remote),
        //     &super::serialize_primary(&assoc_packet)
        //         .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "serialization failed"))?,
        //     self.timeout,
        //     self.retries,
        // )
        // .await?;

        // let assoc_resp: BlobPacket = super::deserialize_primary(&assoc_resp).map_err(|_| {
        //     io::Error::new(io::ErrorKind::InvalidData, "failed to deserialize response")
        // })?;

        // if let BlobPacket::Associate = assoc_resp {
        //     Ok(())
        // } else {
        //     Err(io::Error::new(
        //         io::ErrorKind::InvalidInput,
        //         "expected associate packet",
        //     ))
        // }

        // temp
        return Ok(());
    }
}

impl<T> futures::Stream for BlobTransceiver<Receiver, T>
where
    T: TransmissionProtocol,
{
    type Item = Vec<u8>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::*;

    async fn test_stuff() {
        // let tx = BlobTransceiver::transmitter(todo!(), todo!(), todo!())
        //     .await
        //     .unwrap();

        // let rx = BlobTransceiver::receiver(todo!(), todo!(), todo!())
        //     .await
        //     .unwrap();
    }
}
