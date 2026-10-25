//! Reads Call correlation/control metadata without expanding its business payload.
use operit_link::{CoreLinkRequest, CoreLinkResponse, CoreRequestId};
use operit_peer_link::PeerMessage;
use serde::{Deserialize, Deserializer, de::{self, IgnoredAny, MapAccess, SeqAccess, Visitor}};
use super::StoredMessage;

pub(crate) struct CallHeader {
    pub requestId: CoreRequestId,
    pub request: bool,
    pub control: bool,
}
#[derive(Deserialize)]
#[serde(tag = "type", content = "body")]
enum Envelope { Call(CallHeader), Watch(IgnoredAny), Push(IgnoredAny) }
impl<'de> Deserialize<'de> for CallHeader {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct HeaderVisitor;
        impl<'de> Visitor<'de> for HeaderVisitor {
            type Value = CallHeader;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("a Link Call envelope") }
            fn visit_seq<A: SeqAccess<'de>>(self, mut fields: A) -> Result<Self::Value, A::Error> {
                let length = fields.size_hint();
                let requestId = fields.next_element()?.ok_or_else(|| de::Error::custom("Missing Call correlation"))?;
                let (request, control) = match length {
                    Some(4) => {
                        let target: String = fields.next_element()?.ok_or_else(|| de::Error::custom("Missing Call target"))?;
                        fields.next_element::<String>()?.ok_or_else(|| de::Error::custom("Missing Call method"))?;
                        fields.next_element::<IgnoredAny>()?.ok_or_else(|| de::Error::custom("Missing Call args"))?;
                        (true, target == "$peer.channel")
                    },
                    Some(2) => {
                        fields.next_element::<IgnoredAny>()?.ok_or_else(|| de::Error::custom("Missing Call result"))?;
                        (false, false)
                    },
                    _ => return Err(de::Error::custom("Invalid Call envelope length")),
                };
                if fields.next_element::<IgnoredAny>()?.is_some() { return Err(de::Error::custom("Extra Call field")); }
                Ok(CallHeader { requestId, request, control })
            }
            fn visit_map<A: MapAccess<'de>>(self, mut fields: A) -> Result<Self::Value, A::Error> {
                let (mut requestId, mut target, mut method, mut args, mut result) = (None, None, false, false, false);
                while let Some(key) = fields.next_key::<String>()? {
                    match key.as_str() {
                        "requestId" => { if requestId.is_some() { return Err(de::Error::duplicate_field("requestId")); } requestId = Some(fields.next_value()?); },
                        "target" => { if target.is_some() { return Err(de::Error::duplicate_field("target")); } target = Some(fields.next_value::<String>()?); },
                        "methodName" => { fields.next_value::<String>()?; method = true; },
                        "args" => { fields.next_value::<IgnoredAny>()?; args = true; },
                        "result" => { fields.next_value::<IgnoredAny>()?; result = true; },
                        _ => { fields.next_value::<IgnoredAny>()?; },
                    }
                }
                let requestId = requestId.ok_or_else(|| de::Error::missing_field("requestId"))?;
                let request = target.is_some() && method && args;
                if !request && !result { return Err(de::Error::custom("Incomplete Call envelope")); }
                Ok(CallHeader { requestId, request, control: request && target.as_deref() == Some("$peer.channel") })
            }
        }
        deserializer.deserialize_any(HeaderVisitor)
    }
}
impl StoredMessage<PeerMessage> {
    /// This is only a scheduling hint. The normal decoder still validates the
    /// complete Call before execution or before delivering its correlated result.
    pub(crate) fn callHeader(&self) -> Result<Option<CallHeader>, operit_link::CoreLinkError> {
        match self {
            Self::Value(PeerMessage::Request(CoreLinkRequest::Call(call))) => Ok(Some(CallHeader {
                requestId: call.requestId.clone(), request: true, control: call.target == "$peer.channel",
            })),
            Self::Value(PeerMessage::Response(CoreLinkResponse::Call(call))) => Ok(Some(CallHeader {
                requestId: call.requestId.clone(), request: false, control: false,
            })),
            Self::Value(_) => Ok(None),
            Self::Encoded(bytes) => match operit_link::decodeLink::<Envelope>(bytes) {
                Ok(Envelope::Call(header)) => Ok(Some(header)),
                Ok(_) => Ok(None),
                // Noncanonical envelopes retain the full decoder's compatibility.
                Err(_) => Ok(None),
            },
        }
    }
}
