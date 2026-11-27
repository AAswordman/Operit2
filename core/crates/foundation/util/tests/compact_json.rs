use operit_util::SerializationSetup::{fromJsonValue, fromJsonValueRef};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, PartialEq)]
struct Record {
    id: u64,
    name: String,
    values: BTreeMap<u32, i64>,
    enabled: Option<bool>,
}

#[test]
fn selected_json_records_preserve_numbers_unicode_keys_and_absence() {
    let source = json!({"id": u64::MAX, "name":"猫 café", "values":{"12":i64::MIN}});
    let expected: Record = serde_json::from_value(source.clone()).unwrap();
    assert_eq!(fromJsonValue::<Record>(source.clone()).unwrap(), expected);
    assert_eq!(fromJsonValueRef::<Record>(&source).unwrap(), expected);
    assert_eq!(source["id"], json!(u64::MAX));
    assert_eq!(fromJsonValue::<Value>(source.clone()).unwrap(), source);
}

#[test]
fn invalid_selected_record_is_rejected_without_changing_the_source() {
    for source in [json!({"id":-1,"name":"cat","values":{}}),
        json!({"id":1,"name":null,"values":{}}),
        json!({"id":1,"name":"cat","values":{"bad":0}})] {
        let original = source.clone();
        assert!(serde_json::from_value::<Record>(source.clone()).is_err());
        assert!(fromJsonValueRef::<Record>(&source).is_err());
        assert!(fromJsonValue::<Record>(source.clone()).is_err());
        assert_eq!(source, original);
    }
}
