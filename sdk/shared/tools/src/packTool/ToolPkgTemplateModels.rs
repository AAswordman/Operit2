use crate::ToolPackage::LocalizedText;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ToolPkgManifestWorkflowTemplate {
    pub id: String,
    pub display_name: LocalizedText,
    pub description: LocalizedText,
    pub resource_key: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ToolPkgManifestWorkspaceTemplate {
    pub id: String,
    pub display_name: LocalizedText,
    pub description: LocalizedText,
    pub resource_key: String,
    pub project_type: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ToolPkgWorkflowTemplateRuntime {
    pub id: String,
    pub display_name: LocalizedText,
    pub description: LocalizedText,
    pub resource_key: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ToolPkgWorkspaceTemplateRuntime {
    pub id: String,
    pub display_name: LocalizedText,
    pub description: LocalizedText,
    pub resource_key: String,
    pub project_type: String,
}
