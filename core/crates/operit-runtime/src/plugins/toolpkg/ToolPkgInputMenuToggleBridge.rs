use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::{
    decodeToolPkgHookResult, toolPkgPackageManager, ToolPkgInputMenuToggleHookRegistration,
};
use operit_tools::tools::packTool::PackageManager::PackageManager;
use operit_tools::tools::packTool::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_INPUT_MENU_TOGGLE;
use operit_tools::tools::packTool::ToolPkgParser::ToolPkgContainerRuntime;
use operit_util::AppLogger::AppLogger;

static INPUT_MENU_HOOKS: OnceLock<Mutex<Vec<ToolPkgInputMenuToggleHookRegistration>>> =
    OnceLock::new();
static INPUT_MENU_SPECS_CACHE: OnceLock<Mutex<Vec<InputMenuSpec>>> = OnceLock::new();
static HAS_LOADED_ONCE: AtomicBool = AtomicBool::new(false);
static HOOK_REGISTRY_VERSION: AtomicI64 = AtomicI64::new(0);
static LAST_HOOK_REGISTRY_VERSION: AtomicI64 = AtomicI64::new(-1);
static LAST_PARAMS_CACHE_KEY: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static REFRESH_FLAG: AtomicBool = AtomicBool::new(false);
const TAG: &str = "ToolPkgInputMenuToggleBridge";

pub const INPUT_MENU_SLOT_THINKING: &str = "thinking";
pub const INPUT_MENU_SLOT_MEMORY: &str = "memory";
pub const INPUT_MENU_SLOT_MODEL: &str = "model";
pub const INPUT_MENU_SLOT_TOOLS: &str = "tools";
pub const INPUT_MENU_SLOT_GENERAL: &str = "general";
pub const INPUT_MENU_SLOT_DEFAULT: &str = "default";

#[derive(Clone)]
pub struct InputMenuToggleHookParams {
    pub chatId: Option<String>,
    pub featureStates: BTreeMap<String, bool>,
    pub onToggleFeature: Arc<dyn Fn(String) + Send + Sync>,
    pub runtime: Option<String>,
}

#[derive(Clone)]
pub struct InputMenuToggleDefinition {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub isChecked: bool,
    pub isEnabled: bool,
    pub slot: Option<String>,
    pub onToggle: Arc<dyn Fn() + Send + Sync>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputMenuToggleDefinitionSnapshot {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub isChecked: bool,
    pub isEnabled: bool,
    pub slot: Option<String>,
}

pub trait InputMenuTogglePlugin: Send + Sync {
    fn id(&self) -> &str;

    #[allow(non_snake_case)]
    fn createToggles(&self, params: &InputMenuToggleHookParams) -> Vec<InputMenuToggleDefinition>;
}

static INPUT_MENU_PLUGINS: OnceLock<Mutex<Vec<Arc<dyn InputMenuTogglePlugin>>>> = OnceLock::new();
static INPUT_MENU_CHANGE_VERSION: AtomicI64 = AtomicI64::new(0);

pub struct InputMenuTogglePluginRegistry;

impl InputMenuTogglePluginRegistry {
    pub fn register(plugin: Arc<dyn InputMenuTogglePlugin>) {
        Self::unregister(plugin.id());
        INPUT_MENU_PLUGINS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("input menu plugin mutex poisoned")
            .push(plugin);
        Self::notifyChanged();
    }

    pub fn unregister(pluginId: &str) {
        let mut plugins = INPUT_MENU_PLUGINS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("input menu plugin mutex poisoned");
        let before = plugins.len();
        plugins.retain(|plugin| plugin.id() != pluginId);
        if plugins.len() != before {
            Self::notifyChanged();
        }
    }

    #[allow(non_snake_case)]
    pub fn notifyChanged() {
        INPUT_MENU_CHANGE_VERSION.fetch_add(1, Ordering::SeqCst);
    }

    #[allow(non_snake_case)]
    /// Reports the current input-menu toggle registry change counter.
    pub fn changeVersion() -> i64 {
        INPUT_MENU_CHANGE_VERSION.load(Ordering::SeqCst)
    }

    #[allow(non_snake_case)]
    pub fn createToggles(params: &InputMenuToggleHookParams) -> Vec<InputMenuToggleDefinition> {
        let plugins = INPUT_MENU_PLUGINS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("input menu plugin mutex poisoned")
            .clone();
        plugins
            .iter()
            .flat_map(|plugin| plugin.createToggles(params))
            .collect()
    }
}

pub struct ToolPkgInputMenuToggleBridge;

impl ToolPkgInputMenuToggleBridge {
    pub fn new() -> Self {
        Self
    }

    pub fn register() {
        static INSTALLED: AtomicBool = AtomicBool::new(false);
        if INSTALLED.swap(true, Ordering::SeqCst) {
            return;
        }
        InputMenuTogglePluginRegistry::register(Arc::new(BridgePlugin));
        let manager = toolPkgPackageManager();
        manager.addToolPkgRuntimeChangeListener(Arc::new(|activeContainers| {
            ToolPkgInputMenuToggleBridge::syncToolPkgRegistrations(activeContainers);
        }));
    }

    #[allow(non_snake_case)]
    pub fn syncToolPkgRegistrations(activeContainers: Vec<ToolPkgContainerRuntime>) {
        let mut hooks = activeContainers
            .iter()
            .flat_map(|container| {
                container.inputMenuTogglePlugins.iter().map(|hook| {
                    ToolPkgInputMenuToggleHookRegistration {
                        containerPackageName: container.packageName.clone(),
                        pluginId: hook.id.clone(),
                        functionName: hook.function.clone(),
                        functionSource: hook.functionSource.clone(),
                    }
                })
            })
            .collect::<Vec<_>>();
        hooks.sort_by(|left, right| {
            left.containerPackageName
                .cmp(&right.containerPackageName)
                .then(left.pluginId.cmp(&right.pluginId))
        });
        let mut stored = INPUT_MENU_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg input menu hook mutex poisoned");
        if *stored == hooks {
            return;
        }
        *stored = hooks;
        INPUT_MENU_SPECS_CACHE
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg input menu specs mutex poisoned")
            .clear();
        HAS_LOADED_ONCE.store(false, Ordering::SeqCst);
        *LAST_PARAMS_CACHE_KEY
            .get_or_init(|| Mutex::new(None))
            .lock()
            .expect("toolpkg input menu params cache mutex poisoned") = None;
        HOOK_REGISTRY_VERSION.fetch_add(1, Ordering::SeqCst);
        InputMenuTogglePluginRegistry::notifyChanged();
    }

    #[allow(non_snake_case)]
    pub fn createToggleDefinitions(
        chatId: Option<String>,
        featureStates: BTreeMap<String, bool>,
        runtime: Option<String>,
    ) -> Vec<InputMenuToggleDefinitionSnapshot> {
        let params = InputMenuToggleHookParams {
            chatId,
            featureStates,
            onToggleFeature: Arc::new(|_| {}),
            runtime,
        };
        InputMenuTogglePluginRegistry::createToggles(&params)
            .into_iter()
            .map(|definition| InputMenuToggleDefinitionSnapshot {
                id: definition.id,
                title: definition.title,
                description: definition.description,
                icon: definition.icon,
                isChecked: definition.isChecked,
                isEnabled: definition.isEnabled,
                slot: definition.slot,
            })
            .collect()
    }

    #[allow(non_snake_case)]
    /// Builds input-menu toggle definitions for Flutter from active tool package hooks.
    pub fn createToggleDefinitionsForFlutter(
        &self,
        chatId: Option<String>,
        featureStates: BTreeMap<String, bool>,
        runtime: Option<String>,
    ) -> Vec<InputMenuToggleDefinitionSnapshot> {
        Self::createToggleDefinitions(chatId, featureStates, runtime)
    }

    #[allow(non_snake_case)]
    pub fn triggerToggle(toggleId: &str, chatId: Option<String>, runtime: Option<String>) -> bool {
        let normalizedToggleId = toggleId.trim();
        if normalizedToggleId.is_empty() {
            return false;
        }
        let params = InputMenuToggleHookParams {
            chatId,
            featureStates: BTreeMap::new(),
            onToggleFeature: Arc::new(|_| {}),
            runtime,
        };
        let specs = INPUT_MENU_SPECS_CACHE
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg input menu specs mutex poisoned")
            .clone();
        let Some(spec) = specs.into_iter().find(|spec| spec.id == normalizedToggleId) else {
            return false;
        };
        launchToggle(spec, params);
        true
    }

    #[allow(non_snake_case)]
    /// Runs a Flutter-requested input-menu toggle hook by toggle identifier.
    pub fn triggerToggleForFlutter(
        &self,
        toggleId: String,
        chatId: Option<String>,
        runtime: Option<String>,
    ) -> bool {
        Self::triggerToggle(&toggleId, chatId, runtime)
    }

    #[allow(non_snake_case)]
    /// Reports the input-menu toggle change counter for Flutter watchers.
    pub fn changeVersion(&self) -> i64 {
        InputMenuTogglePluginRegistry::changeVersion()
    }
}

struct BridgePlugin;

impl InputMenuTogglePlugin for BridgePlugin {
    fn id(&self) -> &str {
        "builtin.toolpkg.input-menu-toggle-bridge"
    }

    #[allow(non_snake_case)]
    fn createToggles(&self, params: &InputMenuToggleHookParams) -> Vec<InputMenuToggleDefinition> {
        let registryVersion = HOOK_REGISTRY_VERSION.load(Ordering::SeqCst);
        let paramsCacheKey = buildCacheKey(params);
        let shouldRefreshForParams = {
            let lastParamsCacheKey = LAST_PARAMS_CACHE_KEY
                .get_or_init(|| Mutex::new(None))
                .lock()
                .expect("toolpkg input menu params cache mutex poisoned");
            lastParamsCacheKey.as_deref() != Some(paramsCacheKey.as_str())
        };
        if shouldRefreshForParams {
            INPUT_MENU_SPECS_CACHE
                .get_or_init(|| Mutex::new(Vec::new()))
                .lock()
                .expect("toolpkg input menu specs mutex poisoned")
                .clear();
            HAS_LOADED_ONCE.store(false, Ordering::SeqCst);
        }

        let cachedSpecs = INPUT_MENU_SPECS_CACHE
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg input menu specs mutex poisoned")
            .clone();
        if shouldRefreshForParams
            || !HAS_LOADED_ONCE.load(Ordering::SeqCst)
            || LAST_HOOK_REGISTRY_VERSION.load(Ordering::SeqCst) != registryVersion
        {
            triggerRefresh(params, registryVersion, &paramsCacheKey);
            if cachedSpecs.is_empty() {
                return vec![createLoadingToggle()];
            }
        }
        buildToggleDefinitions(&cachedSpecs, params)
    }
}

#[allow(non_snake_case)]
fn normalizeInputMenuSlot(slot: Option<&str>) -> String {
    match slot.map(str::trim).unwrap_or_default() {
        INPUT_MENU_SLOT_THINKING => INPUT_MENU_SLOT_THINKING.to_string(),
        INPUT_MENU_SLOT_MEMORY => INPUT_MENU_SLOT_MEMORY.to_string(),
        INPUT_MENU_SLOT_MODEL => INPUT_MENU_SLOT_MODEL.to_string(),
        INPUT_MENU_SLOT_TOOLS => INPUT_MENU_SLOT_TOOLS.to_string(),
        INPUT_MENU_SLOT_GENERAL => INPUT_MENU_SLOT_GENERAL.to_string(),
        INPUT_MENU_SLOT_DEFAULT => INPUT_MENU_SLOT_DEFAULT.to_string(),
        _ => INPUT_MENU_SLOT_DEFAULT.to_string(),
    }
}

#[allow(non_snake_case)]
fn buildCacheKey(params: &InputMenuToggleHookParams) -> String {
    format!(
        "{}|{}",
        params.runtime.clone().unwrap_or_default(),
        params.chatId.clone().unwrap_or_default()
    )
}

#[allow(non_snake_case)]
fn buildToggleDefinitions(
    specs: &[InputMenuSpec],
    params: &InputMenuToggleHookParams,
) -> Vec<InputMenuToggleDefinition> {
    if specs.is_empty() {
        return Vec::new();
    }
    specs
        .iter()
        .map(|spec| {
            let resolvedChecked = params
                .featureStates
                .get(&spec.id)
                .copied()
                .unwrap_or(spec.isChecked);
            let specForToggle = spec.clone();
            let paramsForToggle = params.clone();
            InputMenuToggleDefinition {
                id: spec.id.clone(),
                title: Some(spec.title.clone()),
                description: Some(spec.description.clone()),
                icon: spec.icon.clone(),
                isChecked: resolvedChecked,
                isEnabled: true,
                slot: spec.slot.clone(),
                onToggle: Arc::new(move || {
                    if paramsForToggle
                        .featureStates
                        .contains_key(&specForToggle.id)
                    {
                        (paramsForToggle.onToggleFeature)(specForToggle.id.clone());
                        return;
                    }
                    launchToggle(specForToggle.clone(), paramsForToggle.clone());
                }),
            }
        })
        .collect()
}

#[allow(non_snake_case)]
fn createLoadingToggle() -> InputMenuToggleDefinition {
    InputMenuToggleDefinition {
        id: "toolpkg_input_menu_loading".to_string(),
        title: Some("loading".to_string()),
        description: Some("loading".to_string()),
        icon: None,
        isChecked: false,
        isEnabled: false,
        slot: None,
        onToggle: Arc::new(|| {}),
    }
}

#[allow(non_snake_case)]
fn triggerRefresh(params: &InputMenuToggleHookParams, registryVersion: i64, paramsCacheKey: &str) {
    if !REFRESH_FLAG
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        return;
    }
    let paramsForRefresh = params.clone();
    let paramsCacheKeyForRefresh = paramsCacheKey.to_string();
    launchTask(move || {
        let resolved = loadSpecs(&paramsForRefresh);
        *INPUT_MENU_SPECS_CACHE
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg input menu specs mutex poisoned") = resolved;
        HAS_LOADED_ONCE.store(true, Ordering::SeqCst);
        LAST_HOOK_REGISTRY_VERSION.store(registryVersion, Ordering::SeqCst);
        *LAST_PARAMS_CACHE_KEY
            .get_or_init(|| Mutex::new(None))
            .lock()
            .expect("toolpkg input menu params cache mutex poisoned") =
            Some(paramsCacheKeyForRefresh);
        REFRESH_FLAG.store(false, Ordering::SeqCst);
        InputMenuTogglePluginRegistry::notifyChanged();
    });
}

#[allow(non_snake_case)]
fn loadSpecs(params: &InputMenuToggleHookParams) -> Vec<InputMenuSpec> {
    let registeredHooks = INPUT_MENU_HOOKS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .expect("toolpkg input menu hook mutex poisoned")
        .clone();
    let mut resolved = Vec::new();
    let manager = packageManager(params);
    for hook in registeredHooks {
        let result = manager.runToolPkgMainHook(
            &hook.containerPackageName,
            &hook.functionName,
            TOOLPKG_EVENT_INPUT_MENU_TOGGLE,
            None,
            Some(&hook.pluginId),
            hook.functionSource.as_deref(),
            serde_json::json!({
                "action": "create",
                "chatId": params.chatId,
                "runtime": params.runtime,
            }),
            None,
            None,
            None,
        );
        let value = match result {
            Ok(output) => decodeToolPkgHookResult(output),
            Err(error) => {
                AppLogger::e(
                    TAG,
                    &format!(
                        "ToolPkg input menu hook failed: {}:{} {}",
                        hook.containerPackageName, hook.pluginId, error
                    ),
                );
                None
            }
        };
        resolved.extend(parseInputMenuDefinitions(
            value.as_ref(),
            &hook.containerPackageName,
            &hook.functionName,
            &hook.pluginId,
            hook.functionSource.as_deref(),
        ));
    }
    resolved
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InputMenuSpec {
    containerPackageName: String,
    functionName: String,
    pluginId: String,
    functionSource: Option<String>,
    id: String,
    title: String,
    description: String,
    icon: Option<String>,
    isChecked: bool,
    slot: Option<String>,
}

#[allow(non_snake_case)]
fn parseInputMenuDefinitions(
    decoded: Option<&Value>,
    containerPackageName: &str,
    functionName: &str,
    pluginId: &str,
    functionSource: Option<&str>,
) -> Vec<InputMenuSpec> {
    let Some(array) = inputMenuDefinitionArray(decoded) else {
        return Vec::new();
    };
    let mut specs = Vec::new();
    for item in array {
        let Value::Object(object) = item else {
            continue;
        };
        let id = object
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let title = object
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if id.is_empty() || title.is_empty() {
            continue;
        }
        let description = object
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let icon = object
            .get("icon")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let slot = object
            .get("slot")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        specs.push(InputMenuSpec {
            containerPackageName: containerPackageName.to_string(),
            functionName: functionName.to_string(),
            pluginId: pluginId.to_string(),
            functionSource: functionSource.map(ToString::to_string),
            id,
            title,
            description,
            icon,
            isChecked: object
                .get("isChecked")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            slot: slot.map(|value| normalizeInputMenuSlot(Some(&value))),
        });
    }
    specs
}

#[allow(non_snake_case)]
fn inputMenuDefinitionArray(decoded: Option<&Value>) -> Option<Vec<Value>> {
    match decoded? {
        Value::Array(array) => Some(array.clone()),
        Value::Object(object) => object.get("toggles").and_then(Value::as_array).cloned(),
        _ => None,
    }
}

#[allow(non_snake_case)]
fn runInputMenuToggleHook(spec: &InputMenuSpec, params: &InputMenuToggleHookParams) {
    let manager = packageManager(params);
    if let Err(error) = manager.runToolPkgMainHook(
        &spec.containerPackageName,
        &spec.functionName,
        TOOLPKG_EVENT_INPUT_MENU_TOGGLE,
        None,
        Some(&spec.pluginId),
        spec.functionSource.as_deref(),
        serde_json::json!({
            "action": "toggle",
            "toggleId": spec.id,
            "chatId": params.chatId,
            "runtime": params.runtime,
        }),
        None,
        None,
        None,
    ) {
        AppLogger::e(
            TAG,
            &format!(
                "ToolPkg input menu toggle hook failed: {}:{} {}",
                spec.containerPackageName, spec.pluginId, error
            ),
        );
    }
}

#[allow(non_snake_case)]
fn launchToggle(spec: InputMenuSpec, params: InputMenuToggleHookParams) {
    launchTask(move || {
        runInputMenuToggleHook(&spec, &params);
        let registryVersion = HOOK_REGISTRY_VERSION.load(Ordering::SeqCst);
        let paramsCacheKey = buildCacheKey(&params);
        triggerRefresh(&params, registryVersion, &paramsCacheKey);
    });
}

#[allow(non_snake_case)]
fn packageManager(_params: &InputMenuToggleHookParams) -> PackageManager {
    toolPkgPackageManager()
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(non_snake_case)]
fn launchTask<F>(task: F)
where
    F: FnOnce() + Send + 'static,
{
    std::thread::spawn(task);
}

#[cfg(target_arch = "wasm32")]
#[allow(non_snake_case)]
fn launchTask<F>(task: F)
where
    F: FnOnce() + 'static,
{
    let callback = wasm_bindgen::closure::Closure::once_into_js(task);
    let function: &js_sys::Function = callback.unchecked_ref();
    web_sys::window()
        .expect("window must exist for toolpkg input menu task")
        .set_timeout_with_callback_and_timeout_and_arguments_0(function, 0)
        .expect("toolpkg input menu task must be scheduled");
}
