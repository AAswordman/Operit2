//! User-editable policies for app-managed workspace and conversation files.
#![allow(non_snake_case)]
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout::{
    DEFAULT_AUTO_SYNC_FILE_BYTES, DEFAULT_AUTO_SYNC_TOTAL_BYTES,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

pub const POLICY_PATH: &str = ".operit/sync.json";
pub const REFERENCE_SUFFIX: &str = ".operit-ref.json";
pub const MAX_POLICY_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct FileSyncPolicy {
    pub version: u32,
    pub ignoreFiles: Vec<String>,
    pub exclude: Vec<String>,
    pub maxFileBytes: u64,
    pub maxTotalBytes: u64,
}
impl Default for FileSyncPolicy {
    fn default() -> Self {
        Self {
            version: 1,
            ignoreFiles: vec![".gitignore".into()],
            exclude: [
                ".git/",
                "node_modules/",
                "target/",
                "build/",
                "dist/",
                ".dart_tool/",
                ".gradle/",
                "__pycache__/",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            maxFileBytes: DEFAULT_AUTO_SYNC_FILE_BYTES,
            maxTotalBytes: DEFAULT_AUTO_SYNC_TOTAL_BYTES,
        }
    }
}
impl FileSyncPolicy {
    pub fn load(host: &dyn RuntimeStorageHost, root: &str) -> Result<Self, String> {
        let path = format!("{root}/{POLICY_PATH}");
        if !host.exists(&path).map_err(|e| e.to_string())? {
            return Ok(Self::default());
        }
        let bytes = readLimited(host, &path, MAX_POLICY_BYTES)?;
        let policy: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("Invalid {path}: {e}"))?;
        if policy.version != 1 {
            return Err(format!("Unsupported sync policy version in {path}"));
        }
        for name in &policy.ignoreFiles {
            validateRelative(name)?;
        }
        Ok(policy)
    }
    /// Seeds a visible, editable policy without replacing the user's configuration.
    pub fn ensure(host: &dyn RuntimeStorageHost, root: &str) -> Result<(), String> {
        let path = format!("{root}/{POLICY_PATH}");
        if !host.exists(&path).map_err(|e| e.to_string())? {
            host.writeBytesAtomically(
                &path,
                &serde_json::to_vec_pretty(&Self::default()).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn rules(
        &self,
        host: &dyn RuntimeStorageHost,
        root: &str,
    ) -> Result<Vec<Gitignore>, String> {
        let mut builder = GitignoreBuilder::new(root);
        for pattern in &self.exclude {
            builder
                .add_line(None, pattern)
                .map_err(|e| format!("Invalid sync exclude: {e}"))?;
        }
        let mut rules = vec![builder.build().map_err(|e| e.to_string())?];
        for name in &self.ignoreFiles {
            let path = format!("{root}/{name}");
            if let Some(rule) = readIgnore(host, &path)? {
                rules.push(rule);
            }
        }
        Ok(rules)
    }
    pub fn includes(
        &self,
        host: &dyn RuntimeStorageHost,
        root: &str,
        path: &str,
        isDirectory: bool,
    ) -> Result<bool, String> {
        if isControl(root, path) {
            return Ok(true);
        }
        let mut rules = self.rules(host, root)?;
        if self.ignoreFiles.iter().any(|p| p == ".gitignore") {
            let relative = path
                .strip_prefix(&format!("{root}/"))
                .ok_or("Sync path escaped its root")?;
            let parts: Vec<_> = relative.split('/').collect();
            let mut parent = root.to_string();
            for part in parts.iter().take(parts.len().saturating_sub(1)) {
                parent.push('/');
                parent.push_str(part);
                if let Some(rule) = readIgnore(host, &format!("{parent}/.gitignore"))? {
                    rules.push(rule);
                }
            }
        }
        Ok(!ignored(&rules, path, isDirectory))
    }
}
/// Every read is bounded, including races with a writer growing a policy or input file.
pub fn readLimited(
    host: &dyn RuntimeStorageHost,
    path: &str,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    loop {
        let length = limit
            .saturating_sub(bytes.len())
            .saturating_add(1)
            .min(64 * 1024);
        let chunk = host
            .readBytesRange(path, bytes.len() as u64, length)
            .map_err(|e| e.to_string())?;
        if chunk.len() > length || bytes.len().saturating_add(chunk.len()) > limit {
            return Err(format!("{path} exceeds the allowed {limit} bytes"));
        }
        if chunk.is_empty() {
            return Ok(bytes);
        }
        bytes.extend_from_slice(&chunk);
    }
}
pub fn validateRelative(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains(['\\', ':', '\0'])
        || path
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err(format!("Expected a relative sync path: {path}"));
    }
    Ok(())
}
pub fn readIgnore(host: &dyn RuntimeStorageHost, path: &str) -> Result<Option<Gitignore>, String> {
    if !host.exists(path).map_err(|e| e.to_string())? {
        return Ok(None);
    }
    let bytes = readLimited(host, path, MAX_POLICY_BYTES)?;
    let text =
        std::str::from_utf8(&bytes).map_err(|e| format!("Invalid ignore file {path}: {e}"))?;
    let parent = path.rsplit_once('/').ok_or("Invalid ignore path")?.0;
    let mut builder = GitignoreBuilder::new(parent);
    for line in text.lines() {
        builder
            .add_line(Some(PathBuf::from(path)), line)
            .map_err(|e| format!("Invalid {path}: {e}"))?;
    }
    Ok(Some(builder.build().map_err(|e| e.to_string())?))
}
pub fn ignored(rules: &[Gitignore], path: &str, directory: bool) -> bool {
    let mut ignored = false;
    for rule in rules {
        let matched = rule.matched_path_or_any_parents(path, directory);
        if matched.is_ignore() {
            ignored = true;
        } else if matched.is_whitelist() {
            ignored = false;
        }
    }
    ignored
}
pub fn isControl(root: &str, path: &str) -> bool {
    path == format!("{root}/{POLICY_PATH}") || path == format!("{root}/.operit")
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileReference {
    pub format: String,
    pub version: u32,
    pub nodeId: String,
    pub path: String,
    pub fileName: String,
    pub fileSize: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifiedAt: Option<i64>,
}
impl FileReference {
    pub fn new(
        host: Arc<dyn RuntimeStorageHost>,
        path: String,
        fileName: String,
        fileSize: i64,
        modifiedAt: Option<i64>,
    ) -> Result<Self, String> {
        let nodeId = crate::CoreNodeIdentityStore::CoreNodeIdentityStore::new(host)
            .identity()?
            .nodeId;
        Ok(Self {
            format: "operit_file_reference".into(),
            version: 1,
            nodeId,
            path,
            fileName,
            fileSize,
            modifiedAt,
        })
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.format != "operit_file_reference"
            || self.version != 1
            || self.nodeId.trim().is_empty()
            || self.fileSize < 0
            || !["/app/", "/operitapp/", "/mnt/", "/virtual/"]
                .iter()
                .any(|root| self.path.starts_with(root))
            || self.path.contains(['\\', '\0'])
            || self.path.split('/').any(|p| p == "." || p == "..")
            || self.fileName.trim().is_empty()
            || self.fileName.contains(['/', '\\', '\0'])
        {
            return Err("Invalid durable file reference".into());
        }
        Ok(())
    }
    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec_pretty(self).map_err(|e| e.to_string())
    }
}
