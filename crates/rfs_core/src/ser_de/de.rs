//! Implementation of [serde::de::Deserializer] for [RfsDeserializer]

use serde::{
    de::{self, EnumAccess, MapAccess, SeqAccess, VariantAccess},
    Deserializer,
};

use crate::ser_de::consts;

use super::{consts::ByteSizePrefix, err, ByteViewer};

/// Custom deserializer. The counterpart to [RfsSerializer][crate::ser_de::ser::RfsSerializer].
///
/// This deserializer can deserialize the bytes of any data structure serialized using
/// it's associated serializer.
///
/// Structs/enums to be deserialized will need to derive [serde::Deserialize].
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
    ($fn_name: ident: $visitor_fn: ident, $conv_type: ty => $data_type: ty, $prefix: path) => {
        fn $fn_name<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            validate_next_byte! {
                self.input,
                $prefix
                => Self::Error::PrefixNotMatched($prefix)
            }

            // *all* numeric types serialize to 8 bytes
            const NUM_BYTES: usize = std::mem::size_of::<ByteSizePrefix>();

            require_bytes! {self.input, NUM_BYTES, err::Error::OutOfBytes};

            let bytes = self.input.next_bytes_fixed::<NUM_BYTES>(true);
            visitor.$visitor_fn(<$conv_type>::from_be_bytes(bytes) as $data_type)
        }
    };
}

/// Checks the byteviewer if it has sufficient bytes for the operation.
///
/// If there are insufficient bytes, return an error.
macro_rules! require_bytes {
    // for literals
    ($visitor_fn: expr, $num_bytes: literal, $error: path) => {
        if $visitor_fn.distance_to_end() < $num_bytes {
            return Err($error);
        }
    };

    // for idents
    ($visitor_fn: expr, $num_bytes: ident, $error: path) => {
        if $visitor_fn.distance_to_end() < $num_bytes {
            return Err($error);
        }
    };

    // for expressions
    ($visitor_fn: expr, $num_bytes: expr, $error: path) => {
        if $visitor_fn.distance_to_end() < $num_bytes {
            return Err($error);
        }
    };
}

/// Validate the correctness of the next byte from [ByteViewer] and a reference.
///
/// Pass in the appropriate error to return when the bytes do not match
macro_rules! validate_next_byte {
    ($viewer: expr, $known: path => $err: expr) => {
        require_bytes! {$viewer, 1, err::Error::OutOfBytes};

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
        // bools occupy 2 bytes
        require_bytes! {self.input, 2, err::Error::OutOfBytes};

        let prefix = self.input.next_byte();

        match prefix == consts::PREFIX_BOOL {
            true => (),
            false => return Err(super::err::Error::PrefixNotMatched(consts::PREFIX_BOOL)),
        }

        let value = self.input.next_byte();

        match (value == consts::BOOL_TRUE, value == consts::BOOL_FALSE) {
            (false, false) => {
                // TODO: add new error variant, this variant should not be constructed here
                Err(Self::Error::UnexpectedData {
                    exp: "u8::MAX or u8::MIN".to_string(),
                    have: value,
                })
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

    deserialize_numeric_primitive! {deserialize_i64: visit_i64, i64 => i64, consts::PREFIX_NUM}
    deserialize_numeric_primitive! {deserialize_i32: visit_i32, i64 => i32, consts::PREFIX_NUM}
    deserialize_numeric_primitive! {deserialize_i16: visit_i16, i64 => i16, consts::PREFIX_NUM}
    deserialize_numeric_primitive! {deserialize_i8: visit_i8, i64 => i8, consts::PREFIX_NUM}

    deserialize_numeric_primitive! {deserialize_u64: visit_u64, u64 => u64, consts::PREFIX_NUM}
    deserialize_numeric_primitive! {deserialize_u32: visit_u32, u64 => u32, consts::PREFIX_NUM}
    deserialize_numeric_primitive! {deserialize_u16: visit_u16, u64 => u16, consts::PREFIX_NUM}
    deserialize_numeric_primitive! {deserialize_u8: visit_u8, u64 => u8, consts::PREFIX_NUM}

    deserialize_numeric_primitive! {deserialize_f32: visit_f32, f64 => f32, consts::PREFIX_FLOAT}
    deserialize_numeric_primitive! {deserialize_f64: visit_f64 , f64 => f64, consts::PREFIX_FLOAT}

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // chars occupy 4 bytes
        require_bytes! {self.input, 4, err::Error::OutOfBytes};

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

        validate_next_byte! {
            self.input, consts::PREFIX_STR => Self::Error::PrefixNotMatched(consts::PREFIX_STR)
        }

        require_bytes! {self.input, 8, err::Error::OutOfBytes};
        let len = self.input.pop_size();

        require_bytes! {self.input, len as usize, err::Error::OutOfBytes};
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
        validate_next_byte! {
            self.input, consts::PREFIX_STR => Self::Error::PrefixNotMatched(consts::PREFIX_STR)
        }

        // let prefix = self.input.next_byte();
        // match prefix == consts::PREFIX_STR {
        //     true => (),
        //     false => return Err(super::err::Error::PrefixNotMatched(format!(""))),
        // }
        require_bytes! {self.input, 8, err::Error::OutOfBytes};
        let len = self.input.pop_size();

        require_bytes! {self.input, len as usize, err::Error::OutOfBytes};
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
        require_bytes! {self.input, 8, err::Error::OutOfBytes};
        let len = self.input.pop_size();

        require_bytes! {self.input, len as usize, err::Error::OutOfBytes};
        let bytes = self.input.next_bytes(len as usize, true);

        visitor.visit_bytes(bytes)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        validate_next_byte! {self.input, consts::PREFIX_BYTES => Self::Error::PrefixNotMatched(consts::PREFIX_BYTES)}

        require_bytes! {self.input, 8, err::Error::OutOfBytes};
        let len = self.input.pop_size();

        require_bytes! {self.input, len as usize, err::Error::OutOfBytes};
        let bytes = self.input.next_bytes(len as usize, true);

        visitor.visit_byte_buf(bytes.to_owned())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        validate_next_byte! {self.input, consts::PREFIX_OPTIONAL => Self::Error::PrefixNotMatched(consts::PREFIX_OPTIONAL) }

        require_bytes! {self.input, 1, err::Error::OutOfBytes};
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
        require_bytes! {self.input, 1, err::Error::OutOfBytes};
        let unit_prefix = self.input.next_byte();

        match unit_prefix == consts::PREFIX_UNIT {
            true => visitor.visit_unit(),
            false => Err(crate::ser_de::err::Error::PrefixNotMatched(
                consts::PREFIX_UNIT,
            )),
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
        validate_next_byte! {
            self.input, consts::PREFIX_SEQ => Self::Error::PrefixNotMatched(consts::PREFIX_SEQ)
        }
        validate_next_byte! {
            self.input, consts::SEQ_OPEN => Self::Error::DelimiterNotFound(consts::SEQ_OPEN )
        }

        let accessor = CollectionsAccessor::from_deserializer(self, consts::SEQ_CLOSE);
        let val = visitor.visit_seq(accessor);

        validate_next_byte! {
            self.input, consts::SEQ_CLOSE => Self::Error::DelimiterNotFound(consts::SEQ_CLOSE )
        }

        val
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // note that tuples and vecs have different delimiters
        validate_next_byte! {
            self.input, consts::PREFIX_SEQ_CONST => Self::Error::PrefixNotMatched(consts::PREFIX_SEQ_CONST)
        }
        validate_next_byte! {
            self.input, consts::SEQ_CONST_OPEN => Self::Error::DelimiterNotFound(consts::SEQ_CONST_OPEN )
        }

        let accessor = CollectionsAccessor::from_deserializer(self, consts::SEQ_CONST_CLOSE);
        let val = visitor.visit_seq(accessor);

        validate_next_byte! {
            self.input, consts::SEQ_CONST_CLOSE => Self::Error::DelimiterNotFound(consts::SEQ_CONST_CLOSE )
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
        validate_next_byte! {
            self.input, consts::PREFIX_MAP => Self::Error::PrefixNotMatched(consts::PREFIX_MAP)
        }
        validate_next_byte! {
            self.input, consts::MAP_OPEN => Self::Error::DelimiterNotFound(
                consts::MAP_OPEN ,
            )
        }

        let accessor = CollectionsAccessor::from_deserializer(self, consts::MAP_CLOSE);

        let val = visitor.visit_map(accessor);

        validate_next_byte! {
            self.input, consts::MAP_CLOSE => Self::Error::DelimiterNotFound(
                consts::MAP_CLOSE ,
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
        // println!("next byte: {} ({})", next, *next );

        validate_next_byte! {
            self.input, consts::PREFIX_ENUM => Self::Error::PrefixNotMatched(consts::PREFIX_ENUM)
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

        validate_next_byte! {
            self.des.input, consts::MAP_ENTRY_OPEN => Self::Error::DelimiterNotFound(
                consts::MAP_ENTRY_OPEN
            )
        }

        seed.deserialize(&mut *self.des).map(Some)

        // unimplemented!("deserialization implemented in next_entry_seed()")
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        validate_next_byte! {
            self.des.input, consts::MAP_ENTRY_MID => Self::Error::DelimiterNotFound(
                consts::MAP_ENTRY_MID
            )
        }

        let val = seed.deserialize(&mut *self.des);

        validate_next_byte! {
            self.des.input, consts::MAP_ENTRY_CLOSE => Self::Error::DelimiterNotFound(
                consts::MAP_ENTRY_CLOSE
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

        validate_next_byte! {
            self.des.input, consts::MAP_ENTRY_OPEN => Self::Error::DelimiterNotFound(
                consts::MAP_ENTRY_OPEN
            )
        }

        let key = kseed.deserialize(&mut *self.des)?;

        validate_next_byte! {
            self.des.input, consts::MAP_ENTRY_MID => Self::Error::DelimiterNotFound(
                consts::MAP_ENTRY_MID
            )
        }

        let val = vseed.deserialize(&mut *self.des)?;

        validate_next_byte! {
            self.des.input, consts::MAP_ENTRY_CLOSE => Self::Error::DelimiterNotFound(
                consts::MAP_ENTRY_CLOSE
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
