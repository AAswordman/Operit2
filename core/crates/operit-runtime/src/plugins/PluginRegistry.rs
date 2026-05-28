use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, OnceLock};

pub trait OperitPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn register(&self);
}

type PluginList = Mutex<Vec<Arc<dyn OperitPlugin>>>;
type PluginIdSet = Mutex<BTreeSet<String>>;

static PLUGINS: OnceLock<PluginList> = OnceLock::new();
static INSTALLED_PLUGIN_IDS: OnceLock<PluginIdSet> = OnceLock::new();
static BUILTINS_INITIALIZED: OnceLock<Mutex<bool>> = OnceLock::new();

pub struct PluginRegistry;

impl PluginRegistry {
    pub fn register(plugin: Arc<dyn OperitPlugin>) {
        let plugins = PLUGINS.get_or_init(|| Mutex::new(Vec::new()));
        let mut guard = plugins.lock().expect("plugin registry mutex poisoned");
        guard.retain(|registered| registered.id() != plugin.id());
        guard.push(plugin);
    }

    #[allow(non_snake_case)]
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
        Self::register(Arc::new(
            crate::plugins::workflow::WorkflowLifecyclePlugin::WorkflowLifecyclePlugin,
        ));
        Self::installAll();
    }

    #[allow(non_snake_case)]
    pub fn installAll() {
        let plugins = PLUGINS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("plugin registry mutex poisoned")
            .clone();
        let installedPluginIds = INSTALLED_PLUGIN_IDS.get_or_init(|| Mutex::new(BTreeSet::new()));
        for plugin in plugins {
            let mut installed = installedPluginIds
                .lock()
                .expect("plugin registry installed ids mutex poisoned");
            if installed.insert(plugin.id().to_string()) {
                drop(installed);
                plugin.register();
            }
        }
    }
}
