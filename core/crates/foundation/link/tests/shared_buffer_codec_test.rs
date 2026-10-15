use operit_link::{CoreBytes, CoreValue, decodeLink, encodeLink};
use operit_link::codec::{captureLinkBuffers, withLinkBuffers};

/// Keeps both wire size and binary ownership independent of the payload length.
#[test]
fn shared_roundtrip_preserves_the_original_allocation() {
    for length in [0, 65536, 1048576, 16777216] {
        let source: CoreBytes = vec![0xa7; length].into();
        let value = CoreValue::List(vec![CoreValue::Bytes(source.clone()), CoreValue::Unsigned(7)]);
        let (wire, buffers) = captureLinkBuffers(|| encodeLink(&value).unwrap());
        assert_eq!(wire.len(), 29);
        assert_eq!(buffers.len(), 1);
        assert_eq!(buffers[0].as_ptr(), source.as_ptr());
        assert!(decodeLink::<CoreValue>(&wire).is_err());
        let decoded = withLinkBuffers(buffers.into(), || decodeLink::<CoreValue>(&wire).unwrap());
        let CoreValue::List(values) = decoded else { panic!("list expected") };
        let CoreValue::Bytes(bytes) = &values[0] else { panic!("bytes expected") };
        assert_eq!(bytes.as_ptr(), source.as_ptr());
        assert_eq!(bytes.len(), length);
        assert_eq!(values[1], CoreValue::Unsigned(7));
    }
}

/// Validates every reference bound before creating a shared slice.
#[test]
fn malformed_shared_ranges_and_missing_context_are_rejected() {
    let source: CoreBytes = vec![1, 2, 3, 4].into();
    let (wire, buffers) = captureLinkBuffers(|| encodeLink(CoreValue::Bytes(source)).unwrap());
    for (position, value) in [(2, 0x41), (10, 1), (18, 5), (26, 5), (3, 0xff)] {
        let mut invalid = wire.clone();
        invalid[position] = value;
        assert!(withLinkBuffers(buffers.clone().into(), || decodeLink::<CoreValue>(&invalid)).is_err());
    }
    let mut sliced = wire;
    sliced[18] = 1;
    sliced[26] = 2;
    let decoded = withLinkBuffers(buffers.clone().into(), || decodeLink::<CoreValue>(&sliced).unwrap());
    let CoreValue::Bytes(bytes) = decoded else { panic!("bytes expected") };
    assert_eq!(bytes.as_ref(), &[2, 3]);
    assert_eq!(bytes.as_ptr(), unsafe { buffers[0].as_ptr().add(1) });
}

/// Restores an enclosing capture after an inner serializer panics.
#[test]
fn nested_capture_and_unwinding_do_not_leak_transport_context() {
    let source = CoreValue::Bytes(vec![1, 2].into());
    let (_, outer) = captureLinkBuffers(|| {
        let _ = encodeLink(&source).unwrap();
        let outcome = std::panic::catch_unwind(|| captureLinkBuffers(|| {
            let _ = encodeLink(&source).unwrap();
            panic!("intentional serializer failure");
        }));
        assert!(outcome.is_err());
        let _ = encodeLink(&source).unwrap();
    });
    assert_eq!(outer.len(), 2);
    assert_eq!(encodeLink(&source).unwrap(), vec![0xc4, 2, 1, 2]);
}
