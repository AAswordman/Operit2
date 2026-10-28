//! Manufacturer entry point. The default build has no dependencies.
//!
//! Local tools and agent backends work without a display or network. Enable
//! `space` for authenticated Operit pairing, membership and remote services.
//! Enable `scene` only when the product needs the optional scene renderer.
pub mod agent;
#[cfg(feature = "http-agent")]
pub mod http_agent;

#[cfg(feature = "space")]
pub use operit_edge_contract as contract;
#[cfg(feature = "scene")]
pub use operit_edge_scene as scene;
#[cfg(feature = "space")]
pub use operit_host_api as host;
#[cfg(feature = "space")]
pub use operit_link as link;
#[cfg(feature = "space")]
pub use operit_node_edge as device;
#[cfg(feature = "space")]
pub use operit_node_runtime as runtime;
#[cfg(feature = "space")]
pub use operit_proxy_edge as proxy;
#[cfg(feature = "space")]
pub use operit_store as storage;

/// Shared pairing and space components. Hosts provide sockets, storage and tasks.
#[cfg(feature = "space")]
pub mod space {
    pub use crate::space_device::SpaceDevice;
    pub use operit_node_edge::peer::createPeerService;
    pub use operit_node_edge::PeerRouter::EdgePeerRouter;
    pub use operit_node_runtime::HostRuntimePeerService::{
        HostRuntimePeerService, PeerRuntimeLimits,
    };
    pub use operit_node_runtime::NodeServices::NodeServices;
    pub use operit_node_runtime::NodeSpaceService::{NodeSpaceContext, NodeSpaceService};
    pub use operit_node_runtime::PeerStateStore::PeerStateStore;
    pub use operit_node_runtime::RuntimePeerService::RuntimePeerService;
}

#[cfg(feature = "space")]
mod space_device;
