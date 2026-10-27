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
    activatedPackages: Arc<BTreeSet<String>>,
    availablePackages: Arc<BTreeMap<String, ToolPackage>>,
    enabledPackageNames: Arc<BTreeSet<String>>,
    activePackageStateIds: Arc<BTreeMap<String, Option<String>>>,
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
            activatedPackages: Arc::new(BTreeSet::new()),
            availablePackages: Arc::new(BTreeMap::new()),
            enabledPackageNames: Arc::new(BTreeSet::new()),
            activePackageStateIds: Arc::new(BTreeMap::new()),
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
        Arc::make_mut(&mut self.activePackageStateIds).clear();
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
        Arc::make_mut(&mut self.availablePackages).insert(packageName, package);
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
        Arc::make_mut(&mut self.availablePackages).insert(
            loadResult.containerPackage.name.clone(),
            loadResult.containerPackage.clone(),
        );
        for subpackage in self.toolPkgManager.registerToolPkg(loadResult) {
            Arc::make_mut(&mut self.availablePackages).insert(subpackage.name.clone(), subpackage);
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
        self.availablePackages.as_ref().clone()
    }

    /// Returns a shared view of all registered package definitions.
    #[allow(non_snake_case)]
    pub fn availablePackagesRef(&self) -> &BTreeMap<String, ToolPackage> {
        &self.availablePackages
    }

    /// Returns a mutable view of all registered package definitions.
    #[allow(non_snake_case)]
    pub fn availablePackagesMut(&mut self) -> &mut BTreeMap<String, ToolPackage> {
        Arc::make_mut(&mut self.availablePackages)
    }

    /// Replaces all package definitions after an external source scan.
    #[allow(non_snake_case)]
    pub fn replaceAvailablePackages(&mut self, availablePackages: BTreeMap<String, ToolPackage>) {
        self.availablePackages = Arc::new(availablePackages);
        Arc::make_mut(&mut self.activatedPackages).retain(|name| self.availablePackages.contains_key(name));
        Arc::make_mut(&mut self.activePackageStateIds).retain(|name, _| self.availablePackages.contains_key(name));
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
            removed = Arc::make_mut(&mut self.availablePackages).remove(&packageName).is_some();
            Arc::make_mut(&mut self.enabledPackageNames).remove(&packageName);
            Arc::make_mut(&mut self.activatedPackages).remove(&packageName);
            Arc::make_mut(&mut self.activePackageStateIds).remove(&packageName);
            for subpackage in container.subpackages {
                removed = Arc::make_mut(&mut self.availablePackages).remove(&subpackage.packageName)
                    .is_some()
                    || removed;
                Arc::make_mut(&mut self.enabledPackageNames).remove(&subpackage.packageName);
                Arc::make_mut(&mut self.activatedPackages).remove(&subpackage.packageName);
                Arc::make_mut(&mut self.activePackageStateIds).remove(&subpackage.packageName);
            }
            return removed;
        }
        Arc::make_mut(&mut self.enabledPackageNames).remove(&packageName);
        Arc::make_mut(&mut self.activatedPackages).remove(&packageName);
        Arc::make_mut(&mut self.activePackageStateIds).remove(&packageName);
        Arc::make_mut(&mut self.availablePackages).remove(&packageName).is_some()
    }

    /// Replaces the complete enabled package set.
    #[allow(non_snake_case)]
    pub fn setEnabledPackageNames(&mut self, packageNames: &[String]) {
        self.enabledPackageNames = Arc::new(packageNames
            .iter()
            .map(|name| normalizePackageName(name))
            .filter(|name| !name.is_empty())
            .collect());
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
        Arc::make_mut(&mut self.enabledPackageNames).insert(normalizePackageName(packageName))
    }

    /// Marks a package as disabled in memory.
    #[allow(non_snake_case)]
    pub fn disablePackage(&mut self, packageName: &str) -> bool {
        let packageName = normalizePackageName(packageName);
        Arc::make_mut(&mut self.activatedPackages).remove(&packageName);
        Arc::make_mut(&mut self.enabledPackageNames).remove(&packageName)
    }

    /// Activates a package for the current request or prompt session.
    #[allow(non_snake_case)]
    pub fn activatePackage(&mut self, packageName: &str) -> bool {
        Arc::make_mut(&mut self.activatedPackages).insert(normalizePackageName(packageName))
    }

    /// Deactivates a package for the current request or prompt session.
    #[allow(non_snake_case)]
    pub fn deactivatePackage(&mut self, packageName: &str) -> bool {
        Arc::make_mut(&mut self.activatedPackages).remove(&normalizePackageName(packageName))
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
        let package = self.availablePackages.get(&packageName)?;
        let stateId = self.packageStateResolver.resolvePackageStateId(package);
        Arc::make_mut(&mut self.activePackageStateIds).insert(packageName, stateId.clone());
        Some(applyPackageState(package, stateId.as_deref()))
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

    /// Selects only the executable requested by the caller, borrowing all other scripts.
    #[allow(non_snake_case)]
    pub fn effectiveTool(&self, packageName: &str, functionName: &str) -> Result<PackageTool, String> {
        let package = self.availablePackages.get(packageName.trim())
            .ok_or_else(|| format!("Package not registered: {packageName}"))?;
        let stateId = self.packageStateResolver.resolvePackageStateId(package);
        let state = stateId.as_deref().and_then(|id| package.states.iter().find(|state| state.id == id));
        let tools = match state {
            None => package.tools.iter().collect::<Vec<_>>(),
            Some(state) => mergeToolRefsForState(&package.tools, state).into_values().collect(),
        };
        crate::javascript::select_registered_package_tool_ref(&package.name, tools, functionName).cloned()
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
        Arc::make_mut(&mut self.activePackageStateIds).remove(&normalizePackageName(packageName));
    }
}

/// Applies one package state to its base tool list.
#[allow(non_snake_case)]
fn applyPackageState(package: &ToolPackage, stateId: Option<&str>) -> ToolPackage {
    let state = stateId.and_then(|stateId| package.states.iter().find(|state| state.id == stateId));
    let Some(state) = state else { return package.clone(); };
    let tools = mergeToolRefsForState(&package.tools, state).into_values().cloned().collect();
    ToolPackage {
        tools,
        name: package.name.clone(), description: package.description.clone(),
        states: package.states.clone(), env: package.env.clone(),
        is_built_in: package.is_built_in, enabled_by_default: package.enabled_by_default,
        display_name: package.display_name.clone(), category: package.category.clone(),
        author: package.author.clone(),
    }
}

/// Resolves conditional overrides before copying any executable source.
#[allow(non_snake_case)]
fn mergeToolRefsForState<'a>(baseTools: &'a [PackageTool], state: &'a ToolPackageState)
    -> BTreeMap<&'a str, &'a PackageTool> {
    let mut tools = BTreeMap::new();
    if state.inherit_tools {
        for tool in baseTools { tools.insert(tool.name.as_str(), tool); }
    }
    for name in &state.exclude_tools { tools.remove(name.as_str()); }
    for tool in &state.tools { tools.insert(tool.name.as_str(), tool); }
    tools
}

/// Normalizes a package name for map and set lookups.
#[allow(non_snake_case)]
fn normalizePackageName(packageName: &str) -> String {
    packageName.trim().to_string()
}

#[cfg(test)]
mod tests {
    /// Asserts that an immediate mock completion is ready on its first poll.
    fn pollReadyCompletion<T>(mut completion: crate::javascript::JsExecutionCompletion<T>) -> T {
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match std::future::Future::poll(completion.as_mut(), &mut context) {
            std::task::Poll::Ready(value) => value,
            std::task::Poll::Pending => panic!("immediate mock completion must be ready"),
        }
    }

    use std::collections::BTreeMap;
    use std::io::Write;
    use std::sync::Arc;

    use serde_json::Value;

    use super::{PackageStateResolver, PluginPackageManager};
    use crate::execution_result::JsExecutionResult;
    use crate::javascript::{
        JsExecutionEngine, JsExecutionFuture, ToolPkgExecutionContext,
        ToolPkgMainRegistrationCapture,
    };
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
        ) -> crate::javascript::JsExecutionCompletion<JsExecutionResult<Option<String>>> {
            let result = (|| {
                Ok(params
                    .get("event")
                    .and_then(Value::as_str)
                    .map(str::to_string))
            })();
            Box::pin(std::future::ready(result))
        }

        /// Returns the dispatched event name for exact-deadline hook tests.
        #[allow(non_snake_case)]
        fn execute_script_function_with_timeout_millis(
            &self,
            _script: &str,
            _functionName: &str,
            params: &BTreeMap<String, Value>,
            _envOverrides: &BTreeMap<String, String>,
            _onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
            _dispatchIntermediateOnMain: bool,
            _timeoutMillis: u64,
        ) -> crate::javascript::JsExecutionCompletion<JsExecutionResult<Option<String>>> {
            let result = (|| {
                Ok(params
                    .get("event")
                    .and_then(Value::as_str)
                    .map(str::to_string))
            })();
            Box::pin(std::future::ready(result))
        }

        /// Returns the dispatched event asynchronously for package manager tests.
        fn execute_script_function_async(
            &self,
            _script: String,
            _function_name: String,
            params: BTreeMap<String, Value>,
            _env_overrides: BTreeMap<String, String>,
            _on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
            _dispatch_intermediate_on_main: bool,
        ) -> JsExecutionFuture<JsExecutionResult<Option<String>>> {
            Box::pin(async move {
                Ok(params
                    .get("event")
                    .and_then(Value::as_str)
                    .map(str::to_string))
            })
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

        /// Rejects live-session rendering in this recording-only engine.
        fn execute_compose_dsl_script_stream_async(
            &self, _script: String, _runtime_options: BTreeMap<String, Value>,
            _env_overrides: BTreeMap<String, String>, _text_resources: Arc<BTreeMap<String, String>>,
            _on_response: Arc<dyn Fn(operit_host_api::HostJavaScriptValue) + Send + Sync>,
        ) -> crate::javascript::JsExecutionFuture<JsExecutionResult<Option<Value>>> {
            Box::pin(async { Err(crate::execution_result::JsExecutionError::invalid_request("This recording engine does not execute live Compose sessions")) })
        }

        /// Returns the dispatched action id asynchronously for tests.
        fn dispatch_compose_dsl_action_stream_async(
            &self,
            actionId: String,
            _payload: Option<Value>,
            _runtimeOptions: BTreeMap<String, Value>,
            _envOverrides: BTreeMap<String, String>,
            _on_intermediate_result: Arc<dyn Fn(operit_host_api::HostJavaScriptValue) + Send + Sync>,
        ) -> JsExecutionFuture<JsExecutionResult<Option<Value>>> {
            Box::pin(async move { Ok(Some(Value::String(actionId))) })
        }

        /// Releases no resources for the test engine.
        fn destroy(&self) {}
    }

    /// Factory used to create test JavaScript engines.
    struct TestExecutionEngineFactory;

    impl ToolPkgExecutionEngineFactory for TestExecutionEngineFactory {
        /// Creates one generic test JavaScript engine.
        #[allow(non_snake_case)]
        fn createExecutionEngine(&self) -> Result<Arc<dyn JsExecutionEngine>, String> {
            Ok(Arc::new(TestExecutionEngine))
        }

        /// Creates one ToolPkg test JavaScript engine.
        #[allow(non_snake_case)]
        fn createToolPkgExecutionEngine(
            &self,
            _context: ToolPkgExecutionContext,
        ) -> Result<Arc<dyn JsExecutionEngine>, String> {
            Ok(Arc::new(TestExecutionEngine))
        }

        /// Completes the asynchronous factory contract with this consumer's explicit engine implementation.
        #[allow(non_snake_case)]
        fn acquireToolPkgExecutionEngine(
            &self, context: ToolPkgExecutionContext,
        ) -> crate::javascript::JsExecutionFuture<Result<Arc<dyn JsExecutionEngine>, String>> {
            let result = self.createToolPkgExecutionEngine(context);
            Box::pin(async move { result })
        }
    }

    /// Asset source used by tests that load the ToolPkg hook main entry.
    struct TestAssetSource;

    impl ToolPkgAssetSource for TestAssetSource {
        /// Returns a minimal embedded ToolPkg archive containing the hook main entry.
        #[allow(non_snake_case)]
        fn toolPkgAssetBytes(&self, _assetName: &str) -> Option<Vec<u8>> {
            let mut bytes = Vec::new();
            let mut archive = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            archive
                .start_file("main.js", options)
                .expect("test ToolPkg main entry should start");
            archive
                .write_all(b"function onEvent() {}")
                .expect("test ToolPkg main entry should be written");
            archive
                .finish()
                .expect("test ToolPkg archive should finish");
            Some(bytes)
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

    /// Execution snapshots share script storage and detach only when the catalog changes.
    #[test]
    fn catalog_snapshots_share_scripts_and_isolate_writes() {
        let mut manager = packageManager();
        manager.registerPackage(ToolPackage {
            name: "demo".into(), tools: vec![PackageTool {
                name: "run".into(), script: "large source".repeat(1000), ..Default::default()
            }], ..Default::default()
        });
        manager.enablePackage("demo");
        manager.activatePackage("demo");
        let snapshot = manager.clone();
        assert!(Arc::ptr_eq(&manager.availablePackages, &snapshot.availablePackages));
        assert!(Arc::ptr_eq(&manager.enabledPackageNames, &snapshot.enabledPackageNames));
        manager.availablePackagesMut().get_mut("demo").unwrap().tools[0].script = "updated".into();
        manager.disablePackage("demo");
        assert_eq!(manager.effectiveTool("demo", "run").unwrap().script, "updated");
        assert_eq!(snapshot.effectiveTool("demo", "run").unwrap().script, "large source".repeat(1000));
        assert!(snapshot.isPackageEnabled("demo"));
        assert!(snapshot.isPackageActivated("demo"));
        assert!(!manager.isPackageActivated("demo"));
    }

    /// Borrowed selection keeps state exclusion and override behavior identical to catalog display.
    #[test]
    fn effective_tool_preserves_conditional_overrides() {
        let tool = |name: &str, script: &str| PackageTool {
            name: name.into(), script: script.into(), ..Default::default()
        };
        let mut manager = packageManager();
        manager.registerPackage(ToolPackage {
            name: "demo".into(), tools: vec![tool("run", "base"), tool("hidden", "hidden")],
            states: vec![ToolPackageState {
                id: "state".into(), inherit_tools: true, exclude_tools: vec!["hidden".into()],
                tools: vec![tool("run", "override"), tool("added", "added")], ..Default::default()
            }], ..Default::default()
        });
        assert_eq!(manager.effectiveTool("demo", "run").unwrap().script, "override");
        assert_eq!(manager.effectiveTool("demo", "added").unwrap().script, "added");
        assert!(manager.effectiveTool("demo", "hidden").is_err());
        for tool in manager.effectivePackage("demo").unwrap().tools {
            assert_eq!(manager.effectiveTool("demo", &tool.name).unwrap().script, tool.script);
        }
        manager.availablePackagesMut().get_mut("demo").unwrap().states[0].inherit_tools = false;
        assert_eq!(manager.effectiveTool("demo", "run").unwrap().script, "override");
        assert!(manager.effectiveTool("demo", "hidden").is_err());
    }

    /// Fast lookup must retain all executable admission checks without copying the package.
    #[test]
    fn effective_tool_rejects_ambiguous_and_non_executable_declarations() {
        let mut manager = packageManager();
        let declaration = PackageTool { name: "run".into(), script: "return 1".into(), ..Default::default() };
        manager.registerPackage(ToolPackage {
            name: "demo".into(), tools: vec![declaration.clone(), declaration], ..Default::default()
        });
        assert!(manager.effectiveTool("demo", "run").unwrap_err().contains("Duplicate tool registration"));
        let package = manager.availablePackagesMut().get_mut("demo").unwrap();
        package.tools.pop();
        package.tools[0].advice = true;
        assert!(manager.effectiveTool("demo", "run").unwrap_err().contains("advisory"));
        let package = manager.availablePackagesMut().get_mut("demo").unwrap();
        package.tools[0].advice = false;
        package.tools[0].script.clear();
        assert!(manager.effectiveTool("demo", "run").unwrap_err().contains("empty"));
        assert!(manager.effectiveTool("demo", "missing").is_err());
        assert!(manager.effectiveTool("missing", "run").is_err());
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
                sourceType: ToolPkgSourceType::ASSET,
                sourcePath: "test-hook.toolpkg".to_string(),
                ..ToolPkgContainerRuntime::default()
            },
            marketOrigin: None,
        });
        assert!(registered);
        manager.setEnabledPackageNames(&["container".to_string()]);

        let output = pollReadyCompletion(manager.toolPkgManager().dispatchToolPkgHook(
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
                timeoutMillis: None,
                dispatchIntermediateOnMain: true,
                onIntermediateResult: None,
            },
        ))
        .expect("ToolPkg hook dispatch must succeed");

        assert_eq!(output.as_deref(), Some("host_event"));
    }
}
