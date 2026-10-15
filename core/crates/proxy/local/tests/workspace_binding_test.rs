use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use operit_host_api::HostManager::HostManager;
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost, NativeRuntimeStorageHost,
    PosixFileSystemHost,
};
use operit_link::{toCoreValue, CoreCallRequest, CoreLinkSharedClient};
use operit_model::ChatMessage::ChatMessage;
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};
use serde_json::json;

/// Exercises Flutter's generated proxy route, not just the CLI's direct Core calls.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn flutter_workspace_binding_is_idempotent_and_errors_remain_recoverable() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let root = std::env::temp_dir().join(format!(
                "operit-proxy-workspace-binding-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let runtime_root = root.join("runtime");
            let workspace_root = root.join("workspaces");
            std::fs::create_dir_all(&runtime_root).unwrap();
            std::fs::create_dir_all(&workspace_root).unwrap();
            setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
                runtime_root.clone(),
                workspace_root.clone(),
            ));
            let storage = Arc::new(NativeRuntimeStorageHost::new(runtime_root, workspace_root));
            let host_manager = HostManager {
                fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
                runtimeStorageHost: Some(storage.clone()),
                runtimeSqliteHost: Some(storage),
                hostJavaScriptRuntimeHost: Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
                hostRuntimeTaskSchedulerHost: Some(Arc::new(
                    NativeHostRuntimeTaskSchedulerHost::new(),
                )),
                ..HostManager::default()
            };
            let proxy = LocalCoreProxy::new(OperitApplication::newWithContext(host_manager));
            let target = LocalCoreProxy::generatedTargetForSchema("chatRuntimeHolderMain")
                .expect("Flutter chat runtime target must be generated");
            let chat_id = {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                let core = holder.coreForTarget(target).unwrap();
                core.createNewChat(true, None, None).await.unwrap();
                core.chatHistoryDelegate
                    .currentChatIdFlow()
                    .value()
                    .unwrap()
            };
            let response = CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new(
                    "flutter-bind-first",
                    target,
                    "bindChatToWorkspace",
                    toCoreValue(json!({"chatId": chat_id, "workspace": "/app/workspaces/project"}))
                        .unwrap(),
                ),
            )
            .await;
            response
                .result
                .expect("first bind must succeed through the generated proxy");
            let (original_workspace, inherited_chat_id) = {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                let core = holder.coreForTarget(target).unwrap();
                let workspace = core
                    .chatHistoryDelegate
                    .chatHistoryManager
                    .getWorkspaceForChat(&chat_id)
                    .unwrap()
                    .unwrap();
                // Empty chats are deliberately reused; make this one non-empty
                // so the next chat exercises inherited workspace binding.
                let message = ChatMessage::new_with_markdown(
                    "user".to_string(),
                    "Keep the existing workspace".to_string(),
                );
                core.chatHistoryDelegate
                    .addMessageToChat(message, Some(chat_id.clone()));
                core.createNewChat(true, Some(chat_id.clone()), None)
                    .await
                    .unwrap();
                let inherited_chat_id = core
                    .chatHistoryDelegate
                    .currentChatIdFlow()
                    .value()
                    .unwrap();
                let inherited_workspace = core
                    .chatHistoryDelegate
                    .chatHistoryManager
                    .getWorkspaceForChat(&inherited_chat_id)
                    .unwrap()
                    .unwrap();
                assert_eq!(inherited_workspace, workspace);
                (workspace, inherited_chat_id)
            };
            assert_ne!(chat_id, inherited_chat_id);

            for (request_id, path, succeeds) in [
                ("flutter-rebind", "/app/workspaces/project", true),
                (
                    "flutter-rebind-normalized",
                    " /app/workspaces/project/ ",
                    true,
                ),
                ("flutter-invalid-path", "relative/project", false),
                (
                    "flutter-name-conflict",
                    "/app/workspaces/other/project",
                    false,
                ),
                ("flutter-retry-after-error", "/app/workspaces/project", true),
            ] {
                let response = CoreLinkSharedClient::call(
                    &proxy,
                    CoreCallRequest::new(
                        request_id,
                        target,
                        "bindChatToWorkspace",
                        toCoreValue(json!({"chatId": inherited_chat_id, "workspace": path}))
                            .unwrap(),
                    ),
                )
                .await;
                if succeeds {
                    response
                        .result
                        .expect("repeat bind and retry must succeed without duplicate folders");
                } else {
                    let error = response
                        .result
                        .expect_err("invalid binding must return a proxy error, not panic");
                    if request_id == "flutter-name-conflict" {
                        assert!(error
                            .to_string()
                            .contains("duplicate workspace folder name"));
                    }
                }
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                let core = holder.coreForTarget(target).unwrap();
                let workspace = core
                    .chatHistoryDelegate
                    .chatHistoryManager
                    .getWorkspaceForChat(&inherited_chat_id)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    workspace, original_workspace,
                    "rebind/error must not mutate the existing workspace"
                );
            }

            let response = CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new(
                    "flutter-add-different-folder",
                    target,
                    "bindChatToWorkspace",
                    toCoreValue(
                        json!({"chatId": inherited_chat_id, "workspace": "/app/workspaces/extra"}),
                    )
                    .unwrap(),
                ),
            )
            .await;
            response
                .result
                .expect("a different folder must still be mountable after a bind error");
            let holder = proxy.chatRuntimeHolder();
            let mut holder = holder.lock().await;
            let core = holder.coreForTarget(target).unwrap();
            let workspace = core
                .chatHistoryDelegate
                .chatHistoryManager
                .getWorkspaceForChat(&inherited_chat_id)
                .unwrap()
                .unwrap();
            assert_eq!(workspace.id, original_workspace.id);
            assert_eq!(workspace.folders.len(), 2);
            assert_eq!(workspace.folders[0], original_workspace.folders[0]);
            assert_eq!(workspace.folders[1].path, "/app/workspaces/extra");
        })
        .await;
}
