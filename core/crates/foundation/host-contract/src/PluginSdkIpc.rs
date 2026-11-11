use std::sync::Arc;

use crate::{HostError, HostResult};

/// Names the well-known Plugin SDK IPC endpoint owned by Operit on this device.
pub const PLUGIN_SDK_IPC_ENDPOINT_NAME: &str = "operit.plugin.sdk";

/// Identifies one Plugin SDK IPC listener endpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginSdkIpcEndpoint {
    pub name: String,
}

impl PluginSdkIpcEndpoint {
    /// Returns the standard Plugin SDK IPC endpoint used by Operit and third-party clients.
    pub fn standard() -> Self {
        Self {
            name: PLUGIN_SDK_IPC_ENDPOINT_NAME.to_string(),
        }
    }
}

/// Identifies one connected Plugin SDK IPC session.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PluginSdkIpcSessionId(pub String);

impl PluginSdkIpcSessionId {
    /// Creates a session identifier from a host-assigned value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// Host-owned callbacks the runtime installs for Plugin SDK session lifecycle.
#[derive(Clone)]
pub struct PluginSdkIpcSessionCallbacks {
    pub onConnected: Arc<dyn Fn(PluginSdkIpcSessionId) + Send + Sync>,
    pub onMessage: Arc<dyn Fn(PluginSdkIpcSessionId, Vec<u8>) + Send + Sync>,
    pub onClosed: Arc<dyn Fn(PluginSdkIpcSessionId) + Send + Sync>,
}

impl PluginSdkIpcSessionCallbacks {
    /// Creates session callbacks for connect, framed receive, and close.
    pub fn new(
        onConnected: Arc<dyn Fn(PluginSdkIpcSessionId) + Send + Sync>,
        onMessage: Arc<dyn Fn(PluginSdkIpcSessionId, Vec<u8>) + Send + Sync>,
        onClosed: Arc<dyn Fn(PluginSdkIpcSessionId) + Send + Sync>,
    ) -> Self {
        Self {
            onConnected,
            onMessage,
            onClosed,
        }
    }
}

/// Platform IPC carrier that admits third-party Plugin SDK clients into Operit.
pub trait PluginSdkIpcHost: Send + Sync {
    /// Starts the Operit process that owns the Plugin SDK listener and waits until it is ready.
    fn activate(&self, endpoint: PluginSdkIpcEndpoint) -> HostResult<()>;

    /// Starts listening for third-party Plugin SDK connections on the given endpoint.
    fn startListener(
        &self,
        endpoint: PluginSdkIpcEndpoint,
        callbacks: PluginSdkIpcSessionCallbacks,
    ) -> HostResult<()>;

    /// Connects this process to an Operit Plugin SDK listener as a third-party client.
    fn connect(
        &self,
        endpoint: PluginSdkIpcEndpoint,
        callbacks: PluginSdkIpcSessionCallbacks,
    ) -> HostResult<PluginSdkIpcSessionId>;

    /// Sends one Link-encoded frame to a connected session.
    fn send(&self, sessionId: &PluginSdkIpcSessionId, bytes: Vec<u8>) -> HostResult<()>;

    /// Closes one connected session and ends its stream lifetime.
    fn closeSession(&self, sessionId: &PluginSdkIpcSessionId) -> HostResult<()>;

    /// Stops the Plugin SDK IPC listener and closes remaining listener-owned sessions.
    fn stopListener(&self) -> HostResult<()>;
}

/// Builds a host error for Plugin SDK IPC failures.
pub fn pluginSdkIpcError(message: impl Into<String>) -> HostError {
    HostError::new(message)
}
