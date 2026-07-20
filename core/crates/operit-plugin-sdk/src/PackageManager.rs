use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::package::{PackageTool, ToolPackage, ToolPackageState};
use crate::toolpkg::ToolPkgManager::{
    ToolPkgAssetSource, ToolPkgExecutionEngineFactory, ToolPkgManager,
};
use crate::toolpkg::ToolPkgParser::{
    ToolPkgContainerRuntime, ToolPkgLoadResult, ToolPkgSubpackageRuntime,
};
use operit_host_api::FileSystemHost;

/// Resolves the active conditional state for one package.
pub trait PackageStateResolver: Send + Sync {
    /// Returns the selected state id or no state for the supplied package.
    #[allow(non_snake_case)]
    fn resolvePackageStateId(&self, package: &ToolPackage) -> Option<String>;
}

/// Owns generic JavaScript package and ToolPkg state for SDK consumers.
#[derive(Clone)]
#[allow(non_snake_case)]
pub struct PluginPackageManager {
    activatedPackages: BTreeSet<String>,
    availablePackages: BTreeMap<String, ToolPackage>,
    enabledPackageNames: BTreeSet<String>,
    activePackageStateIds: BTreeMap<String, Option<String>>,
    toolPkgManager: ToolPkgManager,
    packageStateResolver: Arc<dyn PackageStateResolver>,
}

impl PluginPackageManager {
    /// Creates a package manager from interfaces implemented by the embedding application.
    pub fn new(
        executionEngineFactory: Arc<dyn ToolPkgExecutionEngineFactory>,
        assetSource: Arc<dyn ToolPkgAssetSource>,
        fileSystemHost: Arc<dyn FileSystemHost>,
        packageStateResolver: Arc<dyn PackageStateResolver>,
    ) -> Self {
        Self {
            activatedPackages: BTreeSet::new(),
            availablePackages: BTreeMap::new(),
            enabledPackageNames: BTreeSet::new(),
            activePackageStateIds: BTreeMap::new(),
            toolPkgManager: ToolPkgManager::new(
                executionEngineFactory,
                assetSource,
                fileSystemHost,
            ),
            packageStateResolver,
        }
    }

    /// Replaces the package state resolver used for conditional package states.
    #[allow(non_snake_case)]
    pub fn updatePackageStateResolver(
        &mut self,
        packageStateResolver: Arc<dyn PackageStateResolver>,
    ) {
        self.packageStateResolver = packageStateResolver;
        self.activePackageStateIds.clear();
    }

    /// Returns the embedded ToolPkg manager.
    #[allow(non_snake_case)]
    pub fn toolPkgManager(&self) -> &ToolPkgManager {
        &self.toolPkgManager
    }

    /// Returns the embedded ToolPkg manager for mutation.
    #[allow(non_snake_case)]
    pub fn toolPkgManagerMut(&mut self) -> &mut ToolPkgManager {
        &mut self.toolPkgManager
    }

    /// Registers or replaces one JavaScript package definition.
    #[allow(non_snake_case)]
    pub fn registerPackage(&mut self, package: ToolPackage) {
        let packageName = normalizePackageName(&package.name);
        self.availablePackages.insert(packageName, package);
    }

    /// Registers a loaded ToolPkg container and every executable subpackage.
    #[allow(non_snake_case)]
    pub fn registerToolPkg(&mut self, loadResult: ToolPkgLoadResult) -> bool {
        if !self
            .toolPkgManager
            .canRegisterToolPkg(&loadResult, &self.availablePackages)
        {
            return false;
        }
        self.availablePackages.insert(
            loadResult.containerPackage.name.clone(),
            loadResult.containerPackage.clone(),
        );
        for subpackage in self.toolPkgManager.registerToolPkg(loadResult) {
            self.availablePackages
                .insert(subpackage.name.clone(), subpackage);
        }
        true
    }

    /// Returns whether a package definition is registered.
    #[allow(non_snake_case)]
    pub fn containsPackage(&self, packageName: &str) -> bool {
        self.availablePackages
            .contains_key(&normalizePackageName(packageName))
    }

    /// Returns one raw package definition.
    #[allow(non_snake_case)]
    pub fn package(&self, packageName: &str) -> Option<ToolPackage> {
        self.availablePackages
            .get(&normalizePackageName(packageName))
            .cloned()
    }

    /// Returns all registered package definitions.
    #[allow(non_snake_case)]
    pub fn availablePackages(&self) -> BTreeMap<String, ToolPackage> {
        self.availablePackages.clone()
    }

    /// Returns a shared view of all registered package definitions.
    #[allow(non_snake_case)]
    pub fn availablePackagesRef(&self) -> &BTreeMap<String, ToolPackage> {
        &self.availablePackages
    }

    /// Returns a mutable view of all registered package definitions.
    #[allow(non_snake_case)]
    pub fn availablePackagesMut(&mut self) -> &mut BTreeMap<String, ToolPackage> {
        &mut self.availablePackages
    }

    /// Replaces all package definitions after an external source scan.
    #[allow(non_snake_case)]
    pub fn replaceAvailablePackages(&mut self, availablePackages: BTreeMap<String, ToolPackage>) {
        self.availablePackages = availablePackages;
        self.activatedPackages
            .retain(|name| self.availablePackages.contains_key(name));
        self.activePackageStateIds
            .retain(|name, _| self.availablePackages.contains_key(name));
    }

    /// Replaces ToolPkg runtime maps after an external source scan.
    #[allow(non_snake_case)]
    pub fn replaceToolPkgRuntimes(
        &mut self,
        containers: BTreeMap<String, ToolPkgContainerRuntime>,
        subpackages: BTreeMap<String, ToolPkgSubpackageRuntime>,
    ) {
        self.toolPkgManager
            .replaceRuntimeMaps(containers, subpackages);
    }

    /// Removes one package or an entire ToolPkg container.
    #[allow(non_snake_case)]
    pub fn removePackage(&mut self, packageName: &str) -> bool {
        let packageName = normalizePackageName(packageName);
        let mut removed = false;
        if let Some(container) = self.toolPkgManager.removeToolPkgContainer(&packageName) {
            removed = self.availablePackages.remove(&packageName).is_some();
            self.enabledPackageNames.remove(&packageName);
            self.activatedPackages.remove(&packageName);
            self.activePackageStateIds.remove(&packageName);
            for subpackage in container.subpackages {
                removed = self
                    .availablePackages
                    .remove(&subpackage.packageName)
                    .is_some()
                    || removed;
                self.enabledPackageNames.remove(&subpackage.packageName);
                self.activatedPackages.remove(&subpackage.packageName);
                self.activePackageStateIds.remove(&subpackage.packageName);
            }
            return removed;
        }
        self.enabledPackageNames.remove(&packageName);
        self.activatedPackages.remove(&packageName);
        self.activePackageStateIds.remove(&packageName);
        self.availablePackages.remove(&packageName).is_some()
    }

    /// Replaces the complete enabled package set.
    #[allow(non_snake_case)]
    pub fn setEnabledPackageNames(&mut self, packageNames: &[String]) {
        self.enabledPackageNames = packageNames
            .iter()
            .map(|name| normalizePackageName(name))
            .filter(|name| !name.is_empty())
            .collect();
    }

    /// Returns enabled package names in stable order.
    #[allow(non_snake_case)]
    pub fn enabledPackageNames(&self) -> Vec<String> {
        self.enabledPackageNames.iter().cloned().collect()
    }

    /// Returns whether a package is enabled.
    #[allow(non_snake_case)]
    pub fn isPackageEnabled(&self, packageName: &str) -> bool {
        self.enabledPackageNames
            .contains(&normalizePackageName(packageName))
    }

    /// Marks a package as enabled in memory.
    #[allow(non_snake_case)]
    pub fn enablePackage(&mut self, packageName: &str) -> bool {
        self.enabledPackageNames
            .insert(normalizePackageName(packageName))
    }

    /// Marks a package as disabled in memory.
    #[allow(non_snake_case)]
    pub fn disablePackage(&mut self, packageName: &str) -> bool {
        let packageName = normalizePackageName(packageName);
        self.activatedPackages.remove(&packageName);
        self.enabledPackageNames.remove(&packageName)
    }

    /// Activates a package for the current request or prompt session.
    #[allow(non_snake_case)]
    pub fn activatePackage(&mut self, packageName: &str) -> bool {
        self.activatedPackages
            .insert(normalizePackageName(packageName))
    }

    /// Deactivates a package for the current request or prompt session.
    #[allow(non_snake_case)]
    pub fn deactivatePackage(&mut self, packageName: &str) -> bool {
        self.activatedPackages
            .remove(&normalizePackageName(packageName))
    }

    /// Returns whether a package is active for the current request.
    #[allow(non_snake_case)]
    pub fn isPackageActivated(&self, packageName: &str) -> bool {
        self.activatedPackages
            .contains(&normalizePackageName(packageName))
    }

    /// Returns active package names in stable order.
    #[allow(non_snake_case)]
    pub fn activePackageNames(&self) -> Vec<String> {
        self.activatedPackages.iter().cloned().collect()
    }

    /// Selects and records the active conditional state for one package.
    #[allow(non_snake_case)]
    pub fn selectPackageState(&mut self, packageName: &str) -> Option<ToolPackage> {
        let packageName = normalizePackageName(packageName);
        let package = self.availablePackages.get(&packageName)?.clone();
        let stateId = self.packageStateResolver.resolvePackageStateId(&package);
        self.activePackageStateIds
            .insert(packageName, stateId.clone());
        Some(applyPackageState(&package, stateId.as_deref()))
    }

    /// Returns one package with its currently resolvable conditional state applied.
    #[allow(non_snake_case)]
    pub fn effectivePackage(&self, packageName: &str) -> Option<ToolPackage> {
        let package = self
            .availablePackages
            .get(&normalizePackageName(packageName))?;
        let stateId = self.packageStateResolver.resolvePackageStateId(package);
        Some(applyPackageState(package, stateId.as_deref()))
    }

    /// Returns the last state id selected for one package.
    #[allow(non_snake_case)]
    pub fn activePackageStateId(&self, packageName: &str) -> Option<String> {
        self.activePackageStateIds
            .get(&normalizePackageName(packageName))
            .cloned()
            .flatten()
    }

    /// Clears the recorded state selection for one package.
    #[allow(non_snake_case)]
    pub fn clearActivePackageState(&mut self, packageName: &str) {
        self.activePackageStateIds
            .remove(&normalizePackageName(packageName));
    }
}

/// Applies one package state to its base tool list.
#[allow(non_snake_case)]
fn applyPackageState(package: &ToolPackage, stateId: Option<&str>) -> ToolPackage {
    let state = stateId.and_then(|stateId| package.states.iter().find(|state| state.id == stateId));
    let Some(state) = state else {
        return package.clone();
    };
    ToolPackage {
        tools: mergeToolsForState(&package.tools, state),
        ..package.clone()
    }
}

/// Merges base tools with one conditional package state.
#[allow(non_snake_case)]
fn mergeToolsForState(baseTools: &[PackageTool], state: &ToolPackageState) -> Vec<PackageTool> {
    let mut toolMap = BTreeMap::new();
    if state.inherit_tools {
        for tool in baseTools {
            toolMap.insert(tool.name.clone(), tool.clone());
        }
    }
    for toolName in &state.exclude_tools {
        toolMap.remove(toolName);
    }
    for tool in &state.tools {
        toolMap.insert(tool.name.clone(), tool.clone());
    }
    toolMap.into_values().collect()
}

/// Normalizes a package name for map and set lookups.
#[allow(non_snake_case)]
fn normalizePackageName(packageName: &str) -> String {
    packageName.trim().to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::Value;

    use super::{PackageStateResolver, PluginPackageManager};
    use crate::execution_result::JsExecutionResult;
    use crate::javascript::{JsExecutionEngine, ToolPkgMainRegistrationCapture};
    use crate::package::{LocalizedText, PackageTool, ToolPackage, ToolPackageState};
    use crate::toolpkg::ToolPkgHooks::{ToolPkgHookDispatcher, ToolPkgHookInvocation};
    use crate::toolpkg::ToolPkgManager::{ToolPkgAssetSource, ToolPkgExecutionEngineFactory};
    use crate::toolpkg::ToolPkgParser::{
        ToolPkgContainerRuntime, ToolPkgLoadResult, ToolPkgSourceType,
    };
    use operit_host_api::{
        FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
        GrepCodeResult, HostEnvironmentDescriptor, HostError, HostResult,
    };

    /// Rejects filesystem operations because these package manager tests do not access host files.
    struct RejectingFileSystemHost;

    impl RejectingFileSystemHost {
        /// Returns the explicit error used for unsupported test filesystem operations.
        fn unsupported<T>() -> HostResult<T> {
            Err(HostError::new("filesystem access is not used by this test"))
        }
    }

    impl FileSystemHost for RejectingFileSystemHost {
        /// Returns the test host label.
        fn envLabel(&self) -> &str {
            "test"
        }

        /// Returns the test environment descriptor.
        fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
            HostEnvironmentDescriptor::linux()
        }

        /// Rejects path validation.
        fn validatePath(&self, _path: &str, _paramName: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects directory listing.
        fn listFiles(&self, _path: &str) -> HostResult<Vec<FileEntry>> {
            Self::unsupported()
        }

        /// Rejects text reads.
        fn readFile(&self, _path: &str) -> HostResult<String> {
            Self::unsupported()
        }

        /// Rejects bounded text reads.
        fn readFileWithLimit(&self, _path: &str, _maxBytes: usize) -> HostResult<String> {
            Self::unsupported()
        }

        /// Rejects byte reads.
        fn readFileBytes(&self, _path: &str) -> HostResult<Vec<u8>> {
            Self::unsupported()
        }

        /// Rejects text writes.
        fn writeFile(&self, _path: &str, _content: &str, _append: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects byte writes.
        fn writeFileBytes(&self, _path: &str, _content: &[u8]) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects deletion.
        fn deleteFile(&self, _path: &str, _recursive: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects file existence checks.
        fn fileExists(&self, _path: &str) -> HostResult<FileExistence> {
            Self::unsupported()
        }

        /// Rejects moves.
        fn moveFile(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects copies.
        fn copyFile(&self, _source: &str, _destination: &str, _recursive: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects directory creation.
        fn makeDirectory(&self, _path: &str, _createParents: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects file searching.
        fn findFiles(&self, _request: FindFilesRequest) -> HostResult<Vec<String>> {
            Self::unsupported()
        }

        /// Rejects file metadata reads.
        fn fileInfo(&self, _path: &str) -> HostResult<FileInfo> {
            Self::unsupported()
        }

        /// Rejects code searches.
        fn grepCode(&self, _request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
            Self::unsupported()
        }

        /// Rejects archive creation.
        fn zipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects archive extraction.
        fn unzipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects host file opening.
        fn openFile(&self, _path: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects host file sharing.
        fn shareFile(&self, _path: &str, _title: &str) -> HostResult<()> {
            Self::unsupported()
        }
    }

    /// JavaScript engine used by package manager contract tests.
    struct TestExecutionEngine;

    impl JsExecutionEngine for TestExecutionEngine {
        /// Returns the dispatched event name as the hook output.
        #[allow(non_snake_case)]
        fn execute_script_function(
            &self,
            _script: &str,
            _functionName: &str,
            params: &BTreeMap<String, Value>,
            _envOverrides: &BTreeMap<String, String>,
            _onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
            _dispatchIntermediateOnMain: bool,
            _timeoutSec: u64,
        ) -> JsExecutionResult<Option<String>> {
            Ok(params
                .get("event")
                .and_then(Value::as_str)
                .map(str::to_string))
        }

        /// Returns an empty registration capture for tests.
        #[allow(non_snake_case)]
        fn execute_toolpkg_main_registration_function_with_text_resources(
            &self,
            _script: &str,
            _functionName: &str,
            _params: &BTreeMap<String, Value>,
            _textResources: Option<Arc<BTreeMap<String, String>>>,
        ) -> JsExecutionResult<ToolPkgMainRegistrationCapture> {
            Ok(ToolPkgMainRegistrationCapture::default())
        }

        /// Returns the supplied Compose DSL script.
        #[allow(non_snake_case)]
        fn execute_compose_dsl_script(
            &self,
            script: &str,
            _runtimeOptions: &BTreeMap<String, Value>,
            _envOverrides: &BTreeMap<String, String>,
        ) -> JsExecutionResult<Option<String>> {
            Ok(Some(script.to_string()))
        }

        /// Returns the dispatched action id.
        #[allow(non_snake_case)]
        fn dispatch_compose_dsl_action(
            &self,
            actionId: &str,
            _payload: Option<Value>,
            _runtimeOptions: &BTreeMap<String, Value>,
            _envOverrides: &BTreeMap<String, String>,
            _onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
        ) -> JsExecutionResult<Option<String>> {
            Ok(Some(actionId.to_string()))
        }

        /// Releases no resources for the test engine.
        fn destroy(&self) {}
    }

    /// Factory used to create test JavaScript engines.
    struct TestExecutionEngineFactory;

    impl ToolPkgExecutionEngineFactory for TestExecutionEngineFactory {
        /// Creates one test JavaScript engine.
        #[allow(non_snake_case)]
        fn createToolPkgExecutionEngine(&self) -> Arc<dyn JsExecutionEngine> {
            Arc::new(TestExecutionEngine)
        }
    }

    /// Asset source used by tests that load ToolPkg resources from directories.
    struct TestAssetSource;

    impl ToolPkgAssetSource for TestAssetSource {
        /// Returns no embedded assets because tests use external directories.
        #[allow(non_snake_case)]
        fn toolPkgAssetBytes(&self, _assetName: &str) -> Option<Vec<u8>> {
            None
        }
    }

    /// Resolver that selects the first declared package state.
    struct TestPackageStateResolver;

    impl PackageStateResolver for TestPackageStateResolver {
        /// Returns the first state id declared by the package.
        #[allow(non_snake_case)]
        fn resolvePackageStateId(&self, package: &ToolPackage) -> Option<String> {
            package.states.first().map(|state| state.id.clone())
        }
    }

    /// Creates a package manager with deterministic test interfaces.
    fn packageManager() -> PluginPackageManager {
        PluginPackageManager::new(
            Arc::new(TestExecutionEngineFactory),
            Arc::new(TestAssetSource),
            Arc::new(RejectingFileSystemHost),
            Arc::new(TestPackageStateResolver),
        )
    }

    /// Creates an isolated temporary directory for one test.
    fn temporaryDirectory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time must be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "operit-plugin-sdk-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    /// Verifies JavaScript package registration, activation, and state selection.
    #[test]
    fn managesJavaScriptPackageState() {
        let mut manager = packageManager();
        manager.registerPackage(ToolPackage {
            name: "demo".to_string(),
            tools: vec![PackageTool {
                name: "base".to_string(),
                ..PackageTool::default()
            }],
            states: vec![ToolPackageState {
                id: "extended".to_string(),
                inherit_tools: true,
                tools: vec![PackageTool {
                    name: "extra".to_string(),
                    ..PackageTool::default()
                }],
                ..ToolPackageState::default()
            }],
            display_name: LocalizedText::default(),
            ..ToolPackage::default()
        });
        assert!(manager.enablePackage("demo"));
        assert!(manager.activatePackage("demo"));
        let selected = manager
            .selectPackageState("demo")
            .expect("registered package must be selectable");

        assert_eq!(selected.tools.len(), 2);
        assert_eq!(
            manager.activePackageStateId("demo").as_deref(),
            Some("extended")
        );
        assert!(manager.isPackageActivated("demo"));
    }

    /// Verifies that an embedding application can dispatch a ToolPkg hook through the SDK.
    #[test]
    fn dispatchesToolPkgHookThroughPublicInterface() {
        let root = temporaryDirectory("hook");
        fs::create_dir_all(&root).expect("temporary ToolPkg directory must be created");
        fs::write(root.join("main.js"), "function onEvent() {}")
            .expect("ToolPkg main script must be written");

        let mut manager = packageManager();
        let registered = manager.registerToolPkg(ToolPkgLoadResult {
            containerPackage: ToolPackage {
                name: "container".to_string(),
                ..ToolPackage::default()
            },
            subpackagePackages: Vec::new(),
            containerRuntime: ToolPkgContainerRuntime {
                packageName: "container".to_string(),
                mainEntry: "main.js".to_string(),
                sourceType: ToolPkgSourceType::EXTERNAL,
                sourcePath: root.to_string_lossy().to_string(),
                ..ToolPkgContainerRuntime::default()
            },
        });
        assert!(registered);
        manager.setEnabledPackageNames(&["container".to_string()]);

        let output = manager
            .toolPkgManager()
            .dispatchToolPkgHook(
                &manager.enabledPackageNames(),
                ToolPkgHookInvocation {
                    containerPackageName: "container".to_string(),
                    functionName: "onEvent".to_string(),
                    event: "host_event".to_string(),
                    eventName: None,
                    pluginId: None,
                    inlineFunctionSource: None,
                    eventPayload: Value::Object(Default::default()),
                    executionContextKey: None,
                    runtimeKind: None,
                    envOverrides: BTreeMap::new(),
                    timestampMs: 1,
                    timeoutSec: 10,
                    dispatchIntermediateOnMain: true,
                    onIntermediateResult: None,
                },
            )
            .expect("ToolPkg hook dispatch must succeed");

        assert_eq!(output.as_deref(), Some("host_event"));
        fs::remove_dir_all(root).expect("temporary ToolPkg directory must be removed");
    }
}
