//! Internal chunked file-transfer endpoints for cross-node copies.
//!
//! These string-named tools are transport plumbing, never model-facing: the
//! orchestrating device streams a source file through `__fs_read_chunk` or
//! pushes chunks with `__fs_transfer_chunk`, and the receiving device keeps
//! per-transfer sessions that finalize into the destination only after the
//! byte size and SHA-256 both match.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use sha2::{Digest, Sha256};

use operit_plugin_sdk::javascript::{
    JsDeviceToolAction, JsDeviceToolCall, JsToolCallRequest, JsToolCallResultData,
};
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{
    AITool, ToolAccessSpec, ToolBoundary, ToolEffect, ToolInvocationContext, ToolInvocationFuture,
    ToolParameter, ToolValidationResult,
};
use crate::tools::ToolResultDataClasses::{JsonResultData, StringResultData, ToolResultData};

use crate::files::RemoteFileGateway::toolRuntimeFileGateway;
use crate::tools::defaultTool::standard::StandardFileSystemTools::StandardFileSystemTools;

/// Chunk payload cap; base64 encoding stays far below the peer message limit.
pub const TRANSFER_CHUNK_BYTES: usize = 512 * 1024;
const SESSION_TIMEOUT_MS: i64 = 10 * 60 * 1000;
/// Resource-backed sources have no seekable native path; read them whole, once.
const MAX_RESOURCE_SOURCE_BYTES: i64 = 256 * 1024 * 1024;

pub const FS_TRANSFER_BEGIN_TOOL: &str = "__fs_transfer_begin";
pub const FS_TRANSFER_CHUNK_TOOL: &str = "__fs_transfer_chunk";
pub const FS_TRANSFER_COMMIT_TOOL: &str = "__fs_transfer_commit";
pub const FS_READ_CHUNK_TOOL: &str = "__fs_read_chunk";
pub const FS_PUSH_FILE_TOOL: &str = "__fs_push_file";

struct TransferSession {
    tempPath: PathBuf,
    destinationPath: String,
    expectedSize: i64,
    received: i64,
    hasher: Sha256,
    createdMs: i64,
}

fn sessions() -> &'static Mutex<HashMap<String, TransferSession>> {
    static SESSIONS: OnceLock<Mutex<HashMap<String, TransferSession>>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn nowMs() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}

/// Drops timed-out sessions (and their partial files) before a new begin.
fn evictExpiredSessions() {
    let cutoff = nowMs() - SESSION_TIMEOUT_MS;
    let mut guard = sessions().lock().expect("transfer sessions mutex poisoned");
    let expired: Vec<String> = guard
        .iter()
        .filter(|(_, session)| session.createdMs < cutoff)
        .map(|(id, _)| id.clone())
        .collect();
    for id in expired {
        if let Some(session) = guard.remove(&id) {
            drop(std::fs::remove_file(&session.tempPath));
        }
    }
}

fn takeSession(transferId: &str) -> Result<TransferSession, String> {
    sessions()
        .lock()
        .expect("transfer sessions mutex poisoned")
        .remove(transferId)
        .ok_or_else(|| format!("Unknown or finished transfer session: {transferId}"))
}

fn restoreSession(session: TransferSession) {
    let transferId = session
        .tempPath
        .file_name()
        .map(|name| name.to_string_lossy().trim_start_matches("operit-part-").to_string())
        .unwrap_or_default();
    sessions()
        .lock()
        .expect("transfer sessions mutex poisoned")
        .insert(transferId, session);
}

fn parameterValue(tool: &AITool, name: &str) -> String {
    tool.parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .map(|parameter| parameter.value.trim().to_string())
        .unwrap_or_default()
}

fn requiredNumber(tool: &AITool, name: &str) -> Result<i64, String> {
    parameterValue(tool, name)
        .parse::<i64>()
        .map_err(|_| format!("{name} must be an integer"))
}

fn jsonResult(value: serde_json::Value) -> ToolResultData {
    ToolResultData::JsonResultData(JsonResultData { value })
}

fn toolError(tool: &AITool, message: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: ToolResultData::StringResultData(StringResultData { value: String::new() }),
        error: Some(message),
    }
}

fn toolData(tool: &AITool, data: ToolResultData) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: data,
        error: None,
    }
}

/// The concrete transfer endpoints served by one device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileTransferOperation {
    Begin,
    Chunk,
    Commit,
    ReadChunk,
    PushFile,
}

/// Executes one internal transfer endpoint against the local VFS.
pub struct FileTransferExecutor {
    pub tools: StandardFileSystemTools,
    pub operation: FileTransferOperation,
}

impl operit_tools::ToolExecutionManager::AsyncToolExecutor for FileTransferExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        let required: &[&str] = match self.operation {
            FileTransferOperation::Begin => &["destination_path", "expected_size"],
            FileTransferOperation::Chunk => &["transfer_id", "offset", "content_base64"],
            FileTransferOperation::Commit => &["transfer_id", "expected_sha256"],
            FileTransferOperation::ReadChunk => &["source_path", "offset", "length"],
            FileTransferOperation::PushFile => {
                &["target_node_id", "source_path", "destination_path"]
            }
        };
        for name in required {
            let present = tool
                .parameters
                .iter()
                .any(|parameter| parameter.name == *name && !parameter.value.trim().is_empty());
            if !present {
                return ToolValidationResult {
                    valid: false,
                    errorMessage: format!("{name} parameter is required"),
                };
            }
        }
        ToolValidationResult { valid: true, errorMessage: String::new() }
    }

    fn accessSpec(&self, _tool: &AITool) -> Result<ToolAccessSpec, String> {
        Ok(ToolAccessSpec {
            effect: match self.operation {
                FileTransferOperation::ReadChunk => ToolEffect::READ,
                _ => ToolEffect::WRITE,
            },
            boundary: ToolBoundary::None,
        })
    }

    fn invokeAndStreamAsync<'a>(
        &'a mut self,
        tool: &'a AITool,
        _context: &'a ToolInvocationContext,
    ) -> ToolInvocationFuture<'a> {
        Box::pin(async move {
            let result = match self.operation {
                FileTransferOperation::Begin => self.tools.fsTransferBegin(tool),
                FileTransferOperation::Chunk => self.tools.fsTransferChunk(tool),
                FileTransferOperation::Commit => self.tools.fsTransferCommit(tool),
                FileTransferOperation::ReadChunk => self.tools.fsReadChunk(tool),
                FileTransferOperation::PushFile => self.tools.fsPushFileAsync(tool).await,
            };
            vec![result]
        })
    }
}

impl StandardFileSystemTools {
    /// Opens a receive session and returns its transfer id.
    #[allow(non_snake_case)]
    pub fn fsTransferBegin(&self, tool: &AITool) -> ToolResult {
        evictExpiredSessions();
        let destinationPath = parameterValue(tool, "destination_path");
        let expectedSize = match requiredNumber(tool, "expected_size") {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };
        if expectedSize < 0 {
            return toolError(tool, "expected_size must not be negative".to_string());
        }
        let vfs = self.transferVfs();
        let resolved = match vfs.resolvePath(&destinationPath) {
            Ok(resolved) => resolved,
            Err(error) => return toolError(tool, format!("Invalid destination: {error}")),
        };
        let physicalPath = match resolved.nativePath() {
            Ok(path) => path,
            Err(_) => {
                return toolError(
                    tool,
                    "Transfer destinations must resolve to a native file path".to_string(),
                )
            }
        };
        if let Some(parent) = std::path::Path::new(&physicalPath).parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                return toolError(tool, format!("Cannot create destination directory: {error}"));
            }
        }
        let transferId = uuid::Uuid::new_v4().to_string();
        let tempPath = match std::path::Path::new(&physicalPath)
            .parent()
            .map(|parent| parent.join(format!("operit-part-{transferId}")))
        {
            Some(path) => path,
            None => return toolError(tool, "Invalid destination parent".to_string()),
        };
        if let Err(error) = std::fs::File::create(&tempPath) {
            return toolError(tool, format!("Cannot create transfer temp file: {error}"));
        }
        sessions().lock().expect("transfer sessions mutex poisoned").insert(
            transferId.clone(),
            TransferSession {
                tempPath,
                destinationPath: physicalPath.clone(),
                expectedSize,
                received: 0,
                hasher: Sha256::new(),
                createdMs: nowMs(),
            },
        );
        toolData(
            tool,
            jsonResult(serde_json::json!({
                "transferId": transferId,
                "destination": physicalPath,
            })),
        )
    }

    /// Appends one ordered chunk to an open receive session.
    #[allow(non_snake_case)]
    pub fn fsTransferChunk(&self, tool: &AITool) -> ToolResult {
        let transferId = parameterValue(tool, "transfer_id");
        let offset = match requiredNumber(tool, "offset") {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };
        let contentBase64 = parameterValue(tool, "content_base64");
        let bytes = match STANDARD.decode(contentBase64.as_bytes()) {
            Ok(bytes) => bytes,
            Err(error) => return toolError(tool, format!("Invalid chunk base64: {error}")),
        };
        let mut session = match takeSession(&transferId) {
            Ok(session) => session,
            Err(error) => return toolError(tool, error),
        };
        if offset != session.received {
            let expected = session.received;
            restoreSession(session);
            return toolError(
                tool,
                format!("Chunk offset {offset} does not match the expected offset {expected}"),
            );
        }
        if session.received + bytes.len() as i64 > session.expectedSize {
            let received = session.received;
            let expected = session.expectedSize;
            restoreSession(session);
            return toolError(
                tool,
                format!(
                    "Chunk exceeds the declared size: received {received} + {} > expected {expected}",
                    bytes.len()
                ),
            );
        }
        let append = std::fs::OpenOptions::new()
            .append(true)
            .open(&session.tempPath)
            .and_then(|mut file| file.write_all(&bytes).map(|_| file));
        if let Err(error) = append {
            restoreSession(session);
            return toolError(tool, format!("Cannot append chunk: {error}"));
        }
        session.hasher.update(&bytes);
        session.received += bytes.len() as i64;
        let written = session.received;
        restoreSession(session);
        toolData(tool, jsonResult(serde_json::json!({ "written": written })))
    }

    /// Verifies size and SHA-256, then publishes the destination file.
    #[allow(non_snake_case)]
    pub fn fsTransferCommit(&self, tool: &AITool) -> ToolResult {
        let transferId = parameterValue(tool, "transfer_id");
        let expectedSha256 = parameterValue(tool, "expected_sha256").to_ascii_lowercase();
        let mut session = match takeSession(&transferId) {
            Ok(session) => session,
            Err(error) => return toolError(tool, error),
        };
        if session.received != session.expectedSize {
            let (received, expected) = (session.received, session.expectedSize);
            let partial = session.tempPath.clone();
            restoreSession(session);
            let _ = std::fs::remove_file(partial);
            return toolError(
                tool,
                format!("Transfer incomplete: received {received} of {expected} bytes"),
            );
        }
        let actualSha256 = format!("{:x}", session.hasher.clone().finalize());
        if actualSha256 != expectedSha256 {
            let partial = session.tempPath.clone();
            restoreSession(session);
            let _ = std::fs::remove_file(partial);
            return toolError(
                tool,
                format!("SHA-256 mismatch: expected {expectedSha256}, received {actualSha256}"),
            );
        }
        if let Err(error) = std::fs::rename(&session.tempPath, &session.destinationPath) {
            restoreSession(session);
            return toolError(tool, format!("Cannot publish destination: {error}"));
        }
        let size = session.received;
        let destination = session.destinationPath.clone();
        toolData(
            tool,
            jsonResult(
                serde_json::json!({ "path": destination, "size": size, "sha256": actualSha256 }),
            ),
        )
    }

    /// Reads one bounded slice of a local file for pull-based transfers.
    #[allow(non_snake_case)]
    pub fn fsReadChunk(&self, tool: &AITool) -> ToolResult {
        let sourcePath = parameterValue(tool, "source_path");
        let offset = match requiredNumber(tool, "offset") {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };
        let length = match requiredNumber(tool, "length") {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };
        if offset < 0 || length <= 0 || length as usize > TRANSFER_CHUNK_BYTES {
            return toolError(
                tool,
                format!("offset/length out of range: offset={offset}, length={length}"),
            );
        }
        let vfs = self.transferVfs();
        let resolved = match vfs.resolvePath(&sourcePath) {
            Ok(resolved) => resolved,
            Err(error) => return toolError(tool, format!("Invalid source: {error}")),
        };
        let totalSize = match vfs.fileInfo(&sourcePath) {
            Ok(info) if info.exists && info.fileType != "directory" => info.size,
            Ok(_) => return toolError(tool, format!("Source is not a file: {sourcePath}")),
            Err(error) => return toolError(tool, error),
        };
        let bytes: Result<Vec<u8>, String> = match resolved.nativePath() {
            Ok(nativePath) => std::fs::File::open(&nativePath)
                .and_then(|mut file| {
                    file.seek(SeekFrom::Start(offset as u64))?;
                    let mut buffer = vec![0u8; length as usize];
                    let mut read = 0;
                    loop {
                        let n = file.read(&mut buffer[read..])?;
                        if n == 0 {
                            break;
                        }
                        read += n;
                        if read == buffer.len() {
                            break;
                        }
                    }
                    buffer.truncate(read);
                    Ok(buffer)
                })
                .map_err(|error| error.to_string()),
            Err(_) => {
                // Resource-backed sources have no seekable native path; read once.
                if totalSize > MAX_RESOURCE_SOURCE_BYTES {
                    return toolError(
                        tool,
                        format!(
                            "Resource-backed source exceeds the {MAX_RESOURCE_SOURCE_BYTES} byte transfer cap"
                        ),
                    );
                }
                vfs.readFileBytes(&sourcePath).map(|all| {
                    let start = (offset as usize).min(all.len());
                    let end = (offset as usize + length as usize).min(all.len());
                    all[start..end].to_vec()
                })
            }
        };
        let bytes = match bytes {
            Ok(bytes) => bytes,
            Err(error) => return toolError(tool, format!("Cannot read source: {error}")),
        };
        let eof = offset + bytes.len() as i64 >= totalSize;
        toolData(
            tool,
            jsonResult(serde_json::json!({
                "contentBase64": STANDARD.encode(&bytes),
                "totalSize": totalSize,
                "eof": eof,
            })),
        )
    }

    /// Push orchestrator, runs on the SOURCE device: streams the local source
    /// file to the target device chunk by chunk, then commits with SHA-256.
    #[allow(non_snake_case)]
    pub async fn fsPushFileAsync(&self, tool: &AITool) -> ToolResult {
        let targetNodeId = parameterValue(tool, "target_node_id");
        let sourcePath = parameterValue(tool, "source_path");
        let destinationPath = parameterValue(tool, "destination_path");
        let vfs = self.transferVfs();
        let resolved = match vfs.resolvePath(&sourcePath) {
            Ok(resolved) => resolved,
            Err(error) => return toolError(tool, format!("Invalid source: {error}")),
        };
        let sourceInfo = match vfs.fileInfo(&sourcePath) {
            Ok(info) if info.exists && info.fileType != "directory" => info,
            Ok(_) => return toolError(tool, format!("Source is not a file: {sourcePath}")),
            Err(error) => return toolError(tool, error),
        };
        let beginReply = match self
            .executeInternalTool(
                &targetNodeId,
                FS_TRANSFER_BEGIN_TOOL,
                serde_json::json!({
                    "destination_path": destinationPath,
                    "expected_size": sourceInfo.size.to_string(),
                }),
            )
            .await
        {
            Ok(reply) => reply,
            Err(error) => return toolError(tool, format!("Cannot open transfer: {error}")),
        };
        let transferId = beginReply
            .get("transferId")
            .and_then(serde_json::Value::as_str)
            .map(|value| value.to_string())
            .unwrap_or_default();
        if transferId.is_empty() {
            return toolError(tool, "Target device returned no transfer id".to_string());
        }

        let nativePath = resolved.nativePath().ok();
        let mut hasher = Sha256::new();
        let mut pushed: i64 = 0;
        let pushResult = if let Some(nativePath) = nativePath {
            self.pushNativeChunks(
                &targetNodeId,
                &transferId,
                &nativePath,
                &mut hasher,
                &mut pushed,
            )
            .await
        } else if sourceInfo.size > MAX_RESOURCE_SOURCE_BYTES {
            Err(format!(
                "Resource-backed source exceeds the {MAX_RESOURCE_SOURCE_BYTES} byte transfer cap"
            ))
        } else {
            match vfs.readFileBytes(&sourcePath) {
                Ok(all) => {
                    self.pushBufferedChunks(
                        &targetNodeId,
                        &transferId,
                        &all,
                        &mut hasher,
                        &mut pushed,
                    )
                    .await
                }
                Err(error) => Err(error),
            }
        };
        if let Err(error) = pushResult {
            // Best-effort abort; the commit with an empty hash never publishes.
            drop(
                self.executeInternalTool(
                    &targetNodeId,
                    FS_TRANSFER_COMMIT_TOOL,
                    serde_json::json!({ "transfer_id": transferId, "expected_sha256": "aborted" }),
                )
                .await,
            );
            return toolError(tool, format!("Transfer failed: {error}"));
        }
        if pushed != sourceInfo.size {
            drop(
                self.executeInternalTool(
                    &targetNodeId,
                    FS_TRANSFER_COMMIT_TOOL,
                    serde_json::json!({ "transfer_id": transferId, "expected_sha256": "aborted" }),
                )
                .await,
            );
            return toolError(
                tool,
                format!("Transfer incomplete: pushed {pushed} of {} bytes", sourceInfo.size),
            );
        }
        let sha256 = format!("{:x}", hasher.finalize());
        let commitReply = match self
            .executeInternalTool(
                &targetNodeId,
                FS_TRANSFER_COMMIT_TOOL,
                serde_json::json!({
                    "transfer_id": transferId,
                    "expected_sha256": sha256,
                }),
            )
            .await
        {
            Ok(reply) => reply,
            Err(error) => return toolError(tool, format!("Cannot commit transfer: {error}")),
        };
        let size = commitReply
            .get("size")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(pushed);
        let destination = commitReply
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(destinationPath.as_str());
        toolData(
            tool,
            jsonResult(serde_json::json!({
                "size": size,
                "sha256": sha256,
                "destination": destination,
            })),
        )
    }

    /// Streams a seekable native source to the target chunk by chunk.
    #[allow(non_snake_case)]
    async fn pushNativeChunks(
        &self,
        targetNodeId: &str,
        transferId: &str,
        nativePath: &str,
        hasher: &mut Sha256,
        pushed: &mut i64,
    ) -> Result<(), String> {
        let mut file = std::fs::File::open(nativePath).map_err(|error| error.to_string())?;
        loop {
            let mut buffer = vec![0u8; TRANSFER_CHUNK_BYTES];
            let n = file.read(&mut buffer).map_err(|error| error.to_string())?;
            if n == 0 {
                break;
            }
            buffer.truncate(n);
            self.pushOneChunk(targetNodeId, transferId, *pushed, &buffer).await?;
            hasher.update(&buffer);
            *pushed += n as i64;
        }
        Ok(())
    }

    /// Streams an in-memory (resource-backed) source to the target.
    #[allow(non_snake_case)]
    async fn pushBufferedChunks(
        &self,
        targetNodeId: &str,
        transferId: &str,
        all: &[u8],
        hasher: &mut Sha256,
        pushed: &mut i64,
    ) -> Result<(), String> {
        for chunk in all.chunks(TRANSFER_CHUNK_BYTES) {
            self.pushOneChunk(targetNodeId, transferId, *pushed, chunk).await?;
            hasher.update(chunk);
            *pushed += chunk.len() as i64;
        }
        Ok(())
    }

    /// Sends one ordered chunk to the receiving device.
    #[allow(non_snake_case)]
    async fn pushOneChunk(
        &self,
        targetNodeId: &str,
        transferId: &str,
        offset: i64,
        bytes: &[u8],
    ) -> Result<(), String> {
        self.executeInternalTool(
            targetNodeId,
            FS_TRANSFER_CHUNK_TOOL,
            serde_json::json!({
                "transfer_id": transferId,
                "offset": offset.to_string(),
                "content_base64": STANDARD.encode(bytes),
            }),
        )
        .await
        .map(|_| ())
    }

    /// Pull-based transfer: streams a remote file onto this device.
    /// Returns (size, sha256).
    #[allow(non_snake_case)]
    pub async fn fsPullFromRemote(
        &self,
        sourceNodeId: &str,
        sourceInnerPath: &str,
        destinationPhysical: &str,
    ) -> Result<(i64, String), String> {
        let destination = std::path::Path::new(destinationPhysical);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("Cannot create destination directory: {error}"))?;
        }
        let tempPath = destination
            .parent()
            .map(|parent| parent.join(format!("operit-part-{}", uuid::Uuid::new_v4())))
            .ok_or("Invalid destination parent")?;
        let first = self
            .executeInternalTool(
                sourceNodeId,
                FS_READ_CHUNK_TOOL,
                serde_json::json!({
                    "source_path": sourceInnerPath,
                    "offset": "0",
                    "length": TRANSFER_CHUNK_BYTES.to_string(),
                }),
            )
            .await?;
        let totalSize = first
            .get("totalSize")
            .and_then(serde_json::Value::as_i64)
            .ok_or("Source device returned no total size")?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tempPath)
            .map_err(|error| format!("Cannot open transfer temp file: {error}"))?;
        let mut hasher = Sha256::new();
        let mut offset: i64 = 0;
        loop {
            let contentBase64: String = if offset == 0 {
                first
                    .get("contentBase64")
                    .and_then(serde_json::Value::as_str)
                    .ok_or("Source device returned no chunk content")?
                    .to_string()
            } else {
                let reply = self
                    .executeInternalTool(
                        sourceNodeId,
                        FS_READ_CHUNK_TOOL,
                        serde_json::json!({
                            "source_path": sourceInnerPath,
                            "offset": offset.to_string(),
                            "length": TRANSFER_CHUNK_BYTES.to_string(),
                        }),
                    )
                    .await?;
                reply
                    .get("contentBase64")
                    .and_then(serde_json::Value::as_str)
                    .ok_or("Source device returned no chunk content")?
                    .to_string()
            };
            let bytes = STANDARD
                .decode(contentBase64.as_bytes())
                .map_err(|error| format!("Source device sent invalid base64: {error}"))?;
            if bytes.is_empty() {
                break;
            }
            hasher.update(&bytes);
            file.write_all(&bytes)
                .map_err(|error| format!("Cannot write transfer temp file: {error}"))?;
            offset += bytes.len() as i64;
            if offset >= totalSize {
                break;
            }
        }
        drop(file);
        if offset != totalSize {
            let _ = std::fs::remove_file(&tempPath);
            return Err(format!(
                "Transfer incomplete: received {offset} of {totalSize} bytes"
            ));
        }
        let sha256 = format!("{:x}", hasher.finalize());
        std::fs::rename(&tempPath, destinationPhysical)
            .map_err(|error| format!("Cannot publish destination: {error}"))?;
        Ok((totalSize, sha256))
    }

    /// Dispatches one internal transport tool to an explicit device.
    #[allow(non_snake_case)]
    pub async fn executeInternalTool(
        &self,
        nodeId: &str,
        toolName: &str,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let request = JsToolCallRequest {
            tool_type: "default".to_string(),
            tool_name: toolName.to_string(),
            parameters: parameters
                .as_object()
                .map(|object| {
                    object
                        .iter()
                        .map(|(key, value)| match value {
                            serde_json::Value::String(text) => {
                                (key.clone(), serde_json::Value::String(text.clone()))
                            }
                            other => (
                                key.clone(),
                                serde_json::Value::String(other.to_string()),
                            ),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            device: Some(JsDeviceToolCall {
                node_id: nodeId.to_string(),
                invocation_id: uuid::Uuid::new_v4().to_string(),
                action: JsDeviceToolAction::Execute,
                timeout_ms: 120_000,
            }),
        };
        let gateway = toolRuntimeFileGateway(self.transferRuntimeSupport());
        let reply = gateway.dispatchRemoteTool(request).await;
        if !reply.success {
            return Err(reply.error.unwrap_or_else(|| "Transfer step failed".to_string()));
        }
        match reply.data {
            JsToolCallResultData::Value(value) => Ok(value),
            JsToolCallResultData::Binary(bytes) => Ok(serde_json::Value::String(
                String::from_utf8_lossy(&bytes).to_string(),
            )),
        }
    }
}

/// One endpoint of a cross-node transfer.
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct TransferEndpoint {
    /// `None` when the endpoint is this device.
    pub nodeId: Option<String>,
    /// Target-local path (prefix stripped).
    pub innerPath: String,
    /// The path exactly as typed, for result echo.
    pub typed: String,
}

/// A copy/move whose endpoints live on different devices.
#[allow(non_snake_case)]
pub struct CrossNodeTransfer {
    pub source: TransferEndpoint,
    pub destination: TransferEndpoint,
}

impl StandardFileSystemTools {
    /// Classifies a two-path tool call. Returns `Some` only when the endpoints
    /// live on different devices; `None` keeps the existing single-device flow.
    #[allow(non_snake_case)]
    pub fn resolveTransferEndpoints(
        &self,
        tool: &AITool,
        pathParameters: &[&str],
    ) -> Result<Option<(TransferEndpoint, TransferEndpoint)>, String> {
        let mut endpoints: Vec<TransferEndpoint> = Vec::new();
        for name in pathParameters {
            let value = parameterValue(tool, name);
            if value.trim().is_empty() {
                continue;
            }
            if splitQualified(&value) {
                let state = self.transferRuntimeSupport().coreNodeRouteState().map_err(|error| {
                    format!("Cross-device path '{value}' requires CoreNode routing: {error}")
                })?;
                match state.resolveQualifiedVfsPath(&value)? {
                    crate::runtime_support::QualifiedVfsResolution::LocalPath(innerPath) => {
                        endpoints.push(TransferEndpoint {
                            nodeId: None,
                            innerPath: innerPath.to_string(),
                            typed: value.trim().to_string(),
                        });
                    }
                    crate::runtime_support::QualifiedVfsResolution::RemoteTarget {
                        node,
                        innerPath,
                        ..
                    } => {
                        if !node.canExecute {
                            return Err(format!(
                                "Device cannot run file operations: {}",
                                crate::runtime_support::coreNodeReferenceLabel(node)
                            ));
                        }
                        if !node.reachable {
                            return Err(format!(
                                "Device is currently unreachable: {}",
                                crate::runtime_support::coreNodeReferenceLabel(node)
                            ));
                        }
                        endpoints.push(TransferEndpoint {
                            nodeId: Some(node.nodeId.clone()),
                            innerPath: innerPath.to_string(),
                            typed: value.trim().to_string(),
                        });
                    }
                }
            } else {
                endpoints.push(TransferEndpoint {
                    nodeId: None,
                    innerPath: value.trim().to_string(),
                    typed: value.trim().to_string(),
                });
            }
        }
        if endpoints.len() < 2 {
            return Ok(None);
        }
        let (source, destination) = (endpoints[0].clone(), endpoints[1].clone());
        if source.nodeId == destination.nodeId {
            return Ok(None);
        }
        Ok(Some((source, destination)))
    }

    fn splitQualifiedCalled(text: &str) -> bool {
        splitQualified(text)
    }

    /// Streams one file between two devices and verifies size + SHA-256.
    /// Returns (bytes, sha256).
    #[allow(non_snake_case)]
    pub async fn executeCrossNodeTransfer(
        &self,
        source: &TransferEndpoint,
        destination: &TransferEndpoint,
    ) -> Result<(i64, String), String> {
        if source.nodeId.is_none() {
            let vfs = self.transferVfs();
            let sourceInfo = vfs
                .fileInfo(&source.innerPath)
                .map_err(|error| format!("Cannot inspect source: {error}"))?;
            if !sourceInfo.exists {
                return Err(format!("Source does not exist: {}", source.typed));
            }
            if sourceInfo.fileType == "directory" {
                return Err(
                    "Cross-node transfer supports files only; compress the directory first"
                        .to_string(),
                );
            }
            let synthetic = AITool {
                name: FS_PUSH_FILE_TOOL.to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "target_node_id".to_string(),
                        value: destination.nodeId.clone().unwrap_or_default(),
                    },
                    ToolParameter {
                        name: "source_path".to_string(),
                        value: source.innerPath.clone(),
                    },
                    ToolParameter {
                        name: "destination_path".to_string(),
                        value: destination.innerPath.clone(),
                    },
                ],
            };
            let result = self.fsPushFileAsync(&synthetic).await;
            return extractTransferSizeSha(result);
        }
        if destination.nodeId.is_none() {
            let vfs = self.transferVfs();
            let resolved = vfs
                .resolvePath(&destination.innerPath)
                .map_err(|error| format!("Invalid destination: {error}"))?;
            let physicalPath = resolved
                .nativePath()
                .map_err(|_| {
                    "Transfer destinations must resolve to a native file path".to_string()
                })?;
            return self
                .fsPullFromRemote(
                    source.nodeId.as_deref().unwrap_or_default(),
                    &source.innerPath,
                    &physicalPath,
                )
                .await;
        }
        // Remote -> remote: the source device pushes directly to the target.
        let reply = self
            .executeInternalTool(
                source.nodeId.as_deref().unwrap_or_default(),
                FS_PUSH_FILE_TOOL,
                serde_json::json!({
                    "target_node_id": destination.nodeId.clone().unwrap_or_default(),
                    "source_path": source.innerPath,
                    "destination_path": destination.innerPath,
                }),
            )
            .await?;
        extract_size_sha(&reply).ok_or_else(|| "Target returned no transfer summary".to_string())
    }

    /// Removes the source after a successful cross-node move.
    #[allow(non_snake_case)]
    pub async fn deleteTransferSource(&self, source: &TransferEndpoint) -> Result<(), String> {
        match &source.nodeId {
            None => {
                let vfs = self.transferVfs();
                vfs.deleteFile(&source.innerPath, true)
            }
            Some(nodeId) => {
                self.executeInternalTool(
                    nodeId,
                    "delete_file",
                    serde_json::json!({ "path": source.innerPath }),
                )
                .await
                .map(|_| ())
            }
        }
    }
}

fn extractTransferSizeSha(result: ToolResult) -> Result<(i64, String), String> {
    if !result.success {
        return Err(result
            .error
            .unwrap_or_else(|| "Transfer failed".to_string()));
    }
    match result.result {
        ToolResultData::JsonResultData(data) => {
            extract_size_sha(&data.value).ok_or_else(|| "Target returned no transfer summary".to_string())
        }
        _ => Err("Target returned an unexpected transfer result".to_string()),
    }
}

fn extract_size_sha(value: &serde_json::Value) -> Option<(i64, String)> {
    let size = value.get("size").and_then(serde_json::Value::as_i64)?;
    let sha256 = value
        .get("sha256")
        .and_then(serde_json::Value::as_str)?
        .to_string();
    Some((size, sha256))
}

/// Syntactic check mirroring the resolver's prefix rule.
fn splitQualified(text: &str) -> bool {
    crate::runtime_support::splitQualifiedVfsPath(text).is_some()
}
