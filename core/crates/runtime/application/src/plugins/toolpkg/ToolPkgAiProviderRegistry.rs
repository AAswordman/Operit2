use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use operit_plugin_sdk::toolpkg::ToolPkgHooks::ToolPkgAiProviderRegistration;
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;

pub struct ToolPkgAiProviderRegistry;

impl ToolPkgAiProviderRegistry {
    /// Returns a registered ToolPkg AI provider by identifier.
    pub fn get(providerId: &str) -> Option<ToolPkgAiProviderRegistration> {
        providersById()
            .lock()
            .expect("toolpkg ai provider registry mutex poisoned")
            .get(&providerId.trim().to_ascii_lowercase())
            .cloned()
    }

    /// Lists all registered ToolPkg AI providers.
    pub fn list() -> Vec<ToolPkgAiProviderRegistration> {
        let mut providers = providersById()
            .lock()
            .expect("toolpkg ai provider registry mutex poisoned")
            .values()
            .cloned()
            .collect::<Vec<_>>();
        providers.sort_by(|left, right| left.providerId.cmp(&right.providerId));
        providers
    }

    /// Synchronizes this bridge from the committed active package catalog.
    #[allow(non_snake_case)]
    pub fn syncToolPkgRegistrations(activeContainers: &[ToolPkgContainerRuntime]) {
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
