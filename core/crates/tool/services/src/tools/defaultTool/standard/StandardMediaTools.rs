#![allow(non_snake_case)]

use std::path::Path;

use operit_host_api::HostManager::HostManager;
use operit_plugin_sdk::js_sdk::results::MediaRegistrationResultData;
use operit_tools::files::PathMapper::PathMapper;
use operit_tools::files::VisualFileSystem::VisualFileSystem;
use operit_util::MediaRegistration::{register_media_base64, register_media_bytes, RegisteredMedia};
use serde::Deserialize;

use super::ToolArguments::decodeArguments;
use crate::tools::ToolInvocation::prepareInvocation;
use crate::tools::ToolResultDataClasses::ToolResultData;
use crate::ToolExecutionManager::{
    PreparedToolInvocation, SharedToolExecutor, ToolCall, ToolEffect, ToolInvocationContext,
};

/// Registers base64 media through the common tool pipeline.
pub(crate) struct RegisterMediaExecutor;

/// Registers a virtual file through the common tool pipeline.
pub(crate) struct RegisterMediaFromPathExecutor;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisterArguments {
    #[serde(rename = "type")]
    kind: String,
    base64: String,
    #[serde(default)]
    mimeType: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisterFromPathArguments {
    #[serde(rename = "type")]
    kind: String,
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeMediaRequest {
    #[serde(rename = "type")]
    kind: String,
    source: String,
    #[serde(default)]
    base64: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    mimeType: Option<String>,
}

impl SharedToolExecutor for RegisterMediaExecutor {
    fn prepare<'a>(
        &'a self,
        request: &'a ToolCall,
        _: &'a ToolInvocationContext,
        checkAccess: bool,
    ) -> Result<PreparedToolInvocation<'a>, String> {
        let args: RegisterArguments = decodeArguments(request)?;
        Ok(prepareInvocation(
            request,
            checkAccess,
            ToolEffect::READ,
            async move {
                Ok(ToolResultData::MediaRegistrationResultData(
                    registration_from_base64(&args.kind, &args.base64, args.mimeType.as_deref())?,
                ))
            },
        ))
    }
}

impl SharedToolExecutor for RegisterMediaFromPathExecutor {
    fn prepare<'a>(
        &'a self,
        request: &'a ToolCall,
        context: &'a ToolInvocationContext,
        checkAccess: bool,
    ) -> Result<PreparedToolInvocation<'a>, String> {
        let args: RegisterFromPathArguments = decodeArguments(request)?;
        let host = context.host().getContext().clone();
        Ok(prepareInvocation(
            request,
            checkAccess,
            ToolEffect::READ,
            async move {
                Ok(ToolResultData::MediaRegistrationResultData(
                    registration_from_path(&host, &args.kind, &args.path)?,
                ))
            },
        ))
    }
}

/// Registers one native media request and returns its JSON result.
pub(crate) fn register_native_media(host: &HostManager, request_json: &str) -> Result<String, String> {
    let request: NativeMediaRequest = serde_json::from_str(request_json)
        .map_err(|error| format!("media registration request is invalid: {error}"))?;
    let mime = request
        .mimeType
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let result = match request.source.as_str() {
        "base64" => {
            let base64 = request
                .base64
                .as_deref()
                .ok_or("media registration requires base64 data")?;
            registration_from_base64(&request.kind, base64, mime)?
        }
        "path" => {
            let path = request
                .path
                .as_deref()
                .ok_or("media registration requires a path")?;
            registration_from_path(host, &request.kind, path)?
        }
        _ => return Err("media registration source must be base64 or path".to_string()),
    };
    serde_json::to_string(&result).map_err(|error| error.to_string())
}

fn registration_from_base64(
    kind: &str,
    base64: &str,
    mime_type: Option<&str>,
) -> Result<MediaRegistrationResultData, String> {
    registration_result(kind, register_media_base64(kind, base64, mime_type)?)
}

fn registration_from_path(
    host: &HostManager,
    kind: &str,
    path: &str,
) -> Result<MediaRegistrationResultData, String> {
    if path.is_empty() {
        return Err("media path is empty".to_string());
    }
    let bytes = read_vfs_bytes(host, path)?;
    let mime = mime_hint_for_path(kind, path)?;
    registration_result(kind, register_media_bytes(kind, &bytes, mime.as_deref())?)
}

fn registration_result(
    kind: &str,
    registered: RegisteredMedia,
) -> Result<MediaRegistrationResultData, String> {
    Ok(MediaRegistrationResultData {
        link: format!("<link type=\"{kind}\" id=\"{}\"></link>", registered.id),
        id: registered.id,
        mimeType: registered.mime_type,
        width: registered.width,
        height: registered.height,
    })
}

fn read_vfs_bytes(host: &HostManager, path: &str) -> Result<Vec<u8>, String> {
    if operit_tools::runtime_support::splitQualifiedVfsPath(path).is_some() {
        return Err(
            "media_register_from_path does not support cross-device paths; copy the file to this device first".to_string(),
        );
    }
    let file_system = host
        .fileSystemHost
        .clone()
        .ok_or("FileSystemHost is required for media registration")?;
    let storage = host
        .runtimeStorageHost
        .clone()
        .ok_or("Runtime storage is required for media registration")?;
    let runtime_root = storage
        .runtimeRootDir()
        .ok_or("Runtime root is unavailable")?;
    let workspace_root = storage
        .workspaceRootDir()
        .ok_or("Workspace root is unavailable")?;
    VisualFileSystem::new(
        file_system,
        PathMapper::new(runtime_root, workspace_root, storage),
    )
    .readFileBytes(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_util::ImagePoolManager::ImagePoolManager;
    use operit_util::MediaPoolManager::MediaPoolManager;

    #[test]
    fn native_base64_request_returns_an_image_link() {
        let json = serde_json::json!({
            "type": "image",
            "source": "base64",
            "base64": "iVBORw0KGgoAAAANSUhEUgAAAAIAAAAD",
        })
        .to_string();
        let result: MediaRegistrationResultData = serde_json::from_str(
            &register_native_media(&HostManager::default(), &json).expect("register"),
        )
        .expect("result");
        assert_eq!(result.mimeType, "image/png");
        assert_eq!(result.width, Some(2));
        assert_eq!(result.height, Some(3));
        assert_eq!(result.link, format!("<link type=\"image\" id=\"{}\"></link>", result.id));
        ImagePoolManager::remove_image(&result.id);
        let audio = register_native_media(
            &HostManager::default(),
            r#"{"type":"audio","source":"base64","base64":"YQ==","mimeType":"audio/mpeg"}"#,
        )
        .expect("audio");
        let audio: MediaRegistrationResultData = serde_json::from_str(&audio).expect("audio json");
        assert!(audio.width.is_none());
        assert!(audio.link.starts_with("<link type=\"audio\" id=\""));
        MediaPoolManager::remove_media(&audio.id);
    }
}

fn mime_hint_for_path(kind: &str, path: &str) -> Result<Option<String>, String> {
    let extension = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mime = match (kind, extension.as_str()) {
        ("image", "jpg" | "jpeg") => "image/jpeg",
        ("image", "gif") => "image/gif",
        ("image", "webp") => "image/webp",
        ("image", "bmp") => "image/bmp",
        ("image", _) => "image/png",
        ("audio", "mp3") => "audio/mpeg",
        ("audio", "wav") => "audio/wav",
        ("audio", "m4a") => "audio/mp4",
        ("audio", "aac") => "audio/aac",
        ("audio", "flac") => "audio/flac",
        ("audio", "ogg") => "audio/ogg",
        ("audio", "opus") => "audio/opus",
        ("video", "mp4") => "video/mp4",
        ("video", "mkv") => "video/x-matroska",
        ("video", "mov") => "video/quicktime",
        ("video", "webm") => "video/webm",
        ("video", "avi") => "video/x-msvideo",
        ("video", "m4v") => "video/x-m4v",
        ("audio" | "video", _) => {
            return Err(format!("{kind} file extension is not supported: {extension}"));
        }
        _ => return Err("media type must be image, audio, or video".to_string()),
    };
    Ok(Some(mime.to_string()))
}
