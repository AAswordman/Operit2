pub mod client;
pub mod local;
pub mod protocol;
pub mod registry;

pub use client::CoreLinkClient;
pub use local::LocalCoreProxy;
pub use protocol::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreLinkError, CoreObjectPath,
    CoreRequestId, CoreValue, CoreWatchRequest,
};
pub use registry::{CoreMethodRegistry, CoreWatchRegistry};
