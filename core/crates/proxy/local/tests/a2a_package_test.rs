//! Exercises the generated Flutter routes, canonical extension ownership and real package dispatch.
use operit_host_api::{HostManager::HostManager, *};
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost, NativeRuntimeStorageHost,
    PosixFileSystemHost,
};
use operit_link::{fromCoreValue, toCoreValue, CoreCallRequest, CoreLinkSharedClient};
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct AgentHost {
    requests: Mutex<Vec<HttpRequestData>>,
    closed: Mutex<Vec<String>>,
    fail_next: Mutex<bool>,
}
fn card() -> Value {
    json!({"name":"Research Agent","description":"Research across documents", "version":"1.0", "supportedInterfaces":[{"url":"https://agent.test/rpc", "protocolBinding":"JSONRPC","protocolVersion":"1.0","tenant":"research-team"}],"skills":[{"id":"research","name":"Research","description":"Find references"}]})
}
impl HttpStreamHost for AgentHost {
    fn openHttpByteStream(
        &self,
        _: String,
        _: HttpRequestData,
        _: HttpStreamOpenedCallback,
        _: HttpStreamChunkCallback,
        _: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        panic!("A2A must use asynchronous response metadata")
    }
    fn closeHttpByteStream(&self, id: &str) -> HostResult<()> {
        self.closed.lock().unwrap().push(id.into());
        Ok(())
    }
    fn openHttpResponseStream(
        &self,
        _: String,
        request: HttpRequestData,
        head: HttpStreamResponseCallback,
        chunk: HttpStreamChunkCallback,
        closed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        let payload = if request.method == "GET" {
            card()
        } else {
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(body["params"]["tenant"], "research-team");
            let result = match body["method"].as_str().unwrap() {
                "SendMessage" => {
                    assert_eq!(body["params"]["message"]["role"], "ROLE_USER");
                    assert_eq!(body["params"]["configuration"]["returnImmediately"], true);
                    if body["params"]["message"]["taskId"] == "task-1" {
                        json!({"task":{"id":"task-1","contextId":"context-1","status":{"state":"TASK_STATE_COMPLETED"},"artifacts":[{"artifactId":"output","parts":[{"text":"Research result"}]}]}})
                    } else {
                        json!({"task":{"id":"task-1","contextId":"context-1","status":{"state":"TASK_STATE_INPUT_REQUIRED"},"history":[{"role":"ROLE_AGENT","parts":[{"text":"Which topic?"}]}]}})
                    }
                }
                "GetTask" => {
                    assert_eq!(body["params"]["id"], "task-1");
                    json!({"id":"task-1","contextId":"context-1","status":{"state":"TASK_STATE_WORKING"}})
                }
                "CancelTask" => json!({"id":"task-1","status":{"state":"TASK_STATE_CANCELED"}}),
                other => panic!("Unexpected A2A method: {other}"),
            };
            json!({"jsonrpc":"2.0","id":body["id"],"result":result})
        };
        self.requests.lock().unwrap().push(request.clone());
        head(HttpResponseHead {
            finalUrl: request.url,
            statusCode: 200,
            statusMessage: "OK".into(),
            headers: vec![("Content-Type".into(), "application/json".into())],
        });
        if std::mem::take(&mut *self.fail_next.lock().unwrap()) {
            closed(Err("connection lost after accepting request".into()));
            return Ok(());
        }
        let bytes = serde_json::to_vec(&payload).unwrap();
        for part in bytes.chunks(13) {
            chunk(part.to_vec());
        }
        closed(Ok(()));
        Ok(())
    }
}
impl HttpHost for AgentHost {
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        assert!(request.url.contains("/entries/"));
        Ok(HttpResponseData {finalUrl:request.url,statusCode:200,statusMessage:"OK".into(),headers:vec![],body:serde_json::to_vec(&json!({"entriesById":{"market-agent":{"id":"market-agent","type":"a2a","title":"Market Agent","latestVersion":{"id":"v1","version":"1.0.0","installConfig":json!({"a2aAgents":{"market":{"url":"https://agent.test", "card":card()}}}).to_string()}}}})).unwrap()})
    }
    fn downloadFiles(
        &self,
        _: HttpDownloadRequest,
        _: HttpDownloadControl,
        _: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        panic!("Not used")
    }
}

async fn call(proxy: &LocalCoreProxy, target: &str, method: &str, args: Value) -> Value {
    let response = CoreLinkSharedClient::call(
        proxy,
        CoreCallRequest::new("a2a-test", target, method, toCoreValue(args).unwrap()),
    )
    .await;
    fromCoreValue(
        response
            .result
            .unwrap_or_else(|error| panic!("{target}.{method}: {error:?}")),
    )
    .unwrap()
}
async fn app(proxy: &LocalCoreProxy, method: &str, args: Value) -> Value {
    call(proxy, "core/application", method, args).await
}
async fn package(proxy: &LocalCoreProxy, method: &str, args: Value) -> Value {
    call(proxy, "core/application.packageManager", method, args).await
}
async fn invoke(proxy: &LocalCoreProxy, operation: &str, args: Value) -> Value {
    let text = app(
        proxy,
        "invokeA2A",
        json!({"id":"research","operation":operation,"argumentsJson":args.to_string()}),
    )
    .await;
    serde_json::from_str(text.as_str().unwrap()).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a2a_is_a_managed_package_with_resumable_tasks_and_market_ownership() {
    tokio::task::LocalSet::new().run_until(async {
        let root = std::env::temp_dir().join(format!("operit-a2a-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let runtime = root.join("runtime"); let workspace = root.join("workspace");
        std::fs::create_dir_all(&runtime).unwrap(); std::fs::create_dir_all(&workspace).unwrap();

        let storage = Arc::new(NativeRuntimeStorageHost::new(runtime.clone(),workspace));
        let http = Arc::new(AgentHost::default());
        let application = OperitApplication::newWithContext(HostManager {
            fileSystemHost:Some(Arc::new(PosixFileSystemHost::new())),
            runtimeStorageHost:Some(storage.clone()),runtimeSqliteHost:Some(storage),httpHost:Some(http.clone()),
            hostJavaScriptRuntimeHost:Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
            hostRuntimeTaskSchedulerHost:Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new())),..HostManager::default()
        });
        let proxy = LocalCoreProxy::new(application);
        let manifest = json!({"a2aAgents":{"research":{"url":"https://agent.test","headers":{"Authorization":"Bearer test"}}}});
        assert_eq!(app(&proxy,"importA2AConfig",json!({"configJson":manifest.to_string()})).await,json!(["research"]));
        let definitions = package(&proxy,"getAvailablePackages",json!({})).await;
        assert_eq!(definitions["a2a.research"]["category"],"A2A");
        assert_eq!(definitions["a2a.research"]["tools"].as_array().unwrap().len(),3);
        let enabled = package(&proxy,"getEnabledPackageNames",json!({})).await;
        assert!(enabled.as_array().unwrap().contains(&json!("a2a.research")));
        let prompt = package(&proxy,"usePackage",json!({"packageName":"a2a.research"})).await;
        assert!(prompt.as_str().unwrap().contains("a2a.research:send_message"));
        let catalog = app(&proxy,"getExtensionCatalogSnapshot",json!({})).await;
        assert_eq!(catalog["installed"].as_array().unwrap().iter().filter(|entry| entry["id"] == "research" && entry["kind"] == "a2a").count(),1);
        assert!(!catalog["installed"].as_array().unwrap().iter().any(|entry|entry["id"] == "a2a.research" && entry["kind"] == "script"));

        // Dispatch through the same package executor and proxy envelope used by AI tools.
        let executed = call(&proxy,"core/application.aiToolHandler","executeTool",json!({"tool":{"name":"package_proxy","parameters":[{"name":"tool_name","value":"a2a.research:send_message"},{"name":"params","value":json!({"message":"Research via package proxy"}).to_string()}]}})).await;
        assert_eq!(executed["success"],true,"{executed}");

        // Remote IDs support follow-up input and explicit status retrieval.
        let result = invoke(&proxy,"send_message",json!({"message":"Research this"})).await;
        assert_eq!(result["task"]["status"]["state"],"TASK_STATE_INPUT_REQUIRED");
        assert_eq!(invoke(&proxy,"get_task",json!({"task_id":"task-1"})).await["status"]["state"],"TASK_STATE_WORKING");
        let result = invoke(&proxy,"send_message",json!({"message":"Rust tooling","task_id":"task-1","context_id":"context-1"})).await;
        assert_eq!(result["task"]["artifacts"][0]["parts"][0]["text"],"Research result");
        assert_eq!(invoke(&proxy,"cancel_task",json!({"task_id":"task-1"})).await["status"]["state"],"TASK_STATE_CANCELED");
        for request in http.requests.lock().unwrap().iter() { assert!(!request.followRedirects); assert!(request.headers.contains(&("Authorization".into(),"Bearer test".into()))); }
        assert_eq!(http.requests.lock().unwrap().len(),http.closed.lock().unwrap().len());

        // Ambiguous send failures are surfaced without replaying a potentially accepted task.
        *http.fail_next.lock().unwrap() = true;
        let before = http.requests.lock().unwrap().len();
        let response = CoreLinkSharedClient::call(&proxy,CoreCallRequest::new("failed-send","core/application","invokeA2A",toCoreValue(json!({"id":"research","operation":"send_message","argumentsJson":json!({"message":"No replay"}).to_string()})).unwrap())).await;
        assert!(response.result.is_err());
        assert_eq!(http.requests.lock().unwrap().len(),before+1);

        package(&proxy,"disablePackage",json!({"packageName":"a2a.research"})).await;
        let disabled_call = call(&proxy,"core/application.aiToolHandler","executeTool",json!({"tool":{"name":"a2a.research:get_task","parameters":[{"name":"task_id","value":"task-1"}]}})).await;
        assert_eq!(disabled_call["success"],false);
        assert!(!package(&proxy,"getEnabledPackageNames",json!({})).await.as_array().unwrap().contains(&json!("a2a.research")));
        package(&proxy,"enablePackage",json!({"packageName":"a2a.research"})).await;
        assert!(package(&proxy,"getEnabledPackageNames",json!({})).await.as_array().unwrap().contains(&json!("a2a.research")));
        package(&proxy,"setPackageScope",json!({"packageName":"a2a.research","scope":"space"})).await;
        assert_eq!(app(&proxy,"getExtensionScopes",json!({"kind":"a2a"})).await["research"],"space");
        let saved = app(&proxy,"getA2AConfig",json!({"id":"research"})).await;
        let saved: Value = serde_json::from_str(saved.as_str().unwrap()).unwrap();
        assert_eq!(saved["card"]["name"],"Research Agent");
        assert_eq!(saved["headers"]["Authorization"],"Bearer test");
        package(&proxy,"deletePackage",json!({"packageName":"a2a.research"})).await;
        assert!(package(&proxy,"getAvailablePackages",json!({})).await.get("a2a.research").is_none());

        // Marketplace installation, update tracking and scope changes use the same owner record.
        assert_eq!(app(&proxy,"installMarketEntry",json!({"entryId":"market-agent","versionId":null})).await,"v1");
        assert_eq!(app(&proxy,"getInstalledMarketVersions",json!({})).await["market-agent"],"v1");
        app(&proxy,"setExtensionScope",json!({"kind":"a2a","id":"market","scope":"space"})).await;
        let catalog = app(&proxy,"getExtensionCatalogSnapshot",json!({})).await;
        let row = catalog["installed"].as_array().unwrap().iter().find(|entry|entry["id"] == "market" && entry["kind"] == "a2a").unwrap();
        // Updating the market entry keeps local authorization, state and scope.
        app(&proxy,"importA2AConfig",json!({"configJson":json!({"a2aAgents":{"market":{"url":"https://agent.test","headers":{"Authorization":"Bearer private"},"disabled":true,"card":card()}}}).to_string()})).await;
        app(&proxy,"installMarketEntry",json!({"entryId":"market-agent","versionId":null})).await;
        let saved = app(&proxy,"getA2AConfig",json!({"id":"market"})).await;
        let saved: Value = serde_json::from_str(saved.as_str().unwrap()).unwrap();
        assert_eq!(saved["headers"]["Authorization"],"Bearer private");
        assert_eq!(saved["disabled"],true);
        package(&proxy,"loadAvailablePackages",json!({})).await;
        assert_eq!(package(&proxy,"getAvailablePackages",json!({})).await["a2a.market"]["category"],"A2A");
        assert!(!package(&proxy,"getEnabledPackageNames",json!({})).await.as_array().unwrap().contains(&json!("a2a.market")));
        assert_eq!(app(&proxy,"getExtensionScopes",json!({"kind":"a2a"})).await["market"],"space");

        assert_eq!(row["source"],"market"); assert_eq!(row["marketEntryId"],"market-agent");
        assert_eq!(row["scope"],"space");
        app(&proxy,"deleteA2A",json!({"id":"market"})).await;
        assert!(app(&proxy,"getInstalledMarketVersions",json!({})).await.get("market-agent").is_none());
        drop(proxy);
        std::fs::remove_dir_all(root).unwrap();
    }).await;
}
