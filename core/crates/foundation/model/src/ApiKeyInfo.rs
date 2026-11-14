use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiKeyAvailabilityStatus {
    UNTESTED,
    AVAILABLE,
    UNAVAILABLE,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub key: String,
    pub name: String,
    pub isEnabled: bool,
    pub availabilityStatus: ApiKeyAvailabilityStatus,
    pub usageCount: i64,
    pub lastUsed: i64,
    pub errorCount: i64,
}

impl ApiKeyInfo {
    /// Creates an enabled, untested pool entry without usage history.
    pub fn new(id: String, key: String) -> Self {
        Self {
            id,
            key,
            name: String::new(),
            isEnabled: true,
            availabilityStatus: ApiKeyAvailabilityStatus::UNTESTED,
            usageCount: 0,
            lastUsed: 0,
            errorCount: 0,
        }
    }
}

/// Returns enabled, nonempty entries that have not been explicitly marked unavailable.
pub fn candidate_keys(pool: &[ApiKeyInfo]) -> Vec<&ApiKeyInfo> {
    pool.iter()
        .filter(|entry| {
            entry.isEnabled
                && !entry.key.trim().is_empty()
                && entry.availabilityStatus != ApiKeyAvailabilityStatus::UNAVAILABLE
        })
        .collect()
}
