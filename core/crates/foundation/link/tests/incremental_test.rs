use std::collections::BTreeMap;
use operit_link::{CoreEventKind, CoreValue, encodeLink};

fn value(items: Vec<CoreValue>) -> CoreValue {
    CoreValue::Map(BTreeMap::from([
        ("unchanged".into(), CoreValue::String("背景内容".repeat(500))),
        ("items".into(), CoreValue::List(items)),
    ]))
}

#[test]
fn link_selects_and_reconstructs_successive_updates_without_business_flags() {
    let mut previous = None;
    let initial = value(vec![CoreValue::Unsigned(1), CoreValue::Unsigned(2)]);
    let (kind, mut received) = CoreValue::incrementalEvent(&mut previous, initial.clone());
    assert_eq!(kind, CoreEventKind::Snapshot);
    assert_eq!(received, initial);
    for current in [
        value(vec![CoreValue::Unsigned(9), CoreValue::Unsigned(2)]),
        value(vec![CoreValue::Unsigned(9)]),
        value(vec![CoreValue::Unsigned(9), CoreValue::Null]),
        value(vec![CoreValue::Unsigned(9), CoreValue::Null]),
    ] {
        let (kind, delta) = CoreValue::incrementalEvent(&mut previous, current.clone());
        assert_eq!(kind, CoreEventKind::Delta);
        assert!(encodeLink(&delta).unwrap().len() < encodeLink(&current).unwrap().len());
        received = received.applyIncrementalDelta(&delta).unwrap();
        assert_eq!(received, current);
        assert_eq!(previous.as_ref(), Some(&current));
    }
    let (kind, small) = CoreValue::incrementalEvent(&mut previous, CoreValue::Bool(true));
    assert_eq!(kind, CoreEventKind::Changed);
    assert_eq!(small, CoreValue::Bool(true));
}

#[test]
fn unchanged_fields_are_absent_from_patch() {
    let mut previous = Some(value(vec![CoreValue::Unsigned(1)]));
    let (_, delta) = CoreValue::incrementalEvent(&mut previous, value(vec![CoreValue::Unsigned(2)]));
    let CoreValue::Map(fields) = delta else { panic!("expected delta") };
    let CoreValue::List(operations) = &fields["$coreDelta"] else { panic!("expected operations") };
    assert_eq!(operations.len(), 1);
    let CoreValue::Map(operation) = &operations[0] else { panic!("expected operation") };
    assert_eq!(operation["path"], CoreValue::List(vec![CoreValue::String("items".into()), CoreValue::Unsigned(0)]));
}

#[test]
fn owned_delta_reuses_unchanged_storage_and_matches_borrowed_application() {
    let base = value(vec![CoreValue::Unsigned(1), CoreValue::Unsigned(2)]);
    let expected = value(vec![CoreValue::Unsigned(9), CoreValue::Unsigned(2)]);
    let mut previous = Some(base.clone());
    let (_, delta) = CoreValue::incrementalEvent(&mut previous, expected.clone());
    let CoreValue::Map(fields) = &base else { panic!("map") };
    let CoreValue::String(text) = &fields["unchanged"] else { panic!("string") };
    let allocation = text.as_ptr();
    assert_eq!(base.applyIncrementalDelta(&delta).unwrap(), expected);
    let result = base.intoIncrementalDelta(&delta).unwrap();
    assert_eq!(result, expected);
    let CoreValue::Map(fields) = &result else { panic!("map") };
    let CoreValue::String(text) = &fields["unchanged"] else { panic!("string") };
    assert_eq!(text.as_ptr(), allocation, "unchanged payload must not be cloned");
}

#[test]
fn borrowed_delta_keeps_original_on_a_later_invalid_operation() {
    let base = value(vec![CoreValue::Unsigned(1)]);
    let unchanged = base.clone();
    let operation = |path, value| CoreValue::Map(BTreeMap::from([
        ("op".into(), CoreValue::String("set".into())),
        ("path".into(), CoreValue::List(path)),
        ("value".into(), value),
    ]));
    let delta = CoreValue::Map(BTreeMap::from([("$coreDelta".into(), CoreValue::List(vec![
        operation(vec![CoreValue::String("items".into()), CoreValue::Unsigned(0)], CoreValue::Unsigned(9)),
        operation(vec![CoreValue::String("items".into()), CoreValue::Unsigned(99)], CoreValue::Null),
    ]))]));
    assert!(base.applyIncrementalDelta(&delta).is_err());
    assert_eq!(base, unchanged);
    assert!(base.intoIncrementalDelta(&delta).is_err());
}

#[test]
fn reusable_encoding_keeps_wire_bytes_and_reuses_buffer() {
    let mut output = Vec::with_capacity(8192);
    let allocation = output.as_ptr();
    for current in [value(vec![CoreValue::Unsigned(1)]), CoreValue::Null, value(vec![CoreValue::Unsigned(2)])] {
        operit_link::encodeLinkInto(&current, &mut output).unwrap();
        assert_eq!(output, encodeLink(&current).unwrap());
        assert_eq!(operit_link::decodeLink::<CoreValue>(&output).unwrap(), current);
        assert_eq!(output.as_ptr(), allocation);
    }
}

#[test]
fn reusable_encoding_clears_output_on_serialization_failure() {
    struct Fails;
    impl serde::Serialize for Fails {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("intentional failure"))
        }
    }
    let mut output = vec![1, 2, 3];
    assert!(operit_link::encodeLinkInto(Fails, &mut output).is_err());
    assert!(output.is_empty());
}

#[test]
fn reusable_encoding_serializes_side_effecting_values_exactly_once() {
    struct Counted(std::cell::Cell<u32>);
    impl serde::Serialize for Counted {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.0.set(self.0.get() + 1);
            serializer.serialize_str("once")
        }
    }
    let value = Counted(std::cell::Cell::new(0));
    let mut output = Vec::new();
    operit_link::encodeLinkInto(&value, &mut output).unwrap();
    assert_eq!(value.0.get(), 1);
}
