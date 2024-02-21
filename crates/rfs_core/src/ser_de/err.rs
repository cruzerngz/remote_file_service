//! Error implementations
#![allow(unused)]

use serde::{de, ser};
pub type SerDeResult<R> = Result<R, Error>;

/// Serialization result
pub type SerResult<R> = Result<R, Error>;
/// Deserialization result
pub type DeResult<R> = Result<R, Error>;

/// Custom error object for this library
#[derive(Debug)]
pub enum Error {
    /// Expected enclosing delimiters are not found.
    DelimiterNotFound(u8),
    /// Type descriptor prefix does not match expected type
    PrefixNotMatched(u8),

    ///
    UnexpectedData { exp: String, have: u8 },

    /// Something's wrong with the data
    MalformedData,

    /// The deserializer does not have sufficient bytes continue the operation.
    OutOfBytes,

    /// A custom error
    Custom(String),
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::Custom(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::Custom(msg.to_string())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Error::DelimiterNotFound(d) => format!("Expected delimiter '{}' not found", *d as char),
            Error::PrefixNotMatched(p) => format!("Expected prefix '{}' not found", *p as char),
            Error::UnexpectedData { exp, have } => {
                format!("Unexpected data. Expected {}, have {:0b}", exp, have)
            }
            Error::OutOfBytes => format!("Out of bytes to deserialize"),
            Error::MalformedData => format!("Malformed data"),
            Error::Custom(c) => format!("Error: {}", c),
        };

        write!(f, "{}", msg)
    }
}
