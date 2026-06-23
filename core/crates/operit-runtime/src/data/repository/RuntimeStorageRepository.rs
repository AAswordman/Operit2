use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;

use operit_util::OperitPaths;

pub struct RuntimeStorageRepository;

impl RuntimeStorageRepository {
    pub fn new() -> Self {
        Self
    }

    #[allow(non_snake_case)]
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
    pub fn writeText(&self, path: String, content: String) -> Result<(), String> {
        defaultRuntimeStorageHost()
            .writeBytes(&path, content.as_bytes())
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    pub fn writeBase64(&self, path: String, base64Content: String) -> Result<(), String> {
        let bytes = STANDARD
            .decode(base64Content.as_bytes())
            .map_err(|error| error.to_string())?;
        defaultRuntimeStorageHost()
            .writeBytes(&path, &bytes)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    pub fn webSessionBrowserBookmarksPath(&self) -> String {
        OperitPaths::RUNTIME_WEBSESSION_BROWSER_BOOKMARKS_PATH.to_string()
    }

    #[allow(non_snake_case)]
    pub fn webSessionBrowserHistoryPath(&self) -> String {
        OperitPaths::RUNTIME_WEBSESSION_BROWSER_HISTORY_PATH.to_string()
    }

    #[allow(non_snake_case)]
    pub fn webSessionBrowserDownloadsPath(&self) -> String {
        OperitPaths::RUNTIME_WEBSESSION_BROWSER_DOWNLOADS_PATH.to_string()
    }

    #[allow(non_snake_case)]
    pub fn webSessionBrowserDownloadFilesDirPath(&self) -> String {
        OperitPaths::RUNTIME_WEBSESSION_BROWSER_DOWNLOAD_FILES_DIR_PATH.to_string()
    }

    #[allow(non_snake_case)]
    pub fn webSessionUserscriptsStatePath(&self) -> String {
        OperitPaths::RUNTIME_WEBSESSION_USERSCRIPTS_STATE_PATH.to_string()
    }
}
