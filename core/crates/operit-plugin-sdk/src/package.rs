use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Serialize)]
/// Localized text value accepted from package manifests.
pub struct LocalizedText {
    pub values: HashMap<String, String>,
}

impl<'de> Deserialize<'de> for LocalizedText {
    /// Deserializes plain strings and legacy or wrapped locale maps.
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
    /// Resolves the localized text for the requested language mode.
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

#[cfg(test)]
mod tests {
    use super::LocalizedText;

    /// Verifies legacy locale objects resolve Chinese and English values.
    #[test]
    fn parses_operit1_locale_object() {
        let text: LocalizedText =
            serde_json::from_str(r#"{"zh":"思考引导","en":"Thinking Guidance"}"#).unwrap();
        assert_eq!(text.resolve(false), "思考引导");
        assert_eq!(text.resolve(true), "Thinking Guidance");
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
/// Environment variable declared by a tool package manifest.
pub struct EnvVar {
    pub name: String,
    pub description: LocalizedText,
    pub required: bool,
    pub default_value: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
/// Parsed package manifest containing metadata, tools, state, and environment needs.
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
/// Conditional package state that can adjust the package's available tools.
pub struct ToolPackageState {
    pub id: String,
    pub condition: String,
    pub inherit_tools: bool,
    pub exclude_tools: Vec<String>,
    pub tools: Vec<PackageTool>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
/// Executable tool declaration inside a package manifest.
pub struct PackageTool {
    pub name: String,
    pub description: LocalizedText,
    pub parameters: Vec<PackageToolParameter>,
    pub script: String,
    pub advice: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
/// Parameter declaration for a packaged tool.
pub struct PackageToolParameter {
    pub name: String,
    pub description: LocalizedText,
    pub parameter_type: String,
    pub required: bool,
}

/// Serializer marker retained for compatibility with package schema code.
pub struct LocalizedTextSerializer;

/// Serializer marker for fields accepted as either a string or a string list.
pub struct StringOrStringListSerializer;

/// Package source that can be published or exported from local package storage.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct PublishablePackageSource {
    pub packageName: String,
    pub displayName: String,
    pub description: String,
    pub author: Vec<String>,
    pub sourcePath: String,
    pub sourceFileName: String,
    pub fileExtension: String,
    pub isToolPkg: bool,
    pub inferredVersion: Option<String>,
}
