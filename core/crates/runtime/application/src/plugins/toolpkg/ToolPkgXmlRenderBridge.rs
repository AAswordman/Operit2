use std::sync::{Mutex, OnceLock};

use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_XML_RENDER;
use operit_plugin_sdk::toolpkg::ToolPkgHookModels::ToolPkgXmlRenderHookObjectResult;
use operit_plugin_sdk::toolpkg::ToolPkgHooks::{
    decodeToolPkgHookResult, ToolPkgXmlRenderHookRegistration,
};
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;
use operit_store::PreferencesDataStore::{MutableStateFlow, StateFlow};
use operit_util::ChainLogger::{self, PLUGIN_CHAIN};
use serde_json::Value;

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;

static XML_RENDER_REGISTRY: OnceLock<XmlRenderHookRegistry> = OnceLock::new();
static XML_RENDER_RUNTIME: OnceLock<ToolPkgBridgeRuntime> = OnceLock::new();

/// Owns the committed XML hooks and publishes invalidations after their replacement.
struct XmlRenderHookRegistry {
    hooks: Mutex<Vec<ToolPkgXmlRenderHookRegistration>>,
    revision: MutableStateFlow<i64>,
}

impl XmlRenderHookRegistry {
    /// Creates an empty registry with an observable initial revision.
    fn new() -> Self {
        Self {
            hooks: Mutex::new(Vec::new()),
            revision: MutableStateFlow::new(0),
        }
    }

    /// Commits changed hooks before notifying observers, without holding the hook lock.
    fn replace(&self, hooks: Vec<ToolPkgXmlRenderHookRegistration>) {
        {
            let mut current = self
                .hooks
                .lock()
                .expect("toolpkg xml render hook mutex poisoned");
            if *current == hooks {
                return;
            }
            *current = hooks;
        }
        // Existing XML nodes must retry even when their message text did not change.
        self.revision.update(|revision| *revision += 1);
    }
}

/// Returns the process registry shared by rendering requests and revision observers.
#[allow(non_snake_case)]
fn xmlRenderRegistry() -> &'static XmlRenderHookRegistry {
    XML_RENDER_REGISTRY.get_or_init(XmlRenderHookRegistry::new)
}

pub struct ToolPkgXmlRenderBridge;

impl ToolPkgXmlRenderBridge {
    /// Registers XML render hooks for one application runtime.
    pub fn register(runtime: ToolPkgBridgeRuntime) {
        XML_RENDER_RUNTIME.get_or_init(|| runtime.clone());
    }

    /// Synchronizes active XML render hook registrations from enabled ToolPkg containers.
    #[allow(non_snake_case)]
    pub fn syncToolPkgRegistrations(activeContainers: &[ToolPkgContainerRuntime]) {
        let hooks = activeContainers
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
        xmlRenderRegistry().replace(hooks);
    }

    /// Observes XML hook replacements after the new registrations are available for rendering.
    #[allow(non_snake_case)]
    pub fn revisionFlow() -> StateFlow<i64> {
        xmlRenderRegistry().revision.asStateFlow()
    }

    /// Renders one XML block through registered ToolPkg hooks.
    #[allow(non_snake_case)]
    pub async fn renderRegisteredXml(
        tagName: String,
        xmlContent: String,
        chatId: Option<String>,
    ) -> Value {
        let Some(runtime) = XML_RENDER_RUNTIME.get() else {
            return Value::Null;
        };
        renderXml(runtime, tagName, xmlContent, chatId).await
    }
}

/// Invokes matching XML render hooks and returns the first handled result.
#[allow(non_snake_case)]
async fn renderXml(
    runtime: &ToolPkgBridgeRuntime,
    tagName: String,
    xmlContent: String,
    chatId: Option<String>,
) -> Value {
    let normalizedTag = tagName.trim().to_ascii_lowercase();
    if normalizedTag.is_empty() {
        return Value::Null;
    }
    let hooks = xmlRenderRegistry()
        .hooks
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
        ChainLogger::verbose_with_level(
            PLUGIN_CHAIN,
            "plugin.toolpkg.xml_render.run.start",
            &[
                ("tag", normalizedTag.clone()),
                ("package", hook.containerPackageName.clone()),
                ("hookId", hook.pluginId.clone()),
                ("function", hook.functionName.clone()),
            ],
            operit_util::AppLogger::VERBOSE_LEVEL_3,
        );
        let result = manager
            .runToolPkgMainHook(
                &hook.containerPackageName,
                &hook.functionName,
                TOOLPKG_EVENT_XML_RENDER,
                None,
                Some(&hook.pluginId),
                hook.functionSource.as_deref(),
                serde_json::json!({
                    "xmlContent": xmlContent,
                    "tagName": tagName,
                    "chatId": chatId.clone(),
                }),
                None,
                None,
                None,
            )
            .await;
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
        ChainLogger::verbose_with_level(
            PLUGIN_CHAIN,
            "plugin.toolpkg.xml_render.run.done",
            &[
                ("tag", normalizedTag.clone()),
                ("package", hook.containerPackageName.clone()),
                ("hookId", hook.pluginId.clone()),
            ],
            operit_util::AppLogger::VERBOSE_LEVEL_3,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Creates one XML hook with stable registration identity.
    fn hook() -> ToolPkgXmlRenderHookRegistration {
        ToolPkgXmlRenderHookRegistration {
            containerPackageName: "test.package".to_string(),
            pluginId: "test.xml".to_string(),
            tag: "plan".to_string(),
            functionName: "renderPlan".to_string(),
            functionSource: None,
        }
    }

    /// Invalidates XML output for additions, changed declarations, and removals only.
    #[test]
    fn revisions_track_changed_hooks_without_redundant_notifications() {
        let registry = XmlRenderHookRegistry::new();
        registry.replace(Vec::new());
        assert_eq!(registry.revision.value(), 0);
        let initial = hook();
        registry.replace(vec![initial.clone()]);
        assert_eq!(registry.revision.value(), 1);
        registry.replace(vec![initial.clone()]);
        assert_eq!(registry.revision.value(), 1);
        let mut changed = initial;
        changed.functionSource = Some("function() { return 'updated'; }".to_string());
        registry.replace(vec![changed]);
        assert_eq!(registry.revision.value(), 2);
        registry.replace(Vec::new());
        assert_eq!(registry.revision.value(), 3);
    }

    /// Allows observers to read committed hooks without reentering the replacement lock.
    #[test]
    fn observers_see_committed_hooks_after_lock_release() {
        let registry = Arc::new(XmlRenderHookRegistry::new());
        let notifications = Arc::new(Mutex::new(Vec::new()));
        let observedRegistry = registry.clone();
        let observedNotifications = notifications.clone();
        let subscription = registry.revision.subscribe(move |revision| {
            let hooks = observedRegistry
                .hooks
                .try_lock()
                .expect("revision must be published after releasing the hook lock");
            observedNotifications
                .lock()
                .unwrap()
                .push((revision, hooks.len()));
        });
        registry.replace(vec![hook()]);
        registry.replace(Vec::new());
        assert_eq!(*notifications.lock().unwrap(), vec![(0, 0), (1, 1), (2, 0)]);
        registry.revision.unsubscribe(subscription);
    }
}
