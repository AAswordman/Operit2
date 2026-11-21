use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use operit_host_api::HostManager::HostManager;
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost, NativeRuntimeStorageHost, NativeHttpHost,
    PosixFileSystemHost,
};
use operit_link::{fromCoreValue, toCoreValue, CoreCallRequest, CoreLinkSharedClient};
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::services::WorkspaceService::WorkspaceFileEntry;
use operit_tools::runtime_support::{CoreNodeToolRuntime, RuntimeCoreNodeRouteState, RuntimeCoreNodeStatus};
use operit_tools::tools::DeviceTools::{DeviceToolRuntime, result_to_js, tool_from_request};
use operit_tools::tools::defaultTool::standard::StandardFileSystemTools::StandardFileSystemTools;
use operit_plugin_sdk::javascript::{JsExecutionCompletion, JsToolCallRequest, JsToolCallResult};
use serde_json::json;

/// Supplies mutable reachability through the production node-routing interface.
struct RouteState(Mutex<RuntimeCoreNodeRouteState>);
impl CoreNodeToolRuntime for RouteState {
    /// Returns the same device-space snapshot to workspace and ordinary file calls.
    fn coreNodeRouteState(&self) -> Result<RuntimeCoreNodeRouteState, String> {
        Ok(self.0.lock().unwrap().clone())
    }
}

/// Replaces only network transport; file operations run on a separate real VFS.
struct RemoteFiles {
    tools: StandardFileSystemTools,
    requests: Arc<Mutex<Vec<JsToolCallRequest>>>,
}
impl DeviceToolRuntime for RemoteFiles {
    /// Executes target-local paths after the existing file router selects the owner.
    fn dispatch(&self, request: JsToolCallRequest) -> JsExecutionCompletion<JsToolCallResult> {
        assert_eq!(request.device.as_ref().unwrap().node_id, "core-remote");
        self.requests.lock().unwrap().push(request.clone());
        let tools = self.tools.clone();
        Box::pin(async move {
            let tool = tool_from_request(&request);
            let result = match tool.name.as_str() {
                "list_files" => tools.listFiles(&tool).await,
                "file_exists" => tools.fileExists(&tool).await,
                "file_info" => tools.fileInfo(&tool).await,
                "read_file_binary" => tools.readFileBinary(&tool).await,
                "write_file_binary" => tools.writeFileBinary(&tool).await,
                "copy_file" => tools.copyFile(&tool).await,
                "move_file" => tools.moveFile(&tool).await,
                "delete_file" => tools.deleteFile(&tool).await,
                "__fs_transfer_begin" => tools.fsTransferBegin(&tool),
                "__fs_transfer_chunk" => tools.fsTransferChunk(&tool),
                "__fs_transfer_commit" => tools.fsTransferCommit(&tool),
                "__fs_read_chunk" => tools.fsReadChunk(&tool),
                "open_file" => operit_tools::ConversationMarkupManager::ToolResult {
                    toolName: tool.name.clone(), success: true, error: None,
                    result: operit_tools::tools::ToolResultDataClasses::ToolResultData::FileOperationData(
                        operit_tools::tools::ToolResultDataClasses::FileOperationData {
                            operation: "open".into(), successful: true, details: "test opener".into(),
                            path: tool.parameters.iter().find(|parameter| parameter.name == "path").unwrap().value.clone(),
                        },
                    ),
                },
                other => panic!("unexpected remote file operation: {other}"),
            };
            result_to_js(result)
        })
    }
}

/// Describes an executable test device without coupling it to chat bindings.
fn device(node: &str) -> RuntimeCoreNodeStatus {
    RuntimeCoreNodeStatus {
        nodeId: node.into(), displayName: node.into(), userName: String::new(),
        platform: "test".into(), model: "test".into(), reachable: true, canExecute: true,
        relayHops: None, relayPath: None,
    }
}

/// Releases application-owned database handles before the parent removes test storage.
#[test]
fn workspace_file_actions_use_the_generated_bridge_and_preserve_existing_data() {
    let root = std::env::temp_dir().join(format!("operit-file-actions-{}-{}", std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "workspace_file_actions_worker", "--ignored", "--nocapture"])
        .env("OPERIT_WORKSPACE_FILE_ACTIONS_ROOT", &root).output().unwrap();
    assert!(output.status.success(), "workspace worker failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    let resolved = root.canonicalize().unwrap();
    assert_eq!(resolved.parent(), Some(std::env::temp_dir().canonicalize().unwrap().as_path()));
    std::fs::remove_dir_all(resolved).unwrap();
}

/// Runs generated workspace calls and two real filesystems in an isolated application process.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "Executed by the parent with an isolated storage directory"]
async fn workspace_file_actions_worker() {
    let root = std::path::PathBuf::from(std::env::var_os("OPERIT_WORKSPACE_FILE_ACTIONS_ROOT").unwrap());
    tokio::task::LocalSet::new().run_until(async {
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
            httpHost: Some(Arc::new(NativeHttpHost::new())),
            fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
            runtimeStorageHost: Some(storage.clone()),
            runtimeSqliteHost: Some(storage),
            hostJavaScriptRuntimeHost: Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
            hostRuntimeTaskSchedulerHost: Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new())),
            ..HostManager::default()
        };
        let application = OperitApplication::newWithContext(context.clone());
        let handler = application.aiToolHandler();
        let proxy = LocalCoreProxy::new(application);
        let local_node = operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::native().identity().unwrap().nodeId;
        let routes = Arc::new(RouteState(Mutex::new(RuntimeCoreNodeRouteState {
            currentNodeId: local_node.clone(), nodes: vec![device(&local_node), device("core-remote"), device("core-chat")],
        })));
        proxy.bindCoreNodeToolRuntime(routes.clone()).unwrap();
        let chat_target = LocalCoreProxy::generatedTargetForSchema("chatRuntimeHolderMain").unwrap();
        let service = LocalCoreProxy::generatedTargetForSchema("application.workspaceService").unwrap();
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

        let source = format!("operit-mount:{}", json!({
            "namespace": "/virtual/local/folders", "backend": "native",
            "root": project, "name": "mounted-project",
        }));
        CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("bind-mount", chat_target,
            "bindChatToWorkspace", toCoreValue(json!({"chatId": chat_id, "workspace": source})).unwrap()))
            .await.result.unwrap();
        let roots: Vec<WorkspaceFileEntry> = fromCoreValue(CoreLinkSharedClient::call(&proxy,
            CoreCallRequest::new("mount-roots", service, "listWorkspaceFiles",
                toCoreValue(json!({"chatId": chat_id, "relativePath": ""})).unwrap()))
            .await.result.unwrap()).unwrap();
        let mounted = roots.iter().find(|entry| entry.name == "mounted-project").unwrap();
        assert!(mounted.path.starts_with(&format!("{local_node}://virtual/local/folders/")));

        let remote_project = root.join("remote-project");
        std::fs::create_dir_all(remote_project.join("src")).unwrap();
        std::fs::write(remote_project.join("report.txt"), "remote content").unwrap();
        std::fs::write(remote_project.join("src/remote.txt"), "remote nested").unwrap();
        let remote_storage = Arc::new(NativeRuntimeStorageHost::new(root.join("remote-runtime"), root.join("remote-workspaces")));
        let mut mount = operit_vfs::MountRegistry::MountRegistry::withStorage(context.runtimeStorageHost.clone().unwrap())
            .list().unwrap().into_iter().find(|mount| mount.name == "mounted-project").unwrap();
        mount.root = remote_project.to_string_lossy().into_owned();
        operit_host_api::RuntimeStorageHost::writeBytesAtomically(remote_storage.as_ref(), "runtime/config/vfs_mounts.json",
            &serde_json::to_vec(&json!({"version": 1, "mounts": [mount]})).unwrap()).unwrap();
        let remote_context = HostManager { runtimeStorageHost: Some(remote_storage.clone()), ..context.clone() };
        let remote_tools = StandardFileSystemTools::new(remote_context.vfs().unwrap(), context.httpHost.clone().unwrap(),
            None, remote_storage, handler.runtimeSupport());
        let requests = Arc::new(Mutex::new(Vec::new()));
        proxy.bindDeviceToolRuntime(Arc::new(RemoteFiles { tools: remote_tools, requests: requests.clone() })).unwrap();
        let (_, mount_path) = operit_util::FileUtils::FileUtils::splitQualifiedVfsPath(&mounted.path).unwrap();
        let remote_address = operit_util::FileUtils::FileUtils::qualifyVfsPath("core-remote", mount_path);
        CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("bind-remote", chat_target,
            "bindChatToWorkspace", toCoreValue(json!({"chatId": chat_id, "workspace": remote_address})).unwrap()))
            .await.result.unwrap();
        let remote_name = mount_path.rsplit('/').next().unwrap();
        let remote_path = |child: &str| format!("{remote_name}/{child}");
        let bindings = operit_store::CoreNodeBindingStore::CoreNodeBindingStore::default().unwrap();
        bindings.compareAndSet(&chat_id, &local_node, "core-chat").unwrap();

        let entries: Vec<WorkspaceFileEntry> = fromCoreValue(CoreLinkSharedClient::call(&proxy,
            CoreCallRequest::new("remote-list", service, "listWorkspaceFiles",
                toCoreValue(json!({"chatId": chat_id, "relativePath": remote_name})).unwrap()))
            .await.result.unwrap()).unwrap();
        assert!(entries.iter().all(|entry| entry.path.starts_with(&remote_address)));
        let text: String = fromCoreValue(CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("remote-read", service,
            "readWorkspaceTextFile", toCoreValue(json!({"chatId": chat_id, "relativePath": remote_path("report.txt")})).unwrap()))
            .await.result.unwrap()).unwrap();
        assert_eq!(text, "remote content");
        let result = operit_tools::ToolExecutionManager::ToolExecutionManager::scopeToolRuntimeContext(
            operit_tools::ToolExecutionManager::ToolRuntimeContext {
                callerName: None,
                callerChatId: Some(chat_id.clone()), callerParticipantId: None,
                workspacePath: Some(remote_address.clone()), workspaceFolders: vec![remote_address.clone()],
                toolExposureMode: operit_tools::ToolExecutionManager::ToolExposureMode::FULL,
                extensionOwner: "test-owner".into(), executionData: Default::default(),
            },
            handler.clone().executeTool(operit_tools::ToolExecutionManager::AITool {
                name: "read_file_binary".into(), parameters: vec![operit_tools::ToolExecutionManager::ToolParameter {
                    name: "path".into(), value: "report.txt".into(),
                }],
            }),
        ).await;
        assert!(result.success, "{:?}", result.error);
        let operit_tools::tools::ToolResultDataClasses::ToolResultData::BinaryFileContentData(bytes) = result.result else {
            panic!("relative file access must return binary content");
        };
        assert_eq!(bytes.path, format!("{remote_address}/report.txt"));
        assert_eq!(bytes.contentBase64, "cmVtb3RlIGNvbnRlbnQ=");
        assert_eq!(std::fs::read_to_string(project.join("report.txt")).unwrap(), "original");
        CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("remote-write", service, "writeWorkspaceTextFile",
            toCoreValue(json!({"chatId": chat_id, "relativePath": remote_path("new.txt"), "content": "remote save"})).unwrap()))
            .await.result.unwrap();
        assert_eq!(std::fs::read_to_string(remote_project.join("new.txt")).unwrap(), "remote save");
        assert!(!project.join("new.txt").exists());
        assert!(mutate!("copy", remote_path("src"), remote_path("src/child")).is_err());
        mutate!("copy", remote_path("new.txt"), remote_path("copy.txt")).unwrap();
        mutate!("move", remote_path("copy.txt"), remote_path("moved.txt")).unwrap();
        mutate!("delete", remote_path("moved.txt"), None::<String>).unwrap();
        assert!(mutate!("move", remote_path("new.txt"), remote_path("report.txt")).is_err());
        assert!(mutate!("delete", remote_name, None::<String>).is_err());
        assert!(mutate!("delete", remote_path("../report.txt"), None::<String>).is_err());
        CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("remote-open", service, "openWorkspaceFile",
            toCoreValue(json!({"chatId": chat_id, "relativePath": remote_path("report.txt")})).unwrap()))
            .await.result.unwrap();
        let suggestions: Vec<WorkspaceFileEntry> = fromCoreValue(CoreLinkSharedClient::call(&proxy,
            CoreCallRequest::new("remote-mentions", service, "listMentionWorkspaceSuggestions",
                toCoreValue(json!({"chatId": chat_id, "searchQuery": "remote.txt"})).unwrap()))
            .await.result.unwrap()).unwrap();
        assert!(suggestions.iter().any(|entry| entry.path == format!("{remote_address}/src/remote.txt")));
        mutate!("copy", "project/report.txt", remote_path("from-local.txt")).unwrap();
        assert_eq!(std::fs::read_to_string(remote_project.join("from-local.txt")).unwrap(), "original");
        mutate!("move", remote_path("new.txt"), "project/from-remote.txt").unwrap();
        assert_eq!(std::fs::read_to_string(project.join("from-remote.txt")).unwrap(), "remote save");
        assert!(!remote_project.join("new.txt").exists());
        {
            let holder = proxy.chatRuntimeHolder();
            let mut holder = holder.lock().await;
            let core = holder.coreForTarget(chat_target).unwrap();
            core.handleAttachment(format!("workspace_mention:{}", remote_path("report.txt"))).await;
            assert!(core.attachments.iter().any(|entry| entry.content.contains("File content:\nremote content")));
        }
        assert!(requests.lock().unwrap().iter().any(|request| request.tool_name == "open_file"));
        let calls_before = requests.lock().unwrap().len();
        routes.0.lock().unwrap().nodes.iter_mut().find(|node| node.nodeId == "core-remote").unwrap().reachable = false;
        let error = CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("offline", service, "readWorkspaceTextFile",
            toCoreValue(json!({"chatId": chat_id, "relativePath": remote_path("report.txt")})).unwrap()))
            .await.result.unwrap_err();
        assert!(error.message.contains("unreachable"));
        assert_eq!(requests.lock().unwrap().len(), calls_before);
        assert_eq!(std::fs::read_to_string(project.join("report.txt")).unwrap(), "original");
    }).await;
}
