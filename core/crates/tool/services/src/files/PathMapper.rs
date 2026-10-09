use std::path::{Path, PathBuf};

use operit_host_api::FileEntry;
use operit_host_api::FileSystemResource::FileSystemResource;
use super::MountRegistry::MountRegistry;
use operit_util::RuntimeStorageLayout::{
    EXTENSIONS_PLUGIN_CONFIGS_DIR_PATH, EXTENSIONS_PLUGIN_DATA_DIR_PATH,
    RUNTIME_ROOT_PATH_PREFIX, WORKSPACE_DIR_PATH,
};

const ROOT_APP: &str = "app";
const ROOT_MNT: &str = "mnt";
const ROOT_SDCARD: &str = "sdcard";
const ROOT_DATA: &str = "data";

const APP_DATA: &str = "data";
const APP_WORKSPACES: &str = WORKSPACE_DIR_PATH;

const MNT_WINDOWS: &str = "windows";
const MNT_ANDROID: &str = "android";
const MNT_LINUX: &str = "linux";
const MNT_MACOS: &str = "macos";
const MNT_ANDROID_SDCARD: &str = "sdcard";
const MNT_ANDROID_ROOT: &str = "root";

/// Resolved mapping from a public VFS path to a host filesystem locator.
/// `physicalPath` retains its legacy name; resource-backed targets are opaque.
/// Consumers requiring a real OS path must call `nativePath()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedVfsPath {
    pub vfsPath: String,
    pub physicalPath: String,
}

impl ResolvedVfsPath {
    /// Returns a real OS path only for backends that provide one (terminal/OCR).
    pub fn nativePath(&self) -> Result<String, String> {
        if FileSystemResource::parse(&self.physicalPath).map_err(|e| e.to_string())?.is_some() {
            return Err(format!("{} is a document/resource mount, not a native working directory", self.vfsPath));
        }
        Ok(self.physicalPath.clone())
    }
}

/// Normalizes VFS paths and maps them onto host-visible storage roots.
#[derive(Debug, Clone)]
pub struct PathMapper {
    runtimeStoreRoot: PathBuf,
    workspaceCollectionRoot: PathBuf,
    mountRegistry: MountRegistry,
}

impl PathMapper {
    /// Creates a built-in-only mapper. Runtime consumers must bind their storage Host.
    pub fn new(runtimeStoreRoot: PathBuf, workspaceCollectionRoot: PathBuf) -> Self {
        let mountRegistry = MountRegistry::withoutStorage(&runtimeStoreRoot);
        Self { runtimeStoreRoot, workspaceCollectionRoot, mountRegistry }
    }

    /// Pins mount resolution to an explicitly supplied identity-local catalog.
    pub fn withMountRegistry(mut self, registry: MountRegistry) -> Self {
        self.mountRegistry = registry;
        self
    }

    /// Uses the same storage capability as the owning runtime, not a mutable default.
    pub fn withMountStorage(self, storage: std::sync::Arc<dyn operit_host_api::RuntimeStorageHost>) -> Self {
        self.withMountRegistry(MountRegistry::withStorage(storage))
    }

    fn mounts(&self) -> Result<Vec<super::MountRegistry::VfsMount>, String> {
        self.mountRegistry.list()
    }

    /// Returns the VFS root containing all runtime workspaces.
    #[allow(non_snake_case)]
    pub fn workspaceCollectionPath() -> &'static str {
        "/app/workspaces"
    }

    /// Builds a VFS path for a workspace id under the workspace collection root.
    #[allow(non_snake_case)]
    pub fn workspacePath(workspaceId: &str) -> Result<String, String> {
        let workspaceId = normalizeSingleSegment(workspaceId, "workspace id")?;
        Ok(format!(
            "{}/{}",
            Self::workspaceCollectionPath(),
            workspaceId
        ))
    }

    /// Normalizes an absolute VFS path.
    #[allow(non_snake_case)]
    pub fn normalizeVfsPath(path: &str) -> Result<String, String> {
        normalizeAbsoluteVfsPath(path)
    }

    /// Converts legacy shared-storage aliases into identity-owned plugin storage paths.
    #[allow(non_snake_case)]
    pub fn canonicalizeVfsPath(path: &str) -> Result<String, String> {
        let normalized = normalizeAbsoluteVfsPath(path)?;
        let segments = pathSegments(&normalized);
        let rest = match segments.as_slice() {
            [ROOT_SDCARD, rest @ ..] | ["storage", "emulated", "0", rest @ ..] => rest,
            _ => return Ok(normalized),
        };
        let (storageRoot, relative) = match rest {
            ["Download", "Operit", "plugins", relative @ ..] =>
                (EXTENSIONS_PLUGIN_CONFIGS_DIR_PATH, relative),
            ["Download", "Operit", relative @ ..] =>
                (EXTENSIONS_PLUGIN_DATA_DIR_PATH, relative),
            relative => (EXTENSIONS_PLUGIN_DATA_DIR_PATH, relative),
        };
        let root = storageRoot.strip_prefix(RUNTIME_ROOT_PATH_PREFIX)
            .expect("plugin storage must belong to runtime");
        let root = Self::joinVfsPath("/app/data", root)?;
        Self::joinVfsPath(&root, &relative.join("/"))
    }

    /// Normalizes a user-selected workspace binding path.
    #[allow(non_snake_case)]
    pub fn normalizeWorkspaceBindingPath(path: &str) -> Result<String, String> {
        let text = path.trim().replace('\\', "/");
        if text.is_empty() {
            return Err("workspace path is required".to_string());
        }
        if text.starts_with('/') {
            let normalizedPath = normalizeAbsoluteVfsPath(&text)?;
            if let Some(vfsPath) = normalizeWorkspaceBindingVfsPath(&normalizedPath)? {
                return Ok(vfsPath);
            }
            return normalizeAbsoluteHostWorkspacePath(&normalizedPath);
        }
        if let Some(vfsPath) = normalizeWindowsHostWorkspacePath(&text)? {
            return Ok(vfsPath);
        }
        Err(format!(
            "Workspace binding must use a VFS path or an absolute host path: {path}"
        ))
    }

    /// Normalizes a path relative to an existing VFS root.
    #[allow(non_snake_case)]
    pub fn normalizeRelativePath(path: &str) -> Result<String, String> {
        normalizeRelativePath(path)
    }

    /// Joins a normalized VFS base path with a relative path.
    #[allow(non_snake_case)]
    pub fn joinVfsPath(base: &str, relativePath: &str) -> Result<String, String> {
        let base = normalizeAbsoluteVfsPath(base)?;
        let relativePath = normalizeRelativePath(relativePath)?;
        if relativePath.is_empty() {
            return Ok(base);
        }
        if base == "/" {
            Ok(format!("/{relativePath}"))
        } else {
            Ok(format!("{}/{}", base.trim_end_matches('/'), relativePath))
        }
    }

    /// Returns the path of a child relative to a normalized VFS root.
    #[allow(non_snake_case)]
    pub fn relativePath(root: &str, fullPath: &str) -> Result<Option<String>, String> {
        let root = normalizeAbsoluteVfsPath(root)?;
        let fullPath = normalizeAbsoluteVfsPath(fullPath)?;
        if fullPath == root {
            return Ok(Some(String::new()));
        }
        let prefix = format!("{}/", root.trim_end_matches('/'));
        if !fullPath.starts_with(&prefix) {
            return Ok(None);
        }
        Ok(Some(fullPath[prefix.len()..].to_string()))
    }

    /// Returns synthetic entries for VFS directories that are not backed by a host directory.
    #[allow(non_snake_case)]
    pub fn virtualDirectoryEntries(&self, path: &str) -> Result<Option<Vec<FileEntry>>, String> {
        let normalizedPath = normalizeAbsoluteVfsPath(path)?;
        let segments = pathSegments(&normalizedPath);
        let mounts = if normalizedPath == "/" || normalizedPath == "/mnt" || normalizedPath.starts_with("/mnt/") {
            self.mounts()?
        } else { Vec::new() };
        let mut entries = match segments.as_slice() {
            [] => {
                let mut entries = vec![directoryEntry(ROOT_APP)];
                if !mntMountEntries().is_empty() || !mounts.is_empty() { entries.push(directoryEntry(ROOT_MNT)); }
                Some(entries)
            }
            [ROOT_APP] => Some(vec![directoryEntry(APP_DATA), directoryEntry(APP_WORKSPACES)]),
            [ROOT_MNT] => { let entries = mntMountEntries(); if entries.is_empty() { None } else { Some(entries) } }
            [ROOT_MNT, MNT_ANDROID] if androidRootMounted() => {
                let mut entries = vec![directoryEntry(MNT_ANDROID_ROOT)];
                if androidSdcardMounted() { entries.push(directoryEntry(MNT_ANDROID_SDCARD)); }
                Some(entries)
            }
            [ROOT_MNT, MNT_WINDOWS] if windowsMounted() => Some(windowsDriveEntries()),
            _ => None,
        };
        let prefix = format!("{}/", normalizedPath.trim_end_matches('/'));
        for mount in mounts {
            let mountPath = mount.vfsPath();
            if let Some(rest) = mountPath.strip_prefix(&prefix) {
                if let Some(name) = rest.split('/').next() {
                    let list = entries.get_or_insert_with(Vec::new);
                    if !list.iter().any(|entry| entry.name == name) { list.push(directoryEntry(name)); }
                }
            }
        }
        Ok(entries)
    }

    /// Resolves a normalized VFS path into its host physical path.
    pub fn resolve(&self, path: &str) -> Result<ResolvedVfsPath, String> {
        let normalizedPath = Self::canonicalizeVfsPath(path)?;
        let segments = pathSegments(&normalizedPath);
        if normalizedPath.starts_with("/mnt/") {
            for mount in self.mounts()? {
                let root = mount.vfsPath();
                let relative = if normalizedPath == root { Some("") } else { normalizedPath.strip_prefix(&format!("{root}/")) };
                if let Some(relative) = relative {
                    let physicalPath = if mount.backend == "native" {
                        physicalPathString(joinPhysical(Path::new(&mount.root), &pathSegments(relative)))
                    } else {
                        FileSystemResource { backend: mount.backend, root: mount.root, path: relative.into() }
                            .encode().map_err(|e| e.to_string())?
                    };
                    return Ok(ResolvedVfsPath { vfsPath: normalizedPath, physicalPath });
                }
            }
        }
        match segments.as_slice() {
            [] => Err("VFS root is a virtual directory".to_string()),
            [ROOT_APP] | [ROOT_MNT] => Err(format!("{normalizedPath} is a virtual directory")),
            [ROOT_APP, APP_DATA, rest @ ..] => Ok(ResolvedVfsPath {
                vfsPath: joinNormalizedSegments(&[ROOT_APP, APP_DATA], rest),
                physicalPath: physicalPathString(joinPhysical(&self.runtimeStoreRoot, rest)),
            }),
            [ROOT_APP, APP_WORKSPACES, rest @ ..] => Ok(ResolvedVfsPath {
                vfsPath: joinNormalizedSegments(&[ROOT_APP, APP_WORKSPACES], rest),
                physicalPath: physicalPathString(joinPhysical(&self.workspaceCollectionRoot, rest)),
            }),
            [ROOT_MNT, MNT_WINDOWS, drive, rest @ ..] => {
                let driveLetter = normalizeDriveLetter(drive)?;
                if !windowsDriveRootExists(&driveLetter) {
                    return Err(format!(
                        "Windows drive is not mounted under /mnt/windows: {driveLetter}"
                    ));
                }
                let mut physical = PathBuf::from(format!("{driveLetter}:/"));
                for segment in rest {
                    physical.push(segment);
                }
                Ok(ResolvedVfsPath {
                    vfsPath: joinNormalizedSegments(&[ROOT_MNT, MNT_WINDOWS, &driveLetter], rest),
                    physicalPath: physicalPathString(physical),
                })
            }
            [ROOT_MNT, MNT_ANDROID, MNT_ANDROID_SDCARD, rest @ ..] => {
                if !androidSdcardMounted() {
                    return Err("/mnt/android/sdcard is not mounted".to_string());
                }
                Ok(ResolvedVfsPath {
                    vfsPath: joinNormalizedSegments(
                        &[ROOT_MNT, MNT_ANDROID, MNT_ANDROID_SDCARD],
                        rest,
                    ),
                    physicalPath: physicalPathString(joinUnixPhysical("/sdcard", rest)),
                })
            }
            [ROOT_MNT, MNT_ANDROID, MNT_ANDROID_ROOT, rest @ ..] => {
                if !androidRootMounted() { return Err("/mnt/android/root is not mounted".into()); }
                Ok(ResolvedVfsPath {
                    vfsPath: joinNormalizedSegments(&[ROOT_MNT, MNT_ANDROID, MNT_ANDROID_ROOT], rest),
                    physicalPath: physicalPathString(joinUnixPhysical("/", rest)),
                })
            }
            [ROOT_MNT, MNT_LINUX, rest @ ..] => {
                if !linuxRootMounted() {
                    return Err("/mnt/linux is not mounted".to_string());
                }
                Ok(ResolvedVfsPath {
                    vfsPath: joinNormalizedSegments(&[ROOT_MNT, MNT_LINUX], rest),
                    physicalPath: physicalPathString(joinUnixPhysical("/", rest)),
                })
            }
            [ROOT_MNT, MNT_MACOS, rest @ ..] => {
                if !macosRootMounted() {
                    return Err("/mnt/macos is not mounted".to_string());
                }
                Ok(ResolvedVfsPath {
                    vfsPath: joinNormalizedSegments(&[ROOT_MNT, MNT_MACOS], rest),
                    physicalPath: physicalPathString(joinUnixPhysical("/", rest)),
                })
            }
            [ROOT_DATA, rest @ ..] => {
                if !androidDataMounted() {
                    return Err("/data is not mounted".to_string());
                }
                Ok(ResolvedVfsPath {
                    vfsPath: joinNormalizedSegments(&[ROOT_DATA], rest),
                    physicalPath: physicalPathString(joinUnixPhysical("/data", rest)),
                })
            }
            _ => Err(format!("Unknown VFS root: {normalizedPath}")),
        }
    }

    /// Maps a host search/listing result back into the VFS tree under a resolved base.
    #[allow(non_snake_case)]
    pub fn mapPhysicalChildToVfs(
        &self,
        base: &ResolvedVfsPath,
        physicalChildPath: &str,
    ) -> Result<String, String> {
        if let Some(baseResource) = FileSystemResource::parse(&base.physicalPath).map_err(|e| e.to_string())? {
            let child = FileSystemResource::parse(physicalChildPath).map_err(|e| e.to_string())?
                .ok_or("Host returned a native path from a resource search")?;
            if child.backend != baseResource.backend || child.root != baseResource.root {
                return Err("Host returned a resource outside the VFS search root".into());
            }
            let relative = if child.path == baseResource.path { "" }
                else if baseResource.path.is_empty() { child.path.as_str() }
                else { child.path.strip_prefix(&format!("{}/", baseResource.path)).ok_or("Host returned a path outside the resource search root")? };
            return Self::joinVfsPath(&base.vfsPath, relative);
        }
        let basePhysical = normalizePhysicalText(&base.physicalPath);
        let childPhysical = normalizePhysicalText(physicalChildPath);
        if childPhysical == basePhysical {
            return Ok(base.vfsPath.clone());
        }
        let prefix = format!("{}/", basePhysical.trim_end_matches('/'));
        if !childPhysical.starts_with(&prefix) {
            return Err(format!(
                "Host returned path outside VFS search root: {physicalChildPath}"
            ));
        }
        let relative = &childPhysical[prefix.len()..];
        Self::joinVfsPath(&base.vfsPath, relative)
    }
}

impl Default for PathMapper {
    /// Creates an empty mapper used by tests and placeholder contexts.
    fn default() -> Self {
        Self::new(PathBuf::new(), PathBuf::new())
    }
}

#[allow(non_snake_case)]
fn normalizeAbsoluteVfsPath(path: &str) -> Result<String, String> {
    let text = path.trim().replace('\\', "/");
    if text.is_empty() {
        return Err("path parameter is required".to_string());
    }
    if !text.starts_with('/') {
        return Err(format!("Invalid VFS path: {path}. Path must start with /."));
    }
    let mut segments = Vec::<String>::new();
    for segment in text.split('/') {
        if segment.is_empty() {
            continue;
        }
        validateSegment(segment, path)?;
        segments.push(segment.to_string());
    }
    if segments.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{}", segments.join("/")))
    }
}

#[allow(non_snake_case)]
fn normalizeRelativePath(path: &str) -> Result<String, String> {
    let text = path.trim().replace('\\', "/");
    let trimmed = text.trim_matches('/');
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let mut segments = Vec::<String>::new();
    for segment in trimmed.split('/') {
        if segment.is_empty() {
            continue;
        }
        validateSegment(segment, path)?;
        segments.push(segment.to_string());
    }
    Ok(segments.join("/"))
}

#[allow(non_snake_case)]
fn normalizeSingleSegment(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{label} is required"));
    }
    validateSegment(value, value)?;
    if value.split('/').count() != 1 || value.chars().any(|character| character == '\\') {
        return Err(format!("invalid {label}: {value}"));
    }
    Ok(value.to_string())
}

#[allow(non_snake_case)]
fn validateSegment(segment: &str, originalPath: &str) -> Result<(), String> {
    if segment == "." || segment == ".." {
        return Err(format!(
            "Invalid VFS path segment in {originalPath}: {segment}"
        ));
    }
    Ok(())
}

#[allow(non_snake_case)]
fn pathSegments(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

/// Maps selected shared-storage folders to host mounts, not legacy plugin aliases.
#[allow(non_snake_case)]
fn normalizeWorkspaceBindingVfsPath(path: &str) -> Result<Option<String>, String> {
    let segments = pathSegments(path);
    match segments.as_slice() {
        [ROOT_APP, APP_WORKSPACES, workspaceId, rest @ ..] => Ok(Some(joinNormalizedSegments(
            &[ROOT_APP, APP_WORKSPACES, workspaceId],
            rest,
        ))),
        [ROOT_MNT, MNT_WINDOWS, drive, rest @ ..] if drive.len() == 1 => {
            let driveLetter = normalizeDriveLetter(drive)?;
            Ok(Some(joinNormalizedSegments(
                &[ROOT_MNT, MNT_WINDOWS, &driveLetter],
                rest,
            )))
        }
        [ROOT_MNT, MNT_ANDROID, MNT_ANDROID_SDCARD, rest @ ..] => Ok(Some(joinNormalizedSegments(
            &[ROOT_MNT, MNT_ANDROID, MNT_ANDROID_SDCARD],
            rest,
        ))),
        [ROOT_MNT, MNT_ANDROID, MNT_ANDROID_ROOT, rest @ ..] => Ok(Some(joinNormalizedSegments(
            &[ROOT_MNT, MNT_ANDROID, MNT_ANDROID_ROOT], rest,
        ))),
        [ROOT_MNT, MNT_LINUX, rest @ ..] => {
            Ok(Some(joinNormalizedSegments(&[ROOT_MNT, MNT_LINUX], rest)))
        }
        [ROOT_MNT, MNT_MACOS, rest @ ..] => {
            Ok(Some(joinNormalizedSegments(&[ROOT_MNT, MNT_MACOS], rest)))
        }
        [ROOT_SDCARD, rest @ ..] | ["storage", "emulated", "0", rest @ ..] => {
            Ok(Some(joinNormalizedSegments(
                &[ROOT_MNT, MNT_ANDROID, MNT_ANDROID_SDCARD],
                rest,
            )))
        }
        [ROOT_DATA, rest @ ..] => Ok(Some(joinNormalizedSegments(&[ROOT_DATA], rest))),
        ["workspace", ..] => Err("Workspace binding cannot use /workspace".to_string()),
        [ROOT_MNT, platform, kind, id, rest @ ..] => Ok(Some(joinNormalizedSegments(
            &[ROOT_MNT, platform, kind, id], rest,
        ))),
        [ROOT_APP, ..] | [ROOT_MNT, ..] => Err(format!(
            "Workspace binding must use /app/workspaces/<id> or a mounted VFS path: {path}"
        )),
        _ => Ok(None),
    }
}

#[allow(non_snake_case)]
fn normalizeWindowsHostWorkspacePath(path: &str) -> Result<Option<String>, String> {
    let bytes = path.as_bytes();
    if bytes.len() < 2 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' {
        return Ok(None);
    }
    let driveLetter = (bytes[0] as char).to_ascii_lowercase().to_string();
    let rest = normalizeRelativePath(path[2..].trim_start_matches('/'))?;
    let restSegments = pathSegments(&rest);
    Ok(Some(joinNormalizedSegments(
        &[ROOT_MNT, MNT_WINDOWS, &driveLetter],
        &restSegments,
    )))
}

/// Maps an absolute host workspace path outside the recognized VFS roots.
#[allow(non_snake_case)]
fn normalizeAbsoluteHostWorkspacePath(path: &str) -> Result<String, String> {
    let segments = pathSegments(path);
    match segments.as_slice() {
        [] => Err("Workspace binding cannot use VFS root".to_string()),
        ["workspace", ..] => Err("Workspace binding cannot use /workspace".to_string()),
        [ROOT_APP, ..] | [ROOT_MNT, ..] => Err(format!(
            "Workspace binding must use /app/workspaces/<id> or a mounted VFS path: {path}"
        )),
        #[cfg(target_os = "macos")]
        _ => Ok(joinNormalizedSegments(&[ROOT_MNT, MNT_MACOS], &segments)),
        #[cfg(target_os = "android")]
        _ => Ok(joinNormalizedSegments(&[ROOT_MNT, MNT_ANDROID, MNT_ANDROID_ROOT], &segments)),
        #[cfg(not(any(target_os = "macos", target_os = "android")))]
        _ => Ok(joinNormalizedSegments(&[ROOT_MNT, MNT_LINUX], &segments)),
    }
}

#[allow(non_snake_case)]
fn joinNormalizedSegments(prefix: &[&str], rest: &[&str]) -> String {
    let mut segments = prefix
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    segments.extend(rest.iter().map(|value| (*value).to_string()));
    format!("/{}", segments.join("/"))
}

#[allow(non_snake_case)]
fn joinPhysical(root: &Path, rest: &[&str]) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in rest {
        path.push(segment);
    }
    path
}

#[allow(non_snake_case)]
fn joinUnixPhysical(root: &str, rest: &[&str]) -> PathBuf {
    let mut path = PathBuf::from(root);
    for segment in rest {
        path.push(segment);
    }
    path
}

#[allow(non_snake_case)]
fn physicalPathString(path: PathBuf) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[allow(non_snake_case)]
fn normalizePhysicalText(path: &str) -> String {
    let mut normalized = path.trim().replace('\\', "/");
    while normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }
    #[cfg(windows)]
    {
        normalized = normalized.to_ascii_lowercase();
    }
    normalized
}

#[allow(non_snake_case)]
fn normalizeDriveLetter(drive: &str) -> Result<String, String> {
    let mut chars = drive.chars();
    let Some(letter) = chars.next() else {
        return Err("Windows drive is required under /mnt/windows".to_string());
    };
    if chars.next().is_some() || !letter.is_ascii_alphabetic() {
        return Err(format!("Invalid Windows drive under /mnt/windows: {drive}"));
    }
    Ok(letter.to_ascii_lowercase().to_string())
}

#[allow(non_snake_case)]
fn directoryEntry(name: &str) -> FileEntry {
    FileEntry {
        name: name.to_string(),
        isDirectory: true,
        size: 0,
        permissions: "rwx".to_string(),
        lastModified: String::new(),
    }
}

#[allow(non_snake_case)]
fn mntMountEntries() -> Vec<FileEntry> {
    let mut entries = Vec::new();
    if windowsMounted() {
        entries.push(directoryEntry(MNT_WINDOWS));
    }
    if androidRootMounted() {
        entries.push(directoryEntry(MNT_ANDROID));
    }
    if linuxRootMounted() {
        entries.push(directoryEntry(MNT_LINUX));
    }
    if macosRootMounted() {
        entries.push(directoryEntry(MNT_MACOS));
    }
    entries
}

#[allow(non_snake_case)]
fn windowsMounted() -> bool {
    !windowsDriveEntries().is_empty()
}

#[allow(non_snake_case)]
fn windowsDriveEntries() -> Vec<FileEntry> {
    let mut entries = Vec::new();
    for letter in 'a'..='z' {
        let driveLetter = letter.to_string();
        if windowsDriveRootExists(&driveLetter) {
            entries.push(directoryEntry(&letter.to_string()));
        }
    }
    entries
}

#[allow(non_snake_case)]
#[cfg(windows)]
fn windowsDriveRootExists(driveLetter: &str) -> bool {
    let path = format!("{}:/", driveLetter.to_ascii_uppercase());
    Path::new(&path).exists()
}

#[allow(non_snake_case)]
#[cfg(not(windows))]
fn windowsDriveRootExists(_driveLetter: &str) -> bool {
    false
}

#[allow(non_snake_case)]
fn androidRootMounted() -> bool { androidPathMounted("/") }

#[allow(non_snake_case)]
fn androidSdcardMounted() -> bool {
    androidPathMounted("/sdcard")
}

#[allow(non_snake_case)]
fn androidDataMounted() -> bool {
    androidPathMounted("/data")
}

#[allow(non_snake_case)]
#[cfg(target_os = "android")]
fn androidPathMounted(path: &str) -> bool {
    Path::new(path).exists()
}

#[allow(non_snake_case)]
#[cfg(not(target_os = "android"))]
fn androidPathMounted(_path: &str) -> bool {
    false
}

#[allow(non_snake_case)]
#[cfg(target_os = "linux")]
fn linuxRootMounted() -> bool {
    Path::new("/").exists()
}

#[allow(non_snake_case)]
#[cfg(not(target_os = "linux"))]
fn linuxRootMounted() -> bool {
    false
}

#[allow(non_snake_case)]
#[cfg(target_os = "macos")]
fn macosRootMounted() -> bool {
    Path::new("/").exists()
}

#[allow(non_snake_case)]
#[cfg(not(target_os = "macos"))]
fn macosRootMounted() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapper() -> PathMapper {
        PathMapper::new(
            PathBuf::from("D:/operit"),
            PathBuf::from("D:/operit-workspaces"),
        )
    }

    #[test]
    fn clean_on_exit_vfs_path_follows_the_active_identity_root() {
        for runtimeRoot in [
            "/Users/test/Library/Containers/app.operit/Data/Library/Application Support/Operit2/runtime data/identities/identity-a",
            "/Volumes/External Disk/custom runtime/identities/identity-b",
        ] {
            let mapper = PathMapper::new(
                PathBuf::from(runtimeRoot),
                PathBuf::from("/custom workspaces"),
            );
            for relative in ["temp/clean_on_exit", "temp/clean_on_exit/terminal_output.log"] {
                let vfsPath = format!("/app/data/{relative}");
                let resolved = mapper.resolve(&vfsPath).unwrap();
                assert_eq!(resolved.vfsPath, vfsPath);
                assert_eq!(resolved.physicalPath, format!("{runtimeRoot}/{relative}"));
            }
            // Do not make arbitrary host paths valid VFS roots to mask a bad
            // caller: all operations must retain the existing mapping contract.
            let hostPath = format!("{runtimeRoot}/temp/clean_on_exit");
            assert!(mapper.resolve(&hostPath).unwrap_err().contains("Unknown VFS root"));
        }
    }

    #[test]
    fn rootListShowsVisibleRootsOnly() {
        let mut expected = vec!["app".to_string()];
        if !mntMountEntries().is_empty() {
            expected.push("mnt".to_string());
        }
        let names = mapper()
            .virtualDirectoryEntries("/")
            .unwrap()
            .unwrap()
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();
        assert_eq!(names, expected);
    }

    #[test]
    fn appWorkspacesResolveIntoWorkspaceCollectionMount() {
        let resolved = mapper()
            .resolve("/app/workspaces/chat-a/src/main.rs")
            .unwrap();
        assert_eq!(resolved.vfsPath, "/app/workspaces/chat-a/src/main.rs");
        assert_eq!(
            resolved.physicalPath,
            "D:/operit-workspaces/chat-a/src/main.rs"
        );
    }

    #[test]
    fn mountsResolveToPhysicalTargets() {
        #[cfg(windows)]
        {
            let driveEntry = windowsDriveEntries().into_iter().next().unwrap();
            let resolved = mapper()
                .resolve(&format!("/mnt/windows/{}", driveEntry.name))
                .unwrap();
            assert_eq!(resolved.physicalPath, format!("{}:/", driveEntry.name));
        }
        #[cfg(target_os = "android")]
        {
            assert_eq!(
                mapper()
                    .resolve("/mnt/android/sdcard/Download/Operit")
                    .unwrap()
                    .physicalPath,
                "/sdcard/Download/Operit"
            );
        }
        #[cfg(target_os = "linux")]
        {
            assert_eq!(
                mapper()
                    .resolve("/mnt/linux/home/user")
                    .unwrap()
                    .physicalPath,
                "/home/user"
            );
        }
        #[cfg(target_os = "macos")]
        {
            assert_eq!(
                mapper()
                    .resolve("/mnt/macos/Users/user/project")
                    .unwrap()
                    .physicalPath,
                "/Users/user/project"
            );
        }
    }

    #[test]
    fn unmountedMntEntriesDoNotResolve() {
        #[cfg(not(windows))]
        {
            assert!(mapper().resolve("/mnt/windows/c").is_err());
        }
        #[cfg(not(target_os = "android"))]
        {
            assert!(mapper().resolve("/mnt/android/sdcard/Download").is_err());
        }
        #[cfg(not(target_os = "linux"))]
        {
            assert!(mapper().resolve("/mnt/linux/home/user").is_err());
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert!(mapper().resolve("/mnt/macos/Users/user").is_err());
        }
    }

    #[test]
    fn hiddenAliasesResolveButDoNotAppearInRootList() {
        let rootNames = mapper()
            .virtualDirectoryEntries("/")
            .unwrap()
            .unwrap()
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();
        assert!(!rootNames.iter().any(|name| name == "sdcard"));
        assert!(!rootNames.iter().any(|name| name == "data"));
        #[cfg(target_os = "android")]
        {
            assert_eq!(
                mapper()
                    .resolve("/sdcard/Download/Operit")
                    .unwrap()
                    .physicalPath,
                "D:/operit/extensions/device/plugins/data"
            );
            assert_eq!(
                mapper().resolve("/data/local/tmp").unwrap().physicalPath,
                "/data/local/tmp"
            );
        }
        #[cfg(not(target_os = "android"))]
        {
            assert_eq!(mapper().resolve("/sdcard/Download/Operit").unwrap().physicalPath, "D:/operit/extensions/device/plugins/data");
            assert!(mapper().resolve("/data/local/tmp").is_err());
        }
    }

    /// Keeps workspace bindings rooted in the app collection or host mounts.
    #[test]
    fn workspaceBindingPathUsesExplicitVfsRoots() {
        assert_eq!(
            PathMapper::normalizeWorkspaceBindingPath("/app/workspaces/chat-a").unwrap(),
            "/app/workspaces/chat-a"
        );
        assert_eq!(
            PathMapper::normalizeWorkspaceBindingPath("/mnt/windows/D/code").unwrap(),
            "/mnt/windows/d/code"
        );
        assert_eq!(
            PathMapper::normalizeWorkspaceBindingPath("D:/code").unwrap(),
            "/mnt/windows/d/code"
        );
        #[cfg(target_os = "macos")]
        assert_eq!(
            PathMapper::normalizeWorkspaceBindingPath("/Users/user/project").unwrap(),
            "/mnt/macos/Users/user/project"
        );
        #[cfg(not(target_os = "macos"))]
        assert_eq!(
            PathMapper::normalizeWorkspaceBindingPath("/home/user/project").unwrap(),
            "/mnt/linux/home/user/project"
        );
        assert_eq!(
            PathMapper::normalizeWorkspaceBindingPath("/storage/emulated/0/Download").unwrap(),
            "/mnt/android/sdcard/Download"
        );
        assert!(PathMapper::normalizeWorkspaceBindingPath("/workspace").is_err());
        assert!(PathMapper::normalizeWorkspaceBindingPath("relative/path").is_err());
    }

    /// Preserves local Android directories across repeated workspace normalization.
    #[test]
    fn androidWorkspaceBindingPathsRemainOnTheSharedStorageMount() {
        for (input, expected) in [
            ("/sdcard", "/mnt/android/sdcard"),
            ("/storage/emulated/0", "/mnt/android/sdcard"),
            ("/sdcard/Download", "/mnt/android/sdcard/Download"),
            (
                "/storage/emulated/0/Download",
                "/mnt/android/sdcard/Download",
            ),
            (
                " /storage/emulated/0/Documents/My Project/ ",
                "/mnt/android/sdcard/Documents/My Project",
            ),
            (
                "/sdcard/Download/Operit/project",
                "/mnt/android/sdcard/Download/Operit/project",
            ),
            (
                "/sdcard/Download/Operit/plugins/project",
                "/mnt/android/sdcard/Download/Operit/plugins/project",
            ),
            (
                "/mnt/android/sdcard/Download/Operit/project",
                "/mnt/android/sdcard/Download/Operit/project",
            ),
        ] {
            let normalized = PathMapper::normalizeWorkspaceBindingPath(input).unwrap();
            assert_eq!(normalized, expected, "incorrect workspace mount for {input}");
            assert_eq!(
                PathMapper::normalizeWorkspaceBindingPath(&normalized).unwrap(),
                normalized,
                "workspace normalization must be idempotent for {input}"
            );
            assert_eq!(
                PathMapper::canonicalizeVfsPath(&normalized).unwrap(),
                normalized,
                "workspace mounts must not be rewritten as plugin data for {input}"
            );
        }
    }

    /// Rejects runtime plugin data as a user-selected workspace binding.
    #[test]
    fn workspaceBindingRejectsPluginStoragePaths() {
        assert!(PathMapper::normalizeWorkspaceBindingPath(
            "/app/data/extensions/plugins/data/Download"
        )
        .is_err());
    }

    /// Rejects parent traversal in virtual paths.
    #[test]
    fn rejectsParentSegments() {
        assert!(mapper().resolve("/app/workspaces/../x").is_err());
    }
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn registeredMountsListResolveRestoreAndUnregister() {
        let root = std::env::temp_dir().join(format!("operit-mapper-test-{}", uuid::Uuid::new_v4()));
        let registry = super::super::MountRegistry::testRegistry(&root);
        let mount = registry.register("/mnt/android/documents", "android_documents", "content://com.termux.documents/tree/opaque%2Fid", "Termux").unwrap();
        let mapper = PathMapper::new(root.clone(), root.join("workspaces")).withMountRegistry(registry.clone());
        for (parent, child) in [("/mnt", "android"), ("/mnt/android", "documents"), ("/mnt/android/documents", mount.id.as_str())] {
            assert!(mapper.virtualDirectoryEntries(parent).unwrap().unwrap().iter().any(|e| e.name == child));
        }
        assert!(mapper.virtualDirectoryEntries(&mount.vfsPath()).unwrap().is_none());
        let path = format!("{}/src/项目.py", mount.vfsPath());
        assert_eq!(PathMapper::normalizeWorkspaceBindingPath(&path).unwrap(), path);
        let resolved = mapper.resolve(&path).unwrap();
        let resource = FileSystemResource::parse(&resolved.physicalPath).unwrap().unwrap();
        assert_eq!(resource.path, "src/项目.py");
        assert_eq!(resource.root, mount.root);
        assert!(resolved.nativePath().is_err());
        let recreated = PathMapper::new(root.clone(), root.join("workspaces")).withMountRegistry(registry.clone());
        assert_eq!(recreated.resolve(&path).unwrap(), resolved);
        registry.remove(&mount.vfsPath()).unwrap();
        assert!(mapper.resolve(&path).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn nativeMountsAndResourceSearchResultsRespectTheirBoundaries() {
        let root = std::env::temp_dir().join(format!("operit-mapper-test-{}", uuid::Uuid::new_v4()));
        let registry = super::super::MountRegistry::testRegistry(&root);
        let mount = registry.register("/mnt/local/folders", "native", root.to_str().unwrap(), "Local").unwrap();
        let mapper = PathMapper::new(root.clone(), root.join("workspaces")).withMountRegistry(registry.clone());
        assert_eq!(mapper.resolve(&format!("{}/a.txt", mount.vfsPath())).unwrap().nativePath().unwrap(), root.join("a.txt").to_string_lossy());
        for platform in ["android", "windows", "macos", "linux"] {
            let path = format!("/mnt/{platform}/folders/mount-a/project");
            assert_eq!(PathMapper::normalizeWorkspaceBindingPath(&path).unwrap(), path);
        }
        let resource = FileSystemResource { backend: "test".into(), root: "opaque".into(), path: "src".into() };
        let base = ResolvedVfsPath { vfsPath: "/mnt/test/resources/id/src".into(), physicalPath: resource.encode().unwrap() };
        let child = FileSystemResource { path: "src/a.txt".into(), ..resource.clone() };
        assert_eq!(mapper.mapPhysicalChildToVfs(&base, &child.encode().unwrap()).unwrap(), "/mnt/test/resources/id/src/a.txt");
        for bad in [FileSystemResource { path: "src-other/a".into(), ..resource.clone() }, FileSystemResource { root: "other".into(), path: "src/a".into(), ..resource }] {
            assert!(mapper.mapPhysicalChildToVfs(&base, &bad.encode().unwrap()).is_err());
        }
        assert!(mapper.resolve(&format!("{}/../escape", mount.vfsPath())).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn androidRootIsAnExplicitUnrewrittenMount() {
        for path in ["/mnt/android/root", "/mnt/android/root/", "/mnt/android/root/data/data/com.termux/files/home", "/mnt/android/root/sdcard/Download/Operit"] {
            let normalized = PathMapper::normalizeWorkspaceBindingPath(path).unwrap();
            assert_eq!(normalized, path.trim_end_matches('/'));
            assert_eq!(PathMapper::canonicalizeVfsPath(&normalized).unwrap(), normalized);
        }
        #[cfg(target_os = "android")]
        for (vfs, native) in [("/mnt/android/root", "/"), ("/mnt/android/root/data", "/data"), ("/mnt/android/root/sdcard/Download/Operit", "/sdcard/Download/Operit")] {
            assert_eq!(mapper().resolve(vfs).unwrap().nativePath().unwrap(), native);
        }
        #[cfg(not(target_os = "android"))]
        assert!(mapper().resolve("/mnt/android/root").is_err());
    }

}
