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
    DelimiterNotFound(char),
    /// Type descriptor prefix does not match expected type
    PrefixNotMatched(String),

    /// Something's wrong with the data
    MalformedData,
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        todo!()
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        todo!()
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
