use super::super::ToolPackage::LocalizedText;

pub struct ToolPkgManifestWorkflowTemplate {
    pub id: String,
    pub display_name: LocalizedText,
    pub description: LocalizedText,
    pub resource_key: String,
}

pub struct ToolPkgManifestWorkspaceTemplate {
    pub id: String,
    pub display_name: LocalizedText,
    pub description: LocalizedText,
    pub resource_key: String,
    pub project_type: String,
}

pub struct ToolPkgWorkflowTemplateRuntime {
    pub id: String,
    pub display_name: LocalizedText,
    pub description: LocalizedText,
    pub resource_key: String,
}

pub struct ToolPkgWorkspaceTemplateRuntime {
    pub id: String,
    pub display_name: LocalizedText,
    pub description: LocalizedText,
    pub resource_key: String,
    pub project_type: String,
}
