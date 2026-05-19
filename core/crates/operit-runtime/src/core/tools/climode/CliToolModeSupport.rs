use crate::data::model::ModelConfigData::ApiProviderType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolExposureMode {
    FULL,
    CLI,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HiddenToolSourceKind {
    BUILTIN,
    INTERNAL,
    PACKAGE,
    MCP,
    ACTIVATION,
}

pub struct HiddenToolCatalogEntry {
    pub target_tool_name: String,
    pub display_name: String,
    pub description: String,
    pub parameter_hints: Vec<String>,
    pub source_kind: HiddenToolSourceKind,
    pub keywords: Vec<String>,
    pub suggested_params_json: Option<String>,
}

pub struct CliToolModeSupport;

impl ToolExposureMode {
    pub fn resolve(provider_type: ApiProviderType) -> Self {
        match provider_type {
            ApiProviderType::LMSTUDIO
            | ApiProviderType::OLLAMA
            | ApiProviderType::OPENAI_LOCAL
            | ApiProviderType::MNN
            | ApiProviderType::LLAMA_CPP => Self::CLI,
            _ => Self::FULL,
        }
    }
}
