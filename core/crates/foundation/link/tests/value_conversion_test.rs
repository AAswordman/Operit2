use operit_link::{CoreValue, decodeLink, encodeLink, fromCoreValue, toCoreValue};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Debug};

fn equivalent<T: Serialize + serde::de::DeserializeOwned + PartialEq + Debug>(value: T) {
    let wire = encodeLink(&value).unwrap();
    let legacy: CoreValue = decodeLink(&wire).unwrap();
    let direct = toCoreValue(&value).unwrap();
    assert_eq!(direct, legacy);
    assert_eq!(fromCoreValue::<T>(direct).unwrap(), value);
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Kind { Unit, Newtype(String), Tuple(u64, bool), Struct { name: String, data: Vec<u8> } }
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag="kind", content="data")]
enum Tagged { Empty, Value { text: String, nested: Option<Vec<Kind>> } }
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Unit;
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Newtype(Vec<Kind>);
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Payload {
    #[serde(with="serde_bytes")] bytes: Vec<u8>,
    signed: i64, unsigned: u64, huge: i128, float: f32, character: char,
    map: BTreeMap<String, Option<Vec<Kind>>>,
}
#[test]
fn direct_conversion_matches_wire_model() {
    equivalent(Kind::Unit);
    equivalent(Kind::Newtype("中文".into()));
    equivalent(Kind::Tuple(u64::MAX, true));
    equivalent(Kind::Struct { name:"n".into(), data:vec![0,127,255] });
    equivalent(Tagged::Empty);
    equivalent(Tagged::Value { text:"text".into(), nested:Some(vec![Kind::Unit]) });
    equivalent(Unit);
    equivalent(Newtype(vec![Kind::Unit]));
    equivalent(Payload { bytes: vec![0, 128, 255], signed:i64::MIN, unsigned:u64::MAX,
        huge:i128::MIN, float:1.25, character:'中', map:BTreeMap::from([("x".into(),Some(vec![Kind::Tuple(2,false)])),("y".into(),None)]) });
    equivalent(Some(42_i64));
    equivalent(None::<String>);
    equivalent(u128::MAX);
    equivalent((1_i8, -33_i16, 65536_i32, true, 1.5_f64));
}
#[test]
fn binary_payload_can_decode_as_byte_sequence() {
    let value=CoreValue::Bytes(vec![1,2,255].into());
    assert_eq!(fromCoreValue::<Vec<u8>>(value.clone()).unwrap(), decodeLink::<Vec<u8>>(&encodeLink(value).unwrap()).unwrap());
}
#[test]
fn integer_and_error_semantics_are_preserved() {
    assert_eq!(fromCoreValue::<i128>(CoreValue::Unsigned(u64::MAX)).unwrap(), u64::MAX as i128);
    assert!(fromCoreValue::<u8>(CoreValue::Unsigned(256)).is_err());
    assert!(fromCoreValue::<u64>(CoreValue::Signed(-1)).is_err());
    assert!(fromCoreValue::<(u8,u8)>(CoreValue::List(vec![CoreValue::Unsigned(1)])).is_err());
    assert!(fromCoreValue::<(u8,)>(CoreValue::List(vec![CoreValue::Unsigned(1),CoreValue::Unsigned(2)])).is_err());
    assert!(toCoreValue(BTreeMap::from([(1_u32,"invalid key")])).is_err());
}
