use operit_link::{codec::appendLink, decodeLink, encodeLink, CoreValue};
use serde::{Serialize, Serializer};
use std::cell::Cell;

/// Preserves an existing transport header and encodes a large payload exactly once.
#[test]
fn framed_encoding_preserves_header_and_wire_representation() {
    let value = CoreValue::List(vec![
        CoreValue::Bytes(vec![0x5a; 1024 * 1024].into()),
        CoreValue::String("tail 中文".into()),
    ]);
    let mut bytes = vec![0, 8, 7, 6, 5, 4, 3, 2, 1];
    let header = bytes.clone();
    appendLink(&value, &mut bytes).unwrap();
    assert_eq!(&bytes[..9], header);
    assert_eq!(&bytes[9..], encodeLink(&value).unwrap());
    assert_eq!(decodeLink::<CoreValue>(&bytes[9..]).unwrap(), value);
}

/// Prevents a partially serialized frame from being transmitted after an error.
#[test]
fn failed_append_clears_the_header_and_partial_payload() {
    struct Fails;
    impl Serialize for Fails {
        /// Writes a sequence prefix before returning an intentional error.
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            use serde::ser::SerializeSeq;
            let mut sequence = serializer.serialize_seq(Some(2))?;
            sequence.serialize_element("partial")?;
            Err(serde::ser::Error::custom("intentional"))
        }
    }
    let mut bytes = vec![0; 9];
    assert!(appendLink(Fails, &mut bytes).is_err());
    assert!(bytes.is_empty());
}

/// Keeps source-capturing serializers from executing twice to size a frame.
#[test]
fn append_does_not_perform_a_serialization_sizing_pass() {
    struct Counted(Cell<u32>);
    impl Serialize for Counted {
        /// Counts observable serialization calls.
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.0.set(self.0.get() + 1);
            serializer.serialize_str("once")
        }
    }
    let value = Counted(Cell::new(0));
    let mut bytes = vec![0; 9];
    appendLink(&value, &mut bytes).unwrap();
    assert_eq!(value.0.get(), 1);
}
