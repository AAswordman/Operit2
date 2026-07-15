use std::fs;
use std::io::{Cursor, Read};
use std::path::Component;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bzip2_rs::DecoderReader;
use operit_host_api::{
    HttpDownloadControl, HttpDownloadFileRequest, HttpDownloadProgress, HttpDownloadProgressState,
    HttpDownloadRequest, HttpHost, RuntimeStorageHost,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::LocalEngineManifest::{
    LocalEngineArchiveFormat, LocalEngineDelivery, LocalEngineManifest, LocalPlatformTarget,
};
use crate::LocalModelRegistry::{InstalledLocalEngine, LocalModelRegistrySnapshot};
use crate::LocalModelRegistryStore::LocalModelRegistryStore;
use crate::LocalModelStorage::{buildLocalEngineStoragePath, validatedStorageSegment};

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum LocalEngineDownloadError {
    #[error("storage error: {0}")]
    Storage(String),
    #[error("http error: {0}")]
    Http(String),
    #[error("engine artifact is unavailable for target: {0}")]
    ArtifactUnavailable(String),
    #[error("engine archive size mismatch")]
    SizeMismatch,
    #[error("engine archive checksum mismatch")]
    ChecksumMismatch,
    #[error("invalid engine archive entry: {0}")]
    InvalidArchiveEntry(String),
    #[error("engine runtime file is missing: {0}")]
    RuntimeFileMissing(String),
    #[error("installed local engine not found: {0}")]
    EngineNotInstalled(String),
    #[error("local model registry error: {0}")]
    Registry(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalEngineInstallRequest {
    pub manifest: LocalEngineManifest,
    pub target: LocalPlatformTarget,
    pub installedAtMs: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalEngineInstallResult {
    pub installedEngine: InstalledLocalEngine,
    pub downloadedBytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalEngineDeleteResult {
    pub removedEngine: InstalledLocalEngine,
    pub removedStoragePath: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub enum LocalEngineDownloadProgress {
    Started {
        engineId: String,
        totalBytes: u64,
    },
    Downloading {
        downloadedBytes: u64,
        totalBytes: u64,
    },
    Extracting {
        engineId: String,
    },
    Completed {
        engineId: String,
        downloadedBytes: u64,
    },
}

pub type LocalEngineDownloadProgressCallback =
    Arc<dyn Fn(LocalEngineDownloadProgress) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct LocalEngineInstaller {
    runtimeRoot: PathBuf,
    registryStore: LocalModelRegistryStore,
    httpHost: Arc<dyn HttpHost>,
    storageHost: Option<Arc<dyn RuntimeStorageHost>>,
}

impl LocalEngineInstaller {
    /// Creates an engine installer from one runtime root directory.
    pub fn forRuntimeRoot(
        runtimeRoot: PathBuf,
        httpHost: Arc<dyn HttpHost>,
    ) -> Result<Self, LocalEngineDownloadError> {
        let registryStore = LocalModelRegistryStore::forRuntimeRoot(runtimeRoot.clone())
            .map_err(|error| LocalEngineDownloadError::Registry(error.to_string()))?;
        Ok(Self {
            runtimeRoot,
            registryStore,
            httpHost,
            storageHost: None,
        })
    }

    /// Creates an engine installer backed by the runtime storage host.
    pub fn forRuntimeStorage(
        storageHost: Arc<dyn RuntimeStorageHost>,
        httpHost: Arc<dyn HttpHost>,
    ) -> Result<Self, LocalEngineDownloadError> {
        let runtimeRoot = storageHost.runtimeRootDir().ok_or_else(|| {
            LocalEngineDownloadError::Storage(
                "RuntimeStorageHost runtime root is not configured".to_string(),
            )
        })?;
        let registryStore = LocalModelRegistryStore::forRuntimeStorage(storageHost.clone());
        Ok(Self {
            runtimeRoot,
            registryStore,
            httpHost,
            storageHost: Some(storageHost),
        })
    }

    /// Installs one engine artifact for an exact platform target.
    pub fn install(
        &self,
        request: LocalEngineInstallRequest,
        control: HttpDownloadControl,
        onProgress: LocalEngineDownloadProgressCallback,
    ) -> Result<LocalEngineInstallResult, LocalEngineDownloadError> {
        validatedStorageSegment("engineId", &request.manifest.id)
            .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
        validatedStorageSegment("version", &request.manifest.version)
            .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
        let artifact = request
            .manifest
            .artifactForTarget(&request.target)
            .cloned()
            .ok_or_else(|| {
                LocalEngineDownloadError::ArtifactUnavailable(request.target.storageSegment())
            })?;
        onProgress(LocalEngineDownloadProgress::Started {
            engineId: request.manifest.id.clone(),
            totalBytes: match artifact.delivery {
                LocalEngineDelivery::DownloadArchive => artifact.byteSize,
                LocalEngineDelivery::Embedded => 0,
            },
        });

        if artifact.delivery == LocalEngineDelivery::Embedded {
            return self.registerEmbeddedEngine(request, artifact, onProgress);
        }

        let storagePath = buildLocalEngineStoragePath(
            &request.manifest.id,
            &request.manifest.version,
            &request.target,
        )
        .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
        let installDir = runtimeLayoutPath(&self.runtimeRoot, &storagePath)?;
        let parent = installDir
            .parent()
            .ok_or_else(|| LocalEngineDownloadError::Storage(storagePath.clone()))?;
        self.createDirectory(parent)?;
        let archivePath = parent.join(format!(
            ".{}-{}.archive.part",
            request.manifest.id,
            request.target.storageSegment()
        ));
        let stagingDir = parent.join(format!(
            ".{}-{}.installing",
            request.manifest.id,
            request.target.storageSegment()
        ));
        self.removePath(&archivePath)?;
        self.removePath(&stagingDir)?;
        let progressCallback = engineDownloadProgressCallback(onProgress.clone());
        let downloadResult = self.httpHost.downloadFiles(
            HttpDownloadRequest {
                downloadId: format!(
                    "local-engine:{}@{}#{}:{}",
                    request.manifest.id,
                    request.manifest.version,
                    request.target.storageSegment(),
                    request.installedAtMs
                ),
                files: vec![HttpDownloadFileRequest {
                    fileId: request.manifest.id.clone(),
                    url: artifact.url.clone(),
                    targetPath: nativePathString(&archivePath)?,
                    headers: Vec::new(),
                    expectedBytes: artifact.byteSize,
                }],
                maxConcurrency: 1,
                connectTimeoutSeconds: 30,
                readTimeoutSeconds: 3_600,
                followRedirects: true,
                ignoreSsl: false,
                proxyHost: String::new(),
                proxyPort: 0,
            },
            control,
            progressCallback,
        );
        if let Err(error) = downloadResult {
            self.removePath(&archivePath)?;
            self.removePath(&stagingDir)?;
            return Err(LocalEngineDownloadError::Http(error.to_string()));
        }
        self.verifyArchive(&archivePath, artifact.byteSize, &artifact.sha256)?;
        onProgress(LocalEngineDownloadProgress::Extracting {
            engineId: request.manifest.id.clone(),
        });
        self.createDirectory(&stagingDir)?;
        self.extractArchive(&archivePath, &stagingDir, &artifact.archiveFormat)?;
        self.verifyEngineRuntimeFiles(&stagingDir, &artifact)?;
        self.commitStagingDirectory(&stagingDir, &installDir)?;
        self.removePath(&archivePath)?;

        let installedEngine = InstalledLocalEngine {
            manifest: request.manifest,
            artifact,
            storagePath,
            installedAtMs: request.installedAtMs,
            verifiedAtMs: Some(request.installedAtMs),
        };
        let mut registry = self
            .registryStore
            .read()
            .map_err(|error| LocalEngineDownloadError::Registry(error.to_string()))?;
        registry.upsertEngine(installedEngine.clone());
        self.registryStore
            .write(&registry)
            .map_err(|error| LocalEngineDownloadError::Registry(error.to_string()))?;
        onProgress(LocalEngineDownloadProgress::Completed {
            engineId: installedEngine.manifest.id.clone(),
            downloadedBytes: installedEngine.artifact.byteSize,
        });
        Ok(LocalEngineInstallResult {
            downloadedBytes: installedEngine.artifact.byteSize,
            installedEngine,
        })
    }

    /// Registers one engine supplied by the application build without downloading runtime files.
    fn registerEmbeddedEngine(
        &self,
        request: LocalEngineInstallRequest,
        artifact: crate::LocalEngineManifest::LocalEngineArtifact,
        onProgress: LocalEngineDownloadProgressCallback,
    ) -> Result<LocalEngineInstallResult, LocalEngineDownloadError> {
        let storagePath = buildLocalEngineStoragePath(
            &request.manifest.id,
            &request.manifest.version,
            &request.target,
        )
        .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
        let installedEngine = InstalledLocalEngine {
            manifest: request.manifest,
            artifact,
            storagePath,
            installedAtMs: request.installedAtMs,
            verifiedAtMs: Some(request.installedAtMs),
        };
        let mut registry = self
            .registryStore
            .read()
            .map_err(|error| LocalEngineDownloadError::Registry(error.to_string()))?;
        registry.upsertEngine(installedEngine.clone());
        self.registryStore
            .write(&registry)
            .map_err(|error| LocalEngineDownloadError::Registry(error.to_string()))?;
        onProgress(LocalEngineDownloadProgress::Completed {
            engineId: installedEngine.manifest.id.clone(),
            downloadedBytes: 0,
        });
        Ok(LocalEngineInstallResult {
            installedEngine,
            downloadedBytes: 0,
        })
    }

    /// Verifies runtime files for one installed engine.
    pub fn verifyInstalledEngine(
        &self,
        engineId: &str,
        version: &str,
        target: &LocalPlatformTarget,
        verifiedAtMs: i64,
    ) -> Result<InstalledLocalEngine, LocalEngineDownloadError> {
        let mut registry = self
            .registryStore
            .read()
            .map_err(|error| LocalEngineDownloadError::Registry(error.to_string()))?;
        let index = registry
            .installedEngines
            .iter()
            .position(|engine| {
                engine.manifest.id == engineId.trim()
                    && engine.manifest.version == version.trim()
                    && engine.artifact.target == *target
            })
            .ok_or_else(|| {
                LocalEngineDownloadError::EngineNotInstalled(format!(
                    "{}@{}#{}",
                    engineId.trim(),
                    version.trim(),
                    target.storageSegment()
                ))
            })?;
        if registry.installedEngines[index].artifact.delivery
            == LocalEngineDelivery::DownloadArchive
        {
            let installDir = runtimeLayoutPath(
                &self.runtimeRoot,
                &registry.installedEngines[index].storagePath,
            )?;
            self.verifyEngineRuntimeFiles(&installDir, &registry.installedEngines[index].artifact)?;
        }
        registry.installedEngines[index].verifiedAtMs = Some(verifiedAtMs);
        let installedEngine = registry.installedEngines[index].clone();
        self.registryStore
            .write(&registry)
            .map_err(|error| LocalEngineDownloadError::Registry(error.to_string()))?;
        Ok(installedEngine)
    }

    /// Deletes one installed engine target and its registry record.
    pub fn deleteInstalledEngine(
        &self,
        engineId: &str,
        version: &str,
        target: &LocalPlatformTarget,
    ) -> Result<LocalEngineDeleteResult, LocalEngineDownloadError> {
        let mut registry = self
            .registryStore
            .read()
            .map_err(|error| LocalEngineDownloadError::Registry(error.to_string()))?;
        let installedEngine = registry
            .getInstalledEngine(engineId, version, target)
            .cloned()
            .ok_or_else(|| {
                LocalEngineDownloadError::EngineNotInstalled(format!(
                    "{}@{}#{}",
                    engineId.trim(),
                    version.trim(),
                    target.storageSegment()
                ))
            })?;
        if installedEngine.artifact.delivery == LocalEngineDelivery::DownloadArchive {
            let installDir = runtimeLayoutPath(&self.runtimeRoot, &installedEngine.storagePath)?;
            self.removePath(&installDir)?;
        }
        registry.removeEngine(engineId, version, target);
        self.registryStore
            .write(&registry)
            .map_err(|error| LocalEngineDownloadError::Registry(error.to_string()))?;
        Ok(LocalEngineDeleteResult {
            removedStoragePath: installedEngine.storagePath.clone(),
            removedEngine: installedEngine,
        })
    }

    /// Reads the shared model and engine registry snapshot.
    pub fn readRegistry(&self) -> Result<LocalModelRegistrySnapshot, LocalEngineDownloadError> {
        self.registryStore
            .read()
            .map_err(|error| LocalEngineDownloadError::Registry(error.to_string()))
    }

    /// Creates a directory for native storage targets.
    fn createDirectory(&self, path: &Path) -> Result<(), LocalEngineDownloadError> {
        match &self.storageHost {
            Some(_) => Ok(()),
            None => fs::create_dir_all(path)
                .map_err(|error| LocalEngineDownloadError::Storage(error.to_string())),
        }
    }

    /// Removes a file or directory from the configured storage backend.
    fn removePath(&self, path: &Path) -> Result<(), LocalEngineDownloadError> {
        match &self.storageHost {
            Some(storageHost) => storageHost
                .delete(&storagePathString(path), true)
                .map_err(|error| LocalEngineDownloadError::Storage(error.to_string())),
            None => removeNativePath(path),
        }
    }

    /// Verifies one downloaded archive against its declared size and SHA-256.
    fn verifyArchive(
        &self,
        archivePath: &Path,
        declaredBytes: u64,
        declaredSha256: &str,
    ) -> Result<(), LocalEngineDownloadError> {
        match &self.storageHost {
            Some(storageHost) => verifyArchiveBytes(
                &storageHost
                    .readBytes(&storagePathString(archivePath))
                    .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?,
                declaredBytes,
                declaredSha256,
            ),
            None => {
                let metadata = fs::metadata(archivePath)
                    .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
                if metadata.len() != declaredBytes {
                    return Err(LocalEngineDownloadError::SizeMismatch);
                }
                let mut file = fs::File::open(archivePath)
                    .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
                verifyArchiveReader(&mut file, declaredSha256)
            }
        }
    }

    /// Extracts one verified engine archive into a staging directory.
    fn extractArchive(
        &self,
        archivePath: &Path,
        stagingDir: &Path,
        archiveFormat: &LocalEngineArchiveFormat,
    ) -> Result<(), LocalEngineDownloadError> {
        match (&self.storageHost, archiveFormat) {
            (Some(storageHost), LocalEngineArchiveFormat::TarBz2) => {
                extractTarBz2ArchiveToStorage(archivePath, stagingDir, storageHost)
            }
            (None, LocalEngineArchiveFormat::TarBz2) => {
                extractTarBz2ArchiveToNative(archivePath, stagingDir)
            }
        }
    }

    /// Verifies the executable or native-library files declared by one engine artifact.
    fn verifyEngineRuntimeFiles(
        &self,
        installDir: &Path,
        artifact: &crate::LocalEngineManifest::LocalEngineArtifact,
    ) -> Result<(), LocalEngineDownloadError> {
        let requiredPaths = requiredEngineRuntimeFiles(installDir, artifact);
        for path in requiredPaths {
            let exists = match &self.storageHost {
                Some(storageHost) => storageHost
                    .exists(&storagePathString(&path))
                    .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?,
                None => path.is_file(),
            };
            if !exists {
                return Err(LocalEngineDownloadError::RuntimeFileMissing(
                    storagePathString(&path),
                ));
            }
        }
        Ok(())
    }

    /// Promotes a verified staging directory into the final engine install path.
    fn commitStagingDirectory(
        &self,
        stagingDir: &Path,
        installDir: &Path,
    ) -> Result<(), LocalEngineDownloadError> {
        match &self.storageHost {
            Some(storageHost) => {
                self.removePath(installDir)?;
                copyStorageDirectory(storageHost, stagingDir, installDir)?;
                self.removePath(stagingDir)
            }
            None => {
                removeNativePath(installDir)?;
                fs::rename(stagingDir, installDir)
                    .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))
            }
        }
    }
}

/// Maps host download progress into engine installation progress events.
fn engineDownloadProgressCallback(
    onProgress: LocalEngineDownloadProgressCallback,
) -> operit_host_api::HttpDownloadProgressCallback {
    Arc::new(move |progress: HttpDownloadProgress| match progress.state {
        HttpDownloadProgressState::Started => {
            onProgress(LocalEngineDownloadProgress::Downloading {
                downloadedBytes: 0,
                totalBytes: progress.fileTotalBytes,
            });
        }
        HttpDownloadProgressState::Downloading => {
            onProgress(LocalEngineDownloadProgress::Downloading {
                downloadedBytes: progress.fileDownloadedBytes,
                totalBytes: progress.fileTotalBytes,
            });
        }
        HttpDownloadProgressState::Completed => {
            onProgress(LocalEngineDownloadProgress::Downloading {
                downloadedBytes: progress.fileDownloadedBytes,
                totalBytes: progress.fileTotalBytes,
            });
        }
    })
}

/// Converts one native archive path into the host download contract representation.
fn nativePathString(path: &Path) -> Result<String, LocalEngineDownloadError> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| LocalEngineDownloadError::Storage(path.to_string_lossy().to_string()))
}

/// Verifies archive bytes against size and SHA-256 declarations.
fn verifyArchiveBytes(
    bytes: &[u8],
    declaredBytes: u64,
    declaredSha256: &str,
) -> Result<(), LocalEngineDownloadError> {
    if bytes.len() as u64 != declaredBytes {
        return Err(LocalEngineDownloadError::SizeMismatch);
    }
    verifyArchiveReader(&mut Cursor::new(bytes), declaredSha256)
}

/// Verifies archive content from a reader against its SHA-256 declaration.
fn verifyArchiveReader(
    reader: &mut dyn Read,
    declaredSha256: &str,
) -> Result<(), LocalEngineDownloadError> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let calculated = format!("{:x}", hasher.finalize());
    if !calculated.eq_ignore_ascii_case(declaredSha256.trim()) {
        return Err(LocalEngineDownloadError::ChecksumMismatch);
    }
    Ok(())
}

/// Extracts a Tar+Bzip2 engine archive into a native staging directory.
fn extractTarBz2ArchiveToNative(
    archivePath: &Path,
    stagingDir: &Path,
) -> Result<(), LocalEngineDownloadError> {
    let file = fs::File::open(archivePath)
        .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
    let decoder = DecoderReader::new(file);
    let mut archive = tar::Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
    for entry in entries {
        let mut entry =
            entry.map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
        let path = entry
            .path()
            .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
        validateArchivePath(&path)?;
        entry
            .unpack_in(stagingDir)
            .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
    }
    Ok(())
}

/// Extracts a Tar+Bzip2 engine archive into runtime storage.
fn extractTarBz2ArchiveToStorage(
    archivePath: &Path,
    stagingDir: &Path,
    storageHost: &Arc<dyn RuntimeStorageHost>,
) -> Result<(), LocalEngineDownloadError> {
    let archiveBytes = storageHost
        .readBytes(&storagePathString(archivePath))
        .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
    let decoder = DecoderReader::new(Cursor::new(archiveBytes));
    let mut archive = tar::Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
    for entry in entries {
        let mut entry =
            entry.map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
        let path = entry
            .path()
            .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?
            .to_path_buf();
        validateArchivePath(&path)?;
        if entry.header().entry_type().is_dir() {
            continue;
        }
        let mut content = Vec::new();
        entry
            .read_to_end(&mut content)
            .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
        let target = stagingDir.join(path);
        storageHost
            .writeBytes(&storagePathString(&target), &content)
            .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
    }
    Ok(())
}

/// Rejects archive paths that can escape the extraction directory.
fn validateArchivePath(path: &Path) -> Result<(), LocalEngineDownloadError> {
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(LocalEngineDownloadError::InvalidArchiveEntry(
            path.to_string_lossy().to_string(),
        ));
    }
    Ok(())
}

/// Builds required runtime file paths for one engine artifact.
fn requiredEngineRuntimeFiles(
    installDir: &Path,
    artifact: &crate::LocalEngineManifest::LocalEngineArtifact,
) -> Vec<PathBuf> {
    if artifact.delivery == LocalEngineDelivery::Embedded {
        return Vec::new();
    }
    let runtimeRoot = installDir.join(&artifact.archiveRoot);
    let mut requiredPaths = Vec::new();
    match &artifact.sttExecutable {
        Some(path) => requiredPaths.push(runtimeRoot.join(path)),
        None => {}
    }
    match &artifact.ttsExecutable {
        Some(path) => requiredPaths.push(runtimeRoot.join(path)),
        None => {}
    }
    match &artifact.androidLibraryDir {
        Some(path) => {
            let libraryDir = runtimeRoot.join(path);
            requiredPaths.push(libraryDir.join("libonnxruntime.so"));
            requiredPaths.push(libraryDir.join("libsherpa-onnx-c-api.so"));
            requiredPaths.push(libraryDir.join("libsherpa-onnx-jni.so"));
        }
        None => {}
    }
    match &artifact.ohosLibraryDir {
        Some(path) => {
            let libraryDir = runtimeRoot.join(path);
            requiredPaths.push(libraryDir.join("libonnxruntime.so"));
            requiredPaths.push(libraryDir.join("libsherpa-onnx-c-api.so"));
        }
        None => {}
    }
    match &artifact.iosFrameworkDir {
        Some(path) => {
            let frameworkDir = runtimeRoot.join(path);
            requiredPaths.push(frameworkDir.join("Info.plist"));
            requiredPaths.push(frameworkDir.join("ios-arm64").join("libsherpa-onnx.a"));
            requiredPaths.push(
                frameworkDir
                    .join("ios-arm64")
                    .join("Headers")
                    .join("sherpa-onnx")
                    .join("c-api")
                    .join("c-api.h"),
            );
        }
        None => {}
    }
    match &artifact.webRuntimeDir {
        Some(path) => {
            let runtimeDir = joinDeclaredRuntimePath(&runtimeRoot, path);
            requiredPaths.push(runtimeDir.join("sherpa-onnx-vad.js"));
            requiredPaths.push(runtimeDir.join("sherpa-onnx-wasm-main-vad.js"));
            requiredPaths.push(runtimeDir.join("sherpa-onnx-wasm-main-vad.wasm"));
        }
        None => {}
    }
    requiredPaths
}

/// Joins a declared runtime path while preserving current-directory semantics.
fn joinDeclaredRuntimePath(root: &Path, relativePath: &str) -> PathBuf {
    if relativePath.trim() == "." {
        return root.to_path_buf();
    }
    root.join(relativePath)
}

/// Maps a runtime layout path into one native runtime root.
fn runtimeLayoutPath(
    runtimeRoot: &Path,
    storagePath: &str,
) -> Result<PathBuf, LocalEngineDownloadError> {
    let relative = storagePath
        .trim()
        .strip_prefix("runtime/")
        .ok_or_else(|| LocalEngineDownloadError::Storage(storagePath.to_string()))?;
    Ok(runtimeRoot.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR)))
}

/// Copies every staged runtime-storage file into a final directory.
fn copyStorageDirectory(
    storageHost: &Arc<dyn RuntimeStorageHost>,
    sourceDir: &Path,
    targetDir: &Path,
) -> Result<(), LocalEngineDownloadError> {
    let sourcePrefix = storagePathString(sourceDir);
    let sourcePrefixWithSlash = format!("{}/", sourcePrefix.trim_end_matches('/'));
    let entries = storageHost
        .list(&sourcePrefix)
        .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
    for entry in entries {
        if entry.isDirectory {
            continue;
        }
        let relative = entry
            .path
            .strip_prefix(&sourcePrefixWithSlash)
            .ok_or_else(|| LocalEngineDownloadError::Storage(entry.path.clone()))?;
        let targetPath = storagePathString(&targetDir.join(relative));
        let content = storageHost
            .readBytes(&entry.path)
            .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
        storageHost
            .writeBytes(&targetPath, &content)
            .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
    }
    Ok(())
}

/// Converts a path into the virtual runtime-storage representation.
fn storagePathString(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Removes one temporary file or directory used by native engine installation.
fn removeNativePath(path: &Path) -> Result<(), LocalEngineDownloadError> {
    if !path.exists() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))?;
    if metadata.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))
    } else {
        fs::remove_file(path).map_err(|error| LocalEngineDownloadError::Storage(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    use operit_host_api::{HostResult, HttpDownloadResult, HttpRequestData, HttpResponseData};

    use crate::LocalEngineCatalog::LocalEngineCatalog;
    use crate::LocalEngineManifest::{LocalArchitecture, LocalPlatform};

    struct RejectingHttpHost;

    impl HttpHost for RejectingHttpHost {
        /// Rejects buffered HTTP because embedded engine installation must remain offline.
        fn executeHttpRequest(&self, _request: HttpRequestData) -> HostResult<HttpResponseData> {
            panic!("embedded engine installation called executeHttpRequest")
        }

        /// Rejects downloads because embedded engine installation must remain offline.
        fn downloadFiles(
            &self,
            _request: HttpDownloadRequest,
            _control: HttpDownloadControl,
            _onProgress: operit_host_api::HttpDownloadProgressCallback,
        ) -> HostResult<HttpDownloadResult> {
            panic!("embedded engine installation called downloadFiles")
        }
    }

    /// Verifies embedded engine lifecycle operations only update the registry.
    #[test]
    fn embeddedEngineLifecycleDoesNotDownloadRuntimeFiles() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let runtimeRoot = std::env::temp_dir().join(format!(
            "operit-local-engine-embedded-test-{}-{timestamp}",
            std::process::id()
        ));
        fs::create_dir_all(&runtimeRoot).unwrap();
        let installer =
            LocalEngineInstaller::forRuntimeRoot(runtimeRoot.clone(), Arc::new(RejectingHttpHost))
                .unwrap();
        let target = LocalPlatformTarget {
            platform: LocalPlatform::Ios,
            architecture: LocalArchitecture::Aarch64,
        };
        let manifest = LocalEngineCatalog::sherpaOnnx();
        let result = installer
            .install(
                LocalEngineInstallRequest {
                    manifest: manifest.clone(),
                    target: target.clone(),
                    installedAtMs: 42,
                },
                HttpDownloadControl::new(),
                Arc::new(|_| {}),
            )
            .unwrap();

        assert_eq!(result.downloadedBytes, 0);
        assert_eq!(
            result.installedEngine.artifact.delivery,
            LocalEngineDelivery::Embedded
        );
        assert!(installer
            .verifyInstalledEngine(&manifest.id, &manifest.version, &target, 43)
            .is_ok());
        assert!(installer
            .deleteInstalledEngine(&manifest.id, &manifest.version, &target)
            .is_ok());
        fs::remove_dir_all(runtimeRoot).unwrap();
    }
}
