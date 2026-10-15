use crate::{ThemeValidationError, ValidationStage};
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::fmt;

pub(crate) const MAX_JSON_DEPTH: usize = 32;

/// serde_json::Value alone accepts duplicate object keys. This seed does not.
struct JsonSeed { depth: usize }

impl<'de> DeserializeSeed<'de> for JsonSeed {
    type Value = Value;

    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        if self.depth > MAX_JSON_DEPTH {
            return Err(de::Error::custom("JSON nesting exceeds 32 levels"));
        }
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for JsonSeed {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("strict theme JSON")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> { Ok(Value::Bool(value)) }
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> { Ok(value.into()) }
    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> { Ok(value.into()) }
    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value).map(Value::Number)
            .ok_or_else(|| de::Error::custom("non-finite JSON number"))
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> { Ok(value.into()) }
    fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> { Ok(value.into()) }
    fn visit_unit<E: de::Error>(self) -> Result<Value, E> { Ok(Value::Null) }
    fn visit_none<E: de::Error>(self) -> Result<Value, E> { Ok(Value::Null) }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element_seed(JsonSeed { depth: self.depth + 1 })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate JSON key: {key}")));
            }
            let value = map.next_value_seed(JsonSeed { depth: self.depth + 1 })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

pub(crate) fn parse(bytes: &[u8], limit: usize, path: &str) -> Result<Value, ThemeValidationError> {
    if bytes.len() > limit {
        return Err(ThemeValidationError::new("theme-input-budget-exceeded", ValidationStage::Input,
            path, format!("JSON exceeds {limit} bytes")));
    }
    let source = std::str::from_utf8(bytes)
        .map_err(|_| ThemeValidationError::contract(path, "JSON must be UTF-8"))?;
    let mut deserializer = serde_json::Deserializer::from_str(source);
    let value = JsonSeed { depth: 0 }.deserialize(&mut deserializer)
        .map_err(|e| ThemeValidationError::contract(path, e.to_string()))?;
    deserializer.end().map_err(|e| ThemeValidationError::contract(path, e.to_string()))?;
    Ok(value)
}
