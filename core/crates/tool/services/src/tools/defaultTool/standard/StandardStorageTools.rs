#![allow(non_snake_case)]

use super::ToolArguments::{decodeArguments, RequestArguments};
use operit_vfs::VfsPath;
use crate::tools::ToolInvocation::prepareInvocation;
use crate::tools::ToolResultDataClasses::{StorageRequestResultData, ToolResultData};
use crate::ToolExecutionManager::{
    PreparedToolInvocation, SharedToolExecutor, ToolCall, ToolEffect, ToolInvocationContext,
};
use operit_plugin_sdk::js_sdk::storage::StorageRequest;

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
            StorageRequest::PreferencesData { .. }
            | StorageRequest::PreferencesBeginEdit { .. }
            | StorageRequest::PreferencesCancelEdit { .. }
            | StorageRequest::GetMany { .. }
            | StorageRequest::Query { .. }
            | StorageRequest::Get { .. }
            | StorageRequest::List { .. }
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
    let storage = hosts
        .runtimeStorageHost
        .ok_or("RuntimeStorageHost is not registered")?;
    if let StorageRequest::Open { path, .. } | StorageRequest::OpenPreferences { path, .. } = &mut request {
        *path = resolve_plugin_storage_path(path)?;
    }
    let session = &context.host().storageSession;
    let response = if session.isPreferencesRequest(&request).map_err(|error| error.to_string())? {
        session.requestPreferences(storage, request)
    } else {
        let sqlite = hosts.runtimeSqliteHost.ok_or("RuntimeSqliteHost is not registered")?;
        let deviceId = SyncOperationStore::new(storage.clone(), RUNTIME_SYNC_DIR_PATH)
            .localDeviceId().map_err(|error| error.to_string())?;
        session.request(sqlite, storage, request, &deviceId)
    }.map_err(|error| error.to_string())?;
    Ok(StorageRequestResultData::from(response))
}

/// Converts virtual addresses and logical storage keys to one validated runtime storage identity.
pub(crate) fn resolve_plugin_storage_path(
    path: &str,
) -> Result<String, String> {
    let normalized = path.replace('\\', "/");
    if normalized.starts_with("runtime/") {
        let path = VfsPath::runtimeDataPath(&normalized)?;
        let relative = VfsPath::relativePath("/operitapp/data", &path)?.ok_or("Storage path is outside runtime data")?;
        return Ok(format!("runtime/{relative}"));
    }
    for root in ["/app/data", "/operitapp/data"] {
        if normalized != root && !normalized.starts_with(&format!("{root}/")) { continue; }
        let relative = VfsPath::relativePath(root, &normalized)
            .map_err(|error| error.to_string())?;
        if let Some(relative) = relative {
            return Ok(format!("runtime/{relative}"));
        }
    }
    Err("Storage path must use a runtime VFS address or a registered runtime/ storage key".into())
}
