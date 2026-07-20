use operit_host_api::{HostError, HostResult, RuntimeStorageEntry, RuntimeStorageHost};
use operit_plugin_sdk::execution_result::JsExecutionResult;
use operit_plugin_sdk::js_sdk::{JsFuture, JsHostError};
use operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost;
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug)]
struct TestRuntimeStorageHost {
    runtime_root: PathBuf,
    workspace_root: PathBuf,
}

impl TestRuntimeStorageHost {
    /// Creates a runtime storage host with explicit runtime and workspace roots.
    fn new(runtime_root: PathBuf, workspace_root: PathBuf) -> Self {
        Self {
            runtime_root,
            workspace_root,
        }
    }

    /// Resolves a virtual runtime storage path into the test runtime root.
    fn resolve(&self, path: &str) -> HostResult<PathBuf> {
        let path = Path::new(path);
        if path.is_absolute() {
            return Err(HostError::new(format!(
                "Runtime storage path must be relative: {}",
                path.display()
            )));
        }
        let mut resolved = self.runtime_root.clone();
        for component in path.components() {
            match component {
                Component::Normal(segment) => resolved.push(segment),
                Component::CurDir => {}
                _ => {
                    return Err(HostError::new(format!(
                        "Invalid runtime storage path: {}",
                        path.display()
                    )))
                }
            }
        }
        Ok(resolved)
    }
}

impl RuntimeStorageHost for TestRuntimeStorageHost {
    /// Returns the test runtime root directory.
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        Some(self.runtime_root.clone())
    }

    /// Returns the test workspace root directory.
    fn workspaceRootDir(&self) -> Option<PathBuf> {
        Some(self.workspace_root.clone())
    }

    /// Reads bytes from the test runtime root.
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        Ok(std::fs::read(self.resolve(path)?)?)
    }

    /// Writes bytes into the test runtime root.
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Deletes an entry from the test runtime root.
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        let path = self.resolve(path)?;
        if !path.exists() {
            return Ok(());
        }
        if path.is_dir() {
            if recursive {
                std::fs::remove_dir_all(path)?;
            } else {
                std::fs::remove_dir(path)?;
            }
        } else {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Checks whether an entry exists inside the test runtime root.
    fn exists(&self, path: &str) -> HostResult<bool> {
        Ok(self.resolve(path)?.exists())
    }

    /// Lists entries under a prefix inside the test runtime root.
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        let directory = self.resolve(prefix)?;
        let mut entries = Vec::new();
        if !directory.exists() {
            return Ok(entries);
        }
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let path = entry
                .path()
                .strip_prefix(&self.runtime_root)
                .map_err(|error| HostError::new(error.to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            entries.push(RuntimeStorageEntry {
                path,
                isDirectory: metadata.is_dir(),
                size: metadata.len() as i64,
            });
        }
        Ok(entries)
    }
}

/// Registers process-wide test runtime storage roots.
pub fn register_test_runtime_storage(label: &str) {
    let root = std::env::temp_dir().join(format!("operit-runtime-{label}"));
    let runtime_root = root.join("runtime");
    let workspace_root = root.join("workspace");
    std::fs::create_dir_all(&runtime_root).expect("test runtime root");
    std::fs::create_dir_all(&workspace_root).expect("test workspace root");
    let host = Arc::new(TestRuntimeStorageHost::new(
        runtime_root.clone(),
        workspace_root.clone(),
    ));
    setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(runtime_root, workspace_root));
    setDefaultRuntimeStorageHost(host);
}

/// Creates a rejected JavaScript host future for test-only host methods.
pub fn rejecting_js_future<T>(message: &'static str) -> JsFuture<T> {
    Box::pin(async move { Err(JsHostError::new(message)) })
}

/// Unwraps a JavaScript execution result into its serialized output string.
pub fn expect_js_output(output: JsExecutionResult<Option<String>>, context: &str) -> String {
    output
        .expect(context)
        .expect("JavaScript execution should return a value")
}

#[macro_export]
macro_rules! impl_rejecting_js_tools_host {
    ($host:ty) => {
        #[allow(non_snake_case)]
        impl operit_plugin_sdk::js_sdk::files::FilesHost for $host {
            /// Rejects directory listing in this test host.
            fn list(&self, _path: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::DirectoryListingData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.list is not part of this test")
            }

            /// Rejects text file reads in this test host.
            fn read_overload_1(&self, _path: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileContentData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.read is not part of this test")
            }

            /// Rejects option-based file reads in this test host.
            fn read_overload_2(&self, _options: operit_plugin_sdk::js_sdk::files::FilesReadFileOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileContentData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.read is not part of this test")
            }

            /// Rejects partial file reads in this test host.
            fn readPart(&self, _path: String, _startLine: Option<f64>, _endLine: Option<f64>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FilePartContentData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.readPart is not part of this test")
            }

            /// Rejects text file writes in this test host.
            fn write(&self, _path: String, _content: String, _append: Option<bool>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.write is not part of this test")
            }

            /// Rejects binary file writes in this test host.
            fn writeBinary(&self, _path: String, _base64Content: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.writeBinary is not part of this test")
            }

            /// Rejects binary file reads in this test host.
            fn readBinary(&self, _path: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BinaryFileContentData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.readBinary is not part of this test")
            }

            /// Rejects file deletion in this test host.
            fn deleteFile(&self, _path: String, _recursive: Option<bool>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.deleteFile is not part of this test")
            }

            /// Rejects file existence checks in this test host.
            fn exists(&self, _path: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileExistsData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.exists is not part of this test")
            }

            /// Rejects file moves in this test host.
            fn r#move(&self, _source: String, _destination: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.move is not part of this test")
            }

            /// Rejects file copies in this test host.
            fn copy(&self, _source: String, _destination: String, _recursive: Option<bool>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.copy is not part of this test")
            }

            /// Rejects directory creation in this test host.
            fn mkdir(&self, _path: String, _create_parents: Option<bool>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.mkdir is not part of this test")
            }

            /// Rejects file finding in this test host.
            fn find(&self, _path: String, _pattern: String, _options: Option<std::collections::BTreeMap<String, serde_json::Value>>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FindFilesResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.find is not part of this test")
            }

            /// Rejects code searching in this test host.
            fn grep(&self, _path: String, _pattern: String, _options: Option<operit_plugin_sdk::js_sdk::files::FilesHostGrepOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::GrepResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.grep is not part of this test")
            }

            /// Rejects intent-based searching in this test host.
            fn grepContext(&self, _path: String, _intent: String, _options: Option<operit_plugin_sdk::js_sdk::files::FilesHostGrepContextOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::GrepResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.grepContext is not part of this test")
            }

            /// Rejects file metadata reads in this test host.
            fn info(&self, _path: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileInfoData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.info is not part of this test")
            }

            /// Rejects patch application in this test host.
            fn apply(&self, _path: String, _type: operit_plugin_sdk::js_sdk::files::ApplyFileType, _old: Option<String>, _newContent: Option<String>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileApplyResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.apply is not part of this test")
            }

            /// Rejects file creation in this test host.
            fn create(&self, _path: String, _newContent: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileApplyResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.create is not part of this test")
            }

            /// Rejects file editing in this test host.
            fn edit(&self, _path: String, _oldContent: String, _newContent: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileApplyResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.edit is not part of this test")
            }

            /// Rejects archive creation in this test host.
            fn zip(&self, _source: String, _destination: String, _include_root_directory: Option<bool>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.zip is not part of this test")
            }

            /// Rejects archive extraction in this test host.
            fn unzip(&self, _source: String, _destination: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.unzip is not part of this test")
            }

            /// Rejects host file opening in this test host.
            fn open(&self, _path: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.open is not part of this test")
            }

            /// Rejects host file sharing in this test host.
            fn share(&self, _path: String, _title: Option<String>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.share is not part of this test")
            }

            /// Rejects file downloads in this test host.
            fn download_overload_1(&self, _url: String, _destination: String, _headers: Option<std::collections::BTreeMap<String, String>>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.download is not part of this test")
            }

            /// Rejects option-based file downloads in this test host.
            fn download_overload_2(&self, _options: operit_plugin_sdk::js_sdk::files::FilesHostDownloadOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::FileOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Files.download is not part of this test")
            }
        }

        #[allow(non_snake_case)]
        impl operit_plugin_sdk::js_sdk::network::NetHost for $host {
            /// Rejects HTTP GET in this test host.
            fn httpGet(&self, _url: String, _ignore_ssl: Option<bool>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::HttpResponseData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.httpGet is not part of this test")
            }

            /// Rejects HTTP POST in this test host.
            fn httpPost(&self, _url: String, _body: operit_plugin_sdk::js_sdk::network::NetHostHttpPostBody, _ignore_ssl: Option<bool>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::HttpResponseData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.httpPost is not part of this test")
            }

            /// Rejects webpage visits in this test host.
            fn visit(&self, _urlOrParams: operit_plugin_sdk::js_sdk::network::NetHostVisitUrlOrParams) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::VisitWebResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.visit is not part of this test")
            }

            /// Rejects browser navigation in this test host.
            fn browserNavigate(&self, _urlOrOptions: operit_plugin_sdk::js_sdk::network::NetHostBrowserNavigateUrlOrOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserNavigate is not part of this test")
            }

            /// Rejects browser back navigation in this test host.
            fn browserNavigateBack(&self, _options: Option<std::collections::BTreeMap<String, operit_plugin_sdk::js_sdk::JsNever>>) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserNavigateBack is not part of this test")
            }

            /// Rejects browser clicks in this test host.
            fn browserClick(&self, _options: operit_plugin_sdk::js_sdk::network::NetHostBrowserClickOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserClick is not part of this test")
            }

            /// Rejects browser close in this test host.
            fn browserClose(&self, _options: Option<std::collections::BTreeMap<String, operit_plugin_sdk::js_sdk::JsNever>>) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserClose is not part of this test")
            }

            /// Rejects closing every browser tab in this test host.
            fn browserCloseAll(&self, _options: Option<std::collections::BTreeMap<String, operit_plugin_sdk::js_sdk::JsNever>>) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserCloseAll is not part of this test")
            }

            /// Rejects browser console reads in this test host.
            fn browserConsoleMessages(&self, _options: Option<operit_plugin_sdk::js_sdk::network::NetHostBrowserConsoleMessagesOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserConsoleMessages is not part of this test")
            }

            /// Rejects browser drag operations in this test host.
            fn browserDrag(&self, _options: operit_plugin_sdk::js_sdk::network::NetHostBrowserDragOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserDrag is not part of this test")
            }

            /// Rejects browser evaluation in this test host.
            fn browserEvaluate(&self, _options: operit_plugin_sdk::js_sdk::network::NetHostBrowserEvaluateOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserEvaluate is not part of this test")
            }

            /// Rejects browser file upload in this test host.
            fn browserFileUpload(&self, _options: Option<operit_plugin_sdk::js_sdk::network::NetHostBrowserFileUploadOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserFileUpload is not part of this test")
            }

            /// Rejects browser form filling in this test host.
            fn browserFillForm(&self, _options: operit_plugin_sdk::js_sdk::network::NetHostBrowserFillFormOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserFillForm is not part of this test")
            }

            /// Rejects browser dialog handling in this test host.
            fn browserHandleDialog(&self, _options: operit_plugin_sdk::js_sdk::network::NetHostBrowserHandleDialogOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserHandleDialog is not part of this test")
            }

            /// Rejects browser hover operations in this test host.
            fn browserHover(&self, _options: operit_plugin_sdk::js_sdk::network::NetHostBrowserHoverOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserHover is not part of this test")
            }

            /// Rejects browser network request reads in this test host.
            fn browserNetworkRequests(&self, _options: Option<operit_plugin_sdk::js_sdk::network::NetHostBrowserNetworkRequestsOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserNetworkRequests is not part of this test")
            }

            /// Rejects browser key presses in this test host.
            fn browserPressKey(&self, _keyOrOptions: operit_plugin_sdk::js_sdk::network::NetHostBrowserPressKeyKeyOrOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserPressKey is not part of this test")
            }

            /// Rejects browser resizing in this test host.
            fn browserResize(&self, _options: operit_plugin_sdk::js_sdk::network::NetHostBrowserResizeOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserResize is not part of this test")
            }

            /// Rejects browser code execution in this test host.
            fn browserRunCode(&self, _options: operit_plugin_sdk::js_sdk::network::NetHostBrowserRunCodeOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserRunCode is not part of this test")
            }

            /// Rejects browser select operations in this test host.
            fn browserSelectOption(&self, _options: operit_plugin_sdk::js_sdk::network::NetHostBrowserSelectOptionOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserSelectOption is not part of this test")
            }

            /// Rejects browser snapshots in this test host.
            fn browserSnapshot(&self, _options: Option<operit_plugin_sdk::js_sdk::network::NetHostBrowserSnapshotOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserSnapshot is not part of this test")
            }

            /// Rejects browser screenshots in this test host.
            fn browserTakeScreenshot(&self, _options: operit_plugin_sdk::js_sdk::network::NetHostBrowserTakeScreenshotOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserTakeScreenshot is not part of this test")
            }

            /// Rejects browser tab management in this test host.
            fn browserTabs(&self, _options: operit_plugin_sdk::js_sdk::network::NetHostBrowserTabsOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserTabs is not part of this test")
            }

            /// Rejects browser typing in this test host.
            fn browserType(&self, _options: operit_plugin_sdk::js_sdk::network::NetHostBrowserTypeOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserType is not part of this test")
            }

            /// Rejects browser waiting in this test host.
            fn browserWaitFor(&self, _options: operit_plugin_sdk::js_sdk::network::NetHostBrowserWaitForOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.browserWaitFor is not part of this test")
            }

            /// Rejects HTTP requests in this test host.
            fn http(&self, _options: operit_plugin_sdk::js_sdk::network::NetHostHttpOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::HttpResponseData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.http is not part of this test")
            }

            /// Rejects file upload requests in this test host.
            fn uploadFile(&self, _options: operit_plugin_sdk::js_sdk::network::NetHostUploadFileOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::HttpResponseData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Net.uploadFile is not part of this test")
            }
        }

        #[allow(non_snake_case)]
        impl operit_plugin_sdk::js_sdk::network::NetCookieManager for $host {
            /// Rejects cookie reads in this test host.
            fn get(&self, _domain: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::HttpResponseData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Cookies.get is not part of this test")
            }

            /// Rejects cookie writes in this test host.
            fn set(&self, _domain: String, _cookies: operit_plugin_sdk::js_sdk::network::NetCookieManagerSetCookies) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::HttpResponseData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Cookies.set is not part of this test")
            }

            /// Rejects cookie clearing in this test host.
            fn clear(&self, _domain: Option<String>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::HttpResponseData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Cookies.clear is not part of this test")
            }
        }

        #[allow(non_snake_case)]
        impl operit_plugin_sdk::js_sdk::system::SystemHost for $host {
            /// Rejects sleep requests in this test host.
            fn sleep(&self, _milliseconds: operit_plugin_sdk::js_sdk::system::SystemHostSleepMilliseconds) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::SleepResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("System.sleep is not part of this test")
            }

            /// Rejects setting reads in this test host.
            fn getSetting(&self, _setting: String, _namespace: Option<String>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::SystemSettingData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("System.getSetting is not part of this test")
            }

            /// Rejects setting writes in this test host.
            fn setSetting(&self, _setting: String, _value: String, _namespace: Option<String>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::SystemSettingData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("System.setSetting is not part of this test")
            }

            /// Rejects device information reads in this test host.
            fn getDeviceInfo(&self) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::DeviceInfoResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("System.getDeviceInfo is not part of this test")
            }

            /// Rejects toast requests in this test host.
            fn toast(&self, _message: String) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("System.toast is not part of this test")
            }

            /// Rejects notification requests in this test host.
            fn sendNotification(&self, _message: String, _title: Option<String>) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("System.sendNotification is not part of this test")
            }

            /// Rejects package activation in this test host.
            fn usePackage(&self, _packageName: String) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("System.usePackage is not part of this test")
            }

            /// Rejects app installation in this test host.
            fn installApp(&self, _path: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::AppOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("System.installApp is not part of this test")
            }

            /// Rejects app uninstallation in this test host.
            fn uninstallApp(&self, _packageName: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::AppOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("System.uninstallApp is not part of this test")
            }

            /// Rejects app stopping in this test host.
            fn stopApp(&self, _packageName: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::AppOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("System.stopApp is not part of this test")
            }

            /// Rejects app listing in this test host.
            fn listApps(&self, _includeSystem: Option<bool>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::AppListData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("System.listApps is not part of this test")
            }

            /// Rejects app starting in this test host.
            fn startApp(&self, _packageName: String, _activity: Option<String>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::AppOperationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("System.startApp is not part of this test")
            }

            /// Rejects notification reads in this test host.
            fn getNotifications(&self, _limit: Option<f64>, _includeOngoing: Option<bool>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::NotificationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("System.getNotifications is not part of this test")
            }

            /// Rejects app usage reads in this test host.
            fn getAppUsageTime(&self, _options: Option<operit_plugin_sdk::js_sdk::system::SystemHostGetAppUsageTimeOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::AppUsageTimeResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("System.getAppUsageTime is not part of this test")
            }

            /// Rejects location reads in this test host.
            fn getLocation(&self, _highAccuracy: Option<bool>, _timeout: Option<f64>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::LocationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("System.getLocation is not part of this test")
            }
        }

        #[allow(non_snake_case)]
        impl operit_plugin_sdk::js_sdk::system::SystemBluetoothHost for $host {
            /// Rejects Bluetooth permission requests in this test host.
            fn requestPermission(&self) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.requestPermission is not part of this test")
            }

            /// Rejects Bluetooth state reads in this test host.
            fn getState(&self) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothStateData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.getState is not part of this test")
            }

            /// Rejects Bluetooth enable requests in this test host.
            fn requestEnable(&self) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.requestEnable is not part of this test")
            }

            /// Rejects Bluetooth bonded-device listing in this test host.
            fn listBondedDevices(&self) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothBondedDevicesData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.listBondedDevices is not part of this test")
            }

            /// Rejects Bluetooth scans in this test host.
            fn scan(&self, _options: Option<operit_plugin_sdk::js_sdk::system::SystemBluetoothHostScanOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothScanResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.scan is not part of this test")
            }

            /// Rejects Bluetooth connections in this test host.
            fn connect(&self, _options: operit_plugin_sdk::js_sdk::system::SystemBluetoothHostConnectOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothSessionData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.connect is not part of this test")
            }

            /// Rejects Bluetooth listening in this test host.
            fn listen(&self, _options: Option<operit_plugin_sdk::js_sdk::system::SystemBluetoothHostListenOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothSessionData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.listen is not part of this test")
            }

            /// Rejects Bluetooth accept operations in this test host.
            fn accept(&self, _listenerSessionId: String, _timeoutMs: Option<operit_plugin_sdk::js_sdk::system::SystemBluetoothHostAcceptTimeoutMs>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothSessionData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.accept is not part of this test")
            }

            /// Rejects Bluetooth sends in this test host.
            fn send(&self, _sessionId: String, _options: operit_plugin_sdk::js_sdk::system::SystemBluetoothHostSendOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothTransferData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.send is not part of this test")
            }

            /// Rejects Bluetooth reads in this test host.
            fn read(&self, _sessionId: String, _options: Option<operit_plugin_sdk::js_sdk::system::SystemBluetoothHostReadOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothReadData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.read is not part of this test")
            }

            /// Rejects Bluetooth send-and-read operations in this test host.
            fn sendAndRead(&self, _sessionId: String, _options: operit_plugin_sdk::js_sdk::system::SystemBluetoothHostSendAndReadOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothReadData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.sendAndRead is not part of this test")
            }

            /// Rejects Bluetooth session closing in this test host.
            fn close(&self, _sessionId: String) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.close is not part of this test")
            }
        }

        #[allow(non_snake_case)]
        impl operit_plugin_sdk::js_sdk::system::SystemBluetoothBleHost for $host {
            /// Rejects BLE connections in this test host.
            fn connect(&self, _options: operit_plugin_sdk::js_sdk::system::SystemBluetoothBleHostConnectOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothSessionData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.BLE.connect is not part of this test")
            }

            /// Rejects BLE service discovery in this test host.
            fn discoverServices(&self, _sessionId: String, _timeoutMs: Option<operit_plugin_sdk::js_sdk::system::SystemBluetoothBleHostDiscoverServicesTimeoutMs>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothBleServicesData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.BLE.discoverServices is not part of this test")
            }

            /// Rejects BLE characteristic reads in this test host.
            fn readCharacteristic(&self, _sessionId: String, _options: operit_plugin_sdk::js_sdk::system::SystemBluetoothBleHostReadCharacteristicOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothReadData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.BLE.readCharacteristic is not part of this test")
            }

            /// Rejects BLE characteristic writes in this test host.
            fn writeCharacteristic(&self, _sessionId: String, _options: operit_plugin_sdk::js_sdk::system::SystemBluetoothBleHostWriteCharacteristicOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothTransferData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.BLE.writeCharacteristic is not part of this test")
            }

            /// Rejects BLE write-and-read operations in this test host.
            fn writeAndReadCharacteristic(&self, _sessionId: String, _options: operit_plugin_sdk::js_sdk::system::SystemBluetoothBleHostWriteAndReadCharacteristicOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothReadData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.BLE.writeAndReadCharacteristic is not part of this test")
            }

            /// Rejects BLE subscriptions in this test host.
            fn subscribe(&self, _sessionId: String, _options: operit_plugin_sdk::js_sdk::system::SystemBluetoothBleHostSubscribeOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothTransferData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.BLE.subscribe is not part of this test")
            }

            /// Rejects BLE notification reads in this test host.
            fn readNotifications(&self, _sessionId: String, _limit: Option<operit_plugin_sdk::js_sdk::system::SystemBluetoothBleHostReadNotificationsLimit>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::BluetoothBleNotificationData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Bluetooth.BLE.readNotifications is not part of this test")
            }
        }

        #[allow(non_snake_case)]
        impl operit_plugin_sdk::js_sdk::system::SystemTerminalHost for $host {
            /// Rejects terminal info reads in this test host.
            fn info(&self) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::TerminalInfoResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Terminal.info is not part of this test")
            }

            /// Rejects terminal creation in this test host.
            fn create(&self, _sessionName: String, _type: operit_plugin_sdk::js_sdk::results::TerminalCreateType) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::TerminalSessionCreationResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Terminal.create is not part of this test")
            }

            /// Rejects terminal command execution in this test host.
            fn exec(&self, _sessionId: String, _command: String, _timeoutMs: Option<operit_plugin_sdk::js_sdk::system::SystemTerminalHostExecTimeoutMs>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::TerminalCommandResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Terminal.exec is not part of this test")
            }

            /// Rejects terminal streaming command execution in this test host.
            fn execStreaming(&self, _sessionId: String, _command: String, _options: Option<operit_plugin_sdk::js_sdk::system::SystemTerminalHostExecStreamingOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::TerminalCommandResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Terminal.execStreaming is not part of this test")
            }

            /// Rejects hidden terminal command execution in this test host.
            fn hiddenExec(&self, _command: String, _options: operit_plugin_sdk::js_sdk::system::SystemTerminalHostHiddenExecOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::HiddenTerminalCommandResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Terminal.hiddenExec is not part of this test")
            }

            /// Rejects terminal closing in this test host.
            fn close(&self, _sessionId: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::TerminalSessionCloseResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Terminal.close is not part of this test")
            }

            /// Rejects terminal screen reads in this test host.
            fn screen(&self, _sessionId: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::TerminalSessionScreenResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Terminal.screen is not part of this test")
            }

            /// Rejects terminal input in this test host.
            fn input(&self, _sessionId: String, _options: Option<operit_plugin_sdk::js_sdk::system::SystemTerminalHostInputOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Terminal.input is not part of this test")
            }
        }

        #[allow(non_snake_case)]
        impl operit_plugin_sdk::js_sdk::system::SystemMusicHost for $host {
            /// Rejects music playback in this test host.
            fn play(&self, _options: operit_plugin_sdk::js_sdk::system::SystemMusicHostPlayOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MusicPlaybackResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Music.play is not part of this test")
            }

            /// Rejects music pause in this test host.
            fn pause(&self) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MusicPlaybackResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Music.pause is not part of this test")
            }

            /// Rejects music resume in this test host.
            fn resume(&self) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MusicPlaybackResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Music.resume is not part of this test")
            }

            /// Rejects music stop in this test host.
            fn stop(&self) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MusicPlaybackResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Music.stop is not part of this test")
            }

            /// Rejects music seek in this test host.
            fn seek(&self, _positionMs: operit_plugin_sdk::js_sdk::system::SystemMusicHostSeekPositionMs) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MusicPlaybackResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Music.seek is not part of this test")
            }

            /// Rejects music volume changes in this test host.
            fn setVolume(&self, _volume: operit_plugin_sdk::js_sdk::system::SystemMusicHostSetVolumeVolume) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MusicPlaybackResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Music.setVolume is not part of this test")
            }

            /// Rejects music status reads in this test host.
            fn status(&self) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MusicPlaybackResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Music.status is not part of this test")
            }
        }

        #[allow(non_snake_case)]
        impl operit_plugin_sdk::js_sdk::software_settings::SoftwareSettingsHost for $host {
            /// Rejects environment reads in this test host.
            fn readEnvironmentVariable(&self, _key: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::EnvironmentVariableReadResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("SoftwareSettings.readEnvironmentVariable is not part of this test")
            }

            /// Rejects environment writes in this test host.
            fn writeEnvironmentVariable(&self, _key: String, _value: Option<String>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::EnvironmentVariableWriteResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("SoftwareSettings.writeEnvironmentVariable is not part of this test")
            }

            /// Rejects command execution in this test host.
            fn exec(&self, _args: Vec<String>) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("SoftwareSettings.exec is not part of this test")
            }
        }

        #[allow(non_snake_case)]
        impl operit_plugin_sdk::js_sdk::chat::ChatHost for $host {
            /// Rejects chat service start in this test host.
            fn startService(&self, _options: Option<operit_plugin_sdk::js_sdk::chat::ChatStartServiceOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::ChatServiceStartResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Chat.startService is not part of this test")
            }

            /// Rejects chat service stop in this test host.
            fn stopService(&self) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::ChatServiceStartResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Chat.stopService is not part of this test")
            }

            /// Rejects chat creation in this test host.
            fn createNew(&self, _group: Option<String>, _setAsCurrentChat: Option<bool>, _characterCardId: Option<String>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::ChatCreationResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Chat.createNew is not part of this test")
            }

            /// Rejects full chat listing in this test host.
            fn listAll(&self) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::ChatListResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Chat.listAll is not part of this test")
            }

            /// Rejects filtered chat listing in this test host.
            fn listChats(&self, _params: Option<operit_plugin_sdk::js_sdk::chat::ChatHostListChatsParams>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::ChatListResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Chat.listChats is not part of this test")
            }

            /// Rejects chat finding in this test host.
            fn findChat(&self, _params: operit_plugin_sdk::js_sdk::chat::ChatHostFindChatParams) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::ChatFindResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Chat.findChat is not part of this test")
            }

            /// Rejects agent status reads in this test host.
            fn agentStatus(&self, _chatId: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::AgentStatusResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Chat.agentStatus is not part of this test")
            }

            /// Rejects chat switching in this test host.
            fn switchTo(&self, _chatId: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::ChatSwitchResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Chat.switchTo is not part of this test")
            }

            /// Rejects title updates in this test host.
            fn updateTitle(&self, _chatId: String, _title: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::ChatTitleUpdateResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Chat.updateTitle is not part of this test")
            }

            /// Rejects chat deletion in this test host.
            fn deleteChat(&self, _chatId: String) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::ChatDeleteResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Chat.deleteChat is not part of this test")
            }

            /// Rejects chat messages in this test host.
            fn sendMessage(&self, _message: String, _chatId: Option<String>, _roleCardId: Option<String>, _senderName: Option<String>, _options: Option<operit_plugin_sdk::js_sdk::chat::ChatSendMessageOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MessageSendResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Chat.sendMessage is not part of this test")
            }

            /// Rejects streaming chat messages in this test host.
            fn sendMessageStreaming(&self, _message: String, _chatId: Option<String>, _roleCardId: Option<String>, _senderName: Option<String>, _options: Option<operit_plugin_sdk::js_sdk::chat::ChatSendMessageStreamingOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MessageSendResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Chat.sendMessageStreaming is not part of this test")
            }

            /// Rejects character card listing in this test host.
            fn listCharacterCards(&self) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::CharacterCardListResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Chat.listCharacterCards is not part of this test")
            }

            /// Rejects message history reads in this test host.
            fn getMessages(&self, _chatId: String, _options: Option<operit_plugin_sdk::js_sdk::chat::ChatHostGetMessagesOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::ChatMessagesResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Chat.getMessages is not part of this test")
            }
        }

        #[allow(non_snake_case)]
        impl operit_plugin_sdk::js_sdk::memory::MemoryHost for $host {
            /// Rejects memory queries in this test host.
            fn query_overload_1(&self, _query: String, _folderPath: Option<String>, _limit: Option<f64>, _startTime: Option<String>, _endTime: Option<String>, _snapshotId: Option<String>, _threshold: Option<f64>, _targetOwnerKey: Option<String>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MemoryQueryResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.query is not part of this test")
            }

            /// Rejects structured memory queries in this test host.
            fn query_overload_2(&self, _options: operit_plugin_sdk::js_sdk::memory::MemoryQueryOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MemoryQueryResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.query is not part of this test")
            }

            /// Rejects title-based memory reads in this test host.
            fn getByTitle_overload_1(&self, _title: String, _targetOwnerKey: String, _chunkIndex: Option<f64>, _chunkRange: Option<String>, _query: Option<String>, _limit: Option<f64>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MemoryQueryResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.getByTitle is not part of this test")
            }

            /// Rejects structured title-based memory reads in this test host.
            fn getByTitle_overload_2(&self, _options: operit_plugin_sdk::js_sdk::memory::MemoryGetByTitleOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MemoryQueryResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.getByTitle is not part of this test")
            }

            /// Rejects memory creation in this test host.
            fn create_overload_1(&self, _title: String, _content: String, _targetOwnerKey: String, _contentType: Option<String>, _source: Option<String>, _folderPath: Option<String>, _tags: Option<String>) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.create is not part of this test")
            }

            /// Rejects structured memory creation in this test host.
            fn create_overload_2(&self, _options: operit_plugin_sdk::js_sdk::memory::MemoryCreateOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.create is not part of this test")
            }

            /// Rejects memory updates in this test host.
            fn update_overload_1(&self, _oldTitle: String, _targetOwnerKey: String, _updates: Option<operit_plugin_sdk::js_sdk::memory::MemoryUpdateOptions>) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.update is not part of this test")
            }

            /// Rejects structured memory updates in this test host.
            fn update_overload_2(&self, _options: operit_plugin_sdk::js_sdk::memory::MemoryHostUpdateOptionsIntersection) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.update is not part of this test")
            }

            /// Rejects user preference updates in this test host.
            fn updateUserPreferences_overload_1(&self, _content: String, _targetOwnerKey: String) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.updateUserPreferences is not part of this test")
            }

            /// Rejects structured user preference updates in this test host.
            fn updateUserPreferences_overload_2(&self, _options: operit_plugin_sdk::js_sdk::memory::MemoryUserPreferencesOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.updateUserPreferences is not part of this test")
            }

            /// Rejects memory deletion in this test host.
            fn deleteMemory_overload_1(&self, _title: String, _targetOwnerKey: String) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.deleteMemory is not part of this test")
            }

            /// Rejects structured memory deletion in this test host.
            fn deleteMemory_overload_2(&self, _options: operit_plugin_sdk::js_sdk::memory::MemoryDeleteOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.deleteMemory is not part of this test")
            }

            /// Rejects memory moves in this test host.
            fn move_overload_1(&self, _targetFolderPath: String, _targetOwnerKey: String, _titles: Option<operit_plugin_sdk::js_sdk::memory::MemoryHostMoveTitles>, _sourceFolderPath: Option<String>) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.move is not part of this test")
            }

            /// Rejects structured memory moves in this test host.
            fn move_overload_2(&self, _options: operit_plugin_sdk::js_sdk::memory::MemoryMoveOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.move is not part of this test")
            }

            /// Rejects memory linking in this test host.
            fn link_overload_1(&self, _sourceTitle: String, _targetTitle: String, _targetOwnerKey: String, _linkType: Option<String>, _weight: Option<f64>, _description: Option<String>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MemoryLinkResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.link is not part of this test")
            }

            /// Rejects structured memory linking in this test host.
            fn link_overload_2(&self, _options: operit_plugin_sdk::js_sdk::memory::MemoryLinkOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MemoryLinkResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.link is not part of this test")
            }

            /// Rejects memory link queries in this test host.
            fn queryLinks_overload_1(&self, _targetOwnerKey: String, _linkId: Option<f64>, _sourceTitle: Option<String>, _targetTitle: Option<String>, _linkType: Option<String>, _limit: Option<f64>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MemoryLinkQueryResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.queryLinks is not part of this test")
            }

            /// Rejects structured memory link queries in this test host.
            fn queryLinks_overload_2(&self, _options: operit_plugin_sdk::js_sdk::memory::MemoryQueryLinksOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MemoryLinkQueryResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.queryLinks is not part of this test")
            }

            /// Rejects memory link updates in this test host.
            fn updateLink_overload_1(&self, _targetOwnerKey: String, _linkId: Option<f64>, _sourceTitle: Option<String>, _targetTitle: Option<String>, _linkType: Option<String>, _newLinkType: Option<String>, _weight: Option<f64>, _description: Option<String>) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MemoryLinkQueryResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.updateLink is not part of this test")
            }

            /// Rejects structured memory link updates in this test host.
            fn updateLink_overload_2(&self, _options: operit_plugin_sdk::js_sdk::memory::MemoryUpdateLinkOptions) -> operit_plugin_sdk::js_sdk::JsFuture<operit_plugin_sdk::js_sdk::results::MemoryLinkQueryResultData> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.updateLink is not part of this test")
            }

            /// Rejects memory link deletion in this test host.
            fn deleteLink_overload_1(&self, _targetOwnerKey: String, _linkId: Option<f64>, _sourceTitle: Option<String>, _targetTitle: Option<String>, _linkType: Option<String>) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.deleteLink is not part of this test")
            }

            /// Rejects structured memory link deletion in this test host.
            fn deleteLink_overload_2(&self, _options: operit_plugin_sdk::js_sdk::memory::MemoryDeleteLinkOptions) -> operit_plugin_sdk::js_sdk::JsFuture<String> {
                $crate::javascript::TestJsToolsHost::rejecting_js_future("Memory.deleteLink is not part of this test")
            }
        }
    };
}
