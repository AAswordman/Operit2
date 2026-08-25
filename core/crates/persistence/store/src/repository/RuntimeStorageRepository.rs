use crate::RuntimeFileSyncStore::RuntimeFileSyncStore;
use crate::RuntimeStorageHost::defaultRuntimeStorageHost;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use operit_util::OperitPaths;
use operit_util::RuntimeStorageLayout::{
    runtimeStorageOwnership, RuntimeStorageOwnership, RUNTIME_SYNC_DIR_PATH,
};

pub struct RuntimeStorageRepository;

impl RuntimeStorageRepository {
    /// Creates a repository that reads and writes through the runtime storage host.
    pub fn new() -> Self {
        Self
    }

    #[allow(non_snake_case)]
    /// Reads a UTF-8 text object from runtime storage.
    pub fn readText(&self, path: String) -> Result<Option<String>, String> {
        let storageHost = defaultRuntimeStorageHost();
        if !storageHost.exists(&path).map_err(|error| error.message)? {
            return Ok(None);
        }
        let bytes = storageHost
            .readBytes(&path)
            .map_err(|error| error.message)?;
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    /// Reads a runtime storage object and returns its base64 representation.
    pub fn readBase64(&self, path: String) -> Result<Option<String>, String> {
        let storageHost = defaultRuntimeStorageHost();
        if !storageHost.exists(&path).map_err(|error| error.message)? {
            return Ok(None);
        }
        let bytes = storageHost
            .readBytes(&path)
            .map_err(|error| error.message)?;
        Ok(Some(STANDARD.encode(bytes)))
    }

    #[allow(non_snake_case)]
    /// Writes UTF-8 text content to runtime storage.
    pub fn writeText(&self, path: String, content: String) -> Result<(), String> {
        let storageHost = defaultRuntimeStorageHost();
        match runtimeStorageOwnership(&path)? {
            RuntimeStorageOwnership::Space => {
                runtimeFileSyncStore(storageHost).writeBytes(&path, content.as_bytes())
            }
            RuntimeStorageOwnership::CoreNode | RuntimeStorageOwnership::Ephemeral => storageHost
                .writeBytes(&path, content.as_bytes())
                .map_err(|error| error.message),
        }
    }

    #[allow(non_snake_case)]
    /// Appends UTF-8 text content to runtime storage.
    pub fn appendText(&self, path: String, content: String) -> Result<(), String> {
        let storageHost = defaultRuntimeStorageHost();
        match runtimeStorageOwnership(&path)? {
            RuntimeStorageOwnership::Space => {
                runtimeFileSyncStore(storageHost).appendBytes(&path, content.as_bytes())
            }
            RuntimeStorageOwnership::CoreNode | RuntimeStorageOwnership::Ephemeral => storageHost
                .appendBytes(&path, content.as_bytes())
                .map_err(|error| error.message),
        }
    }

    #[allow(non_snake_case)]
    /// Decodes base64 content and writes the bytes to runtime storage.
    pub fn writeBase64(&self, path: String, base64Content: String) -> Result<(), String> {
        let bytes = STANDARD
            .decode(base64Content.as_bytes())
            .map_err(|error| error.to_string())?;
        let storageHost = defaultRuntimeStorageHost();
        match runtimeStorageOwnership(&path)? {
            RuntimeStorageOwnership::Space => {
                runtimeFileSyncStore(storageHost).writeBytes(&path, &bytes)
            }
            RuntimeStorageOwnership::CoreNode | RuntimeStorageOwnership::Ephemeral => storageHost
                .writeBytes(&path, &bytes)
                .map_err(|error| error.message),
        }
    }

    /// Deletes one runtime storage object using its registered synchronization scope.
    pub fn delete(&self, path: String) -> Result<(), String> {
        let storageHost = defaultRuntimeStorageHost();
        match runtimeStorageOwnership(&path)? {
            RuntimeStorageOwnership::Space => runtimeFileSyncStore(storageHost).delete(&path),
            RuntimeStorageOwnership::CoreNode | RuntimeStorageOwnership::Ephemeral => storageHost
                .delete(&path, false)
                .map_err(|error| error.message),
        }
    }

    #[allow(non_snake_case)]
    /// Returns the runtime storage path for browser bookmark data.
    pub fn webSessionBrowserBookmarksPath(&self) -> String {
        OperitPaths::RUNTIME_WEBSESSION_BROWSER_BOOKMARKS_PATH.to_string()
    }

    #[allow(non_snake_case)]
    /// Returns the runtime storage path for browser history data.
    pub fn webSessionBrowserHistoryPath(&self) -> String {
        OperitPaths::RUNTIME_WEBSESSION_BROWSER_HISTORY_PATH.to_string()
    }

    #[allow(non_snake_case)]
    /// Returns the runtime storage path for browser download metadata.
    pub fn webSessionBrowserDownloadsPath(&self) -> String {
        OperitPaths::RUNTIME_WEBSESSION_BROWSER_DOWNLOADS_PATH.to_string()
    }

    #[allow(non_snake_case)]
    /// Returns the runtime storage directory for character avatar assets.
    pub fn characterAvatarsDirPath(&self) -> String {
        OperitPaths::RUNTIME_CHARACTER_AVATARS_DIR_PATH.to_string()
    }

    #[allow(non_snake_case)]
    /// Returns the runtime storage directory path for downloaded browser files.
    pub fn webSessionBrowserDownloadFilesDirPath(&self) -> String {
        OperitPaths::RUNTIME_WEBSESSION_BROWSER_DOWNLOAD_FILES_DIR_PATH.to_string()
    }

    #[allow(non_snake_case)]
    /// Returns the runtime storage directory path for imported theme assets.
    pub fn themeAssetsDirPath(&self) -> String {
        OperitPaths::RUNTIME_THEME_ASSETS_DIR_PATH.to_string()
    }

    #[allow(non_snake_case)]
    /// Returns the runtime storage directory for generated share images.
    pub fn shareImageDirPath(&self) -> String {
        OperitPaths::RUNTIME_SHARE_IMAGE_DIR_PATH.to_string()
    }

    #[allow(non_snake_case)]
    /// Returns the runtime storage directory for exported share images.
    pub fn shareImageExportsDirPath(&self) -> String {
        OperitPaths::RUNTIME_SHARE_IMAGE_EXPORTS_DIR_PATH.to_string()
    }

    #[allow(non_snake_case)]
    /// Returns the runtime storage directory for staged workspace videos.
    pub fn workspaceVideoDirPath(&self) -> String {
        OperitPaths::RUNTIME_WORKSPACE_VIDEO_DIR_PATH.to_string()
    }

    #[allow(non_snake_case)]
    /// Returns the runtime storage directory for Compose DSL selected files.
    pub fn composeDslWebViewFilesDirPath(&self) -> String {
        OperitPaths::RUNTIME_COMPOSE_DSL_WEBVIEW_FILES_DIR_PATH.to_string()
    }

    #[allow(non_snake_case)]
    /// Returns the runtime storage directory for materialized Link Access web assets.
    pub fn linkAccessWebAssetsDirPath(&self) -> String {
        OperitPaths::RUNTIME_LINK_ACCESS_WEB_ASSETS_DIR_PATH.to_string()
    }

    #[allow(non_snake_case)]
    /// Returns the runtime storage path for the client log.
    pub fn clientLogPath(&self) -> String {
        OperitPaths::RUNTIME_CLIENT_LOG_PATH.to_string()
    }

    #[allow(non_snake_case)]
    /// Returns the runtime storage path for userscript state data.
    pub fn webSessionUserscriptsStatePath(&self) -> String {
        OperitPaths::RUNTIME_WEBSESSION_USERSCRIPTS_STATE_PATH.to_string()
    }
}

/// Creates the synchronized runtime file store for one host-owned repository operation.
#[allow(non_snake_case)]
fn runtimeFileSyncStore(
    storageHost: std::sync::Arc<dyn operit_host_api::RuntimeStorageHost>,
) -> RuntimeFileSyncStore {
    RuntimeFileSyncStore::new(storageHost, RUNTIME_SYNC_DIR_PATH.to_string())
}
