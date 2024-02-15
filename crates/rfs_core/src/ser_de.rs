//! Serialization and deserialization module
#![allow(unused)]

mod de;
mod ser;
mod err;


/// Serialize a data structure
pub fn to_serialized_bytes<T: serde::Serialize>(value: &T) -> Vec<u8> {
    vec![]
}

/// Deserialize a data structure
pub fn from_serialized_bytes<T: serde::Serialize>(bytes: &[u8]) -> T {
    todo!()
}
