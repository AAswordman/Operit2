use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::core::tools::javascript::JsEngine::JsEngine;
use crate::core::tools::packTool::ToolPkgCommonPluginConstants::*;
use crate::core::tools::packTool::ToolPkgParser::{
    ToolPkgMainRegistration, ToolPkgMainRegistrationParseResult, ToolPkgRegisteredAiProvider,
    ToolPkgRegisteredAppLifecycleHook, ToolPkgRegisteredDesktopWidget,
    ToolPkgRegisteredFunctionHook, ToolPkgRegisteredNavigationEntry,
    ToolPkgRegisteredTagFunctionHook, ToolPkgRegisteredUiModule, ToolPkgRegisteredUiRoute,
};

pub struct ToolPkgMainRegistrationScriptParser;

impl ToolPkgMainRegistrationScriptParser {
    pub fn parse(
        script: &str,
        toolPkgId: &str,
        mainScriptPath: &str,
        jsEngine: &JsEngine,
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

        let capturedResult: Result<
            crate::core::tools::javascript::JsToolPkgRegistration::ToolPkgMainRegistrationCapture,
            String,
        > = jsEngine.executeToolPkgMainRegistrationFunction(script, "registerToolPkg", &params);
        let captured = match capturedResult {
            std::result::Result::Ok(captured) => captured,
            std::result::Result::Err(ref error) => {
                return ToolPkgMainRegistrationParseResult::Failure {
                    message: buildDeveloperFacingFailureMessage(mainScriptPath, error.as_str()),
                }
            }
        };

        let registration = parseCapturedRegistration(captured);
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
    captured: crate::core::tools::javascript::JsToolPkgRegistration::ToolPkgMainRegistrationCapture,
) -> Result<ToolPkgMainRegistration, String> {
    Ok(ToolPkgMainRegistration {
        toolboxUiModules: parseRegisteredItems(
            &captured.toolboxUiModules,
            TOOLPKG_REGISTRATION_TOOLBOX_UI_MODULE,
        )?,
        uiRoutes: parseRegisteredItems(&captured.uiRoutes, TOOLPKG_REGISTRATION_UI_ROUTE)?,
        navigationEntries: parseRegisteredItems(
            &captured.navigationEntries,
            TOOLPKG_REGISTRATION_NAVIGATION_ENTRY,
        )?,
        desktopWidgets: parseRegisteredItems(
            &captured.desktopWidgets,
            TOOLPKG_REGISTRATION_DESKTOP_WIDGET,
        )?,
        appLifecycleHooks: parseRegisteredItems(
            &captured.appLifecycleHooks,
            TOOLPKG_REGISTRATION_APP_LIFECYCLE_HOOK,
        )?,
        messageProcessingPlugins: parseRegisteredItems(
            &captured.messageProcessingPlugins,
            TOOLPKG_REGISTRATION_MESSAGE_PROCESSING_PLUGIN,
        )?,
        xmlRenderPlugins: parseRegisteredItems(
            &captured.xmlRenderPlugins,
            TOOLPKG_REGISTRATION_XML_RENDER_PLUGIN,
        )?,
        inputMenuTogglePlugins: parseRegisteredItems(
            &captured.inputMenuTogglePlugins,
            TOOLPKG_REGISTRATION_INPUT_MENU_TOGGLE_PLUGIN,
        )?,
        chatInputHooks: parseRegisteredItems(
            &captured.chatInputHooks,
            TOOLPKG_REGISTRATION_CHAT_INPUT_HOOK,
        )?,
        chatViewHooks: parseRegisteredItems(
            &captured.chatViewHooks,
            TOOLPKG_REGISTRATION_CHAT_VIEW_HOOK,
        )?,
        toolLifecycleHooks: parseRegisteredItems(
            &captured.toolLifecycleHooks,
            TOOLPKG_REGISTRATION_TOOL_LIFECYCLE_HOOK,
        )?,
        promptInputHooks: parseRegisteredItems(
            &captured.promptInputHooks,
            TOOLPKG_REGISTRATION_PROMPT_INPUT_HOOK,
        )?,
        promptHistoryHooks: parseRegisteredItems(
            &captured.promptHistoryHooks,
            TOOLPKG_REGISTRATION_PROMPT_HISTORY_HOOK,
        )?,
        promptEstimateHistoryHooks: parseRegisteredItems(
            &captured.promptEstimateHistoryHooks,
            TOOLPKG_REGISTRATION_PROMPT_ESTIMATE_HISTORY_HOOK,
        )?,
        systemPromptComposeHooks: parseRegisteredItems(
            &captured.systemPromptComposeHooks,
            TOOLPKG_REGISTRATION_SYSTEM_PROMPT_COMPOSE_HOOK,
        )?,
        toolPromptComposeHooks: parseRegisteredItems(
            &captured.toolPromptComposeHooks,
            TOOLPKG_REGISTRATION_TOOL_PROMPT_COMPOSE_HOOK,
        )?,
        promptFinalizeHooks: parseRegisteredItems(
            &captured.promptFinalizeHooks,
            TOOLPKG_REGISTRATION_PROMPT_FINALIZE_HOOK,
        )?,
        promptEstimateFinalizeHooks: parseRegisteredItems(
            &captured.promptEstimateFinalizeHooks,
            TOOLPKG_REGISTRATION_PROMPT_ESTIMATE_FINALIZE_HOOK,
        )?,
        summaryGenerateHooks: parseRegisteredItems(
            &captured.summaryGenerateHooks,
            TOOLPKG_REGISTRATION_SUMMARY_GENERATE_HOOK,
        )?,
        aiProviders: parseRegisteredItems(&captured.aiProviders, TOOLPKG_REGISTRATION_AI_PROVIDER)?,
    })
}

#[allow(non_snake_case)]
fn parseRegisteredItems<T>(registrations: &[String], registryName: &str) -> Result<Vec<T>, String>
where
    T: DeserializeOwned + ValidateToolPkgRegistration,
{
    registrations
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            let item = serde_json::from_str::<T>(raw).map_err(|error| {
                format!("{registryName} payload[{index}] must be a JSON object: {error}")
            })?;
            item.validate(registryName, index)?;
            Ok(item)
        })
        .collect()
}

trait ValidateToolPkgRegistration {
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

impl ValidateToolPkgRegistration for ToolPkgRegisteredUiModule {
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.screen, "screen", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredUiRoute {
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.screen, "screen", registryName, index)?;
        requireNotBlank(&self.routeId, "route", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredNavigationEntry {
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
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.routeId, "route", registryName, index)?;
        requireNotBlank(&self.renderRouteId, "render", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredAppLifecycleHook {
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.event, "event", registryName, index)?;
        requireNotBlank(&self.function, "function", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredFunctionHook {
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.function, "function", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredTagFunctionHook {
    fn validate(&self, registryName: &str, index: usize) -> Result<(), String> {
        requireNotBlank(&self.id, "id", registryName, index)?;
        requireNotBlank(&self.tag, "tag", registryName, index)?;
        requireNotBlank(&self.function, "function", registryName, index)
    }
}

impl ValidateToolPkgRegistration for ToolPkgRegisteredAiProvider {
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
