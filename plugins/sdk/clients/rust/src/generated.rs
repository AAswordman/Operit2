// GENERATED FILE. Source: operit-proxy-scan.

use operit_link::{CoreEventStream, CoreLinkError, CoreLinkPushSession, CoreValue};
use operit_plugin_sdk_ipc::PluginSdkClient;
use crate::{decode_messagepack_value, OperitPluginSdkTypedEventStream};

/// Generated SDK model for `operit_plugin_sdk::package::EnvVar`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkEnvVar {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: SdkLocalizedText,
    #[serde(rename = "required")]
    pub required: bool,
    #[serde(rename = "default_value")]
    pub default_value: Option<String>,
}

/// Generated SDK model for `operit_plugin_sdk::package::LocalizedText`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkLocalizedText {
    #[serde(rename = "values")]
    pub values: std::collections::HashMap<String,String>,
}

/// Generated SDK model for `operit_plugin_sdk::package::PackageTool`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkPackageTool {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: SdkLocalizedText,
    #[serde(rename = "parameters")]
    pub parameters: Vec<SdkPackageToolParameter>,
    #[serde(rename = "script")]
    pub script: String,
    #[serde(rename = "advice")]
    pub advice: bool,
}

/// Generated SDK model for `operit_plugin_sdk::package::PackageToolParameter`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkPackageToolParameter {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: SdkLocalizedText,
    #[serde(rename = "parameter_type")]
    pub parameter_type: String,
    #[serde(rename = "required")]
    pub required: bool,
}

/// Generated SDK model for `operit_plugin_sdk::package::ToolPackage`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPackage {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: SdkLocalizedText,
    #[serde(rename = "tools")]
    pub tools: Vec<SdkPackageTool>,
    #[serde(rename = "states")]
    pub states: Vec<SdkToolPackageState>,
    #[serde(rename = "env")]
    pub env: Vec<SdkEnvVar>,
    #[serde(rename = "is_built_in")]
    pub is_built_in: bool,
    #[serde(rename = "enabled_by_default")]
    pub enabled_by_default: bool,
    #[serde(rename = "display_name")]
    pub display_name: SdkLocalizedText,
    #[serde(rename = "category")]
    pub category: String,
    #[serde(rename = "author")]
    pub author: Vec<String>,
}

/// Generated SDK model for `operit_plugin_sdk::package::ToolPackageState`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPackageState {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "condition")]
    pub condition: String,
    #[serde(rename = "inherit_tools")]
    pub inherit_tools: bool,
    #[serde(rename = "exclude_tools")]
    pub exclude_tools: Vec<String>,
    #[serde(rename = "tools")]
    pub tools: Vec<SdkPackageTool>,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgContainerDetails`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgContainerDetails {
    #[serde(rename = "packageName")]
    pub packageName: String,
    #[serde(rename = "displayName")]
    pub displayName: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "version")]
    pub version: String,
    #[serde(rename = "apiVersion")]
    pub apiVersion: String,
    #[serde(rename = "logoResourceKey")]
    pub logoResourceKey: Option<String>,
    #[serde(rename = "logoMimeType")]
    pub logoMimeType: Option<String>,
    #[serde(rename = "author")]
    pub author: Vec<String>,
    #[serde(rename = "requires")]
    pub requires: Vec<SdkToolPkgManifestRequirement>,
    #[serde(rename = "resourceCount")]
    pub resourceCount: usize,
    #[serde(rename = "workspaceTemplateCount")]
    pub workspaceTemplateCount: usize,
    #[serde(rename = "uiModuleCount")]
    pub uiModuleCount: usize,
    #[serde(rename = "toolboxUiModules")]
    pub toolboxUiModules: Vec<SdkToolPkgToolboxUiModule>,
    #[serde(rename = "subpackages")]
    pub subpackages: Vec<SdkToolPkgSubpackageInfo>,
    #[serde(rename = "workspaceTemplates")]
    pub workspaceTemplates: Vec<SdkToolPkgWorkspaceTemplate>,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgLogoBytes`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgLogoBytes {
    #[serde(rename = "resourceKey")]
    pub resourceKey: String,
    #[serde(rename = "mimeType")]
    pub mimeType: String,
    #[serde(rename = "fileName")]
    pub fileName: String,
    #[serde(rename = "bytes")]
    pub bytes: Vec<u8>,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgSubpackageInfo`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgSubpackageInfo {
    #[serde(rename = "packageName")]
    pub packageName: String,
    #[serde(rename = "subpackageId")]
    pub subpackageId: String,
    #[serde(rename = "displayName")]
    pub displayName: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "enabledByDefault")]
    pub enabledByDefault: bool,
    #[serde(rename = "toolCount")]
    pub toolCount: usize,
    #[serde(rename = "enabled")]
    pub enabled: bool,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgToolboxUiModule`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgToolboxUiModule {
    #[serde(rename = "containerPackageName")]
    pub containerPackageName: String,
    #[serde(rename = "toolPkgId")]
    pub toolPkgId: String,
    #[serde(rename = "routeId")]
    pub routeId: String,
    #[serde(rename = "uiModuleId")]
    pub uiModuleId: String,
    #[serde(rename = "runtime")]
    pub runtime: String,
    #[serde(rename = "screen")]
    pub screen: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "moduleSpec")]
    pub moduleSpec: std::collections::BTreeMap<String, operit_link::CoreValue>,
    #[serde(rename = "keepAlive")]
    pub keepAlive: bool,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgWorkspaceTemplate`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgWorkspaceTemplate {
    #[serde(rename = "containerPackageName")]
    pub containerPackageName: String,
    #[serde(rename = "toolPkgId")]
    pub toolPkgId: String,
    #[serde(rename = "templateId")]
    pub templateId: String,
    #[serde(rename = "displayName")]
    pub displayName: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "resourceKey")]
    pub resourceKey: String,
    #[serde(rename = "projectType")]
    pub projectType: String,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgAiProviderHandlerRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgAiProviderHandlerRuntime {
    #[serde(rename = "function")]
    pub function: String,
    #[serde(rename = "functionSource")]
    pub functionSource: Option<String>,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgAiProviderRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgAiProviderRuntime {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "displayName")]
    pub displayName: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "listModelsHandler")]
    pub listModelsHandler: SdkToolPkgAiProviderHandlerRuntime,
    #[serde(rename = "sendMessageHandler")]
    pub sendMessageHandler: SdkToolPkgAiProviderHandlerRuntime,
    #[serde(rename = "testConnectionHandler")]
    pub testConnectionHandler: SdkToolPkgAiProviderHandlerRuntime,
    #[serde(rename = "calculateInputTokensHandler")]
    pub calculateInputTokensHandler: SdkToolPkgAiProviderHandlerRuntime,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgAppLifecycleHookRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgAppLifecycleHookRuntime {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "event")]
    pub event: String,
    #[serde(rename = "function")]
    pub function: String,
    #[serde(rename = "functionSource")]
    pub functionSource: Option<String>,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgChatComposerSlotRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgChatComposerSlotRuntime {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "slot")]
    pub slot: String,
    #[serde(rename = "screen")]
    pub screen: String,
    #[serde(rename = "order")]
    pub order: i32,
    #[serde(rename = "keepAlive")]
    pub keepAlive: bool,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgChatMessageMenuDialogRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgChatMessageMenuDialogRuntime {
    #[serde(rename = "screen")]
    pub screen: String,
    #[serde(rename = "title")]
    pub title: SdkLocalizedText,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgChatMessageMenuItemRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgChatMessageMenuItemRuntime {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "title")]
    pub title: SdkLocalizedText,
    #[serde(rename = "icon")]
    pub icon: Option<String>,
    #[serde(rename = "order")]
    pub order: i32,
    #[serde(rename = "senders")]
    pub senders: Vec<String>,
    #[serde(rename = "function")]
    pub function: String,
    #[serde(rename = "functionSource")]
    pub functionSource: Option<String>,
    #[serde(rename = "dialog")]
    pub dialog: Option<SdkToolPkgChatMessageMenuDialogRuntime>,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgContainerRuntime {
    #[serde(rename = "packageName")]
    pub packageName: String,
    #[serde(rename = "displayName")]
    pub displayName: SdkLocalizedText,
    #[serde(rename = "description")]
    pub description: SdkLocalizedText,
    #[serde(rename = "version")]
    pub version: String,
    #[serde(rename = "apiVersion")]
    pub apiVersion: String,
    #[serde(rename = "publicApi")]
    pub publicApi: Option<String>,
    #[serde(rename = "publicApis")]
    pub publicApis: Vec<SdkToolPkgRegisteredFunctionHook>,
    #[serde(rename = "requires")]
    pub requires: Vec<SdkToolPkgManifestRequirement>,
    #[serde(rename = "dependencyIssues")]
    pub dependencyIssues: Vec<SdkToolPkgDependencyIssue>,
    #[serde(rename = "manifestExtensions")]
    pub manifestExtensions: std::collections::BTreeMap<String, operit_link::CoreValue>,
    #[serde(rename = "author")]
    pub author: Vec<String>,
    #[serde(rename = "mainEntry")]
    pub mainEntry: String,
    #[serde(rename = "sourceType")]
    pub sourceType: SdkToolPkgSourceType,
    #[serde(rename = "sourcePath")]
    pub sourcePath: String,
    #[serde(rename = "subpackages")]
    pub subpackages: Vec<SdkToolPkgSubpackageRuntime>,
    #[serde(rename = "resources")]
    pub resources: Vec<SdkToolPkgResourceRuntime>,
    #[serde(rename = "wasmModules")]
    pub wasmModules: Vec<SdkToolPkgWasmModuleRuntime>,
    #[serde(rename = "workflowTemplates")]
    pub workflowTemplates: Vec<SdkToolPkgWorkflowTemplateRuntime>,
    #[serde(rename = "workspaceTemplates")]
    pub workspaceTemplates: Vec<SdkToolPkgWorkspaceTemplateRuntime>,
    #[serde(rename = "uiModules")]
    pub uiModules: Vec<SdkToolPkgUiModuleRuntime>,
    #[serde(rename = "uiRoutes")]
    pub uiRoutes: Vec<SdkToolPkgUiRouteRuntime>,
    #[serde(rename = "chatComposerSlots")]
    pub chatComposerSlots: Vec<SdkToolPkgChatComposerSlotRuntime>,
    #[serde(rename = "navigationEntries")]
    pub navigationEntries: Vec<SdkToolPkgNavigationEntryRuntime>,
    #[serde(rename = "desktopWidgets")]
    pub desktopWidgets: Vec<SdkToolPkgDesktopWidgetRuntime>,
    #[serde(rename = "appLifecycleHooks")]
    pub appLifecycleHooks: Vec<SdkToolPkgAppLifecycleHookRuntime>,
    #[serde(rename = "messageProcessingPlugins")]
    pub messageProcessingPlugins: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "xmlRenderPlugins")]
    pub xmlRenderPlugins: Vec<SdkToolPkgTagFunctionHookRuntime>,
    #[serde(rename = "inputMenuTogglePlugins")]
    pub inputMenuTogglePlugins: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "chatInputHooks")]
    pub chatInputHooks: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "chatViewHooks")]
    pub chatViewHooks: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "chatMessageHooks")]
    pub chatMessageHooks: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "chatLifecycleHooks")]
    pub chatLifecycleHooks: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "chatMessageMenuItems")]
    pub chatMessageMenuItems: Vec<SdkToolPkgChatMessageMenuItemRuntime>,
    #[serde(rename = "chatRuntimeHooks")]
    pub chatRuntimeHooks: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "hostEventHooks")]
    pub hostEventHooks: Vec<SdkToolPkgHostEventHookRuntime>,
    #[serde(rename = "toolLifecycleHooks")]
    pub toolLifecycleHooks: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "promptInputHooks")]
    pub promptInputHooks: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "promptHistoryHooks")]
    pub promptHistoryHooks: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "promptEstimateHistoryHooks")]
    pub promptEstimateHistoryHooks: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "systemPromptComposeHooks")]
    pub systemPromptComposeHooks: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "toolPromptComposeHooks")]
    pub toolPromptComposeHooks: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "promptFinalizeHooks")]
    pub promptFinalizeHooks: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "promptEstimateFinalizeHooks")]
    pub promptEstimateFinalizeHooks: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "summaryGenerateHooks")]
    pub summaryGenerateHooks: Vec<SdkToolPkgFunctionHookRuntime>,
    #[serde(rename = "coreCommands")]
    pub coreCommands: Vec<SdkToolPkgCoreCommandRuntime>,
    #[serde(rename = "aiProviders")]
    pub aiProviders: Vec<SdkToolPkgAiProviderRuntime>,
    #[serde(rename = "manifestExtensionHandlers")]
    pub manifestExtensionHandlers: Vec<SdkToolPkgRegisteredManifestExtension>,
    #[serde(rename = "logoResource")]
    pub logoResource: Option<SdkToolPkgResourceRuntime>,
    #[serde(rename = "marketOrigin")]
    pub marketOrigin: Option<SdkToolPkgMarketOrigin>,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgCoreCommandRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgCoreCommandRuntime {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "title")]
    pub title: SdkLocalizedText,
    #[serde(rename = "description")]
    pub description: SdkLocalizedText,
    #[serde(rename = "usage")]
    pub usage: String,
    #[serde(rename = "function")]
    pub function: String,
    #[serde(rename = "functionSource")]
    pub functionSource: Option<String>,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgDependencyIssue`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgDependencyIssue {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "code")]
    pub code: String,
    #[serde(rename = "requiredMinVersion")]
    pub requiredMinVersion: Option<String>,
    #[serde(rename = "requiredMaxVersion")]
    pub requiredMaxVersion: Option<String>,
    #[serde(rename = "installedVersion")]
    pub installedVersion: Option<String>,
    #[serde(rename = "enabled")]
    pub enabled: bool,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgDesktopWidgetRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgDesktopWidgetRuntime {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "routeId")]
    pub routeId: String,
    #[serde(rename = "renderRouteId")]
    pub renderRouteId: String,
    #[serde(rename = "title")]
    pub title: SdkLocalizedText,
    #[serde(rename = "subtitle")]
    pub subtitle: SdkLocalizedText,
    #[serde(rename = "description")]
    pub description: SdkLocalizedText,
    #[serde(rename = "icon")]
    pub icon: Option<String>,
    #[serde(rename = "order")]
    pub order: i32,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgFunctionHookRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgFunctionHookRuntime {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "function")]
    pub function: String,
    #[serde(rename = "functionSource")]
    pub functionSource: Option<String>,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgHostEventHookRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgHostEventHookRuntime {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "source")]
    pub source: String,
    #[serde(rename = "trigger")]
    pub trigger: operit_link::CoreValue,
    #[serde(rename = "function")]
    pub function: String,
    #[serde(rename = "functionSource")]
    pub functionSource: Option<String>,
    #[serde(rename = "enabled")]
    pub enabled: bool,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgManifestRequirement`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgManifestRequirement {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "min_version")]
    pub minVersion: Option<String>,
    #[serde(rename = "max_version")]
    pub maxVersion: Option<String>,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgMarketOrigin`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgMarketOrigin {
    #[serde(rename = "market")]
    pub market: String,
    #[serde(rename = "toolpkgId")]
    pub toolpkgId: String,
    #[serde(rename = "version")]
    pub version: String,
    #[serde(rename = "author")]
    pub author: Vec<String>,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgNavigationActionHookRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgNavigationActionHookRuntime {
    #[serde(rename = "function")]
    pub function: String,
    #[serde(rename = "functionSource")]
    pub functionSource: Option<String>,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgNavigationEntryRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgNavigationEntryRuntime {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "routeId")]
    pub routeId: String,
    #[serde(rename = "params")]
    pub params: Option<operit_link::CoreValue>,
    #[serde(rename = "surface")]
    pub surface: String,
    #[serde(rename = "title")]
    pub title: SdkLocalizedText,
    #[serde(rename = "action")]
    pub action: Option<SdkToolPkgNavigationActionHookRuntime>,
    #[serde(rename = "icon")]
    pub icon: Option<String>,
    #[serde(rename = "order")]
    pub order: i32,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgRegisteredFunctionHook`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgRegisteredFunctionHook {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "function")]
    pub function: String,
    #[serde(rename = "functionSource")]
    pub functionSource: Option<String>,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgRegisteredManifestExtension`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgRegisteredManifestExtension {
    #[serde(rename = "key")]
    pub key: String,
    #[serde(rename = "function")]
    pub function: String,
    #[serde(rename = "functionSource")]
    pub functionSource: Option<String>,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgResourceRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgResourceRuntime {
    #[serde(rename = "key")]
    pub key: String,
    #[serde(rename = "path")]
    pub path: String,
    #[serde(rename = "mime")]
    pub mime: String,
}

/// Generated SDK enum for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgSourceType`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SdkToolPkgSourceType {
    ASSET,
    MARKET,
    EXTERNAL,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgSubpackageRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgSubpackageRuntime {
    #[serde(rename = "packageName")]
    pub packageName: String,
    #[serde(rename = "containerPackageName")]
    pub containerPackageName: String,
    #[serde(rename = "subpackageId")]
    pub subpackageId: String,
    #[serde(rename = "entryPath")]
    pub entryPath: String,
    #[serde(rename = "displayName")]
    pub displayName: SdkLocalizedText,
    #[serde(rename = "description")]
    pub description: SdkLocalizedText,
    #[serde(rename = "enabledByDefault")]
    pub enabledByDefault: bool,
    #[serde(rename = "toolCount")]
    pub toolCount: usize,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgTagFunctionHookRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgTagFunctionHookRuntime {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "tag")]
    pub tag: String,
    #[serde(rename = "function")]
    pub function: String,
    #[serde(rename = "functionSource")]
    pub functionSource: Option<String>,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgUiModuleRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgUiModuleRuntime {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "runtime")]
    pub runtime: String,
    #[serde(rename = "screen")]
    pub screen: String,
    #[serde(rename = "screenExport")]
    pub screenExport: Option<String>,
    #[serde(rename = "title")]
    pub title: SdkLocalizedText,
    #[serde(rename = "keepAlive")]
    pub keepAlive: bool,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgUiRouteRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgUiRouteRuntime {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "routeId")]
    pub routeId: String,
    #[serde(rename = "runtime")]
    pub runtime: String,
    #[serde(rename = "screen")]
    pub screen: String,
    #[serde(rename = "screenExport")]
    pub screenExport: Option<String>,
    #[serde(rename = "title")]
    pub title: SdkLocalizedText,
    #[serde(rename = "keepAlive")]
    pub keepAlive: bool,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgWasmModuleRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgWasmModuleRuntime {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "path")]
    pub path: String,
    #[serde(rename = "exports")]
    pub exports: Vec<String>,
    #[serde(rename = "sourceLanguage")]
    pub sourceLanguage: String,
    #[serde(rename = "abi")]
    pub abi: String,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgTemplateModels::ToolPkgWorkflowTemplateRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgWorkflowTemplateRuntime {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "display_name")]
    pub display_name: SdkLocalizedText,
    #[serde(rename = "description")]
    pub description: SdkLocalizedText,
    #[serde(rename = "resource_key")]
    pub resource_key: String,
}

/// Generated SDK model for `operit_plugin_sdk::toolpkg::ToolPkgTemplateModels::ToolPkgWorkspaceTemplateRuntime`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgWorkspaceTemplateRuntime {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "display_name")]
    pub display_name: SdkLocalizedText,
    #[serde(rename = "description")]
    pub description: SdkLocalizedText,
    #[serde(rename = "resource_key")]
    pub resource_key: String,
    #[serde(rename = "project_type")]
    pub project_type: String,
}

/// Generated SDK model for `operit_tools::ConversationMarkupManager::ToolResult`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolResult {
    #[serde(rename = "toolName")]
    pub toolName: String,
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "result")]
    pub result: operit_link::CoreValue,
    #[serde(rename = "error")]
    pub error: Option<String>,
}

/// Generated SDK model for `operit_tools::tools::PackageLoadingProgress::PluginLoadingItem`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkPluginLoadingItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "displayName")]
    pub displayName: String,
    #[serde(rename = "kind")]
    pub kind: String,
    #[serde(rename = "status")]
    pub status: String,
    #[serde(rename = "message")]
    pub message: String,
    #[serde(rename = "logText")]
    pub logText: String,
}

/// Generated SDK model for `operit_tools::tools::PackageLoadingProgress::PluginLoadingProgress`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkPluginLoadingProgress {
    #[serde(rename = "visible")]
    pub visible: bool,
    #[serde(rename = "forceExpanded")]
    pub forceExpanded: bool,
    #[serde(rename = "progress")]
    pub progress: f32,
    #[serde(rename = "phase")]
    pub phase: String,
    #[serde(rename = "currentTask")]
    pub currentTask: String,
    #[serde(rename = "pluginsStarted")]
    pub pluginsStarted: i32,
    #[serde(rename = "pluginsTotal")]
    pub pluginsTotal: i32,
    #[serde(rename = "plugins")]
    pub plugins: Vec<SdkPluginLoadingItem>,
}

/// Generated SDK model for `operit_tools::tools::packTool::RuntimePackageManager::ToolPkgLoadIssue`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdkToolPkgLoadIssue {
    #[serde(rename = "sourcePath")]
    pub sourcePath: String,
    #[serde(rename = "packageName")]
    pub packageName: Option<String>,
    #[serde(rename = "displayName")]
    pub displayName: String,
    #[serde(rename = "code")]
    pub code: String,
    #[serde(rename = "message")]
    pub message: String,
    #[serde(rename = "packageKind")]
    pub packageKind: String,
}

/// Generated client for Core object `application`.
#[derive(Clone)]
pub struct OperitApplicationClient { client: PluginSdkClient }

impl OperitApplicationClient {
    /// Creates a generated object client over one connected SDK session.
    pub fn new(client: PluginSdkClient) -> Self { Self { client } }
    /// Watches `pluginLoadingProgressFlow` through the Core Link route.
    pub async fn pluginLoadingProgressFlow(&self) -> Result<OperitPluginSdkTypedEventStream<SdkPluginLoadingProgress>, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([]));
        let stream = self.client.watch(operit_link::CoreWatchRequest::new(self.client.nextRequestId()?, "core/application", "pluginLoadingProgressFlow", args)).await?;
        Ok(OperitPluginSdkTypedEventStream::new(stream))
    }
}

/// Generated client for Core object `application.packageManager`.
#[derive(Clone)]
pub struct OperitApplicationPackageManagerClient { client: PluginSdkClient }

impl OperitApplicationPackageManagerClient {
    /// Creates a generated object client over one connected SDK session.
    pub fn new(client: PluginSdkClient) -> Self { Self { client } }
    /// Calls `activatePackage` through the Core Link route.
    pub async fn activatePackage(&self, packageName: &str) -> Result<bool, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([("packageName".to_string(), operit_link::toCoreValue(&packageName).map_err(|error| CoreLinkError::internal(error.to_string()))?)]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "activatePackage", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `isPackageActivated` through the Core Link route.
    pub async fn isPackageActivated(&self, packageName: &str) -> Result<bool, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([("packageName".to_string(), operit_link::toCoreValue(&packageName).map_err(|error| CoreLinkError::internal(error.to_string()))?)]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "isPackageActivated", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `usePackage` through the Core Link route.
    pub async fn usePackage(&self, packageName: &str) -> Result<String, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([("packageName".to_string(), operit_link::toCoreValue(&packageName).map_err(|error| CoreLinkError::internal(error.to_string()))?)]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "usePackage", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `executeUsePackageTool` through the Core Link route.
    pub async fn executeUsePackageTool(&self, toolName: &str, packageName: &str) -> Result<SdkToolResult, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([("toolName".to_string(), operit_link::toCoreValue(&toolName).map_err(|error| CoreLinkError::internal(error.to_string()))?), ("packageName".to_string(), operit_link::toCoreValue(&packageName).map_err(|error| CoreLinkError::internal(error.to_string()))?)]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "executeUsePackageTool", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `getEnabledPackageNames` through the Core Link route.
    pub async fn getEnabledPackageNames(&self) -> Result<Vec<String>, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "getEnabledPackageNames", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `isPackageEnabled` through the Core Link route.
    pub async fn isPackageEnabled(&self, packageName: &str) -> Result<bool, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([("packageName".to_string(), operit_link::toCoreValue(&packageName).map_err(|error| CoreLinkError::internal(error.to_string()))?)]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "isPackageEnabled", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `getActivePackageNames` through the Core Link route.
    pub async fn getActivePackageNames(&self) -> Result<Vec<String>, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "getActivePackageNames", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `enablePackage` through the Core Link route.
    pub async fn enablePackage(&self, packageName: &str) -> Result<String, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([("packageName".to_string(), operit_link::toCoreValue(&packageName).map_err(|error| CoreLinkError::internal(error.to_string()))?)]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "enablePackage", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `disablePackage` through the Core Link route.
    pub async fn disablePackage(&self, packageName: &str) -> Result<String, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([("packageName".to_string(), operit_link::toCoreValue(&packageName).map_err(|error| CoreLinkError::internal(error.to_string()))?)]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "disablePackage", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `getToolPkgPluginContainerDetails` through the Core Link route.
    pub async fn getToolPkgPluginContainerDetails(&self, useEnglish: bool) -> Result<Vec<SdkToolPkgContainerDetails>, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([("useEnglish".to_string(), operit_link::toCoreValue(&useEnglish).map_err(|error| CoreLinkError::internal(error.to_string()))?)]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "getToolPkgPluginContainerDetails", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `getToolPkgContainerRuntimes` through the Core Link route.
    pub async fn getToolPkgContainerRuntimes(&self) -> Result<Vec<SdkToolPkgContainerRuntime>, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "getToolPkgContainerRuntimes", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `getToolPkgContainerOrder` through the Core Link route.
    pub async fn getToolPkgContainerOrder(&self) -> Result<Vec<String>, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "getToolPkgContainerOrder", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `setToolPkgContainerOrder` through the Core Link route.
    pub async fn setToolPkgContainerOrder(&self, packageNames: Vec<String>) -> Result<(), CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([("packageNames".to_string(), operit_link::toCoreValue(&packageNames).map_err(|error| CoreLinkError::internal(error.to_string()))?)]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "setToolPkgContainerOrder", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `getToolPkgContainerDetails` through the Core Link route.
    pub async fn getToolPkgContainerDetails(&self, packageName: &str, useEnglish: bool) -> Result<Option<SdkToolPkgContainerDetails>, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([("packageName".to_string(), operit_link::toCoreValue(&packageName).map_err(|error| CoreLinkError::internal(error.to_string()))?), ("useEnglish".to_string(), operit_link::toCoreValue(&useEnglish).map_err(|error| CoreLinkError::internal(error.to_string()))?)]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "getToolPkgContainerDetails", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `readToolPkgLogoBytes` through the Core Link route.
    pub async fn readToolPkgLogoBytes(&self, packageName: &str) -> Result<Option<SdkToolPkgLogoBytes>, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([("packageName".to_string(), operit_link::toCoreValue(&packageName).map_err(|error| CoreLinkError::internal(error.to_string()))?)]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "readToolPkgLogoBytes", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `getEffectivePackageTools` through the Core Link route.
    pub async fn getEffectivePackageTools(&self, packageName: &str) -> Result<Option<SdkToolPackage>, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([("packageName".to_string(), operit_link::toCoreValue(&packageName).map_err(|error| CoreLinkError::internal(error.to_string()))?)]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "getEffectivePackageTools", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `getPackageTools` through the Core Link route.
    pub async fn getPackageTools(&self, packageName: &str) -> Result<Option<SdkToolPackage>, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([("packageName".to_string(), operit_link::toCoreValue(&packageName).map_err(|error| CoreLinkError::internal(error.to_string()))?)]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "getPackageTools", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `getAvailablePackages` through the Core Link route.
    pub async fn getAvailablePackages(&self) -> Result<std::collections::BTreeMap<String, SdkToolPackage>, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "getAvailablePackages", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
    /// Calls `getToolPkgLoadIssues` through the Core Link route.
    pub async fn getToolPkgLoadIssues(&self) -> Result<Vec<SdkToolPkgLoadIssue>, CoreLinkError> {
        let args = CoreValue::Map(std::collections::BTreeMap::from([]));
        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, "core/application.packageManager", "getToolPkgLoadIssues", args)).await;
        let value = response.result?;
        decode_messagepack_value(&value)
    }
}

