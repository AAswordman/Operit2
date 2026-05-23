use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::mcp::MCPServerConfig::MCPServerConfig;
use crate::data::mcp::plugins::MCPBridgeClient::MCPBridgeClient;

#[derive(Clone)]
pub struct MCPManager {
    inner: Arc<Mutex<MCPManagerState>>,
}

struct MCPManagerState {
    context: OperitApplicationContext,
    clientCache: BTreeMap<String, MCPBridgeClient>,
    serverConfigCache: BTreeMap<String, MCPServerConfig>,
    connectionFailureReasons: BTreeMap<String, String>,
}

static INSTANCE: OnceLock<Arc<Mutex<MCPManagerState>>> = OnceLock::new();

impl MCPManager {
    #[allow(non_snake_case)]
    pub fn getInstance(context: OperitApplicationContext) -> Self {
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

    #[allow(non_snake_case)]
    pub fn isServerRegistered(&self, serverName: &str) -> bool {
        self.inner
            .lock()
            .expect("mcp manager mutex poisoned")
            .serverConfigCache
            .contains_key(serverName)
    }

    #[allow(non_snake_case)]
    pub fn getRegisteredServers(&self) -> BTreeMap<String, MCPServerConfig> {
        self.inner
            .lock()
            .expect("mcp manager mutex poisoned")
            .serverConfigCache
            .clone()
    }

    #[allow(non_snake_case)]
    pub fn getLastConnectionFailureReason(&self, serverName: &str) -> Option<String> {
        self.inner
            .lock()
            .expect("mcp manager mutex poisoned")
            .connectionFailureReasons
            .get(serverName)
            .cloned()
    }

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
            guard.connectionFailureReasons.insert(serverName.to_string(), detail);
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
            guard.clientCache.insert(serverName.to_string(), client.clone());
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

    #[allow(non_snake_case)]
    pub fn registerServer(&self, serverName: String, serverConfig: MCPServerConfig) {
        let mut guard = self.inner.lock().expect("mcp manager mutex poisoned");
        guard.serverConfigCache.insert(serverName.clone(), serverConfig);
        guard.connectionFailureReasons.remove(&serverName);
        guard.clientCache.remove(&serverName);
    }

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

    #[allow(non_snake_case)]
    pub fn unregisterServer(&self, serverName: &str) {
        let mut guard = self.inner.lock().expect("mcp manager mutex poisoned");
        guard.serverConfigCache.remove(serverName);
        guard.connectionFailureReasons.remove(serverName);
        if let Some(client) = guard.clientCache.remove(serverName) {
            client.disconnect();
        }
    }

    #[allow(non_snake_case)]
    pub fn shutdown(&self) {
        let mut guard = self.inner.lock().expect("mcp manager mutex poisoned");
        for (_, client) in std::mem::take(&mut guard.clientCache) {
            client.disconnect();
        }
    }
}
