use std::io::Write;
use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_host_native_filesystem::PosixFileSystemHost;
use operit_host_native_scheduler::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost,
};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use serde_json::{json, Value};

const CONTAINER_ID: &str = "io.github.test.cli_container";

/// Gives the complete application and command futures a dedicated test stack.
#[test]
fn package_commands_report_toolpkg_containers_truthfully() {
    std::thread::Builder::new()
        .name("cli-package-toolpkg-regression".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(Box::pin(exercise_package_commands()));
        })
        .unwrap()
        .join()
        .unwrap();
}

/// Imports a real ToolPkg container and checks list/show against its subpackage tools.
async fn exercise_package_commands() {
    let root = std::env::temp_dir().join(format!(
        "operit-cli-package-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(root.join("runtime")).unwrap();
    std::fs::create_dir_all(root.join("workspaces/demo")).unwrap();
    std::fs::create_dir_all(root.join("artifacts")).unwrap();
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));
    let mut hosts = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
    hosts.runtimeStorageHost = Some(storage.clone());
    hosts.runtimeSqliteHost = Some(storage.clone());
    hosts.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    hosts.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    hosts.systemOperationHost = Some(Arc::new(LanguageHost));
    let mut app = OperitApplication::newWithContext(hosts);
    {
        let package_manager = app.packageManager();
        let mut manager = package_manager.lock().unwrap();
        manager.claimInitialPackageRegistryLoad().unwrap();
        manager.loadAvailablePackages();
    }

    let archive_path = root.join("artifacts/cli.toolpkg");
    std::fs::write(&archive_path, container_archive()).unwrap();
    let import = execute(
        &mut app,
        &["package", "import", &archive_path.to_string_lossy()],
    )
    .await
    .unwrap();
    assert_eq!(import["result"]["packageName"], CONTAINER_ID);
    execute(&mut app, &["package", "enable", CONTAINER_ID])
        .await
        .unwrap();

    let list = execute(&mut app, &["package", "list"]).await.unwrap();
    let items = list.as_array().unwrap();
    let container = items
        .iter()
        .find(|item| item["name"] == CONTAINER_ID)
        .expect("container must appear in package list");
    assert_eq!(container["kind"], "toolpkg-container");
    assert_eq!(container["tools"], 2);
    assert_eq!(container["subpackages"], 1);
    let subpackage = items
        .iter()
        .find(|item| item["kind"] == "toolpkg-subpackage")
        .expect("subpackage must appear in package list");
    assert_eq!(subpackage["tools"], 2);
    assert_eq!(subpackage["enabled"], true);

    let show = execute(&mut app, &["package", "show", CONTAINER_ID])
        .await
        .unwrap();
    assert_eq!(show["kind"], "toolpkg-container");
    assert_eq!(show["toolCount"], 2);
    let subpackages = show["subpackages"].as_array().unwrap();
    assert_eq!(subpackages.len(), 1);
    assert_eq!(subpackages[0]["toolCount"], 2);
    assert_eq!(subpackages[0]["enabled"], true);

    let args = vec!["package".to_string(), "show".to_string(), CONTAINER_ID.to_string()];
    let text = crate::run_core_command(&mut app, &args).await.unwrap();
    assert!(text.stdout.contains("Tools: 2 (across subpackages)"));
    assert!(!text.stdout.contains("Tools: 0"));

    let subpackage_name = subpackages[0]["packageName"].as_str().unwrap();
    let subpackage_show = execute(&mut app, &["package", "show", subpackage_name])
        .await
        .unwrap();
    assert_eq!(subpackage_show["tools"].as_array().unwrap().len(), 2);
}

/// Builds one ToolPkg container whose single subpackage declares two tools.
fn container_archive() -> Vec<u8> {
    let manifest = json!({
        "schema_version": 1,
        "toolpkg_id": CONTAINER_ID,
        "api_version": "2.0.0",
        "version": "1.0.0",
        "main": "main.js",
        "public_api": "api.ts",
        "enabled_by_default": true,
        "subpackages": [{"id": "probe.child", "entry": "children/probe.child.js"}]
    });
    let metadata = json!({
        "name": "probe.child",
        "enabledByDefault": true,
        "tools": [
            {"name": "probe", "description": "Child probe", "parameters": []},
            {"name": "second", "description": "Second probe", "parameters": []}
        ]
    });
    let entries = vec![
        ("manifest.json".to_string(), manifest.to_string()),
        (
            "main.js".to_string(),
            "exports.registerToolPkg = function() {};".to_string(),
        ),
        ("api.ts".to_string(), "export {};".to_string()),
        (
            "children/probe.child.js".to_string(),
            format!(
                "/* METADATA {metadata} */\nexports.probe = function() {{ return 'child'; }};\nexports.second = function() {{ return 'second'; }};"
            ),
        ),
    ];
    let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, text) in entries {
        archive
            .start_file(
                name,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
        archive.write_all(text.as_bytes()).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

/// Invokes the shared command router and requires an explicit machine-readable result.
async fn execute(app: &mut OperitApplication, args: &[&str]) -> Result<Value, String> {
    let args = std::iter::once("--json")
        .chain(args.iter().copied())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let output = crate::run_core_command(app, &args).await?;
    serde_json::from_str(&output.stdout).map_err(|error| error.to_string())
}

struct LanguageHost;

#[allow(unused_variables)]
impl operit_host_api::SystemOperationHost for LanguageHost {
    fn getSystemLanguageCode(&self) -> operit_host_api::HostResult<String> {
        Ok("en".into())
    }
    fn sendNotification(&self, request: &operit_host_api::SystemNotificationRequest) -> operit_host_api::HostResult<()> {
        panic!("Unexpected host call: sendNotification")
    }
    fn modifySystemSetting(&self, namespace: &str, setting: &str, value: &str) -> operit_host_api::HostResult<operit_host_api::SystemSettingData> {
        panic!("Unexpected host call: modifySystemSetting")
    }
    fn getSystemSetting(&self, namespace: &str, setting: &str) -> operit_host_api::HostResult<operit_host_api::SystemSettingData> {
        panic!("Unexpected host call: getSystemSetting")
    }
    fn installApp(&self, source: &dyn operit_host_api::FileSource, timing: operit_host_api::AppInstallTiming) -> operit_host_api::HostResult<operit_host_api::AppOperationData> {
        panic!("Unexpected host call: installApp")
    }
    fn uninstallApp(&self, packageName: &str) -> operit_host_api::HostResult<operit_host_api::AppOperationData> {
        panic!("Unexpected host call: uninstallApp")
    }
    fn listInstalledApps(&self, includeSystemApps: bool) -> operit_host_api::HostResult<operit_host_api::AppListData> {
        panic!("Unexpected host call: listInstalledApps")
    }
    fn startApp(&self, packageName: &str) -> operit_host_api::HostResult<operit_host_api::AppOperationData> {
        panic!("Unexpected host call: startApp")
    }
    fn stopApp(&self, packageName: &str) -> operit_host_api::HostResult<operit_host_api::AppOperationData> {
        panic!("Unexpected host call: stopApp")
    }
    fn getNotifications(&self, limit: i32, includeOngoing: bool) -> operit_host_api::HostResult<operit_host_api::NotificationData> {
        panic!("Unexpected host call: getNotifications")
    }
    fn getAppUsageTime(&self, packageName: &str, sinceHours: i32, limit: i32, includeSystemApps: bool) -> operit_host_api::HostResult<operit_host_api::AppUsageTimeResultData> {
        panic!("Unexpected host call: getAppUsageTime")
    }
    fn getDeviceLocation(&self, timeout: i32, highAccuracy: bool, includeAddress: bool) -> operit_host_api::HostResult<operit_host_api::LocationData> {
        panic!("Unexpected host call: getDeviceLocation")
    }
    fn getDeviceInfo(&self) -> operit_host_api::HostResult<operit_host_api::DeviceInfoData> {
        panic!("Unexpected host call: getDeviceInfo")
    }
    fn captureScreenshot(&self) -> operit_host_api::HostResult<String> {
        panic!("Unexpected host call: captureScreenshot")
    }
    fn recognizeText(&self, imagePath: &dyn operit_host_api::FileSource, language: operit_host_api::OCRLanguage, quality: operit_host_api::OCRQuality) -> operit_host_api::HostResult<String> {
        panic!("Unexpected host call: recognizeText")
    }
}
