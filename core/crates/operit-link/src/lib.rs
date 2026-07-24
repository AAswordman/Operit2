pub mod client;
pub mod codec;
#[cfg(not(target_arch = "wasm32"))]
pub mod http;
pub mod http_protocol;
pub mod protocol;

pub const LINK_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use client::{CoreLinkClient, CoreLinkSharedClient, CoreLinkTransportClient};
pub use codec::{decodeLink, encodeLink, CoreLinkCodecError};
#[cfg(not(target_arch = "wasm32"))]
pub use http::{
    CoreLinkHttpDispatcher, CoreLinkWsPayload, CoreLinkWsResponse,
};
pub use http_protocol::{
    LinkCallEnvelope,
    LinkPushCloseEnvelope, LinkPushCloseResponse, LinkPushItemResponse, LinkPushOpenEnvelope,
    LinkPushOpenResponse, LinkWatchChannelCloseEnvelope, LinkWatchChannelCloseResponse,
    LinkWatchChannelEnvelope, LinkWatchChannelEvent, LinkWatchChannelOpenEnvelope,
    LinkWatchChannelOpenResponse, LinkWatchEnvelope,
};
pub use protocol::{
    fromCoreValue, toCoreValue, CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind,
    CoreEventStream, CoreLinkError, CoreMethodMode, CoreMethodProtocol, CoreObjectPath,
    CorePayloadKind, CorePushItem, CorePushRequest, CoreRequestId, CoreValue, CoreWatchInitial,
    CoreWatchRequest,
};
