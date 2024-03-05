//! This module contains the structs and related items
//! for transmitting large blobs of data over UDP.
#![allow(unused)]

use std::{
    io, marker,
    net::{Ipv4Addr, SocketAddrV4},
};

use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;

use super::{ContextManager, TransmissionProtocol};

/// A binary blob transmitter/receiver.
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

    proto: marker::PhantomData<T>,
    marker: marker::PhantomData<Mode>,
}

#[derive(Clone, Debug)]
pub struct Transmitter;

#[derive(Clone, Debug)]
pub struct Receiver;

/// The data type sent between `BlobTransceiver`s
#[derive(Clone, Debug, Serialize, Deserialize)]
enum BlobPacket {
    /// Associate something
    Associate()

}

impl<T> BlobTransceiver<Transmitter, T>
where
    T: TransmissionProtocol,
{
    /// Create a blob transmitter
    pub async fn transmitter(bind_addr: Ipv4Addr, remote: SocketAddrV4) -> io::Result<Self> {
        Self::_new(bind_addr, remote).await
    }
}

impl<T> BlobTransceiver<Receiver, T>
where
    T: TransmissionProtocol,
{
    /// Create a blob receiver
    pub async fn receiver(bind_addr: Ipv4Addr, remote: SocketAddrV4) -> io::Result<Self> {
        Self::_new(bind_addr, remote).await
    }
}

impl<Mode, T> BlobTransceiver<Mode, T>
where
    T: TransmissionProtocol,
{
    /// Internal method
    async fn _new(bind_addr: Ipv4Addr, remote: SocketAddrV4) -> io::Result<Self> {
        // bind address gets an OS-assigned socket
        let socket = UdpSocket::bind(SocketAddrV4::new(bind_addr, 0)).await?;

        Ok(Self {
            bind_addr,
            remote,
            socket,
            proto: marker::PhantomData,
            marker: marker::PhantomData,
        })
    }

    /// Associate with it's counterpart at the specified remote address.
    ///
    /// Note that during the association, the address/port of both the transmitter and receiver
    /// may change.
    pub async fn associate(&mut self) -> io::Result<()> {
        todo!()
    }
}
