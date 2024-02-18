//! Implementation of [serde::de::Deserializer] for [RfsDeserializer]

use serde::de::{self, EnumAccess, MapAccess, SeqAccess};

use crate::ser_de::consts;

use super::consts::ByteSizePrefix;

/// This data structure contains the serialized bytes of any arbitrary data structure.
///
/// Structs/enums to be deserialized need to derive [serde::Deserialize].
pub struct RfsDeserializer<'de> {
    input: ByteViewer<'de>,
}

impl<'de> RfsDeserializer<'de> {
    pub fn from_slice(s: &'de [u8]) -> Self {
        Self {
            input: ByteViewer::from_slice(s),
        }
    }
}

/// Impl deserialize signed primitives
macro_rules! deserialize_signed {
    ($fn_name: ident: $data_type: ty => $visitor_fn: ident) => {
        fn $fn_name<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            let bytes = self.input.next_bytes_fixed::<8>(true);
            visitor.$visitor_fn(i64::from_be_bytes(bytes) as $data_type)
        }
    };
}

/// Impl deserialize unsigned primitives
macro_rules! deserialize_unsigned {
    ($fn_name: ident: $data_type: ty => $visitor_fn: ident) => {
        fn $fn_name<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            let bytes = self.input.next_bytes_fixed::<8>(true);
            visitor.$visitor_fn(u64::from_be_bytes(bytes) as $data_type)
        }
    };
}

/// Validate the correctness of the next byte from [ByteViewer] and a reference.
///
/// Pass in the appropriate error to return when the bytes do not match
macro_rules! validate_bytes {
    ($viewer: expr, $known: path => $err: expr) => {
        let next_byte = $viewer.next_byte();
        match next_byte == $known {
            true => (),
            false => return Err($err),
        }
    };
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut RfsDeserializer<'de> {
    type Error = crate::ser_de::err::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("re-format data to be self-describing")
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let prefix = self.input.next_byte();

        match prefix == consts::PREFIX_BOOL {
            true => (),
            false => return Err(super::err::Error::PrefixNotMatched(format!("{}", prefix))),
        }

        let value = self.input.next_byte();

        match (value == consts::BOOL_TRUE, value == consts::BOOL_FALSE) {
            (false, false) => {
                // TODO: add new error variant, this variant should not be constructed here
                (Err(super::err::Error::PrefixNotMatched(
                    "unable to match boolean prefixes".to_string(),
                )))
            }
            (is_true, is_false) => {
                if is_true {
                    visitor.visit_bool(true)
                } else if is_false {
                    visitor.visit_bool(false)
                } else {
                    unimplemented!("branch should never be taken")
                }
            }
        }
    }

    deserialize_signed! {deserialize_i64: i64 => visit_i64}
    deserialize_signed! {deserialize_i32: i32 => visit_i32}
    deserialize_signed! {deserialize_i16: i16 => visit_i16}
    deserialize_signed! {deserialize_i8: i8 => visit_i8}

    deserialize_unsigned! {deserialize_u64: u64 => visit_u64}
    deserialize_unsigned! {deserialize_u32: u32 => visit_u32}
    deserialize_unsigned! {deserialize_u16: u16 => visit_u16}
    deserialize_unsigned! {deserialize_u8: u8 => visit_u8}

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unimplemented!("float serialization is not supported.")
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unimplemented!("float serialization is not supported.")
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let bytes = self.input.next_bytes_fixed::<4>(true);
        let char_num = u32::from_be_bytes(bytes);
        visitor.visit_char(
            char::from_u32(char_num)
                .expect("Deserialization of u32-chars should not fail. Check serialization logic."),
        )
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let prefix = self.input.next_byte();
        match prefix == consts::PREFIX_STR {
            true => (),
            false => return Err(super::err::Error::PrefixNotMatched(format!(""))),
        }

        let len = self.input.pop_size();
        let str_bytes = self.input.next_bytes(len as usize, true);

        visitor.visit_str(
            std::str::from_utf8(str_bytes)
                .expect("Deserialization of strings should not fail. Check serialization logic."),
        )
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let prefix = self.input.next_byte();
        match prefix == consts::PREFIX_STR {
            true => (),
            false => return Err(super::err::Error::PrefixNotMatched(format!(""))),
        }

        let len = self.input.pop_size();
        let str_bytes = self.input.next_bytes(len as usize, true);

        visitor.visit_string(
            std::str::from_utf8(str_bytes)
                .expect("Deserialization of strings should not fail. Check serialization logic.")
                .to_owned(),
        )
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let len = self.input.pop_size();
        let bytes = self.input.next_bytes(len as usize, true);

        visitor.visit_bytes(bytes)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let len = self.input.pop_size();
        let bytes = self.input.next_bytes(len as usize, true);

        visitor.visit_byte_buf(bytes.to_owned())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let opt_prefix = self.input.next_byte();

        match opt_prefix == consts::PREFIX_OPTIONAL {
            true => (),
            false => {
                return Err(super::err::Error::PrefixNotMatched(format!(
                    "expected option prefix ({:?}), found {:?}",
                    consts::PREFIX_OPTIONAL,
                    opt_prefix
                )))
            }
        }

        let variant = self.input.next_byte();

        match variant {
            consts::OPTION_NONE => visitor.visit_none(),
            consts::OPTION_SOME => visitor.visit_some(self),
            _ => unimplemented!("options only have two variants"),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let unit_prefix = self.input.next_byte();

        match unit_prefix == consts::PREFIX_UNIT {
            true => visitor.visit_unit(),
            false => Err(crate::ser_de::err::Error::PrefixNotMatched(format!(
                "expection unit prefix ({:?}), found {:?}",
                consts::PREFIX_UNIT,
                unit_prefix
            ))),
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // V::(self)
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        validate_bytes! {
            self.input, consts::PREFIX_SEQ => Self::Error::PrefixNotMatched(format!(""))
        }
        validate_bytes! {
            self.input, consts::SEQ_OPEN => Self::Error::DelimiterNotFound(consts::SEQ_OPEN as char)
        }

        let accessor = CollectionsAccessor::from_deserializer(self, consts::SEQ_CLOSE);
        let val = visitor.visit_seq(accessor);

        validate_bytes! {
            self.input, consts::SEQ_CLOSE => Self::Error::DelimiterNotFound(consts::SEQ_CLOSE as char)
        }

        val
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        validate_bytes! {
            self.input, consts::PREFIX_SEQ_CONST => Self::Error::PrefixNotMatched(format!(""))
        }
        validate_bytes! {
            self.input, consts::SEQ_CONST_OPEN => Self::Error::DelimiterNotFound(consts::SEQ_CONST_OPEN as char)
        }

        let accessor = CollectionsAccessor::from_deserializer(self, consts::SEQ_CONST_CLOSE);
        let val = visitor.visit_seq(accessor);

        validate_bytes! {
            self.input, consts::SEQ_CONST_CLOSE => Self::Error::DelimiterNotFound(consts::SEQ_CONST_CLOSE as char)
        }

        val
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        validate_bytes! {
            self.input, consts::PREFIX_MAP => Self::Error::PrefixNotMatched(format!(""))
        }
        validate_bytes! {
            self.input, consts::MAP_OPEN => Self::Error::DelimiterNotFound(
                consts::MAP_OPEN as char,
            )
        }

        let accessor = CollectionsAccessor::from_deserializer(self, consts::MAP_CLOSE);

        let val = visitor.visit_map(accessor);

        validate_bytes! {
            self.input, consts::MAP_CLOSE => Self::Error::DelimiterNotFound(
                consts::MAP_CLOSE as char,
            )
        }

        val
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let open_delim = self.input.next_byte();

        match open_delim == consts::MAP_OPEN {
            true => (),
            false => {
                return Err(crate::ser_de::err::Error::DelimiterNotFound(
                    consts::MAP_OPEN as char,
                ))
            }
        }

        todo!()
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }
}

/// A reference into an existing slice of bytes.
///
/// There are multiple accessor implementations here.
/// These will be used when deserializing collections or maps,
/// such as vectors, maps, structs and enums.
///
struct ByteViewer<'arr> {
    slice: &'arr [u8],
    size: usize,
    offset: usize,
}

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
    pub fn curr_iter(&self) -> std::slice::Iter<'arr, u8> {
        let curr_view = &self.slice[self.offset..];
        curr_view.iter()
    }

    /// Advance the view on the underlying slice.
    ///
    /// If the new offset is larger than the size of the slice,
    /// this will return an error.
    #[must_use]
    pub fn advance(&mut self, steps: usize) -> Result<(), ()> {
        match (self.offset + steps) < self.size {
            true => {
                self.offset += steps;

                Ok(())
            }
            false => Err(()),
        }
    }

    /// Peek at the next byte in the slice
    pub fn peek(&self) -> Option<&u8> {
        self.slice.get(self.offset)
    }

    /// Takes the next 8 bytes and parses them into a [u64].
    /// As most contiguous (variable len) collections have their sizes stored at the start,
    /// this is used to retrieve the size of the collection (in bytes) and advance the viewer.
    pub fn pop_size(&mut self) -> ByteSizePrefix {
        const NUM_BYTES: usize = std::mem::size_of::<ByteSizePrefix>();
        let size_bytes = self.next_bytes_fixed::<NUM_BYTES>(true);
        ByteSizePrefix::from_be_bytes(size_bytes)
    }

    /// Return the next byte and advance the view
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
}

/// This wrapper contains implementations for accessing collections.
struct CollectionsAccessor<'a, 'de: 'a> {
    des: &'a mut RfsDeserializer<'de>,
    // checks the immediate char for this terminating condition
    terminator: u8,
}

impl<'a, 'de> CollectionsAccessor<'a, 'de> {
    /// Create a new instance of the collections accessor
    pub fn from_deserializer(des: &'a mut RfsDeserializer<'de>, terminator: u8) -> Self {
        Self { des, terminator }
    }
}

impl<'a, 'de> SeqAccess<'de> for CollectionsAccessor<'a, 'de> {
    type Error = super::err::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        // stop at sequence boundary
        if self.des.input.peek() == Some(&self.terminator) {
            return Ok(None);
        }
        seed.deserialize(&mut *self.des).map(Some)
    }
}

impl<'a, 'de> MapAccess<'de> for CollectionsAccessor<'a, 'de> {
    type Error = super::err::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        unimplemented!("deserialization implemented in next_entry_seed()")
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        unimplemented!("deserialization implemented in next_entry_seed()")
    }

    fn next_entry_seed<K, V>(
        &mut self,
        kseed: K,
        vseed: V,
    ) -> Result<Option<(K::Value, V::Value)>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
        V: de::DeserializeSeed<'de>,
    {
        // stop at map boundary
        if self.des.input.peek() == Some(&self.terminator) {
            return Ok(None);
        }

        validate_bytes! {
            self.des.input, consts::MAP_ENTRY_OPEN => Self::Error::DelimiterNotFound(
                consts::MAP_ENTRY_OPEN as char,
            )
        }

        let key = kseed.deserialize(&mut *self.des)?;

        validate_bytes! {
            self.des.input, consts::MAP_ENTRY_MID => Self::Error::DelimiterNotFound(
                consts::MAP_ENTRY_MID as char,
            )
        }

        let val = vseed.deserialize(&mut *self.des)?;

        validate_bytes! {
            self.des.input, consts::MAP_ENTRY_CLOSE => Self::Error::DelimiterNotFound(
                consts::MAP_ENTRY_CLOSE as char,
            )
        }

        Ok(Some((key, val)))
    }
}
