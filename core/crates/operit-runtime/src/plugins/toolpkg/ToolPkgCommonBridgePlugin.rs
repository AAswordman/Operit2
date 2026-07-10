use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use crate::plugins::PluginRegistry::OperitPlugin;

pub struct ToolPkgCommonBridgePlugin {
    runtime: ToolPkgBridgeRuntime,
}

impl ToolPkgCommonBridgePlugin {
    /// Creates the common ToolPkg bridge plugin for one runtime.
    pub fn new(runtime: ToolPkgBridgeRuntime) -> Self {
        Self { runtime }
    }
}

impl OperitPlugin for ToolPkgCommonBridgePlugin {
    fn id(&self) -> &str {
        "builtin.toolpkg.common-bridge"
    }

    fn register(&self) {
        static INSTALLED: AtomicBool = AtomicBool::new(false);
        if INSTALLED.swap(true, Ordering::SeqCst) {
            return;
        }
        crate::plugins::toolpkg::ToolPkgMessageProcessingBridge::ToolPkgMessageProcessingBridge::register(self.runtime.clone());
        crate::plugins::toolpkg::ToolPkgPromptHookBridge::ToolPkgPromptHookBridge::register(
            self.runtime.clone(),
        );
        crate::plugins::toolpkg::ToolPkgSummaryHookBridge::ToolPkgSummaryHookBridge::register(
            self.runtime.clone(),
        );
        crate::plugins::toolpkg::ToolPkgToolLifecycleBridge::ToolPkgToolLifecycleBridge::register(
            self.runtime.clone(),
        );
        crate::plugins::toolpkg::ToolPkgChatInputHookBridge::ToolPkgChatInputHookBridge::register(
            self.runtime.clone(),
        );
        crate::plugins::toolpkg::ToolPkgChatViewHookBridge::ToolPkgChatViewHookBridge::register(
            self.runtime.clone(),
        );
        crate::plugins::toolpkg::ToolPkgInputMenuToggleBridge::ToolPkgInputMenuToggleBridge::register(self.runtime.clone());
        crate::plugins::toolpkg::ToolPkgAiProviderRegistry::ToolPkgAiProviderRegistry::register(
            self.runtime.clone(),
        );
        crate::plugins::toolpkg::ToolPkgHostEventHookBridge::ToolPkgHostEventHookBridge::register(
            self.runtime.clone(),
        );
        let manager = self.runtime.package_manager();
        let runtime = self.runtime.clone();
        manager.addToolPkgRuntimeChangeListener(Arc::new(move |activeContainers| {
            syncToolPkgRegistrations(&runtime, activeContainers);
        }));
    }
}

#[allow(non_snake_case)]
fn syncToolPkgRegistrations(
    runtime: &ToolPkgBridgeRuntime,
    activeContainers: Vec<operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime>,
) {
    crate::plugins::toolpkg::ToolPkgMessageProcessingBridge::ToolPkgMessageProcessingBridge::syncToolPkgRegistrations(activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgPromptHookBridge::ToolPkgPromptHookBridge::syncToolPkgRegistrations(activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgSummaryHookBridge::ToolPkgSummaryHookBridge::syncToolPkgRegistrations(activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgToolLifecycleBridge::ToolPkgToolLifecycleBridge::syncToolPkgRegistrations(activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgChatInputHookBridge::ToolPkgChatInputHookBridge::syncToolPkgRegistrations(activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgChatViewHookBridge::ToolPkgChatViewHookBridge::syncAndReplayToolPkgRegistrations(runtime, activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgInputMenuToggleBridge::ToolPkgInputMenuToggleBridge::syncToolPkgRegistrations(activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgAiProviderRegistry::ToolPkgAiProviderRegistry::syncToolPkgRegistrations(activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgHostEventHookBridge::ToolPkgHostEventHookBridge::syncToolPkgRegistrations(runtime, activeContainers);
}
