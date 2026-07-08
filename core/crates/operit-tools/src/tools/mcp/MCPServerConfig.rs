use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MCPServerConfig {
    pub name: String,
    pub endpoint: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub extraData: BTreeMap<String, String>,
}
