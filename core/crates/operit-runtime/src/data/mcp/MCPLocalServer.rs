use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::packTool::PackageManager::CachedMcpToolInfo;

pub struct MCPLocalServer;

static CACHED_TOOLS: OnceLock<Mutex<BTreeMap<String, Vec<CachedMcpToolInfo>>>> =
    OnceLock::new();

impl MCPLocalServer {
    #[allow(non_snake_case)]
    pub fn getInstance(_context: &OperitApplicationContext) -> Self {
        Self
    }

    #[allow(non_snake_case)]
    pub fn getCachedTools(&self, serverId: &str) -> Option<Vec<CachedMcpToolInfo>> {
        cachedTools()
            .lock()
            .expect("mcp local server mutex poisoned")
            .get(serverId)
            .cloned()
    }

    #[allow(non_snake_case)]
    pub fn cacheServerTools(&self, serverId: String, tools: Vec<CachedMcpToolInfo>) {
        cachedTools()
            .lock()
            .expect("mcp local server mutex poisoned")
            .insert(serverId, tools);
    }
}

#[allow(non_snake_case)]
fn cachedTools() -> &'static Mutex<BTreeMap<String, Vec<CachedMcpToolInfo>>> {
    CACHED_TOOLS.get_or_init(|| Mutex::new(BTreeMap::new()))
}
