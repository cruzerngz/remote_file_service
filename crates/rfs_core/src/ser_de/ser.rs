//! Implementation of [serde::ser::Serializer] for [RfsSerializer]

use serde::ser;

/// This data structure contains the serialized bytes of any arbitrary data structure.
///
/// Structs/enums to be serialized need to derive [serde::Serialize].
pub struct RfsSerializer {
    output: Vec<u8>,
}

/// Impl serialize unsigned primitives
macro_rules! serialize_unsigned {
    ($fn_name: ident: $num_type: ty) => {
        fn $fn_name(self, v: $num_type) -> Result<Self::Ok, Self::Error> {
            self.serialize_u64(u64::from(v))
        }
    };
}

/// Impl serialize signed primitives
macro_rules! serialize_signed {
    ($fn_name: ident: $num_type: ty) => {
        fn $fn_name(self, v: $num_type) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(i64::from(v))
        }
    };
}

/// Writes the size of the byte slice and the data into a buffer
fn write_bytes(buffer: &mut Vec<u8>, bytes: &[u8]) {
    let len = bytes.len();
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
        todo!()
    }

    serialize_signed! {serialize_i8: i8}
    serialize_signed! {serialize_i16: i16}
    serialize_signed! {serialize_i32: i32}

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.output.extend(v.to_be_bytes());
        Ok(())
    }

    serialize_unsigned! {serialize_u8: u8}
    serialize_unsigned! {serialize_u16: u16}
    serialize_unsigned! {serialize_u32: u32}

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.output.extend(v.to_be_bytes());
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        unimplemented!("float serialization is not supported.")
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        unimplemented!("float serialization is not supported.")
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.output.push(v as u8);
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        let str_bytes = v.as_bytes();
        write_bytes(&mut self.output, str_bytes);

        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        write_bytes(&mut self.output, v);
        Ok(())
    }

    // none variants are serialized to 0b0000_0000
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(u8::MIN);
        Ok(())
    }

    // some variants are prefixed with 0b1111_1111
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        self.output.push(u8::MAX);
        value.serialize(self);

        Ok(())
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_none()
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_none()
    }

    // serialize the index of a unit variant
    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_u32(variant_index)
    }

    // serialize the inner value
    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
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
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        self.serialize_u32(variant_index);
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.output.push('[' as u8);
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_u64(len as u64);
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len);
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.serialize_u32(variant_index);
        self.serialize_tuple(len);
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        todo!()
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        todo!()
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        todo!()
    }
}

impl<'a> ser::SerializeSeq for &'a mut RfsSerializer {
    type Ok = ();

    type Error = crate::ser_de::err::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl<'a> ser::SerializeTuple for &'a mut RfsSerializer {
    type Ok = ();

    type Error = crate::ser_de::err::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut RfsSerializer {
    type Ok = ();

    type Error = crate::ser_de::err::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut RfsSerializer {
    type Ok = ();

    type Error = crate::ser_de::err::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl<'a> ser::SerializeMap for &'a mut RfsSerializer {
    type Ok = ();

    type Error = crate::ser_de::err::Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
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
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
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
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}
