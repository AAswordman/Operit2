//! Exercises legacy numeric representations against every declared relational field.
use super::*;

#[test]
fn accepts_safe_integral_floats_and_preserves_exact_int64_values() {
    for integer in [
        i64::MIN,
        -9_007_199_254_740_993,
        -1,
        0,
        9_007_199_254_740_993,
        i64::MAX,
    ] {
        assert_eq!(integer_value(&json!(integer)).unwrap(), integer);
    }
    for number in [
        -9_007_199_254_740_991.0,
        -1.0,
        -0.0,
        0.0,
        1_791_650_275_159.0,
        9_007_199_254_740_991.0,
    ] {
        assert_eq!(integer_value(&json!(number)).unwrap(), number as i64);
    }
}

#[test]
fn rejects_fractional_unsafe_out_of_range_and_non_numeric_values() {
    for value in [
        json!(0.5),
        json!(-0.5),
        json!(9_007_199_254_740_992.0),
        json!(-9_007_199_254_740_992.0),
        json!(i64::MAX as f64),
        json!(i64::MIN as f64),
        json!(u64::MAX),
        json!(1e30),
        json!("1791650275159"),
        json!(true),
        Value::Null,
    ] {
        assert!(integer_value(&value).is_err(), "accepted {value}");
    }
}

/// Populates optional objects, arrays, maps, and nullable fields to cover nested integer leaves.
fn example(node: &Node, legacy: bool) -> Value {
    match node.kind.as_str() {
        "object" => Value::Object(
            node.fields
                .iter()
                .map(|(key, node)| (key.clone(), example(node, legacy)))
                .collect(),
        ),
        "array" => json!([example(node.item.as_ref().unwrap(), legacy)]),
        "map" => json!({"entry": example(node.item.as_ref().unwrap(), legacy)}),
        "integer" => {
            if legacy {
                json!(1_791_650_275_159.0)
            } else {
                json!(1_791_650_275_159_i64)
            }
        }
        "number" => json!(0.5),
        "vector" => json!([0.0, 0.5, 1.0]),
        "text" => json!("retained"),
        "boolean" => json!(true),
        kind => panic!("Unknown schema kind {kind}"),
    }
}

#[test]
fn every_collection_round_trips_legacy_float_integers_including_nested_fields() {
    let schema: Schema = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../plugins/packages/buildin/character_cards/src/storage/schema.json"
    )))
    .unwrap();
    for (name, node) in schema.collections {
        let expected = example(&node, false);
        let legacy = example(&node, true);
        let root = layout(name.clone(), node, false);
        let rows = encode(&root, "legacy", &legacy, "revision").unwrap();
        let actual = decode(&root, &rows[&name][0], &rows).unwrap();
        assert_eq!(actual, expected, "collection {name}");
    }
}

#[test]
fn invalid_integer_reports_record_and_field_without_rounding() {
    let node: Node = serde_json::from_value(json!({"kind":"object", "fields": {
        "record":{"kind":"object", "fields":{"createdAt":{"kind":"integer"}}}
    }}))
    .unwrap();
    let root = layout("cards".into(), node, false);
    let error = encode(
        &root,
        "default",
        &json!({"record":{"createdAt":1.5}}),
        "revision",
    )
    .unwrap_err();
    assert!(error.contains("cards/default.record__createdAt"), "{error}");
}
