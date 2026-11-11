use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use operit_host_api::HostManager::HostManager;
use operit_host_api::{
    HostError, HostResult, HttpDownloadControl, HttpDownloadProgressCallback, HttpDownloadRequest,
    HttpDownloadResult, HttpFileDownloadResult, HttpHost, HttpImageDelivery, HttpRequestData,
    HttpResponseData, HttpStreamChunkCallback, HttpStreamClosedCallback, HttpStreamHost,
    HttpStreamOpenedCallback,
};
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost, NativeRuntimeStorageHost,
    PosixFileSystemHost,
};
use operit_link::{fromCoreValue, toCoreValue, CoreCallRequest, CoreLinkSharedClient};
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_store::ExtensionStore::ExtensionStore;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

use serde_json::{json, Value};

const SCRIPT: &str = r#"/* METADATA
{"name":"market_test_plugin","version":"1.0.0","description":"Test plugin","tools":[]}
*/
"#;
const SCRIPT_SHA256: &str = "864cc15ac3f5cd567f734cd9364fdf7c1ccaf1342e9cb2a9d3a3b2974c97728e";

struct MarketHttpHost {
    entry: Mutex<Value>,
    payload: Vec<u8>,
}

impl HttpStreamHost for MarketHttpHost {
    fn openHttpByteStream(
        &self,
        _streamId: String,
        _request: HttpRequestData,
        _onOpened: HttpStreamOpenedCallback,
        _onChunk: HttpStreamChunkCallback,
        _onClosed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        Err(HostError::new("Not used by market install test"))
    }

    fn closeHttpByteStream(&self, _streamId: &str) -> HostResult<()> {
        Ok(())
    }
}

impl HttpHost for MarketHttpHost {
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }

    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        let body = if request.url.contains("/entries/") {
            json!({"entriesById": {"entry": self.entry.lock().unwrap().clone()}})
        } else {
            json!({"ok": true})
        };
        Ok(HttpResponseData {
            finalUrl: request.url,
            statusCode: 200,
            statusMessage: "OK".into(),
            headers: vec![],
            body: serde_json::to_vec(&body).unwrap(),
        })
    }

    fn downloadToFile(
        &self,
        request: HttpRequestData,
        target: &dyn operit_host_api::FileTarget,
    ) -> HostResult<HttpFileDownloadResult> {
        let targetPath = target.nativePath()?;
        std::fs::write(&targetPath, &self.payload)
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(HttpFileDownloadResult {
            finalUrl: request.url,
            targetPath,
            downloadedBytes: self.payload.len() as u64,
        })
    }

    fn downloadFiles(
        &self,
        _request: HttpDownloadRequest,
        _control: HttpDownloadControl,
        _onProgress: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        Err(HostError::new("Not used by market install test"))
    }
}

async fn versions(proxy: &LocalCoreProxy) -> BTreeMap<String, String> {
    let response = CoreLinkSharedClient::call(
        proxy,
        CoreCallRequest::new(
            "market-status",
            "core/application",
            "getInstalledMarketVersions",
            toCoreValue(json!({})).unwrap(),
        ),
    )
    .await;
    fromCoreValue(response.result.expect("market status route must succeed")).unwrap()
}

/// Exercises persisted markers and the exact generated routes used by Flutter.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn market_install_markers_survive_reentry_and_do_not_mark_failed_updates_as_installed() {
    tokio::task::LocalSet::new().run_until(async {
        let root = std::env::temp_dir().join(format!("operit-market-state-{}-{}", std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        let runtime_root = root.join("runtime");
        let workspace_root = root.join("workspaces");
        std::fs::create_dir_all(&runtime_root).unwrap();
        std::fs::create_dir_all(&workspace_root).unwrap();

        let storage = Arc::new(NativeRuntimeStorageHost::new(runtime_root.clone(), workspace_root));
        let http = Arc::new(MarketHttpHost {
            entry: Mutex::new(json!({
                "id": "entry", "type": "script", "title": "Test plugin",
                "artifact": {"projectId": "project", "runtimePackageId": "market_test_plugin"},
                "latestVersion": {"id": "v1", "version": "1.0.0"},
                "assets": [{"id": "asset", "versionId": "v1", "assetName": "market_test_plugin.js", "sha256": SCRIPT_SHA256}]
            })),
            payload: SCRIPT.as_bytes().to_vec(),
        });
        let app = OperitApplication::newWithContext(HostManager {
            fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
            runtimeStorageHost: Some(storage.clone()), runtimeSqliteHost: Some(storage),
            httpHost: Some(http.clone()),
            hostJavaScriptRuntimeHost: Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
            hostRuntimeTaskSchedulerHost: Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new())),
            ..HostManager::default()
        });
        let proxy = LocalCoreProxy::new(app);
        assert!(versions(&proxy).await.is_empty());

        let response = CoreLinkSharedClient::call(&proxy, CoreCallRequest::new(
            "market-install", "core/application", "installMarketEntry",
            toCoreValue(json!({"entryId": "entry", "versionId": null})).unwrap(),
        )).await;
        let installed: String = fromCoreValue(response.result.expect("artifact installation must succeed")).unwrap();
        assert_eq!(installed, "v1");
        assert_eq!(versions(&proxy).await.get("entry").map(String::as_str), Some("v1"));

        let config = ExtensionStore::configPathForScope("market_test_plugin", "device").unwrap();
        let marker = RuntimeStorePaths::default().runtime_storage_path(&config).join(".operit/market.json");
        let persisted: Value = serde_json::from_str(&std::fs::read_to_string(&marker).unwrap()).unwrap();
        assert_eq!(persisted, json!({"entryId": "entry", "versionId": "v1"}));

        // Re-reading the Core marker is sufficient; no UI flags or success messages are needed.
        assert_eq!(versions(&proxy).await.get("entry").map(String::as_str), Some("v1"));
        {
            let mut entry = http.entry.lock().unwrap();
            entry["latestVersion"]["id"] = json!("v2");
            entry["assets"][0]["versionId"] = json!("v2");
            entry["assets"][0]["sha256"] = json!("0".repeat(64));
        }
        let failure = CoreLinkSharedClient::call(&proxy, CoreCallRequest::new(
            "market-failed-update", "core/application", "installMarketEntry",
            toCoreValue(json!({"entryId": "entry", "versionId": null})).unwrap(),
        )).await;
        assert!(failure.result.is_err());
        assert_eq!(versions(&proxy).await.get("entry").map(String::as_str), Some("v1"));

        // A corrupt marker must not stop unrelated installed skill markers from loading.
        let skill_root = RuntimeStorePaths::default().skills_dir().join("test_skill");
        std::fs::create_dir_all(skill_root.join(".operit")).unwrap();
        std::fs::write(skill_root.join("SKILL.md"), "---\nname: test_skill\ndescription: Test\n---\n# Test").unwrap();
        std::fs::write(skill_root.join(".operit/market.json"), r#"{"entryId":"skill-entry","versionId":"sv1"}"#).unwrap();
        std::fs::write(&marker, "not json").unwrap();
        let states = versions(&proxy).await;
        assert!(!states.contains_key("entry"));
        assert_eq!(states.get("skill-entry").map(String::as_str), Some("sv1"));
        std::fs::remove_dir_all(skill_root).unwrap();
        assert!(!versions(&proxy).await.contains_key("skill-entry"));

        std::fs::remove_dir_all(root).unwrap();
    }).await;
}
