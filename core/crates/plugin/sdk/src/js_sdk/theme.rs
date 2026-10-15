//! Named appearance configurations exposed to plugins.
use super::JsFuture;
use super::results::{ThemeListConfigsResultData, ThemeApplyConfigResultData};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Preserves one ordinary named appearance configuration from the canonical user-preference ledger.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ThemeConfig {
    pub id: String,
    pub name: String,
    /// Contains every key and exact value validated by the ordinary host appearance snapshot contract.
    pub snapshot: BTreeMap<String, serde_json::Value>,
    pub createdAt: i64,
    pub updatedAt: i64,
}
/// Reads and applies named themes through the canonical appearance manager.
pub trait ThemeHost: Send + Sync {
    /// Lists independent named appearance configurations.
    fn listConfigs(&self) -> JsFuture<ThemeListConfigsResultData>;
    /// Applies an exact named snapshot and active ID in the canonical transaction.
    fn applyConfig(&self, id: String) -> JsFuture<ThemeApplyConfigResultData>;
}
