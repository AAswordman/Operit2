use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_host_native_filesystem::PosixFileSystemHost;
use operit_host_native_http::NativeHttpHost;
use operit_host_native_scheduler::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost,
};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_tools::runtime_support::{
    CoreNodeToolRuntime, RuntimeCoreNodeRouteState, RuntimeCoreNodeStatus,
};
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_util::FileUtils::FileUtils;
use operit_vfs::VfsPath;
use serde_json::{json, Value};

/// Supplies the same local-node ownership route installed by the production CLI proxy.
struct LocalNodeRoute(String);

impl CoreNodeToolRuntime for LocalNodeRoute {
    /// Resolves qualified folder paths locally without a network transport.
    fn coreNodeRouteState(&self) -> Result<RuntimeCoreNodeRouteState, String> {
        Ok(RuntimeCoreNodeRouteState {
            currentNodeId: self.0.clone(),
            nodes: vec![RuntimeCoreNodeStatus {
                nodeId: self.0.clone(),
                displayName: "Workspace fixture".into(),
                userName: String::new(),
                platform: "test".into(),
                model: "test".into(),
                reachable: true,
                canExecute: true,
                relayHops: None,
                relayPath: None,
            }],
        })
    }
}

/// Exercises persisted workspace identity, mounts and chat bindings through the public CLI router.
#[test]
fn workspace_commands_keep_identity_separate_from_mounted_directories() {
    std::thread::Builder::new()
        .name("cli-workspace-regression".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(Box::pin(exercise_workspaces()));
        })
        .unwrap()
        .join()
        .unwrap();
}

/// Uses real stores and folders to cover creation before binding, shared mutations and file preservation.
async fn exercise_workspaces() {
    let root = std::env::temp_dir().join(format!("operit-cli-workspaces-{}", uuid_for_fixture()));
    let project = root.join("app");
    let other = root.join("other folder");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    std::fs::create_dir_all(root.join("runtime")).unwrap();
    std::fs::create_dir_all(root.join("workspaces")).unwrap();
    std::fs::write(project.join("sentinel.txt"), "original project file").unwrap();
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));
    let mut hosts = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
    hosts.httpHost = Some(Arc::new(NativeHttpHost::new()));
    hosts.runtimeStorageHost = Some(storage.clone());
    hosts.runtimeSqliteHost = Some(storage);
    hosts.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    hosts.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    let mut app = OperitApplication::newWithContext(hosts);
    let node_id = operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::native()
        .identity()
        .unwrap()
        .nodeId;
    app.toolHandler
        .runtimeSupport()
        .bindCoreNodeToolRuntime(Arc::new(LocalNodeRoute(node_id)))
        .unwrap();
    operit_runtime::plugins::toolpkg::ToolPkgChatLifecycleHookBridge::ToolPkgChatLifecycleHookBridge::register(
        app.toolPkgBridgeRuntime.clone(),
    ).unwrap();
    operit_runtime::data::preferences::UserPreferencesManager::UserPreferencesManager::getInstance(
    )
    .saveAppLanguage("en".into())
    .unwrap();
    RuntimePackageManager::reloadRegistry(app.packageManager())
        .await
        .unwrap();

    let created = execute(&mut app, &["workspace", "create", "Independent project"])
        .await
        .unwrap();
    let id = created["workspaceId"].as_str().unwrap().to_owned();
    assert_ne!(id, "Independent project");
    let listing = execute(&mut app, &["workspace", "list"]).await.unwrap();
    assert!(listing["workspaces"]
        .as_array()
        .unwrap()
        .iter()
        .any(|workspace| workspace["id"] == id));
    assert_eq!(listing["chatCounts"][&id], 0);
    let default_path = created["workspace"]["folders"][0]["path"].as_str().unwrap();
    let default_directory = FileUtils::splitQualifiedVfsPath(default_path)
        .unwrap()
        .1
        .rsplit('/')
        .next()
        .unwrap()
        .to_owned();
    assert!(root.join("workspaces").join(&default_directory).is_dir());

    let project_native = project.to_string_lossy().into_owned();
    let other_native = other.to_string_lossy().into_owned();
    let project_vfs = VfsPath::nativePathToVfs(&project_native).unwrap();
    let mounted = execute(
        &mut app,
        &[
            "workspace",
            "mount",
            &id,
            "project",
            "--native",
            &project_native,
            "--primary",
        ],
    )
    .await
    .unwrap();
    assert_eq!(mounted["workspaceId"], id);
    assert_eq!(mounted["workspace"]["folders"][0]["name"], "project");
    let address = mounted["workspace"]["folders"][0]["path"].as_str().unwrap();
    assert_eq!(
        FileUtils::splitQualifiedVfsPath(address).unwrap().1,
        project_vfs
    );
    let repeated = execute(
        &mut app,
        &["workspace", "mount", &id, "project", "--vfs", &project_vfs],
    )
    .await
    .unwrap();
    assert_eq!(
        repeated["workspace"]["folders"],
        mounted["workspace"]["folders"]
    );
    let missing = root.join("missing").to_string_lossy().into_owned();
    for args in [
        vec![
            "workspace",
            "mount",
            &id,
            "bad/name",
            "--native",
            &other_native,
        ],
        vec!["workspace", "mount", &id, "missing", "--native", &missing],
    ] {
        assert!(execute(&mut app, &args).await.is_err());
    }
    let shown = execute(&mut app, &["workspace", "show", &id])
        .await
        .unwrap();
    assert_eq!(
        shown["workspace"]["folders"],
        mounted["workspace"]["folders"]
    );
    let duplicate = execute(
        &mut app,
        &[
            "workspace",
            "mount",
            &id,
            "alias",
            "--native",
            &project_native,
            "--primary",
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        duplicate["workspace"]["folders"],
        mounted["workspace"]["folders"]
    );
    let replaced = execute(
        &mut app,
        &[
            "workspace",
            "mount",
            &id,
            "project",
            "--native",
            &other_native,
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        replaced["workspace"]["folders"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        FileUtils::splitQualifiedVfsPath(
            replaced["workspace"]["folders"][0]["path"]
                .as_str()
                .unwrap()
        )
        .unwrap()
        .1,
        VfsPath::nativePathToVfs(&other_native).unwrap()
    );
    execute(
        &mut app,
        &[
            "workspace",
            "mount",
            &id,
            "project",
            "--native",
            &project_native,
        ],
    )
    .await
    .unwrap();
    execute(&mut app, &["workspace", "primary", &id, "default"])
        .await
        .unwrap();
    let primary = execute(&mut app, &["workspace", "primary", &id, "project"])
        .await
        .unwrap();
    assert_eq!(
        primary["workspace"]["folders"],
        mounted["workspace"]["folders"]
    );

    let chat = execute(&mut app, &["chat", "new", "--workspace", &id])
        .await
        .unwrap();
    let chat_id = chat["chatId"].as_str().unwrap().to_owned();
    execute(&mut app, &["workspace", "unbind", &chat_id])
        .await
        .unwrap();
    assert!(execute(&mut app, &["workspace", "unbind", "missing-chat"])
        .await
        .is_err());
    execute(&mut app, &["workspace", "bind", &chat_id, &id])
        .await
        .unwrap();
    let chats = execute(&mut app, &["workspace", "chats", &id])
        .await
        .unwrap();
    assert!(chats["chats"]
        .as_array()
        .unwrap()
        .iter()
        .any(|chat| chat["id"] == chat_id));
    assert!(
        execute(&mut app, &["workspace", "bind", &chat_id, &project_native])
            .await
            .is_err()
    );
    assert!(execute(&mut app, &["workspace", "chats", "missing-id"])
        .await
        .is_err());

    std::fs::create_dir_all(project.join(".operit")).unwrap();
    std::fs::write(
        project.join(".operit/config.json"),
        json!({"commands": [{
            "id": "read", "label": "Read project sentinel", "tool": "read_file_full",
            "toolParameters": {"path": "sentinel.txt"}
        }]})
        .to_string(),
    )
    .unwrap();
    let commands = execute(&mut app, &["workspace", "commands", &id])
        .await
        .unwrap();
    assert_eq!(commands["commands"][0]["id"], "read");
    let ran = execute(&mut app, &["workspace", "run", &id, "read"])
        .await
        .unwrap();
    assert_eq!(ran["workspaceId"], id);
    assert!(ran["resultText"]
        .as_str()
        .unwrap()
        .contains("original project file"));

    let external = execute(
        &mut app,
        &[
            "workspace",
            "create",
            "Existing folder",
            "--native",
            &project_native,
        ],
    )
    .await
    .unwrap();
    let external_id = external["workspaceId"].as_str().unwrap().to_owned();
    execute(&mut app, &["workspace", "bind", &chat_id, &external_id])
        .await
        .unwrap();
    assert!(execute(&mut app, &["workspace", "chats", &id])
        .await
        .unwrap()["chats"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(
        execute(&mut app, &["workspace", "unmount", &external_id, "default"])
            .await
            .is_err()
    );
    execute(&mut app, &["workspace", "unmount", &id, "project"])
        .await
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(project.join("sentinel.txt")).unwrap(),
        "original project file"
    );
    execute(&mut app, &["workspace", "delete", "--yes", &external_id])
        .await
        .unwrap();
    execute(&mut app, &["workspace", "delete", "--yes", &id])
        .await
        .unwrap();
    assert!(!root.join("workspaces").join(&default_directory).exists());
    assert!(project.join("sentinel.txt").exists());
    assert!(execute(&mut app, &["workspace", "show", &id])
        .await
        .is_err());
}

/// Generates an isolated fixture name through the workspace model's existing UUID owner.
fn uuid_for_fixture() -> String {
    operit_model::Workspace::Workspace::fromSingleFolder(
        "fixture".into(),
        "fixture".into(),
        "/fixture".into(),
        0,
    )
    .id
}

/// Requires a structured result from the same Core entry point used by the local and daemon clients.
async fn execute(app: &mut OperitApplication, args: &[&str]) -> Result<Value, String> {
    let args = std::iter::once("--json")
        .chain(args.iter().copied())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let output = crate::run_core_command(app, &args).await?;
    serde_json::from_str(&output.stdout).map_err(|error| error.to_string())
}
