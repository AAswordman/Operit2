use std::sync::{Mutex, OnceLock};

use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_XML_RENDER;
use operit_plugin_sdk::toolpkg::ToolPkgHookModels::ToolPkgXmlRenderHookObjectResult;
use operit_plugin_sdk::toolpkg::ToolPkgHooks::{
    decodeToolPkgHookResult, ToolPkgXmlRenderHookRegistration,
};
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;
use operit_util::ChainLogger::{self, PLUGIN_CHAIN};
use serde_json::Value;

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;

static XML_RENDER_HOOKS: OnceLock<Mutex<Vec<ToolPkgXmlRenderHookRegistration>>> = OnceLock::new();
static XML_RENDER_RUNTIME: OnceLock<ToolPkgBridgeRuntime> = OnceLock::new();

pub struct ToolPkgXmlRenderBridge;

impl ToolPkgXmlRenderBridge {
    /// Registers XML render hooks for one application runtime.
    pub fn register(runtime: ToolPkgBridgeRuntime) {
        XML_RENDER_RUNTIME.get_or_init(|| runtime.clone());
        let manager = runtime.package_manager();
        manager.addToolPkgRuntimeChangeListener(std::sync::Arc::new(|activeContainers| {
            ToolPkgXmlRenderBridge::syncToolPkgRegistrations(activeContainers);
        }));
    }

    /// Synchronizes active XML render hook registrations from enabled ToolPkg containers.
    #[allow(non_snake_case)]
    pub fn syncToolPkgRegistrations(activeContainers: Vec<ToolPkgContainerRuntime>) {
        let mut hooks = activeContainers
            .iter()
            .flat_map(|container| {
                container
                    .xmlRenderPlugins
                    .iter()
                    .map(|hook| ToolPkgXmlRenderHookRegistration {
                        containerPackageName: container.packageName.clone(),
                        pluginId: hook.id.clone(),
                        tag: hook.tag.clone().trim().to_ascii_lowercase(),
                        functionName: hook.function.clone(),
                        functionSource: hook.functionSource.clone(),
                    })
            })
            .collect::<Vec<_>>();
        hooks.sort_by(|left, right| {
            left.tag
                .cmp(&right.tag)
                .then(left.containerPackageName.cmp(&right.containerPackageName))
                .then(left.pluginId.cmp(&right.pluginId))
        });
        *XML_RENDER_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg xml render hook mutex poisoned") = hooks;
    }

    /// Renders one XML block through registered ToolPkg hooks.
    #[allow(non_snake_case)]
    pub fn renderRegisteredXml(tagName: String, xmlContent: String) -> Value {
        let Some(runtime) = XML_RENDER_RUNTIME.get() else {
            return Value::Null;
        };
        renderXml(runtime, tagName, xmlContent)
    }
}

/// Invokes matching XML render hooks and returns the first handled result.
#[allow(non_snake_case)]
fn renderXml(runtime: &ToolPkgBridgeRuntime, tagName: String, xmlContent: String) -> Value {
    let normalizedTag = tagName.trim().to_ascii_lowercase();
    if normalizedTag.is_empty() {
        return Value::Null;
    }
    let hooks = XML_RENDER_HOOKS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .expect("toolpkg xml render hook mutex poisoned")
        .clone()
        .into_iter()
        .filter(|hook| hook.tag == normalizedTag)
        .collect::<Vec<_>>();
    if hooks.is_empty() {
        return Value::Null;
    }
    let manager = runtime.package_manager();
    for hook in hooks {
        ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.toolpkg.xml_render.run.start",
            &[
                ("tag", normalizedTag.clone()),
                ("package", hook.containerPackageName.clone()),
                ("hookId", hook.pluginId.clone()),
                ("function", hook.functionName.clone()),
            ],
        );
        let result = manager.runToolPkgMainHook(
            &hook.containerPackageName,
            &hook.functionName,
            TOOLPKG_EVENT_XML_RENDER,
            None,
            Some(&hook.pluginId),
            hook.functionSource.as_deref(),
            serde_json::json!({
                "xmlContent": xmlContent,
                "tagName": tagName,
            }),
            None,
            None,
            None,
        );
        let decoded = match result {
            Ok(raw) => decodeToolPkgHookResult(raw),
            Err(error) => {
                ChainLogger::error(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.xml_render.run.error",
                    &[
                        ("tag", normalizedTag.clone()),
                        ("package", hook.containerPackageName.clone()),
                        ("hookId", hook.pluginId.clone()),
                        ("function", hook.functionName.clone()),
                        ("error", error),
                    ],
                );
                None
            }
        };
        let Some(rendered) = parseXmlRenderResult(decoded, &hook.containerPackageName) else {
            continue;
        };
        ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.toolpkg.xml_render.run.done",
            &[
                ("tag", normalizedTag.clone()),
                ("package", hook.containerPackageName.clone()),
                ("hookId", hook.pluginId.clone()),
            ],
        );
        return rendered;
    }
    Value::Null
}

/// Parses a ToolPkg XML render hook result into a host-renderable JSON value.
#[allow(non_snake_case)]
fn parseXmlRenderResult(decoded: Option<Value>, containerPackageName: &str) -> Option<Value> {
    match decoded? {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(serde_json::json!({
                    "kind": "text",
                    "text": text,
                }))
            }
        }
        Value::Object(object) => {
            let parsed =
                serde_json::from_value::<ToolPkgXmlRenderHookObjectResult>(Value::Object(object))
                    .ok()?;
            if parsed.handled == Some(false) {
                return None;
            }
            if let Some(composeDsl) = parsed.composeDsl {
                if !composeDsl.screen.trim().is_empty() {
                    return Some(serde_json::json!({
                        "kind": "composeDsl",
                        "containerPackageName": containerPackageName,
                        "screen": composeDsl.screen,
                        "state": composeDsl.state,
                        "memo": composeDsl.memo,
                        "moduleSpec": composeDsl.moduleSpec,
                    }));
                }
            }
            let text = parsed.text.or(parsed.content)?;
            if text.trim().is_empty() {
                None
            } else {
                Some(serde_json::json!({
                    "kind": "text",
                    "text": text,
                }))
            }
        }
        _ => None,
    }
}
