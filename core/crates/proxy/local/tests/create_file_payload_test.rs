use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost, NativeHttpHost,
    NativeRuntimeStorageHost, PosixFileSystemHost,
};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_tools::tools::defaultTool::standard::StandardFileSystemTools::{
    FileSystemToolExecutor, FileSystemToolOperation,
};
use operit_tools::tools::defaultTool::ToolGetter::ToolGetter;
use operit_tools::ToolExecutionManager::{AITool, AsyncToolExecutor, ToolParameter};

/// Checks validation and the actual bytes written by the same create_file operation.
#[tokio::test]
async fn create_file_accepts_content_alias_and_prefers_nonblank_new() {
    let root = std::env::temp_dir().join(format!(
        "operit-create-file-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let runtime = root.join("runtime");
    let workspaces = root.join("workspaces");
    std::fs::create_dir_all(&runtime).unwrap();
    std::fs::create_dir_all(workspaces.join("project")).unwrap();
    let storage = Arc::new(NativeRuntimeStorageHost::new(runtime, workspaces.clone()));
    let hosts = HostManager {
        fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
        httpHost: Some(Arc::new(NativeHttpHost::new())),
        runtimeStorageHost: Some(storage.clone()),
        runtimeSqliteHost: Some(storage),
        hostJavaScriptRuntimeHost: Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
        hostRuntimeTaskSchedulerHost: Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new())),
        ..HostManager::default()
    };
    let application = OperitApplication::newWithContext(hosts.clone());
    let executor = FileSystemToolExecutor {
        tools: ToolGetter::getFileSystemTools(
            &hosts,
            application.toolRuntimeDependencies.shared_runtime_support(),
        )
        .unwrap(),
        operation: FileSystemToolOperation::CreateFile,
    };
    for (index, new, content, expected) in [
        (0, None, Some("  alias body\n"), "  alias body\n"),
        (1, Some(""), Some("alias"), "alias"),
        (2, Some(" \n"), Some("alias"), "alias"),
        (3, Some("canonical"), None, "canonical"),
        (4, Some("canonical"), Some("alias"), "canonical"),
    ] {
        let name = format!("payload-{index}.txt");
        let mut parameters = vec![ToolParameter {
            name: "path".into(),
            value: format!("/app/workspaces/project/{name}"),
        }];
        for (key, value) in [("new", new), ("content", content)] {
            if let Some(value) = value {
                parameters.push(ToolParameter {
                    name: key.into(),
                    value: value.into(),
                });
            }
        }
        let tool = AITool {
            name: "create_file".into(),
            parameters,
        };
        assert!(executor.validateParameters(&tool).valid);
        let result = executor.tools.createFile(&tool).await;
        assert!(result.success, "case {index}: {:?}", result.error);
        assert_eq!(
            std::fs::read_to_string(workspaces.join("project").join(name)).unwrap(),
            expected
        );
    }
    drop(executor);
    drop(application);
    std::fs::remove_dir_all(root).unwrap();
}
