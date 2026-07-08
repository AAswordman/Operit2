use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, OnceLock};

use operit_util::ChainLogger::{self, PLUGIN_CHAIN};

/// Runtime plugin contract for registering feature modules.
pub trait OperitPlugin: Send + Sync {
    /// Returns the stable plugin id.
    fn id(&self) -> &str;
    /// Registers plugin-owned runtime components.
    fn register(&self);
}

type PluginList = Mutex<Vec<Arc<dyn OperitPlugin>>>;
type PluginIdSet = Mutex<BTreeSet<String>>;

static PLUGINS: OnceLock<PluginList> = OnceLock::new();
static INSTALLED_PLUGIN_IDS: OnceLock<PluginIdSet> = OnceLock::new();
static BUILTINS_INITIALIZED: OnceLock<Mutex<bool>> = OnceLock::new();

/// Global registry for built-in runtime plugins.
pub struct PluginRegistry;

impl PluginRegistry {
    /// Registers or replaces a plugin by id.
    pub fn register(plugin: Arc<dyn OperitPlugin>) {
        let pluginId = plugin.id().to_string();
        let plugins = PLUGINS.get_or_init(|| Mutex::new(Vec::new()));
        let mut guard = plugins.lock().expect("plugin registry mutex poisoned");
        guard.retain(|registered| registered.id() != plugin.id());
        guard.push(plugin);
        ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.registry.register",
            &[("pluginId", pluginId)],
        );
    }

    #[allow(non_snake_case)]
    /// Registers and installs all built-in plugins exactly once.
    pub fn initializeBuiltins() {
        let initialized = BUILTINS_INITIALIZED.get_or_init(|| Mutex::new(false));
        {
            let mut guard = initialized
                .lock()
                .expect("plugin registry initialized mutex poisoned");
            if *guard {
                return;
            }
            *guard = true;
        }

        Self::register(Arc::new(
            crate::plugins::toolbox::ToolboxPlugin::ToolboxPlugin,
        ));
        Self::register(Arc::new(
            crate::plugins::toolpkg::ToolPkgCommonBridgePlugin::ToolPkgCommonBridgePlugin,
        ));
        Self::installAll();
    }

    #[allow(non_snake_case)]
    /// Installs all registered plugins that have not yet been installed.
    pub fn installAll() {
        let plugins = PLUGINS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("plugin registry mutex poisoned")
            .clone();
        ChainLogger::info(
            PLUGIN_CHAIN,
            "plugin.registry.install.scan",
            &[("pluginCount", plugins.len().to_string())],
        );
        let installedPluginIds = INSTALLED_PLUGIN_IDS.get_or_init(|| Mutex::new(BTreeSet::new()));
        for plugin in plugins {
            let mut installed = installedPluginIds
                .lock()
                .expect("plugin registry installed ids mutex poisoned");
            if installed.insert(plugin.id().to_string()) {
                let pluginId = plugin.id().to_string();
                drop(installed);
                ChainLogger::info(
                    PLUGIN_CHAIN,
                    "plugin.registry.install.start",
                    &[("pluginId", pluginId.clone())],
                );
                plugin.register();
                ChainLogger::info(
                    PLUGIN_CHAIN,
                    "plugin.registry.install.done",
                    &[("pluginId", pluginId)],
                );
            }
        }
    }
}
