use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::javascript::{JsExecutionEngine, ToolPkgConfigScope, ToolPkgMainRegistrationCapture};
use crate::package::LocalizedText;
use crate::toolpkg::ToolPkgCommonPluginConstants::*;
use crate::toolpkg::ToolPkgParser::{
    ToolPkgMainRegistration, ToolPkgMainRegistrationParseResult, ToolPkgMarketOrigin,
    ToolPkgRegisteredAiProvider, ToolPkgRegisteredAppLifecycleHook,
    ToolPkgRegisteredChatComposerSlot,
    ToolPkgRegisteredChatMessageMenuItem, ToolPkgRegisteredCoreCommand,
    ToolPkgRegisteredDesktopWidget, ToolPkgRegisteredFunctionHook, ToolPkgRegisteredHostEventHook,
    ToolPkgRegisteredNavigationEntry, ToolPkgRegisteredTagFunctionHook, ToolPkgRegisteredUiModule,
    ToolPkgRegisteredUiRoute, ToolPkgRegisteredManifestExtension,
};

/// Executes and validates the `registerToolPkg` declaration exported by a main script.
pub struct ToolPkgMainRegistrationScriptParser;

impl ToolPkgMainRegistrationScriptParser {
    /// Parses a device-scope ToolPkg main script without preloaded text resources.
    pub fn parse(
        script: &str,
        toolPkgId: &str,
        mainScriptPath: &str,
        jsEngine: &dyn JsExecutionEngine,
    ) -> ToolPkgMainRegistrationParseResult {
        Self::parseWithTextResources(
            script,
            toolPkgId,
            mainScriptPath,
            crate::toolpkg::ToolPkgApiVersion::CURRENT_TOOLPKG_API_VERSION,
            ToolPkgConfigScope::Device,
            jsEngine,
            None,
        )
    }

    /// Parses ToolPkg main-script registrations with archive text resources available to JavaScript.
    #[allow(non_snake_case)]
    pub(crate) fn parseWithTextResources(
        script: &str,
        toolPkgId: &str,
        mainScriptPath: &str,
        apiVersion: &str,
        configScope: ToolPkgConfigScope,
        jsEngine: &dyn JsExecutionEngine,
        textResources: Option<Arc<BTreeMap<String, String>>>,
    ) -> ToolPkgMainRegistrationParseResult {
        let mut params = BTreeMap::new();
        params.insert(
            "toolPkgId".to_string(),
            Value::String(toolPkgId.to_string()),
        );
        params.insert(
            "__operit_ui_package_name".to_string(),
            Value::String(toolPkgId.to_string()),
        );
        params.insert(
            "__operit_plugin_id".to_string(),
            Value::String(format!("registerToolPkg:{toolPkgId}")),
        );
        params.insert("__operit_registration_mode".to_string(), Value::Bool(true));
        // The scope is fixed before module evaluation, not looked up through the loading manager.
        params.insert(
            "__operit_registration_config_scope".to_string(),
            Value::String(configScope.as_str().to_string()),
        );
        params.insert(
            "__operit_script_screen".to_string(),
            Value::String(mainScriptPath.to_string()),
        );
        params.insert(
            "__operit_toolpkg_api_version".to_string(),
            Value::String(apiVersion.to_string()),
        );

        let capturedResult = jsEngine
            .execute_toolpkg_main_registration_function_with_text_resources(
                script,
                "registerToolPkg",
                &params,
                textResources,
            );
        let captured = match capturedResult {
            std::result::Result::Ok(captured) => captured,
            std::result::Result::Err(ref error) => {
                return ToolPkgMainRegistrationParseResult::Failure {
                    message: buildDeveloperFacingFailureMessage(mainScriptPath, &error.to_string()),
                }
            }
        };

        let registration = parseCapturedRegistration(captured, toolPkgId);
        match registration {
            Ok(registration) => ToolPkgMainRegistrationParseResult::Success { registration },
            Err(error) => ToolPkgMainRegistrationParseResult::Failure {
                message: buildDeveloperFacingFailureMessage(mainScriptPath, &error),
            },
        }
    }
}

/// Builds a compact error that points developers to the failing main script.
#[allow(non_snake_case)]
fn buildDeveloperFacingFailureMessage(mainScriptPath: &str, error: &str) -> String {
    let compactMessage = error
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("Exception");
    format!(
        "main script '{mainScriptPath}' failed while loading or running registerToolPkg(): {compactMessage}"
    )
}

/// Converts captured JSON registration payloads into validated runtime models.
#[allow(non_snake_case)]
fn parseCapturedRegistration(
    captured: ToolPkgMainRegistrationCapture,
    toolPkgId: &str,
) -> Result<ToolPkgMainRegistration, String> {
    let registration = ToolPkgMainRegistration {
        marketOrigin: parseMarketOrigin(captured.marketOrigin, toolPkgId)?,
        publicApis: parseRegisteredItems(&captured.publicApis, "public_api", toolPkgId)?,
        toolboxUiModules: parseRegisteredItems(
            &captured.toolboxUiModules,
            TOOLPKG_REGISTRATION_TOOLBOX_UI_MODULE,
            toolPkgId,
        )?,
        uiRoutes: parseRegisteredItems(
            &captured.uiRoutes,
            TOOLPKG_REGISTRATION_UI_ROUTE,
            toolPkgId,
        )?,
        chatComposerSlots: parseRegisteredItems(
            &captured.chatComposerSlots,
            TOOLPKG_REGISTRATION_CHAT_COMPOSER_SLOT,
            toolPkgId,
        )?,
        navigationEntries: parseRegisteredItems(
            &captured.navigationEntries,
            TOOLPKG_REGISTRATION_NAVIGATION_ENTRY,
            toolPkgId,
        )?,
        desktopWidgets: parseRegisteredItems(
            &captured.desktopWidgets,
            TOOLPKG_REGISTRATION_DESKTOP_WIDGET,
            toolPkgId,
        )?,
        appLifecycleHooks: parseRegisteredItems(
            &captured.appLifecycleHooks,
            TOOLPKG_REGISTRATION_APP_LIFECYCLE_HOOK,
            toolPkgId,
        )?,
        messageProcessingPlugins: parseRegisteredItems(
            &captured.messageProcessingPlugins,
            TOOLPKG_REGISTRATION_MESSAGE_PROCESSING_PLUGIN,
            toolPkgId,
        )?,
        xmlRenderPlugins: parseRegisteredItems(
            &captured.xmlRenderPlugins,
            TOOLPKG_REGISTRATION_XML_RENDER_PLUGIN,
            toolPkgId,
        )?,
        inputMenuTogglePlugins: parseRegisteredItems(
            &captured.inputMenuTogglePlugins,
            TOOLPKG_REGISTRATION_INPUT_MENU_TOGGLE_PLUGIN,
            toolPkgId,
        )?,
        chatInputHooks: parseRegisteredItems(
            &captured.chatInputHooks,
            TOOLPKG_REGISTRATION_CHAT_INPUT_HOOK,
            toolPkgId,
        )?,
        chatViewHooks: parseRegisteredItems(
            &captured.chatViewHooks,
            TOOLPKG_REGISTRATION_CHAT_VIEW_HOOK,
            toolPkgId,
        )?,
        chatLifecycleHooks: parseRegisteredItems(
            &captured.chatLifecycleHooks,
            TOOLPKG_REGISTRATION_CHAT_LIFECYCLE_HOOK,
            toolPkgId,
        )?,
        chatMessageHooks: parseRegisteredItems(
            &captured.chatMessageHooks,
            TOOLPKG_REGISTRATION_CHAT_MESSAGE_HOOK,
            toolPkgId,
        )?,
        chatMessageMenuItems: parseRegisteredItems(
            &captured.chatMessageMenuItems,
            TOOLPKG_REGISTRATION_CHAT_MESSAGE_MENU_ITEM,
            toolPkgId,
        )?,
        chatRuntimeHooks: parseRegisteredItems(
            &captured.chatRuntimeHooks,
            TOOLPKG_REGISTRATION_CHAT_RUNTIME_HOOK,
            toolPkgId,
        )?,
        hostEventHooks: parseRegisteredItems(
            &captured.hostEventHooks,
            TOOLPKG_REGISTRATION_HOST_EVENT_HOOK,
            toolPkgId,
        )?,
        toolLifecycleHooks: parseRegisteredItems(
            &captured.toolLifecycleHooks,
            TOOLPKG_REGISTRATION_TOOL_LIFECYCLE_HOOK,
            toolPkgId,
        )?,
        promptInputHooks: parseRegisteredItems(
            &captured.promptInputHooks,
            TOOLPKG_REGISTRATION_PROMPT_INPUT_HOOK,
            toolPkgId,
        )?,
        promptHistoryHooks: parseRegisteredItems(
            &captured.promptHistoryHooks,
            TOOLPKG_REGISTRATION_PROMPT_HISTORY_HOOK,
            toolPkgId,
        )?,
        promptEstimateHistoryHooks: parseRegisteredItems(
            &captured.promptEstimateHistoryHooks,
            TOOLPKG_REGISTRATION_PROMPT_ESTIMATE_HISTORY_HOOK,
            toolPkgId,
        )?,
        systemPromptComposeHooks: parseRegisteredItems(
            &captured.systemPromptComposeHooks,
            TOOLPKG_REGISTRATION_SYSTEM_PROMPT_COMPOSE_HOOK,
            toolPkgId,
        )?,
        toolPromptComposeHooks: parseRegisteredItems(
            &captured.toolPromptComposeHooks,
            TOOLPKG_REGISTRATION_TOOL_PROMPT_COMPOSE_HOOK,
            toolPkgId,
        )?,
        promptFinalizeHooks: parseRegisteredItems(
            &captured.promptFinalizeHooks,
            TOOLPKG_REGISTRATION_PROMPT_FINALIZE_HOOK,
            toolPkgId,
        )?,
        promptEstimateFinalizeHooks: parseRegisteredItems(
            &captured.promptEstimateFinalizeHooks,
            TOOLPKG_REGISTRATION_PROMPT_ESTIMATE_FINALIZE_HOOK,
            toolPkgId,
        )?,
        summaryGenerateHooks: parseRegisteredItems(
            &captured.summaryGenerateHooks,
            TOOLPKG_REGISTRATION_SUMMARY_GENERATE_HOOK,
            toolPkgId,
        )?,
        coreCommands: parseRegisteredItems(
            &captured.coreCommands,
            TOOLPKG_REGISTRATION_CORE_COMMAND,
            toolPkgId,
        )?,
        aiProviders: parseRegisteredItems(
            &captured.aiProviders,
            TOOLPKG_REGISTRATION_AI_PROVIDER,
            toolPkgId,
        )?,
        manifestExtensions: parseRegisteredItems(
            &captured.manifestExtensions,
            TOOLPKG_REGISTRATION_MANIFEST_EXTENSION,
            toolPkgId,
        )?,
    };
    if registration.chatLifecycleHooks.len() > 1 {
        return Err(format!("Duplicate chat lifecycle hook owner: {toolPkgId}"));
    }
    validateEmbeddedRoutes(&registration, toolPkgId)?;
    Ok(registration)
}

/// Requires embedded surfaces to reference exactly one package-owned Compose DSL route in the captured registration.
#[allow(non_snake_case)]
fn validateEmbeddedRoutes(registration: &ToolPkgMainRegistration, toolPkgId: &str) -> Result<(), String> {
    let routes = registration.uiRoutes.iter().map(|route| (route.routeId.clone(), route.runtime.as_str()))
        .chain(registration.toolboxUiModules.iter().map(|module| (buildToolPkgRouteId(toolPkgId, module.id.trim()), module.runtime.as_str())))
        .collect::<Vec<_>>();
    for (index, entry) in registration.navigationEntries.iter().enumerate() {
        if entry.surface != TOOLPKG_NAV_SURFACE_CHAT_SIDEBAR_TABS && entry.surface != TOOLPKG_NAV_SURFACE_CHAT_INPUT_MENU { continue; }
        let surface = &entry.surface;
        let routeId = entry.routeId.as_deref().ok_or_else(|| format!("{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[{index}].route is required for {surface}"))?;
        if !routeId.starts_with(&format!("toolpkg:{toolPkgId}:ui:")) {
            return Err(format!("{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[{index}].route must belong to this package for {surface}: {routeId}"));
        }
        let matching = routes.iter().filter(|(route, _)| route == routeId).collect::<Vec<_>>();
        if matching.is_empty() {
            return Err(format!("{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[{index}].route not found: {routeId}"));
        }
        if matching.len() != 1 {
            return Err(format!("{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[{index}].route is duplicated for {surface}: {routeId}"));
        }
        if matching[0].1 != TOOLPKG_RUNTIME_COMPOSE_DSL {
            return Err(format!("{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[{index}].route must use compose_dsl for {surface}: {routeId}"));
        }
    }
    Ok(())
}

/// Normalizes the marketplace origin captured during main-script initialization.
fn parseMarketOrigin(
    origin: Option<ToolPkgMarketOrigin>,
    toolPkgId: &str,
) -> Result<Option<ToolPkgMarketOrigin>, String> {
    let Some(mut origin) = origin else {
        return Ok(None);
    };
    if origin.market.trim() != "Operit" || origin.toolpkgId.trim() != toolPkgId.trim() {
        return Ok(None);
    }
    origin.market = "Operit".to_string();
    origin.toolpkgId = toolPkgId.trim().to_string();
    origin.version = origin.version.trim().to_string();
    origin.author = origin
        .author
        .into_iter()
        .map(|author| author.trim().to_string())
        .filter(|author| !author.is_empty())
        .collect();
    Ok(Some(origin))
}

/// Deserializes, normalizes, and validates one registration collection.
#[allow(non_snake_case)]
fn parseRegisteredItems<T>(
    registrations: &[String],
    registryName: &str,
    toolPkgId: &str,
) -> Result<Vec<T>, String>
where
    T: DeserializeOwned + ValidateToolPkgRegistration,
{
    registrations
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            let mut item = serde_json::from_str::<T>(raw).map_err(|error| {
                format!("{registryName} payload[{index}] must be a JSON object: {error}")
            })?;
            item.normalize(registryName, index, toolPkgId);
            item.validate(registryName, index)?;
            Ok(item)
        })
        .collect()
}

/// Defines normalization and validation required by captured registration records.
trait ValidateToolPkgRegistration {
    /// Applies registry-specific defaults before validation.
    fn normalize(&mut self, registryName: &str, index: usize, toolPkgId: &str);

    /// Validates required fields after normalization.
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String>;
}

/// Rejects a blank required registration field with its registry position.
fn requireNotBlank(
    value: &str,
    fieldName: &str,
    registryName: &str,
    index: usize,
) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{registryName}[{index}].{fieldName} is required"));
    }
    Ok(())
}

/// Returns whether a localized text value contains visible content.
fn hasLocalizedTextContent(text: &LocalizedText) -> bool {
    text.values.values().any(|value| !value.trim().is_empty())
}

/// Creates a default-language localized text value.
fn localizedTextOf(value: &str) -> LocalizedText {
    LocalizedText {
        values: HashMap::from([("default".to_string(), value.to_string())]),
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredUiModule {
    /// Applies default runtime and title values to a UI module registration.
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {
        if self.runtime.trim().is_empty() {
            self.runtime = TOOLPKG_RUNTIME_COMPOSE_DSL.to_string();
        }
        if !hasLocalizedTextContent(&self.title) {
            self.title = localizedTextOf(self.id.trim());
        }
    }

    /// Validates the identifier and screen path of a UI module registration.
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.screen, "screen", registryName, index)?;
        if let Some(exportName) = &self.screenExport {
            requireNotBlank(exportName, "screenExport", registryName, index)?;
        }
        Ok(())
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredUiRoute {
    /// Generates missing route, runtime, and title values for a UI route registration.
    fn normalize(&mut self, _registryName: &str, _index: usize, toolPkgId: &str) {
        if self.routeId.trim().is_empty() {
            self.routeId = buildToolPkgRouteId(toolPkgId, self.id.trim());
        }
        if self.runtime.trim().is_empty() {
            self.runtime = TOOLPKG_RUNTIME_COMPOSE_DSL.to_string();
        }
        if !hasLocalizedTextContent(&self.title) {
            self.title = localizedTextOf(self.id.trim());
        }
    }

    /// Validates the identifier, screen path, and route id of a UI route registration.
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.screen, "screen", registryName, index)?;
        if let Some(exportName) = &self.screenExport {
            requireNotBlank(exportName, "screenExport", registryName, index)?;
        }
        requireNotBlank(&self.routeId, "route", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredChatComposerSlot {
    /// Preserves explicitly declared chat composer slot metadata.
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {}

    /// Validates one chat composer slot contribution.
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.slot, "slot", registryName, index)?;
        requireNotBlank(&self.screen, "screen", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredNavigationEntry {
    /// Generates a title from the navigation entry identifier when omitted.
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {
        if !hasLocalizedTextContent(&self.title) {
            self.title = localizedTextOf(self.id.trim());
        }
    }

    /// Validates exact surfaces and requires embedded UI routes without native action callbacks.
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireToolPkgNavigationSurface(&self.surface).map_err(|error| {
            format!("{registryName}[{index}].surface {error}")
        })?;
        if self.surface == TOOLPKG_NAV_SURFACE_CHAT_ATTACHMENTS || self.surface == TOOLPKG_NAV_SURFACE_CHAT_SIDEBAR_TABS || self.surface == TOOLPKG_NAV_SURFACE_CHAT_INPUT_MENU {
            if self.action.is_some() {
                return Err(format!("{registryName}[{index}].action is unsupported for {}", self.surface));
            }
            let route = self.routeId.as_deref().ok_or_else(|| {
                format!("{registryName}[{index}].route is required for {}", self.surface)
            })?;
            requireNotBlank(route, "route", registryName, index)?;
        }
        if self
            .routeId
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            && self.action.is_none()
        {
            return Err(format!(
                "{registryName}[{index}].route or action is required"
            ));
        }
        Ok(())
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredDesktopWidget {
    /// Generates missing render route and title values for a desktop widget.
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {
        if self.renderRouteId.trim().is_empty() {
            self.renderRouteId = self.routeId.trim().to_string();
        }
        if !hasLocalizedTextContent(&self.title) {
            self.title = localizedTextOf(self.id.trim());
        }
    }

    /// Validates the identifier and route references of a desktop widget.
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.routeId, "route", registryName, index)?;
        requireNotBlank(&self.renderRouteId, "render", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredAppLifecycleHook {
    /// Leaves an application lifecycle hook unchanged before validation.
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {}

    /// Validates the id, event, and function of an application lifecycle hook.
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.event, "event", registryName, index)?;
        requireNotBlank(&self.function, "function", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredFunctionHook {
    /// Leaves a function hook unchanged before validation.
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {}

    /// Validates the id and function of a generic function hook.
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.function, "function", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredCoreCommand {
    /// Leaves a Core command unchanged before validation.
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {}

    /// Validates all required metadata and the callback of a Core command.
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.name, "name", registryName, index)?;
        if !hasLocalizedTextContent(&self.title) {
            return Err(format!("{registryName}[{index}].title is required"));
        }
        if !hasLocalizedTextContent(&self.description) {
            return Err(format!("{registryName}[{index}].description is required"));
        }
        requireNotBlank(&self.usage, "usage", registryName, index)?;
        requireNotBlank(&self.function, "function", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredChatMessageMenuItem {
    /// Generates a title from the menu item identifier when omitted.
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {
        if !hasLocalizedTextContent(&self.title) {
            self.title = localizedTextOf(self.id.trim());
        }
    }

    /// Validates the id and function of a chat message menu item.
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.function, "function", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredHostEventHook {
    /// Leaves a host event hook unchanged before validation.
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {}

    /// Validates the id, source, and function of a host event hook.
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.source, "source", registryName, index)?;
        requireNotBlank(&self.function, "function", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredTagFunctionHook {
    /// Leaves a tag function hook unchanged before validation.
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {}

    /// Validates the id, tag, and function of a tag function hook.
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.tag, "tag", registryName, index)?;
        requireNotBlank(&self.function, "function", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredManifestExtension {
    /// Normalizes manifest extension keys before validation.
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {
        self.key = self.key.trim().to_string();
    }

    /// Validates one manifest extension handler registration.
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.key, "key", registryName, index)?;
        requireNotBlank(&self.function, "function", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredAiProvider {
    /// Generates a provider display name from its identifier when omitted.
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {
        if self.displayName.trim().is_empty() {
            self.displayName = self.id.trim().to_string();
        }
    }

    /// Validates all required handler functions of a ToolPkg AI provider.
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(
            &self.listModelsHandler.function,
            "listModels.function",
            registryName,
            index,
        )?;
        requireNotBlank(
            &self.sendMessageHandler.function,
            "sendMessage.function",
            registryName,
            index,
        )?;
        requireNotBlank(
            &self.testConnectionHandler.function,
            "testConnection.function",
            registryName,
            index,
        )?;
        requireNotBlank(
            &self.calculateInputTokensHandler.function,
            "calculateInputTokens.function",
            registryName,
            index,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::javascript::ToolPkgMainRegistrationCapture;

    /// Verifies Kotlin-style registration aliases normalize into runtime records.
    #[test]
    fn parses_kotlin_style_registration_fields() {
        let captured = ToolPkgMainRegistrationCapture {
            toolboxUiModules: vec![r#"{"id":"tools","screen":"ui/tools.js"}"#.to_string()],
            uiRoutes: vec![
                r#"{"id":"main","route":"main_route","screen":"ui/main.js"}"#.to_string(),
                r#"{"id":"auto","screen":"ui/auto.js"}"#.to_string(),
            ],
            navigationEntries: vec![
                r#"{"id":"nav","route":"main_route","surface":"toolbox"}"#.to_string()
            ],
            desktopWidgets: vec![r#"{"id":"widget","route":"main_route"}"#.to_string()],
            coreCommands: vec![r#"{
                    "id":"hello_command",
                    "name":"hello",
                    "title":{"en":"Hello","zh":"你好"},
                    "description":{"en":"Greets the user","zh":"问候用户"},
                    "usage":"/hello <name>",
                    "function":"runHello"
                }"#
            .to_string()],
            aiProviders: vec![r#"{
                    "id":"provider",
                    "listModels":{"function":"listModels"},
                    "sendMessage":{"function":"sendMessage"},
                    "testConnection":{"function":"testConnection"},
                    "calculateInputTokens":{"function":"calculateInputTokens"}
                }"#
            .to_string()],
            ..Default::default()
        };

        let registration = parseCapturedRegistration(captured, "demo_toolpkg").unwrap();

        assert_eq!(
            registration.toolboxUiModules[0].runtime,
            TOOLPKG_RUNTIME_COMPOSE_DSL
        );
        assert_eq!(
            registration.toolboxUiModules[0].title.resolve(true),
            "tools"
        );

        assert_eq!(registration.uiRoutes[0].routeId, "main_route");
        assert_eq!(
            registration.uiRoutes[0].runtime,
            TOOLPKG_RUNTIME_COMPOSE_DSL
        );
        assert_eq!(registration.uiRoutes[0].title.resolve(true), "main");
        assert_eq!(
            registration.uiRoutes[1].routeId,
            buildToolPkgRouteId("demo_toolpkg", "auto")
        );

        assert_eq!(
            registration.navigationEntries[0].routeId.as_deref(),
            Some("main_route")
        );
        assert_eq!(registration.navigationEntries[0].title.resolve(true), "nav");

        assert_eq!(registration.desktopWidgets[0].renderRouteId, "main_route");
        assert_eq!(registration.desktopWidgets[0].title.resolve(true), "widget");

        assert_eq!(registration.coreCommands[0].name, "hello");
        assert_eq!(registration.coreCommands[0].title.resolve(true), "Hello");
        assert_eq!(registration.coreCommands[0].function, "runHello");

        assert_eq!(registration.aiProviders[0].displayName, "provider");
        assert_eq!(
            registration.aiProviders[0].listModelsHandler.function,
            "listModels"
        );
        assert_eq!(
            registration.aiProviders[0]
                .calculateInputTokensHandler
                .function,
            "calculateInputTokens"
        );
    }

    /// Preserves opaque attachment-route JSON through the actual captured-registration parser.
    #[test]
    fn parses_attachment_surface_and_opaque_params() {
        let input = serde_json::json!({
            "id": "attachment", "surface": "chat_attachments", "route": "plugin-owned-route",
            "title": "Attachment", "icon": "attachment", "order": 12,
            "params": { "screen": "plugin-view", "ids": ["9223372036854775807"], "nested": [null, true, 1.5] }
        });
        let entries = parseRegisteredItems::<ToolPkgRegisteredNavigationEntry>(
            &[input.to_string()], TOOLPKG_REGISTRATION_NAVIGATION_ENTRY, "sample",
        ).unwrap();
        assert_eq!(entries[0].surface, TOOLPKG_NAV_SURFACE_CHAT_ATTACHMENTS);
        assert_eq!(entries[0].routeId.as_deref(), Some("plugin-owned-route"));
        assert_eq!(entries[0].params.as_ref(), Some(&input["params"]));
        assert_eq!(entries[0].icon.as_deref(), Some("attachment"));
        assert_eq!(entries[0].order, 12);
    }

    /// Rejects unsupported surface spellings instead of silently normalizing them.
    #[test]
    fn rejects_inexact_navigation_surfaces() {
        for surface in ["", "Toolbox", "CHAT_ATTACHMENTS", "chat_attachments ", " chat_attachments", "attachment"] {
            let input = serde_json::json!({ "id": "entry", "surface": surface, "route": "route" });
            let error = parseRegisteredItems::<ToolPkgRegisteredNavigationEntry>(
                &[input.to_string()], TOOLPKG_REGISTRATION_NAVIGATION_ENTRY, "sample",
            ).unwrap_err();
            assert_eq!(error, format!("{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[0].surface is unsupported: {surface}"));
        }
    }

    /// Requires a route for an attachment entry without supplying an action or default params.
    #[test]
    fn requires_attachment_route_and_keeps_params_optional() {
        let invalid = serde_json::json!({ "id": "entry", "surface": "chat_attachments" });
        let error = parseRegisteredItems::<ToolPkgRegisteredNavigationEntry>(
            &[invalid.to_string()], TOOLPKG_REGISTRATION_NAVIGATION_ENTRY, "sample",
        ).unwrap_err();
        assert_eq!(error, format!("{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[0].route is required for chat_attachments"));
        let valid = serde_json::json!({ "id": "entry", "surface": "chat_attachments", "route": "route" });
        let entries = parseRegisteredItems::<ToolPkgRegisteredNavigationEntry>(
            &[valid.to_string()], TOOLPKG_REGISTRATION_NAVIGATION_ENTRY, "sample",
        ).unwrap();
        assert!(entries[0].params.is_none());
    }

    /// Rejects route-plus-action attachment entries while preserving action callbacks on every preexisting surface.
    #[test]
    fn rejects_attachment_actions_and_retains_existing_surface_actions() {
        let invalid = serde_json::json!({ "id": "entry", "surface": "chat_attachments", "route": "registered-route", "action": { "function": "existingAction" } });
        let error = parseRegisteredItems::<ToolPkgRegisteredNavigationEntry>(
            &[invalid.to_string()], TOOLPKG_REGISTRATION_NAVIGATION_ENTRY, "sample",
        ).unwrap_err();
        assert_eq!(error, format!("{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[0].action is unsupported for chat_attachments"));
        for surface in ["toolbox", "main_sidebar_plugins", "app_bar"] {
            let valid = serde_json::json!({ "id": "entry", "surface": surface, "action": { "function": "existingAction" } });
            let entries = parseRegisteredItems::<ToolPkgRegisteredNavigationEntry>(
                &[valid.to_string()], TOOLPKG_REGISTRATION_NAVIGATION_ENTRY, "sample",
            ).unwrap();
            assert_eq!(entries[0].action.as_ref().unwrap().function, "existingAction");
        }
    }

    /// Supplies captured route and navigation JSON to the actual registration parser without a JavaScript or host substitute.
    fn sidebar_capture(routes: Vec<Value>, entry: Value) -> ToolPkgMainRegistrationCapture {
        ToolPkgMainRegistrationCapture {
            uiRoutes: routes.into_iter().map(|route| route.to_string()).collect(),
            navigationEntries: vec![entry.to_string()],
            ..Default::default()
        }
    }

    /// Preserves two independent package tab registrations and every localized title, icon, order and opaque JSON field.
    #[test]
    fn parses_sidebar_tabs_for_two_arbitrary_packages() {
        for package in ["com.example.alpha", "org.example.beta"] {
            let route = buildToolPkgRouteId(package, "panel");
            let params = serde_json::json!({ "opaque": ["9223372036854775807", null, true, 1.5] });
            let captured = sidebar_capture(
                vec![serde_json::json!({ "id": "panel", "route": route, "runtime": "compose_dsl", "screen": "ui/panel.js" })],
                serde_json::json!({ "id": "panel-tab", "surface": "chat_sidebar_tabs", "route": route,
                    "title": { "en": "Independent panel", "zh": "独立面板" }, "icon": "Dashboard", "order": 23, "params": params }),
            );
            let registration = parseCapturedRegistration(captured, package).unwrap();
            let entry = &registration.navigationEntries[0];
            assert_eq!(entry.routeId.as_deref(), Some(route.as_str()));
            assert_eq!(entry.params, Some(params));
            assert_eq!(entry.title.resolve(true), "Independent panel");
            assert_eq!(entry.title.resolve(false), "独立面板");
            assert_eq!(entry.icon.as_deref(), Some("Dashboard"));
            assert_eq!(entry.order, 23);
            assert!(entry.action.is_none());
        }
    }

    /// Rejects sidebar actions and missing routes through typed captured metadata before producing a runtime registration.
    #[test]
    fn sidebar_capture_requires_routes_without_actions() {
        let missing = sidebar_capture(vec![], serde_json::json!({ "id": "tab", "surface": "chat_sidebar_tabs" }));
        assert_eq!(parseCapturedRegistration(missing, "sample").unwrap_err(), format!("{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[0].route is required for chat_sidebar_tabs"));
        let action = sidebar_capture(vec![], serde_json::json!({ "id": "tab", "surface": "chat_sidebar_tabs", "route": "toolpkg:sample:ui:panel", "action": { "function": "callback" } }));
        assert_eq!(parseCapturedRegistration(action, "sample").unwrap_err(), format!("{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[0].action is unsupported for chat_sidebar_tabs"));
    }

    /// Rejects foreign and undeclared route ownership even when the captured JSON claims a valid sidebar surface.
    #[test]
    fn sidebar_capture_rejects_foreign_and_missing_routes() {
        let own_route = serde_json::json!({ "id": "panel", "route": "toolpkg:sample:ui:panel", "runtime": "compose_dsl", "screen": "ui/panel.js" });
        for route in ["toolpkg:other:ui:panel", "toolpkg:sample:ui:missing", "toolpkg:sample:ui:PANEL"] {
            let captured = sidebar_capture(vec![own_route.clone()], serde_json::json!({ "id": "tab", "surface": "chat_sidebar_tabs", "route": route }));
            let error = parseCapturedRegistration(captured, "sample").unwrap_err();
            let expected = if route == "toolpkg:other:ui:panel" {
                format!("{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[0].route must belong to this package for chat_sidebar_tabs: {route}")
            } else { format!("{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[0].route not found: {route}") };
            assert_eq!(error, expected);
        }
    }

    /// Rejects duplicate declarations and non-Compose route runtimes before a sidebar can be embedded by the host.
    #[test]
    fn sidebar_capture_requires_one_compose_route() {
        let route = serde_json::json!({ "id": "panel", "route": "toolpkg:sample:ui:panel", "runtime": "compose_dsl", "screen": "ui/panel.js" });
        let entry = serde_json::json!({ "id": "tab", "surface": "chat_sidebar_tabs", "route": "toolpkg:sample:ui:panel" });
        let duplicate = sidebar_capture(vec![route.clone(), route.clone()], entry.clone());
        assert_eq!(parseCapturedRegistration(duplicate, "sample").unwrap_err(), format!("{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[0].route is duplicated for chat_sidebar_tabs: toolpkg:sample:ui:panel"));
        let mut unsupported = route; unsupported["runtime"] = Value::String("unsupported".to_string());
        let invalid = sidebar_capture(vec![unsupported], entry);
        assert_eq!(parseCapturedRegistration(invalid, "sample").unwrap_err(), format!("{TOOLPKG_REGISTRATION_NAVIGATION_ENTRY}[0].route must use compose_dsl for chat_sidebar_tabs: toolpkg:sample:ui:panel"));
    }

}
