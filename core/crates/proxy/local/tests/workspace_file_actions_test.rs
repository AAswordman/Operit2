use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use operit_host_api::HostManager::HostManager;
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost, NativeRuntimeStorageHost,
    PosixFileSystemHost,
};
use operit_link::{toCoreValue, CoreCallRequest, CoreLinkSharedClient};
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_file_actions_use_the_generated_bridge_and_preserve_existing_data() {
    tokio::task::LocalSet::new().run_until(async {
        let root = std::env::temp_dir().join(format!("operit-file-actions-{}-{}", std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        let runtime_root = root.join("runtime");
        let workspace_root = root.join("workspaces");
        let project = workspace_root.join("project");
        std::fs::create_dir_all(&runtime_root).unwrap();
        std::fs::create_dir_all(project.join("src/nested")).unwrap();
        std::fs::write(project.join("src/nested/data.bin"), [0, 255, 1, 128]).unwrap();
        std::fs::write(project.join("report.txt"), "original").unwrap();
        std::fs::write(project.join("existing.txt"), "keep me").unwrap();
        let storage = Arc::new(NativeRuntimeStorageHost::new(runtime_root, workspace_root));
        let context = HostManager {
            fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
            runtimeStorageHost: Some(storage.clone()),
            runtimeSqliteHost: Some(storage),
            hostJavaScriptRuntimeHost: Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
            hostRuntimeTaskSchedulerHost: Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new())),
            ..HostManager::default()
        };
        let proxy = LocalCoreProxy::new(OperitApplication::newWithContext(context));
        let chat_target = LocalCoreProxy::generatedTargetForSchema("chatRuntimeHolderMain").unwrap();
        let service = LocalCoreProxy::generatedTargetForSchema("services.workspaceService").unwrap();
        let chat_id = {
            let holder = proxy.chatRuntimeHolder();
            let mut holder = holder.lock().await;
            let core = holder.coreForTarget(chat_target).unwrap();
            let manager = &core.chatHistoryDelegate.chatHistoryManager;
            let draft = manager.newChatDraft("File actions".into(), Default::default(), None, None).unwrap();
            let chat = manager.commitChatDraft(draft, None).unwrap();
            core.chatHistoryDelegate.currentChatIdFlow.set_value(Some(chat.id.clone()));
            chat.id
        };
        macro_rules! mutate {
            ($operation:expr, $path:expr, $destination:expr) => {
                CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("file-action", service,
                    "mutateWorkspaceFile", toCoreValue(json!({"chatId": chat_id,
                        "operation": $operation, "relativePath": $path,
                        "destinationPath": $destination})).unwrap())).await.result
            };
        }
        // Unbound chat output files use the same operations and cannot escape their base.
        CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("write-output", service,
            "writeWorkspaceTextFile", toCoreValue(json!({"chatId": chat_id,
                "relativePath": "output.txt", "content": "output"})).unwrap())).await.result.unwrap();
        mutate!("copy", "output.txt", "copy.txt").unwrap();
        mutate!("move", "copy.txt", "moved.txt").unwrap();
        mutate!("delete", "moved.txt", None::<String>).unwrap();
        assert!(mutate!("delete", "../output.txt", None::<String>).is_err());
        assert!(mutate!("delete", "", None::<String>).is_err());

        let folder_name = {
            let holder = proxy.chatRuntimeHolder();
            let mut holder = holder.lock().await;
            let core = holder.coreForTarget(chat_target).unwrap();
            let manager = &core.chatHistoryDelegate.chatHistoryManager;
            let workspace = manager.createWorkspace("Project".into(), vec![
                operit_model::Workspace::WorkspaceFolder {
                    name: "project".into(), path: "/operitapp/workspaces/project".into(),
                },
            ]).unwrap();
            manager.updateChatWorkspaceId(chat_id.clone(), Some(workspace.id)).unwrap();
            "project".to_string()
        };
        let path = |child: &str| format!("{folder_name}/{child}");
        mutate!("copy", path("report.txt"), path("copy.txt")).unwrap();
        assert_eq!(std::fs::read_to_string(project.join("copy.txt")).unwrap(), "original");
        mutate!("move", path("copy.txt"), path("renamed.txt")).unwrap();
        assert!(!project.join("copy.txt").exists());
        assert!(mutate!("move", path("report.txt"), path("existing.txt")).is_err());
        assert_eq!(std::fs::read_to_string(project.join("existing.txt")).unwrap(), "keep me");
        assert_eq!(std::fs::read_to_string(project.join("report.txt")).unwrap(), "original");
        assert!(mutate!("copy", path("src"), path("src/nested/child")).is_err());
        assert!(mutate!("move", path("src"), path("src")).is_err());
        assert!(mutate!("delete", folder_name.clone(), None::<String>).is_err());
        assert!(mutate!("move", folder_name.clone(), path("renamed-root")).is_err());
        assert!(mutate!("delete", path("../existing.txt"), None::<String>).is_err());
        assert!(mutate!("unsupported", path("report.txt"), None::<String>).is_err());
        assert!(mutate!("delete", path("missing.txt"), None::<String>).is_err());
        mutate!("copy", path("src"), path("src-copy")).unwrap();
        assert_eq!(std::fs::read(project.join("src-copy/nested/data.bin")).unwrap(), [0, 255, 1, 128]);
        mutate!("move", path("src-copy"), path("src-moved")).unwrap();
        assert!(!project.join("src-copy").exists());
        mutate!("delete", path("src-moved"), None::<String>).unwrap();
        assert!(!project.join("src-moved").exists());
        mutate!("delete", path("renamed.txt"), None::<String>).unwrap();
        assert!(!project.join("renamed.txt").exists());
        assert!(project.join("src/nested/data.bin").exists());
        std::fs::remove_dir_all(root).unwrap();
    }).await;
}
