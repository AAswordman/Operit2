use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;

use super::ToolPkgAiProviderRegistry::ToolPkgAiProviderRegistry;
use super::ToolPkgAppLifecycleHookBridge::ToolPkgAppLifecycleHookBridge;
use super::ToolPkgChatComposerSlotBridge::ToolPkgChatComposerSlotBridge;
use super::ToolPkgChatInputHookBridge::ToolPkgChatInputHookBridge;
use super::ToolPkgChatMessageHookBridge::ToolPkgChatMessageHookBridge;
use super::ToolPkgChatRuntimeHookBridge::ToolPkgChatRuntimeHookBridge;
use super::ToolPkgChatViewHookBridge::ToolPkgChatViewHookBridge;
use super::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use super::ToolPkgHostEventHookBridge::ToolPkgHostEventHookBridge;
use super::ToolPkgInputMenuToggleBridge::ToolPkgInputMenuToggleBridge;
use super::ToolPkgManifestExtensionBridge::ToolPkgManifestExtensionBridge;
use super::ToolPkgMessageProcessingBridge::ToolPkgMessageProcessingBridge;
use super::ToolPkgPromptHookBridge::ToolPkgPromptHookBridge;
use super::ToolPkgSummaryHookBridge::ToolPkgSummaryHookBridge;
use super::ToolPkgToolLifecycleBridge::ToolPkgToolLifecycleBridge;
use super::ToolPkgXmlRenderBridge::ToolPkgXmlRenderBridge;
use crate::plugins::PluginRegistry::OperitPlugin;

/// Owns bridge entry points and the single listener for active package registrations.
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
    /// Returns the identity of the shared ToolPkg bridge plugin.
    fn id(&self) -> &str {
        "builtin.toolpkg.common-bridge"
    }

    /// Registers bridge entry points before subscribing to the active package catalog.
    fn register(&self) {
        static INSTALLED: AtomicBool = AtomicBool::new(false);
        if INSTALLED.swap(true, Ordering::SeqCst) {
            return;
        }
        ToolPkgMessageProcessingBridge::register(self.runtime.clone());
        ToolPkgXmlRenderBridge::register(self.runtime.clone());
        ToolPkgPromptHookBridge::register(self.runtime.clone());
        ToolPkgSummaryHookBridge::register(self.runtime.clone());
        ToolPkgToolLifecycleBridge::register(self.runtime.clone());
        ToolPkgChatInputHookBridge::register(self.runtime.clone());
        ToolPkgChatViewHookBridge::register(self.runtime.clone());
        ToolPkgChatMessageHookBridge::register(self.runtime.clone());
        ToolPkgChatRuntimeHookBridge::register(self.runtime.clone());
        ToolPkgInputMenuToggleBridge::register(self.runtime.clone());
        let manager = self.runtime.package_manager();
        let runtime = self.runtime.clone();
        manager.addToolPkgRuntimeChangeListener(Arc::new(move |activeContainers| {
            syncToolPkgRegistrations(&runtime, &activeContainers);
        }));
    }
}

/// Applies one committed catalog to every bridge without copying its runtime models.
#[allow(non_snake_case)]
fn syncToolPkgRegistrations(
    runtime: &ToolPkgBridgeRuntime,
    activeContainers: &[ToolPkgContainerRuntime],
) {
    ToolPkgAppLifecycleHookBridge::syncAndReplayToolPkgRegistrations(runtime, activeContainers);
    ToolPkgManifestExtensionBridge::syncAndDispatch(runtime, activeContainers);
    ToolPkgMessageProcessingBridge::syncToolPkgRegistrations(activeContainers);
    ToolPkgXmlRenderBridge::syncToolPkgRegistrations(activeContainers);
    ToolPkgPromptHookBridge::syncToolPkgRegistrations(activeContainers);
    ToolPkgSummaryHookBridge::syncToolPkgRegistrations(activeContainers);
    ToolPkgToolLifecycleBridge::syncToolPkgRegistrations(activeContainers);
    ToolPkgChatViewHookBridge::syncAndReplayToolPkgRegistrations(runtime, activeContainers);
    ToolPkgChatMessageHookBridge::syncToolPkgRegistrations(activeContainers);
    ToolPkgChatRuntimeHookBridge::syncToolPkgRegistrations(activeContainers);
    ToolPkgInputMenuToggleBridge::syncToolPkgRegistrations(activeContainers);
    ToolPkgChatComposerSlotBridge::syncToolPkgRegistrations(activeContainers);
    ToolPkgAiProviderRegistry::syncToolPkgRegistrations(activeContainers);
    ToolPkgHostEventHookBridge::syncToolPkgRegistrations(runtime, activeContainers);
}
