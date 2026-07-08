use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use operit_host_api::HostManager::HostManager;
use operit_tools::tools::mcp::MCPServerConfig::MCPServerConfig;
use crate::tools::mcp_runtime::plugins::MCPBridgeClient::MCPBridgeClient;

#[derive(Clone)]
pub struct MCPManager {
    inner: Arc<Mutex<MCPManagerState>>,
}

struct MCPManagerState {
    context: HostManager,
    clientCache: BTreeMap<String, MCPBridgeClient>,
    serverConfigCache: BTreeMap<String, MCPServerConfig>,
    connectionFailureReasons: BTreeMap<String, String>,
}

static INSTANCE: OnceLock<Arc<Mutex<MCPManagerState>>> = OnceLock::new();

impl MCPManager {
    /// Returns the shared MCP manager and updates its application context.
    #[allow(non_snake_case)]
    pub fn getInstance(context: HostManager) -> Self {
        let inner = INSTANCE
            .get_or_init(|| {
                Arc::new(Mutex::new(MCPManagerState {
                    context: context.clone(),
                    clientCache: BTreeMap::new(),
                    serverConfigCache: BTreeMap::new(),
                    connectionFailureReasons: BTreeMap::new(),
                }))
            })
            .clone();
        {
            let mut guard = inner.lock().expect("mcp manager mutex poisoned");
            guard.context = context;
        }
        Self { inner }
    }

    /// Returns whether a server name has been registered.
    #[allow(non_snake_case)]
    pub fn isServerRegistered(&self, serverName: &str) -> bool {
        self.inner
            .lock()
            .expect("mcp manager mutex poisoned")
            .serverConfigCache
            .contains_key(serverName)
    }

    /// Returns all registered MCP server configurations.
    #[allow(non_snake_case)]
    pub fn getRegisteredServers(&self) -> BTreeMap<String, MCPServerConfig> {
        self.inner
            .lock()
            .expect("mcp manager mutex poisoned")
            .serverConfigCache
            .clone()
    }

    /// Returns the most recent connection failure detail for a server.
    #[allow(non_snake_case)]
    pub fn getLastConnectionFailureReason(&self, serverName: &str) -> Option<String> {
        self.inner
            .lock()
            .expect("mcp manager mutex poisoned")
            .connectionFailureReasons
            .get(serverName)
            .cloned()
    }

    /// Returns a connected bridge client for a registered MCP server.
    #[allow(non_snake_case)]
    pub fn getOrCreateClient(&self, serverName: &str) -> Option<MCPBridgeClient> {
        let cached = {
            self.inner
                .lock()
                .expect("mcp manager mutex poisoned")
                .clientCache
                .get(serverName)
                .cloned()
        };
        if let Some(client) = cached {
            if client.isConnected() {
                return Some(client);
            }
            if client.connect() {
                self.inner
                    .lock()
                    .expect("mcp manager mutex poisoned")
                    .connectionFailureReasons
                    .remove(serverName);
                return Some(client);
            }
            let detail = client.getLastConnectionFailureDetail().unwrap_or_else(|| {
                "Reconnect attempt failed, but the client did not report a detailed reason."
                    .to_string()
            });
            let mut guard = self.inner.lock().expect("mcp manager mutex poisoned");
            guard
                .connectionFailureReasons
                .insert(serverName.to_string(), detail);
            guard.clientCache.remove(serverName);
        }

        let (context, hasConfig) = {
            let guard = self.inner.lock().expect("mcp manager mutex poisoned");
            (
                guard.context.clone(),
                guard.serverConfigCache.contains_key(serverName),
            )
        };
        if !hasConfig {
            self.inner
                .lock()
                .expect("mcp manager mutex poisoned")
                .connectionFailureReasons
                .insert(
                    serverName.to_string(),
                    "Server is not registered in MCPManager.".to_string(),
                );
            return None;
        }

        let client = MCPBridgeClient::new(context, serverName.to_string());
        if client.connect() {
            let mut guard = self.inner.lock().expect("mcp manager mutex poisoned");
            guard
                .clientCache
                .insert(serverName.to_string(), client.clone());
            guard.connectionFailureReasons.remove(serverName);
            return Some(client);
        }
        self.inner
            .lock()
            .expect("mcp manager mutex poisoned")
            .connectionFailureReasons
            .insert(
                serverName.to_string(),
                client.getLastConnectionFailureDetail().unwrap_or_else(|| {
                    "Connection attempt failed, but no detailed reason was reported.".to_string()
                }),
            );
        None
    }

    /// Registers or replaces an MCP server configuration.
    #[allow(non_snake_case)]
    pub fn registerServer(&self, serverName: String, serverConfig: MCPServerConfig) {
        let mut guard = self.inner.lock().expect("mcp manager mutex poisoned");
        guard
            .serverConfigCache
            .insert(serverName.clone(), serverConfig);
        guard.connectionFailureReasons.remove(&serverName);
        guard.clientCache.remove(&serverName);
    }

    /// Registers an MCP server from an endpoint URL and description.
    #[allow(non_snake_case)]
    pub fn registerServerFromEndpoint(
        &self,
        serverName: String,
        endpoint: String,
        description: String,
    ) {
        self.registerServer(
            serverName.clone(),
            MCPServerConfig {
                name: serverName,
                endpoint,
                description,
                capabilities: vec!["tools".to_string()],
                extraData: BTreeMap::new(),
            },
        );
    }

    /// Removes an MCP server and disconnects its cached client.
    #[allow(non_snake_case)]
    pub fn unregisterServer(&self, serverName: &str) {
        let mut guard = self.inner.lock().expect("mcp manager mutex poisoned");
        guard.serverConfigCache.remove(serverName);
        guard.connectionFailureReasons.remove(serverName);
        if let Some(client) = guard.clientCache.remove(serverName) {
            client.disconnect();
        }
    }

    /// Disconnects all cached MCP bridge clients.
    #[allow(non_snake_case)]
    pub fn shutdown(&self) {
        let mut guard = self.inner.lock().expect("mcp manager mutex poisoned");
        for (_, client) in std::mem::take(&mut guard.clientCache) {
            client.disconnect();
        }
    }
}
