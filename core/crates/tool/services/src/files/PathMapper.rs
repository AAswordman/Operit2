use std::path::{Path, PathBuf};
use std::sync::Arc;

use operit_host_api::{FileEntry, RuntimeStorageHost};
use operit_host_api::FileSystemResource::FileSystemResource;
use super::MountRegistry::{mountRelativePath, MountRegistry};
use operit_util::RuntimeStorageLayout::{
    EXTENSIONS_PLUGIN_CONFIGS_DIR_PATH, EXTENSIONS_PLUGIN_DATA_DIR_PATH,
    RUNTIME_ROOT_PATH_PREFIX, WORKSPACE_DIR_PATH,
};

const ROOT_APP: &str = "app";
const ROOT_MNT: &str = "mnt";
const ROOT_OPERITAPP: &str = "operitapp";
const ROOT_VIRTUAL: &str = "virtual";
const ROOT_SDCARD: &str = "sdcard";
const ROOT_DATA: &str = "data";

/// Controls discovery in the VFS root listing without changing path resolution.
struct VfsRoot {
    name: &'static str,
    hidden: bool,
}

const VFS_ROOTS: &[VfsRoot] = &[
    VfsRoot { name: ROOT_APP, hidden: true },
    VfsRoot { name: ROOT_OPERITAPP, hidden: false },
    VfsRoot { name: ROOT_MNT, hidden: true },
    VfsRoot { name: ROOT_VIRTUAL, hidden: false },
    VfsRoot { name: ROOT_SDCARD, hidden: true },
    VfsRoot { name: ROOT_DATA, hidden: true },
];

const APP_DATA: &str = "data";
const APP_WORKSPACES: &str = WORKSPACE_DIR_PATH;
const APP_OUTPUTS: &str = "outputs";

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
    /// Creates a mapper with the owning runtime's required storage capability.
    /// Omitting the Host is a compile error rather than silently hiding persistent mounts.
    ///
    /// ```compile_fail
    /// use operit_tools::files::PathMapper::PathMapper;
    /// let mapper = PathMapper::new("runtime".into(), "workspaces".into());
    /// ```
    ///
    /// A default mapper cannot bypass the required Host either.
    /// ```compile_fail
    /// use operit_tools::files::PathMapper::PathMapper;
    /// let mapper: PathMapper = Default::default();
    /// ```
    pub fn new(
        runtimeStoreRoot: PathBuf,
        workspaceCollectionRoot: PathBuf,
        storage: Arc<dyn RuntimeStorageHost>,
    ) -> Self {
        Self::fromRegistry(
            runtimeStoreRoot,
            workspaceCollectionRoot,
            MountRegistry::withStorage(storage),
        )
    }

    /// Explicitly opts out of persistent mounts for built-in path mapping only.
    /// Runtime consumers that need registered mounts must use `new` with their Host.
    pub fn builtinOnly(runtimeStoreRoot: PathBuf, workspaceCollectionRoot: PathBuf) -> Self {
        let mountRegistry = MountRegistry::withoutStorage(&runtimeStoreRoot);
        Self::fromRegistry(runtimeStoreRoot, workspaceCollectionRoot, mountRegistry)
    }

    fn fromRegistry(
        runtimeStoreRoot: PathBuf,
        workspaceCollectionRoot: PathBuf,
        mountRegistry: MountRegistry,
    ) -> Self {
        Self { runtimeStoreRoot, workspaceCollectionRoot, mountRegistry }
    }

    fn mounts(&self) -> Result<Vec<super::MountRegistry::VfsMount>, String> {
        self.mountRegistry.list()
    }

    /// Returns the VFS root containing all runtime workspaces.
    #[allow(non_snake_case)]
    pub fn workspaceCollectionPath() -> &'static str {
        "/operitapp/workspaces"
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
        let root = Self::joinVfsPath("/operitapp/data", root)?;
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

    /// Resolves an absolute resource locator or a path relative to a bound workspace.
    #[allow(non_snake_case)]
    pub fn resolveWorkspaceResourcePath(base: &str, path: &str) -> Result<String, String> {
        let text = path.replace('\\', "/");
        if text.starts_with('/') {
            return Self::normalizeWorkspaceBindingPath(&text);
        }
        match normalizeWindowsHostWorkspacePath(&text)? {
            Some(absolute) => Ok(absolute),
            None => Self::joinVfsPath(base, &text),
        }
    }

    /// Resolves a file-tool path against the active workspace without rewriting absolute paths.
    #[allow(non_snake_case)]
    pub fn resolveFileToolPath(workspace: Option<&str>, path: &str) -> Result<String, String> {
        let text = path.trim();
        if text.starts_with('/') || normalizeWindowsHostWorkspacePath(text)?.is_some() {
            return Ok(text.to_string());
        }
        let base = workspace.filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "Relative file paths require a current file directory".to_string())?;
        let base = normalizeAbsoluteVfsPath(base)?;
        let mut segments: Vec<_> = base.split('/').filter(|part| !part.is_empty()).collect();
        for part in text.split('/') {
            match part {
                "" | "." => {},
                ".." => { segments.pop().ok_or("Relative file path escapes the VFS root")?; },
                value if value.contains(['\\', '\0']) => return Err("Invalid relative file path".into()),
                value => segments.push(value),
            }
        }
        Ok(format!("/{}", segments.join("/")))
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
        let rootSegments = pathSegments(&root);
        let fullSegments = pathSegments(&fullPath);
        if fullSegments.len() < rootSegments.len() {
            return Ok(None);
        }
        let inside = rootSegments.iter().zip(&fullSegments).enumerate().all(|(index, (root, path))| {
            match (index, *root, *path) {
                (0, ROOT_APP | ROOT_OPERITAPP, ROOT_APP | ROOT_OPERITAPP)
                | (0, ROOT_MNT | ROOT_VIRTUAL, ROOT_MNT | ROOT_VIRTUAL) => true,
                _ => root == path,
            }
        });
        if !inside { return Ok(None); }
        Ok(Some(fullSegments[rootSegments.len()..].join("/")))
    }

    /// Expresses a stable VFS locator from the current file directory, including sibling paths.
    /// Root aliases are compared using the same mount identities as path resolution.
    #[allow(non_snake_case)]
    pub fn relativeFileToolPath(base: &str, path: &str) -> Result<String, String> {
        let base = normalizeAbsoluteVfsPath(base)?;
        let path = normalizeAbsoluteVfsPath(path)?;
        let baseSegments = pathSegments(&base);
        let pathSegments = pathSegments(&path);
        let shared = baseSegments.iter().zip(&pathSegments).enumerate().take_while(|(index, (left, right))| {
            match (*index, **left, **right) {
                (0, ROOT_APP | ROOT_OPERITAPP, ROOT_APP | ROOT_OPERITAPP)
                | (0, ROOT_MNT | ROOT_VIRTUAL, ROOT_MNT | ROOT_VIRTUAL) => true,
                _ => left == right,
            }
        }).count();
        let mut relative = vec![".."; baseSegments.len() - shared];
        relative.extend_from_slice(&pathSegments[shared..]);
        Ok(if relative.is_empty() { ".".into() } else { relative.join("/") })
    }

    /// Returns synthetic entries for VFS directories that are not backed by a host directory.
    #[allow(non_snake_case)]
    pub fn virtualDirectoryEntries(&self, path: &str) -> Result<Option<Vec<FileEntry>>, String> {
        let normalizedPath = normalizeAbsoluteVfsPath(path)?;
        let segments = pathSegments(&normalizedPath);
        let mountPath = mountRelativePath(&normalizedPath);
        let mounts = if normalizedPath == "/" || mountPath.is_some() {
            self.mounts()?
        } else { Vec::new() };
        let mut entries = match segments.as_slice() {
            [] => {
                let hasPlatformMounts = !mntMountEntries().is_empty() || !mounts.is_empty();
                Some(VFS_ROOTS.iter()
                    .filter(|root| !root.hidden)
                    .filter(|root| root.name != ROOT_VIRTUAL || hasPlatformMounts)
                    .map(|root| directoryEntry(root.name))
                    .collect())
            }
            [ROOT_APP | ROOT_OPERITAPP] => Some(vec![directoryEntry(APP_DATA), directoryEntry(APP_WORKSPACES), directoryEntry(APP_OUTPUTS)]),
            [ROOT_MNT | ROOT_VIRTUAL] => { let entries = mntMountEntries(); if entries.is_empty() { None } else { Some(entries) } }
            [ROOT_MNT | ROOT_VIRTUAL, MNT_ANDROID] if androidRootMounted() => {
                let mut entries = vec![directoryEntry(MNT_ANDROID_ROOT)];
                if androidSdcardMounted() { entries.push(directoryEntry(MNT_ANDROID_SDCARD)); }
                Some(entries)
            }
            [ROOT_MNT | ROOT_VIRTUAL, MNT_WINDOWS] if windowsMounted() => Some(windowsDriveEntries()),
            _ => None,
        };
        if let Some(mountPath) = mountPath {
            let prefix = if mountPath.is_empty() { String::new() } else { format!("{mountPath}/") };
            for mount in mounts {
                let path = mount.vfsPath();
                if let Some(rest) = mountRelativePath(&path).and_then(|path| path.strip_prefix(&prefix)) {
                    if let Some(name) = rest.split('/').next() {
                        let list = entries.get_or_insert_with(Vec::new);
                        if !list.iter().any(|entry| entry.name == name) { list.push(directoryEntry(name)); }
                    }
                }
            }
        }
        Ok(entries)
    }

    /// Resolves a normalized VFS path into its host physical path.
    pub fn resolve(&self, path: &str) -> Result<ResolvedVfsPath, String> {
        let normalizedPath = Self::canonicalizeVfsPath(path)?;
        let segments = pathSegments(&normalizedPath);
        if let Some(mountPath) = mountRelativePath(&normalizedPath) {
            for mount in self.mounts()? {
                let path = mount.vfsPath();
                let root = mountRelativePath(&path).expect("validated mount namespace");
                let relative = if mountPath == root { Some("") } else { mountPath.strip_prefix(&format!("{root}/")) };
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
            [ROOT_APP | ROOT_OPERITAPP] | [ROOT_MNT | ROOT_VIRTUAL] => Err(format!("{normalizedPath} is a virtual directory")),
            [root @ (ROOT_APP | ROOT_OPERITAPP), APP_DATA, rest @ ..] => Ok(ResolvedVfsPath {
                vfsPath: joinNormalizedSegments(&[root, APP_DATA], rest),
                physicalPath: physicalPathString(joinPhysical(&self.runtimeStoreRoot, rest)),
            }),
            [root @ (ROOT_APP | ROOT_OPERITAPP), APP_OUTPUTS, rest @ ..] => Ok(ResolvedVfsPath {
                vfsPath: joinNormalizedSegments(&[root, APP_OUTPUTS], rest),
                physicalPath: physicalPathString(joinPhysical(&self.runtimeStoreRoot.join("data/chat_outputs"), rest)),
            }),
            [root @ (ROOT_APP | ROOT_OPERITAPP), APP_WORKSPACES, rest @ ..] => Ok(ResolvedVfsPath {
                vfsPath: joinNormalizedSegments(&[root, APP_WORKSPACES], rest),
                physicalPath: physicalPathString(joinPhysical(&self.workspaceCollectionRoot, rest)),
            }),
            [root @ (ROOT_MNT | ROOT_VIRTUAL), MNT_WINDOWS, drive, rest @ ..] => {
                let driveLetter = normalizeDriveLetter(drive)?;
                if !windowsDriveRootExists(&driveLetter) {
                    return Err(format!(
                        "Windows drive is not mounted under /virtual/windows: {driveLetter}"
                    ));
                }
                let mut physical = PathBuf::from(format!("{driveLetter}:/"));
                for segment in rest {
                    physical.push(segment);
                }
                Ok(ResolvedVfsPath {
                    vfsPath: joinNormalizedSegments(&[root, MNT_WINDOWS, &driveLetter], rest),
                    physicalPath: physicalPathString(physical),
                })
            }
            [root @ (ROOT_MNT | ROOT_VIRTUAL), MNT_ANDROID, MNT_ANDROID_SDCARD, rest @ ..] => {
                if !androidSdcardMounted() {
                    return Err("/virtual/android/sdcard is not mounted".to_string());
                }
                Ok(ResolvedVfsPath {
                    vfsPath: joinNormalizedSegments(
                        &[root, MNT_ANDROID, MNT_ANDROID_SDCARD],
                        rest,
                    ),
                    physicalPath: physicalPathString(joinUnixPhysical("/sdcard", rest)),
                })
            }
            [root @ (ROOT_MNT | ROOT_VIRTUAL), MNT_ANDROID, MNT_ANDROID_ROOT, rest @ ..] => {
                if !androidRootMounted() { return Err("/virtual/android/root is not mounted".into()); }
                Ok(ResolvedVfsPath {
                    vfsPath: joinNormalizedSegments(&[root, MNT_ANDROID, MNT_ANDROID_ROOT], rest),
                    physicalPath: physicalPathString(joinUnixPhysical("/", rest)),
                })
            }
            [root @ (ROOT_MNT | ROOT_VIRTUAL), MNT_LINUX, rest @ ..] => {
                if !linuxRootMounted() {
                    return Err("/virtual/linux is not mounted".to_string());
                }
                Ok(ResolvedVfsPath {
                    vfsPath: joinNormalizedSegments(&[root, MNT_LINUX], rest),
                    physicalPath: physicalPathString(joinUnixPhysical("/", rest)),
                })
            }
            [root @ (ROOT_MNT | ROOT_VIRTUAL), MNT_MACOS, rest @ ..] => {
                if !macosRootMounted() {
                    return Err("/virtual/macos is not mounted".to_string());
                }
                Ok(ResolvedVfsPath {
                    vfsPath: joinNormalizedSegments(&[root, MNT_MACOS], rest),
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
        [root @ (ROOT_APP | ROOT_OPERITAPP), APP_WORKSPACES, workspaceId, rest @ ..] => Ok(Some(joinNormalizedSegments(
            &[root, APP_WORKSPACES, workspaceId],
            rest,
        ))),
        [root @ (ROOT_MNT | ROOT_VIRTUAL), MNT_WINDOWS, drive, rest @ ..] if drive.len() == 1 => {
            let driveLetter = normalizeDriveLetter(drive)?;
            Ok(Some(joinNormalizedSegments(
                &[root, MNT_WINDOWS, &driveLetter],
                rest,
            )))
        }
        [root @ (ROOT_MNT | ROOT_VIRTUAL), MNT_ANDROID, MNT_ANDROID_SDCARD, rest @ ..] => Ok(Some(joinNormalizedSegments(
            &[root, MNT_ANDROID, MNT_ANDROID_SDCARD],
            rest,
        ))),
        [root @ (ROOT_MNT | ROOT_VIRTUAL), MNT_ANDROID, MNT_ANDROID_ROOT, rest @ ..] => Ok(Some(joinNormalizedSegments(
            &[root, MNT_ANDROID, MNT_ANDROID_ROOT], rest,
        ))),
        [root @ (ROOT_MNT | ROOT_VIRTUAL), MNT_LINUX, rest @ ..] => {
            Ok(Some(joinNormalizedSegments(&[root, MNT_LINUX], rest)))
        }
        [root @ (ROOT_MNT | ROOT_VIRTUAL), MNT_MACOS, rest @ ..] => {
            Ok(Some(joinNormalizedSegments(&[root, MNT_MACOS], rest)))
        }
        [ROOT_SDCARD, rest @ ..] | ["storage", "emulated", "0", rest @ ..] => {
            Ok(Some(joinNormalizedSegments(
                &[ROOT_VIRTUAL, MNT_ANDROID, MNT_ANDROID_SDCARD],
                rest,
            )))
        }
        [ROOT_DATA, rest @ ..] => Ok(Some(joinNormalizedSegments(&[ROOT_DATA], rest))),
        ["workspace", ..] => Err("Workspace binding cannot use /workspace".to_string()),
        [root @ (ROOT_MNT | ROOT_VIRTUAL), platform, kind, id, rest @ ..] => Ok(Some(joinNormalizedSegments(
            &[root, platform, kind, id], rest,
        ))),
        [ROOT_APP | ROOT_OPERITAPP, ..] | [ROOT_MNT | ROOT_VIRTUAL, ..] => Err(format!(
            "Workspace binding must use /operitapp/workspaces/<id> or a mounted VFS path: {path}"
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
        &[ROOT_VIRTUAL, MNT_WINDOWS, &driveLetter],
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
        [ROOT_APP | ROOT_OPERITAPP, ..] | [ROOT_MNT | ROOT_VIRTUAL, ..] => Err(format!(
            "Workspace binding must use /operitapp/workspaces/<id> or a mounted VFS path: {path}"
        )),
        #[cfg(target_os = "macos")]
        _ => Ok(joinNormalizedSegments(&[ROOT_VIRTUAL, MNT_MACOS], &segments)),
        #[cfg(target_os = "android")]
        _ => Ok(joinNormalizedSegments(&[ROOT_VIRTUAL, MNT_ANDROID, MNT_ANDROID_ROOT], &segments)),
        #[cfg(not(any(target_os = "macos", target_os = "android")))]
        _ => Ok(joinNormalizedSegments(&[ROOT_VIRTUAL, MNT_LINUX], &segments)),
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
        return Err("Windows drive is required under /virtual/windows".to_string());
    };
    if chars.next().is_some() || !letter.is_ascii_alphabetic() {
        return Err(format!("Invalid Windows drive under /virtual/windows: {drive}"));
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

    #[test]
    fn file_tool_paths_resolve_against_workspace_without_alias_rewrites() {
        let base = Some("/app/workspaces/chat-a");
        for (path, expected) in [
            (".", "/app/workspaces/chat-a"),
            ("./", "/app/workspaces/chat-a"),
            ("./src/中文 file.rs", "/app/workspaces/chat-a/src/中文 file.rs"),
            ("src/file", "/app/workspaces/chat-a/src/file"),
            ("/app/data/a", "/app/data/a"),
            ("/operitapp/data/a", "/operitapp/data/a"),
        ] {
            assert_eq!(PathMapper::resolveFileToolPath(base, path).unwrap(), expected);
        }
        assert!(PathMapper::resolveFileToolPath(None, "src/file").is_err());
        assert_eq!(PathMapper::resolveFileToolPath(base, "../outside").unwrap(), "/app/workspaces/outside");
        assert_eq!(PathMapper::resolveFileToolPath(base, "src/../../outside").unwrap(), "/app/workspaces/outside");
        assert!(PathMapper::resolveFileToolPath(base, "../../../../outside").is_err());
    }

    #[test]
    fn attachment_relative_paths_round_trip_across_managed_directories_and_aliases() {
        for (base, target, expected) in [
            ("/operitapp/workspaces/project", "/operitapp/workspaces/project/.operit/attachments/id/中文 & file.pdf", ".operit/attachments/id/中文 & file.pdf"),
            ("/operitapp/outputs/family", "/operitapp/outputs/family/.operit/attachments/id/file.pdf", ".operit/attachments/id/file.pdf"),
            ("/operitapp/workspaces/project", "/operitapp/outputs/family/file.pdf", "../../outputs/family/file.pdf"),
            ("/app/workspaces/project", "/operitapp/workspaces/project/file.pdf", "file.pdf"),
            ("/mnt/local/project", "/virtual/local/project/file.pdf", "file.pdf"),
            ("/operitapp/workspaces/project", "/operitapp/workspaces/project", "."),
        ] {
            let relative = PathMapper::relativeFileToolPath(base, target).unwrap();
            assert_eq!(relative, expected);
            let resolved = PathMapper::resolveFileToolPath(Some(base), &relative).unwrap();
            assert_eq!(PathMapper::relativeFileToolPath(&resolved, target).unwrap(), ".");
        }
    }

    /// Resolves relative, mounted and native resource paths at one runtime boundary.
    #[test]
    fn workspace_resource_locators_keep_absolute_and_relative_ownership() {
        let base = "/operitapp/workspaces/chat-a";
        assert_eq!(
            PathMapper::resolveWorkspaceResourcePath(base, "src/a file.ts").unwrap(),
            "/operitapp/workspaces/chat-a/src/a file.ts"
        );
        assert_eq!(
            PathMapper::resolveWorkspaceResourcePath(base, "D:\\repo\\a.ts").unwrap(),
            "/virtual/windows/d/repo/a.ts"
        );
        assert_eq!(
            PathMapper::resolveWorkspaceResourcePath("", "D:\\repo\\a.ts").unwrap(),
            "/virtual/windows/d/repo/a.ts"
        );
        assert!(PathMapper::resolveWorkspaceResourcePath("", "src/a.ts").is_err());
        assert_eq!(
            PathMapper::resolveWorkspaceResourcePath(base, "/virtual/linux/home/repo/a.ts").unwrap(),
            "/virtual/linux/home/repo/a.ts"
        );
        assert!(PathMapper::resolveWorkspaceResourcePath(base, "../other/a.ts").is_err());
    }

    fn mapper() -> PathMapper {
        PathMapper::builtinOnly(
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
            let mapper = PathMapper::builtinOnly(
                PathBuf::from(runtimeRoot),
                PathBuf::from("/custom workspaces"),
            );
            for relative in ["temp/clean_on_exit", "temp/clean_on_exit/terminal_output.log"] {
                let vfsPath = format!("/operitapp/data/{relative}");
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
    fn both_roots_preserve_addresses_and_share_storage() {
        for (first, second) in [
            ("/app/workspaces/chat-a", "/operitapp/workspaces/chat-a"),
            ("/mnt/linux/home", "/virtual/linux/home"),
        ] {
            for path in [first, second] {
                assert_eq!(PathMapper::normalizeVfsPath(path).unwrap(), path);
                assert_eq!(PathMapper::normalizeWorkspaceBindingPath(path).unwrap(), path);
                assert_eq!(PathMapper::joinVfsPath(path, "file.txt").unwrap(), format!("{path}/file.txt"));
                assert_eq!(PathMapper::relativePath(path, &format!("{path}/file.txt")).unwrap(), Some("file.txt".into()));
            }
            assert_eq!(PathMapper::relativePath(first, &format!("{second}/file.txt")).unwrap(), Some("file.txt".into()));
            assert_eq!(PathMapper::relativePath(second, &format!("{first}/file.txt")).unwrap(), Some("file.txt".into()));
        }
        for suffix in ["data/config/example.json", "workspaces/chat-a/file.txt"] {
            let first = mapper().resolve(&format!("/app/{suffix}")).unwrap();
            let second = mapper().resolve(&format!("/operitapp/{suffix}")).unwrap();
            assert_eq!(first.vfsPath, format!("/app/{suffix}"));
            assert_eq!(second.vfsPath, format!("/operitapp/{suffix}"));
            assert_eq!(first.physicalPath, second.physicalPath);
        }
        assert_eq!(PathMapper::relativePath("/app/workspaces/chat-a", "/operitapp/workspaces/chat-ab/file.txt").unwrap(), None);
        assert!(mapper().resolve("/application/data").is_err());
        assert!(mapper().resolve("/mnt-other/linux/home").is_err());
        assert!(mapper().resolve("/operitapp-other/data").is_err());
        assert!(mapper().resolve("/virtual-other/linux/home").is_err());
    }


    #[test]
    fn rootListShowsVisibleRootsOnly() {
        let mut expected = vec!["operitapp".to_string()];
        if !mntMountEntries().is_empty() {
            expected.push("virtual".to_string());
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
            .resolve("/operitapp/workspaces/chat-a/src/main.rs")
            .unwrap();
        assert_eq!(resolved.vfsPath, "/operitapp/workspaces/chat-a/src/main.rs");
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
                .resolve(&format!("/virtual/windows/{}", driveEntry.name))
                .unwrap();
            assert_eq!(resolved.physicalPath, format!("{}:/", driveEntry.name));
        }
        #[cfg(target_os = "android")]
        {
            assert_eq!(
                mapper()
                    .resolve("/virtual/android/sdcard/Download/Operit")
                    .unwrap()
                    .physicalPath,
                "/sdcard/Download/Operit"
            );
        }
        #[cfg(target_os = "linux")]
        {
            assert_eq!(
                mapper()
                    .resolve("/virtual/linux/home/user")
                    .unwrap()
                    .physicalPath,
                "/home/user"
            );
        }
        #[cfg(target_os = "macos")]
        {
            assert_eq!(
                mapper()
                    .resolve("/virtual/macos/Users/user/project")
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
            assert!(mapper().resolve("/virtual/windows/c").is_err());
        }
        #[cfg(not(target_os = "android"))]
        {
            assert!(mapper().resolve("/virtual/android/sdcard/Download").is_err());
        }
        #[cfg(not(target_os = "linux"))]
        {
            assert!(mapper().resolve("/virtual/linux/home/user").is_err());
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert!(mapper().resolve("/virtual/macos/Users/user").is_err());
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
        for hidden in ["app", "mnt", "sdcard", "data"] {
            assert!(!rootNames.iter().any(|name| name == hidden), "{hidden} must stay hidden");
        }
        assert_eq!(
            mapper().virtualDirectoryEntries("/app").unwrap(),
            mapper().virtualDirectoryEntries("/operitapp").unwrap(),
        );
        assert_eq!(
            mapper().virtualDirectoryEntries("/mnt").unwrap(),
            mapper().virtualDirectoryEntries("/virtual").unwrap(),
        );
        assert_eq!(
            mapper().resolve("/app/workspaces/demo").unwrap().physicalPath,
            mapper().resolve("/operitapp/workspaces/demo").unwrap().physicalPath,
        );
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
            PathMapper::normalizeWorkspaceBindingPath("/operitapp/workspaces/chat-a").unwrap(),
            "/operitapp/workspaces/chat-a"
        );
        assert_eq!(
            PathMapper::normalizeWorkspaceBindingPath("/virtual/windows/D/code").unwrap(),
            "/virtual/windows/d/code"
        );
        assert_eq!(
            PathMapper::normalizeWorkspaceBindingPath("D:/code").unwrap(),
            "/virtual/windows/d/code"
        );
        #[cfg(target_os = "macos")]
        assert_eq!(
            PathMapper::normalizeWorkspaceBindingPath("/Users/user/project").unwrap(),
            "/virtual/macos/Users/user/project"
        );
        #[cfg(not(target_os = "macos"))]
        assert_eq!(
            PathMapper::normalizeWorkspaceBindingPath("/home/user/project").unwrap(),
            "/virtual/linux/home/user/project"
        );
        assert_eq!(
            PathMapper::normalizeWorkspaceBindingPath("/storage/emulated/0/Download").unwrap(),
            "/virtual/android/sdcard/Download"
        );
        assert!(PathMapper::normalizeWorkspaceBindingPath("/workspace").is_err());
        assert!(PathMapper::normalizeWorkspaceBindingPath("relative/path").is_err());
    }

    /// Preserves local Android directories across repeated workspace normalization.
    #[test]
    fn androidWorkspaceBindingPathsRemainOnTheSharedStorageMount() {
        for (input, expected) in [
            ("/sdcard", "/virtual/android/sdcard"),
            ("/storage/emulated/0", "/virtual/android/sdcard"),
            ("/sdcard/Download", "/virtual/android/sdcard/Download"),
            (
                "/storage/emulated/0/Download",
                "/virtual/android/sdcard/Download",
            ),
            (
                " /storage/emulated/0/Documents/My Project/ ",
                "/virtual/android/sdcard/Documents/My Project",
            ),
            (
                "/sdcard/Download/Operit/project",
                "/virtual/android/sdcard/Download/Operit/project",
            ),
            (
                "/sdcard/Download/Operit/plugins/project",
                "/virtual/android/sdcard/Download/Operit/plugins/project",
            ),
            (
                "/virtual/android/sdcard/Download/Operit/project",
                "/virtual/android/sdcard/Download/Operit/project",
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
            "/operitapp/data/extensions/plugins/data/Download"
        )
        .is_err());
    }

    /// Rejects parent traversal in virtual paths.
    #[test]
    fn rejectsParentSegments() {
        assert!(mapper().resolve("/operitapp/workspaces/../x").is_err());
    }
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn registeredMountsListResolveRestoreAndUnregister() {
        let root = std::env::temp_dir().join(format!("operit-mapper-test-{}", uuid::Uuid::new_v4()));
        let storage = Arc::new(operit_host_native_storage::NativeRuntimeStorageHost::new(
            root.clone(), root.join("workspaces"),
        ));
        let registry = MountRegistry::withStorage(storage.clone());
        let mount = registry.register("/virtual/android/documents", "android_documents", "content://com.termux.documents/tree/opaque%2Fid", "Termux").unwrap();
        let mapper = PathMapper::new(root.clone(), root.join("workspaces"), storage.clone());
        for (parent, child) in [("/virtual", "android"), ("/virtual/android", "documents"), ("/virtual/android/documents", mount.id.as_str())] {
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
        let other = format!("/mnt/android/documents/{}/src/项目.py", mount.id);
        let otherResolved = mapper.resolve(&other).unwrap();
        assert_eq!(otherResolved.vfsPath, other);
        assert_eq!(otherResolved.physicalPath, resolved.physicalPath);
        assert!(otherResolved.nativePath().is_err());
        let recreated = PathMapper::new(root.clone(), root.join("workspaces"), storage.clone());
        assert_eq!(recreated.resolve(&path).unwrap(), resolved);
        registry.remove(&mount.vfsPath()).unwrap();
        assert!(mapper.resolve(&path).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Keeps persisted mounts out of the builtin-only mapper on every host.
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn builtinOnlyExplicitlyExcludesPersistedMounts() {
        let root = std::env::temp_dir().join(format!("operit-builtin-mapper-test-{}", uuid::Uuid::new_v4()));
        let storage = Arc::new(operit_host_native_storage::NativeRuntimeStorageHost::new(
            root.clone(), root.join("workspaces"),
        ));
        let registry = MountRegistry::withStorage(storage.clone());
        let mount = registry.register("/virtual/test/resources", "test", "opaque", "Test").unwrap();
        let path = format!("{}/file.txt", mount.vfsPath());
        let persistent = PathMapper::new(root.clone(), root.join("workspaces"), storage);
        assert!(persistent.resolve(&path).is_ok());

        let builtin = PathMapper::builtinOnly(root.clone(), root.join("workspaces"));
        assert!(builtin.resolve(&path).is_err());
        assert_eq!(builtin.resolve("/operitapp/data/config/example.json").unwrap().physicalPath,
            root.join("config/example.json").to_string_lossy().replace('\\', "/"));
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Preserves native mount paths and rejects resources outside their registered roots.
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn nativeMountsAndResourceSearchResultsRespectTheirBoundaries() {
        let root = std::env::temp_dir().join(format!("operit-mapper-test-{}", uuid::Uuid::new_v4()));
        let storage = Arc::new(operit_host_native_storage::NativeRuntimeStorageHost::new(
            root.clone(), root.join("workspaces"),
        ));
        let registry = MountRegistry::withStorage(storage.clone());
        let mount = registry.register("/virtual/local/folders", "native", root.to_str().unwrap(), "Local").unwrap();
        let mapper = PathMapper::new(root.clone(), root.join("workspaces"), storage.clone());
        assert_eq!(mapper.resolve(&format!("{}/a.txt", mount.vfsPath())).unwrap().nativePath().unwrap(), root.join("a.txt").to_string_lossy().replace('\\', "/"));
        for platform in ["android", "windows", "macos", "linux"] {
            let path = format!("/virtual/{platform}/folders/mount-a/project");
            assert_eq!(PathMapper::normalizeWorkspaceBindingPath(&path).unwrap(), path);
        }
        let resource = FileSystemResource { backend: "test".into(), root: "opaque".into(), path: "src".into() };
        let base = ResolvedVfsPath { vfsPath: "/virtual/test/resources/id/src".into(), physicalPath: resource.encode().unwrap() };
        let child = FileSystemResource { path: "src/a.txt".into(), ..resource.clone() };
        assert_eq!(mapper.mapPhysicalChildToVfs(&base, &child.encode().unwrap()).unwrap(), "/virtual/test/resources/id/src/a.txt");
        for bad in [FileSystemResource { path: "src-other/a".into(), ..resource.clone() }, FileSystemResource { root: "other".into(), path: "src/a".into(), ..resource }] {
            assert!(mapper.mapPhysicalChildToVfs(&base, &bad.encode().unwrap()).is_err());
        }
        assert!(mapper.resolve(&format!("{}/../escape", mount.vfsPath())).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn androidRootIsAnExplicitUnrewrittenMount() {
        for path in ["/virtual/android/root", "/virtual/android/root/", "/virtual/android/root/data/data/com.termux/files/home", "/virtual/android/root/sdcard/Download/Operit"] {
            let normalized = PathMapper::normalizeWorkspaceBindingPath(path).unwrap();
            assert_eq!(normalized, path.trim_end_matches('/'));
            assert_eq!(PathMapper::canonicalizeVfsPath(&normalized).unwrap(), normalized);
        }
        #[cfg(target_os = "android")]
        for (vfs, native) in [("/virtual/android/root", "/"), ("/virtual/android/root/data", "/data"), ("/virtual/android/root/sdcard/Download/Operit", "/sdcard/Download/Operit")] {
            assert_eq!(mapper().resolve(vfs).unwrap().nativePath().unwrap(), native);
        }
        #[cfg(not(target_os = "android"))]
        assert!(mapper().resolve("/virtual/android/root").is_err());
    }

}
