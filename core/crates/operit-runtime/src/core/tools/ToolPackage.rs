use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct LocalizedText {
    pub values: HashMap<String, String>,
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

#[derive(Clone, Debug, Default)]
pub struct EnvVar {
    pub name: String,
    pub description: LocalizedText,
    pub required: bool,
    pub default_value: Option<String>,
}

#[derive(Clone, Debug, Default)]
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

#[derive(Clone, Debug, Default)]
pub struct ToolPackageState {
    pub id: String,
    pub condition: String,
    pub inherit_tools: bool,
    pub exclude_tools: Vec<String>,
    pub tools: Vec<PackageTool>,
}

#[derive(Clone, Debug, Default)]
pub struct PackageTool {
    pub name: String,
    pub description: LocalizedText,
    pub parameters: Vec<PackageToolParameter>,
    pub script: String,
    pub advice: bool,
}

#[derive(Clone, Debug, Default)]
pub struct PackageToolParameter {
    pub name: String,
    pub description: LocalizedText,
    pub parameter_type: String,
    pub required: bool,
}

pub struct LocalizedTextSerializer;

pub struct StringOrStringListSerializer;
