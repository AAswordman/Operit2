use serde_json::json;

use crate::{
    loadToolPkgFile, repoFixtureToolPkgPath, ToolPkgMainHookCall, ToolPkgRuntime,
    ToolPkgRuntimeOptions,
};

fn ensure_test_runtime_root() {
    let root = std::env::temp_dir().join(format!(
        "toolpkg-runtime-sdk-tests-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("test runtime root");
    operit_store::RuntimeStorePaths::setDefaultRuntimeStoreRoot(root);
}

#[test]
fn loads_dino_runner_toolpkg() {
    ensure_test_runtime_root();
    let path = repoFixtureToolPkgPath("dino_runner.toolpkg");
    let outcome = loadToolPkgFile(path, "en").expect("dino runner toolpkg should load");
    assert_eq!(
        outcome.package.containerRuntime.packageName,
        "com.operit.dino_runner"
    );
    assert_eq!(outcome.package.containerRuntime.uiRoutes.len(), 1);
    assert_eq!(outcome.package.containerRuntime.navigationEntries.len(), 1);
    assert!(outcome.packageLoadErrors.is_empty());
}

#[test]
fn loads_message_insert_toolpkg_hooks() {
    ensure_test_runtime_root();
    let path = repoFixtureToolPkgPath("message_insert.toolpkg");
    let outcome = loadToolPkgFile(path, "en").expect("message insert toolpkg should load");
    assert_eq!(
        outcome.package.containerRuntime.packageName,
        "com.operit.message_insert_bundle"
    );
    assert_eq!(outcome.package.containerRuntime.promptInputHooks.len(), 1);
    assert_eq!(
        outcome.package.containerRuntime.promptFinalizeHooks.len(),
        1
    );
    assert_eq!(
        outcome
            .package
            .containerRuntime
            .inputMenuTogglePlugins
            .len(),
        1
    );
    assert!(outcome.packageLoadErrors.is_empty());
}

#[test]
fn runtime_loads_fixture_and_reads_main_script() {
    ensure_test_runtime_root();
    let runtime = ToolPkgRuntime::new(ToolPkgRuntimeOptions {
        languageCode: "en".to_string(),
    });
    let path = repoFixtureToolPkgPath("message_insert.toolpkg");
    runtime
        .loadToolPkgFile(path)
        .expect("message insert toolpkg should load");
    let mainScript = runtime
        .readToolPkgTextResource("com.operit.message_insert_bundle", "dist/main.js")
        .expect("main.js should be cached");
    assert!(mainScript.contains("registerToolPkg"));
    runtime.destroy();
}

#[test]
fn run_main_hook_reports_missing_function_as_execution_result() {
    ensure_test_runtime_root();
    let runtime = ToolPkgRuntime::new(ToolPkgRuntimeOptions {
        languageCode: "en".to_string(),
    });
    let path = repoFixtureToolPkgPath("message_insert.toolpkg");
    runtime
        .loadToolPkgFile(path)
        .expect("message insert toolpkg should load");
    let outcome = runtime
        .runMainHook(ToolPkgMainHookCall {
            containerPackageName: "com.operit.message_insert_bundle".to_string(),
            functionName: "__missing_for_runtime_test".to_string(),
            event: "runtime_test".to_string(),
            eventName: None,
            pluginId: None,
            functionSource: None,
            eventPayload: json!({}),
            executionContextKey: None,
            runtimeKind: Some("main".to_string()),
        })
        .expect("execution should return JS payload");
    assert!(outcome.value.unwrap_or_default().contains("success"));
    runtime.destroy();
}
