//! Keeps legacy built-in skill updates visible through the Flutter catalog and import route.
use operit_host_api::HostManager::HostManager;
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost, NativeRuntimeStorageHost,
    PosixFileSystemHost,
};
use operit_link::{fromCoreValue, toCoreValue, CoreCallRequest, CoreLinkSharedClient};
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use serde_json::{json, Value};
use std::sync::Arc;

async fn call(proxy: &LocalCoreProxy, target: &str, method: &str, args: Value) -> Value {
    let response = CoreLinkSharedClient::call(
        proxy,
        CoreCallRequest::new(
            "skill-update-test",
            target,
            method,
            toCoreValue(args).unwrap(),
        ),
    )
    .await;
    fromCoreValue(
        response
            .result
            .unwrap_or_else(|error| panic!("{target}.{method}: {error:?}")),
    )
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn legacy_package_builder_is_updatable_through_the_extension_catalog() {
    tokio::task::LocalSet::new().run_until(async {
        let root = std::env::temp_dir().join(format!("operit-skill-update-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let runtime = root.join("runtime");
        std::fs::create_dir_all(&runtime).unwrap();
        let storage = Arc::new(NativeRuntimeStorageHost::new(runtime.clone(), root.join("workspaces")));
        // Seed an old installation before startup, with no display_name or source marker.
        let skill_dir = runtime.join("extensions/device/skills/PackageBuilder");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"),
            "---\nname: PackageBuilder\ndescription: Legacy package builder\n---\n# PackageBuilder\n").unwrap();
        let application = OperitApplication::newWithContext(HostManager {
            fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
            runtimeStorageHost: Some(storage.clone()), runtimeSqliteHost: Some(storage),
            hostJavaScriptRuntimeHost: Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
            hostRuntimeTaskSchedulerHost: Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new())),
            ..HostManager::default()
        });
        let proxy = LocalCoreProxy::new(application);
        let snapshot = call(&proxy, "core/application", "getExtensionCatalogSnapshot", json!({})).await;
        let skill = snapshot["installed"].as_array().unwrap().iter()
            .find(|entry| entry["kind"] == "skill" && entry["id"] == "PackageBuilder").unwrap();
        assert_eq!(skill["source"], "builtin");
        assert_eq!(skill["displayName"], "PackageBuilder");
        assert_eq!(skill["updateState"], "available");
        assert_eq!(skill["activationState"], "aiVisible");
        call(&proxy, "core/application.skillRepository", "importBundledExternalSkill",
            json!({"skillName": "PackageBuilder"})).await;
        let snapshot = call(&proxy, "core/application", "getExtensionCatalogSnapshot", json!({})).await;
        let skill = snapshot["installed"].as_array().unwrap().iter()
            .find(|entry| entry["kind"] == "skill" && entry["id"] == "PackageBuilder").unwrap();
        assert_eq!(skill["source"], "builtin");
        assert_eq!(skill["displayName"], "Operit 插件开发套件");
        assert_eq!(skill["updateState"], "current");
        assert_eq!(skill["activationState"], "aiVisible");
        std::fs::remove_dir_all(root).unwrap();
    }).await;
}
