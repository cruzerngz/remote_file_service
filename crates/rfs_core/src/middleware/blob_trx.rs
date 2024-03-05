//! This module contains the structs and related items
//! for transmitting large blobs of data over UDP.

use std::marker;

use super::{ContextManager, TransmissionProtocol};

/// A binary blob transmitter/recviver
#[derive(Clone, Debug)]
pub struct BlobTransceiver<Mode, T>
where
    T: TransmissionProtocol,
{
    ctx: ContextManager<T>,
    marker: marker::PhantomData<Mode>,
}

#[derive(Clone, Debug)]
pub struct Transmitter;

#[derive(Clone, Debug)]
pub struct Receiver;

impl<T> BlobTransceiver<Transmitter, T>
where
    T: TransmissionProtocol,
{
    /// Create a blob transmitter
    pub fn transmitter(ctx: ContextManager<T>) -> Self {
        Self {
            ctx,
            marker: marker::PhantomData,
        }
    }
}

impl<T> BlobTransceiver<Receiver, T>
where
    T: TransmissionProtocol,
{
    /// Create a blob receiver
    pub fn receiver(ctx: ContextManager<T>) -> Self {
        Self {
            ctx,
            marker: marker::PhantomData,
        }
    }
}

impl<Mode, T> BlobTransceiver<Mode, T> where T: TransmissionProtocol {}
