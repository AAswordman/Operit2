pub mod client;
pub mod protocol;
pub mod remote;

pub const LINK_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use client::CoreLinkClient;
pub use protocol::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkError,
    CoreMethodMode, CoreMethodProtocol, CoreObjectPath, CorePayloadKind, CoreRequestId, CoreValue,
    CoreWatchInitial, CoreWatchRequest,
};
pub use remote::{
    PairedRemoteSession, PairedRemoteSessionRecord, PairFinishRequest, PairFinishResponse,
    PairStartRequest, PairStartResponse, PairStartState, RemoteHostInteractionBroker,
    RemoteHostInteractionPollEnvelope, RemoteHostInteractionPollResponse,
    RemoteHostInteractionRequest, RemoteHostInteractionRespondEnvelope, RemoteLinkClient,
    RemoteLinkServer, RemoteLinkServerConfig, RemoteSessionInfoEnvelope,
    RemoteSessionInfoResponse, RemoteWsEnvelope, RemoteWsPayload, RemoteWsResponse,
};
