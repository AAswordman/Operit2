pub mod client;
pub mod isolation;
pub mod session;
pub use session::CoreLinkSession;
pub mod codec;
mod operation_codec;
pub use operation_codec::{decodeLinkRequest, decodeLinkResponse, decodeLinkRequestOwned, decodeLinkResponseOwned};
pub use bytes::Bytes as CoreBytes;
#[path = "CoreStream.rs"]
mod core_stream;
pub mod protocol;
pub mod route_runtime;
mod value_codec;
mod buffer;
pub use buffer::CoreBuffer;

pub const LINK_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use client::{CoreLinkClient, CoreLinkPushSession, CoreLinkSharedClient};
pub use codec::{decodeLink, decodeLinkOwned, encodeLink, encodeLinkInto, encodedLinkSize, CoreLinkCodecError};
pub use core_stream::{
    withCoreStreamCapture, withCoreStreamCaptureSync, withCoreStreamSourceResolverSync, CoreStream,
    CoreStreamAttachment, CoreStreamDescriptor, CoreStreamSource,
};
pub use protocol::{
    fromCoreValue, toCoreValue, CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind,
    CoreEventStream, CoreLinkError, CoreLinkPushRequestMessage, CoreLinkRequest, CoreLinkResponse,
    CoreLinkWatchRequest, CoreLinkWatchResponse, CoreLinkPushResponse, CoreMethodMode, CoreMethodProtocol, CorePayloadKind, CorePushItem,
    CorePushRequest, CoreRequestId, CoreValue, CoreWatchInitial, CoreWatchRequest, LinkDeviceInfo,
    RoutedCoreRequest, RoutedCoreRequestKind,
    CORE_INCREMENTAL_VALUES_ARGUMENT, CORE_INTERNAL_TARGET, CORE_ROUTE_STREAM_SOURCE_ARGS_ARGUMENT,
    CORE_ROUTE_STREAM_SOURCE_METHOD_ARGUMENT, CORE_ROUTE_STREAM_SOURCE_MODE_ARGUMENT,
    CORE_STREAM_TARGET,
};
pub use route_runtime::{
    clearCoreRouteRuntime, coreForceLocal, coreRouteRuntime, coreRouteWatchSnapshot,
    installCoreRouteRuntime, nextCoreRouteRequestId, withCoreForceLocal, CoreRouteRuntime,
};
