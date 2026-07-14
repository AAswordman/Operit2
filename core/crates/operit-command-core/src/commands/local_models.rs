use std::path::{Path, PathBuf};

use crate::output::CoreCommandOutput;
use operit_local_models::LocalModelManifest::LocalModelKind;
use operit_local_models::LocalModelRegistry::{InstalledLocalEngine, InstalledLocalModel};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::services::LocalModelService::{LocalModelCatalogStatus, LocalModelService};
use operit_util::RuntimeStorageLayout::{
    RUNTIME_LOCAL_ENGINES_DIR_PATH, RUNTIME_LOCAL_MODELS_DIR_PATH,
    RUNTIME_LOCAL_MODEL_REGISTRY_PATH,
};

/// Runs local model repository and installation commands.
pub fn run_local_models_command(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("paths") if args.len() == 1 => print_local_model_paths(application, output),
        Some("catalog") if args.len() == 1 => print_catalog(application, output),
        Some("show") if args.len() == 3 => {
            print_catalog_status(application, &args[1], &args[2], output)
        }
        Some("installed") if args.len() == 1 => print_installed(application, output),
        Some("installed-show") if args.len() == 3 => {
            print_installed_model(application, &args[1], &args[2], output)
        }
        Some("install") if args.len() == 3 => {
            install_local_model(application, &args[1], &args[2], output)
        }
        Some("install-statuses") if args.len() == 1 => print_install_statuses(application, output),
        Some("install-status") if args.len() == 3 => {
            print_install_status(application, &args[1], &args[2], output)
        }
        Some("install-cancel") if args.len() == 3 => {
            cancel_local_model_install(application, &args[1], &args[2], output)
        }
        Some("verify") if args.len() == 3 => {
            verify_installed_model(application, &args[1], &args[2], output)
        }
        Some("delete") if args.len() == 3 => {
            delete_installed_model(application, &args[1], &args[2], output)
        }
        Some("engine-delete") if args.len() == 3 => {
            delete_installed_engine(application, &args[1], &args[2], output)
        }
        _ => {
            print_local_models_usage(output);
            Ok(())
        }
    }
}

/// Prints native paths used by the local model registry, models, and engines.
fn print_local_model_paths(
    application: &OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let runtime_root = runtime_root_dir(application)?;
    output.push_stdout_line(format!("runtimeRoot={}", runtime_root.display()));
    output.push_stdout_line(format!(
        "localModelsDir={}",
        runtime_layout_path(&runtime_root, RUNTIME_LOCAL_MODELS_DIR_PATH)?.display()
    ));
    output.push_stdout_line(format!(
        "localEnginesDir={}",
        runtime_layout_path(&runtime_root, RUNTIME_LOCAL_ENGINES_DIR_PATH)?.display()
    ));
    output.push_stdout_line(format!(
        "registryPath={}",
        runtime_layout_path(&runtime_root, RUNTIME_LOCAL_MODEL_REGISTRY_PATH)?.display()
    ));
    Ok(())
}

/// Prints local model catalog rows with current platform installation state.
fn print_catalog(
    application: &OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let service = local_model_service(application)?;
    let mut statuses = service.getCatalogStatus()?;
    statuses.sort_by(|left, right| {
        left.manifest
            .registryKey()
            .cmp(&right.manifest.registryKey())
    });
    for status in statuses {
        print_catalog_row(&status, output);
    }
    Ok(())
}

/// Prints one catalog status record as JSON.
fn print_catalog_status(
    application: &OperitApplication,
    model_id: &str,
    version: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let status = find_catalog_status(application, model_id, version)?;
    output.push_stdout_line(
        serde_json::to_string_pretty(&status).map_err(|error| error.to_string())?,
    );
    Ok(())
}

/// Prints installed models and engines for the current platform.
fn print_installed(
    application: &OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let service = local_model_service(application)?;
    let target = service.getPlatformTarget()?;
    let mut registry = service.getRegistry()?;
    registry
        .installedModels
        .sort_by_key(InstalledLocalModel::registryKey);
    registry
        .installedEngines
        .sort_by_key(InstalledLocalEngine::registryKey);
    output.push_stdout_line(format!("target={}", target.storageSegment()));
    for model in registry.installedModels {
        print_installed_model_row(&model, output);
    }
    for engine in registry.installedEngines {
        print_installed_engine_row(&engine, output);
    }
    Ok(())
}

/// Prints one installed local model registry record as JSON.
fn print_installed_model(
    application: &OperitApplication,
    model_id: &str,
    version: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let registry = local_model_service(application)?.getRegistry()?;
    let installed = registry
        .getInstalledModel(model_id, version)
        .ok_or_else(|| format!("installed local model not found: {model_id}@{version}"))?;
    output.push_stdout_line(
        serde_json::to_string_pretty(installed).map_err(|error| error.to_string())?,
    );
    Ok(())
}

/// Installs one catalog model and its exact platform engine dependency.
fn install_local_model(
    application: &OperitApplication,
    model_id: &str,
    version: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let result = local_model_service(application)?
        .installModel(model_id.to_string(), version.to_string())?;
    output.push_stdout_line(format!(
        "installedModel={}@{}\tmodelBytes={}\tmodelStoragePath={}",
        result.installedModel.manifest.id,
        result.installedModel.manifest.version,
        result.modelDownloadedBytes,
        result.installedModel.storagePath
    ));
    output.push_stdout_line(format!(
        "installedEngine={}@{}\ttarget={}\tengineBytes={}\tengineStoragePath={}",
        result.installedEngine.manifest.id,
        result.installedEngine.manifest.version,
        result.installedEngine.artifact.target.storageSegment(),
        result.engineDownloadedBytes,
        result.installedEngine.storagePath
    ));
    Ok(())
}

/// Prints every retained local model installation operation as JSON.
fn print_install_statuses(
    application: &OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let statuses = local_model_service(application)?.getInstallStatuses()?;
    output.push_stdout_line(
        serde_json::to_string_pretty(&statuses).map_err(|error| error.to_string())?,
    );
    Ok(())
}

/// Prints one local model installation operation as JSON.
fn print_install_status(
    application: &OperitApplication,
    model_id: &str,
    version: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let operation_id = format!("{}@{}", model_id.trim(), version.trim());
    let status = local_model_service(application)?
        .getInstallStatus(model_id.to_string(), version.to_string())?
        .ok_or_else(|| format!("local model install operation not found: {operation_id}"))?;
    output.push_stdout_line(
        serde_json::to_string_pretty(&status).map_err(|error| error.to_string())?,
    );
    Ok(())
}

/// Requests cancellation for one active local model installation.
fn cancel_local_model_install(
    application: &OperitApplication,
    model_id: &str,
    version: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let status = local_model_service(application)?
        .cancelInstall(model_id.to_string(), version.to_string())?;
    output.push_stdout_line(
        serde_json::to_string_pretty(&status).map_err(|error| error.to_string())?,
    );
    Ok(())
}

/// Verifies one installed model and its exact platform engine dependency.
fn verify_installed_model(
    application: &OperitApplication,
    model_id: &str,
    version: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let status =
        local_model_service(application)?.verifyModel(model_id.to_string(), version.to_string())?;
    output.push_stdout_line(format!(
        "verifiedModel={}@{}\tmodel=true\tengine=true",
        status.manifest.id, status.manifest.version
    ));
    Ok(())
}

/// Deletes one installed model from the local repository.
fn delete_installed_model(
    application: &OperitApplication,
    model_id: &str,
    version: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    local_model_service(application)?.deleteModel(model_id.to_string(), version.to_string())?;
    output.push_stdout_line(format!("deletedModel={model_id}@{version}"));
    Ok(())
}

/// Deletes one installed engine when no installed model references it.
fn delete_installed_engine(
    application: &OperitApplication,
    engine_id: &str,
    version: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    local_model_service(application)?.deleteEngine(engine_id.to_string(), version.to_string())?;
    output.push_stdout_line(format!("deletedEngine={engine_id}@{version}"));
    Ok(())
}

/// Returns the local model service for the active application context.
fn local_model_service(application: &OperitApplication) -> Result<LocalModelService, String> {
    LocalModelService::getInstance(&application.hostManager)
}

/// Returns the runtime root directory from the active storage host.
fn runtime_root_dir(application: &OperitApplication) -> Result<PathBuf, String> {
    let host = application
        .hostManager
        .runtimeStorageHost
        .as_ref()
        .ok_or_else(|| "RuntimeStorageHost is not registered".to_string())?;
    host.runtimeRootDir()
        .ok_or_else(|| "RuntimeStorageHost runtime root is not configured".to_string())
}

/// Maps a runtime-layout path into a native path below the runtime root.
fn runtime_layout_path(runtime_root: &Path, layout_path: &str) -> Result<PathBuf, String> {
    let relative = layout_path
        .trim()
        .strip_prefix("runtime/")
        .ok_or_else(|| format!("invalid runtime layout path: {layout_path}"))?;
    Ok(runtime_root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR)))
}

/// Returns one catalog status matching an exact model id and version.
fn find_catalog_status(
    application: &OperitApplication,
    model_id: &str,
    version: &str,
) -> Result<LocalModelCatalogStatus, String> {
    local_model_service(application)?
        .getCatalogStatus()?
        .into_iter()
        .find(|status| status.manifest.id == model_id && status.manifest.version == version)
        .ok_or_else(|| format!("local model catalog entry not found: {model_id}@{version}"))
}

/// Prints one built-in model status as a tab-separated row.
fn print_catalog_row(status: &LocalModelCatalogStatus, output: &mut CoreCommandOutput) {
    let manifest = &status.manifest;
    let engine = match manifest.engineRequirement.as_ref() {
        Some(requirement) => format!("{}@{}", requirement.engineId, requirement.version),
        None => "not-declared".to_string(),
    };
    output.push_stdout_line(format!(
        "{}\tversion={}\tkind={}\tengine={}\tbytes={}\tlicense={}\tcompatible={}\tmodelInstalled={}\tengineInstalled={}",
        manifest.id,
        manifest.version,
        local_model_kind_name(&manifest.kind),
        engine,
        manifest.declaredByteSize(),
        manifest.license,
        status.platformCompatible,
        status.installedModel.is_some(),
        status.installedEngine.is_some()
    ));
}

/// Prints one installed model as a tab-separated row.
fn print_installed_model_row(model: &InstalledLocalModel, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!(
        "model={}@{}\tkind={}\tbytes={}\tinstalledAtMs={}\tverifiedAtMs={}\tstoragePath={}",
        model.manifest.id,
        model.manifest.version,
        local_model_kind_name(&model.manifest.kind),
        model.manifest.declaredByteSize(),
        model.installedAtMs,
        optional_timestamp(model.verifiedAtMs),
        model.storagePath
    ));
}

/// Prints one installed engine as a tab-separated row.
fn print_installed_engine_row(engine: &InstalledLocalEngine, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!(
        "engine={}@{}\ttarget={}\tbytes={}\tinstalledAtMs={}\tverifiedAtMs={}\tstoragePath={}",
        engine.manifest.id,
        engine.manifest.version,
        engine.artifact.target.storageSegment(),
        engine.artifact.byteSize,
        engine.installedAtMs,
        optional_timestamp(engine.verifiedAtMs),
        engine.storagePath
    ));
}

/// Formats an optional verification timestamp for CLI output.
fn optional_timestamp(value: Option<i64>) -> String {
    match value {
        Some(timestamp) => timestamp.to_string(),
        None => "not-verified".to_string(),
    }
}

/// Returns the stable display name for a local model kind.
fn local_model_kind_name(kind: &LocalModelKind) -> &'static str {
    match kind {
        LocalModelKind::SpeechToText => "SpeechToText",
        LocalModelKind::TextToSpeech => "TextToSpeech",
        LocalModelKind::Chat => "Chat",
        LocalModelKind::Embedding => "Embedding",
    }
}

/// Prints local model repository command usage.
fn print_local_models_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 local-models paths");
    output.push_stdout_line("operit2 local-models catalog");
    output.push_stdout_line("operit2 local-models show <model-id> <version>");
    output.push_stdout_line("operit2 local-models installed");
    output.push_stdout_line("operit2 local-models installed-show <model-id> <version>");
    output.push_stdout_line("operit2 local-models install <model-id> <version>");
    output.push_stdout_line("operit2 local-models install-statuses");
    output.push_stdout_line("operit2 local-models install-status <model-id> <version>");
    output.push_stdout_line("operit2 local-models install-cancel <model-id> <version>");
    output.push_stdout_line("operit2 local-models verify <model-id> <version>");
    output.push_stdout_line("operit2 local-models delete <model-id> <version>");
    output.push_stdout_line("operit2 local-models engine-delete <engine-id> <version>");
}
