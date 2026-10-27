//! Decode the existing operation envelope before entering its typed payload.
//! This reuses the constant-stack scanner instead of generating an additional
//! general-purpose MessagePack visitor for every adjacent-tagged enum.
#![allow(non_snake_case)]
use crate::{
    codec::{decodeLink, decodeOwnedWith, linkStructField},
    protocol::*,
    CoreBytes, CoreLinkCodecError,
};
use serde::{de::DeserializeOwned, Deserialize};

#[derive(Deserialize)]
enum Operation {
    Call,
    Watch,
    Push,
}
#[derive(Deserialize)]
enum WatchOperation {
    Snapshot,
    Open,
    Close,
}
#[derive(Deserialize)]
enum PushOperation {
    Open,
    Item,
    Close,
}

/// Selects the same named or positional fields used by Serde's tagged enums.
fn envelope<T: DeserializeOwned>(bytes: &[u8]) -> Result<(T, &[u8]), CoreLinkCodecError> {
    let required = |name, index| {
        linkStructField(bytes, name, index)?
            .ok_or_else(|| CoreLinkCodecError::Decode(format!("Missing Link {name}")))
    };
    let tag = decodeLink(required("type", 0)?)?;
    let body = required("body", 1)?;
    // Named enums allow unknown fields. Positional enums have exactly two slots.
    if matches!(bytes.first(), Some(0x90..=0x9f | 0xdc | 0xdd))
        && linkStructField(bytes, "", 2)?.is_some()
    {
        return Err(CoreLinkCodecError::Decode(
            "Extra Link envelope field".into(),
        ));
    }
    Ok((tag, body))
}

/// Serde's adjacent-tagged struct variants require a named body map.
fn requireVariantMap(body: &[u8]) -> Result<(), CoreLinkCodecError> {
    if matches!(body.first(), Some(0x80..=0x8f | 0xde | 0xdf)) {
        Ok(())
    } else {
        Err(CoreLinkCodecError::Decode(
            "Expected Link struct variant map".into(),
        ))
    }
}

/// Decodes a standard Call/Watch/Push request without expanding its envelope.
pub fn decodeLinkRequest(bytes: &[u8]) -> Result<CoreLinkRequest, CoreLinkCodecError> {
    let (operation, body) = envelope::<Operation>(bytes)?;
    Ok(match operation {
        Operation::Call => CoreLinkRequest::Call(decodeLink(body)?),
        Operation::Watch => {
            let (operation, body) = envelope::<WatchOperation>(body)?;
            CoreLinkRequest::Watch(match operation {
                WatchOperation::Snapshot => CoreLinkWatchRequest::Snapshot(decodeLink(body)?),
                WatchOperation::Open => CoreLinkWatchRequest::Open(decodeLink(body)?),
                WatchOperation::Close => {
                    requireVariantMap(body)?;
                    #[derive(Deserialize)]
                    struct Close {
                        requestId: CoreRequestId,
                    }
                    CoreLinkWatchRequest::Close {
                        requestId: decodeLink::<Close>(body)?.requestId,
                    }
                }
            })
        }
        Operation::Push => {
            let (operation, body) = envelope::<PushOperation>(body)?;
            CoreLinkRequest::Push(match operation {
                PushOperation::Open => CoreLinkPushRequestMessage::Open(decodeLink(body)?),
                PushOperation::Item => CoreLinkPushRequestMessage::Item(decodeLink(body)?),
                PushOperation::Close => {
                    requireVariantMap(body)?;
                    #[derive(Deserialize)]
                    struct Close {
                        pushId: String,
                    }
                    CoreLinkPushRequestMessage::Close {
                        pushId: decodeLink::<Close>(body)?.pushId,
                    }
                }
            })
        }
    })
}

/// Decodes each response body directly into its standard typed representation.
pub fn decodeLinkResponse(bytes: &[u8]) -> Result<CoreLinkResponse, CoreLinkCodecError> {
    let (operation, body) = envelope::<Operation>(bytes)?;
    Ok(match operation {
        Operation::Call => CoreLinkResponse::Call(decodeLink(body)?),
        Operation::Watch => {
            requireVariantMap(body)?;
            #[derive(Deserialize)]
            struct Watch {
                requestId: CoreRequestId,
                result: Result<CoreLinkWatchResponse, CoreLinkError>,
            }
            let Watch { requestId, result } = decodeLink(body)?;
            CoreLinkResponse::Watch { requestId, result }
        }
        Operation::Push => {
            requireVariantMap(body)?;
            #[derive(Deserialize)]
            struct Push {
                pushId: String,
                result: Result<CoreLinkPushResponse, CoreLinkError>,
            }
            let Push { pushId, result } = decodeLink(body)?;
            CoreLinkResponse::Push { pushId, result }
        }
    })
}

/// Retains binary request fields as slices of the authenticated frame.
pub fn decodeLinkRequestOwned(bytes: CoreBytes) -> Result<CoreLinkRequest, CoreLinkCodecError> {
    decodeOwnedWith(bytes, decodeLinkRequest)
}

/// Retains binary response fields as slices of the authenticated frame.
pub fn decodeLinkResponseOwned(bytes: CoreBytes) -> Result<CoreLinkResponse, CoreLinkCodecError> {
    decodeOwnedWith(bytes, decodeLinkResponse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encodeLink;
    use serde_json::json;

    /// Named, reordered and positional envelopes keep the existing Serde schema.
    #[test]
    fn requests_match_the_standard_decoder() {
        let calls = [
            json!({"type":"Call","body":{"requestId":"1","target":"test","methodName":"call","args":{"k":[1,true]}}}),
            json!({"type":"Watch","body":{"type":"Close","body":{"requestId":"2"}}}),
            json!({"type":"Push","body":{"type":"Close","body":{"pushId":"3"}}}),
        ];
        for value in calls {
            let standard = encodeLink(&value).unwrap();
            let expected: CoreLinkRequest = decodeLink(&standard).unwrap();
            assert_eq!(decodeLinkRequest(&standard).unwrap(), expected);
            // Body precedes type and an unrelated field is ignored, as before.
            let mut reordered = vec![0x83];
            for (key, value) in [
                ("body", &value["body"]),
                ("ignored", &json!(null)),
                ("type", &value["type"]),
            ] {
                reordered.extend(encodeLink(key).unwrap());
                reordered.extend(encodeLink(value).unwrap());
            }
            assert_eq!(decodeLinkRequest(&reordered).unwrap(), expected);
            let positional = encodeLink(json!([value["type"], value["body"]])).unwrap();
            assert_eq!(
                decodeLinkRequest(&positional).unwrap(),
                decodeLink::<CoreLinkRequest>(&positional).unwrap()
            );
        }
    }

    /// Invalid selectors, duplicate fields, trailing bytes and bad bodies still fail.
    #[test]
    fn malformed_envelopes_are_rejected() {
        for value in [
            json!({"type":"Wrong","body":{}}),
            json!({"type":"Call"}),
            json!({"body":null}),
            json!({"type":"Call","body":false}),
            json!(["Call", {}, 0]),
            json!({"type":"Watch","body":{"type":"Wrong","body":{}}}),
        ] {
            let bytes = encodeLink(value).unwrap();
            assert!(decodeLinkRequest(&bytes).is_err());
            assert!(decodeLinkResponse(&bytes).is_err());
        }
        let mut duplicate = vec![0x83];
        for (key, value) in [
            ("type", json!("Call")),
            ("body", json!({})),
            ("type", json!("Call")),
        ] {
            duplicate.extend(encodeLink(key).unwrap());
            duplicate.extend(encodeLink(value).unwrap());
        }
        assert!(decodeLinkRequest(&duplicate).is_err());
        let mut bytes = encodeLink(CoreLinkResponse::Call(CoreCallResponse::ok(
            CoreRequestId("1".into()),
            CoreValue::Null,
        )))
        .unwrap();
        for end in 0..bytes.len() {
            assert!(decodeLinkResponse(&bytes[..end]).is_err());
        }
        bytes.push(0xc0);
        assert!(decodeLinkResponse(&bytes).is_err());
    }

    /// The specialized path borrows binary bytes from the same owned source scope.
    #[test]
    fn owned_decoding_keeps_binary_ranges_shared() {
        let source: CoreBytes = encodeLink(CoreLinkResponse::Call(CoreCallResponse::ok(
            CoreRequestId("1".into()),
            CoreValue::Bytes(vec![7; 1024].into()),
        )))
        .unwrap()
        .into();
        let base = source.as_ptr() as usize;
        let end = base + source.len();
        let CoreLinkResponse::Call(CoreCallResponse {
            result: Ok(CoreValue::Bytes(bytes)),
            ..
        }) = decodeLinkResponseOwned(source).unwrap()
        else {
            panic!("binary response expected")
        };
        assert!((base..end).contains(&(bytes.as_ptr() as usize)));
        assert!(bytes.try_into_mut().is_ok());
    }
}
