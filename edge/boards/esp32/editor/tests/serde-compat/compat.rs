//! Compare generated decoders against the pinned upstream derive crate.
#![cfg(test)]
#![allow(dead_code)]
use serde::de::DeserializeOwned;

macro_rules! fixture {
    ($module:ident, $derive:path) => { mod $module {
        #[derive(Debug, $derive)]
        #[serde(deny_unknown_fields)]
        pub struct Record {
            #[serde(alias = "identifier")]
            id: u64,
            #[serde(default)]
            name: String,
            enabled: Option<bool>,
        }
        #[derive(Debug, $derive)]
        pub struct Flat {
            id: u32,
            #[serde(flatten)]
            extra: std::collections::BTreeMap<String, serde_json::Value>,
        }
        #[derive(Debug, $derive)]
        pub enum Mode { Idle, Running { id: u32 }, Other(String) }
        #[derive(Debug, $derive)]
        #[serde(tag = "type", content = "body")]
        pub enum Tagged { Start { id: u32 }, Stop }
        #[derive(Debug, $derive)]
        #[serde(untagged)]
        pub enum Flexible { Number(i64), Text(String), Record { name: String } }
        #[derive(Debug, $derive)]
        pub struct Raw { value: Box<serde_json::value::RawValue> }
        #[derive(Debug, $derive)]
        pub struct Borrowed<'a> { pub name: &'a str, #[serde(borrow)] pub raw: &'a serde_json::value::RawValue }
        #[derive(Debug, $derive)]
        #[serde(field_identifier)]
        pub enum Field<'a> { Known, Other(&'a str) }
    }};
}
fixture!(reference, reference_derive::Deserialize);
fixture!(prepared, prepared_derive::Deserialize);

fn json<T: DeserializeOwned + std::fmt::Debug>(input: &str) -> Result<String, String> {
    serde_json::from_str::<T>(input).map(|v| format!("{v:?}")).map_err(|e| e.to_string())
}
fn pack<T: DeserializeOwned + std::fmt::Debug>(input: &[u8]) -> Result<String, String> {
    rmp_serde::from_slice::<T>(input).map(|v| format!("{v:?}")).map_err(|e| e.to_string())
}
macro_rules! compare {
    ($kind:ident, $input:expr) => {{
        let input = $input;
        assert_eq!(json::<prepared::$kind>(input), json::<reference::$kind>(input), "{input}");
        let value: serde_json::Value = serde_json::from_str(input).unwrap();
        let bytes = rmp_serde::to_vec_named(&value).unwrap();
        assert_eq!(pack::<prepared::$kind>(&bytes), pack::<reference::$kind>(&bytes), "{input}");
    }};
}
#[test]
fn named_fields_aliases_missing_fields_duplicates_and_unknown_fields_match() {
    for input in [r#"{"id":18446744073709551615,"name":"猫"}"#,
        r#"{"identifier":7,"enabled":false}"#, r#"{"id":1,"identifier":2}"#,
        r#"{"id":-1}"#, r#"{"name":"missing"}"#, r#"{"id":1,"unknown":2}"#,
        r#"{"id":1,"enabled":"invalid"}"#, r#"[1,"猫",null]"#] {
        compare!(Record, input);
    }
    assert_eq!(json::<prepared::Record>(r#"{"id":1,"id":2}"#), json::<reference::Record>(r#"{"id":1,"id":2}"#));
}
#[test]
fn flattened_content_and_enum_representations_match() {
    compare!(Flat, r#"{"id":1,"猫":{"x":[true,-2,null]}}"#);
    for input in [r#""Idle""#, r#"{"Running":{"id":7}}"#, r#"{"Other":"猫"}"#, r#""Unknown""#] { compare!(Mode, input); }
    for input in [r#"{"type":"Start","body":{"id":3}}"#, r#"{"type":"Stop"}"#, r#"{"type":"Unknown"}"#] { compare!(Tagged, input); }
    for input in ["-3", r#""猫""#, r#"{"name":"cat"}"#, "false"] { compare!(Flexible, input); }
    let input = r#"{"value": { "nested" : [1,true] }}"#;
    assert_eq!(json::<prepared::Raw>(input), json::<reference::Raw>(input));
}
#[test]
fn binary_field_names_and_invalid_utf8_match() {
    // MessagePack map with one binary key: id / unknown invalid UTF-8.
    for key in [b"id".as_slice(), b"identifier", b"unknown", &[255]] {
        let mut bytes = vec![0x81, 0xc4, key.len() as u8]; bytes.extend_from_slice(key); bytes.push(7);
        assert_eq!(pack::<prepared::Record>(&bytes), pack::<reference::Record>(&bytes));
    }
}

#[test]
fn borrowed_fields_raw_json_and_custom_identifier_fallbacks_match() {
    let input = r#"{"name":"cat", "raw": { "kept" : [1,true] }}"#;
    let expected: reference::Borrowed<'_> = serde_json::from_str(input).unwrap();
    let actual: prepared::Borrowed<'_> = serde_json::from_str(input).unwrap();
    assert_eq!(actual.name, expected.name);
    assert_eq!(actual.raw.get(), expected.raw.get());
    assert!(std::ptr::eq(actual.name.as_ptr(), expected.name.as_ptr()));
    for input in [r#""Known""#, r#""未知""#, r#""escaped\tkey""#] {
        assert_eq!(format!("{:?}", serde_json::from_str::<prepared::Field<'_>>(input)),
                   format!("{:?}", serde_json::from_str::<reference::Field<'_>>(input)));
    }
}
