use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Cursor;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::tools::packTool::ToolPkgCommonPluginConstants::*;
use crate::core::tools::packTool::ToolPkgTemplateModels::{
    ToolPkgManifestWorkflowTemplate, ToolPkgManifestWorkspaceTemplate,
    ToolPkgWorkflowTemplateRuntime, ToolPkgWorkspaceTemplateRuntime,
};
use crate::core::tools::ToolPackage::{LocalizedText, ToolPackage};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolPkgSourceType {
    #[default]
    ASSET,
    EXTERNAL,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgResourceRuntime {
    pub key: String,
    pub path: String,
    pub mime: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgUiModuleRuntime {
    pub id: String,
    pub runtime: String,
    pub screen: String,
    pub title: LocalizedText,
    #[serde(rename = "keepAlive")]
    pub keepAlive: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgUiRouteRuntime {
    pub id: String,
    #[serde(rename = "routeId")]
    pub routeId: String,
    pub runtime: String,
    pub screen: String,
    pub title: LocalizedText,
    #[serde(rename = "keepAlive")]
    pub keepAlive: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgNavigationEntryRuntime {
    pub id: String,
    #[serde(rename = "routeId")]
    pub routeId: String,
    pub surface: String,
    pub title: LocalizedText,
    pub action: Option<ToolPkgNavigationActionHookRuntime>,
    pub icon: Option<String>,
    pub order: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgNavigationActionHookRuntime {
    pub function: String,
    #[serde(rename = "functionSource", alias = "function_source")]
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgDesktopWidgetRuntime {
    pub id: String,
    #[serde(rename = "routeId")]
    pub routeId: String,
    #[serde(rename = "renderRouteId")]
    pub renderRouteId: String,
    pub title: LocalizedText,
    pub subtitle: LocalizedText,
    pub description: LocalizedText,
    pub icon: Option<String>,
    pub order: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgAppLifecycleHookRuntime {
    pub id: String,
    pub event: String,
    pub function: String,
    #[serde(rename = "functionSource", alias = "function_source")]
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgFunctionHookRuntime {
    pub id: String,
    pub function: String,
    #[serde(rename = "functionSource", alias = "function_source")]
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgHostEventHookRuntime {
    pub id: String,
    pub source: String,
    pub trigger: Value,
    pub function: String,
    #[serde(rename = "functionSource", alias = "function_source")]
    pub functionSource: Option<String>,
    #[serde(default = "defaultTrue")]
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgAiProviderHandlerRuntime {
    pub function: String,
    #[serde(rename = "functionSource", alias = "function_source")]
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgAiProviderRuntime {
    pub id: String,
    #[serde(rename = "displayName")]
    pub displayName: String,
    pub description: String,
    #[serde(rename = "listModelsHandler")]
    pub listModelsHandler: ToolPkgAiProviderHandlerRuntime,
    #[serde(rename = "sendMessageHandler")]
    pub sendMessageHandler: ToolPkgAiProviderHandlerRuntime,
    #[serde(rename = "testConnectionHandler")]
    pub testConnectionHandler: ToolPkgAiProviderHandlerRuntime,
    #[serde(rename = "calculateInputTokensHandler")]
    pub calculateInputTokensHandler: ToolPkgAiProviderHandlerRuntime,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgTagFunctionHookRuntime {
    pub id: String,
    pub tag: String,
    pub function: String,
    #[serde(rename = "functionSource")]
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgSubpackageRuntime {
    #[serde(rename = "packageName")]
    pub packageName: String,
    #[serde(rename = "containerPackageName")]
    pub containerPackageName: String,
    #[serde(rename = "subpackageId")]
    pub subpackageId: String,
    #[serde(rename = "entryPath")]
    pub entryPath: String,
    #[serde(rename = "displayName")]
    pub displayName: LocalizedText,
    pub description: LocalizedText,
    #[serde(rename = "enabledByDefault")]
    pub enabledByDefault: bool,
    #[serde(rename = "toolCount")]
    pub toolCount: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgRegisteredUiModule {
    pub id: String,
    #[serde(default)]
    pub runtime: String,
    pub screen: String,
    #[serde(default)]
    pub title: LocalizedText,
    #[serde(rename = "keepAlive")]
    #[serde(default)]
    pub keepAlive: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgRegisteredUiRoute {
    pub id: String,
    #[serde(rename = "routeId", alias = "route", default)]
    pub routeId: String,
    #[serde(default)]
    pub runtime: String,
    pub screen: String,
    #[serde(default)]
    pub title: LocalizedText,
    #[serde(rename = "keepAlive")]
    #[serde(default)]
    pub keepAlive: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgRegisteredNavigationEntry {
    pub id: String,
    #[serde(default)]
    pub surface: String,
    #[serde(rename = "routeId", alias = "route", default)]
    pub routeId: Option<String>,
    #[serde(default)]
    pub action: Option<ToolPkgNavigationActionHookRuntime>,
    #[serde(default)]
    pub title: LocalizedText,
    #[serde(default)]
    pub subtitle: LocalizedText,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub order: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgRegisteredDesktopWidget {
    pub id: String,
    #[serde(rename = "routeId", alias = "route", default)]
    pub routeId: String,
    #[serde(rename = "renderRouteId", alias = "render", default)]
    pub renderRouteId: String,
    #[serde(default)]
    pub title: LocalizedText,
    #[serde(default)]
    pub subtitle: LocalizedText,
    #[serde(default)]
    pub description: LocalizedText,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub order: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgRegisteredAppLifecycleHook {
    pub id: String,
    pub event: String,
    pub function: String,
    #[serde(rename = "functionSource", alias = "function_source")]
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgRegisteredFunctionHook {
    pub id: String,
    pub function: String,
    #[serde(rename = "functionSource", alias = "function_source")]
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgRegisteredHostEventHook {
    pub id: String,
    pub source: String,
    pub trigger: Value,
    pub function: String,
    #[serde(rename = "functionSource", alias = "function_source")]
    pub functionSource: Option<String>,
    #[serde(default = "defaultTrue")]
    pub enabled: bool,
}
fn defaultTrue() -> bool {
    true
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgRegisteredAiProviderHandler {
    pub function: String,
    #[serde(rename = "functionSource", alias = "function_source")]
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgRegisteredAiProvider {
    pub id: String,
    #[serde(rename = "displayName")]
    #[serde(default)]
    pub displayName: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "listModelsHandler", alias = "listModels", default)]
    pub listModelsHandler: ToolPkgRegisteredAiProviderHandler,
    #[serde(rename = "sendMessageHandler", alias = "sendMessage", default)]
    pub sendMessageHandler: ToolPkgRegisteredAiProviderHandler,
    #[serde(rename = "testConnectionHandler", alias = "testConnection", default)]
    pub testConnectionHandler: ToolPkgRegisteredAiProviderHandler,
    #[serde(
        rename = "calculateInputTokensHandler",
        alias = "calculateInputTokens",
        default
    )]
    pub calculateInputTokensHandler: ToolPkgRegisteredAiProviderHandler,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgRegisteredTagFunctionHook {
    pub id: String,
    pub tag: String,
    pub function: String,
    #[serde(rename = "functionSource", alias = "function_source")]
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgMainRegistration {
    #[serde(rename = "toolboxUiModules", default)]
    pub toolboxUiModules: Vec<ToolPkgRegisteredUiModule>,
    #[serde(rename = "uiRoutes", default)]
    pub uiRoutes: Vec<ToolPkgRegisteredUiRoute>,
    #[serde(rename = "navigationEntries", default)]
    pub navigationEntries: Vec<ToolPkgRegisteredNavigationEntry>,
    #[serde(rename = "desktopWidgets", default)]
    pub desktopWidgets: Vec<ToolPkgRegisteredDesktopWidget>,
    #[serde(rename = "appLifecycleHooks", default)]
    pub appLifecycleHooks: Vec<ToolPkgRegisteredAppLifecycleHook>,
    #[serde(rename = "messageProcessingPlugins", default)]
    pub messageProcessingPlugins: Vec<ToolPkgRegisteredFunctionHook>,
    #[serde(rename = "xmlRenderPlugins", default)]
    pub xmlRenderPlugins: Vec<ToolPkgRegisteredTagFunctionHook>,
    #[serde(rename = "inputMenuTogglePlugins", default)]
    pub inputMenuTogglePlugins: Vec<ToolPkgRegisteredFunctionHook>,
    #[serde(rename = "chatInputHooks", default)]
    pub chatInputHooks: Vec<ToolPkgRegisteredFunctionHook>,
    #[serde(rename = "chatViewHooks", default)]
    pub chatViewHooks: Vec<ToolPkgRegisteredFunctionHook>,
    #[serde(rename = "hostEventHooks", default)]
    pub hostEventHooks: Vec<ToolPkgRegisteredHostEventHook>,
    #[serde(rename = "toolLifecycleHooks", default)]
    pub toolLifecycleHooks: Vec<ToolPkgRegisteredFunctionHook>,
    #[serde(rename = "promptInputHooks", default)]
    pub promptInputHooks: Vec<ToolPkgRegisteredFunctionHook>,
    #[serde(rename = "promptHistoryHooks", default)]
    pub promptHistoryHooks: Vec<ToolPkgRegisteredFunctionHook>,
    #[serde(rename = "promptEstimateHistoryHooks", default)]
    pub promptEstimateHistoryHooks: Vec<ToolPkgRegisteredFunctionHook>,
    #[serde(rename = "systemPromptComposeHooks", default)]
    pub systemPromptComposeHooks: Vec<ToolPkgRegisteredFunctionHook>,
    #[serde(rename = "toolPromptComposeHooks", default)]
    pub toolPromptComposeHooks: Vec<ToolPkgRegisteredFunctionHook>,
    #[serde(rename = "promptFinalizeHooks", default)]
    pub promptFinalizeHooks: Vec<ToolPkgRegisteredFunctionHook>,
    #[serde(rename = "promptEstimateFinalizeHooks", default)]
    pub promptEstimateFinalizeHooks: Vec<ToolPkgRegisteredFunctionHook>,
    #[serde(rename = "summaryGenerateHooks", default)]
    pub summaryGenerateHooks: Vec<ToolPkgRegisteredFunctionHook>,
    #[serde(rename = "aiProviders", default)]
    pub aiProviders: Vec<ToolPkgRegisteredAiProvider>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ToolPkgMainRegistrationParseResult {
    Success {
        registration: ToolPkgMainRegistration,
    },
    Failure {
        message: String,
    },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgContainerRuntime {
    #[serde(rename = "packageName")]
    pub packageName: String,
    #[serde(rename = "displayName")]
    pub displayName: LocalizedText,
    pub description: LocalizedText,
    pub version: String,
    pub author: Vec<String>,
    #[serde(rename = "mainEntry")]
    pub mainEntry: String,
    #[serde(rename = "sourceType")]
    pub sourceType: ToolPkgSourceType,
    #[serde(rename = "sourcePath")]
    pub sourcePath: String,
    pub subpackages: Vec<ToolPkgSubpackageRuntime>,
    pub resources: Vec<ToolPkgResourceRuntime>,
    #[serde(rename = "workflowTemplates")]
    pub workflowTemplates:
        Vec<crate::core::tools::packTool::ToolPkgTemplateModels::ToolPkgWorkflowTemplateRuntime>,
    #[serde(rename = "workspaceTemplates")]
    pub workspaceTemplates:
        Vec<crate::core::tools::packTool::ToolPkgTemplateModels::ToolPkgWorkspaceTemplateRuntime>,
    #[serde(rename = "uiModules")]
    pub uiModules: Vec<ToolPkgUiModuleRuntime>,
    #[serde(rename = "uiRoutes")]
    pub uiRoutes: Vec<ToolPkgUiRouteRuntime>,
    #[serde(rename = "navigationEntries")]
    pub navigationEntries: Vec<ToolPkgNavigationEntryRuntime>,
    #[serde(rename = "desktopWidgets")]
    pub desktopWidgets: Vec<ToolPkgDesktopWidgetRuntime>,
    #[serde(rename = "appLifecycleHooks")]
    pub appLifecycleHooks: Vec<ToolPkgAppLifecycleHookRuntime>,
    #[serde(rename = "messageProcessingPlugins")]
    pub messageProcessingPlugins: Vec<ToolPkgFunctionHookRuntime>,
    #[serde(rename = "xmlRenderPlugins")]
    pub xmlRenderPlugins: Vec<ToolPkgTagFunctionHookRuntime>,
    #[serde(rename = "inputMenuTogglePlugins")]
    pub inputMenuTogglePlugins: Vec<ToolPkgFunctionHookRuntime>,
    #[serde(rename = "chatInputHooks")]
    pub chatInputHooks: Vec<ToolPkgFunctionHookRuntime>,
    #[serde(rename = "chatViewHooks")]
    pub chatViewHooks: Vec<ToolPkgFunctionHookRuntime>,
    #[serde(rename = "hostEventHooks")]
    pub hostEventHooks: Vec<ToolPkgHostEventHookRuntime>,
    #[serde(rename = "toolLifecycleHooks")]
    pub toolLifecycleHooks: Vec<ToolPkgFunctionHookRuntime>,
    #[serde(rename = "promptInputHooks")]
    pub promptInputHooks: Vec<ToolPkgFunctionHookRuntime>,
    #[serde(rename = "promptHistoryHooks")]
    pub promptHistoryHooks: Vec<ToolPkgFunctionHookRuntime>,
    #[serde(rename = "promptEstimateHistoryHooks")]
    pub promptEstimateHistoryHooks: Vec<ToolPkgFunctionHookRuntime>,
    #[serde(rename = "systemPromptComposeHooks")]
    pub systemPromptComposeHooks: Vec<ToolPkgFunctionHookRuntime>,
    #[serde(rename = "toolPromptComposeHooks")]
    pub toolPromptComposeHooks: Vec<ToolPkgFunctionHookRuntime>,
    #[serde(rename = "promptFinalizeHooks")]
    pub promptFinalizeHooks: Vec<ToolPkgFunctionHookRuntime>,
    #[serde(rename = "promptEstimateFinalizeHooks")]
    pub promptEstimateFinalizeHooks: Vec<ToolPkgFunctionHookRuntime>,
    #[serde(rename = "summaryGenerateHooks")]
    pub summaryGenerateHooks: Vec<ToolPkgFunctionHookRuntime>,
    #[serde(rename = "aiProviders")]
    pub aiProviders: Vec<ToolPkgAiProviderRuntime>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgLoadResult {
    #[serde(rename = "containerPackage")]
    pub containerPackage: ToolPackage,
    #[serde(rename = "subpackagePackages")]
    pub subpackagePackages: Vec<ToolPackage>,
    #[serde(rename = "containerRuntime")]
    pub containerRuntime: ToolPkgContainerRuntime,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgManifest {
    #[serde(rename = "schema_version", default = "defaultSchemaVersion")]
    pub schemaVersion: i32,
    #[serde(rename = "toolpkg_id")]
    pub toolpkgId: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub main: String,
    #[serde(rename = "display_name", default)]
    pub displayName: LocalizedText,
    #[serde(default)]
    pub description: LocalizedText,
    #[serde(default, deserialize_with = "deserializeStringOrStringList")]
    pub author: Vec<String>,
    #[serde(rename = "enabled_by_default", default = "defaultEnabledByDefault")]
    pub enabledByDefault: bool,
    #[serde(default)]
    pub subpackages: Vec<ToolPkgManifestSubpackage>,
    #[serde(default)]
    pub resources: Vec<ToolPkgManifestResource>,
    #[serde(rename = "workflow_templates", default)]
    pub workflowTemplates: Vec<ToolPkgManifestWorkflowTemplate>,
    #[serde(rename = "workspace_templates", default)]
    pub workspaceTemplates: Vec<ToolPkgManifestWorkspaceTemplate>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgManifestSubpackage {
    pub id: String,
    pub entry: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgManifestResource {
    pub key: String,
    pub path: String,
    #[serde(default)]
    pub mime: String,
}

#[derive(Clone, Debug, Default)]
pub struct ToolPkgEntryIndex {
    pub entryNames: BTreeSet<String>,
    entryNamesByNormalizedLowercase: BTreeMap<String, String>,
}

impl ToolPkgEntryIndex {
    #[allow(non_snake_case)]
    pub fn containsEntry(&self, rawPath: &str) -> bool {
        self.resolveEntryName(rawPath).is_some()
    }

    #[allow(non_snake_case)]
    pub fn resolveEntryName(&self, rawPath: &str) -> Option<String> {
        let normalizedPath = ToolPkgArchiveParser::normalizeZipEntryPath(rawPath)?;
        self.entryNamesByNormalizedLowercase
            .get(&normalizedPath.to_ascii_lowercase())
            .cloned()
    }

    #[allow(non_snake_case)]
    pub fn containsEntriesUnderDirectory(&self, rawDirectoryPath: &str) -> bool {
        let normalizedDirectoryPath =
            match ToolPkgArchiveParser::normalizeResourcePath(rawDirectoryPath) {
                Some(value) => value,
                None => return false,
            };
        let prefix = format!("{}/", normalizedDirectoryPath.trim_end_matches('/'));
        self.entryNames.iter().any(|entryName| {
            entryName
                .to_ascii_lowercase()
                .starts_with(&prefix.to_ascii_lowercase())
        })
    }
}

#[derive(Clone, Debug)]
pub struct ToolPkgManifestPreview {
    pub entryName: String,
    pub manifest: ToolPkgManifest,
}

pub struct ToolPkgParser;

pub struct ToolPkgArchiveParser;

impl ToolPkgArchiveParser {
    #[allow(non_snake_case)]
    pub fn parseToolPkgFromIndexedEntries<
        FReadEntryText,
        FParseJsPackage,
        FParseMainRegistration,
        FReportPackageLoadError,
    >(
        entryIndex: &ToolPkgEntryIndex,
        mut readEntryText: FReadEntryText,
        sourceType: ToolPkgSourceType,
        sourcePath: &str,
        isBuiltIn: bool,
        mut parseJsPackage: FParseJsPackage,
        mut parseMainRegistration: FParseMainRegistration,
        mut reportPackageLoadError: FReportPackageLoadError,
    ) -> Result<ToolPkgLoadResult, String>
    where
        FReadEntryText: FnMut(&str) -> Option<String>,
        FParseJsPackage: FnMut(&str, &mut dyn FnMut(String, String)) -> Option<ToolPackage>,
        FParseMainRegistration: FnMut(&str, &str, &str) -> ToolPkgMainRegistrationParseResult,
        FReportPackageLoadError: FnMut(String, String),
    {
        let manifestEntryName = findManifestEntry(&entryIndex.entryNames)
            .ok_or_else(|| "manifest.hjson or manifest.json not found".to_string())?;
        let manifestText = readEntryText(&manifestEntryName)
            .ok_or_else(|| "Failed to read manifest entry".to_string())?;
        let manifest = parseToolPkgManifest(&manifestText, &manifestEntryName)?;
        let manifestBasePath = manifestEntryName
            .rsplit_once('/')
            .map(|(base, _)| base.to_string())
            .unwrap_or_default();

        if manifest.toolpkgId.trim().is_empty() {
            return Err("manifest.toolpkg_id is required".to_string());
        }
        let normalizedMainEntry =
            Self::resolveManifestRelativeZipEntryPath(&manifestBasePath, &manifest.main)
                .ok_or_else(|| "manifest.main is required".to_string())?;
        if !entryIndex.containsEntry(&normalizedMainEntry) {
            return Err(format!(
                "Cannot find manifest.main entry '{}'",
                manifest.main
            ));
        }
        let mainScriptText = readEntryText(&normalizedMainEntry)
            .ok_or_else(|| format!("Failed to read manifest.main entry '{}'", manifest.main))?;

        let mut subpackagePackages = Vec::new();
        let mut subpackageRuntimes = Vec::new();
        for subpackage in &manifest.subpackages {
            let rawSubpackageId = subpackage.id.trim();
            let subpackageErrorKey = if rawSubpackageId.is_empty() {
                format!("{}:unknown_subpackage", manifest.toolpkgId)
            } else {
                rawSubpackageId.to_string()
            };

            if rawSubpackageId.is_empty() {
                reportPackageLoadError(
                    subpackageErrorKey,
                    format!("{sourcePath}: subpackage.id is required"),
                );
                continue;
            }
            if subpackage.entry.trim().is_empty() {
                reportPackageLoadError(
                    subpackageErrorKey,
                    format!("{sourcePath}: subpackage.entry is required for '{rawSubpackageId}'"),
                );
                continue;
            }

            let normalizedSubpackageId = rawSubpackageId.to_string();
            let packageName = normalizedSubpackageId.clone();
            let result = (|| {
                let normalizedSubpackageEntry =
                    Self::resolveManifestRelativeZipEntryPath(&manifestBasePath, &subpackage.entry)
                        .ok_or_else(|| {
                            format!("Invalid subpackage entry '{}'", subpackage.entry)
                        })?;
                let jsContent = readEntryText(&normalizedSubpackageEntry).ok_or_else(|| {
                    format!("Cannot find subpackage entry '{}'", subpackage.entry)
                })?;
                let mut parserErrorReporter = |_: String, error: String| {
                    reportPackageLoadError(
                        packageName.clone(),
                        format!("{sourcePath}:{}: {error}", subpackage.entry),
                    );
                };
                let parsedPackage = parseJsPackage(&jsContent, &mut parserErrorReporter)
                    .ok_or_else(|| {
                        format!("Failed to parse subpackage script '{}'", subpackage.entry)
                    })?;
                let resolvedDescription = parsedPackage.description.clone();
                let resolvedDisplayName = if hasLocalizedTextContent(&parsedPackage.display_name) {
                    parsedPackage.display_name.clone()
                } else {
                    localizedTextOf(&parsedPackage.name)
                };
                let normalizedPackage = ToolPackage {
                    name: packageName.clone(),
                    is_built_in: isBuiltIn,
                    ..parsedPackage
                };
                let runtime = ToolPkgSubpackageRuntime {
                    packageName: packageName.clone(),
                    containerPackageName: manifest.toolpkgId.clone(),
                    subpackageId: normalizedSubpackageId.clone(),
                    entryPath: normalizedSubpackageEntry,
                    displayName: resolvedDisplayName,
                    description: resolvedDescription,
                    enabledByDefault: normalizedPackage.enabled_by_default,
                    toolCount: normalizedPackage.tools.len(),
                };
                Ok::<(ToolPackage, ToolPkgSubpackageRuntime), String>((normalizedPackage, runtime))
            })();
            match result {
                Ok((package, runtime)) => {
                    subpackagePackages.push(package);
                    subpackageRuntimes.push(runtime);
                }
                Err(error) => {
                    reportPackageLoadError(
                        packageName,
                        format!("{sourcePath}:{}: {error}", subpackage.entry),
                    );
                }
            }
        }

        if !manifest.subpackages.is_empty() && subpackagePackages.is_empty() {
            return Err(format!(
                "No valid subpackages were loaded from toolpkg '{}'",
                manifest.toolpkgId
            ));
        }

        let mut resources = Vec::new();
        for resource in &manifest.resources {
            if resource.key.trim().is_empty() {
                return Err("resource.key is required".to_string());
            }
            if resource.path.trim().is_empty() {
                return Err(format!(
                    "resource.path is required for key '{}'",
                    resource.key
                ));
            }
            let normalizedPath =
                Self::resolveManifestRelativeResourcePath(&manifestBasePath, &resource.path)
                    .ok_or_else(|| format!("Invalid resource path: {}", resource.path))?;
            if Self::isDirectoryResourceMime(Some(&resource.mime)) {
                if !entryIndex.containsEntriesUnderDirectory(&normalizedPath) {
                    return Err(format!(
                        "Cannot find resource directory '{}'",
                        resource.path
                    ));
                }
            } else if !entryIndex.containsEntry(&normalizedPath) {
                return Err(format!("Cannot find resource path '{}'", resource.path));
            }
            resources.push(ToolPkgResourceRuntime {
                key: resource.key.clone(),
                path: normalizedPath,
                mime: resource.mime.clone(),
            });
        }

        let resourceByKey = resources
            .iter()
            .map(|resource| (resource.key.to_ascii_lowercase(), resource))
            .collect::<BTreeMap<_, _>>();

        let mut workflowTemplateIds = BTreeSet::new();
        let mut workflowTemplates = Vec::new();
        for (index, template) in manifest.workflowTemplates.iter().enumerate() {
            let templateId = template.id.trim().to_string();
            if templateId.is_empty() {
                return Err(format!("workflow_templates[{index}].id is required"));
            }
            if !workflowTemplateIds.insert(templateId.to_ascii_lowercase()) {
                return Err(format!("Duplicate workflow template id: {templateId}"));
            }
            let resourceKey = template.resource_key.trim().to_string();
            if resourceKey.is_empty() {
                return Err(format!(
                    "workflow_templates[{index}].resource_key is required"
                ));
            }
            let resource = resourceByKey.get(&resourceKey.to_ascii_lowercase()).ok_or_else(|| {
                format!("workflow_templates[{index}].resource_key not found in manifest.resources: {resourceKey}")
            })?;
            if Self::isDirectoryResourceMime(Some(&resource.mime)) {
                return Err(format!(
                    "workflow_templates[{index}].resource_key must reference a file resource: {resourceKey}"
                ));
            }
            workflowTemplates.push(ToolPkgWorkflowTemplateRuntime {
                id: templateId,
                display_name: template.display_name.clone(),
                description: template.description.clone(),
                resource_key: resource.key.clone(),
            });
        }

        let mut workspaceTemplateIds = BTreeSet::new();
        let mut workspaceTemplates = Vec::new();
        for (index, template) in manifest.workspaceTemplates.iter().enumerate() {
            let templateId = template.id.trim().to_string();
            if templateId.is_empty() {
                return Err(format!("workspace_templates[{index}].id is required"));
            }
            if !workspaceTemplateIds.insert(templateId.to_ascii_lowercase()) {
                return Err(format!("Duplicate workspace template id: {templateId}"));
            }
            let resourceKey = template.resource_key.trim().to_string();
            if resourceKey.is_empty() {
                return Err(format!(
                    "workspace_templates[{index}].resource_key is required"
                ));
            }
            let resource = resourceByKey.get(&resourceKey.to_ascii_lowercase()).ok_or_else(|| {
                format!("workspace_templates[{index}].resource_key not found in manifest.resources: {resourceKey}")
            })?;
            if !Self::isDirectoryResourceMime(Some(&resource.mime)) {
                return Err(format!(
                    "workspace_templates[{index}].resource_key must reference a directory resource: {resourceKey}"
                ));
            }
            workspaceTemplates.push(ToolPkgWorkspaceTemplateRuntime {
                id: templateId,
                display_name: template.display_name.clone(),
                description: template.description.clone(),
                resource_key: resource.key.clone(),
                project_type: template.project_type.trim().to_string(),
            });
        }

        let containerDisplayName = if hasLocalizedTextContent(&manifest.displayName) {
            manifest.displayName.clone()
        } else {
            localizedTextOf(&manifest.toolpkgId)
        };
        let mainRegistration =
            match parseMainRegistration(&mainScriptText, &manifest.toolpkgId, &normalizedMainEntry)
            {
                ToolPkgMainRegistrationParseResult::Success { registration } => registration,
                ToolPkgMainRegistrationParseResult::Failure { message } => {
                    return Err(format!(
                        "Failed to parse main registration from '{}': {message}",
                        manifest.main
                    ));
                }
            };

        let mut registeredUiRoutes = Vec::new();
        for module in &mainRegistration.toolboxUiModules {
            registeredUiRoutes.push(ToolPkgRegisteredUiRoute {
                id: module.id.clone(),
                routeId: buildToolPkgRouteId(&manifest.toolpkgId, &module.id),
                runtime: module.runtime.clone(),
                screen: module.screen.clone(),
                title: module.title.clone(),
                keepAlive: module.keepAlive,
            });
        }
        registeredUiRoutes.extend(mainRegistration.uiRoutes.clone());

        let mut registeredNavigationEntries = Vec::new();
        for (index, module) in mainRegistration.toolboxUiModules.iter().enumerate() {
            registeredNavigationEntries.push(ToolPkgRegisteredNavigationEntry {
                id: format!("toolbox_{}", module.id),
                routeId: Some(buildToolPkgRouteId(&manifest.toolpkgId, &module.id)),
                surface: TOOLPKG_NAV_SURFACE_TOOLBOX.to_string(),
                title: module.title.clone(),
                order: index as i32,
                ..Default::default()
            });
        }
        registeredNavigationEntries.extend(mainRegistration.navigationEntries.clone());

        let mut uiModules = Vec::new();
        let mut uiRoutes = Vec::new();
        let mut uiModuleIds = BTreeSet::new();
        let mut routeIds = BTreeSet::new();
        for (index, module) in registeredUiRoutes.iter().enumerate() {
            let id = module.id.trim().to_string();
            if id.is_empty() {
                return Err(format!(
                    "{TOOLPKG_REGISTRATION_UI_ROUTE}[{index}].id is required"
                ));
            }
            if !uiModuleIds.insert(id.to_ascii_lowercase()) {
                return Err(format!("Duplicate toolpkg ui route id: {id}"));
            }
            let runtimeName = module.runtime.trim();
            let runtimeName = if runtimeName.is_empty() {
                TOOLPKG_RUNTIME_COMPOSE_DSL.to_string()
            } else {
                runtimeName.to_string()
            };
            let routeId = module.routeId.trim().to_string();
            if routeId.is_empty() {
                return Err(format!(
                    "{TOOLPKG_REGISTRATION_UI_ROUTE}[{index}].route is required"
                ));
            }
            if !routeIds.insert(routeId.to_ascii_lowercase()) {
                return Err(format!("Duplicate toolpkg route id: {routeId}"));
            }
            let normalizedScreenPath =
                Self::normalizeZipEntryPath(&module.screen).ok_or_else(|| {
                    format!(
                        "{TOOLPKG_REGISTRATION_UI_ROUTE}[{index}].screen is invalid: {}",
                        module.screen
                    )
                })?;
            if !entryIndex.containsEntry(&normalizedScreenPath) {
                return Err(format!(
                    "{TOOLPKG_REGISTRATION_UI_ROUTE}[{index}].screen not found: {}",
                    module.screen
                ));
            }
            uiModules.push(ToolPkgUiModuleRuntime {
                id: id.clone(),
                runtime: runtimeName.clone(),
                screen: normalizedScreenPath.clone(),
                title: module.title.clone(),
                keepAlive: module.keepAlive,
            });
            uiRoutes.push(ToolPkgUiRouteRuntime {
                id,
                routeId,
                runtime: runtimeName,
                screen: normalizedScreenPath,
                title: module.title.clone(),
                keepAlive: module.keepAlive,
            });
        }

        let mut navigationEntries = Vec::new();
        let mut navigationEntryIds = BTreeSet::new();
        for (index, entry) in registeredNavigationEntries.iter().enumerate() {
            let id = entry.id.trim().to_string();
            let routeId = entry.routeId.clone().unwrap_or_default().trim().to_string();
            let surface = entry.surface.trim().to_ascii_lowercase();
            if id.is_empty() {
                return Err(format!(
                    "{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[{index}].id is required"
                ));
            }
            if !navigationEntryIds.insert(id.to_ascii_lowercase()) {
                return Err(format!("Duplicate toolpkg navigation entry id: {id}"));
            }
            if routeId.is_empty() && entry.action.is_none() {
                return Err(format!(
                    "{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[{index}].route or action is required"
                ));
            }
            if !routeId.is_empty()
                && !uiRoutes
                    .iter()
                    .any(|route| route.routeId.eq_ignore_ascii_case(&routeId))
            {
                return Err(format!(
                    "{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[{index}].route not found: {routeId}"
                ));
            }
            if surface != TOOLPKG_NAV_SURFACE_TOOLBOX
                && surface != TOOLPKG_NAV_SURFACE_MAIN_SIDEBAR_PLUGINS
            {
                return Err(format!("{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[{index}].surface is unsupported: {surface}"));
            }
            navigationEntries.push(ToolPkgNavigationEntryRuntime {
                id,
                routeId,
                surface,
                title: entry.title.clone(),
                action: entry.action.clone(),
                icon: entry.icon.clone(),
                order: entry.order,
            });
        }

        let desktopWidgets = validateDesktopWidgets(&mainRegistration, &uiRoutes)?;
        let appLifecycleHooks = validateFunctionHooksWithEvent(
            &mainRegistration.appLifecycleHooks,
            TOOLPKG_REGISTRATION_APP_LIFECYCLE_HOOK,
        )?;
        let messageProcessingPlugins = validateFunctionHooks(
            &mainRegistration.messageProcessingPlugins,
            TOOLPKG_REGISTRATION_MESSAGE_PROCESSING_PLUGIN,
        )?;
        let xmlRenderPlugins = validateTagFunctionHooks(
            &mainRegistration.xmlRenderPlugins,
            TOOLPKG_REGISTRATION_XML_RENDER_PLUGIN,
        )?;
        let inputMenuTogglePlugins = validateFunctionHooks(
            &mainRegistration.inputMenuTogglePlugins,
            TOOLPKG_REGISTRATION_INPUT_MENU_TOGGLE_PLUGIN,
        )?;
        let chatInputHooks = validateFunctionHooks(
            &mainRegistration.chatInputHooks,
            TOOLPKG_REGISTRATION_CHAT_INPUT_HOOK,
        )?;
        let chatViewHooks = validateFunctionHooks(
            &mainRegistration.chatViewHooks,
            TOOLPKG_REGISTRATION_CHAT_VIEW_HOOK,
        )?;
        let hostEventHooks = validateHostEventHooks(
            &mainRegistration.hostEventHooks,
            TOOLPKG_REGISTRATION_HOST_EVENT_HOOK,
        )?;
        let toolLifecycleHooks = validateFunctionHooks(
            &mainRegistration.toolLifecycleHooks,
            TOOLPKG_REGISTRATION_TOOL_LIFECYCLE_HOOK,
        )?;
        let promptInputHooks = validateFunctionHooks(
            &mainRegistration.promptInputHooks,
            TOOLPKG_REGISTRATION_PROMPT_INPUT_HOOK,
        )?;
        let promptHistoryHooks = validateFunctionHooks(
            &mainRegistration.promptHistoryHooks,
            TOOLPKG_REGISTRATION_PROMPT_HISTORY_HOOK,
        )?;
        let promptEstimateHistoryHooks = validateFunctionHooks(
            &mainRegistration.promptEstimateHistoryHooks,
            TOOLPKG_REGISTRATION_PROMPT_ESTIMATE_HISTORY_HOOK,
        )?;
        let systemPromptComposeHooks = validateFunctionHooks(
            &mainRegistration.systemPromptComposeHooks,
            TOOLPKG_REGISTRATION_SYSTEM_PROMPT_COMPOSE_HOOK,
        )?;
        let toolPromptComposeHooks = validateFunctionHooks(
            &mainRegistration.toolPromptComposeHooks,
            TOOLPKG_REGISTRATION_TOOL_PROMPT_COMPOSE_HOOK,
        )?;
        let promptFinalizeHooks = validateFunctionHooks(
            &mainRegistration.promptFinalizeHooks,
            TOOLPKG_REGISTRATION_PROMPT_FINALIZE_HOOK,
        )?;
        let promptEstimateFinalizeHooks = validateFunctionHooks(
            &mainRegistration.promptEstimateFinalizeHooks,
            TOOLPKG_REGISTRATION_PROMPT_ESTIMATE_FINALIZE_HOOK,
        )?;
        let summaryGenerateHooks = validateFunctionHooks(
            &mainRegistration.summaryGenerateHooks,
            TOOLPKG_REGISTRATION_SUMMARY_GENERATE_HOOK,
        )?;
        let aiProviders = validateAiProviders(&mainRegistration.aiProviders)?;

        let containerDescription = if hasLocalizedTextContent(&manifest.description) {
            manifest.description.clone()
        } else if hasLocalizedTextContent(&manifest.displayName) {
            manifest.displayName.clone()
        } else {
            localizedTextOf(&manifest.toolpkgId)
        };
        let containerPackage = ToolPackage {
            name: manifest.toolpkgId.clone(),
            description: containerDescription.clone(),
            tools: Vec::new(),
            is_built_in: isBuiltIn,
            enabled_by_default: manifest.enabledByDefault,
            display_name: containerDisplayName.clone(),
            category: "ToolPkg".to_string(),
            author: manifest.author.clone(),
            ..Default::default()
        };
        let runtime = ToolPkgContainerRuntime {
            packageName: manifest.toolpkgId.clone(),
            displayName: containerDisplayName,
            description: containerDescription,
            version: manifest.version.clone(),
            author: manifest.author.clone(),
            mainEntry: normalizedMainEntry,
            sourceType,
            sourcePath: sourcePath.to_string(),
            subpackages: subpackageRuntimes,
            resources,
            workflowTemplates,
            workspaceTemplates,
            uiModules,
            uiRoutes,
            navigationEntries,
            desktopWidgets,
            appLifecycleHooks,
            messageProcessingPlugins,
            xmlRenderPlugins,
            inputMenuTogglePlugins,
            chatInputHooks,
            chatViewHooks,
            hostEventHooks,
            toolLifecycleHooks,
            promptInputHooks,
            promptHistoryHooks,
            promptEstimateHistoryHooks,
            systemPromptComposeHooks,
            toolPromptComposeHooks,
            promptFinalizeHooks,
            promptEstimateFinalizeHooks,
            summaryGenerateHooks,
            aiProviders,
        };
        Ok(ToolPkgLoadResult {
            containerPackage,
            subpackagePackages,
            containerRuntime: runtime,
        })
    }

    #[allow(non_snake_case)]
    pub fn buildZipEntryIndex<R: std::io::Read + std::io::Seek>(
        archive: &mut zip::ZipArchive<R>,
    ) -> ToolPkgEntryIndex {
        let mut normalizedEntryNames = BTreeSet::new();
        let mut entryNamesByNormalizedLowercase = BTreeMap::new();
        for index in 0..archive.len() {
            let Ok(entry) = archive.by_index(index) else {
                continue;
            };
            if entry.is_dir() {
                continue;
            }
            let Some(normalizedName) = Self::normalizeZipEntryPath(entry.name()) else {
                continue;
            };
            normalizedEntryNames.insert(normalizedName.clone());
            let key = normalizedName.to_ascii_lowercase();
            entryNamesByNormalizedLowercase
                .entry(key)
                .or_insert_with(|| entry.name().to_string());
        }
        ToolPkgEntryIndex {
            entryNames: normalizedEntryNames,
            entryNamesByNormalizedLowercase,
        }
    }

    #[allow(non_snake_case)]
    pub fn buildDirectoryEntryIndex(rootDir: &std::path::Path) -> ToolPkgEntryIndex {
        let mut normalizedEntryNames = BTreeSet::new();
        let mut entryNamesByNormalizedLowercase = BTreeMap::new();
        if !rootDir.exists() {
            return ToolPkgEntryIndex::default();
        }
        collectDirectoryEntryIndex(
            rootDir,
            rootDir,
            &mut normalizedEntryNames,
            &mut entryNamesByNormalizedLowercase,
        );
        ToolPkgEntryIndex {
            entryNames: normalizedEntryNames,
            entryNamesByNormalizedLowercase,
        }
    }

    #[allow(non_snake_case)]
    pub fn normalizeZipEntryPath(rawPath: &str) -> Option<String> {
        let normalized = rawPath
            .replace('\\', "/")
            .trim()
            .trim_start_matches('/')
            .to_string();
        if normalized.trim().is_empty() || normalized.contains("..") {
            return None;
        }
        Some(normalized)
    }

    #[allow(non_snake_case)]
    pub fn resolveManifestRelativeZipEntryPath(
        manifestBasePath: &str,
        rawPath: &str,
    ) -> Option<String> {
        let normalized = Self::normalizeZipEntryPath(rawPath)?;
        if manifestBasePath.trim().is_empty() {
            return Some(normalized);
        }
        Self::normalizeZipEntryPath(&format!("{manifestBasePath}/{normalized}"))
    }

    #[allow(non_snake_case)]
    pub fn normalizeResourcePath(rawPath: &str) -> Option<String> {
        let normalized = Self::normalizeZipEntryPath(rawPath)?;
        let trimmed = normalized.trim_end_matches('/').to_string();
        if trimmed.is_empty() {
            return None;
        }
        Some(trimmed)
    }

    #[allow(non_snake_case)]
    pub fn resolveManifestRelativeResourcePath(
        manifestBasePath: &str,
        rawPath: &str,
    ) -> Option<String> {
        let normalized = Self::normalizeResourcePath(rawPath)?;
        if manifestBasePath.trim().is_empty() {
            return Some(normalized);
        }
        Self::normalizeResourcePath(&format!("{manifestBasePath}/{normalized}"))
    }

    #[allow(non_snake_case)]
    pub fn isDirectoryResourceMime(mime: Option<&str>) -> bool {
        matches!(
            mime.unwrap_or_default()
                .trim()
                .to_ascii_lowercase()
                .as_str(),
            "vnd.android.document/directory" | "inode/directory" | "application/x-directory"
        )
    }

    #[allow(non_snake_case)]
    pub fn readZipEntryText<R: std::io::Read + std::io::Seek>(
        archive: &mut zip::ZipArchive<R>,
        entryIndex: &ToolPkgEntryIndex,
        rawPath: &str,
    ) -> Option<String> {
        let archiveEntryName = entryIndex.resolveEntryName(rawPath)?;
        let mut entry = archive.by_name(&archiveEntryName).ok()?;
        let mut text = String::new();
        std::io::Read::read_to_string(&mut entry, &mut text).ok()?;
        Some(text)
    }

    #[allow(non_snake_case)]
    pub fn readDirectoryEntryText(
        rootDir: &std::path::Path,
        entryIndex: &ToolPkgEntryIndex,
        rawPath: &str,
    ) -> Option<String> {
        let relativePath = entryIndex.resolveEntryName(rawPath)?;
        std::fs::read_to_string(rootDir.join(relativePath)).ok()
    }

    #[allow(non_snake_case)]
    pub fn extractZipEntriesFromExternal(
        zipFilePath: &str,
        destinationDir: &std::path::Path,
    ) -> bool {
        let zipFile = std::path::Path::new(zipFilePath);
        if !zipFile.exists() {
            return false;
        }
        Self::extractZipEntriesFromFile(zipFile, destinationDir)
    }

    #[allow(non_snake_case)]
    pub fn extractZipEntriesFromAssetBytes(
        bytes: &'static [u8],
        destinationDir: &std::path::Path,
    ) -> bool {
        Self::extractZipEntriesFromReader(Cursor::new(bytes), destinationDir)
    }

    #[allow(non_snake_case)]
    fn extractZipEntriesFromFile(
        zipFile: &std::path::Path,
        destinationDir: &std::path::Path,
    ) -> bool {
        let Ok(file) = std::fs::File::open(zipFile) else {
            return false;
        };
        Self::extractZipEntriesFromReader(file, destinationDir)
    }

    #[allow(non_snake_case)]
    fn extractZipEntriesFromReader<R: std::io::Read + std::io::Seek>(
        reader: R,
        destinationDir: &std::path::Path,
    ) -> bool {
        let Ok(mut archive) = zip::ZipArchive::new(reader) else {
            return false;
        };
        for index in 0..archive.len() {
            let Ok(mut entry) = archive.by_index(index) else {
                return false;
            };
            if entry.is_dir() {
                continue;
            }
            let Some(normalizedEntry) = Self::normalizeZipEntryPath(entry.name()) else {
                continue;
            };
            let outputFile = destinationDir.join(normalizedEntry);
            if let Some(parent) = outputFile.parent() {
                if std::fs::create_dir_all(parent).is_err() {
                    return false;
                }
            }
            let Ok(mut output) = std::fs::File::create(outputFile) else {
                return false;
            };
            if std::io::copy(&mut entry, &mut output).is_err() {
                return false;
            }
        }
        true
    }
}

#[allow(non_snake_case)]
fn validateDesktopWidgets(
    registration: &ToolPkgMainRegistration,
    uiRoutes: &[ToolPkgUiRouteRuntime],
) -> Result<Vec<ToolPkgDesktopWidgetRuntime>, String> {
    let mut desktopWidgets = Vec::new();
    let mut desktopWidgetIds = BTreeSet::new();
    for (index, widget) in registration.desktopWidgets.iter().enumerate() {
        let id = widget.id.trim().to_string();
        let routeId = widget.routeId.trim().to_string();
        let renderRouteId = widget.renderRouteId.trim().to_string();
        if id.is_empty() {
            return Err(format!(
                "{TOOLPKG_REGISTRATION_DESKTOP_WIDGET}[{index}].id is required"
            ));
        }
        if !desktopWidgetIds.insert(id.to_ascii_lowercase()) {
            return Err(format!("Duplicate toolpkg desktop widget id: {id}"));
        }
        if routeId.is_empty() {
            return Err(format!(
                "{TOOLPKG_REGISTRATION_DESKTOP_WIDGET}[{index}].route is required"
            ));
        }
        if !uiRoutes
            .iter()
            .any(|route| route.routeId.eq_ignore_ascii_case(&routeId))
        {
            return Err(format!(
                "{TOOLPKG_REGISTRATION_DESKTOP_WIDGET}[{index}].route not found: {routeId}"
            ));
        }
        if renderRouteId.is_empty() {
            return Err(format!(
                "{TOOLPKG_REGISTRATION_DESKTOP_WIDGET}[{index}].render is required"
            ));
        }
        if !uiRoutes
            .iter()
            .any(|route| route.routeId.eq_ignore_ascii_case(&renderRouteId))
        {
            return Err(format!(
                "{TOOLPKG_REGISTRATION_DESKTOP_WIDGET}[{index}].render not found: {renderRouteId}"
            ));
        }
        desktopWidgets.push(ToolPkgDesktopWidgetRuntime {
            id,
            routeId,
            renderRouteId,
            title: widget.title.clone(),
            subtitle: widget.subtitle.clone(),
            description: widget.description.clone(),
            icon: widget.icon.clone(),
            order: widget.order,
        });
    }
    Ok(desktopWidgets)
}

#[allow(non_snake_case)]
fn validateFunctionHooks(
    hooks: &[ToolPkgRegisteredFunctionHook],
    registryName: &str,
) -> Result<Vec<ToolPkgFunctionHookRuntime>, String> {
    let mut runtimes = Vec::new();
    let mut ids = BTreeSet::new();
    for (index, hook) in hooks.iter().enumerate() {
        let id = hook.id.trim().to_string();
        if id.is_empty() {
            return Err(format!("{registryName}[{index}].id is required"));
        }
        if !ids.insert(id.to_ascii_lowercase()) {
            return Err(format!(
                "Duplicate {} id: {id}",
                duplicateLabel(registryName)
            ));
        }
        let function = hook.function.trim().to_string();
        if function.is_empty() {
            return Err(format!("{registryName}[{index}].function is required"));
        }
        runtimes.push(ToolPkgFunctionHookRuntime {
            id,
            function,
            functionSource: hook.functionSource.clone(),
        });
    }
    Ok(runtimes)
}

#[allow(non_snake_case)]
fn validateFunctionHooksWithEvent(
    hooks: &[ToolPkgRegisteredAppLifecycleHook],
    registryName: &str,
) -> Result<Vec<ToolPkgAppLifecycleHookRuntime>, String> {
    let mut runtimes = Vec::new();
    let mut ids = BTreeSet::new();
    for (index, hook) in hooks.iter().enumerate() {
        let id = hook.id.trim().to_string();
        if id.is_empty() {
            return Err(format!("{registryName}[{index}].id is required"));
        }
        if !ids.insert(id.to_ascii_lowercase()) {
            return Err(format!("Duplicate app lifecycle hook id: {id}"));
        }
        let event = hook.event.trim().to_ascii_lowercase();
        let function = hook.function.trim().to_string();
        if event.is_empty() {
            return Err(format!("{registryName}[{index}].event is required"));
        }
        if function.is_empty() {
            return Err(format!("{registryName}[{index}].function is required"));
        }
        runtimes.push(ToolPkgAppLifecycleHookRuntime {
            id,
            event,
            function,
            functionSource: hook.functionSource.clone(),
        });
    }
    Ok(runtimes)
}

#[allow(non_snake_case)]
fn validateHostEventHooks(
    hooks: &[ToolPkgRegisteredHostEventHook],
    registryName: &str,
) -> Result<Vec<ToolPkgHostEventHookRuntime>, String> {
    let mut runtimes = Vec::new();
    let mut ids = BTreeSet::new();
    for (index, hook) in hooks.iter().enumerate() {
        let id = hook.id.trim().to_string();
        if id.is_empty() {
            return Err(format!("{registryName}[{index}].id is required"));
        }
        if !ids.insert(id.to_ascii_lowercase()) {
            return Err(format!(
                "Duplicate {} id: {id}",
                duplicateLabel(registryName)
            ));
        }
        let source = hook.source.trim().to_string();
        if source.is_empty() {
            return Err(format!("{registryName}[{index}].source is required"));
        }
        let function = hook.function.trim().to_string();
        if function.is_empty() {
            return Err(format!("{registryName}[{index}].function is required"));
        }
        runtimes.push(ToolPkgHostEventHookRuntime {
            id,
            source,
            trigger: hook.trigger.clone(),
            function,
            functionSource: hook.functionSource.clone(),
            enabled: hook.enabled,
        });
    }
    Ok(runtimes)
}

#[allow(non_snake_case)]
fn validateTagFunctionHooks(
    hooks: &[ToolPkgRegisteredTagFunctionHook],
    registryName: &str,
) -> Result<Vec<ToolPkgTagFunctionHookRuntime>, String> {
    let mut runtimes = Vec::new();
    let mut ids = BTreeSet::new();
    for (index, hook) in hooks.iter().enumerate() {
        let id = hook.id.trim().to_string();
        if id.is_empty() {
            return Err(format!("{registryName}[{index}].id is required"));
        }
        if !ids.insert(id.to_ascii_lowercase()) {
            return Err(format!(
                "Duplicate {} id: {id}",
                duplicateLabel(registryName)
            ));
        }
        let tag = hook.tag.trim().to_ascii_lowercase();
        let function = hook.function.trim().to_string();
        if tag.is_empty() {
            return Err(format!("{registryName}[{index}].tag is required"));
        }
        if function.is_empty() {
            return Err(format!("{registryName}[{index}].function is required"));
        }
        runtimes.push(ToolPkgTagFunctionHookRuntime {
            id,
            tag,
            function,
            functionSource: hook.functionSource.clone(),
        });
    }
    Ok(runtimes)
}

#[allow(non_snake_case)]
fn validateAiProviders(
    providers: &[ToolPkgRegisteredAiProvider],
) -> Result<Vec<ToolPkgAiProviderRuntime>, String> {
    let mut runtimes = Vec::new();
    let mut ids = BTreeSet::new();
    for (index, provider) in providers.iter().enumerate() {
        let id = provider.id.trim().to_string();
        if id.is_empty() {
            return Err(format!(
                "{TOOLPKG_REGISTRATION_AI_PROVIDER}[{index}].id is required"
            ));
        }
        if !ids.insert(id.to_ascii_lowercase()) {
            return Err(format!("Duplicate ai provider id: {id}"));
        }
        runtimes.push(ToolPkgAiProviderRuntime {
            id: id.clone(),
            displayName: {
                let value = provider.displayName.trim();
                if value.is_empty() {
                    id.clone()
                } else {
                    value.to_string()
                }
            },
            description: provider.description.trim().to_string(),
            listModelsHandler: buildAiProviderHandler(
                index,
                "listModels",
                &provider.listModelsHandler,
            )?,
            sendMessageHandler: buildAiProviderHandler(
                index,
                "sendMessage",
                &provider.sendMessageHandler,
            )?,
            testConnectionHandler: buildAiProviderHandler(
                index,
                "testConnection",
                &provider.testConnectionHandler,
            )?,
            calculateInputTokensHandler: buildAiProviderHandler(
                index,
                "calculateInputTokens",
                &provider.calculateInputTokensHandler,
            )?,
        });
    }
    Ok(runtimes)
}

#[allow(non_snake_case)]
fn buildAiProviderHandler(
    index: usize,
    fieldName: &str,
    handler: &ToolPkgRegisteredAiProviderHandler,
) -> Result<ToolPkgAiProviderHandlerRuntime, String> {
    let function = handler.function.trim().to_string();
    if function.is_empty() {
        return Err(format!(
            "{TOOLPKG_REGISTRATION_AI_PROVIDER}[{index}].{fieldName} is required"
        ));
    }
    Ok(ToolPkgAiProviderHandlerRuntime {
        function,
        functionSource: handler.functionSource.clone(),
    })
}

#[allow(non_snake_case)]
fn duplicateLabel(registryName: &str) -> &'static str {
    match registryName {
        TOOLPKG_REGISTRATION_MESSAGE_PROCESSING_PLUGIN => "message processing plugin",
        TOOLPKG_REGISTRATION_XML_RENDER_PLUGIN => "xml render plugin",
        TOOLPKG_REGISTRATION_INPUT_MENU_TOGGLE_PLUGIN => "input menu toggle plugin",
        TOOLPKG_REGISTRATION_CHAT_INPUT_HOOK => "chat input hook",
        TOOLPKG_REGISTRATION_CHAT_VIEW_HOOK => "chat view hook",
        TOOLPKG_REGISTRATION_HOST_EVENT_HOOK => "host event hook",
        TOOLPKG_REGISTRATION_TOOL_LIFECYCLE_HOOK => "tool lifecycle hook",
        TOOLPKG_REGISTRATION_PROMPT_INPUT_HOOK => "prompt input hook",
        TOOLPKG_REGISTRATION_PROMPT_HISTORY_HOOK => "prompt history hook",
        TOOLPKG_REGISTRATION_PROMPT_ESTIMATE_HISTORY_HOOK => "prompt estimate history hook",
        TOOLPKG_REGISTRATION_SYSTEM_PROMPT_COMPOSE_HOOK => "system prompt compose hook",
        TOOLPKG_REGISTRATION_TOOL_PROMPT_COMPOSE_HOOK => "tool prompt compose hook",
        TOOLPKG_REGISTRATION_PROMPT_FINALIZE_HOOK => "prompt finalize hook",
        TOOLPKG_REGISTRATION_PROMPT_ESTIMATE_FINALIZE_HOOK => "prompt estimate finalize hook",
        TOOLPKG_REGISTRATION_SUMMARY_GENERATE_HOOK => "summary generate hook",
        _ => "hook",
    }
}

#[allow(non_snake_case)]
fn collectDirectoryEntryIndex(
    rootDir: &std::path::Path,
    currentDir: &std::path::Path,
    normalizedEntryNames: &mut BTreeSet<String>,
    entryNamesByNormalizedLowercase: &mut BTreeMap<String, String>,
) {
    let Ok(entries) = std::fs::read_dir(currentDir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collectDirectoryEntryIndex(
                rootDir,
                &path,
                normalizedEntryNames,
                entryNamesByNormalizedLowercase,
            );
        } else if path.is_file() {
            let Ok(relativePath) = path.strip_prefix(rootDir) else {
                continue;
            };
            let relativePath = relativePath.to_string_lossy().replace('\\', "/");
            let Some(normalizedName) = ToolPkgArchiveParser::normalizeZipEntryPath(&relativePath)
            else {
                continue;
            };
            normalizedEntryNames.insert(normalizedName.clone());
            entryNamesByNormalizedLowercase
                .entry(normalizedName.to_ascii_lowercase())
                .or_insert(relativePath);
        }
    }
}

#[allow(non_snake_case)]
fn findManifestEntry(entryNames: &BTreeSet<String>) -> Option<String> {
    if let Some(entry) = entryNames
        .iter()
        .find(|entry| entry.eq_ignore_ascii_case("manifest.hjson"))
    {
        return Some(entry.clone());
    }
    if let Some(entry) = entryNames
        .iter()
        .find(|entry| entry.eq_ignore_ascii_case("manifest.json"))
    {
        return Some(entry.clone());
    }
    if let Some(entry) = entryNames.iter().find(|entry| {
        entry
            .rsplit('/')
            .next()
            .is_some_and(|fileName| fileName.eq_ignore_ascii_case("manifest.hjson"))
    }) {
        return Some(entry.clone());
    }
    entryNames
        .iter()
        .find(|entry| {
            entry
                .rsplit('/')
                .next()
                .is_some_and(|fileName| fileName.eq_ignore_ascii_case("manifest.json"))
        })
        .cloned()
}

#[allow(non_snake_case)]
fn parseToolPkgManifest(content: &str, manifestEntryName: &str) -> Result<ToolPkgManifest, String> {
    let manifestJson = if manifestEntryName.to_ascii_lowercase().ends_with(".hjson") {
        let value: Value =
            json5::from_str(&normalizeHjsonLike(content)).map_err(|error| error.to_string())?;
        serde_json::to_string(&value).map_err(|error| error.to_string())?
    } else {
        content.to_string()
    };
    serde_json::from_str::<ToolPkgManifest>(&manifestJson).map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
fn normalizeHjsonLike(content: &str) -> String {
    content
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[allow(non_snake_case)]
fn hasLocalizedTextContent(text: &LocalizedText) -> bool {
    text.values.values().any(|value| !value.trim().is_empty())
}

#[allow(non_snake_case)]
fn localizedTextOf(value: &str) -> LocalizedText {
    LocalizedText {
        values: HashMap::from([("default".to_string(), value.to_string())]),
    }
}

#[allow(non_snake_case)]
fn defaultSchemaVersion() -> i32 {
    1
}

#[allow(non_snake_case)]
fn defaultEnabledByDefault() -> bool {
    true
}

#[allow(non_snake_case)]
fn deserializeStringOrStringList<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    if let Some(text) = value.as_str() {
        return Ok(vec![text.to_string()]);
    }
    if let Some(items) = value.as_array() {
        return Ok(items
            .iter()
            .filter_map(|item| item.as_str().map(ToString::to_string))
            .collect());
    }
    Ok(Vec::new())
}
