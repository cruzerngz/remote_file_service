//! Serialization and deserialization module

use self::err::SerDeResult;

pub mod byte_packer;
mod consts;
pub mod de;
pub mod err;
pub mod ser;

pub use consts::ByteSizePrefix;

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

/// Serialize a data structure and pack the bits.
///
/// The packing process must not fail.
pub fn serialize_packed<T: serde::Serialize>(value: &T) -> SerDeResult<Vec<u8>> {
    serialize(value).map(|v| byte_packer::pack_bytes(&v))
}

/// Unpack the bits and deserialize the data structure.
///
/// The unpacking process must not fail.
pub fn deserialize_packed<T>(bytes: &[u8]) -> SerDeResult<T>
where
    T: for<'a> serde::Deserialize<'a>,
{
    deserialize(&byte_packer::unpack_bytes(bytes))
}

/// Serialize a data structure with a header appended to the start
pub fn serialize_packed_with_header<T: serde::Serialize>(
    value: &T,
    header: &[u8],
) -> SerDeResult<Vec<u8>> {
    Ok([header, &serialize_packed(value)?].concat())
}

/// Match headers and then deserialize a sequence of bytes
pub fn deserialize_packed_with_header<T>(bytes: &[u8], header: &[u8]) -> SerDeResult<T>
where
    T: for<'a> serde::Deserialize<'a>,
{
    match bytes.starts_with(header) {
        true => deserialize_packed(&bytes[header.len()..]),
        false => Err(err::Error::MalformedData),
    }
}

/// A reference into an existing slice of bytes.
///
/// This data structure can perform various (immutable) operations on a slice of
/// bytes.
pub struct ByteViewer<'arr> {
    slice: &'arr [u8],
    size: usize,
    offset: usize,
}

// #[allow(unused)]
impl<'arr> ByteViewer<'arr> {
    /// Create a new viewer on a byte slice
    pub fn from_slice(s: &'arr [u8]) -> Self {
        Self {
            slice: s,
            size: s.len(),
            offset: 0,
        }
    }

    /// Returns the current slice starting from the internal
    /// offset as an iterator.
    ///
    /// This function will panic if there are no more elements to iterate over.
    ///
    pub fn curr_iter(&self) -> std::slice::Iter<'arr, u8> {
        // let curr_view = match self.is_end() {
        //     true => &self.slice[0..=0], // return a zero-sized slice
        //     false => &self.slice[self.offset..],
        // };

        let curr_view = &self.slice[self.offset..];
        curr_view.iter()
    }

    /// Advance the view on the underlying slice.
    ///
    /// If the new offset is larger than the size of the slice,
    /// this will return an error.
    ///
    /// The view can be advanced to the end.
    #[must_use]
    pub fn advance(&mut self, steps: usize) -> Result<(), ()> {
        match (self.offset + steps) <= self.size {
            true => {
                self.offset += steps;

                Ok(())
            }
            false => Err(()),
        }
    }

    /// Peek at the next byte in the slice.
    ///
    /// If the viewer is at the end, this returns `None`.
    pub fn peek(&self) -> Option<&u8> {
        self.slice.get(self.offset)
    }

    /// Takes the next 8 bytes and parses them into a [ByteSizePrefix].
    /// As most contiguous (variable len) collections have their sizes stored at the start,
    /// this is used to retrieve the size of the collection (in bytes) and advance the viewer.
    ///
    /// This can also be used to retrieve any primitive unsigned numeric type, as all numeric types are
    /// promoted to 64-bits during serialization.
    pub fn pop_size(&mut self) -> ByteSizePrefix {
        const NUM_BYTES: usize = std::mem::size_of::<ByteSizePrefix>();
        let size_bytes = self.next_bytes_fixed::<NUM_BYTES>(true);
        ByteSizePrefix::from_be_bytes(size_bytes)
    }

    /// Return the next byte and advance the view.
    ///
    /// There are no explicit bounds checks here.
    pub fn next_byte(&mut self) -> u8 {
        let b = self.slice[self.offset];
        self.offset += 1;

        b
    }

    /// Returns the next slice of bytes and advances the counter.
    /// If peeking, the counter does not advance.
    ///
    /// There are no explicit bounds check on the allowed size here.
    pub fn next_bytes(&mut self, size: usize, advance: bool) -> &'arr [u8] {
        let view = &self.slice[self.offset..(self.offset + size)];

        match advance {
            true => self.offset += size,
            false => (),
        }

        view
    }

    /// Returns a copy of the next slice of bytes as a fixed-size array.
    ///
    /// If `advance` is set to `true`, the internal counter is advanced.
    pub fn next_bytes_fixed<const ARR_SIZE: usize>(&mut self, advance: bool) -> [u8; ARR_SIZE] {
        let view = &self.slice[self.offset..(self.offset + ARR_SIZE)];

        match advance {
            true => self.offset += ARR_SIZE,
            false => (),
        }

        view.try_into()
            .expect("slice and array should have the same length")
    }

    /// Find the next byte that matches and returns the offset.
    pub fn find_byte(&self, byte: u8) -> Option<usize> {
        if self.is_end() {
            return None;
        }

        self.curr_iter()
            .enumerate()
            .find_map(|(idx, b)| match *b == byte {
                true => Some(idx),
                false => None,
            })
    }

    /// Returns the number of duplicate bytes starting from the view at
    /// the current offset.
    pub fn num_duplicates(&mut self) -> usize {
        let view = self.curr_iter();

        let first = match self.slice.get(self.offset) {
            Some(f) => *f,
            None => return 0,
        };

        let res = view
            .scan(0_usize, |count, num| {
                *count += 1;

                match *num == first {
                    true => Some(*count),
                    false => None,
                }
            })
            .last()
            .expect("there will be at least one iteration (first element)");

        res
    }

    /// Returns the distance left from the current position in the view to the last element.
    pub fn distance_to_end(&self) -> usize {
        self.size - self.offset
    }

    /// Checks if the viewer has reached the end of the slice.
    pub fn is_end(&self) -> bool {
        self.offset == self.size
    }
}

/// Serializing and deserializing tests
#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fmt::Debug};

    use crate::RemoteMethodSignature;

    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Traditional {
        item: bool,
        number: i32,
        s: String,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Tuple((bool, String, u32));

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct NewType(bool);

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Unit;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum E {
        VUnit,
        VNewType(bool),
        VTuple((i32, bool)),
        VStruct { a: bool, b: i8, c: String },
        VNewTypeStruct(NewType),
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct AllNumeric {
        i8: i8,
        i16: i16,
        i32: i32,
        i64: i64,
        u8: u8,
        u16: u16,
        u32: u32,
        u64: u64,
        f32: f32,
        f64: f64,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct ContiguousBytes {
        s: String,
        c: char,
        b: Vec<u8>,
        v_nums: Vec<u32>,
    }

    /// All possible serde data types
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct AllTheThings {
        numbers: AllNumeric,
        byteslike: ContiguousBytes,
        optionals: (Option<E>, Option<E>), // one for each variant
        unit: (),
        unit_struct: Unit,
        newtype_struct: NewType,
        tup: (E, E, E, E, E),
        map: HashMap<String, u64>,
    }

    impl RemoteMethodSignature for AllTheThings {
        fn remote_method_signature() -> &'static [u8] {
            "AllTheThingsPayload".as_bytes()
        }
    }

    /// Performs a ser-de process
    fn ser_de_loop<T: Debug + PartialEq + Serialize + for<'a> Deserialize<'a>>(input: &T) {
        let ser = serialize(&input).unwrap();
        println!("serialized: {} - {:?}", ser.len(), ser);

        // let packed = pack_bytes(&ser);
        // println!("packed bytes: {} - {:?}", packed.len(), packed);
        // let unpacked = unpack_bytes(&packed);
        // println!("unpacked  : {} - {:?}", unpacked.len(), unpacked);

        // assert_eq!(ser, unpacked);

        println!("{:?}", std::str::from_utf8(&ser));
        let des: T = deserialize(&ser).unwrap();

        println!("{:?}", des);

        assert_eq!(*input, des)
    }

    fn ser_de_pack_loop<T: Debug + PartialEq + Serialize + for<'a> Deserialize<'a>>(input: &T) {
        let ser = serialize_packed(&input).unwrap();
        println!("serialized: {} - {:?}", ser.len(), ser);

        println!("{:?}", std::str::from_utf8(&ser));
        let des: T = deserialize_packed(&ser).unwrap();

        println!("{:?}", des);

        assert_eq!(*input, des)
    }

    fn ser_de_pack_header_loop<T>(input: &T)
    where
        T: Debug + PartialEq + Serialize + for<'a> Deserialize<'a> + RemoteMethodSignature,
    {
        let ser = serialize_packed_with_header(&input, T::remote_method_signature()).unwrap();
        println!("serialized: {} - {:?}", ser.len(), ser);

        let des: T = deserialize_packed_with_header(&ser, T::remote_method_signature()).unwrap();

        println!("{:?}", des);
        assert_eq!(*input, des);
    }

    #[test]
    fn test_ser_de_map() {
        let map: HashMap<String, i32> = HashMap::from([
            ("asd".to_string(), 10_000),
            ("how about that 👏👏👏".to_string(), 69),
        ]);

        ser_de_loop(&map);
        ser_de_pack_loop(&map);
    }

    /// Testing ser_de of sequences, like vectors and tuples
    #[test]
    fn test_ser_de_seq() {
        let seq = vec![100, 200, 300, 400];
        ser_de_loop(&seq);
        ser_de_pack_loop(&seq);
        let tup = (12, 100, 20000);
        ser_de_loop(&tup);
        ser_de_pack_loop(&tup);
        let tup_enum = (
            E::VStruct {
                a: true,
                b: 0,
                c: "first tuple element".to_string(),
            },
            E::VStruct {
                a: false,
                b: 120,
                c: "second tuple element".to_string(),
            },
        );
        ser_de_loop(&tup_enum);
        ser_de_pack_loop(&tup_enum);
    }

    /// Testing ser_de of structs
    #[test]
    fn test_ser_de_struct() {
        ser_de_loop(&Traditional {
            item: false,
            number: 10000,
            s: "asd".to_string(),
        });
        ser_de_loop(&Tuple((
            true,
            "how does serialization work?".to_string(),
            314159,
        )));
        ser_de_loop(&NewType(true));
        ser_de_pack_loop(&NewType(true));

        ser_de_loop(&Unit);
        ser_de_pack_loop(&Unit);
    }

    /// Testing ser_de of enum and various variants
    #[test]
    fn test_ser_de_enum() {
        ser_de_loop(&E::VUnit);
        ser_de_loop(&E::VNewType(false));
        ser_de_loop(&E::VTuple((10, true)));
        ser_de_loop(&E::VStruct {
            a: true,
            b: i8::MAX,
            c: "Hello How are You".to_string(),
        });

        ser_de_pack_loop(&E::VUnit);
        ser_de_pack_loop(&E::VNewType(false));
        ser_de_pack_loop(&E::VTuple((10, true)));
        ser_de_pack_loop(&E::VStruct {
            a: true,
            b: i8::MAX,
            c: "Hello How are You".to_string(),
        });
    }

    /// Test the serialization of ALL possible types
    #[test]
    fn test_ser_de_all() {
        let everything = AllTheThings {
            numbers: AllNumeric {
                i8: 100,
                i16: 1_000,
                i32: 1_000_000_000,
                i64: 1_000_000_000_000,

                u8: 100,
                u16: 1_000,
                u32: 1_000_000_000,
                u64: 1_000_000_000_000,

                f32: 3.14,
                f64: 1.414213562373095048801,
            },
            byteslike: ContiguousBytes {
                s: "this is data for an owned string".to_string(),
                c: 'c',
                b: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                v_nums: (0..10_u32).map(|num| num.pow(3)).collect(),
            },
            optionals: (Some(E::VUnit), None),
            unit: (),
            unit_struct: Unit,
            newtype_struct: NewType(false),
            tup: (
                E::VUnit,
                E::VNewType(true),
                E::VTuple((10_000, false)),
                E::VNewTypeStruct(NewType(true)),
                E::VStruct {
                    a: true,
                    b: 100,
                    c: "struct field".to_string(),
                },
            ),
            map: HashMap::from([("this".to_string(), 1500), ("that".to_string(), 5100)]),
        };

        ser_de_loop(&everything);
        ser_de_pack_loop(&everything);
        ser_de_pack_header_loop(&everything);
    }

    #[test]
    fn test_byte_viewer() {
        // sequence with 5 `6`s
        let sequence = vec![0, 1, 2, 3, 4, 5, 6, 6, 6, 6, 6, 5, 5];
        let mut viewer = ByteViewer::from_slice(&sequence);

        assert_eq!(viewer.distance_to_end(), sequence.len());

        let offset = viewer.find_byte(0).unwrap();
        viewer.advance(offset).unwrap();
        let num_dups = viewer.num_duplicates();
        println!("{}", num_dups);
        assert_eq!(1, num_dups); // 0 occurs once

        // find and advance to byte
        let offset = viewer.find_byte(6).unwrap();
        viewer.advance(offset).unwrap();

        let num_dups = viewer.num_duplicates();
        println!("{}", num_dups);
        assert_eq!(5, num_dups); // 6 occurs 5 times

        let offset = viewer.find_byte(100);
        assert!(matches!(offset, None));

        let dist_to_end = viewer.distance_to_end();
        let slice_to_end = viewer.next_bytes(dist_to_end, false); // this should not panic
        assert_eq!(dist_to_end, slice_to_end.len());
        viewer
            .advance(dist_to_end)
            .expect("we should have advanced straight to the end");

        assert!(viewer.is_end())
    }
}
