pub mod client;
pub mod codec;
#[path = "CoreStream.rs"]
mod core_stream;
pub mod protocol;
pub mod route_runtime;

pub const LINK_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use client::{CoreLinkClient, CoreLinkPushSession, CoreLinkSharedClient};
pub use codec::{decodeLink, encodeLink, CoreLinkCodecError};
pub use core_stream::{
    withCoreStreamCapture, withCoreStreamCaptureSync, withCoreStreamSourceResolverSync, CoreStream,
    CoreStreamAttachment, CoreStreamDescriptor, CoreStreamSource,
};
pub use protocol::{
    fromCoreValue, toCoreValue, CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind,
    CoreEventStream, CoreExecutionSegment, CoreHandoffRequest, CoreHandoffResponse, CoreLinkError,
    CoreMethodMode, CoreMethodProtocol, CorePayloadKind, CorePushItem, CorePushRequest,
    CoreRequestId, CoreValue, CoreWatchInitial, CoreWatchRequest, CORE_INCREMENTAL_VALUES_ARGUMENT,
    CORE_INTERNAL_ROUTE_OBJECT_ID, CORE_ROUTE_STREAM_SOURCE_ARGS_ARGUMENT,
    CORE_ROUTE_STREAM_SOURCE_METHOD_ARGUMENT, CORE_ROUTE_STREAM_SOURCE_MODE_ARGUMENT,
    CORE_STREAM_POOL_OBJECT_ID,
};
pub use route_runtime::{
    clearCoreRouteRuntime, coreForceLocal, coreRouteRuntime, installCoreRouteRuntime,
    nextCoreRouteRequestId, withCoreForceLocal, CoreRouteRuntime,
};
