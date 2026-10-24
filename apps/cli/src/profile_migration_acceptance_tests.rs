//! Runs production startup and plugin reads only against an explicitly prepared disposable profile.
use crate::bootstrap::create_cli_application;
use operit_command_core::run_core_command;
use serde_json::{json, Value};
use std::path::PathBuf;

/// Executes a real registered command and requires its structured production result.
async fn command(
    application: &mut operit_runtime::core::application::OperitApplication::OperitApplication,
    args: &[&str],
) -> Value {
    let args = args.iter().map(|argument| argument.to_string()).chain(["--json".to_string()]).collect::<Vec<_>>();
    let output = run_core_command(application, &args).await.expect("Production command must succeed");
    assert!(output.stderr.is_empty(), "Production command reported an error");
    let value: Value = serde_json::from_str(&output.stdout).expect("Production command must return JSON");
    assert_ne!(value.get("ok"), Some(&Value::Bool(false)), "Plugin command failed");
    value
}

/// Exercises the native importer and actual plugin schema without starting peer or Space workers.
#[tokio::test(flavor = "current_thread")]
#[ignore = "Requires a disposable profile prepared by tools/tests/character_migration_acceptance.py"]
async fn restored_profile_migration_acceptance() {
    let root = std::fs::canonicalize(PathBuf::from(std::env::var_os("OPERIT_MIGRATION_ACCEPTANCE_ROOT")
        .expect("Acceptance root must be explicit"))).expect("Acceptance root must exist");
    let input: Value = serde_json::from_slice(&std::fs::read(root.join("acceptance-input.json"))
        .expect("Prepared acceptance manifest must exist")).expect("Acceptance manifest must be JSON");
    let source = std::fs::canonicalize(input["source"].as_str().expect("Source root is required")).unwrap();
    assert!(!root.starts_with(&source) && !source.starts_with(&root), "Acceptance must use an independent copy");
    let config = std::fs::canonicalize(std::env::var_os("OPERIT_CLI_CONFIG_DIR").unwrap()).unwrap();
    assert_eq!(config, root.join("config"));
    assert_eq!(std::fs::canonicalize(std::env::var_os("APPDATA").unwrap()).unwrap(), root.join("appdata"));
    let storage: Value = serde_json::from_slice(&std::fs::read(config.join("storage.json")).unwrap()).unwrap();
    assert_eq!(std::fs::canonicalize(storage["runtimeRoot"].as_str().unwrap()).unwrap(), root.join("runtime"));
    assert_eq!(std::fs::canonicalize(storage["workspaceRoot"].as_str().unwrap()).unwrap(), root.join("workspaces"));
    assert_eq!(storage["activeIdentityId"], input["identity"]);
    operit_util::AppLogger::AppLogger::set_enable_console_logging(false);
    let mut application = create_cli_application();
    application.onCreate().expect("Restored production startup must migrate successfully");
    let cards = command(&mut application, &["character", "list"]).await;
    let mut complete = Vec::new();
    let mut memories = Vec::new();
    for card in cards.as_array().expect("Character list must be an array") {
        let id = card["id"].as_str().expect("Character identity must be present");
        complete.push(command(&mut application, &["character", "show", id]).await);
        memories.push(command(&mut application, &["memory", "character", id, "export"]).await);
        command(&mut application, &["memory", "character", id, "user", "show"]).await;
        command(&mut application, &["memory", "character", id, "graph"]).await;
    }
    let groups = command(&mut application, &["group", "list"]).await;
    let tags = command(&mut application, &["tag", "list"]).await;
    let stores = command(&mut application, &["memory", "shared", "list"]).await;
    let active = command(&mut application, &["active-prompt", "show"]).await;
    let report = json!({"cards":complete,"memoryExports":memories,"groups":groups,"tags":tags,"stores":stores,"active":active});
    let reportPath = root.join(std::env::var("OPERIT_MIGRATION_ACCEPTANCE_REPORT").expect("Report name must be explicit"));
    assert_eq!(reportPath.parent(), Some(root.as_path()));
    std::fs::write(reportPath, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    let shutdown = application.toolHandler.getOrCreatePackageManager().lock().unwrap().executionEngineShutdown();
    shutdown();
    println!("Production restored-profile migration and plugin reads passed; peer services were not started.");
}
