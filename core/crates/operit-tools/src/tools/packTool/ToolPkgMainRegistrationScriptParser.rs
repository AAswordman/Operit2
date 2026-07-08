use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;

use operit_tools::tools::javascript::{JsExecutionEngine, ToolPkgMainRegistrationCapture};
use operit_tools::tools::packTool::ToolPkgCommonPluginConstants::*;
use operit_tools::tools::packTool::ToolPkgParser::{
    ToolPkgMainRegistration, ToolPkgMainRegistrationParseResult, ToolPkgRegisteredAiProvider,
    ToolPkgRegisteredAppLifecycleHook, ToolPkgRegisteredDesktopWidget,
    ToolPkgRegisteredFunctionHook, ToolPkgRegisteredHostEventHook,
    ToolPkgRegisteredNavigationEntry, ToolPkgRegisteredTagFunctionHook, ToolPkgRegisteredUiModule,
    ToolPkgRegisteredUiRoute,
};
use operit_tools::tools::ToolPackage::LocalizedText;

pub struct ToolPkgMainRegistrationScriptParser;

impl ToolPkgMainRegistrationScriptParser {
    pub fn parse(
        script: &str,
        toolPkgId: &str,
        mainScriptPath: &str,
        jsEngine: &dyn JsExecutionEngine,
    ) -> ToolPkgMainRegistrationParseResult {
        Self::parseWithTextResources(script, toolPkgId, mainScriptPath, jsEngine, None)
    }

    #[allow(non_snake_case)]
    pub(crate) fn parseWithTextResources(
        script: &str,
        toolPkgId: &str,
        mainScriptPath: &str,
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
        params.insert(
            "__operit_script_screen".to_string(),
            Value::String(mainScriptPath.to_string()),
        );

        let capturedResult: Result<ToolPkgMainRegistrationCapture, String> =
            jsEngine.executeToolPkgMainRegistrationFunctionWithTextResources(
            script,
            "registerToolPkg",
            &params,
            textResources,
        );
        let captured = match capturedResult {
            std::result::Result::Ok(captured) => captured,
            std::result::Result::Err(ref error) => {
                return ToolPkgMainRegistrationParseResult::Failure {
                    message: buildDeveloperFacingFailureMessage(mainScriptPath, error.as_str()),
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

#[allow(non_snake_case)]
fn parseCapturedRegistration(
    captured: ToolPkgMainRegistrationCapture,
    toolPkgId: &str,
) -> Result<ToolPkgMainRegistration, String> {
    Ok(ToolPkgMainRegistration {
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
        aiProviders: parseRegisteredItems(
            &captured.aiProviders,
            TOOLPKG_REGISTRATION_AI_PROVIDER,
            toolPkgId,
        )?,
    })
}

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

trait ValidateToolPkgRegistration {
    fn normalize(&mut self, registryName: &str, index: usize, toolPkgId: &str);
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String>;
}

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

fn hasLocalizedTextContent(text: &LocalizedText) -> bool {
    text.values.values().any(|value| !value.trim().is_empty())
}

fn localizedTextOf(value: &str) -> LocalizedText {
    LocalizedText {
        values: HashMap::from([("default".to_string(), value.to_string())]),
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredUiModule {
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {
        if self.runtime.trim().is_empty() {
            self.runtime = TOOLPKG_RUNTIME_COMPOSE_DSL.to_string();
        }
        if !hasLocalizedTextContent(&self.title) {
            self.title = localizedTextOf(self.id.trim());
        }
    }

    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.screen, "screen", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredUiRoute {
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

    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.screen, "screen", registryName, index)?;
        requireNotBlank(&self.routeId, "route", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredNavigationEntry {
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {
        if !hasLocalizedTextContent(&self.title) {
            self.title = localizedTextOf(self.id.trim());
        }
    }

    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
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
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {
        if self.renderRouteId.trim().is_empty() {
            self.renderRouteId = self.routeId.trim().to_string();
        }
        if !hasLocalizedTextContent(&self.title) {
            self.title = localizedTextOf(self.id.trim());
        }
    }

    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.routeId, "route", registryName, index)?;
        requireNotBlank(&self.renderRouteId, "render", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredAppLifecycleHook {
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {}

    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.event, "event", registryName, index)?;
        requireNotBlank(&self.function, "function", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredFunctionHook {
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {}

    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.function, "function", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredHostEventHook {
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {}

    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.source, "source", registryName, index)?;
        requireNotBlank(&self.function, "function", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredTagFunctionHook {
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {}

    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.tag, "tag", registryName, index)?;
        requireNotBlank(&self.function, "function", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredAiProvider {
    fn normalize(&mut self, _registryName: &str, _index: usize, _toolPkgId: &str) {
        if self.displayName.trim().is_empty() {
            self.displayName = self.id.trim().to_string();
        }
    }

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
    use operit_tools::tools::javascript::ToolPkgMainRegistrationCapture;

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
}
