#![allow(non_snake_case)]

use super::*;
use crate::runtime_support::*;
use crate::tools::AIToolHandler::AIToolHandler;
use operit_host_api::{
    FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
    GrepCodeResult, HostError, HostResult, HostRuntimeAsyncTask, HostRuntimeTask,
    HostRuntimeTaskSchedulerHost, HostRuntimeTurnFuture, HttpHost, HttpImageDelivery, HttpStreamHost,
    RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeStorageEntry, RuntimeStorageHost,
    RuntimeStorageWriteSession,
};
use operit_host_native_filesystem::PosixFileSystemHost;
use operit_host_native_scheduler::{NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_js_bridge::javascript::JsExecutionProvider::HostJsExecutionProvider;
use operit_model::ToolPrompt::SystemToolPromptCategory;
use operit_store::RuntimeFileSyncStore::RuntimeFileSyncStore;
use operit_store::SyncOperationStore::{SyncClock, SyncOperationStore};
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use serde_json::{json, Value};
use std::io::Write;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};

const TEST_ID: &str = "local.update.fixture";

fn serialTest() -> std::sync::MutexGuard<'static, ()> {
    static SERIAL: Mutex<()> = Mutex::new(());
    SERIAL.lock().unwrap_or_else(|error| error.into_inner())
}

#[derive(Clone)]
struct Fault {
    operation: &'static str,
    suffix: String,
    skip: usize,
    after: bool,
    partial: bool,
}

impl Fault {
    fn new(operation: &'static str, suffix: impl Into<String>, after: bool) -> Self {
        Self { operation, suffix: suffix.into(), skip: 0, after, partial: false }
    }
}

#[derive(Default)]
struct Faults {
    rules: Mutex<Vec<Fault>>,
    mutations: Mutex<Vec<(String, String)>>,
}

impl Faults {
    fn arm(&self, fault: Fault) {
        self.rules.lock().unwrap().push(fault);
    }

    fn take(&self, operation: &str, path: &str) -> Option<Fault> {
        let mut rules = self.rules.lock().unwrap();
        let index = rules.iter().position(|rule| rule.operation == operation && path.ends_with(&rule.suffix))?;
        if rules[index].skip != 0 {
            rules[index].skip -= 1;
            return None;
        }
        Some(rules.remove(index))
    }

    fn perform<T>(&self, operation: &'static str, path: &str, action: impl FnOnce() -> HostResult<T>) -> HostResult<T> {
        if matches!(operation, "write" | "delete" | "move" | "store_write" | "store_delete") {
            self.mutations.lock().unwrap().push((operation.into(), path.into()));
        }
        match self.take(operation, path) {
            Some(fault) => {
                if fault.after { action()?; }
                Err(HostError::new(format!("injected {operation} failure")))
            }
            None => action(),
        }
    }
}

struct FaultFileSystem {
    inner: PosixFileSystemHost,
    faults: Arc<Faults>,
    frozen: Mutex<Option<(String, i64, String)>>,
    failAfterPublication: Mutex<Option<(String, ToolPkgManager, String)>>,
}

impl FileSystemHost for FaultFileSystem {
    fn envLabel(&self) -> &str { self.inner.envLabel() }
    fn environmentDescriptor(&self) -> HostEnvironmentDescriptor { self.inner.environmentDescriptor() }
    fn validatePath(&self, path: &str, name: &str) -> HostResult<()> { self.inner.validatePath(path, name) }
    fn listFiles(&self, path: &str) -> HostResult<Vec<FileEntry>> {
        self.faults.perform("list", path, || self.inner.listFiles(path))
    }
    fn readFile(&self, path: &str) -> HostResult<String> { self.inner.readFile(path) }
    fn readFileWithLimit(&self, path: &str, limit: usize) -> HostResult<String> { self.inner.readFileWithLimit(path, limit) }
    fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        let mut publication = self.failAfterPublication.lock().unwrap();
        if publication.as_ref().is_some_and(|(suffix, registry, version)| {
            path.ends_with(suffix) && registry.getToolPkgContainerRuntime(TEST_ID)
                .is_some_and(|runtime| runtime.version == *version)
        }) {
            publication.take();
            return Err(HostError::new("injected post-publication validation failure"));
        }
        drop(publication);
        self.faults.perform("read", path, || self.inner.readFileBytes(path))
    }
    fn readFileBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        self.inner.readFileBytesRange(path, offset, length)
    }
    fn writeFile(&self, path: &str, content: &str, append: bool) -> HostResult<()> {
        self.faults.perform("write", path, || self.inner.writeFile(path, content, append))
    }
    fn writeFileBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.faults.perform("write", path, || self.inner.writeFileBytes(path, content))
    }
    fn appendFileBytes(&self, path: &str, content: &[u8]) -> HostResult<()> { self.inner.appendFileBytes(path, content) }
    fn deleteFile(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.faults.perform("delete", path, || self.inner.deleteFile(path, recursive))
    }
    fn fileExists(&self, path: &str) -> HostResult<FileExistence> { self.inner.fileExists(path) }
    fn moveFile(&self, source: &str, destination: &str) -> HostResult<()> {
        self.faults.perform("move", destination, || self.inner.moveFile(source, destination))
    }
    fn copyFile(&self, source: &str, destination: &str, recursive: bool) -> HostResult<()> {
        self.inner.copyFile(source, destination, recursive)
    }
    fn makeDirectory(&self, path: &str, parents: bool) -> HostResult<()> { self.inner.makeDirectory(path, parents) }
    fn findFiles(&self, request: FindFilesRequest) -> HostResult<Vec<String>> { self.inner.findFiles(request) }
    fn fileInfo(&self, path: &str) -> HostResult<FileInfo> {
        let mut info = self.inner.fileInfo(path)?;
        if let Some((suffix, size, modified)) = self.frozen.lock().unwrap().as_ref() {
            if path.ends_with(suffix) { info.size = *size; info.lastModified = modified.clone(); }
        }
        Ok(info)
    }
    fn grepCode(&self, request: GrepCodeRequest) -> HostResult<GrepCodeResult> { self.inner.grepCode(request) }
    fn zipFiles(&self, source: &str, destination: &str) -> HostResult<()> { self.inner.zipFiles(source, destination) }
    fn unzipFiles(&self, source: &str, destination: &str) -> HostResult<()> { self.inner.unzipFiles(source, destination) }
    fn openFile(&self, path: &str) -> HostResult<()> { self.inner.openFile(path) }
    fn shareFile(&self, path: &str, title: &str) -> HostResult<()> { self.inner.shareFile(path, title) }
}

#[derive(Clone)]
struct FaultStorage {
    inner: Arc<NativeRuntimeStorageHost>,
    faults: Arc<Faults>,
    frozen: Arc<Mutex<BTreeMap<String, i64>>>,
    extensionRecordListings: Arc<AtomicUsize>,
}

impl RuntimeStorageHost for FaultStorage {
    fn openWriteSession(&self, path: &str) -> HostResult<Option<Box<dyn RuntimeStorageWriteSession>>> {
        self.inner.openWriteSession(path)
    }
    fn openTransactionDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        self.inner.openTransactionDatabase(path)
    }
    fn runtimeRootDir(&self) -> Option<PathBuf> { self.inner.runtimeRootDir() }
    fn workspaceRootDir(&self) -> Option<PathBuf> { self.inner.workspaceRootDir() }
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> { self.inner.readBytes(path) }
    fn readBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        self.inner.readBytesRange(path, offset, length)
    }
    fn writeBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
        self.faults.mutations.lock().unwrap().push(("store_write".into(), path.into()));
        if let Some(fault) = self.faults.take("store_write", path) {
            if fault.partial { self.inner.writeBytes(path, b"{")?; }
            else if fault.after { self.inner.writeBytes(path, bytes)?; }
            return Err(HostError::new("injected store_write failure"));
        }
        self.inner.writeBytes(path, bytes)
    }
    fn modifiedAt(&self, path: &str) -> HostResult<i64> {
        if let Some(time) = self.frozen.lock().unwrap().get(path) { return Ok(*time); }
        self.inner.modifiedAt(path)
    }
    fn setModifiedAt(&self, path: &str, time: i64) -> HostResult<()> { self.inner.setModifiedAt(path, time) }
    fn writeBytesAtomically(&self, path: &str, bytes: &[u8]) -> HostResult<()> { self.inner.writeBytesAtomically(path, bytes) }
    fn appendBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> { self.inner.appendBytes(path, bytes) }
    fn flushFile(&self, path: &str) -> HostResult<()> { self.inner.flushFile(path) }
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.faults.perform("store_delete", path, || self.inner.delete(path, recursive))
    }
    fn exists(&self, path: &str) -> HostResult<bool> { self.inner.exists(path) }
    fn list(&self, path: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        if path.starts_with("runtime/extensions/") && path.ends_with("/records") {
            self.extensionRecordListings.fetch_add(1, Ordering::Relaxed);
        }
        self.inner.list(path)
    }
}

impl RuntimeSqliteHost for FaultStorage {
    fn runtimeStorage(&self) -> Option<Arc<dyn RuntimeStorageHost>> { Some(Arc::new(self.clone())) }
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        self.inner.openSqliteDatabase(path)
    }
}

struct TestPackageProvider {
    inner: HostJsExecutionProvider,
    registrations: AtomicUsize,
    failAt: AtomicUsize,
}

impl TestPackageProvider {
    fn failRegistration(&self, nth: usize) {
        self.registrations.store(0, Ordering::SeqCst);
        self.failAt.store(nth, Ordering::SeqCst);
    }
}

impl operit_plugin_sdk::javascript::JsExecutionProvider for TestPackageProvider {
    fn create_execution_engine(&self, host: Arc<dyn operit_plugin_sdk::javascript::JsExecutionHost>)
        -> Result<Arc<dyn operit_plugin_sdk::javascript::JsExecutionEngine>, String> {
        let call = self.registrations.fetch_add(1, Ordering::SeqCst) + 1;
        if self.failAt.compare_exchange(call, 0, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            return Err("injected registration failure".into());
        }
        self.inner.create_execution_engine(host)
    }
    fn create_toolpkg_execution_engine(&self, host: Arc<dyn operit_plugin_sdk::javascript::JsExecutionHost>,
        context: ToolPkgExecutionContext) -> Result<Arc<dyn operit_plugin_sdk::javascript::JsExecutionEngine>, String> {
        self.inner.create_toolpkg_execution_engine(host, context)
    }
    fn acquire_toolpkg_execution_engine(&self, host: Arc<dyn operit_plugin_sdk::javascript::JsExecutionHost>,
        context: ToolPkgExecutionContext)
        -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Arc<dyn operit_plugin_sdk::javascript::JsExecutionEngine>, String>> {
        self.inner.acquire_toolpkg_execution_engine(host, context)
    }
    fn create_package_executor(&self, runtime: Arc<dyn operit_plugin_sdk::javascript::JsPackageRuntime>,
        host: Arc<dyn operit_plugin_sdk::javascript::JsExecutionHost>)
        -> Arc<dyn operit_plugin_sdk::javascript::JsPackageExecutor> {
        self.inner.create_package_executor(runtime, host)
    }
}

struct TestPackageRuntimeSupport {
    builtin: &'static [RuntimePluginAsset],
    bundled: &'static [RuntimePluginAsset],
}

struct RecordingScheduler {
    inner: NativeHostRuntimeTaskSchedulerHost,
    recordNotifications: bool,
    pending: Mutex<Vec<HostRuntimeTask>>,
}

impl RecordingScheduler {
    fn runNotificationsInReverse(&self) {
        let mut tasks = std::mem::take(&mut *self.pending.lock().unwrap());
        while let Some(task) = tasks.pop() { task(); }
    }
}

impl HostRuntimeTaskSchedulerHost for RecordingScheduler {
    fn monotonicTimeMillis(&self) -> HostResult<u64> { self.inner.monotonicTimeMillis() }
    fn scheduleHostRuntimeTask(&self, name: &str, task: HostRuntimeTask) -> HostResult<()> {
        if self.recordNotifications && name == "operit-toolpkg-runtime-change" {
            self.pending.lock().unwrap().push(task);
            Ok(())
        } else { self.inner.scheduleHostRuntimeTask(name, task) }
    }
    fn scheduleHostRuntimeAsyncTask(&self, name: &str, task: HostRuntimeAsyncTask) -> HostResult<()> {
        self.inner.scheduleHostRuntimeAsyncTask(name, task)
    }
    fn scheduleHostRuntimeBlockingAsyncTask(&self, name: &str, task: HostRuntimeAsyncTask) -> HostResult<()> {
        self.inner.scheduleHostRuntimeBlockingAsyncTask(name, task)
    }
    fn scheduleHostRuntimeCooperativeAsyncTask(&self, name: &str, task: HostRuntimeAsyncTask) -> HostResult<()> {
        self.inner.scheduleHostRuntimeCooperativeAsyncTask(name, task)
    }
    fn scheduleDelayedHostRuntimeTask(&self, name: &str, delay: u64, task: HostRuntimeTask) -> HostResult<()> {
        self.inner.scheduleDelayedHostRuntimeTask(name, delay, task)
    }
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture { self.inner.waitForHostRuntimeTaskTurn() }
    fn waitForHostRuntimeDelay(&self, delay: u64) -> HostRuntimeTurnFuture { self.inner.waitForHostRuntimeDelay(delay) }
}

struct LocalUpdateFixture {
    root: PathBuf,
    storage: Arc<FaultStorage>,
    filesystem: Arc<FaultFileSystem>,
    provider: Arc<TestPackageProvider>,
    handler: AIToolHandler,
    manager: Arc<Mutex<RuntimePackageManager>>,
    scheduler: Arc<RecordingScheduler>,
}

impl LocalUpdateFixture {
    fn new() -> Self { Self::withAssets(&[], &[]) }

    fn withAssets(builtin: &'static [RuntimePluginAsset], bundled: &'static [RuntimePluginAsset]) -> Self {
        Self::build(builtin, bundled, false)
    }

    fn build(builtin: &'static [RuntimePluginAsset], bundled: &'static [RuntimePluginAsset], recordNotifications: bool) -> Self {
        let root = std::env::temp_dir().join(format!("operit-local-update-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("artifacts")).unwrap();
        let storage = Arc::new(FaultStorage {
            inner: Arc::new(NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces"))),
            faults: Arc::new(Faults::default()), frozen: Default::default(), extensionRecordListings: Default::default(),
        });
        operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost(storage.clone());
        operit_store::RuntimeStorageHost::setDefaultRuntimeSqliteHost(storage.clone());
        let scheduler = Arc::new(RecordingScheduler {
            inner: NativeHostRuntimeTaskSchedulerHost::new(), recordNotifications, pending: Default::default(),
        });
        let javascript = Arc::new(NativeHostJavaScriptRuntimeHost::new());
        operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(scheduler.clone());
        operit_host_api::HostManager::setDefaultHostJavaScriptRuntimeHost(javascript.clone());
        let filesystem = Arc::new(FaultFileSystem {
            inner: PosixFileSystemHost::new(), faults: Arc::new(Faults::default()), frozen: Default::default(),
            failAfterPublication: Default::default(),
        });
        let provider = Arc::new(TestPackageProvider {
            inner: HostJsExecutionProvider::new(), registrations: AtomicUsize::new(0), failAt: AtomicUsize::new(0),
        });
        let handler = AIToolHandler::new(HostManager {
            runtimeStorageHost: Some(storage.clone()), runtimeSqliteHost: Some(storage.clone()),
            fileSystemHost: Some(filesystem.clone()), httpHost: Some(Arc::new(RejectingHttpHost)),
            systemOperationHost: Some(Arc::new(LanguageHost)),
            hostRuntimeTaskSchedulerHost: Some(scheduler.clone()), hostJavaScriptRuntimeHost: Some(javascript),
            hostEnvironment: HostEnvironmentDescriptor::linux(), ..Default::default()
        }, ToolRuntimeDependencies::new(Arc::new(TestPackageRuntimeSupport { builtin, bundled }), provider.clone()));
        let manager = handler.getOrCreatePackageManager();
        {
            let mut registry = manager.lock().unwrap();
            assert!(registry.claimInitialPackageRegistryLoad().unwrap());
            registry.loadAvailablePackages();
            registry.packageRegistryReadiness().require_ready().unwrap();
        }
        Self { root, storage, filesystem, provider, handler, manager, scheduler }
    }

    fn import(&self, filename: &str, bytes: &[u8]) -> Result<ExternalPackageImportResult, String> {
        let path = self.root.join("artifacts").join(filename);
        std::fs::write(&path, bytes).unwrap();
        self.manager.lock().unwrap().addPackageFileFromExternalStorageResult(&path.to_string_lossy())
    }

    fn install(&self, scope: &str) {
        let bytes = packageArchive(TEST_ID, Some(scope), "1.0.0", "v1", &[("kept.child", true)]);
        self.import("previous.toolpkg", &bytes).unwrap();
        let mut manager = self.manager.lock().unwrap();
        manager.enablePackage(TEST_ID);
        assert!(manager.isPackageEnabled(TEST_ID));
    }

    async fn probe(&self, payload: Value) -> Value {
        let invocation = self.manager.lock().unwrap().invokeToolPkgPublicApiWithTimeoutMillis(
            TEST_ID, "test.configuration", payload, 10_000,
        );
        invocation.await.unwrap()
    }

    fn record(&self) -> ExtensionRecord {
        ExtensionStore::new(self.storage.clone()).record("package", TEST_ID).unwrap()
    }

    fn descriptorKey(&self) -> String {
        let record = self.record();
        operit_store::ExtensionStore::recordPath("package", TEST_ID, &record.scope).unwrap()
    }

    fn archiveKey(&self) -> String {
        let record = self.record();
        format!("{}/{}", ExtensionStore::root("package", &record.scope).unwrap(), record.sourceName)
    }

    fn nativeSuffix(&self) -> String {
        format!("/{}", self.archiveKey().strip_prefix("runtime/").unwrap())
    }

    fn temporaryFiles(&self) -> Vec<String> {
        let record = self.record();
        self.storage.list(&ExtensionStore::root("package", &record.scope).unwrap()).unwrap().into_iter()
            .filter(|entry| entry.path.contains(".local-toolpkg-update-"))
            .map(|entry| entry.path).collect()
    }

    fn latestFileBytes(&self, key: &str) -> Vec<u8> {
        let operations = SyncOperationStore::new(self.storage.clone(), RUNTIME_SYNC_DIR_PATH)
            .operationsSince(&SyncClock::empty(), &[], 10_000).unwrap();
        let operation = operations.iter().filter(|operation| operation.entityId == key
            && operation.domain == "runtime_file" && operation.operation == "upsert")
            .max_by_key(|operation| operation.sequence)
            .expect("canonical Space file must have an explicit synchronization snapshot");
        let reference = RuntimeFileSyncStore::requiredBlob(operation).unwrap().unwrap();
        RuntimeFileSyncStore::new(self.storage.clone(), RUNTIME_SYNC_DIR_PATH)
            .readBlobChunk(&reference.contentHash, 0, reference.size).unwrap()
    }
}

impl Drop for LocalUpdateFixture {
    fn drop(&mut self) {
        self.filesystem.failAfterPublication.lock().unwrap().take();
        self.scheduler.pending.lock().unwrap().clear();
        let shutdown = self.manager.lock().ok().map(|manager| manager.executionEngineShutdown());
        if let Some(shutdown) = shutdown { shutdown(); }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

struct InstallationSnapshot {
    record: ExtensionRecord,
    descriptor: Vec<u8>,
    archive: Vec<u8>,
    sentinels: Vec<(String, Vec<u8>)>,
}

impl InstallationSnapshot {
    async fn capture(fixture: &LocalUpdateFixture) -> Self {
        let saved = fixture.probe(json!({"write": true, "value": "saved preference"})).await;
        assert_eq!(saved["preference"], "saved preference");
        let record = fixture.record();
        let config = ExtensionStore::configPathForScope(TEST_ID, &record.scope).unwrap();
        let namespace = operit_util::OperitPaths::pluginConfigDirName("kept.child").unwrap();
        let paths = [
            format!("{config}/namespaces/{namespace}/saved.json"),
            format!("{}/legacy.json", config.replacen("/plugins/configs/", "/plugins/data/", 1)),
            format!("{}/stable.txt", ExtensionStore::dataPathForScope(TEST_ID, "device").unwrap()),
            format!("{}/stable.txt", ExtensionStore::dataPathForScope(TEST_ID, "space").unwrap()),
        ];
        let mut sentinels = Vec::new();
        for (index, path) in paths.into_iter().enumerate() {
            let bytes = format!("preserved {index}").into_bytes();
            fixture.storage.writeBytes(&path, &bytes).unwrap();
            sentinels.push((path, bytes));
        }
        let configFile = format!("{config}/update-test.json");
        sentinels.push((configFile.clone(), fixture.storage.readBytes(&configFile).unwrap()));
        let environment = format!("{config}/environment.json");
        sentinels.push((environment.clone(), fixture.storage.readBytes(&environment).unwrap()));
        Self {
            record, descriptor: fixture.storage.readBytes(&fixture.descriptorKey()).unwrap(),
            archive: fixture.storage.readBytes(&fixture.archiveKey()).unwrap(), sentinels,
        }
    }

    fn assertData(&self, fixture: &LocalUpdateFixture) {
        for (path, bytes) in &self.sentinels {
            assert_eq!(fixture.storage.readBytes(path).unwrap(), *bytes, "configuration changed: {path}");
        }
    }

    async fn assertRollback(&self, fixture: &LocalUpdateFixture) {
        assert_eq!(fixture.record(), self.record);
        assert_eq!(fixture.storage.readBytes(&fixture.descriptorKey()).unwrap(), self.descriptor);
        assert_eq!(fixture.storage.readBytes(&fixture.archiveKey()).unwrap(), self.archive);
        self.assertData(fixture);
        assert!(fixture.temporaryFiles().is_empty());
        let read = fixture.probe(json!({})).await;
        assert_eq!(read["preference"], "saved preference");
        assert_eq!(read["codeVersion"], "v1");
    }
}

fn packageEntries(id: &str, scope: Option<&str>, version: &str, marker: &str,
    children: &[(&str, bool)]) -> Vec<(String, String)> {
    let mut manifest = json!({
        "schema_version": 1, "toolpkg_id": id, "api_version": "2.0.0", "version": version,
        "main": "main.js", "public_api": "api.ts", "enabled_by_default": true,
        "subpackages": children.iter().map(|(name, _)| json!({"id": name,
            "entry": format!("children/{name}.js")})).collect::<Vec<_>>()
    });
    if let Some(scope) = scope { manifest["installation_scope"] = json!(scope); }
    let main = format!(r#"
exports.configProbe = async function(event) {{
    const config = await PluginConfig.use('update-test', {{ preference: 'default' }});
    if (event.payload.write) {{
        config.preference = event.payload.value;
        await PluginConfig.flush(config);
    }}
    return {{ preference: config.preference, codeVersion: '{marker}' }};
}};
exports.registerToolPkg = function() {{
    ToolPkg.registerApi({{ name: 'test.configuration', function: exports.configProbe }});
}};
"#);
    let mut entries = vec![("manifest.json".into(), manifest.to_string()),
        ("main.js".into(), main), ("api.ts".into(), "export {};".into())];
    for (name, enabled) in children {
        let metadata = json!({"name": name, "enabledByDefault": enabled,
            "tools": [{"name": "probe", "description": "Child probe", "parameters": []}]});
        entries.push((format!("children/{name}.js"),
            format!("/* METADATA {metadata} */\nexports.probe = function() {{ return 'child'; }};")));
    }
    entries
}

fn archiveEntries(entries: Vec<(String, String)>) -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, text) in entries {
        archive.start_file(name, zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)).unwrap();
        archive.write_all(text.as_bytes()).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

fn packageArchive(id: &str, scope: Option<&str>, version: &str, marker: &str,
    children: &[(&str, bool)]) -> Vec<u8> {
    archiveEntries(packageEntries(id, scope, version, marker, children))
}

fn nextArchive(scope: Option<&str>) -> Vec<u8> {
    packageArchive(TEST_ID, scope, "2.0.0", "v2", &[("kept.child", false), ("new.child", true)])
}

#[test]
fn owner_check_skips_extension_scan_until_an_a2a_package_is_published() {
    use crate::tools::a2a::A2ARepository::{A2AConfig, A2ARepository};

    let _serial = serialTest();
    let fixture = LocalUpdateFixture::new();
    fixture.install("device");
    let mut manager = fixture.manager.lock().unwrap();
    let repository = A2ARepository::new(manager.context.as_ref().clone()).unwrap();
    repository.save("test-agent", A2AConfig { url: "https://example.invalid/".into(), ..Default::default() }).unwrap();
    let listings = &fixture.storage.extensionRecordListings;
    let before = listings.load(Ordering::Relaxed);
    manager.requireEnabledToolPkgContainer(TEST_ID).unwrap();
    assert_eq!(listings.load(Ordering::Relaxed), before);
    assert!(!manager.isPackageEnabled("a2a.test-agent"));

    manager.reloadA2APackages();
    assert!(manager.isPackageEnabled("a2a.test-agent"));
    repository.set_enabled("test-agent", false).unwrap();
    assert!(!manager.isPackageEnabled("a2a.test-agent"));
    manager.requireEnabledToolPkgContainer(TEST_ID).unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn local_same_id_update_preserves_configuration_and_scope() {
    let _serial = serialTest();
    for scope in ["device", "space"] {
        let fixture = LocalUpdateFixture::new();
        fixture.install(scope);
        let snapshot = InstallationSnapshot::capture(&fixture).await;
        let replacement = nextArchive(None);
        let result = fixture.import("different-filename.toolpkg", &replacement).unwrap();
        let record = fixture.record();
        assert_eq!(result.packageName, TEST_ID);
        assert_eq!(record.sourceName, snapshot.record.sourceName);
        assert_eq!(record.scope, scope);
        for key in ["order", "installationId", "builtin"] {
            assert_eq!(record.settings[key], snapshot.record.settings[key]);
        }
        snapshot.assertData(&fixture);
        assert!(fixture.temporaryFiles().is_empty());
        let read = fixture.probe(json!({})).await;
        assert_eq!(read["preference"], "saved preference");
        assert_eq!(read["codeVersion"], "v2");
        let manager = fixture.manager.lock().unwrap();
        assert!(manager.isPackageEnabled("kept.child"));
        assert!(!manager.isPackageEnabled("new.child"));
        assert_eq!(manager.getToolPkgContainerRuntime(TEST_ID).unwrap().version, "2.0.0");
        drop(manager);
        if scope == "space" { assert_eq!(fixture.latestFileBytes(&fixture.archiveKey()), replacement); }
    }
}

#[allow(unused_variables)]
impl ToolRuntimeSupport for TestPackageRuntimeSupport {
    fn bindCoreNodeToolRuntime(&self, runtime: Arc<dyn CoreNodeToolRuntime>) -> Result<(), String> { panic!("Unexpected runtime service: bindCoreNodeToolRuntime") }
    fn coreNodeRouteState(&self) -> Result<RuntimeCoreNodeRouteState, String> { panic!("Unexpected runtime service: coreNodeRouteState") }
    fn bindCoreRouteChangeHandler(&self, handler: CoreRouteChangeHandler) -> Result<(), String> { panic!("Unexpected runtime service: bindCoreRouteChangeHandler") }
    fn requestCoreRouteChange<'a>(&'a self, chatId: String, targetNodeId: String, resumeContext: CoreRouteResumeContext) -> ToolRuntimeSupportFuture<'a, Result<(), String>> { panic!("Unexpected runtime service: requestCoreRouteChange") }
    fn listModelSummaries(&self) -> Result<Vec<operit_plugin_sdk::js_sdk::models::ModelsSummary>, String> { panic!("Unexpected runtime service: listModelSummaries") }
    fn speakSpeech(&self, _request: operit_plugin_sdk::js_sdk::tts::TtsSpeakRequest) -> futures_util::future::BoxFuture<'_, Result<operit_plugin_sdk::js_sdk::results::TtsSpeakResultData, String>> { panic!("Unexpected runtime service: speakSpeech") }
    fn listTtsConfigs(&self) -> Result<Vec<operit_plugin_sdk::js_sdk::tts::TtsConfig>, String> { panic!("Unexpected runtime service: listTtsConfigs") }
    fn listThemeConfigs(&self) -> Result<Vec<operit_plugin_sdk::js_sdk::theme::ThemeConfig>, String> { panic!("Unexpected runtime service: listThemeConfigs") }
    fn applyThemeConfig(&self, id: String) -> Result<operit_plugin_sdk::js_sdk::theme::ThemeConfig, String> { panic!("Unexpected runtime service: applyThemeConfig") }
    fn getCurrentTtsConfigId(&self) -> Result<String, String> { panic!("Unexpected runtime service: getCurrentTtsConfigId") }
    fn setCurrentTtsConfigId(&self, id: String) -> Result<String, String> { panic!("Unexpected runtime service: setCurrentTtsConfigId") }
    fn filterToolCatalog<'a>(
        &'a self,
        _entries: Vec<crate::tools::climode::CliToolModeSupport::HiddenToolCatalogEntry>,
        _context: crate::ToolExecutionManager::ToolRuntimeContext,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<crate::tools::climode::CliToolModeSupport::HiddenToolCatalogEntry>, String>> + Send + 'a>> {
        panic!("Unexpected runtime service: filterToolCatalog")
    }
    fn readToolSourceCatalog<'a>(&'a self)
        -> Pin<Box<dyn Future<Output = Result<operit_plugin_sdk::js_sdk::catalog::CatalogToolSources, String>> + Send + 'a>> {
        panic!("Unexpected runtime service: readToolSourceCatalog")
    }
    fn readEnvironmentVariable(&self, packageName: &str, key: &str) -> Result<Option<String>, String> { panic!("Unexpected runtime service: readEnvironmentVariable") }
    fn writeEnvironmentVariable(&self, packageName: &str, key: &str, value: &str) -> Result<(), String> { panic!("Unexpected runtime service: writeEnvironmentVariable") }
    fn removeEnvironmentVariable(&self, packageName: &str, key: &str) -> Result<(), String> { panic!("Unexpected runtime service: removeEnvironmentVariable") }
    fn buildBuiltinAndInternalCategories(&self, useEnglish: bool, hostEnvironment: &HostEnvironmentDescriptor) -> Vec<SystemToolPromptCategory> { Vec::new() }
    fn buildBuiltinToolNameSet(&self, useEnglish: bool, hostEnvironment: &HostEnvironmentDescriptor) -> BTreeSet<String> { BTreeSet::new() }
    fn aiVisibleSkillPackages(&self) -> Vec<RuntimeSkillCatalogEntry> { Vec::new() }
    fn cachedMcpTools(&self, serverName: &str) -> Vec<crate::runtime_support::CachedMcpToolInfo> { Vec::new() }
    fn builtinPluginAssets(&self) -> &'static [RuntimePluginAsset] { self.builtin }
    fn bundledExternalPluginAssets(&self) -> &'static [RuntimePluginAsset] { self.bundled }
    fn builtinSkillNames(&self) -> &'static [&'static str] { &[] }
    fn bundledExternalSkillAssets(&self) -> &'static [RuntimeBundledExternalSkillAsset] { &[] }
    fn isSkillVisibleToAi(&self, skillName: &str) -> bool { panic!("Unexpected runtime service: isSkillVisibleToAi") }
    fn setSkillVisibleToAi(&self, skillName: &str, visible: bool) -> Result<(), String> { panic!("Unexpected runtime service: setSkillVisibleToAi") }
    fn generateMcpPluginDescription<'a>(&'a self, pluginName: &'a str, toolDescriptions: &'a [String]) -> ToolRuntimeSupportFuture<'a, Result<String, String>> { panic!("Unexpected runtime service: generateMcpPluginDescription") }
    fn startChatServices(&self) -> Result<(), String> { panic!("Unexpected runtime service: startChatServices") }
    fn stopChatServices(&self) -> Result<(), String> { panic!("Unexpected runtime service: stopChatServices") }
    fn isChatProcessing(&self, chatId: &str) -> Result<bool, String> { panic!("Unexpected runtime service: isChatProcessing") }
    fn switchMainChat<'a>(&'a self, chatId: &'a str) -> ToolRuntimeSupportFuture<'a, Result<(), String>> { panic!("Unexpected runtime service: switchMainChat") }
    fn createChatRuntime<'a>(&'a self, setAsCurrentChat: bool, sourceChatId: Option<String>, parentChatId: Option<String>, input: Option<serde_json::Value>) -> ToolRuntimeSupportFuture<'a, Result<String, String>> { panic!("Unexpected runtime service: createChatRuntime") }
    fn listScheduledChatMessages<'a>(&'a self, owner: &'a str)
        -> futures_util::future::BoxFuture<'a, Result<operit_plugin_sdk::js_sdk::results::ChatScheduledMessagesResultData, String>> { panic!("Unexpected scheduled chat runtime access") }
    fn setScheduledChatForeground(&self, _: bool) {}
    fn deliverPreparedChatMessage<'a>(&'a self, request: operit_plugin_sdk::js_sdk::chat::ChatPreparedMessageRequest)
        -> futures_util::future::BoxFuture<'a, Result<operit_plugin_sdk::js_sdk::results::ChatPreparedDeliveryResultData, String>> { panic!("Unexpected prepared chat runtime access") }
    fn scheduleChatMessage<'a>(&'a self, owner: &'a str, request: operit_plugin_sdk::js_sdk::chat::ChatScheduledMessageRequest)
        -> futures_util::future::BoxFuture<'a, Result<operit_plugin_sdk::js_sdk::results::ChatScheduleMessageResultData, String>> { panic!("Unexpected prepared chat runtime access") }
    fn cancelScheduledChatMessage<'a>(&'a self, owner: &'a str, id: String)
        -> futures_util::future::BoxFuture<'a, Result<operit_plugin_sdk::js_sdk::results::ChatCancelScheduledMessageResultData, String>> { panic!("Unexpected prepared chat runtime access") }

    fn readChatExtension(&self, owner: &str, target: &operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget) -> Result<Option<operit_plugin_sdk::js_sdk::core::JsonObject>, String> { panic!("Unexpected runtime service: readChatExtension") }
    fn writeChatExtension(&self, owner: &str, target: &operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget, value: operit_plugin_sdk::js_sdk::core::JsonObject) -> Result<operit_plugin_sdk::js_sdk::core::JsonObject, String> { panic!("Unexpected runtime service: writeChatExtension") }
    fn deleteChatExtension(&self, owner: &str, target: &operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget) -> Result<bool, String> { panic!("Unexpected runtime service: deleteChatExtension") }
    fn openPluginChatMessage(&self, owner: &str, request: operit_plugin_sdk::js_sdk::chat::ChatSendRequest, observeParts: bool) -> Result<operit_plugin_sdk::js_sdk::JsAsyncIterable<operit_plugin_sdk::js_sdk::chat::ChatSendEvent>, String> { panic!("Unexpected runtime service: openPluginChatMessage") }
    fn requestPluginChatCancellation(&self, owner: &str, chatId: &str) -> Result<operit_plugin_sdk::js_sdk::chat::ChatCancelResult, String> { panic!("Unexpected runtime service: requestPluginChatCancellation") }
    fn sendChatMessage<'a>(&'a self, request: RuntimeChatSendRequest) -> ToolRuntimeSupportFuture<'a, Result<operit_plugin_sdk::js_sdk::results::MessageSendResultData, String>> { panic!("Unexpected runtime service: sendChatMessage") }
    fn callChatModel<'a>(&'a self, request: RuntimeChatCallRequest) -> ToolRuntimeSupportFuture<'a, Result<String, String>> { panic!("Unexpected runtime service: callChatModel") }
    fn processFileBindingOperations(&self, originalContent: &str, operations: &[RuntimeStructuredEditOperation]) -> (String, String) { panic!("Unexpected runtime service: processFileBindingOperations") }
    /// Omits UI diff rendering while file tools perform real configuration writes.
    fn generateUnifiedDiff(&self, oldContent: &str, newContent: &str) -> String { String::new() }
}

struct LanguageHost;
#[allow(unused_variables)]
impl operit_host_api::SystemOperationHost for LanguageHost {
    fn getSystemLanguageCode(&self) -> operit_host_api::HostResult<String> { Ok("en".into()) }
    fn sendNotification(&self, request: &operit_host_api::SystemNotificationRequest) -> operit_host_api::HostResult<()> { panic!("Unexpected host call: sendNotification") }
    fn modifySystemSetting(&self, namespace: &str, setting: &str, value: &str,) -> operit_host_api::HostResult<operit_host_api::SystemSettingData> { panic!("Unexpected host call: modifySystemSetting") }
    fn getSystemSetting(&self, namespace: &str, setting: &str) -> operit_host_api::HostResult<operit_host_api::SystemSettingData> { panic!("Unexpected host call: getSystemSetting") }
    fn installApp(&self, source: &dyn operit_host_api::FileSource, timing: operit_host_api::AppInstallTiming) -> operit_host_api::HostResult<operit_host_api::AppOperationData> { panic!("Unexpected host call: installApp") }
    fn uninstallApp(&self, packageName: &str) -> operit_host_api::HostResult<operit_host_api::AppOperationData> { panic!("Unexpected host call: uninstallApp") }
    fn listInstalledApps(&self, includeSystemApps: bool) -> operit_host_api::HostResult<operit_host_api::AppListData> { panic!("Unexpected host call: listInstalledApps") }
    fn startApp(&self, packageName: &str) -> operit_host_api::HostResult<operit_host_api::AppOperationData> { panic!("Unexpected host call: startApp") }
    fn stopApp(&self, packageName: &str) -> operit_host_api::HostResult<operit_host_api::AppOperationData> { panic!("Unexpected host call: stopApp") }
    fn getNotifications(&self, limit: i32, includeOngoing: bool) -> operit_host_api::HostResult<operit_host_api::NotificationData> { panic!("Unexpected host call: getNotifications") }
    fn getAppUsageTime(&self, packageName: &str, sinceHours: i32, limit: i32, includeSystemApps: bool,) -> operit_host_api::HostResult<operit_host_api::AppUsageTimeResultData> { panic!("Unexpected host call: getAppUsageTime") }
    fn getDeviceLocation(&self, timeout: i32, highAccuracy: bool, includeAddress: bool,) -> operit_host_api::HostResult<operit_host_api::LocationData> { panic!("Unexpected host call: getDeviceLocation") }
    fn getDeviceInfo(&self) -> operit_host_api::HostResult<operit_host_api::DeviceInfoData> { panic!("Unexpected host call: getDeviceInfo") }
    fn captureScreenshot(&self) -> operit_host_api::HostResult<String> { panic!("Unexpected host call: captureScreenshot") }
    fn recognizeText(&self, imagePath: &dyn operit_host_api::FileSource, language: operit_host_api::OCRLanguage, quality: operit_host_api::OCRQuality,) -> operit_host_api::HostResult<String> { panic!("Unexpected host call: recognizeText") }
}

struct RejectingHttpHost;

impl operit_host_api::HttpStreamHost for RejectingHttpHost {
    /// Rejects streams unrelated to local transfer endpoints.
    fn openHttpByteStream(
        &self, _: String, _: operit_host_api::HttpRequestData,
        _: operit_host_api::HttpStreamOpenedCallback,
        _: operit_host_api::HttpStreamChunkCallback,
        _: operit_host_api::HttpStreamClosedCallback,
    ) -> operit_host_api::HostResult<()> { unreachable!() }

    /// Rejects stream shutdown without a corresponding transfer HTTP request.
    fn closeHttpByteStream(&self, _: &str) -> operit_host_api::HostResult<()> { unreachable!() }
}

impl operit_host_api::HttpHost for RejectingHttpHost {
    /// Rejects image delivery unrelated to byte transfers.
    fn imageDelivery(&self) -> operit_host_api::HttpImageDelivery { unreachable!() }

    /// Rejects accidental buffered network reads in local transfer tests.
    fn executeHttpRequest(&self, _: operit_host_api::HttpRequestData)
        -> operit_host_api::HostResult<operit_host_api::HttpResponseData> { unreachable!() }

    /// Rejects accidental bulk downloads in local transfer tests.
    fn downloadFiles(
        &self, _: operit_host_api::HttpDownloadRequest, _: operit_host_api::HttpDownloadControl,
        _: operit_host_api::HttpDownloadProgressCallback,
    ) -> operit_host_api::HostResult<operit_host_api::HttpDownloadResult> { unreachable!() }
}


fn pureRuntime(children: &[(&str, bool)]) -> ToolPkgContainerRuntime {
    ToolPkgContainerRuntime {
        packageName: TEST_ID.into(),
        subpackages: children.iter().map(|(name, enabled)| ToolPkgSubpackageRuntime {
            packageName: (*name).into(), containerPackageName: TEST_ID.into(),
            subpackageId: (*name).into(), entryPath: format!("children/{name}.js"),
            enabledByDefault: *enabled, ..Default::default()
        }).collect(),
        ..Default::default()
    }
}

#[test]
fn member_merge_keeps_old_effective_states_and_disables_new_children() {
    let old = pureRuntime(&[("kept.true", false), ("kept.false", true), ("kept.default", true), ("removed", false)]);
    let new = pureRuntime(&[("kept.true", false), ("kept.false", true), ("kept.default", false), ("added", true)]);
    let record = ExtensionRecord {
        schemaVersion: 1, kind: "package".into(), id: TEST_ID.into(), scope: "space".into(),
        sourceName: "previous.toolpkg".into(), files: BTreeMap::new(),
        settings: json!({
            "members": [TEST_ID, "kept.true", "kept.false", "kept.default", "removed", "added"],
            "enabledNames": [TEST_ID, "kept.true", "removed", "added"],
            "disabledNames": ["kept.false", "removed", "added"],
            "subpackageStates": {"kept.true": true, "kept.false": false, "removed": true, "added": true},
            "order": 17, "builtin": false, "installationId": uuid::Uuid::nil().simple().to_string(),
            "installationScope": "space", "customPreference": {"value": "unchanged"}
        }),
    };
    let original = record.clone();
    let merged = RuntimePackageManager::mergeLocalToolPkgUpdateSettings(&record, &old, &new).unwrap();
    assert_eq!(merged["members"], json!([TEST_ID, "kept.true", "kept.false", "kept.default", "added"]));
    assert_eq!(merged["enabledNames"], json!([TEST_ID, "kept.true"]));
    assert_eq!(merged["disabledNames"], json!(["kept.false"]));
    assert_eq!(merged["subpackageStates"], json!({"kept.true": true, "kept.false": false,
        "kept.default": true, "added": false}));
    assert!(merged.get("installationScope").is_none());
    for field in ["order", "installationId", "builtin", "customPreference"] {
        assert_eq!(merged[field], record.settings[field]);
    }
    assert_eq!(record, original, "merge must not mutate the installed descriptor");
    let mut fixed = new;
    fixed.manifestExtensions.insert("installation_scope".into(), json!("space"));
    assert_eq!(RuntimePackageManager::mergeLocalToolPkgUpdateSettings(&record, &old, &fixed)
        .unwrap()["installationScope"], "space");
}

#[tokio::test(flavor = "current_thread")]
async fn transaction_file_failures_restore_the_old_installation() {
    let _serial = serialTest();
    for phase in ["staging", "staging_after", "backup", "backup_after", "record_backup", "delete", "move", "move_after", "scan", "cache"] {
        let fixture = LocalUpdateFixture::new();
        fixture.install("device");
        let snapshot = InstallationSnapshot::capture(&fixture).await;
        let (oldEngine, oldLease) = fixture.manager.lock().unwrap().toolPkgManager()
            .leaseToolPkgMainExecutionEngine(TEST_ID).unwrap();
        let suffix = fixture.nativeSuffix();
        let fault = match phase {
            "staging" => Fault::new("write", ".staging", false),
            "staging_after" => Fault::new("write", ".staging", true),
            "backup" => Fault::new("write", ".backup", false),
            "backup_after" => Fault::new("write", ".backup", true),
            "record_backup" => Fault::new("write", ".record-backup", false),
            "delete" => Fault::new("delete", &suffix, false),
            "move" => Fault::new("move", &suffix, false),
            "move_after" => Fault::new("move", &suffix, true),
            "scan" => Fault::new("list", "/extensions/device/packages", false),
            "cache" => {
                let manager = fixture.manager.lock().unwrap();
                let runtime = manager.getToolPkgContainerRuntime(TEST_ID).unwrap();
                let cache = manager.ensureToolPkgCache(&runtime).unwrap();
                let key = RuntimeStorePaths::default().runtime_storage_key(&cache).unwrap();
                Fault::new("delete", format!("/{}", key.strip_prefix("runtime/").unwrap()), false)
            }
            _ => unreachable!(),
        };
        fixture.filesystem.faults.arm(fault);
        assert!(fixture.import("replacement.toolpkg", &nextArchive(None)).is_err(), "{phase}");
        if matches!(phase, "staging" | "staging_after" | "backup" | "backup_after" | "record_backup") {
            let current = fixture.manager.lock().unwrap().toolPkgManager()
                .findToolPkgExecutionEngine(&format!("toolpkg_main:{TEST_ID}"), TEST_ID).unwrap();
            assert!(Arc::ptr_eq(&oldEngine, &current), "pre-replacement {phase} must retain the old realm");
        }
        snapshot.assertRollback(&fixture).await;
        let replacement = fixture.manager.lock().unwrap().toolPkgManager()
            .findToolPkgExecutionEngine(&format!("toolpkg_main:{TEST_ID}"), TEST_ID).unwrap();
        drop(oldLease);
        let current = fixture.manager.lock().unwrap().toolPkgManager()
            .findToolPkgExecutionEngine(&format!("toolpkg_main:{TEST_ID}"), TEST_ID).unwrap();
        assert!(Arc::ptr_eq(&replacement, &current), "dropping an old lease must not release the current realm");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn final_registration_and_scan_failures_restore_old_code() {
    let _serial = serialTest();
    for registration in [1, 2] {
        let fixture = LocalUpdateFixture::new();
        fixture.install("device");
        let snapshot = InstallationSnapshot::capture(&fixture).await;
        fixture.provider.failRegistration(registration);
        assert!(fixture.import("replacement.toolpkg", &nextArchive(None)).is_err(), "registration {registration}");
        assert_eq!(fixture.provider.failAt.load(Ordering::SeqCst), 0, "fault was not exercised");
        snapshot.assertRollback(&fixture).await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn invalid_and_partial_staging_never_replace_the_owned_archive() {
    let _serial = serialTest();
    let fixture = LocalUpdateFixture::new();
    fixture.install("device");
    let snapshot = InstallationSnapshot::capture(&fixture).await;
    let mut missingMain = packageEntries(TEST_ID, None, "2.0.0", "v2", &[("kept.child", true)]);
    missingMain.retain(|(name, _)| name != "main.js");
    let mut brokenRegistration = packageEntries(TEST_ID, None, "2.0.0", "v2", &[("kept.child", true)]);
    brokenRegistration.iter_mut().find(|(name, _)| name == "main.js").unwrap().1 =
        "exports.registerToolPkg = function() { throw new Error('fixture registration failure'); };".into();
    let mut partial = packageEntries(TEST_ID, None, "2.0.0", "v2", &[("kept.child", true)]);
    let manifest = &mut partial.iter_mut().find(|(name, _)| name == "manifest.json").unwrap().1;
    let mut parsed: Value = serde_json::from_str(manifest).unwrap();
    parsed["subpackages"].as_array_mut().unwrap().push(json!({"id":"missing.child","entry":"missing.js"}));
    *manifest = parsed.to_string();
    for invalid in [b"not a ZIP".to_vec(), archiveEntries(missingMain), archiveEntries(brokenRegistration), archiveEntries(partial)] {
        assert!(fixture.import("invalid.toolpkg", &invalid).is_err());
        snapshot.assertRollback(&fixture).await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn partial_descriptor_write_is_restored_from_original_bytes() {
    let _serial = serialTest();
    for scope in ["device", "space"] {
        for partial in [false, true] {
            let fixture = LocalUpdateFixture::new();
            fixture.install(scope);
            let snapshot = InstallationSnapshot::capture(&fixture).await;
            let mut fault = Fault::new("store_write", fixture.descriptorKey(), true);
            fault.partial = partial;
            fixture.storage.faults.arm(fault);
            assert!(fixture.import("replacement.toolpkg", &nextArchive(None)).is_err());
            snapshot.assertRollback(&fixture).await;
            if scope == "space" {
                assert_eq!(fixture.latestFileBytes(&fixture.archiveKey()), snapshot.archive);
                assert_eq!(fixture.latestFileBytes(&fixture.descriptorKey()), snapshot.descriptor);
            }
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn space_archive_write_failure_restores_and_publishes_previous_bytes() {
    let _serial = serialTest();
    for after in [false, true] {
        let fixture = LocalUpdateFixture::new();
        fixture.install("space");
        let snapshot = InstallationSnapshot::capture(&fixture).await;
        fixture.storage.faults.arm(Fault::new("store_write", fixture.archiveKey(), after));
        assert!(fixture.import("replacement.toolpkg", &nextArchive(None)).is_err());
        snapshot.assertRollback(&fixture).await;
        assert_eq!(fixture.latestFileBytes(&fixture.archiveKey()), snapshot.archive);
        assert_eq!(fixture.latestFileBytes(&fixture.descriptorKey()), snapshot.descriptor);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn cross_scope_update_is_rejected_before_registration_or_staging() {
    let _serial = serialTest();
    for (scope, other) in [("device", "space"), ("space", "device")] {
        let fixture = LocalUpdateFixture::new();
        fixture.install(scope);
        let snapshot = InstallationSnapshot::capture(&fixture).await;
        fixture.provider.failRegistration(0);
        fixture.filesystem.faults.mutations.lock().unwrap().clear();
        fixture.storage.faults.mutations.lock().unwrap().clear();
        assert!(fixture.import("replacement.toolpkg", &nextArchive(Some(other))).is_err());
        assert_eq!(fixture.provider.registrations.load(Ordering::SeqCst), 0);
        assert!(fixture.filesystem.faults.mutations.lock().unwrap().is_empty());
        assert!(fixture.storage.faults.mutations.lock().unwrap().is_empty());
        snapshot.assertRollback(&fixture).await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn disabled_parent_and_restarts_do_not_enable_new_children() {
    let _serial = serialTest();
    let fixture = LocalUpdateFixture::new();
    fixture.install("device");
    let snapshot = InstallationSnapshot::capture(&fixture).await;
    fixture.manager.lock().unwrap().disablePackage(TEST_ID);
    fixture.import("replacement.toolpkg", &nextArchive(None)).unwrap();
    {
        let manager = fixture.manager.lock().unwrap();
        assert!(!manager.isPackageEnabled(TEST_ID));
        assert!(!manager.isPackageEnabled("new.child"));
    }
    fixture.manager.lock().unwrap().enablePackage(TEST_ID);
    assert!(fixture.manager.lock().unwrap().isPackageEnabled("kept.child"));
    assert!(!fixture.manager.lock().unwrap().isPackageEnabled("new.child"));
    {
        let context = fixture.handler.getContext();
        let restarted = AIToolHandler::new(context, fixture.handler.runtimeDependencies());
        let shared = restarted.getOrCreatePackageManager();
        let mut manager = shared.lock().unwrap();
        assert!(manager.claimInitialPackageRegistryLoad().unwrap());
        manager.loadAvailablePackages();
        assert!(manager.isPackageEnabled(TEST_ID));
        assert!(!manager.isPackageEnabled("new.child"));
        let shutdown = manager.executionEngineShutdown();
        drop(manager);
        shutdown();
    }
    snapshot.assertData(&fixture);
}

#[tokio::test(flavor = "current_thread")]
async fn raw_market_artifact_cannot_use_local_replacement() {
    let _serial = serialTest();
    let fixture = LocalUpdateFixture::new();
    fixture.install("device");
    let snapshot = InstallationSnapshot::capture(&fixture).await;
    let bytes = nextArchive(None);
    let path = fixture.root.join("artifacts/market-raw.toolpkg");
    std::fs::write(&path, &bytes).unwrap();
    let result = fixture.manager.lock().unwrap().addMarketArtifactFile(
        path.to_string_lossy().into_owned(), sha256Hex(&bytes),
    );
    assert!(!result.starts_with("Successfully imported"), "raw market must retain its old duplicate-name rejection");
    snapshot.assertRollback(&fixture).await;
    fixture.import("same-bytes-local.toolpkg", &bytes).unwrap();
    assert_eq!(fixture.probe(json!({})).await["codeVersion"], "v2");
}

#[tokio::test(flavor = "current_thread")]
async fn failed_restoration_reports_both_errors_and_keeps_recovery_backups() {
    let _serial = serialTest();
    let fixture = LocalUpdateFixture::new();
    fixture.install("device");
    let snapshot = InstallationSnapshot::capture(&fixture).await;
    fixture.filesystem.faults.arm(Fault::new("move", fixture.nativeSuffix(), true));
    fixture.filesystem.faults.arm(Fault::new("write", fixture.nativeSuffix(), false));
    let error = fixture.import("replacement.toolpkg", &nextArchive(None)).unwrap_err();
    assert!(error.contains("injected move failure"), "{error}");
    assert!(error.contains("injected write failure"), "{error}");
    assert!(error.contains("rollback failed"), "{error}");
    let backups = fixture.temporaryFiles();
    assert!(backups.iter().any(|path| path.ends_with(".backup")));
    assert!(backups.iter().any(|path| path.ends_with(".record-backup")));
    assert!(!backups.iter().any(|path| path.ends_with(".staging")));
    snapshot.assertData(&fixture);
}

fn bundledAsset(name: &'static str, bytes: Vec<u8>) -> &'static [RuntimePluginAsset] {
    Box::leak(vec![RuntimePluginAsset {
        name, bytes: Box::leak(bytes.into_boxed_slice()), sha256: "local-update-test-asset",
    }].into_boxed_slice())
}

#[tokio::test(flavor = "current_thread")]
async fn builtin_and_customized_owners_keep_their_existing_import_flow() {
    let _serial = serialTest();
    let bytes = packageArchive(TEST_ID, Some("device"), "1.0.0", "v1", &[]);
    let fixture = LocalUpdateFixture::withAssets(bundledAsset("builtin-fixture.toolpkg", bytes), &[]);
    let before = fixture.record();
    assert_eq!(before.settings["builtin"], true);
    let preview = RuntimePackageManager::readToolPkgImportManifest(&nextArchive(None)).unwrap();
    assert!(fixture.manager.lock().unwrap().localToolPkgUpdateTarget(&preview).unwrap().is_none());
    assert_eq!(fixture.record(), before, "the ordinary replacement gate must not customize a builtin");
    drop(fixture);

    let fixture = LocalUpdateFixture::new();
    fixture.install("device");
    let mut settings = fixture.record().settings;
    settings["customized"] = json!(true);
    ExtensionStore::new(fixture.storage.clone()).setSettings("package", TEST_ID, settings).unwrap();
    let before = fixture.record();
    assert!(fixture.manager.lock().unwrap().localToolPkgUpdateTarget(&preview).unwrap().is_none());
    assert_eq!(fixture.record(), before);
}

#[tokio::test(flavor = "current_thread")]
async fn bundled_source_cannot_be_replaced_by_local_import() {
    let _serial = serialTest();
    let bytes = packageArchive(TEST_ID, Some("device"), "1.0.0", "v1", &[("kept.child", true)]);
    let fixture = LocalUpdateFixture::withAssets(&[], bundledAsset("bundled-fixture.toolpkg", bytes));
    let imported = fixture.manager.lock().unwrap().importBundledExternalToolPkgContainer(TEST_ID);
    assert!(imported.starts_with("Successfully imported"), "{imported}");
    fixture.manager.lock().unwrap().enablePackage(TEST_ID);
    let snapshot = InstallationSnapshot::capture(&fixture).await;
    let error = fixture.import("replacement.toolpkg", &nextArchive(None)).unwrap_err();
    assert!(error.contains("Bundled"), "{error}");
    snapshot.assertRollback(&fixture).await;
}

#[tokio::test(flavor = "current_thread")]
async fn market_and_mismatched_runtime_owners_are_rejected_without_staging() {
    let _serial = serialTest();
    for mismatch in ["market", "container_name", "source_path"] {
        let fixture = LocalUpdateFixture::new();
        fixture.install("device");
        let snapshot = InstallationSnapshot::capture(&fixture).await;
        let mut manager = fixture.manager.lock().unwrap();
        let mut containers = manager.toolPkgManager().getToolPkgContainerRuntimeMap();
        let children = manager.toolPkgManager().getToolPkgSubpackageRuntimeMap();
        let original = containers.clone();
        let changed = containers.get_mut(TEST_ID).unwrap();
        match mismatch {
            "market" => changed.sourceType = ToolPkgSourceType::MARKET,
            "container_name" => changed.packageName = "other.container".into(),
            "source_path" => changed.sourcePath = "/operitapp/data/extensions/device/packages/other.toolpkg".into(),
            _ => unreachable!(),
        }
        manager.toolPkgManagerMut().replaceRuntimeMaps(containers, children.clone());
        drop(manager);
        fixture.provider.failRegistration(0);
        fixture.filesystem.faults.mutations.lock().unwrap().clear();
        assert!(fixture.import("replacement.toolpkg", &nextArchive(None)).is_err(), "{mismatch}");
        assert_eq!(fixture.provider.registrations.load(Ordering::SeqCst), 0);
        assert!(fixture.filesystem.faults.mutations.lock().unwrap().is_empty());
        assert_eq!(fixture.record(), snapshot.record);
        snapshot.assertData(&fixture);
        fixture.manager.lock().unwrap().toolPkgManagerMut().replaceRuntimeMaps(original, children);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn legacy_external_market_seal_cannot_use_ordinary_replacement() {
    let _serial = serialTest();
    let fixture = LocalUpdateFixture::new();
    fixture.install("device");
    let before = fixture.record();
    let archive = fixture.storage.readBytes(&fixture.archiveKey()).unwrap();
    let identity = decodeMarketToolPkgInstallationId(before.settings["installationId"].as_str().unwrap()).unwrap();
    let sealed = ToolPkgProtection::attachMarketInstallSeal(&archive, &identity).unwrap();
    fixture.storage.writeBytes(&fixture.archiveKey(), &sealed).unwrap();
    fixture.provider.failRegistration(0);
    assert!(fixture.import("replacement.toolpkg", &nextArchive(None)).is_err());
    assert_eq!(fixture.provider.registrations.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.storage.readBytes(&fixture.archiveKey()).unwrap(), sealed);
    assert_eq!(fixture.record(), before);
}

#[tokio::test(flavor = "current_thread")]
async fn missing_archive_and_directory_sources_cannot_be_updated() {
    let _serial = serialTest();
    for directory in [false, true] {
        let fixture = LocalUpdateFixture::new();
        fixture.install("device");
        let snapshot = InstallationSnapshot::capture(&fixture).await;
        let key = fixture.archiveKey();
        fixture.storage.delete(&key, false).unwrap();
        if directory {
            let native = fixture.storage.inner.runtimeRootDir().unwrap()
                .join(key.strip_prefix("runtime/").unwrap());
            std::fs::create_dir_all(native).unwrap();
        }
        fixture.provider.failRegistration(0);
        assert!(fixture.import("replacement.toolpkg", &nextArchive(None)).is_err());
        assert_eq!(fixture.provider.registrations.load(Ordering::SeqCst), 0);
        assert_eq!(fixture.record(), snapshot.record);
        snapshot.assertData(&fixture);
        assert!(fixture.temporaryFiles().is_empty());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn standalone_owner_cannot_be_replaced_by_a_toolpkg() {
    let _serial = serialTest();
    let fixture = LocalUpdateFixture::new();
    let script = format!("/* METADATA {{\"name\":\"{TEST_ID}\",\"tools\":[]}} */");
    fixture.import("standalone.js", script.as_bytes()).unwrap();
    let before = fixture.record();
    let key = fixture.archiveKey();
    fixture.provider.failRegistration(0);
    assert!(fixture.import("replacement.toolpkg", &nextArchive(None)).is_err());
    assert_eq!(fixture.provider.registrations.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.record(), before);
    assert_eq!(fixture.storage.readBytes(&key).unwrap(), script.as_bytes());
}

#[tokio::test(flavor = "current_thread")]
async fn duplicated_scope_records_are_not_merged_or_overwritten() {
    let _serial = serialTest();
    let fixture = LocalUpdateFixture::new();
    fixture.install("device");
    let snapshot = InstallationSnapshot::capture(&fixture).await;
    let originalKey = fixture.descriptorKey();
    let originalArchiveKey = fixture.archiveKey();
    let mut duplicate = snapshot.record.clone();
    duplicate.scope = "space".into();
    duplicate.sourceName = "shadow.toolpkg".into();
    duplicate.settings["installationScope"] = json!("space");
    let shadowKey = operit_store::ExtensionStore::recordPath("package", TEST_ID, "space").unwrap();
    let shadow = serde_json::to_vec(&duplicate).unwrap();
    fixture.storage.writeBytes(&shadowKey, &shadow).unwrap();
    fixture.provider.failRegistration(0);
    assert!(fixture.import("replacement.toolpkg", &nextArchive(None)).is_err());
    assert_eq!(fixture.provider.registrations.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.storage.readBytes(&originalKey).unwrap(), snapshot.descriptor);
    assert_eq!(fixture.storage.readBytes(&shadowKey).unwrap(), shadow);
    assert_eq!(fixture.storage.readBytes(&originalArchiveKey).unwrap(), snapshot.archive);
    snapshot.assertData(&fixture);
}

#[tokio::test(flavor = "current_thread")]
async fn same_version_size_and_mtime_update_refreshes_resources_and_syncs_bytes() {
    let _serial = serialTest();
    for scope in ["device", "space"] {
        let fixture = LocalUpdateFixture::new();
        fixture.install(scope);
        let snapshot = InstallationSnapshot::capture(&fixture).await;
        let bytes = packageArchive(TEST_ID, Some(scope), "1.0.0", "v2", &[("kept.child", true)]);
        assert_eq!(bytes.len(), snapshot.archive.len());
        let record = fixture.record();
        let archivePath = RuntimeStorePaths::default().runtime_storage_path(&fixture.archiveKey());
        let info = fixture.manager.lock().unwrap().filesystem.fileInfo(&vfsPathString(&archivePath)).unwrap();
        *fixture.filesystem.frozen.lock().unwrap() = Some((fixture.nativeSuffix(), info.size, info.lastModified));
        let stamp = fixture.storage.inner.modifiedAt(&fixture.archiveKey()).unwrap();
        fixture.storage.frozen.lock().unwrap().insert(fixture.archiveKey(), stamp);
        let (oldEngine, lease) = fixture.manager.lock().unwrap().toolPkgManager()
            .leaseToolPkgMainExecutionEngine(TEST_ID).unwrap();
        fixture.import("same-version.toolpkg", &bytes).unwrap();
        let read = fixture.probe(json!({})).await;
        assert_eq!(read["preference"], "saved preference");
        assert_eq!(read["codeVersion"], "v2", "metadata signatures alone cannot invalidate old resources");
        let current = fixture.manager.lock().unwrap().toolPkgManager()
            .findToolPkgExecutionEngine(&format!("toolpkg_main:{TEST_ID}"), TEST_ID).unwrap();
        assert!(!Arc::ptr_eq(&oldEngine, &current));
        drop(lease);
        assert_eq!(fixture.record().sourceName, record.sourceName);
        snapshot.assertData(&fixture);
        assert!(fixture.temporaryFiles().is_empty());
        if scope == "space" { assert_eq!(fixture.latestFileBytes(&fixture.archiveKey()), bytes); }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn rollback_does_not_deliver_uncommitted_new_runtime_notifications() {
    let _serial = serialTest();
    let fixture = LocalUpdateFixture::build(&[], &[], true);
    fixture.install("device");
    let snapshot = InstallationSnapshot::capture(&fixture).await;
    fixture.scheduler.runNotificationsInReverse();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let notifications = observed.clone();
    let registry = fixture.manager.lock().unwrap().toolPkgManager().clone();
    fixture.manager.lock().unwrap().addToolPkgRuntimeChangeListener(Arc::new(move |containers| {
        for runtime in containers.into_iter().filter(|runtime| runtime.packageName == TEST_ID) {
            notifications.lock().unwrap().push(runtime.version);
        }
    }));
    observed.lock().unwrap().clear();
    *fixture.filesystem.failAfterPublication.lock().unwrap() =
        Some((fixture.nativeSuffix(), registry, "2.0.0".into()));
    let error = fixture.import("replacement.toolpkg", &nextArchive(None)).unwrap_err();
    assert!(error.contains("injected post-publication validation failure"), "{error}");
    assert!(fixture.filesystem.failAfterPublication.lock().unwrap().is_none(), "validation fault did not run");
    fixture.scheduler.runNotificationsInReverse();
    assert_eq!(*observed.lock().unwrap(), vec!["1.0.0".to_string()],
        "a failed update must not queue v2 even when the observer executes older tasks last");
    snapshot.assertRollback(&fixture).await;
}

#[tokio::test(flavor = "current_thread")]
async fn removed_children_drop_state_but_keep_configuration_namespaces() {
    let _serial = serialTest();
    let fixture = LocalUpdateFixture::new();
    fixture.install("device");
    let mut settings = fixture.record().settings;
    settings["enabledNames"] = json!([TEST_ID, "kept.child", "new.child"]);
    settings["disabledNames"] = json!(["kept.child", "new.child"]);
    settings["subpackageStates"] = json!({"kept.child": true, "new.child": true});
    ExtensionStore::new(fixture.storage.clone()).setSettings("package", TEST_ID, settings).unwrap();
    fixture.manager.lock().unwrap().loadAvailablePackages();
    let snapshot = InstallationSnapshot::capture(&fixture).await;
    let bytes = packageArchive(TEST_ID, None, "2.0.0", "v2", &[("new.child", true)]);
    fixture.import("without-old-child.toolpkg", &bytes).unwrap();
    let record = fixture.record();
    assert_eq!(record.settings["members"], json!([TEST_ID, "new.child"]));
    assert_eq!(record.settings["enabledNames"], json!([TEST_ID]));
    assert_eq!(record.settings["disabledNames"], json!([]));
    assert_eq!(record.settings["subpackageStates"], json!({"new.child": false}));
    assert!(!fixture.manager.lock().unwrap().isPackageEnabled("new.child"));
    assert!(fixture.manager.lock().unwrap().getToolPkgContainerRuntime(TEST_ID).unwrap()
        .subpackages.iter().all(|child| child.packageName != "kept.child"));
    snapshot.assertData(&fixture);
    assert_eq!(fixture.probe(json!({})).await["codeVersion"], "v2");
}
