//! Shared path preparation and device routing for tools that consume VFS files.
use std::sync::Arc;

use operit_vfs::VisualFileSystem::VisualFileSystem;
use super::RemoteFileGateway::{
    buildRemoteFileToolRequest, remoteReplyToToolResult, RemoteFileGateway,
};
use crate::runtime_support::{
    coreNodeReferenceLabel, splitQualifiedVfsPath, QualifiedVfsResolution,
};
use crate::tools::ToolResultDataClasses::stringResultData;
use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::{AITool, ToolExecutionManager};
use operit_host_api::RuntimeStorageHost;

/// Message for calls whose path parameters resolve to different devices.
const CROSS_NODE_TRANSFER_MESSAGE: &str = "File tools run every operation on one device. Moving or copying between different devices (source and destination on different nodes) is not supported yet; read the file from the source device, then write it on the destination device.";

/// One file-tool invocation resolved to its execution device.
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub(crate) enum PreparedFileToolCall {
    /// Runs on this device; self-owned `nodeName:` prefixes were stripped.
    Local(AITool),
    /// Runs on another space member; paths are already target-local.
    Remote {
        call: AITool,
        nodeId: String,
        /// The qualified path exactly as typed, for result echo round-trips.
        typedPath: String,
        /// The typed node reference alone, for re-qualifying child paths.
        nodePrefix: String,
    },
}

/// Resolves file arguments using the current directory and the runtime's device space.
#[derive(Clone)]
pub(crate) struct FileToolRouting {
    filesystem: VisualFileSystem,
    runtimeStorageHost: Arc<dyn RuntimeStorageHost>,
    gateway: Arc<dyn RemoteFileGateway>,
}

impl FileToolRouting {
    /// Binds path preparation to the same storage and transport as the owning runtime.
    pub(crate) fn new(
        filesystem: VisualFileSystem,
        runtimeStorageHost: Arc<dyn RuntimeStorageHost>,
        gateway: Arc<dyn RemoteFileGateway>,
    ) -> Self {
        Self {
            filesystem,
            runtimeStorageHost,
            gateway,
        }
    }

    /// Resolves one local argument against the owning invocation's file directory.
    pub(crate) fn resolveLocalPath(&self, path: &str) -> Result<String, String> {
        let context = ToolExecutionManager::currentToolRuntimeContext();
        let directory = context.as_ref()
            .map(|context| context.fileDirectory(self.runtimeStorageHost.clone()))
            .transpose()?.flatten();
        self.filesystem.resolveResourcePath(directory.as_deref(), path)
    }

    /// Resolves relative paths and qualified device references before any host access.
    #[allow(non_snake_case)]
    pub(crate) fn prepare(
        &self,
        tool: &AITool,
        pathParameters: &[&str],
    ) -> Result<PreparedFileToolCall, String> {
        let mut call = tool.clone();
        let mut remoteTarget: Option<(String, String, String)> = None;
        let mut localPathSeen = false;
        for name in pathParameters {
            let value = parameterValue(tool, name);
            if splitQualifiedVfsPath(&value).is_none() {
                if !value.trim().is_empty() {
                    // A bare path always evaluates on this device; combining it
                    // with a remote parameter would leave it unrewritten on the
                    // remote, where its mounts do not exist.
                    if remoteTarget.is_some() {
                        return Err(CROSS_NODE_TRANSFER_MESSAGE.to_string());
                    }
                    localPathSeen = true;
                    let resolved = self.resolveLocalPath(&value)?;
                    Self::rewritePathParameter(&mut call, name, &resolved);
                }
                continue;
            }
            let state = self.gateway.routeState().map_err(|error| {
                format!("Cross-device path '{value}' requires CoreNode routing: {error}")
            })?;
            match state.resolveQualifiedVfsPath(&value)? {
                QualifiedVfsResolution::LocalPath(innerPath) => {
                    if remoteTarget.is_some() {
                        return Err(CROSS_NODE_TRANSFER_MESSAGE.to_string());
                    }
                    localPathSeen = true;
                    let resolved = self.resolveLocalPath(&innerPath)?;
                    Self::rewritePathParameter(&mut call, name, &resolved);
                }
                QualifiedVfsResolution::RemoteTarget {
                    node,
                    pathPrefix,
                    innerPath,
                } => {
                    if localPathSeen {
                        return Err(CROSS_NODE_TRANSFER_MESSAGE.to_string());
                    }
                    if !node.canExecute {
                        return Err(format!(
                            "Device cannot run file operations: {}",
                            coreNodeReferenceLabel(node)
                        ));
                    }
                    if !node.reachable {
                        return Err(format!(
                            "Device is currently unreachable: {}",
                            coreNodeReferenceLabel(node)
                        ));
                    }
                    match &remoteTarget {
                        None => {
                            remoteTarget = Some((
                                node.nodeId.clone(),
                                value.trim().to_string(),
                                pathPrefix.to_string(),
                            ));
                        }
                        Some((nodeId, _, _)) if nodeId != &node.nodeId => {
                            return Err(CROSS_NODE_TRANSFER_MESSAGE.to_string());
                        }
                        _ => {}
                    }
                    Self::rewritePathParameter(&mut call, name, innerPath);
                }
            }
        }
        match remoteTarget {
            None => Ok(PreparedFileToolCall::Local(call)),
            Some((nodeId, typedPath, nodePrefix)) => Ok(PreparedFileToolCall::Remote {
                call,
                nodeId,
                typedPath,
                nodePrefix,
            }),
        }
    }

    /// Replaces one declared path parameter after resolving its execution device.
    #[allow(non_snake_case)]
    fn rewritePathParameter(call: &mut AITool, name: &str, value: &str) {
        if let Some(parameter) = call
            .parameters
            .iter_mut()
            .find(|parameter| parameter.name == name)
        {
            parameter.value = value.to_string();
        }
    }

    /// Runs the same tool on its selected device under that device's own policies.
    #[allow(non_snake_case)]
    pub(crate) async fn dispatchRemote(
        &self,
        call: AITool,
        nodeId: String,
        typedPath: &str,
        nodePrefix: &str,
    ) -> ToolResult {
        let request = match buildRemoteFileToolRequest(&call, &nodeId) {
            Ok(request) => request,
            Err(error) => return toolError(&call, error),
        };
        let reply = self.gateway.dispatchRemoteTool(request).await;
        remoteReplyToToolResult(&call, reply, typedPath, nodePrefix)
    }
}

/// Reads one named path argument without interpreting its contents.
fn parameterValue(tool: &AITool, name: &str) -> String {
    tool.parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .map(|parameter| parameter.value.clone())
        .unwrap_or_default()
}

/// Reports routing failures before any local or remote side effect occurs.
fn toolError(tool: &AITool, error: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: stringResultData(""),
        error: Some(error),
    }
}
