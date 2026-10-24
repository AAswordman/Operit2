//! Outbound cross-device file access over the existing device-tool transport.
//!
//! A remote file operation runs the same built-in tool on the target node via
//! `core.tools`, so the target's own path mapping, capability mode, and result
//! shapes apply unchanged. Only the echoed paths are rewritten back into the
//! `node:` syntax the caller typed.

use std::collections::BTreeMap;
use std::sync::Arc;

use operit_plugin_sdk::javascript::{
    JsDeviceToolAction, JsDeviceToolCall, JsExecutionCompletion, JsToolCallRequest,
    JsToolCallResult, JsToolCallResultData,
};
use serde_json::Value;

use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::AITool;
use crate::tools::ToolResultDataClasses::{stringResultData, JsonResultData, ToolResultData};
use crate::runtime_support::{RuntimeCoreNodeRouteState, ToolRuntimeSupport};

/// One remote file operation deadline; relayed links may be slow but reads are small.
const REMOTE_FILE_TOOL_TIMEOUT_MS: u64 = 60_000;

/// Outbound cross-device file transport owned by the embedding runtime.
pub trait RemoteFileGateway: Send + Sync {
    /// Returns one consistent snapshot of the device space.
    #[allow(non_snake_case)]
    fn routeState(&self) -> Result<RuntimeCoreNodeRouteState, String>;

    /// Runs one tool invocation on an explicitly selected device.
    #[allow(non_snake_case)]
    fn dispatchRemoteTool(
        &self,
        request: JsToolCallRequest,
    ) -> JsExecutionCompletion<JsToolCallResult>;
}

struct ToolRuntimeFileGateway(Arc<dyn ToolRuntimeSupport>);

impl RemoteFileGateway for ToolRuntimeFileGateway {
    #[allow(non_snake_case)]
    fn routeState(&self) -> Result<RuntimeCoreNodeRouteState, String> {
        self.0.coreNodeRouteState()
    }

    #[allow(non_snake_case)]
    fn dispatchRemoteTool(
        &self,
        request: JsToolCallRequest,
    ) -> JsExecutionCompletion<JsToolCallResult> {
        self.0.executeDeviceTool(request)
    }
}

/// Wraps the installed tool runtime support into the narrower file gateway.
#[allow(non_snake_case)]
pub fn toolRuntimeFileGateway(support: Arc<dyn ToolRuntimeSupport>) -> Arc<dyn RemoteFileGateway> {
    Arc::new(ToolRuntimeFileGateway(support))
}

/// Builds a device-tool request that runs the same tool on the target node.
///
/// Callers pass parameters whose path values were already rewritten to target
///-local form; every other parameter travels verbatim as its string form.
#[allow(non_snake_case)]
pub fn buildRemoteFileToolRequest(
    call: &AITool,
    nodeId: &str,
) -> Result<JsToolCallRequest, String> {
    let mut parameters = BTreeMap::new();
    for parameter in &call.parameters {
        parameters.insert(parameter.name.clone(), Value::String(parameter.value.clone()));
    }
    let device = JsDeviceToolCall {
        node_id: nodeId.to_string(),
        invocation_id: uuid::Uuid::new_v4().to_string(),
        action: JsDeviceToolAction::Execute,
        timeout_ms: REMOTE_FILE_TOOL_TIMEOUT_MS,
    };
    device.validate()?;
    Ok(JsToolCallRequest {
        tool_type: "default".to_string(),
        tool_name: call.name.trim().to_string(),
        parameters,
        device: Some(device),
    })
}

/// Converts one remote tool reply into a local tool result, restoring the
/// typed `node:` prefix on every echoed path. Child paths returned by the
/// remote (`find_files`, `grep_*`) take `nodePrefix:` so they stay addressable.
#[allow(non_snake_case)]
pub fn remoteReplyToToolResult(
    call: &AITool,
    reply: JsToolCallResult,
    typedPath: &str,
    nodePrefix: &str,
) -> ToolResult {
    if !reply.success {
        return ToolResult {
            toolName: call.name.clone(),
            success: false,
            result: stringResultData(""),
            error: Some(reply.error.unwrap_or_else(|| {
                "Remote file operation failed without a reported error".to_string()
            })),
        };
    }
    let value = match reply.data {
        JsToolCallResultData::Value(value) => value,
        JsToolCallResultData::Binary(bytes) => {
            Value::String(String::from_utf8_lossy(&bytes).to_string())
        }
    };
    let mut data: ToolResultData = serde_json::from_value(value.clone())
        .unwrap_or_else(|_| ToolResultData::JsonResultData(JsonResultData { value }));
    qualifyRemoteResultPaths(&mut data, typedPath, nodePrefix);
    ToolResult {
        toolName: call.name.clone(),
        success: true,
        result: data,
        error: None,
    }
}

/// Restores `node:` prefixes on every echoed path of a remote reply.
#[allow(non_snake_case)]
pub fn qualifyRemoteResultPaths(data: &mut ToolResultData, typedPath: &str, nodePrefix: &str) {
    match data {
        ToolResultData::DirectoryListingData(entry) => entry.path = typedPath.to_string(),
        ToolResultData::FileContentData(entry) => entry.path = typedPath.to_string(),
        ToolResultData::BinaryFileContentData(entry) => entry.path = typedPath.to_string(),
        ToolResultData::FilePartContentData(entry) => entry.path = typedPath.to_string(),
        ToolResultData::FileExistsData(entry) => entry.path = typedPath.to_string(),
        ToolResultData::FileInfoData(entry) => entry.path = typedPath.to_string(),
        ToolResultData::FileOperationData(entry) => entry.path = typedPath.to_string(),
        ToolResultData::FileApplyResultData(entry) => entry.operation.path = typedPath.to_string(),
        ToolResultData::FindFilesResultData(entry) => {
            entry.path = typedPath.to_string();
            entry.files = entry
                .files
                .iter()
                .map(|file| format!("{nodePrefix}:{file}"))
                .collect();
        }
        ToolResultData::GrepResultData(entry) => {
            entry.searchPath = typedPath.to_string();
            for fileMatch in &mut entry.matches {
                let filePath = fileMatch.filePath.clone();
                fileMatch.filePath = format!("{nodePrefix}:{filePath}");
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolExecutionManager::ToolParameter;
    use crate::tools::ToolResultDataClasses::{
        BinaryFileContentData, FileContentData, FileOperationData, FindFilesResultData,
        GrepFileMatch, GrepResultData,
    };

    fn call(name: &str, parameters: Vec<(&str, &str)>) -> AITool {
        AITool {
            name: name.to_string(),
            parameters: parameters
                .into_iter()
                .map(|(name, value)| ToolParameter { name: name.to_string(), value: value.to_string() })
                .collect(),
        }
    }

    fn replyValue(data: ToolResultData) -> JsToolCallResult {
        JsToolCallResult {
            success: true,
            data: JsToolCallResultData::Value(
                serde_json::from_str(&data.toJson()).expect("reply serialization failed"),
            ),
            error: None,
        }
    }

    #[test]
    fn remoteRequestsKeepToolIdentityAndRewrittenPaths() {
        let call = call(
            "read_file",
            vec![("path", "/home/user/a.txt"), ("encoding", "utf8")],
        );
        let request = buildRemoteFileToolRequest(&call, "core-1-cas").unwrap();
        let device = request.device.as_ref().unwrap();
        assert_eq!(device.node_id, "core-1-cas");
        assert_eq!(device.action, JsDeviceToolAction::Execute);
        assert_eq!(request.qualified_tool_name(), "read_file");
        assert_eq!(
            request.parameters.get("path").and_then(Value::as_str),
            Some("/home/user/a.txt")
        );
        assert_eq!(
            request.parameters.get("encoding").and_then(Value::as_str),
            Some("utf8")
        );
    }

    #[test]
    fn remoteRepliesRestoreTypedPathsOnEveryEcho() {
        let listCall = call("list_files", vec![("path", "cas:/mnt/nas/data")]);
        let reply = replyValue(ToolResultData::DirectoryListingData(crate::tools::ToolResultDataClasses::DirectoryListingData {
            path: "/mnt/nas/data".to_string(),
            entries: vec![],
        }));
        let result = remoteReplyToToolResult(&listCall, reply, "cas:/mnt/nas/data", "cas");
        assert!(result.success);
        let ToolResultData::DirectoryListingData(listing) = result.result else {
            panic!("expected the directory listing to survive the round-trip");
        };
        assert_eq!(listing.path, "cas:/mnt/nas/data");

        let readCall = call("read_file", vec![("path", "cas:/home/user/a.txt")]);
        let reply = replyValue(ToolResultData::FileContentData(FileContentData {
            path: "/home/user/a.txt".to_string(),
            content: "hello".to_string(),
            size: 5,
        }));
        let result = remoteReplyToToolResult(&readCall, reply, "cas:/home/user/a.txt", "cas");
        let ToolResultData::FileContentData(content) = result.result else {
            panic!("expected the file content to survive the round-trip");
        };
        assert_eq!(content.path, "cas:/home/user/a.txt");
        assert_eq!(content.content, "hello");
    }

    #[test]
    fn remoteFindAndGrepRepliesPrefixEveryChildPath() {
        let mut findData = ToolResultData::FindFilesResultData(FindFilesResultData {
            path: "/mnt/nas".to_string(),
            pattern: "*.rs".to_string(),
            files: vec!["/mnt/nas/src/lib.rs".to_string(), "/mnt/nas/README.md".to_string()],
        });
        qualifyRemoteResultPaths(&mut findData, "cas:/mnt/nas", "cas");
        let ToolResultData::FindFilesResultData(find) = &findData else { panic!() };
        assert_eq!(find.path, "cas:/mnt/nas");
        assert_eq!(find.files[0], "cas:/mnt/nas/src/lib.rs");
        assert_eq!(find.files[1], "cas:/mnt/nas/README.md");

        let mut grepData = ToolResultData::GrepResultData(GrepResultData {
            searchPath: "/mnt/nas".to_string(),
            pattern: "TODO".to_string(),
            matches: vec![GrepFileMatch {
                filePath: "/mnt/nas/src/lib.rs".to_string(),
                lineMatches: vec![],
            }],
            totalMatches: 1,
            filesSearched: 3,
        });
        qualifyRemoteResultPaths(&mut grepData, "cas:/mnt/nas", "cas");
        let ToolResultData::GrepResultData(grep) = &grepData else { panic!() };
        assert_eq!(grep.searchPath, "cas:/mnt/nas");
        assert_eq!(grep.matches[0].filePath, "cas:/mnt/nas/src/lib.rs");
    }

    #[test]
    fn untypedRepliesFallBackToJsonAndFailuresStayFailures() {
        let call = call("file_info", vec![("path", "cas:/mnt/nas")]);
        let untyped = JsToolCallResult {
            success: true,
            data: JsToolCallResultData::Value(serde_json::json!({ "custom": true })),
            error: None,
        };
        let result = remoteReplyToToolResult(&call, untyped, "cas:/mnt/nas", "cas");
        assert!(result.success);
        assert!(matches!(result.result, ToolResultData::JsonResultData(_)));

        let failure = JsToolCallResult {
            success: false,
            data: JsToolCallResultData::Value(serde_json::json!(null)),
            error: Some("Device is offline".to_string()),
        };
        let result = remoteReplyToToolResult(&call, failure, "cas:/mnt/nas", "cas");
        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("Device is offline"));
    }

    /// Keeps every file payload variant deserializable through the tagged enum.
    #[test]
    fn taggedPayloadsSurviveSerialization() {
        for json in [
            ToolResultData::BinaryFileContentData(BinaryFileContentData {
                path: "/mnt/nas/a.bin".to_string(),
                contentBase64: "aGk=".to_string(),
                size: 2,
            })
            .toJson(),
            ToolResultData::FileOperationData(FileOperationData {
                operation: "delete".to_string(),
                path: "/mnt/nas/a.bin".to_string(),
                successful: true,
                details: String::new(),
            })
            .toJson(),
        ] {
            let parsed: ToolResultData =
                serde_json::from_str(&json).expect("payload must deserialize");
            assert_eq!(parsed.toJson(), json);
        }
    }
}
