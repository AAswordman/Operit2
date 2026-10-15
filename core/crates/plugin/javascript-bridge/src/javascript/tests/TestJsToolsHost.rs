use operit_host_api::{HostError, HostResult, RuntimeStorageEntry, RuntimeStorageHost};
use operit_plugin_sdk::execution_result::JsExecutionResult;
use operit_plugin_sdk::js_sdk::{JsFuture, JsHostError};
use operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost;
use operit_util::RuntimeStorageLayout::{RUNTIME_ROOT_DIR_PATH, WORKSPACE_DIR_PATH};
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

    /// Appends bytes into the test runtime root.
    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        std::io::Write::write_all(&mut file, content)?;
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
    let runtime_root = root.join(RUNTIME_ROOT_DIR_PATH);
    let workspace_root = root.join(WORKSPACE_DIR_PATH);
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
pub fn expect_js_output<T>(output: JsExecutionResult<Option<T>>, context: &str) -> T {
    output
        .expect(context)
        .expect("JavaScript execution should return a value")
}

