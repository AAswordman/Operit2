use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Serialize)]
pub struct LocalizedText {
    pub values: HashMap<String, String>,
}

impl<'de> Deserialize<'de> for LocalizedText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if let Some(text) = value.as_str() {
            return Ok(LocalizedText {
                values: HashMap::from([("default".to_string(), text.to_string())]),
            });
        }
        if let Some(object) = value.as_object() {
            let source = object
                .get("values")
                .and_then(Value::as_object)
                .unwrap_or(object);
            let values = source
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|text| (key.clone(), text.to_string()))
                })
                .collect::<HashMap<_, _>>();
            return Ok(LocalizedText { values });
        }
        Ok(LocalizedText::default())
    }
}

impl LocalizedText {
    pub fn resolve(&self, useEnglish: bool) -> String {
        let primary = if useEnglish { "en" } else { "zh" };
        self.values
            .get(primary)
            .or_else(|| self.values.get("default"))
            .or_else(|| self.values.values().next())
            .cloned()
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub description: LocalizedText,
    pub required: bool,
    pub default_value: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPackage {
    pub name: String,
    pub description: LocalizedText,
    pub tools: Vec<PackageTool>,
    pub states: Vec<ToolPackageState>,
    pub env: Vec<EnvVar>,
    pub is_built_in: bool,
    pub enabled_by_default: bool,
    pub display_name: LocalizedText,
    pub category: String,
    pub author: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPackageState {
    pub id: String,
    pub condition: String,
    pub inherit_tools: bool,
    pub exclude_tools: Vec<String>,
    pub tools: Vec<PackageTool>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PackageTool {
    pub name: String,
    pub description: LocalizedText,
    pub parameters: Vec<PackageToolParameter>,
    pub script: String,
    pub advice: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PackageToolParameter {
    pub name: String,
    pub description: LocalizedText,
    pub parameter_type: String,
    pub required: bool,
}
