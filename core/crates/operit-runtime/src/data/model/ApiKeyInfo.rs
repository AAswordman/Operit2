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
