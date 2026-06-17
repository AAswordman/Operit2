use std::path::{Path, PathBuf};

use operit_host_api::FileEntry;

const ROOT_APP: &str = "app";
const ROOT_WORKSPACE: &str = "workspace";
const ROOT_MNT: &str = "mnt";
const ROOT_SDCARD: &str = "sdcard";
const ROOT_DATA: &str = "data";

const APP_DATA: &str = "data";
const APP_FILES: &str = "files";
const APP_WORKSPACES: &str = "workspaces";

const MNT_WINDOWS: &str = "windows";
const MNT_ANDROID: &str = "android";
const MNT_LINUX: &str = "linux";
const MNT_ANDROID_SDCARD: &str = "sdcard";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedVfsPath {
    pub vfsPath: String,
    pub physicalPath: String,
}

#[derive(Debug, Clone)]
pub struct PathMapper {
    runtimeStoreRoot: PathBuf,
    appFilesRoot: Option<PathBuf>,
    workspaceCollectionRoot: PathBuf,
    currentWorkspacePath: Option<String>,
}

impl PathMapper {
    pub fn new(
        runtimeStoreRoot: PathBuf,
        appFilesRoot: Option<PathBuf>,
        workspaceCollectionRoot: PathBuf,
        currentWorkspacePath: Option<String>,
    ) -> Self {
        Self {
            runtimeStoreRoot,
            appFilesRoot,
            workspaceCollectionRoot,
            currentWorkspacePath,
        }
    }

    #[allow(non_snake_case)]
    pub fn workspaceCollectionPath() -> &'static str {
        "/app/workspaces"
    }

    #[allow(non_snake_case)]
    pub fn workspacePath(workspaceId: &str) -> Result<String, String> {
        let workspaceId = normalizeSingleSegment(workspaceId, "workspace id")?;
        Ok(format!(
            "{}/{}",
            Self::workspaceCollectionPath(),
            workspaceId
        ))
    }

    #[allow(non_snake_case)]
    pub fn normalizeVfsPath(path: &str) -> Result<String, String> {
        normalizeAbsoluteVfsPath(path)
    }

    #[allow(non_snake_case)]
    pub fn normalizeRelativePath(path: &str) -> Result<String, String> {
        normalizeRelativePath(path)
    }

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

    #[allow(non_snake_case)]
    pub fn virtualDirectoryEntries(&self, path: &str) -> Result<Option<Vec<FileEntry>>, String> {
        let normalizedPath = normalizeAbsoluteVfsPath(path)?;
        let segments = pathSegments(&normalizedPath);
        match segments.as_slice() {
            [] => {
                let mut entries = vec![directoryEntry(ROOT_APP), directoryEntry(ROOT_WORKSPACE)];
                if !mntMountEntries().is_empty() {
                    entries.push(directoryEntry(ROOT_MNT));
                }
                Ok(Some(entries))
            }
            [ROOT_APP] => Ok(Some(vec![
                directoryEntry(APP_DATA),
                directoryEntry(APP_FILES),
                directoryEntry(APP_WORKSPACES),
            ])),
            [ROOT_MNT] => {
                let entries = mntMountEntries();
                if entries.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(entries))
                }
            }
            [ROOT_MNT, MNT_ANDROID] if androidSdcardMounted() => {
                Ok(Some(vec![directoryEntry(MNT_ANDROID_SDCARD)]))
            }
            [ROOT_MNT, MNT_WINDOWS] if windowsMounted() => Ok(Some(windowsDriveEntries())),
            _ => Ok(None),
        }
    }

    pub fn resolve(&self, path: &str) -> Result<ResolvedVfsPath, String> {
        let normalizedPath = normalizeAbsoluteVfsPath(path)?;
        let segments = pathSegments(&normalizedPath);
        match segments.as_slice() {
            [] => Err("VFS root is a virtual directory".to_string()),
            [ROOT_APP] | [ROOT_MNT] => Err(format!("{normalizedPath} is a virtual directory")),
            [ROOT_WORKSPACE, rest @ ..] => self.resolveWorkspace(rest),
            [ROOT_APP, APP_DATA, rest @ ..] => Ok(ResolvedVfsPath {
                vfsPath: joinNormalizedSegments(&[ROOT_APP, APP_DATA], rest),
                physicalPath: physicalPathString(joinPhysical(&self.runtimeStoreRoot, rest)),
            }),
            [ROOT_APP, APP_FILES, rest @ ..] => {
                let Some(root) = self.appFilesRoot.as_ref() else {
                    return Err("/app/files root is not configured".to_string());
                };
                Ok(ResolvedVfsPath {
                    vfsPath: joinNormalizedSegments(&[ROOT_APP, APP_FILES], rest),
                    physicalPath: physicalPathString(joinPhysical(root, rest)),
                })
            }
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
            [ROOT_MNT, MNT_LINUX, rest @ ..] => {
                if !linuxRootMounted() {
                    return Err("/mnt/linux is not mounted".to_string());
                }
                Ok(ResolvedVfsPath {
                    vfsPath: joinNormalizedSegments(&[ROOT_MNT, MNT_LINUX], rest),
                    physicalPath: physicalPathString(joinUnixPhysical("/", rest)),
                })
            }
            [ROOT_SDCARD, rest @ ..] => {
                if !androidSdcardMounted() {
                    return Err("/sdcard is not mounted".to_string());
                }
                Ok(ResolvedVfsPath {
                    vfsPath: joinNormalizedSegments(&[ROOT_SDCARD], rest),
                    physicalPath: physicalPathString(joinUnixPhysical("/sdcard", rest)),
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

    #[allow(non_snake_case)]
    pub fn mapPhysicalChildToVfs(
        &self,
        base: &ResolvedVfsPath,
        physicalChildPath: &str,
    ) -> Result<String, String> {
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

    fn resolveWorkspace(&self, rest: &[&str]) -> Result<ResolvedVfsPath, String> {
        let Some(workspacePath) = self.currentWorkspacePath.as_ref() else {
            return Err("Current chat has no VFS workspace binding".to_string());
        };
        let workspacePath = normalizeAbsoluteVfsPath(workspacePath)?;
        let workspaceSegments = pathSegments(&workspacePath);
        if workspaceSegments.first() == Some(&ROOT_WORKSPACE) {
            return Err("Workspace binding cannot point to /workspace".to_string());
        }
        let targetPath = if rest.is_empty() {
            workspacePath
        } else {
            Self::joinVfsPath(&workspacePath, &rest.join("/"))?
        };
        self.resolve(&targetPath).map(|resolved| ResolvedVfsPath {
            vfsPath: joinNormalizedSegments(&[ROOT_WORKSPACE], rest),
            physicalPath: resolved.physicalPath,
        })
    }
}

impl Default for PathMapper {
    fn default() -> Self {
        Self {
            runtimeStoreRoot: PathBuf::new(),
            appFilesRoot: None,
            workspaceCollectionRoot: PathBuf::new(),
            currentWorkspacePath: None,
        }
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
    if androidSdcardMounted() {
        entries.push(directoryEntry(MNT_ANDROID));
    }
    if linuxRootMounted() {
        entries.push(directoryEntry(MNT_LINUX));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mapper() -> PathMapper {
        PathMapper::new(
            PathBuf::from("D:/operit"),
            Some(PathBuf::from("D:/operit/files")),
            PathBuf::from("D:/operit-workspaces"),
            Some("/app/workspaces/chat-a".to_string()),
        )
    }

    #[test]
    fn rootListShowsVisibleRootsOnly() {
        let mut expected = vec!["app".to_string(), "workspace".to_string()];
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
    fn currentWorkspaceUsesBoundWorkspace() {
        let resolved = mapper().resolve("/workspace/src/main.rs").unwrap();
        assert_eq!(resolved.vfsPath, "/workspace/src/main.rs");
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
                "/sdcard/Download/Operit"
            );
            assert_eq!(
                mapper().resolve("/data/local/tmp").unwrap().physicalPath,
                "/data/local/tmp"
            );
        }
        #[cfg(not(target_os = "android"))]
        {
            assert!(mapper().resolve("/sdcard/Download/Operit").is_err());
            assert!(mapper().resolve("/data/local/tmp").is_err());
        }
    }

    #[test]
    fn rejectsParentSegments() {
        assert!(mapper().resolve("/app/workspaces/../x").is_err());
    }
}
