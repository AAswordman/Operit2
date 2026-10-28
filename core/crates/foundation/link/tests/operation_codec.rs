//! Compatibility checks against Serde's existing standard Link representation.
#![allow(non_snake_case)]
use operit_link::*;

#[test]
fn all_request_lifecycles_preserve_named_and_positional_wire_values() {
    let watch = CoreWatchRequest {
        requestId: CoreRequestId::new("watch"),
        target: "test".into(),
        propertyName: "changed".into(),
        args: CoreValue::Bytes(vec![0, 1, 255].into()),
    };
    let requests = [
        CoreLinkRequest::Call(CoreCallRequest::new(
            "call",
            "test",
            "echo",
            watch.args.clone(),
        )),
        CoreLinkRequest::Watch(CoreLinkWatchRequest::Snapshot(watch.clone())),
        CoreLinkRequest::Watch(CoreLinkWatchRequest::Open(watch)),
        CoreLinkRequest::Watch(CoreLinkWatchRequest::Close {
            requestId: CoreRequestId::new("watch"),
        }),
        CoreLinkRequest::Push(CoreLinkPushRequestMessage::Open(CorePushRequest::new(
            "push", "test", "input",
        ))),
        CoreLinkRequest::Push(CoreLinkPushRequestMessage::Item(CorePushItem {
            pushId: "push".into(),
            sequence: 7,
            args: CoreValue::Null,
        })),
        CoreLinkRequest::Push(CoreLinkPushRequestMessage::Close {
            pushId: "push".into(),
        }),
    ];
    for request in requests {
        for bytes in [
            encodeLink(&request).unwrap(),
            rmp_serde::to_vec(&request).unwrap(),
        ] {
            match decodeLink::<CoreLinkRequest>(&bytes) {
                Ok(expected) => assert_eq!(decodeLinkRequest(&bytes).unwrap(), expected),
                Err(_) => assert!(
                    decodeLinkRequest(&bytes).is_err(),
                    "accepted a rejected request representation"
                ),
            }
            for end in 0..bytes.len() {
                assert!(decodeLinkRequest(&bytes[..end]).is_err());
            }
        }
    }
}

#[test]
fn successful_and_failed_responses_keep_the_existing_result_encoding() {
    let mut responses = vec![
        CoreLinkResponse::Call(CoreCallResponse::ok(
            CoreRequestId::new("call"),
            CoreValue::Bytes(vec![5; 32].into()),
        )),
        CoreLinkResponse::Call(CoreCallResponse::err(
            CoreRequestId::new("call"),
            CoreLinkError::internal("denied"),
        )),
    ];
    for result in [
        Ok(CoreLinkWatchResponse::Opened),
        Ok(CoreLinkWatchResponse::Closed),
        Err(CoreLinkError::internal("watch denied")),
    ] {
        responses.push(CoreLinkResponse::Watch {
            requestId: CoreRequestId::new("watch"),
            result,
        });
    }
    for result in [
        Ok(CoreLinkPushResponse::Opened),
        Ok(CoreLinkPushResponse::ItemAccepted { sequence: 5 }),
        Ok(CoreLinkPushResponse::Closed),
        Err(CoreLinkError::internal("push denied")),
    ] {
        responses.push(CoreLinkResponse::Push {
            pushId: "push".into(),
            result,
        });
    }
    for response in responses {
        for bytes in [
            encodeLink(&response).unwrap(),
            rmp_serde::to_vec(&response).unwrap(),
        ] {
            match decodeLink::<CoreLinkResponse>(&bytes) {
                Ok(expected) => assert_eq!(decodeLinkResponse(&bytes).unwrap(), expected),
                Err(_) => assert!(
                    decodeLinkResponse(&bytes).is_err(),
                    "accepted a rejected response representation"
                ),
            }
            for end in 0..bytes.len() {
                assert!(decodeLinkResponse(&bytes[..end]).is_err());
            }
        }
    }
}

#[test]
fn owned_request_binary_borrows_the_original_frame_and_releases_it_on_error() {
    let frame: CoreBytes = encodeLink(CoreLinkRequest::Call(CoreCallRequest::new(
        "binary",
        "test",
        "echo",
        CoreValue::Bytes(vec![8; 512].into()),
    )))
    .unwrap()
    .into();
    let start = frame.as_ptr() as usize;
    let end = start + frame.len();
    let CoreLinkRequest::Call(request) = decodeLinkRequestOwned(frame).unwrap() else {
        panic!("Call expected")
    };
    let CoreValue::Bytes(bytes) = request.args else {
        panic!("binary expected")
    };
    assert!((start..end).contains(&(bytes.as_ptr() as usize)));
    assert!(bytes.try_into_mut().is_ok());
    let invalid: CoreBytes = vec![0xc1].into();
    assert!(decodeLinkRequestOwned(invalid.clone()).is_err());
    assert!(invalid.try_into_mut().is_ok());
}
