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
    use std::{collections::HashMap, fmt::Debug};

    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, Default)]
    struct S {
        item: bool,
        number: i32,
        s: String,
    }

    /// Performs a ser-de process
    fn ser_de_loop<T: Debug + Serialize + for<'a> Deserialize<'a>>(input: T) {
        let ser = serialize(&input).unwrap();

        println!("bytes: {} - {:?}", ser.len(), ser);
        println!("{:?}", std::str::from_utf8(&ser));
        let des: T = deserialize(&ser).unwrap();

        println!("{:?}", des);
    }

    #[test]
    fn test_ser_de_map() {
        let map: HashMap<String, i32> = HashMap::from([
            ("asd".to_string(), 10_000),
            ("how about that 👏👏👏".to_string(), 69),
        ]);

        ser_de_loop(map);
    }

    /// Testing ser_de of sequences, like vectors and tuples
    #[test]
    fn test_ser_de_seq() {
        let seq = vec![100, 200, 300, 400];
        ser_de_loop(seq);

        let tup = (12, 100, 20000);
        ser_de_loop(tup);
    }

    /// Testing ser_de of structs
    #[test]
    fn test_ser_de_struct() {
        let s = S {
            item: false,
            number: 10000,
            s: "asd".to_string(),
        };
        ser_de_loop(s);
    }

    #[test]
    fn test_ser_de_enum() {
        #[derive(Debug, Serialize, Deserialize)]
        enum E {
            This,
            That(bool),
            WhatEver((i32, bool)),
            IDontCare { a: bool, b: i8, c: String },
        }

        ser_de_loop(E::This);
        ser_de_loop(E::That(false));
        ser_de_loop(E::WhatEver((10, true)));
        ser_de_loop(E::IDontCare {
            a: true,
            b: i8::MAX,
            c: "Hello How are You".to_string(),
        });
    }
}
