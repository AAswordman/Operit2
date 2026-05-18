use std::collections::HashMap;

pub struct LocalizedText {
    pub values: HashMap<String, String>,
}

pub struct EnvVar {
    pub name: String,
    pub description: LocalizedText,
    pub required: bool,
    pub default_value: Option<String>,
}

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

pub struct ToolPackageState {
    pub id: String,
    pub condition: String,
    pub inherit_tools: bool,
    pub exclude_tools: Vec<String>,
    pub tools: Vec<PackageTool>,
}

pub struct PackageTool {
    pub name: String,
    pub description: LocalizedText,
    pub parameters: Vec<PackageToolParameter>,
    pub script: String,
    pub advice: bool,
}

pub struct PackageToolParameter {
    pub name: String,
    pub description: LocalizedText,
    pub parameter_type: String,
    pub required: bool,
}

pub struct LocalizedTextSerializer;

pub struct StringOrStringListSerializer;
