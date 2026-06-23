use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::toolPkgPackageManager;
use crate::plugins::PluginRegistry::OperitPlugin;

pub struct ToolPkgCommonBridgePlugin;

impl OperitPlugin for ToolPkgCommonBridgePlugin {
    fn id(&self) -> &str {
        "builtin.toolpkg.common-bridge"
    }

    fn register(&self) {
        static INSTALLED: AtomicBool = AtomicBool::new(false);
        if INSTALLED.swap(true, Ordering::SeqCst) {
            return;
        }
        crate::plugins::toolpkg::ToolPkgMessageProcessingBridge::ToolPkgMessageProcessingBridge::register();
        crate::plugins::toolpkg::ToolPkgPromptHookBridge::ToolPkgPromptHookBridge::register();
        crate::plugins::toolpkg::ToolPkgSummaryHookBridge::ToolPkgSummaryHookBridge::register();
        crate::plugins::toolpkg::ToolPkgToolLifecycleBridge::ToolPkgToolLifecycleBridge::register();
        crate::plugins::toolpkg::ToolPkgChatInputHookBridge::ToolPkgChatInputHookBridge::register();
        crate::plugins::toolpkg::ToolPkgChatViewHookBridge::ToolPkgChatViewHookBridge::register();
        crate::plugins::toolpkg::ToolPkgInputMenuToggleBridge::ToolPkgInputMenuToggleBridge::register();
        crate::plugins::toolpkg::ToolPkgAiProviderRegistry::ToolPkgAiProviderRegistry::register();
        crate::plugins::toolpkg::ToolPkgHostEventHookBridge::ToolPkgHostEventHookBridge::register();
        let manager = toolPkgPackageManager();
        manager.addToolPkgRuntimeChangeListener(Arc::new(|activeContainers| {
            syncToolPkgRegistrations(activeContainers);
        }));
    }
}

#[allow(non_snake_case)]
fn syncToolPkgRegistrations(
    activeContainers: Vec<operit_tools::packTool::ToolPkgParser::ToolPkgContainerRuntime>,
) {
    crate::plugins::toolpkg::ToolPkgMessageProcessingBridge::ToolPkgMessageProcessingBridge::syncToolPkgRegistrations(activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgPromptHookBridge::ToolPkgPromptHookBridge::syncToolPkgRegistrations(activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgSummaryHookBridge::ToolPkgSummaryHookBridge::syncToolPkgRegistrations(activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgToolLifecycleBridge::ToolPkgToolLifecycleBridge::syncToolPkgRegistrations(activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgChatInputHookBridge::ToolPkgChatInputHookBridge::syncToolPkgRegistrations(activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgChatViewHookBridge::ToolPkgChatViewHookBridge::syncAndReplayToolPkgRegistrations(activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgInputMenuToggleBridge::ToolPkgInputMenuToggleBridge::syncToolPkgRegistrations(activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgAiProviderRegistry::ToolPkgAiProviderRegistry::syncToolPkgRegistrations(activeContainers.clone());
    crate::plugins::toolpkg::ToolPkgHostEventHookBridge::ToolPkgHostEventHookBridge::syncToolPkgRegistrations(activeContainers);
}
