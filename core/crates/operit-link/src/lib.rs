pub mod client;
pub mod codec;
#[cfg(not(target_arch = "wasm32"))]
pub mod http;
pub mod protocol;

pub const LINK_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use client::{CoreLinkClient, CoreLinkSharedClient};
pub use codec::{decodeCbor, decodeMessagePack, encodeCbor, encodeMessagePack, CoreLinkCodecError};
#[cfg(not(target_arch = "wasm32"))]
pub use http::{
    CoreLinkHttpDispatcher, CoreLinkWsPayload, CoreLinkWsResponse, LinkCallEnvelope,
    LinkWatchChannelCloseEnvelope, LinkWatchChannelEnvelope, LinkWatchChannelEvent,
    LinkWatchChannelOpenEnvelope, LinkWatchChannelOpenResponse, LinkWatchEnvelope,
};
pub use protocol::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkError,
    CoreMethodMode, CoreMethodProtocol, CoreObjectPath, CorePayloadKind, CoreRequestId, CoreValue,
    CoreWatchInitial, CoreWatchRequest,
};
