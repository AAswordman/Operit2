//! Direct Serde conversion matching the Link MessagePack data model.
use crate::CoreValue;
use serde::de::{self, IntoDeserializer, Visitor};
use serde::ser::{self, SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

type Error = de::value::Error;
pub(crate) fn to_value(value: impl Serialize) -> Result<CoreValue, Error> {
    value.serialize(ValueSerializer)
}
pub(crate) fn from_value<T: de::DeserializeOwned>(value: CoreValue) -> Result<T, Error> {
    T::deserialize(value)
}
fn error(message: &str) -> Error {
    de::Error::custom(message)
}
struct ValueSerializer;
impl Serializer for ValueSerializer {
    type Ok = CoreValue;
    type Error = Error;
    type SerializeSeq = Sequence;
    type SerializeTuple = Sequence;
    type SerializeTupleStruct = Sequence;
    type SerializeTupleVariant = Sequence;
    type SerializeMap = Mapping;
    type SerializeStruct = Mapping;
    type SerializeStructVariant = Mapping;
    fn is_human_readable(&self) -> bool {
        false
    }
    fn serialize_bool(self, v: bool) -> Result<CoreValue, Error> {
        Ok(CoreValue::Bool(v))
    }
    fn serialize_i64(self, v: i64) -> Result<CoreValue, Error> {
        Ok(if v >= 0 {
            CoreValue::Unsigned(v as u64)
        } else {
            CoreValue::Signed(v)
        })
    }
    fn serialize_u64(self, v: u64) -> Result<CoreValue, Error> {
        Ok(CoreValue::Unsigned(v))
    }
    fn serialize_f64(self, v: f64) -> Result<CoreValue, Error> {
        Ok(CoreValue::Float(v))
    }
    fn serialize_f32(self, v: f32) -> Result<CoreValue, Error> {
        self.serialize_f64(v as f64)
    }
    fn serialize_str(self, v: &str) -> Result<CoreValue, Error> {
        Ok(CoreValue::String(v.to_owned()))
    }
    fn serialize_char(self, v: char) -> Result<CoreValue, Error> {
        self.serialize_str(v.encode_utf8(&mut [0; 4]))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<CoreValue, Error> {
        Ok(CoreValue::Bytes(crate::CoreBytes::copy_from_slice(v)))
    }
    fn serialize_i128(self, v: i128) -> Result<CoreValue, Error> {
        self.serialize_bytes(&v.to_be_bytes())
    }
    fn serialize_u128(self, v: u128) -> Result<CoreValue, Error> {
        self.serialize_bytes(&v.to_be_bytes())
    }
    fn serialize_none(self) -> Result<CoreValue, Error> {
        Ok(CoreValue::Null)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<CoreValue, Error> {
        v.serialize(self)
    }
    fn serialize_unit(self) -> Result<CoreValue, Error> {
        Ok(CoreValue::Null)
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<CoreValue, Error> {
        Ok(CoreValue::List(vec![]))
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<CoreValue, Error> {
        self.serialize_str(variant)
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        v: &T,
    ) -> Result<CoreValue, Error> {
        if name == "_ExtStruct" {
            return Err(error("MessagePack extensions are not Link values"));
        }
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        v: &T,
    ) -> Result<CoreValue, Error> {
        Ok(wrap(Some(variant), v.serialize(self)?))
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Sequence, Error> {
        Ok(Sequence {
            values: Vec::with_capacity(len.unwrap_or(0)),
            variant: None,
        })
    }
    fn serialize_tuple(self, len: usize) -> Result<Sequence, Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(self, _: &'static str, len: usize) -> Result<Sequence, Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Sequence, Error> {
        Ok(Sequence {
            values: Vec::with_capacity(len),
            variant: Some(variant),
        })
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Mapping, Error> {
        Ok(Mapping {
            values: BTreeMap::new(),
            key: None,
            variant: None,
        })
    }
    fn serialize_struct(self, _: &'static str, len: usize) -> Result<Mapping, Error> {
        self.serialize_map(Some(len))
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Mapping, Error> {
        Ok(Mapping {
            values: BTreeMap::new(),
            key: None,
            variant: Some(variant),
        })
    }
    fn serialize_i8(self, v: i8) -> Result<CoreValue, Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_i16(self, v: i16) -> Result<CoreValue, Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_i32(self, v: i32) -> Result<CoreValue, Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_u8(self, v: u8) -> Result<CoreValue, Error> {
        self.serialize_u64(v as u64)
    }
    fn serialize_u16(self, v: u16) -> Result<CoreValue, Error> {
        self.serialize_u64(v as u64)
    }
    fn serialize_u32(self, v: u32) -> Result<CoreValue, Error> {
        self.serialize_u64(v as u64)
    }
}
fn wrap(variant: Option<&str>, value: CoreValue) -> CoreValue {
    match variant {
        None => value,
        Some(name) => CoreValue::Map(BTreeMap::from([(name.to_owned(), value)])),
    }
}
struct Sequence {
    values: Vec<CoreValue>,
    variant: Option<&'static str>,
}
impl SerializeSeq for Sequence {
    type Ok = CoreValue;
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        self.values.push(value.serialize(ValueSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<CoreValue, Error> {
        Ok(wrap(self.variant, CoreValue::List(self.values)))
    }
}
impl ser::SerializeTuple for Sequence {
    type Ok = CoreValue;
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<CoreValue, Error> {
        SerializeSeq::end(self)
    }
}
impl ser::SerializeTupleStruct for Sequence {
    type Ok = CoreValue;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<CoreValue, Error> {
        SerializeSeq::end(self)
    }
}
impl ser::SerializeTupleVariant for Sequence {
    type Ok = CoreValue;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<CoreValue, Error> {
        SerializeSeq::end(self)
    }
}
struct Mapping {
    values: BTreeMap<String, CoreValue>,
    key: Option<String>,
    variant: Option<&'static str>,
}
impl SerializeMap for Mapping {
    type Ok = CoreValue;
    type Error = Error;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Error> {
        match key.serialize(ValueSerializer)? {
            CoreValue::String(key) => {
                self.key = Some(key);
                Ok(())
            }
            _ => Err(error("Link map keys must be strings")),
        }
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        let key = self.key.take().ok_or_else(|| error("missing map key"))?;
        self.values.insert(key, value.serialize(ValueSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<CoreValue, Error> {
        Ok(wrap(self.variant, CoreValue::Map(self.values)))
    }
}
impl ser::SerializeStruct for Mapping {
    type Ok = CoreValue;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.values
            .insert(key.to_owned(), value.serialize(ValueSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<CoreValue, Error> {
        SerializeMap::end(self)
    }
}
impl ser::SerializeStructVariant for Mapping {
    type Ok = CoreValue;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.values
            .insert(key.to_owned(), value.serialize(ValueSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<CoreValue, Error> {
        SerializeMap::end(self)
    }
}

impl<'de> IntoDeserializer<'de, Error> for CoreValue {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self {
        self
    }
}
impl<'de> Deserializer<'de> for CoreValue {
    type Error = Error;
    fn is_human_readable(&self) -> bool {
        false
    }
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self {
            CoreValue::Null => visitor.visit_unit(),
            CoreValue::Bool(v) => visitor.visit_bool(v),
            CoreValue::Signed(v) if v >= 0 => visitor.visit_u64(v as u64),
            CoreValue::Signed(v) => visitor.visit_i64(v),
            CoreValue::Unsigned(v) => visitor.visit_u64(v),
            CoreValue::Float(v) => visitor.visit_f64(v),
            CoreValue::String(v) => visitor.visit_string(v),
            CoreValue::Bytes(v) => visitor.visit_bytes(&v),
            CoreValue::List(v) => {
                let mut seq = de::value::SeqDeserializer::new(v.into_iter());
                let result = visitor.visit_seq(&mut seq)?;
                seq.end()?;
                Ok(result)
            }
            CoreValue::Map(v) => {
                let mut map = de::value::MapDeserializer::new(v.into_iter());
                let result = visitor.visit_map(&mut map)?;
                map.end()?;
                Ok(result)
            }
        }
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self {
            CoreValue::Null => visitor.visit_none(),
            v => visitor.visit_some(v),
        }
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self {
            CoreValue::List(v) if v.is_empty() => visitor.visit_unit(),
            v => v.deserialize_any(visitor),
        }
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self {
            CoreValue::String(name) => {
                visitor.visit_enum(de::value::StringDeserializer::<Error>::new(name))
            }
            CoreValue::Map(v) if v.len() == 1 => {
                let (name, value) = v.into_iter().next().unwrap();
                visitor.visit_enum(ValueEnum(name, value))
            }
            _ => Err(error("expected enum string or single-entry map")),
        }
    }
    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self {
            CoreValue::Signed(v) => visitor.visit_i128(v as i128),
            CoreValue::Unsigned(v) => visitor.visit_i128(v as i128),
            CoreValue::Bytes(v) => visitor.visit_i128(i128::from_be_bytes(
                v.as_ref().try_into().map_err(|_| error("expected 16 bytes"))?,
            )),
            _ => Err(error("expected binary i128")),
        }
    }
    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self {
            CoreValue::Signed(v) => visitor.visit_u128(v as i128 as u128),
            CoreValue::Unsigned(v) => visitor.visit_u128(v as u128),
            CoreValue::Bytes(v) => visitor.visit_u128(u128::from_be_bytes(
                v.as_ref().try_into().map_err(|_| error("expected 16 bytes"))?,
            )),
            _ => Err(error("expected binary u128")),
        }
    }
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self {
            CoreValue::Bytes(v) => {
                let mut seq = de::value::SeqDeserializer::new(v.iter().copied());
                let result = visitor.visit_seq(&mut seq)?;
                seq.end()?;
                Ok(result)
            }
            v => v.deserialize_any(visitor),
        }
    }
    fn deserialize_tuple<V: Visitor<'de>>(self, _: usize, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }
    serde::forward_to_deserialize_any! { bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf unit map identifier ignored_any }
}
struct ValueEnum(String, CoreValue);
impl<'de> de::EnumAccess<'de> for ValueEnum {
    type Error = Error;
    type Variant = CoreValue;
    fn variant_seed<V: de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, CoreValue), Error> {
        Ok((
            seed.deserialize(de::value::StringDeserializer::<Error>::new(self.0))?,
            self.1,
        ))
    }
}
impl<'de> de::VariantAccess<'de> for CoreValue {
    type Error = Error;
    fn unit_variant(self) -> Result<(), Error> {
        Deserialize::deserialize(self)
    }
    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(self)
    }
    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_tuple(len, visitor)
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_struct("", fields, visitor)
    }
}
