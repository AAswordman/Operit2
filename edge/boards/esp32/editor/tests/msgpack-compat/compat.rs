//! Compare the board's overlay with the pinned, unmodified registry decoder.
#![cfg(test)]
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::io::Cursor;

#[derive(Debug, PartialEq)]
struct Numeric(String);
impl<'de> Deserialize<'de> for Numeric {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;
        macro_rules! integer {
            ($method:ident, $ty:ty) => {
                fn $method<E: serde::de::Error>(self, value: $ty) -> Result<Numeric, E> {
                    Ok(Numeric(format!("{}:{value}", stringify!($ty))))
                }
            };
        }
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Numeric;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result { f.write_str("a numeric token") }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Numeric, E> { Ok(Numeric("unit".into())) }
            integer!(visit_bool, bool);
            integer!(visit_u8, u8); integer!(visit_u16, u16);
            integer!(visit_u32, u32); integer!(visit_u64, u64);
            integer!(visit_i8, i8); integer!(visit_i16, i16);
            integer!(visit_i32, i32); integer!(visit_i64, i64);
            fn visit_f32<E: serde::de::Error>(self, value: f32) -> Result<Numeric, E> {
                Ok(Numeric(format!("f32:{:x}", value.to_bits())))
            }
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Numeric, E> {
                Ok(Numeric(format!("f64:{:x}", value.to_bits())))
            }
        }
        de.deserialize_any(Visitor)
    }
}

macro_rules! outcome {
    ($codec:ident, $bytes:expr, $ty:ty, $depth:expr) => {{
        let mut de = $codec::Deserializer::new(Cursor::new($bytes));
        de.set_max_depth($depth);
        let result = <$ty>::deserialize(&mut de).map(|v| format!("{v:?}"))
            .map_err(|e| format!("{e:?}"));
        (result, de.position())
    }};
}
fn compare(bytes: &[u8]) {
    macro_rules! check {
        ($ty:ty) => {
            assert_eq!(outcome!(prepared, bytes, $ty, 8), outcome!(reference, bytes, $ty, 8),
                "{} input={bytes:02x?}", stringify!($ty));
            let prepared = prepared::from_slice::<$ty>(bytes).map(|v| format!("{v:?}"))
                .map_err(|e| format!("{e:?}"));
            let reference = reference::from_slice::<$ty>(bytes).map(|v| format!("{v:?}"))
                .map_err(|e| format!("{e:?}"));
            assert_eq!(prepared, reference, "slice {} input={bytes:02x?}", stringify!($ty));
        };
    }
    check!(Numeric);
    check!(bool);
    check!(u64);
    check!(i64);
    check!(f32);
    check!(f64);
    check!(serde_json::Value);
    check!(Vec<u8>);
    check!(serde_bytes::ByteBuf);
    check!(Option<(u64, String)>);
}

#[test]
fn every_marker_and_truncated_payload_matches_reference() {
    for marker in 0..=255u8 {
        for fill in [0, 1, 0x7f, 0x80, 0xff] {
            let mut bytes = vec![marker];
            bytes.extend([fill; 20]);
            for end in 1..=bytes.len() { compare(&bytes[..end]); }
        }
    }
}

#[test]
fn deterministic_malformed_inputs_match_reference() {
    let mut seed = 0x55ad81f0u32;
    for length in 0..96 {
        for _ in 0..32 {
            let bytes: Vec<u8> = (0..length).map(|_| {
                seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5;
                seed as u8
            }).collect();
            compare(&bytes);
        }
    }
}

#[derive(Debug, PartialEq, serde::Serialize, Deserialize)]
enum Example { Unit, Newtype(u64), Tuple(String, Vec<u8>), Struct { enabled: bool, version: i64 } }
#[test]
fn structured_enums_options_and_borrowed_buffers_match_reference() {
    let records = [Example::Unit, Example::Newtype(u64::MAX),
        Example::Tuple("任务猫\n\"".into(), vec![0, 127, 128, 255]),
        Example::Struct { enabled: true, version: i64::MIN }];
    for record in records {
        for bytes in [reference::to_vec(&record).unwrap(), reference::to_vec_named(&record).unwrap()] {
            assert_eq!(outcome!(prepared, &bytes, Example, 8), outcome!(reference, &bytes, Example, 8));
            let result: Example = prepared::from_slice(&bytes).unwrap();
            assert_eq!(result, record);
        }
    }
    #[derive(Debug, Deserialize, PartialEq)]
    struct Borrowed<'a> { name: &'a str, #[serde(borrow, with = "serde_bytes")] bytes: &'a [u8] }
    let encoded = [0x92, 0xa3, b'c', b'a', b't', 0xc4, 3, 0, 128, 255];
    let ours: Borrowed = prepared::from_slice(&encoded).unwrap();
    let upstream: Borrowed = reference::from_slice(&encoded).unwrap();
    assert_eq!(ours, upstream);
    assert_eq!(ours.name.as_ptr(), upstream.name.as_ptr());
    assert_eq!(ours.bytes.as_ptr(), upstream.bytes.as_ptr());
}

#[test]
fn depth_limits_and_invalid_utf8_match_reference() {
    for count in 1..20 {
        let mut bytes = vec![0x91; count]; bytes.push(0xc0);
        for limit in 1..22 {
            assert_eq!(outcome!(prepared, &bytes, serde_json::Value, limit),
                outcome!(reference, &bytes, serde_json::Value, limit), "depth={limit}, nesting={count}");
        }
    }
    for bytes in [&[0xa1, 0xff][..], &[0xd9, 1, 0xff], &[0xda, 0, 1, 0xff]] { compare(bytes); }
}

#[derive(Debug, PartialEq, serde::Serialize, Deserialize)]
#[serde(rename = "_ExtStruct")]
struct Extension((i8, serde_bytes::ByteBuf));

#[test]
fn extension_tags_payloads_and_truncation_match_reference() {
    for length in [1, 2, 3, 4, 8, 16, 32, 256, 65536] {
        let ext = Extension((-17, serde_bytes::ByteBuf::from(vec![0xa7; length])));
        let bytes = reference::to_vec(&ext).unwrap();
        assert_eq!(prepared::to_vec(&ext).unwrap(), bytes);
        assert_eq!(outcome!(prepared, &bytes, Extension, 8), outcome!(reference, &bytes, Extension, 8));
        for end in [0, 1, 2, bytes.len() / 2, bytes.len() - 1] {
            assert_eq!(outcome!(prepared, &bytes[..end], Extension, 8),
                outcome!(reference, &bytes[..end], Extension, 8));
        }
    }
}
