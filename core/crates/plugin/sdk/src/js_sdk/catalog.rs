//! Canonical tool-source identities exposed to plugins.
use super::JsFuture;
use super::results::CatalogReadToolSourcesResultData;
use serde::{Deserialize, Serialize};

/// Identifies one actual configured tool source or registered built-in tool.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct CatalogToolSource {
    pub name: String,
    pub displayName: String,
    pub description: String,
}
/// Separates the actual canonical source registries rather than classifying names heuristically.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct CatalogToolSources {
    pub builtinTools: Vec<CatalogToolSource>,
    pub packages: Vec<CatalogToolSource>,
    pub skills: Vec<CatalogToolSource>,
    pub mcpServers: Vec<CatalogToolSource>,
}

/// Reads tool-source identities from their canonical registries.
pub trait CatalogHost: Send + Sync {
    /// Returns actual builtin, package, skill and MCP source identities.
    fn readToolSources(&self) -> JsFuture<CatalogReadToolSourcesResultData>;
}
