use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct MCPServerConfig {
    pub name: String,
    pub endpoint: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub extraData: BTreeMap<String, String>,
}
