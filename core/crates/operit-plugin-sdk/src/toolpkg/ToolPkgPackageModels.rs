use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Summarizes one ToolPkg subpackage and its enabled state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgSubpackageInfo {
    pub packageName: String,
    pub subpackageId: String,
    pub displayName: String,
    pub description: String,
    pub enabledByDefault: bool,
    pub toolCount: usize,
    pub enabled: bool,
}

/// Describes the public UI, resource, and subpackage surface of a ToolPkg container.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgContainerDetails {
    pub packageName: String,
    pub displayName: String,
    pub description: String,
    pub version: String,
    pub author: Vec<String>,
    pub resourceCount: usize,
    pub workspaceTemplateCount: usize,
    pub uiModuleCount: usize,
    pub toolboxUiModules: Vec<ToolPkgToolboxUiModule>,
    pub subpackages: Vec<ToolPkgSubpackageInfo>,
    pub workspaceTemplates: Vec<ToolPkgWorkspaceTemplate>,
}

/// Describes one workspace template exposed by a ToolPkg container.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgWorkspaceTemplate {
    pub containerPackageName: String,
    pub toolPkgId: String,
    pub templateId: String,
    pub displayName: String,
    pub description: String,
    pub resourceKey: String,
    pub projectType: String,
}

/// Reports the workspace created from a ToolPkg template.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgWorkspaceTemplateImportResult {
    pub containerPackageName: String,
    pub toolPkgId: String,
    pub templateId: String,
    pub workspacePath: String,
    pub workspaceConfig: Value,
}

/// Describes one ToolPkg module shown on the toolbox surface.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgToolboxUiModule {
    pub containerPackageName: String,
    pub toolPkgId: String,
    pub routeId: String,
    pub uiModuleId: String,
    pub runtime: String,
    pub screen: String,
    pub title: String,
    pub description: String,
    pub moduleSpec: BTreeMap<String, Value>,
    pub keepAlive: bool,
}

/// Describes one public ToolPkg UI route.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgUiRoute {
    pub containerPackageName: String,
    pub toolPkgId: String,
    pub routeId: String,
    pub uiModuleId: String,
    pub runtime: String,
    pub screen: String,
    pub title: String,
    pub description: String,
    pub moduleSpec: BTreeMap<String, Value>,
    pub keepAlive: bool,
}

/// Describes one ToolPkg navigation entry.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgNavigationEntry {
    pub containerPackageName: String,
    pub toolPkgId: String,
    pub entryId: String,
    pub routeId: String,
    pub surface: String,
    pub title: String,
    pub description: String,
    pub action: Option<ToolPkgNavigationActionHook>,
    pub icon: Option<String>,
    pub order: i32,
}

/// Describes the hook invoked by a ToolPkg navigation entry.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgNavigationActionHook {
    pub functionName: String,
    pub functionSource: Option<String>,
}

/// Describes one desktop widget exposed by a ToolPkg container.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgDesktopWidget {
    pub containerPackageName: String,
    pub toolPkgId: String,
    pub widgetId: String,
    pub routeId: String,
    pub renderRouteId: String,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub icon: Option<String>,
    pub order: i32,
}
