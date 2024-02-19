//! This module contains the client and server side
//! objects that transmit the contents of method invocations
//! over the network.
//!
//!

/// The context manager (middleware) for remote invocations.
///
/// The context manager handles the transmission of data to it's server-side counterpart,
/// the dispatcher.
///
/// Integrity checks, validation, etc. are performed here.
#[derive(Debug)]
pub struct ContextManager {}

/// The dispatcher for remote invocations.
///
/// The dispatcher routes the contents of remote invocations to their
/// appropriate handlers.
#[derive(Debug)]
pub struct Dispatcher {}
