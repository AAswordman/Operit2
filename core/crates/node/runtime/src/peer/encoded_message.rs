//! Owns queued wire bytes without reserving space for expanded Link values.
use super::error;
use operit_link::{decodeLink, encodeLink, CoreLinkCodecError, CoreLinkError, CoreRequestId};
use operit_peer_link::PeerMessage;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Keeps queued messages compact until their actual consumer needs the typed value.
pub(crate) struct EncodedMessage<T> {
    bytes: Vec<u8>,
    value: PhantomData<fn() -> T>,
}

impl<T> EncodedMessage<T> {
    /// Takes ownership of an existing authenticated or locally encoded frame.
    pub(crate) fn fromBytes(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            value: PhantomData,
        }
    }

    /// Transfers the encoded frame without decoding or copying its payload.
    pub(crate) fn intoBytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Borrows the frame for direct typed routing without expanding its value tree.
    pub(crate) fn bytesMut(&mut self) -> &mut Vec<u8> {
        &mut self.bytes
    }

    /// Reads the declared direction without expanding Watch or Push payloads.
    pub(crate) fn isResponse(&self) -> Result<bool, CoreLinkError> {
        let body = operit_link::codec::linkMapField(&self.bytes, "body")
            .map_err(|failure| error(failure.to_string()))?
            .ok_or_else(|| error("Missing Link body"))?;
        operit_link::codec::linkMapField(body, "result")
            .map(|result| result.is_some())
            .map_err(|failure| error(failure.to_string()))
    }

    /// Decodes the frame only at its execution or consumption boundary.
    pub(crate) fn decodeWith(
        self,
        decode: impl FnOnce(&[u8]) -> Result<T, CoreLinkCodecError>,
    ) -> Result<T, CoreLinkError> {
        decode(&self.bytes).map_err(|failure| error(failure.to_string()))
    }
}

impl<T: Serialize> EncodedMessage<T> {
    /// Serializes a locally produced value once before transferring its ownership.
    pub(crate) fn new(value: T) -> Result<Self, CoreLinkError> {
        encodeLink(value)
            .map(Self::fromBytes)
            .map_err(|failure| error(failure.to_string()))
    }
}

/// Carries only the fields required for Call scheduling and heartbeat control.
pub(crate) struct CallHeader {
    pub requestId: CoreRequestId,
    pub request: bool,
    pub control: bool,
}

#[derive(Deserialize)]
enum Operation {
    Call,
    Watch,
    Push,
}

impl EncodedMessage<PeerMessage> {
    /// Scans exact wire fields without recursively decoding the business payload.
    pub(crate) fn callHeader(&self) -> Result<Option<CallHeader>, CoreLinkError> {
        let operation: Operation = decodeLink(self.field(&self.bytes, "type")?)
            .map_err(|failure| error(failure.to_string()))?;
        if !matches!(operation, Operation::Call) {
            return Ok(None);
        }
        let body = self.field(&self.bytes, "body")?;
        let requestId = decodeLink(self.field(body, "requestId")?)
            .map_err(|failure| error(failure.to_string()))?;
        let response = operit_link::codec::linkMapField(body, "result")
            .map_err(|failure| error(failure.to_string()))?
            .is_some();
        let control = if response {
            false
        } else {
            let target: String = decodeLink(self.field(body, "target")?)
                .map_err(|failure| error(failure.to_string()))?;
            target == "$peer.channel"
        };
        Ok(Some(CallHeader {
            requestId,
            request: !response,
            control,
        }))
    }

    /// Requires a declared map field using the shared constant-stack wire scanner.
    fn field<'a>(&self, bytes: &'a [u8], name: &str) -> Result<&'a [u8], CoreLinkError> {
        operit_link::codec::linkMapField(bytes, name)
            .map_err(|failure| error(failure.to_string()))?
            .ok_or_else(|| error(format!("Missing Link {name}")))
    }
}
