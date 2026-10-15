use super::*;
use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeResultData, DeviceInfoData, LocationData,
    NotificationData, OCRLanguage, OCRQuality, SystemNotificationRequest, SystemOperationHost,
    SystemSettingData,
};

/// Supplies a deterministic locale and rejects unrelated system side effects.
struct TestSystemLocale;

macro_rules! unsupported_system_operations {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $result:ty;)*) => {
        $(/// Rejects system operations outside this test fixture.
        #[allow(non_snake_case, unused_variables)]
        fn $name(&self, $($arg: $ty),*) -> HostResult<$result> {
            panic!("unexpected system operation: {}", stringify!($name));
        })*
    };
}

impl SystemOperationHost for TestSystemLocale {
    /// Returns the fixed locale used by embedded package registration.
    #[allow(non_snake_case)]
    fn getSystemLanguageCode(&self) -> HostResult<String> {
        Ok("zh-CN".to_string())
    }

    unsupported_system_operations! {
        sendNotification(request: &SystemNotificationRequest) -> ();
        modifySystemSetting(namespace: &str, setting: &str, value: &str) -> SystemSettingData;
        getSystemSetting(namespace: &str, setting: &str) -> SystemSettingData;
        installApp(path: &str) -> AppOperationData;
        uninstallApp(packageName: &str) -> AppOperationData;
        listInstalledApps(includeSystemApps: bool) -> AppListData;
        startApp(packageName: &str) -> AppOperationData;
        stopApp(packageName: &str) -> AppOperationData;
        getNotifications(limit: i32, includeOngoing: bool) -> NotificationData;
        getAppUsageTime(packageName: &str, sinceHours: i32, limit: i32, includeSystemApps: bool) -> AppUsageTimeResultData;
        getDeviceLocation(timeout: i32, highAccuracy: bool, includeAddress: bool) -> LocationData;
        getDeviceInfo() -> DeviceInfoData;
        captureScreenshot() -> String;
        recognizeText(imagePath: &str, language: OCRLanguage, quality: OCRQuality) -> String;
    }
}

/// Runs the opt-in real-package latency probe without starting Flutter or creating user data.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "headless real-package latency probe"]
async fn package_ui_headless_latency_probe() {
    use operit_plugin_sdk::toolpkg::ToolPkgPackageService::ToolPkgPackageHost;
    use std::time::Instant;
    let _test_guard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    let storage = register_test_runtime_roots();
    let mut hosts = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
    hosts.runtimeStorageHost = Some(storage.clone());
    hosts.runtimeSqliteHost = Some(storage);
    hosts.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
    hosts.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    hosts.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    configure_package_hosts(&mut hosts);
    let app = OperitApplication::newWithContext(hosts);
    let packages = app.packageManager();
    packages.lock().unwrap().loadAvailablePackages();
    let proxy = LocalCoreProxy::new(app);
    let package = "com.operit.character_cards";
    {
        let manager = packages.lock().unwrap();
        println!(
            "UI_PROBE packages={:?} issues={:?}",
            ToolPkgPackageHost::toolPkgContainersInternal(&*manager)
                .keys()
                .collect::<Vec<_>>(),
            manager.getToolPkgLoadIssues()
        );
    }
    let runtime = packages
        .lock()
        .unwrap()
        .getToolPkgContainerRuntime(package)
        .expect("real character cards package must load");
    let module = runtime
        .uiModules
        .iter()
        .find(|module| module.screen == "dist/ui/main/index.ui.js")
        .expect("character cards main route must exist");
    let target = LocalCoreProxy::generatedTargetForSchema("application.packageManager").unwrap();
    let mut samples: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    for iteration in 0..21 {
        for phase in [
            "catalog-copy",
            "one-container",
            "enabled-names",
            "screen-path",
            "script-read",
            "resource-read",
        ] {
            let start = Instant::now();
            match phase {
                "script-read" => {
                    let pending = packages
                        .lock()
                        .unwrap()
                        .getToolPkgComposeDslScript(package, Some(&module.id));
                    assert!(pending.await.is_some());
                }
                _ => {
                    let manager = packages.lock().unwrap();
                    match phase {
                        "catalog-copy" => {
                            std::hint::black_box(ToolPkgPackageHost::toolPkgContainersInternal(
                                &*manager,
                            ));
                        }
                        "one-container" => {
                            assert!(manager.getToolPkgContainerRuntime(package).is_some());
                        }
                        "enabled-names" => {
                            std::hint::black_box(manager.getEnabledPackageNames());
                        }
                        "screen-path" => {
                            assert_eq!(
                                manager
                                    .getToolPkgComposeDslScreenPath(package, Some(&module.id))
                                    .as_deref(),
                                Some(module.screen.as_str())
                            );
                        }
                        "resource-read" => {
                            assert!(ToolPkgPackageHost::readToolPkgResourceBytes(
                                &*manager,
                                &runtime,
                                &module.screen
                            )
                            .is_some());
                        }
                        _ => unreachable!(),
                    }
                }
            }
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            if iteration == 0 {
                println!("UI_PROBE first phase={phase} ms={ms:.3}");
            } else {
                samples.entry(phase).or_default().push(ms);
            }
        }
        let start = Instant::now();
        let response = CoreLinkSharedClient::call(
            &proxy,
            CoreCallRequest::new(
                format!("probe-script-{iteration}"),
                target,
                "getToolPkgComposeDslScript",
                toCoreValue(json!({"containerPackageName": package, "uiModuleId": module.id}))
                    .unwrap(),
            ),
        )
        .await;
        let script: Option<String> = fromCoreValue(response.result.expect("script proxy call must succeed")).unwrap();
        assert!(script.is_some(), "script proxy must return the registered entry");
        if iteration > 0 {
            samples
                .entry("proxy-script")
                .or_default()
                .push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    for (phase, mut values) in samples {
        values.sort_by(f64::total_cmp);
        println!(
            "UI_PROBE phase={phase} n={} p50Ms={:.3} p95Ms={:.3}",
            values.len(),
            values[values.len() / 2],
            values[(values.len() * 95).div_ceil(100) - 1]
        );
    }
}

use operit_host_api::{
    FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
    GrepCodeResult, HostEnvironmentDescriptor,
};
use std::sync::{mpsc, Condvar};
use std::time::{Duration, Instant};

/// Stops one selected host operation until the test explicitly releases it.
struct ResourceGate {
    operation: Mutex<Option<&'static str>>,
    released: Mutex<bool>,
    changed: Condvar,
    entered: mpsc::Sender<()>,
    main_reads: std::sync::atomic::AtomicUsize,
}

impl ResourceGate {
    /// Arms exactly one operation on the real character-card screen resource.
    fn arm(&self, operation: &'static str) {
        *self.released.lock().unwrap() = false;
        *self.operation.lock().unwrap() = Some(operation);
    }

    /// Blocks the selected operation without holding fixture state needed by other requests.
    fn intercept(&self, operation: &str, path: &str) {
        if operation == "read" && std::path::Path::new(path).ends_with("dist/main.js") {
            self.main_reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        if !std::path::Path::new(path).ends_with("dist/ui/main/index.ui.js") {
            return;
        }
        {
            let mut armed = self.operation.lock().unwrap();
            if *armed != Some(operation) {
                return;
            }
            *armed = None;
        }
        self.entered.send(()).unwrap();
        let released = self.released.lock().unwrap();
        let (released, timeout) = self
            .changed
            .wait_timeout_while(released, Duration::from_secs(15), |released| !*released)
            .unwrap();
        assert!(
            *released && !timeout.timed_out(),
            "resource gate was not released"
        );
    }

    /// Releases the pending resource operation even during assertion unwinding.
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.changed.notify_all();
    }
}

/// Ensures a failed assertion cannot leave the filesystem worker blocked.
struct ReleaseGate(Arc<ResourceGate>);
impl Drop for ReleaseGate {
    /// Opens the gate when its test scope ends.
    fn drop(&mut self) {
        self.0.release();
    }
}

/// Delegates actual filesystem work while allowing deterministic I/O contention.
struct GatedFileSystem {
    inner: PosixFileSystemHost,
    gate: Arc<ResourceGate>,
}

macro_rules! delegate_filesystem {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $result:ty;)*) => {
        $(/// Delegates an uninstrumented operation to the real filesystem host.
        #[allow(non_snake_case)]
        fn $name(&self, $($arg: $ty),*) -> $result {
            self.inner.$name($($arg),*)
        })*
    };
}

impl FileSystemHost for GatedFileSystem {
    /// Reads the real file after passing the controlled read gate.
    fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.gate.intercept("read", path);
        self.inner.readFileBytes(path)
    }
    /// Writes extracted archive bytes after passing the controlled extraction gate.
    fn writeFileBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.gate.intercept("write", path);
        self.inner.writeFileBytes(path, content)
    }
    delegate_filesystem! {
        envLabel() -> &str;
        environmentDescriptor() -> HostEnvironmentDescriptor;
        validatePath(path: &str, paramName: &str) -> HostResult<()>;
        listFiles(path: &str) -> HostResult<Vec<FileEntry>>;
        readFile(path: &str) -> HostResult<String>;
        readFileWithLimit(path: &str, maxBytes: usize) -> HostResult<String>;
        writeFile(path: &str, content: &str, append: bool) -> HostResult<()>;
        deleteFile(path: &str, recursive: bool) -> HostResult<()>;
        fileExists(path: &str) -> HostResult<FileExistence>;
        moveFile(source: &str, destination: &str) -> HostResult<()>;
        copyFile(source: &str, destination: &str, recursive: bool) -> HostResult<()>;
        makeDirectory(path: &str, createParents: bool) -> HostResult<()>;
        findFiles(request: FindFilesRequest) -> HostResult<Vec<String>>;
        fileInfo(path: &str) -> HostResult<FileInfo>;
        grepCode(request: GrepCodeRequest) -> HostResult<GrepCodeResult>;
        zipFiles(source: &str, destination: &str) -> HostResult<()>;
        unzipFiles(source: &str, destination: &str) -> HostResult<()>;
        openFile(path: &str) -> HostResult<()>;
        shareFile(path: &str, title: &str) -> HostResult<()>;
    }
}

/// Constructs the production proxy request for package-manager operations.
fn package_request(method: &str, args: Value) -> CoreCallRequest {
    CoreCallRequest::new(
        method,
        LocalCoreProxy::generatedTargetForSchema("application.packageManager").unwrap(),
        method,
        toCoreValue(args).unwrap(),
    )
}

/// Runs a non-Send proxy future on its own thread, matching independently scheduled callers.
fn start_package_call(
    proxy: LocalCoreProxy,
    request: CoreCallRequest,
) -> std::thread::JoinHandle<operit_link::CoreCallResponse> {
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(CoreLinkSharedClient::call(&proxy, request))
    })
}

/// Proves extraction and session I/O release the manager, including closure during resource loading.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocked_ui_resources_do_not_block_engines_or_resurrect_closed_sessions() {
    let _test_guard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    let storage = register_test_runtime_roots();
    let (entered, entering) = mpsc::channel();
    let gate = Arc::new(ResourceGate {
        operation: Mutex::new(None),
        released: Mutex::new(false),
        changed: Condvar::new(),
        entered,
        main_reads: std::sync::atomic::AtomicUsize::new(0),
    });
    let _release = ReleaseGate(gate.clone());
    let mut hosts = HostManager::withFileSystemHost(Arc::new(GatedFileSystem {
        inner: PosixFileSystemHost::new(),
        gate: gate.clone(),
    }));
    hosts.runtimeStorageHost = Some(storage.clone());
    hosts.runtimeSqliteHost = Some(storage);
    hosts.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
    hosts.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    hosts.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    configure_package_hosts(&mut hosts);
    let app = OperitApplication::newWithContext(hosts);
    let packages = app.packageManager();
    packages.lock().unwrap().loadAvailablePackages();
    let package = "com.operit.character_cards";
    let runtime = packages
        .lock()
        .unwrap()
        .getToolPkgContainerRuntime(package)
        .unwrap();
    let module = runtime
        .uiModules
        .iter()
        .find(|module| module.screen == "dist/ui/main/index.ui.js")
        .unwrap();
    let proxy = LocalCoreProxy::new(app);

    gate.arm("write");
    let script = start_package_call(
        proxy.clone(),
        package_request(
            "getToolPkgComposeDslScript",
            json!({
                "containerPackageName": package, "uiModuleId": module.id,
            }),
        ),
    );
    entering
        .recv_timeout(Duration::from_secs(10))
        .expect("cold script load must reach archive extraction");
    assert!(
        packages.try_lock().is_ok(),
        "archive extraction must release the package manager"
    );
    let started = Instant::now();
    let acquired = tokio::time::timeout(
        Duration::from_secs(5),
        CoreLinkSharedClient::call(
            &proxy,
            package_request(
                "acquireToolPkgExecutionEngine",
                json!({"contextKey": "blocked-resources", "containerPackageName": package}),
            ),
        ),
    )
    .await
    .expect("engine acquisition must finish while extraction is blocked");
    assert!(acquired.result.is_ok(), "{:?}", acquired.result);
    println!(
        "UI_CONTENTION acquireWhileExtractionBlockedMs={:.3}",
        started.elapsed().as_secs_f64() * 1000.0
    );
    gate.release();
    let result = script.join().unwrap().result.unwrap();
    let script: Option<String> = fromCoreValue(result).unwrap();
    let expected = {
        let manager = packages.lock().unwrap();
        operit_plugin_sdk::toolpkg::ToolPkgPackageService::ToolPkgPackageService::new(&*manager)
            .getToolPkgComposeDslScript(package, Some(&module.id))
    };
    assert_eq!(
        script, expected,
        "owned resource reads must preserve script bytes and export binding"
    );
    assert!(script.is_some());

    gate.arm("read");
    let session = start_package_call(
        proxy.clone(),
        package_request(
            "openComposeDslSession",
            json!({
                "contextKey": "blocked-resources", "containerPackageName": package,
            }),
        ),
    );
    entering
        .recv_timeout(Duration::from_secs(10))
        .expect("session must reach cold resource snapshot loading");
    assert!(
        packages.try_lock().is_ok(),
        "session resource loading must release the package manager"
    );
    let started = Instant::now();
    let acquired = CoreLinkSharedClient::call(
        &proxy,
        package_request(
            "acquireToolPkgExecutionEngine",
            json!({
                "contextKey": "other-page", "containerPackageName": package,
            }),
        ),
    )
    .await;
    assert!(acquired.result.is_ok(), "{:?}", acquired.result);
    println!(
        "UI_CONTENTION acquireWhileSessionReadBlockedMs={:.3}",
        started.elapsed().as_secs_f64() * 1000.0
    );
    let released = CoreLinkSharedClient::call(
        &proxy,
        package_request(
            "releaseToolPkgExecutionEngine",
            json!({
                "contextKey": "blocked-resources", "containerPackageName": package,
            }),
        ),
    )
    .await;
    assert!(released.result.is_ok());
    gate.release();
    assert!(
        session.join().unwrap().result.is_err(),
        "closing a context during I/O must prevent session publication"
    );

    // A captured open must not attach to a replacement engine with the same key.
    let stale_open = packages
        .lock()
        .unwrap()
        .openComposeDslSession("other-page", package);
    let args = json!({"contextKey": "other-page", "containerPackageName": package});
    assert!(CoreLinkSharedClient::call(
        &proxy,
        package_request("releaseToolPkgExecutionEngine", args.clone())
    )
    .await
    .result
    .is_ok());
    assert!(CoreLinkSharedClient::call(
        &proxy,
        package_request("acquireToolPkgExecutionEngine", args.clone())
    )
    .await
    .result
    .is_ok());
    assert!(
        stale_open.await.is_err(),
        "stale opens must not attach to a replacement owner"
    );
    let first = CoreLinkSharedClient::call(
        &proxy,
        package_request("openComposeDslSession", args.clone()),
    )
    .await
    .result
    .unwrap();
    let reopened = CoreLinkSharedClient::call(
        &proxy,
        package_request("openComposeDslSession", args.clone()),
    )
    .await
    .result
    .unwrap();
    assert_eq!(
        first, reopened,
        "the same live context must retain its session identity"
    );
    assert!(CoreLinkSharedClient::call(
        &proxy,
        package_request("releaseToolPkgExecutionEngine", args)
    )
    .await
    .result
    .is_ok());
}

/// Provides the required HTTP capability while rejecting network use in resource tests.
struct NoNetworkHttpHost;
impl operit_host_api::HttpHost for NoNetworkHttpHost {
    /// Declares buffered image delivery for tool registration.
    fn imageDelivery(&self) -> operit_host_api::HttpImageDelivery {
        operit_host_api::HttpImageDelivery::Bytes
    }
    /// Rejects unexpected network requests.
    fn executeHttpRequest(
        &self,
        _: operit_host_api::HttpRequestData,
    ) -> HostResult<operit_host_api::HttpResponseData> {
        panic!("headless resource tests must not issue HTTP requests")
    }
    /// Rejects unexpected network downloads.
    fn downloadFiles(
        &self,
        _: operit_host_api::HttpDownloadRequest,
        _: operit_host_api::HttpDownloadControl,
        _: operit_host_api::HttpDownloadProgressCallback,
    ) -> HostResult<operit_host_api::HttpDownloadResult> {
        panic!("headless resource tests must not download files")
    }
}
impl operit_host_api::HttpStreamHost for NoNetworkHttpHost {
    /// Rejects unexpected network streams.
    fn openHttpByteStream(
        &self,
        _: String,
        _: operit_host_api::HttpRequestData,
        _: operit_host_api::HttpStreamOpenedCallback,
        _: operit_host_api::HttpStreamChunkCallback,
        _: operit_host_api::HttpStreamClosedCallback,
    ) -> HostResult<()> {
        panic!("headless resource tests must not open HTTP streams")
    }
    /// Rejects unexpected network stream operations.
    fn closeHttpByteStream(&self, _: &str) -> HostResult<()> {
        panic!("headless resource tests do not own HTTP streams")
    }
}

/// Supplies locale and tool-registration capabilities required by real package loading.
pub(super) fn configure_package_hosts(hosts: &mut HostManager) {
    hosts.systemOperationHost = Some(Arc::new(TestSystemLocale));
    hosts.httpHost = Some(Arc::new(NoNetworkHttpHost));
}

/// Sends the real small character-card request through the production IPC dispatcher.
fn character_preferences_request() -> operit_plugin_sdk::javascript::JsToolPkgIpcRequest {
    operit_plugin_sdk::javascript::JsToolPkgIpcRequest {
        dependency_caller: None,
        package_target: "com.operit.character_cards".into(),
        caller_context_key: "ipc-resource-test".into(),
        target_context_key: None,
        target_runtime: Some("main".into()),
        channel: "character-memory.request".into(),
        payload: json!({"action": "readGlobalMemoryPreferences"}),
    }
}

/// Exercises actual public API and IPC dispatch with cold I/O, warm reads, reloads and revocation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ipc_and_public_api_share_resources_without_holding_manager() {
    use operit_tools::tools::ToolJsRuntime::invokeToolPkgIpc;
    use std::sync::atomic::Ordering;
    let _test_guard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    let storage = register_test_runtime_roots();
    let (entered, entering) = mpsc::channel();
    let gate = Arc::new(ResourceGate {
        operation: Mutex::new(None),
        released: Mutex::new(false),
        changed: Condvar::new(),
        entered,
        main_reads: std::sync::atomic::AtomicUsize::new(0),
    });
    let _release = ReleaseGate(gate.clone());
    let mut hosts = HostManager::withFileSystemHost(Arc::new(GatedFileSystem {
        inner: PosixFileSystemHost::new(),
        gate: gate.clone(),
    }));
    hosts.runtimeStorageHost = Some(storage.clone());
    hosts.runtimeSqliteHost = Some(storage);
    hosts.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
    hosts.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    hosts.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    configure_package_hosts(&mut hosts);
    let app = OperitApplication::newWithContext(hosts);
    let packages = app.packageManager();
    let handler = app.aiToolHandler();
    packages.lock().unwrap().loadAvailablePackages();
    let package = "com.operit.character_cards";
    let context = format!("toolpkg_main:{package}");
    let proxy = LocalCoreProxy::new(app);

    gate.arm("read");
    let api = start_package_call(proxy.clone(), package_request("invokeToolPkgPublicApi", json!({
        "packageName": package, "methodName": "chat.sidebar.prepare", "payload": {"chats": []},
    })));
    entering.recv_timeout(Duration::from_secs(20)).expect("public API must load the shared resource snapshot");
    assert!(packages.try_lock().is_ok(), "public API resource I/O must release the manager lock");
    gate.release();
    let result = api.join().unwrap().result.expect("real sidebar public API must succeed");
    let result: Value = fromCoreValue(result).unwrap();
    assert_eq!(result["entries"][0]["entryId"], "sidebar-characters");
    let reads = gate.main_reads.load(Ordering::SeqCst);
    assert_eq!(reads, 1, "cold public API must read the main entry exactly once");

    let mut direct_read = Vec::new();
    let mut cached_read = Vec::new();
    for _ in 0..20 {
        let started = Instant::now();
        let original = packages.lock().unwrap().getRegisteredToolPkgMainScript(package).unwrap();
        direct_read.push(started.elapsed().as_secs_f64() * 1000.0);
        let started = Instant::now();
        let pending = packages.lock().unwrap().toolPkgTextResources(package);
        let resources = pending.await.unwrap();
        let cached = resources.get("dist/main.js").unwrap().clone();
        cached_read.push(started.elapsed().as_secs_f64() * 1000.0);
        assert_eq!(cached, original, "shared IPC resources must preserve exact script bytes");
    }
    direct_read.sort_by(f64::total_cmp);
    cached_read.sort_by(f64::total_cmp);
    println!("IPC_PROBE directScriptP50Ms={:.3} cachedScriptP50Ms={:.3}", direct_read[10], cached_read[10]);

    let expected = invokeToolPkgIpc(&handler, character_preferences_request()).await.unwrap();
    let mut warm = Vec::new();
    for _ in 0..20 {
        let started = Instant::now();
        assert_eq!(invokeToolPkgIpc(&handler, character_preferences_request()).await.unwrap(), expected);
        warm.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    warm.sort_by(f64::total_cmp);
    assert_eq!(gate.main_reads.load(Ordering::SeqCst), reads, "warm IPC must reuse the public API snapshot");
    println!("IPC_PROBE warmBusinessP50Ms={:.3} warmBusinessP95Ms={:.3} mainFileReads={reads}", warm[10], warm[18]);

    let concurrent = (0..4).map(|_| {
        let handler = handler.clone();
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
                .block_on(invokeToolPkgIpc(&handler, character_preferences_request()))
        })
    }).collect::<Vec<_>>();
    for call in concurrent {
        assert_eq!(call.join().unwrap().unwrap(), expected);
    }
    assert_eq!(gate.main_reads.load(Ordering::SeqCst), reads, "concurrent IPC must share the same resources");

    let mut missing = character_preferences_request();
    missing.target_context_key = Some("toolpkg_xml_render:missing".into());
    missing.target_runtime = Some("ui".into());
    assert!(invokeToolPkgIpc(&handler, missing).await.is_err(), "IPC must reject an inactive UI context");
    let mut invalid_caller = character_preferences_request();
    invalid_caller.dependency_caller = Some(package.into());
    assert!(invokeToolPkgIpc(&handler, invalid_caller).await.is_err(), "dependency validation must remain enforced");

    packages.lock().unwrap().loadAvailablePackages();
    let api = start_package_call(proxy.clone(), package_request("invokeToolPkgPublicApi", json!({
        "packageName": package, "methodName": "chat.sidebar.prepare", "payload": {"chats": []},
    })));
    assert!(api.join().unwrap().result.is_ok());
    assert_eq!(gate.main_reads.load(Ordering::SeqCst), reads + 1, "registry reload must invalidate the shared main script");

    packages.lock().unwrap().loadAvailablePackages();
    gate.arm("read");
    let api = start_package_call(proxy, package_request("invokeToolPkgPublicApi", json!({
        "packageName": package, "methodName": "chat.sidebar.prepare", "payload": {"chats": []},
    })));
    entering.recv_timeout(Duration::from_secs(20)).expect("reloaded API must read fresh resources");
    let lease = packages.lock().unwrap().acquireToolPkgExecutionEngine(&context, package);
    lease.await.unwrap();
    packages.lock().unwrap().releaseToolPkgExecutionEngine(&context, package);
    gate.release();
    assert!(api.join().unwrap().result.is_err(), "a revoked engine must not execute after resource loading");

    packages.lock().unwrap().loadAvailablePackages();
    gate.arm("read");
    let ipc = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
            .block_on(invokeToolPkgIpc(&handler, character_preferences_request()))
    });
    entering.recv_timeout(Duration::from_secs(20)).expect("cold IPC must use the shared resource reader");
    assert!(packages.try_lock().is_ok(), "IPC resource I/O must release the manager lock");
    let lease = packages.lock().unwrap().acquireToolPkgExecutionEngine(&context, package);
    lease.await.unwrap();
    packages.lock().unwrap().releaseToolPkgExecutionEngine(&context, package);
    gate.release();
    assert_eq!(ipc.join().unwrap().unwrap_err(), "ToolPkg execution context has been released or replaced");
}
