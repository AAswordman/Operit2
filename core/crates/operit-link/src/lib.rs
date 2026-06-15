pub mod client;
pub mod protocol;
#[cfg(not(target_arch = "wasm32"))]
pub mod remote;

pub const LINK_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use client::CoreLinkClient;
pub use protocol::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkError,
    CoreMethodMode, CoreMethodProtocol, CoreObjectPath, CorePayloadKind, CoreRequestId, CoreValue,
    CoreWatchInitial, CoreWatchRequest,
};
#[cfg(not(target_arch = "wasm32"))]
pub use remote::{
    AcceptedRemoteSessionLoader, AcceptedRemoteSessionRecord, AcceptedRemoteSessionStore,
    PairFinishRequest, PairFinishResponse, PairStartRequest, PairStartResponse, PairStartState,
    PairedRemoteSession, PairedRemoteSessionRecord, RemoteDeviceInfo, RemoteHostInteractionBroker,
    RemoteHostInteractionPollEnvelope, RemoteHostInteractionPollResponse,
    RemoteHostInteractionRequest, RemoteHostInteractionRespondEnvelope, RemoteLinkClient,
    RemoteLinkServer, RemoteLinkServerConfig, RemotePairingCodeRecord, RemotePairingCodeSink,
    RemoteSessionInfoEnvelope, RemoteSessionInfoResponse, RemoteWebAccessConfig, RemoteWsEnvelope,
    RemoteWsPayload, RemoteWsResponse,
};
