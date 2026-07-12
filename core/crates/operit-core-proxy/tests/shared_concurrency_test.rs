use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use operit_core_proxy::LocalCoreProxy;
use operit_host_api::HostManager::HostManager;
use operit_host_api::{
    BrowserSessionCommand, BrowserSessionCommandResult, BrowserSessionHost, BrowserSessionInfo,
    BrowserSessionSnapshot, HostResult, HostSecretStore, RuntimeSqliteConnection,
    RuntimeSqliteHost, RuntimeStorageEntry, RuntimeStorageHost,
};
use operit_host_native_common::NativeRuntimeStorageHost;
use operit_link::{
    fromCoreValue, toCoreValue, CoreCallRequest, CoreLinkSharedClient, CoreWatchRequest,
};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::services::RuntimeHostInteractionService::{
    requestOwnerBrowserSession, RuntimeHostInteractionBrowserSessionPayload,
};
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};
use serde_json::{json, Value};

struct OwnerInteractionBrowserHost;

#[derive(Default)]
struct TestSecretStore {
    values: Mutex<BTreeMap<String, Vec<u8>>>,
}

struct TestRuntimeStorageHost {
    inner: NativeRuntimeStorageHost,
}

impl TestRuntimeStorageHost {
    /// Creates a test storage host around the native storage implementation.
    fn new(runtime_root: std::path::PathBuf, workspace_root: std::path::PathBuf) -> Self {
        Self {
            inner: NativeRuntimeStorageHost::new(runtime_root, workspace_root),
        }
    }

    /// Maps legacy secure paths into the test runtime namespace.
    fn path(path: &str) -> String {
        if path == "secure" || path.starts_with("secure/") {
            return format!("runtime/{path}");
        }
        path.to_string()
    }
}

impl RuntimeStorageHost for TestRuntimeStorageHost {
    /// Returns the physical test runtime root.
    fn runtimeRootDir(&self) -> Option<std::path::PathBuf> {
        self.inner.runtimeRootDir()
    }

    /// Returns the physical test workspace root.
    fn workspaceRootDir(&self) -> Option<std::path::PathBuf> {
        self.inner.workspaceRootDir()
    }

    /// Reads bytes from the mapped test path.
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.inner.readBytes(&Self::path(path))
    }

    /// Writes bytes to the mapped test path.
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.writeBytes(&Self::path(path), content)
    }

    /// Deletes an entry at the mapped test path.
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.inner.delete(&Self::path(path), recursive)
    }

    /// Checks an entry at the mapped test path.
    fn exists(&self, path: &str) -> HostResult<bool> {
        self.inner.exists(&Self::path(path))
    }

    /// Lists entries below the mapped test path.
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        self.inner.list(&Self::path(prefix))
    }
}

impl RuntimeSqliteHost for TestRuntimeStorageHost {
    /// Opens a SQLite database through the native test host.
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        self.inner.openSqliteDatabase(&Self::path(path))
    }
}

impl HostSecretStore for TestSecretStore {
    /// Reads one test secret from process memory.
    fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        Ok(self
            .values
            .lock()
            .expect("test secret store mutex must remain valid")
            .get(key)
            .cloned())
    }

    /// Writes one test secret into process memory.
    fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()> {
        self.values
            .lock()
            .expect("test secret store mutex must remain valid")
            .insert(key.to_string(), content.to_vec());
        Ok(())
    }

    /// Deletes one test secret from process memory.
    fn deleteSecret(&self, key: &str) -> HostResult<()> {
        self.values
            .lock()
            .expect("test secret store mutex must remain valid")
            .remove(key);
        Ok(())
    }
}

impl BrowserSessionHost for OwnerInteractionBrowserHost {
    /// Lists sessions by requesting the runtime owner app through Core interaction.
    fn listBrowserSessions(&self) -> HostResult<Vec<BrowserSessionInfo>> {
        let response = requestOwnerBrowserSession(
            RuntimeHostInteractionBrowserSessionPayload {
                commandJson: json!({
                    "action": "list",
                    "sessionId": null,
                    "url": null,
                    "script": null,
                    "payloadJson": "",
                    "userAgent": null,
                    "headers": {}
                })
                .to_string(),
            },
            Duration::from_secs(2),
        )
        .expect("owner browser response must arrive");
        let result: BrowserSessionCommandResult =
            serde_json::from_str(&response.resultJson).expect("browser result must decode");
        Ok(result.sessions)
    }

    /// Rejects session creation because this test only exercises listing.
    fn createBrowserSession(
        &self,
        _initialUrl: &str,
        _userAgent: Option<&str>,
        _headers: BTreeMap<String, String>,
    ) -> HostResult<BrowserSessionInfo> {
        panic!("createBrowserSession is not used by this test")
    }

    /// Rejects session updates because this test only exercises listing.
    fn updateBrowserSession(
        &self,
        _sessionId: &str,
        _userAgent: Option<&str>,
        _headers: BTreeMap<String, String>,
    ) -> HostResult<BrowserSessionInfo> {
        panic!("updateBrowserSession is not used by this test")
    }

    /// Rejects semantic commands because this test only exercises listing.
    fn submitBrowserCommand(
        &self,
        _command: BrowserSessionCommand,
    ) -> HostResult<BrowserSessionCommandResult> {
        panic!("submitBrowserCommand is not used by this test")
    }

    /// Rejects snapshots because this test only exercises listing.
    fn getBrowserSessionSnapshot(&self, _sessionId: &str) -> HostResult<BrowserSessionSnapshot> {
        panic!("getBrowserSessionSnapshot is not used by this test")
    }

    /// Rejects session closing because this test only exercises listing.
    fn closeBrowserSession(&self, _sessionId: &str) -> HostResult<BrowserSessionCommandResult> {
        panic!("closeBrowserSession is not used by this test")
    }
}

/// Builds an empty successful browser-session command result.
fn browser_result_json() -> String {
    json!({
        "success": true,
        "session": null,
        "sessions": [],
        "resultJson": "",
        "error": null
    })
    .to_string()
}

/// Registers isolated runtime roots required by application service constructors.
fn register_test_runtime_roots() -> Arc<TestRuntimeStorageHost> {
    let root = std::env::temp_dir().join(format!(
        "operit-core-proxy-shared-concurrency-{}",
        std::process::id()
    ));
    let runtime_root = root.join("runtime");
    let workspace_root = root.join("workspaces");
    std::fs::create_dir_all(&runtime_root).expect("test runtime root must be created");
    std::fs::create_dir_all(&workspace_root).expect("test workspace root must be created");
    setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
        runtime_root.clone(),
        workspace_root.clone(),
    ));
    Arc::new(TestRuntimeStorageHost::new(runtime_root, workspace_root))
}

/// Verifies an owner response can enter Core while the originating Core call is waiting.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shared_core_accepts_nested_owner_response() {
    let storage_host = register_test_runtime_roots();
    let mut host_manager =
        HostManager::new().withBrowserSessionHost(Arc::new(OwnerInteractionBrowserHost));
    host_manager.runtimeStorageHost = Some(storage_host.clone());
    host_manager.runtimeSqliteHost = Some(storage_host);
    host_manager.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
    let proxy = Arc::new(LocalCoreProxy::new(OperitApplication::newWithContext(
        host_manager,
    )));
    let mut owner_events = CoreLinkSharedClient::watch(
        proxy.as_ref(),
        CoreWatchRequest::new(
            "owner-events",
            "services.runtimeHostInteractionService",
            "ownerHostInteractionEvents",
            toCoreValue(json!({"kinds": ["browser_session"]})).unwrap(),
        ),
    )
    .await
    .expect("owner event stream must open");

    let list_proxy = proxy.clone();
    let list_call = tokio::spawn(async move {
        CoreLinkSharedClient::call(
            list_proxy.as_ref(),
            CoreCallRequest::new(
                "list-browser-sessions",
                "services.runtimeBrowserService",
                "listBrowserSessions",
                toCoreValue(json!({})).unwrap(),
            ),
        )
        .await
    });

    let owner_event = tokio::time::timeout(Duration::from_millis(500), owner_events.recv())
        .await
        .expect("owner request must be published without waiting for the list timeout")
        .expect("owner event stream must remain open");
    let owner_request_value: Value = fromCoreValue(owner_event.value).unwrap();
    let owner_request = owner_request_value
        .as_object()
        .expect("owner request must be an object");
    let owner_request_id = owner_request
        .get("requestId")
        .and_then(Value::as_str)
        .expect("owner request id must be present");

    let response = CoreLinkSharedClient::call(
        proxy.as_ref(),
        CoreCallRequest::new(
            "respond-owner",
            "services.runtimeHostInteractionService",
            "respondOwnerHostInteraction",
            toCoreValue(json!({
                "requestId": owner_request_id,
                "response": {
                    "browserAutomation": null,
                    "browserSession": {"resultJson": browser_result_json()},
                    "webVisit": null,
                    "composeWebViewController": null,
                    "systemCaptureScreenshot": null,
                    "systemLanguageCode": null,
                    "systemRecognizeText": null,
                    "audioPlay": null,
                    "musicPlayback": null,
                    "bluetooth": null,
                    "ttsSynthesis": null,
                    "ttsPlayback": null,
                    "toolPermission": null
                }
            }))
            .unwrap(),
        ),
    )
    .await;
    assert!(
        response.result.is_ok(),
        "owner response call failed: {response:?}"
    );

    let list_response = tokio::time::timeout(Duration::from_millis(500), list_call)
        .await
        .expect("browser list call must complete after the owner response")
        .expect("browser list task must not panic");
    assert!(
        list_response.result.is_ok(),
        "browser list call failed: {list_response:?}"
    );
}

/// Verifies concurrent Application child calls wait for the shared application lock.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shared_core_serializes_application_child_access() {
    let storage_host = register_test_runtime_roots();
    let mut host_manager = HostManager::new();
    host_manager.runtimeStorageHost = Some(storage_host.clone());
    host_manager.runtimeSqliteHost = Some(storage_host);
    host_manager.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
    let proxy = Arc::new(LocalCoreProxy::new(OperitApplication::newWithContext(
        host_manager,
    )));
    let barrier = Arc::new(tokio::sync::Barrier::new(16));
    let mut calls = Vec::with_capacity(16);

    for index in 0..16 {
        let call_proxy = proxy.clone();
        let call_barrier = barrier.clone();
        calls.push(tokio::spawn(async move {
            call_barrier.wait().await;
            let (method_name, args) = if index % 2 == 0 {
                (
                    "getToolPkgUiRoutes",
                    json!({"runtime": "compose_dsl", "useEnglish": false}),
                )
            } else {
                ("getToolPkgNavigationEntries", json!({"useEnglish": false}))
            };
            CoreLinkSharedClient::call(
                call_proxy.as_ref(),
                CoreCallRequest::new(
                    format!("application-child-{index}"),
                    "application.packageManager",
                    method_name,
                    toCoreValue(args).unwrap(),
                ),
            )
            .await
        }));
    }

    for call in calls {
        let response = call.await.expect("application child call must not panic");
        assert!(
            response.result.is_ok(),
            "application child call failed: {response:?}"
        );
    }
}
