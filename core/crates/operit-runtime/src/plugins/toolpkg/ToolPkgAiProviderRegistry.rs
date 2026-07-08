use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use operit_tools::tools::packTool::ToolPkgParser::ToolPkgContainerRuntime;
use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::{
    toolPkgPackageManager, ToolPkgAiProviderRegistration,
};

pub struct ToolPkgAiProviderRegistry;

impl ToolPkgAiProviderRegistry {
    pub fn register() {
        static INSTALLED: AtomicBool = AtomicBool::new(false);
        if INSTALLED.swap(true, Ordering::SeqCst) {
            return;
        }
        let manager = toolPkgPackageManager();
        manager.addToolPkgRuntimeChangeListener(std::sync::Arc::new(|activeContainers| {
            ToolPkgAiProviderRegistry::syncToolPkgRegistrations(activeContainers);
        }));
    }

    pub fn get(providerId: &str) -> Option<ToolPkgAiProviderRegistration> {
        Self::register();
        providersById()
            .lock()
            .expect("toolpkg ai provider registry mutex poisoned")
            .get(&providerId.trim().to_ascii_lowercase())
            .cloned()
    }

    pub fn list() -> Vec<ToolPkgAiProviderRegistration> {
        Self::register();
        let mut providers = providersById()
            .lock()
            .expect("toolpkg ai provider registry mutex poisoned")
            .values()
            .cloned()
            .collect::<Vec<_>>();
        providers.sort_by(|left, right| left.providerId.cmp(&right.providerId));
        providers
    }

    #[allow(non_snake_case)]
    pub fn syncToolPkgRegistrations(activeContainers: Vec<ToolPkgContainerRuntime>) {
        let providers = activeContainers
            .iter()
            .flat_map(|runtime| {
                runtime
                    .aiProviders
                    .iter()
                    .map(|provider| ToolPkgAiProviderRegistration {
                        containerPackageName: runtime.packageName.clone(),
                        providerId: provider.id.clone(),
                        displayName: provider.displayName.clone(),
                        description: provider.description.clone(),
                        listModelsFunctionName: provider.listModelsHandler.function.clone(),
                        listModelsFunctionSource: provider.listModelsHandler.functionSource.clone(),
                        sendMessageFunctionName: provider.sendMessageHandler.function.clone(),
                        sendMessageFunctionSource: provider
                            .sendMessageHandler
                            .functionSource
                            .clone(),
                        testConnectionFunctionName: provider.testConnectionHandler.function.clone(),
                        testConnectionFunctionSource: provider
                            .testConnectionHandler
                            .functionSource
                            .clone(),
                        calculateInputTokensFunctionName: provider
                            .calculateInputTokensHandler
                            .function
                            .clone(),
                        calculateInputTokensFunctionSource: provider
                            .calculateInputTokensHandler
                            .functionSource
                            .clone(),
                    })
            })
            .map(|registration| {
                (
                    registration.providerId.trim().to_ascii_lowercase(),
                    registration,
                )
            })
            .collect::<BTreeMap<_, _>>();
        *providersById()
            .lock()
            .expect("toolpkg ai provider registry mutex poisoned") = providers;
    }
}

#[allow(non_snake_case)]
fn providersById() -> &'static Mutex<BTreeMap<String, ToolPkgAiProviderRegistration>> {
    static PROVIDERS_BY_ID: OnceLock<Mutex<BTreeMap<String, ToolPkgAiProviderRegistration>>> =
        OnceLock::new();
    PROVIDERS_BY_ID.get_or_init(|| Mutex::new(BTreeMap::new()))
}
