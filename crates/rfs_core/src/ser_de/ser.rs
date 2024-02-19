//! Implementation of [serde::ser::Serializer] for [RfsSerializer]

use serde::{ser, Serialize};

use super::consts::{self, ByteSizePrefix};

/// This data structure contains the serialized bytes of any arbitrary data structure.
///
/// Structs/enums to be serialized need to derive [serde::Serialize].
pub struct RfsSerializer {
    pub(crate) output: Vec<u8>,
}

impl Default for RfsSerializer {
    fn default() -> Self {
        Self {
            output: Default::default(),
        }
    }
}

/// Impl serialize for primitives
macro_rules! serialize_numeric_primitive {
    ($fn_name: ident, $num_type: ty => $conv_type: ty) => {
        fn $fn_name(self, v: $num_type) -> Result<Self::Ok, Self::Error> {
            self.output.push(consts::PREFIX_NUM);
            self.output.extend((v as $conv_type).to_be_bytes());
            Ok(())
        }
    };
}

/// Writes the size of the byte slice and the data into a buffer.
///
/// The prefix is written first, then the length of the slice, then the slice.
fn write_bytes(buffer: &mut Vec<u8>, prefix: &[u8], bytes: &[u8]) {
    let len = bytes.len() as ByteSizePrefix;
    buffer.extend(prefix);
    buffer.extend(len.to_be_bytes());
    buffer.extend(bytes);
}

impl<'a> ser::Serializer for &'a mut RfsSerializer {
    type Ok = ();

    type Error = crate::ser_de::err::Error;

    type SerializeSeq = Self;

    type SerializeTuple = Self;

    type SerializeTupleStruct = Self;

    type SerializeTupleVariant = Self;

    type SerializeMap = Self;

    type SerializeStruct = Self;

    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.output.push(consts::PREFIX_BOOL);

        match v {
            true => self.output.push(u8::MAX),
            false => self.output.push(u8::MIN),
        }

        Ok(())
    }

    serialize_numeric_primitive! {serialize_i8, i8 => i64}
    serialize_numeric_primitive! {serialize_i16, i16 => i64}
    serialize_numeric_primitive! {serialize_i32, i32 => i64}
    serialize_numeric_primitive! {serialize_i64, i64 => i64}

    serialize_numeric_primitive! {serialize_u8, u8 => u64}
    serialize_numeric_primitive! {serialize_u16, u16 => u64}
    serialize_numeric_primitive! {serialize_u32, u32 => u64}
    serialize_numeric_primitive! {serialize_u64, u64 => u64}

    fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
        unimplemented!("float serialization is not supported.")
    }

    fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
        unimplemented!("float serialization is not supported.")
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.output.extend((v as u32).to_be_bytes());
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        let str_bytes = v.as_bytes();
        write_bytes(&mut self.output, &[consts::PREFIX_STR], str_bytes);

        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        write_bytes(&mut self.output, &[consts::PREFIX_BYTES], v);
        Ok(())
    }

    // none variants are serialized to 0b0000_0000
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(consts::PREFIX_OPTIONAL);
        self.output.push(consts::OPTION_NONE);
        Ok(())
    }

    // some variants are prefixed with 0b1111_1111
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        self.output.push(consts::PREFIX_OPTIONAL);
        self.output.push(consts::OPTION_SOME);
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(consts::PREFIX_UNIT);
        Ok(())
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    // serialize the index of a unit variant
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.output.push(consts::PREFIX_ENUM);
        variant.serialize(&mut *self)
    }

    // serialize the inner value
    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    // serialize the index, then the inner variant
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        self.output.push(consts::PREFIX_ENUM);
        // self.serialize_u32(variant_index)?;
        variant.serialize(&mut *self)?;
        value.serialize(&mut *self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.output.push(consts::PREFIX_SEQ);
        self.output.push(consts::SEQ_OPEN);
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.output.push(consts::PREFIX_SEQ_CONST);
        self.output.push(consts::SEQ_CONST_OPEN);
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)?;
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _len: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.output.push(consts::PREFIX_ENUM);
        variant.serialize(&mut *self)?;
        self.serialize_tuple(len)?;
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.output.push(consts::PREFIX_MAP);
        self.output.push(consts::MAP_OPEN);
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.output.push(consts::PREFIX_MAP);
        self.output.push(consts::MAP_OPEN);
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _len: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.output.push(consts::PREFIX_ENUM);
        variant.serialize(&mut *self)?;
        self.serialize_struct(name, len)
    }
}

impl<'a> ser::SerializeSeq for &'a mut RfsSerializer {
    type Ok = ();

    type Error = crate::ser_de::err::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(consts::SEQ_CLOSE);
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut RfsSerializer {
    type Ok = ();

    type Error = crate::ser_de::err::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(consts::SEQ_CONST_CLOSE);
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut RfsSerializer {
    type Ok = ();

    type Error = crate::ser_de::err::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        // uses [ser::SerializeTuple]
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut RfsSerializer {
    type Ok = ();

    type Error = crate::ser_de::err::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        // uses [ser::SerializeTuple]
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut RfsSerializer {
    type Ok = ();

    type Error = crate::ser_de::err::Error;

    fn serialize_entry<K: ?Sized, V: ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), Self::Error>
    where
        K: serde::Serialize,
        V: serde::Serialize,
    {
        // serialize key-value pairs as: <'key'-'val'>
        self.output.push(consts::MAP_ENTRY_OPEN);
        key.serialize(&mut **self)?;
        self.output.push(consts::MAP_ENTRY_MID);
        value.serialize(&mut **self)?;
        self.output.push(consts::MAP_ENTRY_CLOSE);

        Ok(())
    }

    fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        unimplemented!("use serialize_entry()")
    }

    fn serialize_value<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        unimplemented!("use serialize_entry()")
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(consts::MAP_CLOSE);
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &'a mut RfsSerializer {
    type Ok = ();

    type Error = crate::ser_de::err::Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        // same as map
        self.output.push(consts::MAP_ENTRY_OPEN);
        key.serialize(&mut **self)?;
        self.output.push(consts::MAP_ENTRY_MID);
        value.serialize(&mut **self)?;
        self.output.push(consts::MAP_ENTRY_CLOSE);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(consts::MAP_CLOSE);
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut RfsSerializer {
    type Ok = ();

    type Error = crate::ser_de::err::Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.output.push(consts::MAP_ENTRY_OPEN);
        key.serialize(&mut **self)?;
        self.output.push(consts::MAP_ENTRY_MID);
        value.serialize(&mut **self)?;
        self.output.push(consts::MAP_ENTRY_CLOSE);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(consts::MAP_CLOSE);

        Ok(())
    }
}
