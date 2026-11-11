use std::sync::Arc;
use std::time::Duration;

use operit_host_api::HostManager::HostManager;
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost,
    NativeRuntimeStorageHost, PosixFileSystemHost,
};
use operit_link::{toCoreValue, CoreCallRequest, CoreLinkSharedClient};
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::plugins::toolpkg::{
    ToolPkgChatInputHookBridge::ToolPkgChatInputHookBridge,
    ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime,
};

use serde_json::json;

/// A widget submit waiting for plugins must release Holder so the plugin's native send can reenter it.
#[tokio::test(flavor = "current_thread")]
async fn pending_widget_submit_releases_holder_and_preserves_registry_failure() {
    let root = std::env::temp_dir().join(format!("operit-chat-input-reentry-{}", std::process::id()));
    let runtime = root.join("runtime");
    let workspace = root.join("workspace");
    std::fs::create_dir_all(&runtime).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();

    let storage = Arc::new(NativeRuntimeStorageHost::new(runtime, workspace));
    let mut hosts = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
    hosts.runtimeStorageHost = Some(storage.clone());
    hosts.runtimeSqliteHost = Some(storage);
    hosts.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    hosts.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    let application = OperitApplication::newWithContext(hosts.clone());
    ToolPkgChatInputHookBridge::register(ToolPkgBridgeRuntime::new(application.aiToolHandler(), hosts));
    let packages = application.packageManager();
    let readiness = packages.lock().unwrap().packageRegistryReadiness();
    assert!(readiness.claim_initial_load().unwrap());
    let proxy = LocalCoreProxy::new(application);
    let target = LocalCoreProxy::generatedTargetForSchema("chatRuntimeHolderMain").unwrap();
    let holder = proxy.chatRuntimeHolder();
    let chatId = {
        let mut holder = holder.lock().await;
        let manager = &holder.coreForTarget(target).unwrap().chatHistoryDelegate.chatHistoryManager;
        let draft = manager.newChatDraft("Input reentry".into(), Default::default(), None, None).unwrap();
        manager.commitChatDraft(draft, None).unwrap().id
    };
    let request = CoreCallRequest::new("pending-submit", target, "dispatchChatInputSubmitRequested",
        toCoreValue(json!({ "chatIdOverride": chatId, "messageText": "group input",
            "selectionStart": 11, "selectionEnd": 11, "attachments": [],
            "replyToMessageTimestamp": null })).unwrap());
    let mut pending = Box::pin(CoreLinkSharedClient::call(&proxy, request));
    std::future::poll_fn(|context| {
        assert!(std::future::Future::poll(pending.as_mut(), context).is_pending());
        std::task::Poll::Ready(())
    }).await;
    assert!(holder.try_lock().is_ok(), "a pending input hook must permit native-send reentry");
    let unrelated = CoreLinkSharedClient::call(&proxy, CoreCallRequest::new(
        "loading-state", target, "currentChatIsLoading", toCoreValue(json!({})).unwrap()));
    assert!(tokio::time::timeout(Duration::from_secs(2), unrelated).await
        .expect("the pending hook must not block chat state reads").result.is_ok());
    readiness.fail("original input registry failure".into());
    let result = tokio::time::timeout(Duration::from_secs(2), pending).await.unwrap();
    assert_eq!(result.result.unwrap_err().message, "original input registry failure");
}
