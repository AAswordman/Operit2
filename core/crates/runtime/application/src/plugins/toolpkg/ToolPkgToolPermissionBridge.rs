use std::sync::{Mutex, OnceLock};

use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_TOOL_PERMISSION;
use operit_plugin_sdk::toolpkg::ToolPkgParser::{ToolPkgContainerRuntime, ToolPkgFunctionHookRuntime};
use operit_util::AppLogger::AppLogger;

use super::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use crate::services::RuntimeHostInteractionService::pendingToolPermissions;

static RUNTIME: OnceLock<ToolPkgBridgeRuntime> = OnceLock::new();
static HOOKS: OnceLock<Mutex<Vec<(String, ToolPkgFunctionHookRuntime)>>> = OnceLock::new();

/// Publishes approval snapshots independently of tool execution and its awaited policy hooks.
pub struct ToolPkgToolPermissionBridge;

impl ToolPkgToolPermissionBridge {
    /// Installs the application runtime used to execute presentation hooks.
    pub fn register(runtime: ToolPkgBridgeRuntime) {
        RUNTIME.get_or_init(|| runtime);
    }

    /// Replaces active presenters and replays pending requests after catalog changes.
    pub fn sync(active: &[ToolPkgContainerRuntime]) {
        *HOOKS.get_or_init(Default::default).lock().expect("permission hook mutex poisoned") = active
            .iter()
            .flat_map(|container| container.toolPermissionHooks.iter()
                .map(move |hook| (container.packageName.clone(), hook.clone())))
            .collect();
        Self::notify();
    }

    /// Serializes current snapshots on a dedicated queue, independently of execution-policy hooks.
    pub fn notify() {
        let Some(runtime) = RUNTIME.get().cloned() else { return; };
        static QUEUE: OnceLock<Result<tokio::sync::mpsc::UnboundedSender<()>, String>> = OnceLock::new();
        let queue = QUEUE.get_or_init(|| {
            let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<()>();
            defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
                "operit-tool-permissions",
                Box::new(move || Box::pin(async move {
                    while receiver.recv().await.is_some() {
                        let hooks = HOOKS.get_or_init(Default::default).lock()
                            .expect("permission hook mutex poisoned").clone();
                        let manager = runtime.package_manager();
                        for (owner, hook) in hooks {
                            let requests = pendingToolPermissions();
                            if let Err(error) = manager.runToolPkgMainHook(
                                &owner, &hook.function, TOOLPKG_EVENT_TOOL_PERMISSION,
                                Some("requests_changed"), Some(&hook.id), hook.functionSource.as_deref(),
                                serde_json::json!({"requests": requests}), None, None, None,
                            ).await {
                                AppLogger::e("ToolPkgToolPermissionBridge", &format!("{owner}/{}: {error}", hook.id));
                            }
                        }
                    }
                })),
            ).map_err(|error| error.to_string())?;
            Ok(sender)
        });
        let result = match queue {
            Ok(sender) => sender.send(()).map_err(|error| error.to_string()),
            Err(error) => Err(error.clone()),
        };
        if let Err(error) = result {
            AppLogger::e("ToolPkgToolPermissionBridge", &format!("Unable to enqueue permission notification: {error}"));
        }
    }
}
