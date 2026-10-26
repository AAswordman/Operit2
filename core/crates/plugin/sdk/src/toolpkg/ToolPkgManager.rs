use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::sync::{Arc, Mutex, RwLock};

use operit_host_api::FileSystemHost;
use serde_json::Value;

use crate::javascript::{JsExecutionFuture, JsExecutionEngine, ToolPkgExecutionContext, ToolPkgTextResourceHost};
use crate::package::ToolPackage;
use crate::toolpkg::ToolPkgHooks::{ToolPkgHookDispatcher, ToolPkgHookInvocation};
use crate::toolpkg::ToolPkgParser::{
    ToolPkgContainerRuntime, ToolPkgLoadResult, ToolPkgSourceType, ToolPkgSubpackageRuntime,
};

/// Listener notified when the set of active ToolPkg containers changes.
pub type ToolPkgRuntimeChangeListener = Arc<dyn Fn(Vec<ToolPkgContainerRuntime>) + Send + Sync>;

/// Creates JavaScript engines used to execute ToolPkg hooks and UI scripts.
pub trait ToolPkgExecutionEngineFactory: Send + Sync {
    /// Creates one isolated JavaScript engine without a ToolPkg package environment.
    #[allow(non_snake_case)]
    fn createExecutionEngine(&self) -> Result<Arc<dyn JsExecutionEngine>, String>;

    /// Creates one isolated JavaScript engine for a ToolPkg execution context.
    #[allow(non_snake_case)]
    fn createToolPkgExecutionEngine(
        &self,
        context: ToolPkgExecutionContext,
    ) -> Result<Arc<dyn JsExecutionEngine>, String>;

    /// Acquires an isolated prepared engine for a package through asynchronous scheduling.
    #[allow(non_snake_case)]
    fn acquireToolPkgExecutionEngine(
        &self,
        context: ToolPkgExecutionContext,
    ) -> JsExecutionFuture<Result<Arc<dyn JsExecutionEngine>, String>>;
}

/// Resolves embedded ToolPkg archive bytes owned by the embedding application.
pub trait ToolPkgAssetSource: Send + Sync {
    /// Returns the bytes of one embedded ToolPkg archive by asset name.
    #[allow(non_snake_case)]
    fn toolPkgAssetBytes(&self, assetName: &str) -> Option<Vec<u8>>;
}

/// Associates one cached JavaScript engine with its owning ToolPkg container.
#[derive(Clone)]
struct ToolPkgExecutionEngineEntry {
    containerPackageName: String,
    engine: Arc<dyn JsExecutionEngine>,
    activeLeases: usize,
    /// Container-owned main realms survive the end of an individual invocation.
    containerOwned: bool,
}

/// Holds one execution lease until the associated asynchronous operation finishes or is cancelled.
pub struct ToolPkgExecutionEngineLease {
    manager: ToolPkgManager,
    contextKey: String,
    containerPackageName: String,
    engine: Arc<dyn JsExecutionEngine>,
}

impl Drop for ToolPkgExecutionEngineLease {
    /// Releases the execution lease after the caller stops owning the asynchronous operation.
    fn drop(&mut self) {
        self.manager
            .releaseExecutionEngineLease(&self.contextKey, &self.containerPackageName, Some(&self.engine));
    }
}

/// Tracks live engines and revocation generations under the same registry lock.
#[derive(Default)]
struct ToolPkgExecutionEngineRegistry {
    mainScripts: BTreeMap<String, String>,
    entries: BTreeMap<String, ToolPkgExecutionEngineEntry>,
    packageGenerations: BTreeMap<String, u64>,
    factoryGeneration: u64,
    closed: bool,
}

type ExecutionEngineGeneration = (u64, u64);

/// Manages loaded ToolPkg runtimes, resources, listeners, and execution engines.
#[derive(Clone)]
pub struct ToolPkgManager {
    containers: Arc<RwLock<BTreeMap<String, ToolPkgContainerRuntime>>>,
    subpackageByPackageName: Arc<RwLock<BTreeMap<String, ToolPkgSubpackageRuntime>>>,
    runtimeRevisions: Arc<RwLock<BTreeMap<String, Option<u64>>>>,
    runtimeChangeListeners: Arc<Mutex<Vec<ToolPkgRuntimeChangeListener>>>,
    toolPkgExecutionEngines: Arc<Mutex<ToolPkgExecutionEngineRegistry>>,
    executionEngineFactory: Arc<dyn ToolPkgExecutionEngineFactory>,
    assetSource: Arc<dyn ToolPkgAssetSource>,
    fileSystemHost: Arc<dyn FileSystemHost>,
}

impl ToolPkgManager {
    /// Creates a ToolPkg manager from application-supplied execution and asset interfaces.
    pub fn new(
        executionEngineFactory: Arc<dyn ToolPkgExecutionEngineFactory>,
        assetSource: Arc<dyn ToolPkgAssetSource>,
        fileSystemHost: Arc<dyn FileSystemHost>,
    ) -> Self {
        Self {
            containers: Arc::new(RwLock::new(BTreeMap::new())),
            subpackageByPackageName: Arc::new(RwLock::new(BTreeMap::new())),
            runtimeRevisions: Arc::new(RwLock::new(BTreeMap::new())),
            runtimeChangeListeners: Arc::new(Mutex::new(Vec::new())),
            toolPkgExecutionEngines: Arc::new(Mutex::new(ToolPkgExecutionEngineRegistry::default())),
            executionEngineFactory,
            assetSource,
            fileSystemHost,
        }
    }

    /// Replaces the JavaScript engine factory and destroys cached engines.
    #[allow(non_snake_case)]
    pub fn updateExecutionEngineFactory(
        &mut self,
        executionEngineFactory: Arc<dyn ToolPkgExecutionEngineFactory>,
    ) {
        self.removeExecutionEnginesMatching(|_| true, |registry| registry.factoryGeneration += 1);
        self.executionEngineFactory = executionEngineFactory;
    }

    /// Ends the owning Core lifetime, including in-flight and container-owned realms.
    /// The closed registry also rejects work scheduled before an identity/root switch.
    #[allow(non_snake_case)]
    pub fn shutdownExecutionEngines(&self) {
        self.removeExecutionEnginesMatching(|_| true, |registry| {
            registry.closed = true;
            registry.mainScripts.clear();
            registry.factoryGeneration += 1;
        });
    }

    /// Returns whether a package name belongs to a registered ToolPkg container.
    #[allow(non_snake_case)]
    pub fn isToolPkgContainer(&self, packageName: &str) -> bool {
        self.containers
            .read()
            .expect("toolpkg container runtime lock poisoned")
            .contains_key(packageName.trim())
    }

    /// Returns whether a package name resolves to a ToolPkg subpackage.
    #[allow(non_snake_case)]
    pub fn hasSubpackage(&self, packageName: &str) -> bool {
        self.subpackageByPackageName
            .read()
            .expect("toolpkg subpackage runtime lock poisoned")
            .contains_key(packageName.trim())
    }

    /// Returns all registered ToolPkg container runtimes in stable name order.
    #[allow(non_snake_case)]
    pub fn getToolPkgContainerRuntimes(&self) -> Vec<ToolPkgContainerRuntime> {
        let mut runtimes = self
            .containers
            .read()
            .expect("toolpkg container runtime lock poisoned")
            .values()
            .cloned()
            .collect::<Vec<_>>();
        runtimes.sort_by(|left, right| left.packageName.cmp(&right.packageName));
        runtimes
    }

    /// Returns the registered ToolPkg container map keyed by package name.
    #[allow(non_snake_case)]
    pub fn getToolPkgContainerRuntimeMap(&self) -> BTreeMap<String, ToolPkgContainerRuntime> {
        self.containers
            .read()
            .expect("toolpkg container runtime lock poisoned")
            .clone()
    }

    /// Returns the registered ToolPkg subpackage map keyed by package name.
    #[allow(non_snake_case)]
    pub fn getToolPkgSubpackageRuntimeMap(&self) -> BTreeMap<String, ToolPkgSubpackageRuntime> {
        self.subpackageByPackageName
            .read()
            .expect("toolpkg subpackage runtime lock poisoned")
            .clone()
    }

    /// Returns one registered ToolPkg container runtime by package name.
    #[allow(non_snake_case)]
    pub fn getToolPkgContainerRuntime(
        &self,
        containerPackageName: &str,
    ) -> Option<ToolPkgContainerRuntime> {
        self.containers
            .read()
            .expect("toolpkg container runtime lock poisoned")
            .get(containerPackageName.trim())
            .cloned()
    }

    /// Resolves one ToolPkg subpackage runtime by package name.
    #[allow(non_snake_case)]
    pub fn resolveToolPkgSubpackageRuntimeInternal(
        &self,
        packageName: &str,
    ) -> Option<ToolPkgSubpackageRuntime> {
        self.subpackageByPackageName
            .read()
            .expect("toolpkg subpackage runtime lock poisoned")
            .get(packageName.trim())
            .cloned()
    }

    /// Validates whether a loaded ToolPkg can be registered without name conflicts.
    #[allow(non_snake_case)]
    pub fn canRegisterToolPkg(
        &self,
        loadResult: &ToolPkgLoadResult,
        availablePackages: &BTreeMap<String, ToolPackage>,
    ) -> bool {
        let containerName = loadResult.containerPackage.name.trim();
        let containers = self
            .containers
            .read()
            .expect("toolpkg container runtime lock poisoned");
        let subpackages = self
            .subpackageByPackageName
            .read()
            .expect("toolpkg subpackage runtime lock poisoned");
        if containerName.is_empty()
            || containers.contains_key(containerName)
            || availablePackages.contains_key(containerName)
        {
            return false;
        }
        for subpackage in &loadResult.subpackagePackages {
            let packageName = subpackage.name.trim();
            if packageName.is_empty()
                || containers.contains_key(packageName)
                || availablePackages.contains_key(packageName)
                || subpackages.contains_key(packageName)
            {
                return false;
            }
        }
        true
    }

    /// Registers a loaded ToolPkg and returns its executable subpackages.
    #[allow(non_snake_case)]
    pub fn registerToolPkg(&mut self, loadResult: ToolPkgLoadResult) -> Vec<ToolPackage> {
        let containerName = loadResult.containerPackage.name.clone();
        self.runtimeRevisions.write().expect("toolpkg revision lock poisoned")
            .insert(containerName.clone(), self.runtimeRevision(&loadResult.containerRuntime));
        self.containers
            .write()
            .expect("toolpkg container runtime lock poisoned")
            .insert(containerName.clone(), loadResult.containerRuntime.clone());
        let mut subpackages = self
            .subpackageByPackageName
            .write()
            .expect("toolpkg subpackage runtime lock poisoned");
        for runtime in loadResult.containerRuntime.subpackages {
            subpackages.insert(runtime.packageName.clone(), runtime);
        }
        drop(subpackages);
        // Publish the replacement before revoking: a racing creator must never
        // bind old metadata using the replacement generation.
        self.revokeContainerExecutionContexts(&containerName);
        loadResult.subpackagePackages
    }

    /// Removes a ToolPkg container and its subpackage runtime mappings.
    #[allow(non_snake_case)]
    pub fn removeToolPkgContainer(
        &mut self,
        containerPackageName: &str,
    ) -> Option<ToolPkgContainerRuntime> {
        let runtime = self
            .containers
            .write()
            .expect("toolpkg container runtime lock poisoned")
            .remove(containerPackageName.trim())?;
        let mut subpackages = self
            .subpackageByPackageName
            .write()
            .expect("toolpkg subpackage runtime lock poisoned");
        for subpackage in &runtime.subpackages {
            subpackages.remove(subpackage.packageName.trim());
        }
        drop(subpackages);
        self.revokeContainerExecutionContexts(&runtime.packageName);
        self.runtimeRevisions.write().expect("toolpkg revision lock poisoned").remove(&runtime.packageName);
        Some(runtime)
    }

    /// Preserves unchanged container realms; changed/removed sources revoke every owned context.
    #[allow(non_snake_case)]
    pub fn replaceRuntimeMaps(
        &mut self,
        containers: BTreeMap<String, ToolPkgContainerRuntime>,
        subpackageByPackageName: BTreeMap<String, ToolPkgSubpackageRuntime>,
    ) {
        let revisions = containers.iter()
            .map(|(name, runtime)| (name.clone(), self.runtimeRevision(runtime)))
            .collect::<BTreeMap<_, _>>();
        let revoked = {
            let mut previous = self.runtimeRevisions.write().expect("toolpkg revision lock poisoned");
            let revoked = previous.iter().filter_map(|(name, revision)| {
                // A failed source read must never make a stale realm appear unchanged.
                (revision.is_none() || revisions.get(name) != Some(revision))
                    .then(|| name.clone())
            }).collect::<Vec<_>>();
            *previous = revisions;
            revoked
        };
        *self.containers.write().expect("toolpkg container runtime lock poisoned") = containers;
        *self.subpackageByPackageName.write().expect("toolpkg subpackage runtime lock poisoned") = subpackageByPackageName;
        for name in revoked {
            self.revokeContainerExecutionContexts(&name);
        }
    }

    /// Fingerprints source content at catalog boundaries, never on the invocation hot path.
    /// Embedded assets are immutable for this process. Directory contents must be checked:
    /// replacing a child can leave both the manifest version and parent mtime unchanged.
    #[allow(non_snake_case)]
    fn runtimeRevision(&self, runtime: &ToolPkgContainerRuntime) -> Option<u64> {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        let mut metadata = serde_json::to_value(runtime).ok()?;
        // Localized text uses HashMap; fresh scans must hash identical content identically.
        metadata.sort_all_objects();
        serde_json::to_vec(&metadata).ok()?.hash(&mut hash);
        if runtime.sourceType != ToolPkgSourceType::ASSET {
            self.hashSource(&runtime.sourcePath, 0, &mut hash)?;
        }
        Some(hash.finish())
    }

    #[allow(non_snake_case)]
    fn hashSource(&self, path: &str, depth: usize, hash: &mut impl Hasher) -> Option<()> {
        // Fail closed on pathological/symlink directory trees rather than recurse forever.
        if depth > 64 { return None; }
        let info = self.fileSystemHost.fileExists(path).ok()?;
        if !info.exists { return None; }
        path.hash(hash);
        if info.isDirectory {
            let mut entries = self.fileSystemHost.listFiles(path).ok()?;
            entries.sort_by(|a, b| a.name.cmp(&b.name));
            for entry in entries {
                if entry.name == "." || entry.name == ".." { continue; }
                self.hashSource(&joinToolPkgSourcePath(path, &entry.name), depth + 1, hash)?;
            }
        } else {
            self.fileSystemHost.readFileBytes(path).ok()?.hash(hash);
        }
        Some(())
    }

    /// Checks one container using the same direct-or-subpackage rule as the active catalog.
    #[allow(non_snake_case)]
    pub fn isEnabledToolPkgContainer(
        &self,
        packageName: &str,
        enabledPackageNames: &[String],
    ) -> bool {
        let containers = self.containers.read().expect("toolpkg container runtime lock poisoned");
        let Some(runtime) = containers.get(packageName.trim()) else {
            return false;
        };
        enabledPackageNames.iter().any(|enabled| {
            enabled == &runtime.packageName
                || runtime.subpackages.iter().any(|subpackage| enabled == &subpackage.packageName)
        })
    }

    /// Returns the ToolPkg containers enabled directly or through a subpackage.
    #[allow(non_snake_case)]
    pub fn getEnabledToolPkgContainerRuntimes(
        &self,
        enabledPackageNames: &[String],
    ) -> Vec<ToolPkgContainerRuntime> {
        let enabledPackageNames = BTreeSet::from_iter(enabledPackageNames.iter().cloned());
        let mut runtimes = self
            .containers
            .read()
            .expect("toolpkg container runtime lock poisoned")
            .values()
            .filter(|runtime| {
                enabledPackageNames.contains(&runtime.packageName)
                    || runtime
                        .subpackages
                        .iter()
                        .any(|subpackage| enabledPackageNames.contains(&subpackage.packageName))
            })
            .cloned()
            .collect::<Vec<_>>();
        runtimes.sort_by(|left, right| left.packageName.cmp(&right.packageName));
        runtimes
    }

    /// Registers a listener and immediately sends the current active containers.
    #[allow(non_snake_case)]
    pub fn addToolPkgRuntimeChangeListener(
        &self,
        listener: ToolPkgRuntimeChangeListener,
        activeContainers: Vec<ToolPkgContainerRuntime>,
    ) {
        {
            let mut listeners = self
                .runtimeChangeListeners
                .lock()
                .expect("toolpkg runtime listener mutex poisoned");
            listeners.push(listener.clone());
        }
        listener(activeContainers);
    }

    /// Notifies every registered runtime change listener.
    #[allow(non_snake_case)]
    pub fn notifyToolPkgRuntimeChangeListeners(
        &self,
        activeContainers: Vec<ToolPkgContainerRuntime>,
    ) {
        let listeners = self
            .runtimeChangeListeners
            .lock()
            .expect("toolpkg runtime listener mutex poisoned")
            .clone();
        for listener in listeners {
            listener(activeContainers.clone());
        }
    }

    /// Returns the main script of an enabled ToolPkg container.
    #[allow(non_snake_case)]
    pub fn getToolPkgMainScriptInternal(
        &self,
        containerPackageName: &str,
        enabledPackageNames: &[String],
    ) -> Option<String> {
        let normalizedContainerPackageName = containerPackageName.trim();
        let runtime = self.getToolPkgContainerRuntime(normalizedContainerPackageName)?;
        let enabledPackageNames = BTreeSet::from_iter(enabledPackageNames.iter().cloned());
        let enabled = runtime.packageName.eq(normalizedContainerPackageName)
            && enabledPackageNames.contains(&runtime.packageName)
            || runtime
                .subpackages
                .iter()
                .any(|subpackage| enabledPackageNames.contains(&subpackage.packageName));
        if !enabled || runtime.mainEntry.trim().is_empty() {
            return None;
        }
        self.readToolPkgMainScript(&runtime)
    }

    /// Returns the main script of one registered ToolPkg container for runtime IPC dispatch.
    #[allow(non_snake_case)]
    pub fn getRegisteredToolPkgMainScript(&self, containerPackageName: &str) -> Option<String> {
        let runtime = self.getToolPkgContainerRuntime(containerPackageName)?;
        if runtime.mainEntry.trim().is_empty() {
            return None;
        }
        self.readToolPkgMainScript(&runtime)
    }

    /// Reads a text resource from an enabled ToolPkg container or subpackage.
    #[allow(non_snake_case)]
    pub fn readToolPkgTextResource(
        &self,
        packageNameOrSubpackageId: &str,
        resourcePath: &str,
        enabledPackageNames: &[String],
    ) -> Option<String> {
        let normalizedPackageName = packageNameOrSubpackageId.trim();
        let enabledPackageNames = BTreeSet::from_iter(enabledPackageNames.iter().cloned());
        let runtime = self
            .getToolPkgContainerRuntime(normalizedPackageName)
            .or_else(|| {
                let subpackage =
                    self.resolveToolPkgSubpackageRuntimeInternal(normalizedPackageName)?;
                self.getToolPkgContainerRuntime(&subpackage.containerPackageName)
            })?;
        let enabled = enabledPackageNames.contains(&runtime.packageName)
            || runtime
                .subpackages
                .iter()
                .any(|subpackage| enabledPackageNames.contains(&subpackage.packageName));
        if !enabled {
            return None;
        }
        self.readToolPkgResourceText(&runtime, resourcePath)
    }

    /// Returns a cached execution engine or creates one with explicit container ownership.
    #[allow(non_snake_case)]
    pub fn getToolPkgExecutionEngine(
        &self,
        contextKey: &str,
        containerPackageName: &str,
    ) -> Result<Arc<dyn JsExecutionEngine>, String> {
        self.executionEngineWithLease(contextKey, containerPackageName, false, false)
    }

    /// Acquires one counted lease for a ToolPkg JavaScript execution context.
    #[allow(non_snake_case)]
    pub fn acquireToolPkgExecutionEngine(&self, contextKey: &str, containerPackageName: &str) -> Result<(), String> {
        self.executionEngineWithLease(contextKey, containerPackageName, true, false).map(|_| ())
    }

    /// Acquires an execution lease whose lifetime is coupled to the caller's asynchronous work.
    #[allow(non_snake_case)]
    pub fn leaseToolPkgExecutionEngine(
        &self,
        contextKey: &str,
        containerPackageName: &str,
    ) -> Result<(Arc<dyn JsExecutionEngine>, ToolPkgExecutionEngineLease), String> {
        self.leaseExecutionEngine(contextKey, containerPackageName, false)
    }

    /// Main hooks, public APIs and package calls share the container-owned main realm.
    #[allow(non_snake_case)]
    pub fn leaseToolPkgMainExecutionEngine(
        &self, containerPackageName: &str,
    ) -> Result<(Arc<dyn JsExecutionEngine>, ToolPkgExecutionEngineLease), String> {
        let contextKey = format!("toolpkg_main:{}", containerPackageName.trim());
        self.leaseExecutionEngine(&contextKey, containerPackageName, true)
    }

    #[allow(non_snake_case)]
    fn leaseExecutionEngine(
        &self, contextKey: &str, containerPackageName: &str, containerOwned: bool,
    ) -> Result<(Arc<dyn JsExecutionEngine>, ToolPkgExecutionEngineLease), String> {
        let engine = self.executionEngineWithLease(contextKey, containerPackageName, true, containerOwned)?;
        Ok((
            engine.clone(),
            ToolPkgExecutionEngineLease {
                manager: self.clone(),
                contextKey: contextKey.trim().to_string(),
                containerPackageName: containerPackageName.trim().to_string(),
                engine,
            },
        ))
    }

    /// Starts workers without holding the registry lock. Startup authenticates via
    /// the package manager; concurrent release holds that manager while consulting
    /// this registry. Holding both across startup creates a lock inversion.
    #[allow(non_snake_case)]
    fn executionEngineWithLease(
        &self,
        contextKey: &str,
        containerPackageName: &str,
        acquireLease: bool,
        containerOwned: bool,
    ) -> Result<Arc<dyn JsExecutionEngine>, String> {
        let (existing, generation) = self.cachedExecutionEngine(contextKey, containerPackageName, acquireLease, containerOwned)?;
        if let Some(engine) = existing { return Ok(engine); }
        let context = self.executionContext(contextKey, containerPackageName)?;
        let created = self.executionEngineFactory.createToolPkgExecutionEngine(context)?;
        self.publishExecutionEngine(contextKey, containerPackageName, acquireLease, containerOwned, generation, created)
    }

    /// Acquires a counted page lease while awaiting a prepared engine outside registry locks.
    #[allow(non_snake_case)]
    pub async fn acquireToolPkgExecutionEngineAsync(
        &self, contextKey: &str, containerPackageName: &str,
    ) -> Result<(), String> {
        let (existing, generation) = self.cachedExecutionEngine(contextKey, containerPackageName, true, false)?;
        if existing.is_some() { return Ok(()); }
        let context = self.executionContext(contextKey, containerPackageName)?;
        let created = self.executionEngineFactory.acquireToolPkgExecutionEngine(context).await?;
        self.publishExecutionEngine(contextKey, containerPackageName, true, false, generation, created).map(|_| ())
    }

    /// Validates an explicit owner and counts a lease only for an existing matching entry.
    #[allow(non_snake_case)]
    fn cachedExecutionEngine(
        &self, contextKey: &str, containerPackageName: &str, acquireLease: bool, containerOwned: bool,
    ) -> Result<(Option<Arc<dyn JsExecutionEngine>>, ExecutionEngineGeneration), String> {
        let normalizedKey = contextKey.trim();
        let normalizedContainer = containerPackageName.trim();
        if normalizedKey.is_empty() { return Err("ToolPkg execution context key is required".into()); }
        if normalizedContainer.is_empty() { return Err("ToolPkg execution container is required".into()); }
        {
            let mut engines = self.toolPkgExecutionEngines.lock()
                .expect("toolpkg execution engine mutex poisoned");
            if engines.closed { return Err("ToolPkg execution owner has shut down".into()); }
            let packageGeneration = *engines.packageGenerations.entry(normalizedContainer.to_string()).or_insert(0);
            let generation = (engines.factoryGeneration, packageGeneration);
            if let Some(entry) = engines.entries.get_mut(normalizedKey) {
                if entry.containerPackageName != normalizedContainer { return Err("ToolPkg execution context belongs to a different container".into()); }
                if acquireLease { entry.activeLeases += 1; }
                entry.containerOwned |= containerOwned;
                return Ok((Some(entry.engine.clone()), generation));
            }
            Ok((None, generation))
        }
    }

    /// Captures the registered package identity and resource owner for a new execution context.
    #[allow(non_snake_case)]
    fn executionContext(&self, contextKey: &str, containerPackageName: &str) -> Result<ToolPkgExecutionContext, String> {
        let normalizedKey = contextKey.trim();
        let normalizedContainer = containerPackageName.trim();
        let apiVersion = self.containers.read().expect("toolpkg container registry poisoned")
            .get(normalizedContainer)
            .ok_or_else(|| format!("ToolPkg execution context requires a registered container: {normalizedContainer}"))?
            .apiVersion.clone();
        Ok(ToolPkgExecutionContext {
            context_key: normalizedKey.to_string(), container_package_name: normalizedContainer.to_string(),
            api_version: apiVersion, text_resource_host: Arc::new(self.clone()),
        })
    }

    /// Publishes a freshly bound engine and destroys an unused racing creator outside locks.
    #[allow(non_snake_case)]
    fn publishExecutionEngine(
        &self, contextKey: &str, containerPackageName: &str, acquireLease: bool, containerOwned: bool,
        generation: ExecutionEngineGeneration, created: Arc<dyn JsExecutionEngine>,
    ) -> Result<Arc<dyn JsExecutionEngine>, String> {
        let normalizedKey = contextKey.trim();
        let normalizedContainer = containerPackageName.trim();
        let winner = {
            let mut engines = self.toolPkgExecutionEngines.lock()
                .expect("toolpkg execution engine mutex poisoned");
            let packageGeneration = *engines.packageGenerations.get(normalizedContainer)
                .expect("Acquisition must capture a package generation");
            if generation != (engines.factoryGeneration, packageGeneration) {
                Err("ToolPkg execution context was revoked during engine acquisition".to_string())
            } else if let Some(entry) = engines.entries.get_mut(normalizedKey) {
                if entry.containerPackageName != normalizedContainer {
                    Err("ToolPkg execution context belongs to a different container".to_string())
                } else {
                    if acquireLease { entry.activeLeases += 1; }
                    entry.containerOwned |= containerOwned;
                    Ok(Some(entry.engine.clone()))
                }
            } else {
                engines.entries.insert(normalizedKey.to_string(), ToolPkgExecutionEngineEntry {
                    containerPackageName: normalizedContainer.to_string(),
                    engine: created.clone(), activeLeases: usize::from(acquireLease),
                    containerOwned: containerOwned || !acquireLease,
                });
                Ok(None)
            }
        };
        let winner = match winner {
            Ok(winner) => winner,
            Err(error) => { created.destroy(); return Err(error); }
        };
        if let Some(winner) = winner {
            // A racing creator won. Dispose the unused worker outside the lock too.
            created.destroy();
            Ok(winner)
        } else {
            Ok(created)
        }
    }

    /// Finds a cached ToolPkg execution engine without creating one.
    #[allow(non_snake_case)]
    pub fn findToolPkgExecutionEngine(
        &self,
        contextKey: &str,
        containerPackageName: &str,
    ) -> Option<Arc<dyn JsExecutionEngine>> {
        let normalizedKey = contextKey.trim();
        let normalizedContainer = containerPackageName.trim();
        if normalizedKey.is_empty() || normalizedContainer.is_empty() {
            return None;
        }
        let engines = self
            .toolPkgExecutionEngines
            .lock()
            .expect("toolpkg execution engine mutex poisoned");
        let entry = engines.entries.get(normalizedKey)?;
        assert_eq!(
            entry.containerPackageName, normalizedContainer,
            "ToolPkg execution context belongs to a different container"
        );
        Some(entry.engine.clone())
    }

    /// Releases one counted lease for a ToolPkg JavaScript execution context.
    #[allow(non_snake_case)]
    pub fn releaseToolPkgExecutionEngine(&self, contextKey: &str, containerPackageName: &str) {
        self.releaseExecutionEngineLease(contextKey, containerPackageName, None);
    }

    /// An old in-flight call must not decrement a replacement realm's lease count.
    #[allow(non_snake_case)]
    fn releaseExecutionEngineLease(
        &self, contextKey: &str, containerPackageName: &str,
        expectedEngine: Option<&Arc<dyn JsExecutionEngine>>,
    ) {
        let normalizedKey = contextKey.trim();
        let normalizedContainer = containerPackageName.trim();
        if normalizedKey.is_empty() || normalizedContainer.is_empty() { return; }
        let removed = {
            let mut engines = self
                .toolPkgExecutionEngines
                .lock()
                .expect("toolpkg execution engine mutex poisoned");
            let Some(entry) = engines.entries.get_mut(normalizedKey) else {
                return;
            };
            if expectedEngine.is_some_and(|expected| !Arc::ptr_eq(expected, &entry.engine)) {
                return;
            }
            assert_eq!(
                entry.containerPackageName, normalizedContainer,
                "ToolPkg execution context belongs to a different container"
            );
            assert!(
                entry.activeLeases > 0,
                "ToolPkg execution context has no active lease"
            );
            entry.activeLeases -= 1;
            if entry.activeLeases == 0 && !entry.containerOwned {
                engines.entries.remove(normalizedKey)
            } else {
                None
            }
        };
        if let Some(entry) = removed {
            entry.engine.destroy();
        }
    }

    /// Revokes every context, including leased contexts, when a container is disabled or removed.
    #[allow(non_snake_case)]
    pub fn revokeContainerExecutionContexts(&self, containerPackageName: &str) {
        let normalizedContainer = containerPackageName.trim();
        if normalizedContainer.is_empty() {
            return;
        }
        self.removeExecutionEnginesMatching(
            |entry| entry.containerPackageName == normalizedContainer,
            |registry| {
                registry.mainScripts.remove(normalizedContainer);
                *registry.packageGenerations.entry(normalizedContainer.to_string()).or_insert(0) += 1;
            },
        );
    }

    /// Removes selected registry entries under the lock and destroys their workers after unlocking.
    #[allow(non_snake_case)]
    fn removeExecutionEnginesMatching(
        &self,
        shouldRemove: impl Fn(&ToolPkgExecutionEngineEntry) -> bool,
        revoke: impl FnOnce(&mut ToolPkgExecutionEngineRegistry),
    ) {
        let removed = {
            let mut engines = self
                .toolPkgExecutionEngines
                .lock()
                .expect("toolpkg execution engine mutex poisoned");
            revoke(&mut engines);
            let keys = engines.entries
                .iter()
                .filter(|(_, entry)| shouldRemove(entry))
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>();
            keys.into_iter()
                .map(|key| engines.entries.remove(&key).expect("selected engine must remain registered while locked"))
                .collect::<Vec<_>>()
        };
        for entry in removed {
            entry.engine.destroy();
        }
    }

    /// Shares the main source for one catalog generation and rejects publication after revocation.
    #[allow(non_snake_case)]
    fn readToolPkgMainScript(&self, runtime: &ToolPkgContainerRuntime) -> Option<String> {
        let containers = self.containers.read().expect("toolpkg container runtime lock poisoned");
        let registered = containers.get(&runtime.packageName)?;
        if registered.sourceType != runtime.sourceType || registered.sourcePath != runtime.sourcePath
            || registered.mainEntry != runtime.mainEntry {
            return None;
        }
        let generation = {
            let mut registry = self.toolPkgExecutionEngines.lock().expect("toolpkg execution engine mutex poisoned");
            if registry.closed { return None; }
            if let Some(script) = registry.mainScripts.get(&runtime.packageName) {
                return Some(script.clone());
            }
            *registry.packageGenerations.entry(runtime.packageName.clone()).or_insert(0)
        };
        let script = self.readToolPkgResourceText(runtime, &runtime.mainEntry)?;
        let mut registry = self.toolPkgExecutionEngines.lock().expect("toolpkg execution engine mutex poisoned");
        if registry.closed || registry.packageGenerations.get(&runtime.packageName) != Some(&generation) {
            return None;
        }
        Some(registry.mainScripts.entry(runtime.packageName.clone()).or_insert(script).clone())
    }

    /// Reads one ToolPkg resource as UTF-8 text.
    #[allow(non_snake_case)]
    fn readToolPkgResourceText(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resourcePath: &str,
    ) -> Option<String> {
        let bytes = self.readToolPkgResourceBytes(runtime, resourcePath)?;
        crate::toolpkg::ToolPkgProtection::decodeUtf8(&bytes).ok()
    }

    /// Reads one ToolPkg resource as raw bytes.
    #[allow(non_snake_case)]
    pub fn readToolPkgResourceBytes(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resourcePath: &str,
    ) -> Option<Vec<u8>> {
        let normalizedResourcePath = normalizeToolPkgEntryPath(resourcePath)?;
        match runtime.sourceType {
            ToolPkgSourceType::EXTERNAL | ToolPkgSourceType::MARKET => {
                let sourceInfo = self.fileSystemHost.fileExists(&runtime.sourcePath).ok()?;
                if sourceInfo.isDirectory {
                    let resourceFile =
                        joinToolPkgSourcePath(&runtime.sourcePath, &normalizedResourcePath);
                    let bytes = self.fileSystemHost.readFileBytes(&resourceFile).ok()?;
                    return crate::toolpkg::ToolPkgProtection::decryptIfNeeded(&bytes).ok();
                }
                if sourceInfo.exists
                    && runtime
                        .sourcePath
                        .to_ascii_lowercase()
                        .ends_with(".toolpkg")
                {
                    let archiveBytes = self
                        .fileSystemHost
                        .readFileBytes(&runtime.sourcePath)
                        .ok()?;
                    let mut archive =
                        zip::ZipArchive::new(std::io::Cursor::new(archiveBytes)).ok()?;
                    let mut entry = archive.by_name(&normalizedResourcePath).ok()?;
                    let mut bytes = Vec::new();
                    entry.read_to_end(&mut bytes).ok()?;
                    return crate::toolpkg::ToolPkgProtection::decryptIfNeeded(&bytes).ok();
                }
                None
            }
            ToolPkgSourceType::ASSET => {
                let assetBytes = self.assetSource.toolPkgAssetBytes(&runtime.sourcePath)?;
                let cursor = std::io::Cursor::new(assetBytes);
                let mut archive = zip::ZipArchive::new(cursor).ok()?;
                let mut entry = archive.by_name(&normalizedResourcePath).ok()?;
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).ok()?;
                crate::toolpkg::ToolPkgProtection::decryptIfNeeded(&bytes).ok()
            }
        }
    }
}

impl ToolPkgTextResourceHost for ToolPkgManager {
    /// Reads one UTF-8 resource through the registered ToolPkg source host.
    fn read_toolpkg_text_resource(
        &self,
        package_name_or_subpackage_id: &str,
        resource_path: &str,
    ) -> Result<Option<String>, String> {
        let normalizedPackageName = package_name_or_subpackage_id.trim();
        let runtime = self
            .getToolPkgContainerRuntime(normalizedPackageName)
            .or_else(|| {
                let subpackage =
                    self.resolveToolPkgSubpackageRuntimeInternal(normalizedPackageName)?;
                self.getToolPkgContainerRuntime(&subpackage.containerPackageName)
            })
            .ok_or_else(|| format!("ToolPkg container not found: {normalizedPackageName}"))?;
        Ok(self.readToolPkgResourceText(&runtime, resource_path))
    }
}

/// Joins a normalized ToolPkg entry beneath one host-owned source directory.
fn joinToolPkgSourcePath(sourcePath: &str, entryPath: &str) -> String {
    format!("{}/{}", sourcePath.trim_end_matches(['/', '\\']), entryPath)
}

impl ToolPkgHookDispatcher for ToolPkgManager {
    /// Preserves typed failures and applies a deadline only to an explicitly timed before hook.
    #[allow(non_snake_case)]
    fn dispatchToolPkgHook(
        &self,
        enabledPackageNames: &[String],
        invocation: ToolPkgHookInvocation,
    ) -> crate::javascript::JsExecutionCompletion<
        crate::execution_result::JsExecutionResult<Option<String>>,
    > {
        let prepared = self.prepareToolPkgHook(enabledPackageNames, &invocation);
        Box::pin(async move {
            let (engine, script, params, _lease) =
                prepared.map_err(crate::execution_result::JsExecutionError::invalid_request)?;
            match invocation.timeoutMillis {
                Some(timeout) => {
                    engine
                        .execute_script_function_with_timeout_millis(
                            &script,
                            &invocation.functionName,
                            &params,
                            &invocation.envOverrides,
                            invocation.onIntermediateResult,
                            invocation.dispatchIntermediateOnMain,
                            timeout,
                        )
                        .await
                }
                None => {
                    engine
                        .execute_script_function(
                            &script,
                            &invocation.functionName,
                            &params,
                            &invocation.envOverrides,
                            invocation.onIntermediateResult,
                            invocation.dispatchIntermediateOnMain,
                        )
                        .await
                }
            }
        })
    }
}

impl ToolPkgManager {
    /// Uses the same typed lifecycle contract for asynchronous hook dispatch.
    #[allow(non_snake_case)]
    pub async fn dispatchToolPkgHookAsync(
        &self,
        enabledPackageNames: &[String],
        invocation: ToolPkgHookInvocation,
    ) -> crate::execution_result::JsExecutionResult<Option<String>> {
        self.dispatchToolPkgHook(enabledPackageNames, invocation)
            .await
    }

    /// Resolves the engine, script, and event parameters shared by hook execution APIs.
    #[allow(non_snake_case)]
    fn prepareToolPkgHook(
        &self,
        enabledPackageNames: &[String],
        invocation: &ToolPkgHookInvocation,
    ) -> Result<(
        Arc<dyn JsExecutionEngine>,
        String,
        BTreeMap<String, Value>,
        ToolPkgExecutionEngineLease,
    ), String> {
        let containerPackageName = invocation.containerPackageName.trim();
        let runtime = self
            .getToolPkgContainerRuntime(containerPackageName)
            .ok_or_else(|| format!("ToolPkg container not found: {containerPackageName}"))?;
        let script = self
            .getToolPkgMainScriptInternal(&runtime.packageName, enabledPackageNames)
            .ok_or_else(|| {
                format!(
                    "ToolPkg main script is unavailable: {}",
                    runtime.packageName
                )
            })?;

        let eventName = invocation
            .eventName
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(invocation.event.as_str());
        let mut params = BTreeMap::<String, Value>::new();
        params.insert("event".to_string(), Value::String(eventName.to_string()));
        params.insert(
            "eventName".to_string(),
            Value::String(eventName.to_string()),
        );
        params.insert("eventPayload".to_string(), invocation.eventPayload.clone());
        params.insert(
            "timestampMs".to_string(),
            Value::Number(serde_json::Number::from(invocation.timestampMs)),
        );
        params.insert(
            "functionName".to_string(),
            Value::String(invocation.functionName.clone()),
        );
        params.insert(
            "toolPkgId".to_string(),
            Value::String(runtime.packageName.clone()),
        );
        params.insert(
            "containerPackageName".to_string(),
            Value::String(runtime.packageName.clone()),
        );
        params.insert(
            "__operit_toolpkg_api_version".to_string(),
            Value::String(runtime.apiVersion.clone()),
        );
        params.insert(
            "__operit_ui_package_name".to_string(),
            Value::String(runtime.packageName.clone()),
        );
        params.insert(
            "__operit_script_screen".to_string(),
            Value::String(runtime.mainEntry.clone()),
        );
        if let Some(pluginId) = invocation
            .pluginId
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            params.insert("pluginId".to_string(), Value::String(pluginId.to_string()));
        }
        if let Some(chatId) = invocation
            .eventPayload
            .get("chatId")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            params.insert(
                "__operit_package_chat_id".to_string(),
                Value::String(chatId.to_string()),
            );
        }
        if let Some(functionSource) = invocation
            .inlineFunctionSource
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            params.insert(
                "__operit_inline_function_name".to_string(),
                Value::String(invocation.functionName.clone()),
            );
            params.insert(
                "__operit_inline_function_source".to_string(),
                Value::String(functionSource.to_string()),
            );
        }
        if let Some(contextKey) = invocation
            .executionContextKey
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            params.insert(
                "__operit_execution_context_key".to_string(),
                Value::String(contextKey.to_string()),
            );
        }
        if let Some(runtimeKind) = invocation
            .runtimeKind
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            params.insert(
                "__operit_toolpkg_runtime_kind".to_string(),
                Value::String(runtimeKind.to_ascii_lowercase()),
            );
        }

        let contextKey = resolveToolPkgExecutionContextKey(&runtime.packageName, &params);
        let (engine, lease) = if contextKey == format!("toolpkg_main:{}", runtime.packageName) {
            self.leaseToolPkgMainExecutionEngine(&runtime.packageName)?
        } else {
            self.leaseToolPkgExecutionEngine(&contextKey, &runtime.packageName)?
        };
        Ok((engine, script, params, lease))
    }
}

/// Resolves the execution context key encoded in hook parameters.
#[allow(non_snake_case)]
fn resolveToolPkgExecutionContextKey(
    containerPackageName: &str,
    params: &BTreeMap<String, Value>,
) -> String {
    params
        .get("__operit_execution_context_key")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("toolpkg_main:{containerPackageName}"))
}

/// Normalizes a ToolPkg entry path and rejects parent-directory traversal.
#[allow(non_snake_case)]
fn normalizeToolPkgEntryPath(rawPath: &str) -> Option<String> {
    let normalized = rawPath
        .trim()
        .replace('\\', "/")
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect::<Vec<_>>()
        .join("/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.split('/').any(|segment| segment == "..")
    {
        return None;
    }
    Some(normalized)
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

    use std::io::Write;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;

    use super::*;
    use crate::execution_result::JsExecutionResult;
    use crate::javascript::ToolPkgMainRegistrationCapture;
    use operit_host_api::{
        FileEntry, FileExistence, FileInfo, FindFilesRequest, GrepCodeRequest, GrepCodeResult,
        HostEnvironmentDescriptor, HostError, HostResult,
    };

    /// Rejects filesystem operations because these engine lifecycle tests do not access files.
    #[derive(Default)]
    struct TestFileSystemHost {
        files: Mutex<BTreeMap<String, Vec<u8>>>,
        byteReadCount: AtomicUsize,
    }

    impl TestFileSystemHost {
        /// Returns the explicit error used for unsupported test filesystem operations.
        fn unsupported<T>() -> HostResult<T> {
            Err(HostError::new("filesystem access is not used by this test"))
        }
    }

    impl FileSystemHost for TestFileSystemHost {
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
        fn listFiles(&self, path: &str) -> HostResult<Vec<FileEntry>> {
            let prefix = format!("{}/", path.trim_end_matches('/'));
            Ok(self.files.lock().unwrap().iter().filter_map(|(name, bytes)| {
                let child = name.strip_prefix(&prefix)?;
                (!child.contains('/')).then(|| FileEntry {
                    name: child.to_string(), isDirectory: false, size: bytes.len() as i64,
                    permissions: String::new(), lastModified: "unchanged".into(),
                })
            }).collect())
        }

        /// Rejects text reads.
        fn readFile(&self, _path: &str) -> HostResult<String> {
            Self::unsupported()
        }

        /// Rejects bounded text reads.
        fn readFileWithLimit(&self, _path: &str, _maxBytes: usize) -> HostResult<String> {
            Self::unsupported()
        }

        /// Counts and returns the explicitly registered test bytes.
        fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
            self.byteReadCount.fetch_add(1, Ordering::Relaxed);
            self.files.lock().unwrap().get(path).cloned()
                .ok_or_else(|| HostError::new("file missing"))
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
        fn fileExists(&self, path: &str) -> HostResult<FileExistence> {
            let files = self.files.lock().unwrap();
            if let Some(bytes) = files.get(path) {
                return Ok(FileExistence { exists: true, isDirectory: false, size: bytes.len() as i64 });
            }
            let prefix = format!("{}/", path.trim_end_matches('/'));
            Ok(FileExistence { exists: files.keys().any(|key| key.starts_with(&prefix)), isDirectory: true, size: 0 })
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

    /// Records whether a test execution engine was destroyed.
    #[derive(Default)]
    struct RecordingExecutionEngine {
        destroyed: AtomicBool,
        asyncHookParams: Mutex<Option<BTreeMap<String, Value>>>,
    }

    impl JsExecutionEngine for RecordingExecutionEngine {
        /// Records hook parameters and yields once through the canonical host execution API.
        fn execute_script_function(
            &self,
            _script: &str,
            _function_name: &str,
            params: &BTreeMap<String, Value>,
            _env_overrides: &BTreeMap<String, String>,
            _on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
            _dispatch_intermediate_on_main: bool,
        ) -> crate::javascript::JsExecutionCompletion<JsExecutionResult<Option<String>>> {
            *self
                .asyncHookParams
                .lock()
                .expect("async hook params mutex poisoned") = Some(params.clone());
            let mut yielded = false;
            Box::pin(std::future::poll_fn(move |context| {
                if yielded {
                    return std::task::Poll::Ready(Ok(Some("hook completed".to_string())));
                }
                yielded = true;
                context.waker().wake_by_ref();
                std::task::Poll::Pending
            }))
        }

        /// Returns no script result for exact-deadline registry tests.
        fn execute_script_function_with_timeout_millis(
            &self,
            _script: &str,
            _function_name: &str,
            _params: &BTreeMap<String, Value>,
            _env_overrides: &BTreeMap<String, String>,
            _on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
            _dispatch_intermediate_on_main: bool,
            _timeout_millis: u64,
        ) -> crate::javascript::JsExecutionCompletion<JsExecutionResult<Option<String>>> {
            let result = (|| Ok(None))();
            Box::pin(std::future::ready(result))
        }

        /// Records hook parameters and yields once before completing asynchronous execution.
        fn execute_script_function_async(
            &self,
            _script: String,
            _function_name: String,
            params: BTreeMap<String, Value>,
            _env_overrides: BTreeMap<String, String>,
            _on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
            _dispatch_intermediate_on_main: bool,
        ) -> crate::javascript::JsExecutionFuture<JsExecutionResult<Option<String>>> {
            self.execute_script_function(
                &_script,
                &_function_name,
                &params,
                &_env_overrides,
                _on_intermediate_result,
                _dispatch_intermediate_on_main,
            )
        }

        /// Returns an empty ToolPkg registration capture for registry tests.
        fn execute_toolpkg_main_registration_function_with_text_resources(
            &self,
            _script: &str,
            _function_name: &str,
            _params: &BTreeMap<String, Value>,
            _text_resources: Option<Arc<BTreeMap<String, String>>>,
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

        /// Returns no Compose DSL action result asynchronously for registry tests.
        fn dispatch_compose_dsl_action_stream_async(
            &self,
            _action_id: String,
            _payload: Option<Value>,
            _runtime_options: BTreeMap<String, Value>,
            _env_overrides: BTreeMap<String, String>,
            _on_intermediate_result: Arc<dyn Fn(operit_host_api::HostJavaScriptValue) + Send + Sync>,
        ) -> crate::javascript::JsExecutionFuture<JsExecutionResult<Option<Value>>> {
            Box::pin(async { Ok(None) })
        }

        /// Marks this test execution engine as destroyed.
        fn destroy(&self) {
            self.destroyed.store(true, Ordering::Release);
        }
    }

    /// Creates recording engines and retains them for lifecycle assertions.
    #[derive(Default)]
    struct RecordingExecutionEngineFactory {
        engines: Mutex<Vec<Arc<RecordingExecutionEngine>>>,
        contexts: Mutex<Vec<ToolPkgExecutionContext>>,
        onCreate: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
        creationError: Mutex<Option<String>>,
    }

    impl ToolPkgExecutionEngineFactory for RecordingExecutionEngineFactory {
        /// Creates one generic recording execution engine.
        #[allow(non_snake_case)]
        fn createExecutionEngine(&self) -> Result<Arc<dyn JsExecutionEngine>, String> {
            Ok(Arc::new(RecordingExecutionEngine::default()))
        }

        /// Creates one recording execution engine with its ToolPkg context.
        #[allow(non_snake_case)]
        fn createToolPkgExecutionEngine(
            &self,
            context: ToolPkgExecutionContext,
        ) -> Result<Arc<dyn JsExecutionEngine>, String> {
            if let Some(error) = self.creationError.lock().unwrap().clone() { return Err(error); }
            let callback = self.onCreate.lock().unwrap().clone();
            if let Some(callback) = callback { callback(); }
            let engine = Arc::new(RecordingExecutionEngine::default());
            self.contexts
                .lock()
                .expect("recording context factory mutex poisoned")
                .push(context);
            self.engines
                .lock()
                .expect("recording engine factory mutex poisoned")
                .push(engine.clone());
            Ok(engine)
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

    /// Supplies no assets because registry tests only exercise engine ownership.
    struct EmptyAssetSource;

    impl ToolPkgAssetSource for EmptyAssetSource {
        /// Returns no embedded ToolPkg archive.
        #[allow(non_snake_case)]
        fn toolPkgAssetBytes(&self, _assetName: &str) -> Option<Vec<u8>> {
            None
        }
    }

    /// Supplies a minimal ToolPkg archive containing main and shared JavaScript modules.
    struct SnapshotAssetSource;

    impl ToolPkgAssetSource for SnapshotAssetSource {
        /// Returns a ToolPkg archive used to verify main-hook resource snapshots.
        #[allow(non_snake_case)]
        fn toolPkgAssetBytes(&self, _assetName: &str) -> Option<Vec<u8>> {
            let mut archiveBytes = Vec::new();
            let mut archive = zip::ZipWriter::new(std::io::Cursor::new(&mut archiveBytes));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            archive
                .start_file("dist/main.js", options)
                .expect("snapshot test main entry should start");
            archive
                .write_all(b"exports.onInputMenuToggle = function() { return []; };")
                .expect("snapshot test main entry should be written");
            archive
                .start_file("dist/shared.js", options)
                .expect("snapshot test shared entry should start");
            archive
                .write_all(b"exports.createDefinition = function() { return {}; };")
                .expect("snapshot test shared entry should be written");
            archive
                .finish()
                .expect("snapshot test ToolPkg archive should finish");
            Some(archiveBytes)
        }
    }

    /// Creates one manager and its recording engine factory.
    fn recordingManager() -> (ToolPkgManager, Arc<RecordingExecutionEngineFactory>) {
        let factory = Arc::new(RecordingExecutionEngineFactory::default());
        let mut manager = ToolPkgManager::new(
            factory.clone(),
            Arc::new(EmptyAssetSource),
            Arc::new(TestFileSystemHost::default()),
        );
        // Engine ownership checks require genuine registered containers, not
        // fabricated engines for undeclared package IDs.
        for name in ["package_a", "package_b"] {
            manager.registerToolPkg(ToolPkgLoadResult {
                containerPackage: ToolPackage { name: name.to_string(), ..ToolPackage::default() },
                containerRuntime: ToolPkgContainerRuntime {
                    packageName: name.to_string(), mainEntry: "dist/main.js".to_string(),
                    ..ToolPkgContainerRuntime::default()
                },
                ..ToolPkgLoadResult::default()
            });
        }
        (manager, factory)
    }

    /// Keeps container activation consistent for hooks and execution owners when only a child is enabled.
    #[test]
    fn containerActivationIncludesEnabledSubpackageAndRejectsInactiveOwners() {
        let (mut manager, _) = recordingManager();
        let owner = "com.operit.daily_life";
        let child = "daily_life";
        manager.registerToolPkg(ToolPkgLoadResult {
            containerPackage: ToolPackage { name: owner.into(), ..ToolPackage::default() },
            containerRuntime: ToolPkgContainerRuntime {
                packageName: owner.into(), mainEntry: "dist/main.js".into(),
                subpackages: vec![ToolPkgSubpackageRuntime {
                    packageName: child.into(), containerPackageName: owner.into(),
                    subpackageId: child.into(), ..ToolPkgSubpackageRuntime::default()
                }],
                ..ToolPkgContainerRuntime::default()
            },
            ..ToolPkgLoadResult::default()
        });
        for enabled in [owner, child] {
            let active = manager.getEnabledToolPkgContainerRuntimes(&[enabled.into()]);
            assert_eq!(active.len(), 1);
            assert_eq!(active[0].packageName, owner);
        }
        assert!(manager.getEnabledToolPkgContainerRuntimes(&[]).is_empty());
        assert!(manager.getEnabledToolPkgContainerRuntimes(&["unregistered-child".into()]).is_empty());
    }

    /// Creates a manager whose ToolPkg archive exposes modules for resource snapshot assertions.
    fn snapshotRecordingManager() -> (ToolPkgManager, Arc<RecordingExecutionEngineFactory>) {
        let factory = Arc::new(RecordingExecutionEngineFactory::default());
        let manager = ToolPkgManager::new(
            factory.clone(),
            Arc::new(SnapshotAssetSource),
            Arc::new(TestFileSystemHost::default()),
        );
        (manager, factory)
    }

    /// Verifies an opaque context key cannot be reused under a different container owner.
    #[test]
    fn rejectsContextOwnershipMismatch() {
        let (manager, _) = recordingManager();
        manager.getToolPkgExecutionEngine("opaque-main-context", "package_a").unwrap();
        assert_eq!(manager.getToolPkgExecutionEngine("opaque-main-context", "package_b").err().unwrap(), "ToolPkg execution context belongs to a different container");
    }

    /// Verifies each acquired context lease keeps the engine alive until released.
    #[test]
    fn retainsExecutionEngineUntilEveryLeaseIsReleased() {
        let (manager, factory) = recordingManager();
        manager.acquireToolPkgExecutionEngine("shared-ui-context", "package_a");
        manager.acquireToolPkgExecutionEngine("shared-ui-context", "package_a");

        manager.releaseToolPkgExecutionEngine("shared-ui-context", "package_a");

        let engines = factory
            .engines
            .lock()
            .expect("recording engine factory mutex poisoned");
        assert_eq!(engines.len(), 1);
        assert!(!engines[0].destroyed.load(Ordering::Acquire));
        drop(engines);
        assert!(manager
            .findToolPkgExecutionEngine("shared-ui-context", "package_a")
            .is_some());

        manager.releaseToolPkgExecutionEngine("shared-ui-context", "package_a");

        let engines = factory
            .engines
            .lock()
            .expect("recording engine factory mutex poisoned");
        assert!(engines[0].destroyed.load(Ordering::Acquire));
        assert!(manager
            .findToolPkgExecutionEngine("shared-ui-context", "package_a")
            .is_none());
    }

    /// Regression: tab disposal can consult/release the registry while a new
    /// sidebar worker waits for package-manager authentication during startup.
    #[test]
    fn workerStartupDoesNotHoldExecutionRegistryLock() {
        let (manager, factory) = recordingManager();
        manager.acquireToolPkgExecutionEngine("old-sidebar", "package_a");
        let registry = manager.toolPkgExecutionEngines.clone();
        let duringStartup = manager.clone();
        *factory.onCreate.lock().unwrap() = Some(Arc::new(move || {
            assert!(registry.try_lock().is_ok(), "worker startup holds execution registry lock");
            duringStartup.releaseToolPkgExecutionEngine("old-sidebar", "package_a");
        }));
        manager.acquireToolPkgExecutionEngine("new-sidebar", "package_a");
        assert!(manager.findToolPkgExecutionEngine("old-sidebar", "package_a").is_none());
        assert!(manager.findToolPkgExecutionEngine("new-sidebar", "package_a").is_some());
        *factory.onCreate.lock().unwrap() = None;
        manager.releaseToolPkgExecutionEngine("new-sidebar", "package_a");
    }

    /// Racing creations reuse one winner, count both leases, and dispose the
    /// losing worker without holding the registry mutex.
    #[test]
    fn concurrentStartupKeepsBothLeasesAndDestroysUnusedWorker() {
        let (manager, factory) = recordingManager();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        *factory.onCreate.lock().unwrap() = Some(Arc::new(move || { barrier.wait(); }));
        std::thread::scope(|scope| {
            for _ in 0..2 {
                let manager = manager.clone();
                scope.spawn(move || manager.acquireToolPkgExecutionEngine("same-sidebar", "package_a"));
            }
        });
        *factory.onCreate.lock().unwrap() = None;
        let destroyed = || factory.engines.lock().unwrap().iter().filter(|engine| engine.destroyed.load(Ordering::Acquire)).count();
        assert_eq!(factory.engines.lock().unwrap().len(), 2);
        assert_eq!(destroyed(), 1);
        manager.releaseToolPkgExecutionEngine("same-sidebar", "package_a");
        assert!(manager.findToolPkgExecutionEngine("same-sidebar", "package_a").is_some());
        manager.releaseToolPkgExecutionEngine("same-sidebar", "package_a");
        assert_eq!(destroyed(), 2);
    }

    /// Replaces the current catalog through the same entry point used by package scans.
    fn rescanRuntimeCatalog(manager: &mut ToolPkgManager) {
        let containers = manager.containers.read().unwrap().clone();
        let subpackages = manager.subpackageByPackageName.read().unwrap().clone();
        manager.replaceRuntimeMaps(containers, subpackages);
    }

    /// Keeps both container-owned main realms and leased pages through unchanged catalog scans.
    #[test]
    fn catalogScanPreservesMainRealmsAndLeasedContexts() {
        let (manager, factory) = recordingManager();
        manager.acquireToolPkgExecutionEngine("package-a-sidebar", "package_a").unwrap();
        let sidebar = manager.findToolPkgExecutionEngine("package-a-sidebar", "package_a").unwrap();
        manager.getToolPkgExecutionEngine("package-a-main", "package_a").unwrap();
        manager.acquireToolPkgExecutionEngine("package-b-provider", "package_b").unwrap();
        let provider = manager.findToolPkgExecutionEngine("package-b-provider", "package_b").unwrap();
        manager.getToolPkgExecutionEngine("package-b-main", "package_b").unwrap();

        rescanRuntimeCatalog(&mut manager.clone());

        assert!(Arc::ptr_eq(&sidebar, &manager.findToolPkgExecutionEngine("package-a-sidebar", "package_a").unwrap()));
        assert!(Arc::ptr_eq(&provider, &manager.findToolPkgExecutionEngine("package-b-provider", "package_b").unwrap()));
        assert!(manager.findToolPkgExecutionEngine("package-a-main", "package_a").is_some());
        assert!(manager.findToolPkgExecutionEngine("package-b-main", "package_b").is_some());
        {
            let engines = factory.engines.lock().unwrap();
            assert!(!engines[0].destroyed.load(Ordering::Acquire));
            assert!(!engines[1].destroyed.load(Ordering::Acquire));
            assert!(!engines[2].destroyed.load(Ordering::Acquire));
            assert!(!engines[3].destroyed.load(Ordering::Acquire));
        }
        manager.releaseToolPkgExecutionEngine("package-a-sidebar", "package_a");
        manager.releaseToolPkgExecutionEngine("package-b-provider", "package_b");
        manager.revokeContainerExecutionContexts("package_a");
        manager.revokeContainerExecutionContexts("package_b");
        assert!(factory.engines.lock().unwrap().iter().all(|engine| engine.destroyed.load(Ordering::Acquire)));
    }

    /// Preserves a live sidebar when fresh scans deserialize identical multilingual metadata.
    #[test]
    fn multilingualCatalogRescanPreservesSidebarRealm() {
        let (mut manager, factory) = recordingManager();
        let mut containers = manager.containers.read().unwrap().clone();
        let runtime = containers.get_mut("package_a").unwrap();
        runtime.displayName = serde_json::from_value(serde_json::json!({
            "zh": "角色卡", "en": "Characters", "default": "Characters",
            "ja": "キャラクター", "fr": "Personnages", "de": "Charaktere"
        })).unwrap();
        runtime.description = runtime.displayName.clone();
        let snapshot = serde_json::to_value(&containers).unwrap();
        let subpackages = manager.subpackageByPackageName.read().unwrap().clone();
        manager.replaceRuntimeMaps(containers, subpackages.clone());
        manager.acquireToolPkgExecutionEngine("character-sidebar", "package_a").unwrap();
        let sidebar = manager.findToolPkgExecutionEngine("character-sidebar", "package_a").unwrap();
        let (_, mainLease) = manager.leaseToolPkgMainExecutionEngine("package_a").unwrap();
        drop(mainLease);
        for _ in 0..32 {
            let rescanned = serde_json::from_value(snapshot.clone()).unwrap();
            manager.replaceRuntimeMaps(rescanned, subpackages.clone());
            let current = manager.findToolPkgExecutionEngine("character-sidebar", "package_a")
                .expect("unchanged multilingual metadata must preserve the sidebar");
            assert!(Arc::ptr_eq(&sidebar, &current));
            assert!(factory.engines.lock().unwrap().iter()
                .all(|engine| !engine.destroyed.load(Ordering::Acquire)));
        }
        let mut changed: BTreeMap<String, ToolPkgContainerRuntime> =
            serde_json::from_value(snapshot).unwrap();
        changed.get_mut("package_a").unwrap().displayName.values
            .insert("en".into(), "Updated characters".into());
        manager.replaceRuntimeMaps(changed, subpackages);
        assert!(manager.findToolPkgExecutionEngine("character-sidebar", "package_a").is_none());
        assert!(factory.engines.lock().unwrap().iter()
            .all(|engine| engine.destroyed.load(Ordering::Acquire)));
    }

    /// Preserves engine identity and every counted lease across repeated catalog replacement.
    #[test]
    fn catalogReplacementKeepsLeasedContextUntilLastRelease() {
        let (mut manager, factory) = recordingManager();
        manager.acquireToolPkgExecutionEngine("shared-sidebar", "package_a").unwrap();
        manager.acquireToolPkgExecutionEngine("shared-sidebar", "package_a").unwrap();
        let engine = manager.findToolPkgExecutionEngine("shared-sidebar", "package_a").unwrap();
        let containers = manager.containers.read().unwrap().clone();
        let subpackages = manager.subpackageByPackageName.read().unwrap().clone();

        manager.replaceRuntimeMaps(containers, subpackages);
        rescanRuntimeCatalog(&mut manager);
        assert!(Arc::ptr_eq(&engine, &manager.findToolPkgExecutionEngine("shared-sidebar", "package_a").unwrap()));
        assert_eq!(factory.engines.lock().unwrap().len(), 1);

        manager.releaseToolPkgExecutionEngine("shared-sidebar", "package_a");
        rescanRuntimeCatalog(&mut manager);
        assert!(!factory.engines.lock().unwrap()[0].destroyed.load(Ordering::Acquire));
        assert!(manager.findToolPkgExecutionEngine("shared-sidebar", "package_a").is_some());

        manager.releaseToolPkgExecutionEngine("shared-sidebar", "package_a");
        assert!(factory.engines.lock().unwrap()[0].destroyed.load(Ordering::Acquire));
        assert!(manager.findToolPkgExecutionEngine("shared-sidebar", "package_a").is_none());
    }

    /// Revokes leased and cached contexts for one container without affecting other owners.
    #[test]
    fn containerRevocationIncludesLeasesAndPreservesOtherOwners() {
        let (manager, factory) = recordingManager();
        manager.getToolPkgExecutionEngine("package-a-main", "package_a");
        manager.acquireToolPkgExecutionEngine("package-a-xml-node", "package_a").unwrap();
        manager.getToolPkgExecutionEngine("package-b-provider", "package_b");

        manager.revokeContainerExecutionContexts("package_a");

        let engines = factory
            .engines
            .lock()
            .expect("recording engine factory mutex poisoned");
        assert!(engines[0].destroyed.load(Ordering::Acquire));
        assert!(engines[1].destroyed.load(Ordering::Acquire));
        assert!(!engines[2].destroyed.load(Ordering::Acquire));
        assert!(manager
            .findToolPkgExecutionEngine("package-a-main", "package_a")
            .is_none());
        assert!(manager
            .findToolPkgExecutionEngine("package-b-provider", "package_b")
            .is_some());
    }

    /// Verifies a cloned IPC runtime view remains usable while its external owner lock is held.
    #[test]
    fn clonedRuntimeViewDoesNotAcquireExternalOwnerLock() {
        let (mut manager, _) = recordingManager();
        manager.registerToolPkg(ToolPkgLoadResult {
            containerPackage: ToolPackage {
                name: "package_a".to_string(),
                ..ToolPackage::default()
            },
            containerRuntime: ToolPkgContainerRuntime {
                packageName: "package_a".to_string(),
                mainEntry: "dist/main.js".to_string(),
                ..ToolPkgContainerRuntime::default()
            },
            ..ToolPkgLoadResult::default()
        });
        let runtimeView = manager.clone();
        let externalOwner = Arc::new(Mutex::new(manager));
        let ownerGuard = externalOwner
            .lock()
            .expect("external ToolPkg owner mutex poisoned");
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let runtime = runtimeView.getToolPkgContainerRuntime("package_a");
            sender
                .send(runtime.map(|value| value.packageName))
                .expect("runtime view result receiver must remain connected");
        });

        assert_eq!(
            receiver
                .recv_timeout(Duration::from_secs(1))
                .expect("runtime view must not wait for the external owner lock"),
            Some("package_a".to_string())
        );
        drop(ownerGuard);
        worker.join().expect("runtime view worker must finish");
    }

    /// Verifies navigation hooks yield to the caller and preserve their event payload.
    #[test]
    fn asyncNavigationHookYieldsUntilHostExecutionCompletes() {
        let (mut manager, factory) = snapshotRecordingManager();
        manager.registerToolPkg(ToolPkgLoadResult {
            containerPackage: ToolPackage {
                name: "snapshot_package".to_string(),
                ..ToolPackage::default()
            },
            containerRuntime: ToolPkgContainerRuntime {
                packageName: "snapshot_package".to_string(),
                mainEntry: "dist/main.js".to_string(),
                sourceType: ToolPkgSourceType::ASSET,
                sourcePath: "snapshot-package.toolpkg".to_string(),
                ..ToolPkgContainerRuntime::default()
            },
            ..ToolPkgLoadResult::default()
        });
        let enabled = vec!["snapshot_package".to_string()];
        let payload = serde_json::json!({"entryId": "open_bing_with_action"});
        let mut execution = Box::pin(manager.dispatchToolPkgHookAsync(
            &enabled,
            ToolPkgHookInvocation {
                containerPackageName: "snapshot_package".to_string(),
                functionName: "openBingFromSidebar".to_string(),
                event: "toolpkg_navigation_entry_action".to_string(),
                eventName: Some("navigation_entry_action".to_string()),
                pluginId: Some("open_bing_with_action".to_string()),
                inlineFunctionSource: None,
                eventPayload: payload.clone(),
                executionContextKey: None,
                runtimeKind: None,
                envOverrides: BTreeMap::new(),
                timestampMs: 1,
                timeoutMillis: None,
                dispatchIntermediateOnMain: true,
                onIntermediateResult: None,
            },
        ));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(std::future::Future::poll(execution.as_mut(), &mut context).is_pending());
        assert_eq!(
            std::future::Future::poll(execution.as_mut(), &mut context),
            std::task::Poll::Ready(Ok(Some("hook completed".to_string())))
        );
        let engines = factory
            .engines
            .lock()
            .expect("recording engine mutex poisoned");
        let params = engines[0]
            .asyncHookParams
            .lock()
            .expect("async hook params mutex poisoned");
        let params = params
            .as_ref()
            .expect("hook must use asynchronous host execution");
        assert_eq!(params["eventPayload"], payload);
        assert_eq!(params["event"], "navigation_entry_action");
        assert_eq!(params["functionName"], "openBingFromSidebar");
    }

    /// Verifies ToolPkg contexts resolve modules through their bound resource host.
    #[test]
    fn createsToolPkgExecutionEngineWithResourceHost() {
        let (mut manager, factory) = snapshotRecordingManager();
        manager.registerToolPkg(ToolPkgLoadResult {
            containerPackage: ToolPackage {
                name: "snapshot_package".to_string(),
                ..ToolPackage::default()
            },
            containerRuntime: ToolPkgContainerRuntime {
                packageName: "snapshot_package".to_string(),
                mainEntry: "dist/main.js".to_string(),
                sourceType: ToolPkgSourceType::ASSET,
                sourcePath: "snapshot-package.toolpkg".to_string(),
                ..ToolPkgContainerRuntime::default()
            },
            ..ToolPkgLoadResult::default()
        });

        pollReadyCompletion(manager.dispatchToolPkgHook(
            &["snapshot_package".to_string()],
            ToolPkgHookInvocation {
                containerPackageName: "snapshot_package".to_string(),
                functionName: "onInputMenuToggle".to_string(),
                event: "input_menu_toggle".to_string(),
                eventName: None,
                pluginId: Some("snapshot_hook".to_string()),
                inlineFunctionSource: None,
                eventPayload: Value::Object(Default::default()),
                executionContextKey: None,
                runtimeKind: None,
                envOverrides: BTreeMap::new(),
                timestampMs: 1,
                timeoutMillis: Some(1_000),
                dispatchIntermediateOnMain: true,
                onIntermediateResult: None,
            },
        ))
        .expect("snapshot ToolPkg hook dispatch must succeed");

        let contexts = factory
            .contexts
            .lock()
            .expect("recording context factory mutex poisoned");
        let context = contexts
            .first()
            .expect("ToolPkg hook execution must create a bound context");
        assert_eq!(context.context_key, "toolpkg_main:snapshot_package");
        assert_eq!(context.container_package_name, "snapshot_package");
        assert!(context
            .text_resource_host
            .read_toolpkg_text_resource("snapshot_package", "dist/main.js")
            .expect("bound host must read the main module")
            .expect("main module must exist")
            .contains("onInputMenuToggle"));
        assert!(context
            .text_resource_host
            .read_toolpkg_text_resource("snapshot_package", "dist/shared.js")
            .expect("bound host must read the shared module")
            .expect("shared module must exist")
            .contains("createDefinition"));
    }
    struct AcquisitionTestWaker;

    impl std::task::Wake for AcquisitionTestWaker {
        /// Keeps immediately completing factory tests independent of an async executor.
        fn wake(self: Arc<Self>) {}
    }

    /// Polls the recording factory's immediate asynchronous acquisition exactly once.
    fn finishImmediateAcquisition(future: impl std::future::Future<Output = Result<(), String>>) -> Result<(), String> {
        let waker = std::task::Waker::from(Arc::new(AcquisitionTestWaker));
        let mut context = std::task::Context::from_waker(&waker);
        let mut future = std::pin::pin!(future);
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(result) => result,
            std::task::Poll::Pending => panic!("Recording factory must complete immediately"),
        }
    }

    /// Asynchronous acquisition preserves counted leases and reuses the live page context.
    #[test]
    fn asynchronousAcquisitionKeepsCountedLeases() {
        let (manager, factory) = recordingManager();
        finishImmediateAcquisition(manager.acquireToolPkgExecutionEngineAsync("async-page", "package_a")).unwrap();
        finishImmediateAcquisition(manager.acquireToolPkgExecutionEngineAsync("async-page", "package_a")).unwrap();
        assert_eq!(factory.engines.lock().unwrap().len(), 1);
        manager.releaseToolPkgExecutionEngine("async-page", "package_a");
        assert!(manager.findToolPkgExecutionEngine("async-page", "package_a").is_some());
        manager.releaseToolPkgExecutionEngine("async-page", "package_a");
        assert!(factory.engines.lock().unwrap()[0].destroyed.load(Ordering::Acquire));
    }

    /// Revoking a container while preparation is pending prevents publication of a stale engine.
    #[test]
    fn asynchronousAcquisitionRejectsRevokedContainer() {
        let (manager, factory) = recordingManager();
        let revoker = manager.clone();
        *factory.onCreate.lock().unwrap() = Some(Arc::new(move || {
            revoker.revokeContainerExecutionContexts("package_a");
        }));
        let result = finishImmediateAcquisition(manager.acquireToolPkgExecutionEngineAsync("async-revoked", "package_a"));
        assert_eq!(result.unwrap_err(), "ToolPkg execution context was revoked during engine acquisition");
        assert!(manager.findToolPkgExecutionEngine("async-revoked", "package_a").is_none());
        assert!(factory.engines.lock().unwrap()[0].destroyed.load(Ordering::Acquire));
        *factory.onCreate.lock().unwrap() = None;
    }

    /// Factory failures leave no published page context or counted lease.
    #[test]
    fn asynchronousAcquisitionPreservesFactoryError() {
        let (manager, factory) = recordingManager();
        *factory.creationError.lock().unwrap() = Some("prepared bootstrap failed".into());
        let result = finishImmediateAcquisition(manager.acquireToolPkgExecutionEngineAsync("async-error", "package_a"));
        assert_eq!(result.unwrap_err(), "prepared bootstrap failed");
        assert!(manager.findToolPkgExecutionEngine("async-error", "package_a").is_none());
        assert!(factory.engines.lock().unwrap().is_empty());
    }

    /// Verifies failed startup creates no cached engine and consumes no context lease.
    #[test]
    fn failedInitializationDoesNotRegisterAnEngineOrLease() {
        let (manager, factory) = recordingManager();
        *factory.creationError.lock().unwrap() = Some("duplicate tool catalog".into());
        assert_eq!(manager.acquireToolPkgExecutionEngine("failed-sidebar", "package_a").unwrap_err(), "duplicate tool catalog");
        assert!(manager.findToolPkgExecutionEngine("failed-sidebar", "package_a").is_none());
        assert!(factory.engines.lock().unwrap().is_empty());
        *factory.creationError.lock().unwrap() = None;
        manager.acquireToolPkgExecutionEngine("failed-sidebar", "package_a").unwrap();
        manager.releaseToolPkgExecutionEngine("failed-sidebar", "package_a");
        assert!(manager.findToolPkgExecutionEngine("failed-sidebar", "package_a").is_none());
        assert!(factory.engines.lock().unwrap()[0].destroyed.load(Ordering::Acquire));
    }

    /// Verifies a racing different owner rejects the losing engine and disposes it outside the registry lock.
    #[test]
    fn concurrentDifferentOwnerDisposesRejectedWorker() {
        let (manager, factory) = recordingManager();
        let competing = manager.clone();
        let weakFactory = Arc::downgrade(&factory);
        *factory.onCreate.lock().unwrap() = Some(Arc::new(move || {
            weakFactory.upgrade().unwrap().onCreate.lock().unwrap().take();
            competing.acquireToolPkgExecutionEngine("raced-sidebar", "package_b").unwrap();
        }));
        assert_eq!(manager.acquireToolPkgExecutionEngine("raced-sidebar", "package_a").unwrap_err(),
            "ToolPkg execution context belongs to a different container");
        assert_eq!(manager.getToolPkgExecutionEngine("raced-sidebar", "package_a").err().unwrap(),
            "ToolPkg execution context belongs to a different container");
        assert!(manager.findToolPkgExecutionEngine("raced-sidebar", "package_b").is_some());
        let engines = factory.engines.lock().unwrap().clone();
        assert_eq!(engines.len(), 2);
        assert!(!engines[0].destroyed.load(Ordering::Acquire));
        assert!(engines[1].destroyed.load(Ordering::Acquire));
        manager.releaseToolPkgExecutionEngine("raced-sidebar", "package_b");
        assert!(engines[0].destroyed.load(Ordering::Acquire));
    }

    #[test]
    fn mainInvocationsKeepRealmAfterCompletionAndCancellation() {
        let (mut manager, factory) = recordingManager();
        let (first, lease) = manager.leaseToolPkgMainExecutionEngine("package_a").unwrap();
        drop(lease);
        rescanRuntimeCatalog(&mut manager);
        for _ in 0..3 {
            let (next, lease) = manager.leaseToolPkgMainExecutionEngine("package_a").unwrap();
            assert!(Arc::ptr_eq(&first, &next));
            // Dropping the future cancels the call; the container still owns the realm.
            drop(async move { let _lease = lease; std::future::pending::<()>().await });
        }
        assert_eq!(factory.engines.lock().unwrap().len(), 1);
        assert!(!factory.engines.lock().unwrap()[0].destroyed.load(Ordering::Acquire));
        manager.revokeContainerExecutionContexts("package_a");
        assert!(factory.engines.lock().unwrap()[0].destroyed.load(Ordering::Acquire));
    }

    #[test]
    fn transientLeaseDestroysRealmWhenLastCallEnds() {
        let (manager, factory) = recordingManager();
        let (_, first) = manager.leaseToolPkgExecutionEngine("temporary", "package_a").unwrap();
        let (_, second) = manager.leaseToolPkgExecutionEngine("temporary", "package_a").unwrap();
        drop(first);
        assert!(!factory.engines.lock().unwrap()[0].destroyed.load(Ordering::Acquire));
        drop(second);
        assert!(factory.engines.lock().unwrap()[0].destroyed.load(Ordering::Acquire));
    }

    #[test]
    fn staleLeaseCannotReleaseReplacementRealm() {
        let (manager, factory) = recordingManager();
        let (_, stale) = manager.leaseToolPkgExecutionEngine("same-key", "package_a").unwrap();
        manager.revokeContainerExecutionContexts("package_a");
        let (_, current) = manager.leaseToolPkgExecutionEngine("same-key", "package_a").unwrap();
        drop(stale);
        assert!(!factory.engines.lock().unwrap()[1].destroyed.load(Ordering::Acquire));
        drop(current);
        assert!(factory.engines.lock().unwrap()[1].destroyed.load(Ordering::Acquire));
    }

    #[test]
    fn changedCatalogRevokesMainAndActivePageOnlyForChangedOwner() {
        let (mut manager, factory) = recordingManager();
        let (_, stale) = manager.leaseToolPkgMainExecutionEngine("package_a").unwrap();
        let (_, page) = manager.leaseToolPkgExecutionEngine("sidebar", "package_a").unwrap();
        let (other, otherLease) = manager.leaseToolPkgMainExecutionEngine("package_b").unwrap();
        drop(otherLease);
        let mut containers = manager.containers.read().unwrap().clone();
        containers.get_mut("package_a").unwrap().mainEntry = "dist/updated-main.js".into();
        let subpackages = manager.subpackageByPackageName.read().unwrap().clone();
        manager.replaceRuntimeMaps(containers, subpackages);
        assert!(factory.engines.lock().unwrap()[0].destroyed.load(Ordering::Acquire));
        assert!(factory.engines.lock().unwrap()[1].destroyed.load(Ordering::Acquire));
        assert!(Arc::ptr_eq(&other, &manager.findToolPkgExecutionEngine("toolpkg_main:package_b", "package_b").unwrap()));
        let (_, replacement) = manager.leaseToolPkgMainExecutionEngine("package_a").unwrap();
        drop(stale);
        drop(page);
        drop(replacement);
        assert!(!factory.engines.lock().unwrap()[3].destroyed.load(Ordering::Acquire));
        manager.removeToolPkgContainer("package_a");
        assert!(factory.engines.lock().unwrap()[3].destroyed.load(Ordering::Acquire));
        manager.updateExecutionEngineFactory(Arc::new(RecordingExecutionEngineFactory::default()));
        assert!(factory.engines.lock().unwrap()[2].destroyed.load(Ordering::Acquire));
    }

    /// Creates a real compressed main-script archive for source-generation tests.
    fn mainScriptArchive(script: &str) -> Vec<u8> {
        let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        archive.start_file("main.js", zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)).unwrap();
        archive.write_all(script.as_bytes()).unwrap();
        archive.finish().unwrap().into_inner()
    }

    /// Reads main code once per catalog generation and invalidates both directory and archive snapshots.
    #[test]
    fn mainScriptSourcesFollowCatalogGenerationAndShutdown() {
        for (source, file, archived) in [("/plugin", "/plugin/main.js", false), ("/plugin.toolpkg", "/plugin.toolpkg", true)] {
            let (mut manager, _) = recordingManager();
            let fs = Arc::new(TestFileSystemHost::default());
            let old = "exports.read = () => 1;";
            let new = "exports.read = () => 2;";
            let bytes = if archived { mainScriptArchive(old) } else { old.as_bytes().to_vec() };
            fs.files.lock().unwrap().insert(file.into(), bytes);
            manager.fileSystemHost = fs.clone();
            let mut runtime = manager.getToolPkgContainerRuntime("package_a").unwrap();
            runtime.sourceType = ToolPkgSourceType::EXTERNAL;
            runtime.sourcePath = source.into();
            runtime.mainEntry = "main.js".into();
            manager.registerToolPkg(ToolPkgLoadResult {
                containerPackage: ToolPackage { name: "package_a".into(), ..ToolPackage::default() },
                containerRuntime: runtime, ..ToolPkgLoadResult::default()
            });
            fs.byteReadCount.store(0, Ordering::Relaxed);
            for _ in 0..9 {
                assert_eq!(manager.getRegisteredToolPkgMainScript("package_a").as_deref(), Some(old));
                assert_eq!(manager.getToolPkgMainScriptInternal("package_a", &["package_a".into()]).as_deref(), Some(old));
            }
            assert_eq!(fs.byteReadCount.load(Ordering::Relaxed), 1);
            let bytes = if archived { mainScriptArchive(new) } else { new.as_bytes().to_vec() };
            fs.files.lock().unwrap().insert(file.into(), bytes);
            rescanRuntimeCatalog(&mut manager);
            fs.byteReadCount.store(0, Ordering::Relaxed);
            assert_eq!(manager.getRegisteredToolPkgMainScript("package_a").as_deref(), Some(new));
            assert_eq!(fs.byteReadCount.load(Ordering::Relaxed), 1);
            manager.shutdownExecutionEngines();
            assert!(manager.getRegisteredToolPkgMainScript("package_a").is_none());
        }
    }

    /// Same-size edits with an unchanged manifest and parent mtime still replace the realm.
    #[test]
    fn catalogSourceEditsRevokeDirectoryAndArchiveRealmsWithoutVersionChanges() {
        for source in ["/plugin", "/plugin.toolpkg"] {
            let (mut manager, factory) = recordingManager();
            let fs = Arc::new(TestFileSystemHost::default());
            let file = if source.ends_with(".toolpkg") { source.to_string() } else { format!("{source}/main.js") };
            fs.files.lock().unwrap().insert(file.clone(), b"old-code".to_vec());
            manager.fileSystemHost = fs.clone();
            let mut runtime = manager.getToolPkgContainerRuntime("package_a").unwrap();
            runtime.sourceType = ToolPkgSourceType::EXTERNAL;
            runtime.sourcePath = source.to_string();
            manager.registerToolPkg(ToolPkgLoadResult {
                containerPackage: ToolPackage { name: "package_a".into(), ..ToolPackage::default() },
                containerRuntime: runtime, ..ToolPkgLoadResult::default()
            });
            let (_, stale) = manager.leaseToolPkgMainExecutionEngine("package_a").unwrap();
            rescanRuntimeCatalog(&mut manager);
            assert!(!factory.engines.lock().unwrap()[0].destroyed.load(Ordering::Acquire));
            fs.files.lock().unwrap().insert(file.clone(), b"new-code".to_vec());
            rescanRuntimeCatalog(&mut manager);
            assert!(factory.engines.lock().unwrap()[0].destroyed.load(Ordering::Acquire));
            let (_, current) = manager.leaseToolPkgMainExecutionEngine("package_a").unwrap();
            drop(stale);
            drop(current);
            assert!(!factory.engines.lock().unwrap()[1].destroyed.load(Ordering::Acquire));
            fs.files.lock().unwrap().remove(&file);
            rescanRuntimeCatalog(&mut manager);
            assert!(factory.engines.lock().unwrap()[1].destroyed.load(Ordering::Acquire));
        }
    }

    #[test]
    fn coreShutdownDestroysAllRealmsAndRejectsLateCalls() {
        let (manager, factory) = recordingManager();
        let (_, main) = manager.leaseToolPkgMainExecutionEngine("package_a").unwrap();
        let (_, page) = manager.leaseToolPkgExecutionEngine("sidebar", "package_b").unwrap();
        manager.shutdownExecutionEngines();
        assert!(factory.engines.lock().unwrap().iter().all(|engine| engine.destroyed.load(Ordering::Acquire)));
        drop(main);
        drop(page);
        assert_eq!(manager.leaseToolPkgMainExecutionEngine("package_a").err().unwrap(),
            "ToolPkg execution owner has shut down");
        manager.shutdownExecutionEngines();
    }

}
