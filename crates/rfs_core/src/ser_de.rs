//! Serialization and deserialization module
#![allow(unused)]

use self::err::SerDeResult;

mod consts;
mod de;
mod err;
mod ser;

/// Serialize a data structure to a vector of bytes
pub fn serialize<T: serde::Serialize>(value: &T) -> SerDeResult<Vec<u8>> {
    let mut serializer = ser::RfsSerializer::default();

    value.serialize(&mut serializer)?;

    Ok(serializer.output)
}

/// Deserialize a data structure from a slice of bytes
pub fn deserialize<T>(bytes: &[u8]) -> SerDeResult<T>
where
    T: for<'a> serde::Deserialize<'a>,
{
    let mut deserializer = de::RfsDeserializer::from_slice(bytes);

    T::deserialize(&mut deserializer)
}

/// Serializing and deserializing tests
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, Default)]
    struct S {
        item: bool,
        number: i32,
        s: String,
    }

    #[test]
    fn test_ser_de_struct() {
        let instance = S {
            item: true,
            number: 10_000,
            s: "testing".to_string(),
        };

        let instance: HashMap<String, i32> = HashMap::from([
            ("asd".to_string(), 10_000),
            ("how about that 👏👏👏".to_string(), 69),
        ]);

        let serialized = serialize(&instance).unwrap();

        println!("{:?}", instance);
        println!("{:?}", serialized);
        let x = std::str::from_utf8(&serialized);
        println!("{:?}", x);

        let deserialized: HashMap<String, i32> = deserialize(&serialized).unwrap();

        println!("{:?}", deserialized)
    }
}
