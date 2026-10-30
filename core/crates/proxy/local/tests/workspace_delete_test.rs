use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use operit_host_api::HostManager::HostManager;
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost, NativeRuntimeStorageHost,
    PosixFileSystemHost,
};
use operit_link::{toCoreValue, CoreCallRequest, CoreLinkSharedClient};
use operit_model::ChatMessage::ChatMessage;
use operit_model::Workspace::WorkspaceFolder;
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};
use serde_json::json;

/// Exercises the sidebar's generated deletion route with real storage, mounts, and chat metadata.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_deletion_preserves_chats_and_mounted_files() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let root = std::env::temp_dir().join(format!(
                "operit-proxy-workspace-delete-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
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
            let hosts = HostManager {
                fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
                runtimeStorageHost: Some(storage.clone()),
                runtimeSqliteHost: Some(storage),
                hostJavaScriptRuntimeHost: Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
                hostRuntimeTaskSchedulerHost: Some(Arc::new(
                    NativeHostRuntimeTaskSchedulerHost::new(),
                )),
                ..HostManager::default()
            };
            let proxy = LocalCoreProxy::new(OperitApplication::newWithContext(hosts));
            let target = LocalCoreProxy::generatedTargetForSchema("chatRuntimeHolderMain").unwrap();
            let project = root.join("workspaces/project");
            let extra = root.join("workspaces/extra");
            let external = root.join("external");
            std::fs::create_dir_all(project.join("nested")).unwrap();
            std::fs::create_dir_all(&extra).unwrap();
            std::fs::create_dir_all(&external).unwrap();
            std::fs::write(project.join("nested/keep.txt"), "shared area").unwrap();
            std::fs::write(extra.join("delete.txt"), "default area").unwrap();
            std::fs::write(external.join("keep.txt"), "mounted original").unwrap();
            let registry = operit_tools::files::MountRegistry::MountRegistry::withStorage(
                operit_store::RuntimeStorageHost::defaultRuntimeStorageHost(),
            );
            let private_mount = registry
                .register(
                    "/mnt/macos/folders",
                    "native",
                    external.to_str().unwrap(),
                    "External",
                )
                .unwrap();
            let shared_mount = registry
                .register(
                    "/mnt/macos/folders",
                    "native",
                    project.to_str().unwrap(),
                    "Shared",
                )
                .unwrap();
            let folder = |name: &str, path: String| WorkspaceFolder {
                name: name.into(),
                path,
            };
            let (workspace, survivor, chats, messages) = {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                let core = holder.coreForTarget(target).unwrap();
                let manager = &core.chatHistoryDelegate.chatHistoryManager;
                let workspace = manager
                    .createWorkspace(
                        "Project".into(),
                        vec![
                            folder("project", "/app/workspaces/project".into()),
                            folder("extra", "/app/workspaces/extra".into()),
                            folder("external", private_mount.vfsPath()),
                            folder("shared", shared_mount.vfsPath()),
                        ],
                    )
                    .unwrap();
                let survivor = manager
                    .createWorkspace(
                        "Survivor".into(),
                        vec![
                            folder("nested", "/app/workspaces/project/nested".into()),
                            folder("shared", shared_mount.vfsPath()),
                        ],
                    )
                    .unwrap();
                // Seed canonical chats directly so this fixture does not require plugin creation hooks.
                let mut chats = Vec::new();
                for title in ["First", "Second"] {
                    let mut draft = manager
                        .newChatDraft(
                            title.into(),
                            Default::default(),
                            Some(workspace.id.clone()),
                            None,
                        )
                        .unwrap();
                    draft
                        .messages
                        .push(ChatMessage::new_with_markdown("user".into(), title.into()));
                    chats.push(manager.commitChatDraft(draft, None).unwrap());
                }
                core.chatHistoryDelegate
                    .currentChatIdFlow
                    .set_value(Some(chats[0].id.clone()));
                let messages = chats
                    .iter()
                    .map(|chat| manager.loadChatMessages(&chat.id).unwrap())
                    .collect::<Vec<_>>();
                (workspace, survivor, chats, messages)
            };
            // A cleanup error must keep workspace metadata and chat bindings for retry.
            let catalog_path = root.join("runtime/config/vfs_mounts.json");
            let catalog = std::fs::read(&catalog_path).unwrap();
            std::fs::write(&catalog_path, "invalid catalog").unwrap();
            let failed = CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new(
                    "flutter-delete-failed",
                    target,
                    "deleteWorkspace",
                    toCoreValue(json!({"workspaceId": workspace.id})).unwrap(),
                ),
            )
            .await;
            assert!(failed.result.is_err());
            assert!(extra.join("delete.txt").exists());
            {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                let core = holder.coreForTarget(target).unwrap();
                let manager = &core.chatHistoryDelegate.chatHistoryManager;
                assert!(manager.getWorkspace(&workspace.id).unwrap().is_some());
                for chat in &chats {
                    assert_eq!(
                        manager
                            .loadChatHistory(chat.id.clone())
                            .unwrap()
                            .unwrap()
                            .workspaceId,
                        Some(workspace.id.clone())
                    );
                }
            }
            std::fs::write(&catalog_path, catalog).unwrap();
            let response = CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new(
                    "flutter-delete-workspace",
                    target,
                    "deleteWorkspace",
                    toCoreValue(json!({"workspaceId": workspace.id})).unwrap(),
                ),
            )
            .await;
            response
                .result
                .expect("workspace deletion must succeed through the generated proxy");
            assert!(!extra.exists(), "shared default area must be deleted");
            assert!(
                project.join("nested/keep.txt").exists(),
                "another workspace's folder must survive"
            );
            assert!(
                external.join("keep.txt").exists(),
                "mounted original files must survive"
            );
            assert_eq!(registry.list().unwrap(), vec![shared_mount]);
            {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                let core = holder.coreForTarget(target).unwrap();
                let manager = &core.chatHistoryDelegate.chatHistoryManager;
                assert!(manager.getWorkspace(&workspace.id).unwrap().is_none());
                assert_eq!(
                    manager.getWorkspace(&survivor.id).unwrap(),
                    Some(survivor.clone())
                );
                for (chat, original_messages) in chats.iter().zip(messages) {
                    let retained = manager.loadChatHistory(chat.id.clone()).unwrap().unwrap();
                    assert_eq!(retained.workspaceId, None);
                    assert_eq!(retained.title, chat.title);
                    assert_eq!(
                        manager.loadChatMessages(&chat.id).unwrap(),
                        original_messages
                    );
                }
                assert_eq!(
                    core.chatHistoryDelegate.currentChatIdFlow.value(),
                    Some(chats[0].id.clone())
                );
            }
            // Empty workspaces are deletable, and stale requests return a recoverable error.
            for (request_id, workspace_id, succeeds) in [
                ("flutter-delete-missing", workspace.id, false),
                ("flutter-delete-empty", survivor.id, true),
            ] {
                let response = CoreLinkSharedClient::call(
                    &proxy,
                    CoreCallRequest::new(
                        request_id,
                        target,
                        "deleteWorkspace",
                        toCoreValue(json!({"workspaceId": workspace_id})).unwrap(),
                    ),
                )
                .await;
                assert_eq!(response.result.is_ok(), succeeds);
            }
            assert!(registry.list().unwrap().is_empty());
            assert!(external.join("keep.txt").exists());
        })
        .await;
}
