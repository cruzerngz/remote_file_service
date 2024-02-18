//! Implementation of [serde::de::Deserializer] for [RfsDeserializer]

use serde::de::{self, EnumAccess, MapAccess, SeqAccess};

use crate::ser_de::consts;

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
        let peeked = self.input.next_bytes(consts::BYTES_BOOL_FALSE.len(), false);

        match (
            peeked.starts_with(consts::BYTES_BOOL_TRUE),
            peeked.starts_with(consts::BYTES_BOOL_FALSE),
        ) {
            (false, false) => {
                (Err(super::err::Error::PrefixNotMatched(
                    "unable to match boolean prefixes".to_string(),
                )))
            }
            (is_true, is_false) => {
                self.input
                    .advance(consts::BYTES_BOOL_TRUE.len())
                    .expect("slice bounds should not be exceeded");

                if is_true {
                    visitor.visit_bool(true)
                } else if is_false {
                    visitor.visit_bool(false)
                } else {
                    unimplemented!("branch should never be taken")
                }
            }
        }
        // todo!()
    }

    deserialize_signed! {deserialize_i32: i32 => visit_i32}
    deserialize_signed! {deserialize_i16: i16 => visit_i16}
    deserialize_signed! {deserialize_i8: i8 => visit_i8}

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let bytes = self.input.next_bytes_fixed::<8>(true);
        visitor.visit_i64(i64::from_be_bytes(bytes))
    }

    deserialize_unsigned! {deserialize_u32: u32 => visit_u32}
    deserialize_unsigned! {deserialize_u16: u16 => visit_u16}
    deserialize_unsigned! {deserialize_u8: u8 => visit_u8}

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let bytes = self.input.next_bytes_fixed::<8>(true);
        visitor.visit_u64(u64::from_be_bytes(bytes))
    }

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
        let str_prefix = self.input.next_bytes(consts::BYTES_STR.len(), true);
        match str_prefix.starts_with(consts::BYTES_STR) {
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
        let str_prefix = self.input.next_bytes(consts::BYTES_STR.len(), true);
        match str_prefix.starts_with(consts::BYTES_STR) {
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
        let opt_prefix = self.input.next_bytes(consts::BYTES_OPTIONAL.len(), false);

        match opt_prefix.starts_with(consts::BYTES_OPTIONAL) {
            true => self.input.advance(consts::BYTES_OPTIONAL.len()).unwrap(),
            false => {
                return Err(super::err::Error::PrefixNotMatched(format!(
                    "expected option prefix ({:?}), found {:?}",
                    consts::BYTES_OPTIONAL,
                    opt_prefix
                )))
            }
        }

        let variant = self.input.next_byte();

        match variant {
            consts::OPTION_NONE_VARIANT => visitor.visit_none(),
            consts::OPTION_SOME_VARIANT => visitor.visit_some(self),
            _ => unimplemented!("options only have two variants"),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let unit_prefix = self.input.next_bytes(consts::BYTES_UNIT.len(), false);

        match unit_prefix.starts_with(consts::BYTES_UNIT) {
            true => {
                self.input.advance(consts::BYTES_UNIT.len());
                visitor.visit_unit()
            }
            false => Err(crate::ser_de::err::Error::PrefixNotMatched(format!(
                "expection unit prefix ({:?}), found {:?}",
                consts::BYTES_UNIT,
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
        todo!()
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
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
        let open_delim = self.input.next_byte();
        if open_delim != consts::MAP_OPEN {
            return Err(crate::ser_de::err::Error::DelimiterNotFound(
                consts::MAP_OPEN as char,
            ));
        }

        let accessor = CollectionsAccessor::from_deserializer(self);

        let val = visitor.visit_map(accessor);

        let close_delim = self.input.next_byte();
        if close_delim != consts::MAP_CLOSE {
            return Err(crate::ser_de::err::Error::DelimiterNotFound(
                consts::MAP_CLOSE as char,
            ));
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
    pub fn pop_size(&mut self) -> u64 {
        let u64_bytes = self.next_bytes_fixed::<8>(true);
        u64::from_be_bytes(u64_bytes)
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

    /// Returns a copy of the next slice of bytes as a fixed-size array
    /// and advance the internal counter.
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
}

impl<'a, 'de> CollectionsAccessor<'a, 'de> {
    /// Create a new instance of the collections accessor
    pub fn from_deserializer(des: &'a mut RfsDeserializer<'de>) -> Self {
        Self { des }
    }
}

impl<'a, 'de> SeqAccess<'de> for CollectionsAccessor<'a, 'de> {
    type Error = super::err::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        // stop
        if self.des.input.peek() == Some(&consts::SEQ_CLOSE) {
            self.des.input.advance(1).unwrap();
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
        // stop
        if self.des.input.peek() == Some(&consts::MAP_CLOSE) {
            return Ok(None);
        }

        let open_delim = self.des.input.next_byte();
        if open_delim != consts::MAP_ENTRY_OPEN {
            return Err(crate::ser_de::err::Error::DelimiterNotFound(
                consts::MAP_ENTRY_OPEN as char,
            ));
        }

        let key = kseed.deserialize(&mut *self.des)?;

        let mid_delim = self.des.input.next_byte();
        if mid_delim != consts::MAP_ENTRY_MID {
            return Err(crate::ser_de::err::Error::DelimiterNotFound(
                consts::MAP_ENTRY_MID as char,
            ));
        }

        let val = vseed.deserialize(&mut *self.des)?;

        let end_delim = self.des.input.next_byte();
        if end_delim != consts::MAP_ENTRY_CLOSE {
            return Err(crate::ser_de::err::Error::DelimiterNotFound(
                consts::MAP_ENTRY_CLOSE as char,
            ));
        }

        Ok(Some((key, val)))
    }
}
