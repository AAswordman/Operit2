#![allow(non_snake_case)]

use super::ToolArguments::{decodeArguments, RequestArguments};
use crate::files::PathMapper::PathMapper;
use crate::tools::ToolInvocation::prepareInvocation;
use crate::tools::ToolResultDataClasses::{StorageRequestResultData, ToolResultData};
use crate::ToolExecutionManager::{
    PreparedToolInvocation, SharedToolExecutor, ToolCall, ToolEffect, ToolInvocationContext,
};
use operit_plugin_sdk::js_sdk::storage::StorageRequest;

use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_store::SyncOperationStore::SyncOperationStore;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;

/// Executes owner-authenticated storage operations through the registered tool pipeline.
pub(crate) struct StorageToolExecutor;

impl SharedToolExecutor for StorageToolExecutor {
    fn prepare<'a>(
        &'a self,
        request: &'a ToolCall,
        context: &'a ToolInvocationContext,
        checkAccess: bool,
    ) -> Result<PreparedToolInvocation<'a>, String> {
        let args: RequestArguments<StorageRequest> = decodeArguments(request)?;
        let effect = match &args.request {
            StorageRequest::Query { .. }
            | StorageRequest::Get { .. }
            | StorageRequest::List { .. }
            | StorageRequest::Changes { .. }
            | StorageRequest::Close { .. } => ToolEffect::READ,
            _ => ToolEffect::WRITE,
        };
        Ok(prepareInvocation(
            request,
            checkAccess,
            effect,
            async move {
                let result = executePluginStorage(args.request, context).await?;
                Ok(ToolResultData::StorageRequestResultData(result))
            },
        ))
    }
}

/// Executes plugin storage through the current engine's owned session and runtime Hosts.
pub(crate) async fn executePluginStorage(
    mut request: StorageRequest,
    context: &ToolInvocationContext,
) -> Result<StorageRequestResultData, String> {
    context.requirePluginOwner()?;
    let hosts = context.host().getContext();
    let sqlite = hosts
        .runtimeSqliteHost
        .ok_or("RuntimeSqliteHost is not registered")?;
    let storage = hosts
        .runtimeStorageHost
        .ok_or("RuntimeStorageHost is not registered")?;
    if let StorageRequest::Open { path, .. } = &mut request {
        *path = resolve_plugin_storage_path(path, &RuntimeStorePaths::default())?;
    }
    let deviceId = SyncOperationStore::new(storage.clone(), RUNTIME_SYNC_DIR_PATH)
        .localDeviceId()
        .map_err(|error| error.to_string())?;
    let response = context
        .host()
        .storageSession
        .request(sqlite, storage, request, &deviceId)
        .map_err(|error| error.to_string())?;
    Ok(StorageRequestResultData::from(response))
}

/// Converts supported public VFS, runtime-relative and Host paths to one canonical runtime storage identity.
pub(crate) fn resolve_plugin_storage_path(
    path: &str,
    roots: &RuntimeStorePaths,
) -> Result<String, String> {
    let normalized = path.replace('\\', "/");
    if normalized.starts_with("runtime/") {
        return Ok(normalized);
    }
    if normalized == "/app/data" || normalized.starts_with("/app/data/") {
        let relative = PathMapper::relativePath("/app/data", &normalized)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| String::from("Storage VFS path is not inside the runtime root"))?;
        return Ok(format!("runtime/{relative}"));
    }
    let root = roots.runtime_dir().to_string_lossy().replace('\\', "/");
    let relative = normalized
        .strip_prefix(&format!("{root}/"))
        .ok_or_else(|| String::from("Storage path is not inside the runtime Host root"))?;
    Ok(format!("runtime/{relative}"))
}
