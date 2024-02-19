//! Implementation of [serde::de::Deserializer] for [RfsDeserializer]

use serde::{
    de::{self, EnumAccess, MapAccess, SeqAccess, VariantAccess},
    Deserializer,
};

use crate::ser_de::consts;

use super::{consts::ByteSizePrefix, ByteViewer};

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

/// Impl deserialize for primitives
macro_rules! deserialize_numeric_primitive {
    ($fn_name: ident: $visitor_fn: ident, $conv_type: ty => $data_type: ty) => {
        fn $fn_name<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            validate_bytes! {
                self.input,
                consts::PREFIX_NUM
                => Self::Error::PrefixNotMatched(format!("numeric prefix not found"))
            }

            const NUM_BYTES: usize = std::mem::size_of::<ByteSizePrefix>();
            let bytes = self.input.next_bytes_fixed::<NUM_BYTES>(true);
            visitor.$visitor_fn(<$conv_type>::from_be_bytes(bytes) as $data_type)
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
        let prefix = self.input.peek();

        // only higher-level data-types are prefixed.
        // primitive types like `char` cannot be inferred.
        match prefix {
            Some(&consts::PREFIX_BOOL) => self.deserialize_bool(visitor),
            Some(&consts::PREFIX_BYTES) => self.deserialize_bytes(visitor),
            Some(&consts::PREFIX_ENUM) => unimplemented!("insufficient information"),
            Some(&consts::PREFIX_MAP) => self.deserialize_map(visitor),
            Some(&consts::PREFIX_NUM) => self.deserialize_u64(visitor),
            Some(&consts::PREFIX_OPTIONAL) => self.deserialize_option(visitor),
            Some(&consts::PREFIX_SEQ) => self.deserialize_seq(visitor),
            Some(&consts::PREFIX_SEQ_CONST) => unimplemented!("insufficient information"),
            Some(&consts::PREFIX_STR) => self.deserialize_str(visitor),
            Some(&consts::PREFIX_UNIT) => self.deserialize_unit(visitor),

            _ => Err(Self::Error::MalformedData),
        }

        // todo!("re-format data to be self-describing")
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
                Err(super::err::Error::PrefixNotMatched(
                    "unable to match boolean prefixes".to_string(),
                ))
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

    deserialize_numeric_primitive! {deserialize_i64: visit_i64, i64 => i64}
    deserialize_numeric_primitive! {deserialize_i32: visit_i32, i64 => i32}
    deserialize_numeric_primitive! {deserialize_i16: visit_i16, i64 => i16}
    deserialize_numeric_primitive! {deserialize_i8: visit_i8, i64 => i8}

    deserialize_numeric_primitive! {deserialize_u64: visit_u64, u64 => u64}
    deserialize_numeric_primitive! {deserialize_u32: visit_u32, u64 => u32}
    deserialize_numeric_primitive! {deserialize_u16: visit_u16, u64 => u16}
    deserialize_numeric_primitive! {deserialize_u8: visit_u8, u64 => u8}

    fn deserialize_f32<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unimplemented!("float serialization is not supported.")
    }

    fn deserialize_f64<V>(self, _: V) -> Result<V::Value, Self::Error>
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
        // let next = self.input.peek().unwrap();
        // println!("next byte: {} ({})", next, *next as char);

        validate_bytes! {
            self.input, consts::PREFIX_STR => Self::Error::PrefixNotMatched(format!("str prefix unable to be matched"))
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
        validate_bytes! {
            self.input, consts::PREFIX_STR => Self::Error::PrefixNotMatched(format!("string prefix unable to be matched"))
        }

        // let prefix = self.input.next_byte();
        // match prefix == consts::PREFIX_STR {
        //     true => (),
        //     false => return Err(super::err::Error::PrefixNotMatched(format!(""))),
        // }

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
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
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
        // note that vecs and tuples have different delimiters
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

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // note that tuples and vecs have different delimiters
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
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
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
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // structs and maps use the same underlying logic
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // let next = self.input.peek().unwrap();
        // println!("next byte: {} ({})", next, *next as char);

        validate_bytes! {
            self.input, consts::PREFIX_ENUM => Self::Error::PrefixNotMatched(format!("enum prefix unable to be matched"))
        }

        // let next = self.input.peek().unwrap();
        // println!("next byte: {} ({})", next, *next as char);

        let accessor = CollectionsAccessor::from_deserializer(self, 0);
        visitor.visit_enum(accessor)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // for now, identifiers are serialized directly
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unimplemented!()
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
        if self.des.input.peek() == Some(&self.terminator) {
            return Ok(None);
        }

        validate_bytes! {
            self.des.input, consts::MAP_ENTRY_OPEN => Self::Error::DelimiterNotFound(
                consts::MAP_ENTRY_OPEN as char,
            )
        }

        seed.deserialize(&mut *self.des).map(Some)

        // unimplemented!("deserialization implemented in next_entry_seed()")
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        validate_bytes! {
            self.des.input, consts::MAP_ENTRY_MID => Self::Error::DelimiterNotFound(
                consts::MAP_ENTRY_MID as char,
            )
        }

        let val = seed.deserialize(&mut *self.des);

        validate_bytes! {
            self.des.input, consts::MAP_ENTRY_CLOSE => Self::Error::DelimiterNotFound(
                consts::MAP_ENTRY_CLOSE as char,
            )
        }

        val

        // unimplemented!("deserialization implemented in next_entry_seed()")
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

impl<'a, 'de> EnumAccess<'de> for CollectionsAccessor<'a, 'de> {
    type Error = super::err::Error;

    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        // get the variant number
        // let variant_index = self.des.input.pop_size();

        let val = seed.deserialize(&mut *self.des)?;

        Ok((val, self))
    }
}

impl<'a, 'de> VariantAccess<'de> for CollectionsAccessor<'a, 'de> {
    type Error = super::err::Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.des)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.des.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.des.deserialize_map(visitor)
    }
}
