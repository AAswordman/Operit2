#[cfg(test)]
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

use crate::LocalModelManifest::{
    LocalModelArchive, LocalModelArchiveFormat, LocalModelFile, LocalModelInstallSource,
    LocalModelManifest,
};
use crate::LocalModelRegistry::{InstalledLocalModel, LocalModelRegistrySnapshot};
use crate::LocalModelRegistryStore::LocalModelRegistryStore;
use crate::LocalModelStorage::{
    buildLocalModelStoragePath, validatedStorageSegment, LocalModelStorageError,
};

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum LocalModelDownloadError {
    #[error("storage error: {0}")]
    Storage(String),
    #[error("http error: {0}")]
    Http(String),
    #[error("manifest source not found: {sourceId}")]
    SourceNotFound { sourceId: String },
    #[error("installed local model not found: {modelId}@{version}")]
    ModelNotInstalled { modelId: String, version: String },
    #[error("downloaded file size mismatch: {path}")]
    SizeMismatch { path: String },
    #[error("downloaded file checksum mismatch: {path}")]
    ChecksumMismatch { path: String },
    #[error("invalid local model archive entry: {path}")]
    InvalidArchiveEntry { path: String },
    #[error("invalid local file name: {path}")]
    InvalidFileName { path: String },
    #[error("invalid model storage path: {0}")]
    InvalidStoragePath(String),
    #[error("json error: {0}")]
    Json(String),
}

impl From<LocalModelStorageError> for LocalModelDownloadError {
    /// Converts a storage path validation error into a download error.
    fn from(value: LocalModelStorageError) -> Self {
        Self::InvalidStoragePath(value.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelInstallRequest {
    pub manifest: LocalModelManifest,
    pub installedAtMs: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelInstallResult {
    pub installedModel: InstalledLocalModel,
    pub downloadedBytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelVerifyResult {
    pub installedModel: InstalledLocalModel,
    pub verifiedBytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelDeleteResult {
    pub removedModel: InstalledLocalModel,
    pub removedStoragePath: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub enum LocalModelDownloadProgress {
    Started {
        modelId: String,
        totalBytes: u64,
    },
    FileStarted {
        relativePath: String,
        totalFileBytes: u64,
    },
    FileProgress {
        relativePath: String,
        fileBytes: u64,
        totalFileBytes: u64,
        downloadedBytes: u64,
        totalBytes: u64,
    },
    FileDownloaded {
        relativePath: String,
        downloadedBytes: u64,
        totalBytes: u64,
    },
    Extracting {
        modelId: String,
    },
    Completed {
        modelId: String,
        downloadedBytes: u64,
    },
}

pub type LocalModelDownloadProgressCallback =
    Arc<dyn Fn(LocalModelDownloadProgress) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct LocalModelInstaller {
    runtimeRoot: PathBuf,
    registryStore: LocalModelRegistryStore,
    httpHost: Arc<dyn HttpHost>,
    storageHost: Option<Arc<dyn RuntimeStorageHost>>,
}

impl LocalModelInstaller {
    /// Creates a filesystem-backed installer for native test coverage.
    #[cfg(test)]
    pub fn forRuntimeRoot(
        runtimeRoot: PathBuf,
        httpHost: Arc<dyn HttpHost>,
    ) -> Result<Self, LocalModelDownloadError> {
        let registryStore = LocalModelRegistryStore::forRuntimeRoot(runtimeRoot.clone())
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
        Ok(Self {
            runtimeRoot,
            registryStore,
            httpHost,
            storageHost: None,
        })
    }

    /// Creates a filesystem-backed installer for native test coverage.
    #[cfg(test)]
    pub fn new(
        runtimeRoot: PathBuf,
        registryPath: PathBuf,
        httpHost: Arc<dyn HttpHost>,
    ) -> Result<Self, LocalModelDownloadError> {
        Ok(Self {
            runtimeRoot,
            registryStore: LocalModelRegistryStore::new(registryPath),
            httpHost,
            storageHost: None,
        })
    }

    /// Creates an installer backed by the runtime storage host.
    pub fn forRuntimeStorage(
        storageHost: Arc<dyn RuntimeStorageHost>,
        httpHost: Arc<dyn HttpHost>,
    ) -> Result<Self, LocalModelDownloadError> {
        let runtimeRoot = storageHost.runtimeRootDir().ok_or_else(|| {
            LocalModelDownloadError::Storage(
                "RuntimeStorageHost runtime root is not configured".to_string(),
            )
        })?;
        Ok(Self {
            runtimeRoot,
            registryStore: LocalModelRegistryStore::forRuntimeStorage(storageHost.clone()),
            httpHost,
            storageHost: Some(storageHost),
        })
    }

    /// Installs a manifest and records it in the local model registry file.
    pub fn install(
        &self,
        request: LocalModelInstallRequest,
        control: HttpDownloadControl,
        onProgress: LocalModelDownloadProgressCallback,
    ) -> Result<LocalModelInstallResult, LocalModelDownloadError> {
        validateManifestStorageSegments(&request.manifest)?;
        let totalBytes = request.manifest.declaredByteSize();
        onProgress(LocalModelDownloadProgress::Started {
            modelId: request.manifest.id.clone(),
            totalBytes,
        });
        let storagePath = buildLocalModelStoragePath(
            &request.manifest.kind,
            &request.manifest.engine,
            &request.manifest.id,
            &request.manifest.version,
        )?;
        let installDir = self.runtimeStoragePath(&storagePath)?;
        let downloadedBytes = match &request.manifest.installSource {
            LocalModelInstallSource::Files => self.installFileSource(
                &request.manifest,
                request.installedAtMs,
                &installDir,
                control,
                onProgress.clone(),
            )?,
            LocalModelInstallSource::Archives { archives } => self.installArchiveSource(
                &request.manifest,
                request.installedAtMs,
                &installDir,
                archives,
                control,
                onProgress.clone(),
            )?,
        };

        let installedModel = InstalledLocalModel {
            manifest: request.manifest,
            storagePath,
            installedAtMs: request.installedAtMs,
            verifiedAtMs: Some(request.installedAtMs),
        };
        let mut registry = self.readRegistry()?;
        registry.upsert(installedModel.clone());
        self.writeRegistry(&registry)?;
        onProgress(LocalModelDownloadProgress::Completed {
            modelId: installedModel.manifest.id.clone(),
            downloadedBytes,
        });
        Ok(LocalModelInstallResult {
            installedModel,
            downloadedBytes,
        })
    }

    /// Installs a manifest whose source files are downloaded directly.
    fn installFileSource(
        &self,
        manifest: &LocalModelManifest,
        installedAtMs: i64,
        installDir: &Path,
        control: HttpDownloadControl,
        onProgress: LocalModelDownloadProgressCallback,
    ) -> Result<u64, LocalModelDownloadError> {
        self.createDirectory(installDir)?;

        let mut downloadFiles = Vec::new();
        let mut downloadedTargets = Vec::new();
        for file in &manifest.files {
            let source = manifest.sourceForFile(file).ok_or_else(|| {
                LocalModelDownloadError::SourceNotFound {
                    sourceId: file.sourceId.clone(),
                }
            })?;
            let url = source.fileUrl(&file.relativePath);
            let target = installDir.join(normalizedRelativePath(&file.relativePath)?);
            let tempTarget = downloadTempPath(&target)?;
            downloadFiles.push(HttpDownloadFileRequest {
                fileId: file.relativePath.clone(),
                url,
                targetPath: nativePathString(&tempTarget)?,
                headers: Vec::new(),
                expectedBytes: file.byteSize,
            });
            downloadedTargets.push((file.clone(), tempTarget, target));
        }
        let progressCallback = modelDownloadProgressCallback(onProgress);
        let downloadResult = self
            .httpHost
            .downloadFiles(
                HttpDownloadRequest {
                    downloadId: format!("local-model:{}:{installedAtMs}", manifest.registryKey()),
                    maxConcurrency: downloadFiles.len().min(4),
                    files: downloadFiles,
                    connectTimeoutSeconds: 30,
                    readTimeoutSeconds: 3_600,
                    followRedirects: true,
                    ignoreSsl: false,
                    proxyHost: String::new(),
                    proxyPort: 0,
                },
                control,
                progressCallback,
            )
            .map_err(|error| LocalModelDownloadError::Http(error.to_string()));
        let downloadResult = match downloadResult {
            Ok(result) => result,
            Err(error) => {
                self.removeDownloadedTargets(&downloadedTargets)?;
                return Err(error);
            }
        };
        for (file, tempTarget, _) in &downloadedTargets {
            if let Err(error) = self.verifyInstalledFile(file, tempTarget) {
                self.removeDownloadedTargets(&downloadedTargets)?;
                return Err(error);
            }
        }
        for (_, tempTarget, target) in &downloadedTargets {
            self.replaceFileWithVerifiedDownload(tempTarget, target)?;
        }
        Ok(downloadResult.downloadedBytes)
    }

    /// Installs a manifest whose source files are expanded from archives.
    fn installArchiveSource(
        &self,
        manifest: &LocalModelManifest,
        installedAtMs: i64,
        installDir: &Path,
        archives: &[LocalModelArchive],
        control: HttpDownloadControl,
        onProgress: LocalModelDownloadProgressCallback,
    ) -> Result<u64, LocalModelDownloadError> {
        let parent = installDir
            .parent()
            .ok_or_else(|| LocalModelDownloadError::Storage(installDir.display().to_string()))?;
        self.createDirectory(parent)?;
        let stagingDir = parent.join(format!(".{}-{}.installing", manifest.id, manifest.version));
        self.removePath(&stagingDir)?;

        let mut downloadFiles = Vec::new();
        let mut archiveTargets = Vec::new();
        for archive in archives {
            let source = manifest.sourceForArchive(archive).ok_or_else(|| {
                LocalModelDownloadError::SourceNotFound {
                    sourceId: archive.sourceId.clone(),
                }
            })?;
            let archivePath = archiveDownloadPath(parent, archive)?;
            self.removePath(&archivePath)?;
            downloadFiles.push(HttpDownloadFileRequest {
                fileId: archive.archiveId.clone(),
                url: source.fileUrl(&archive.relativePath),
                targetPath: nativePathString(&archivePath)?,
                headers: Vec::new(),
                expectedBytes: archive.byteSize,
            });
            archiveTargets.push((archive.clone(), archivePath));
        }

        let progressCallback = modelDownloadProgressCallback(onProgress.clone());
        let downloadResult = self.httpHost.downloadFiles(
            HttpDownloadRequest {
                downloadId: format!("local-model:{}:{installedAtMs}", manifest.registryKey()),
                maxConcurrency: downloadFiles.len().min(4),
                files: downloadFiles,
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
        let downloadResult = match downloadResult {
            Ok(result) => result,
            Err(error) => {
                self.removeArchiveTargets(&archiveTargets)?;
                self.removePath(&stagingDir)?;
                return Err(LocalModelDownloadError::Http(error.to_string()));
            }
        };

        for (archive, archivePath) in &archiveTargets {
            if let Err(error) = self.verifyDownloadedArchive(archive, archivePath) {
                self.removeArchiveTargets(&archiveTargets)?;
                self.removePath(&stagingDir)?;
                return Err(error);
            }
        }
        onProgress(LocalModelDownloadProgress::Extracting {
            modelId: manifest.id.clone(),
        });
        self.createDirectory(&stagingDir)?;
        for (archive, archivePath) in &archiveTargets {
            if let Err(error) =
                self.extractModelArchive(archivePath, &stagingDir, &archive.archiveFormat)
            {
                self.removeArchiveTargets(&archiveTargets)?;
                self.removePath(&stagingDir)?;
                return Err(error);
            }
        }
        if let Err(error) = self.verifyInstalledFiles(manifest, &stagingDir) {
            self.removeArchiveTargets(&archiveTargets)?;
            self.removePath(&stagingDir)?;
            return Err(error);
        }
        self.commitStagingDirectory(&stagingDir, installDir)?;
        self.removeArchiveTargets(&archiveTargets)?;
        Ok(downloadResult.downloadedBytes)
    }

    /// Verifies files for an installed model and updates its verification time.
    pub fn verifyInstalledModel(
        &self,
        modelId: &str,
        version: &str,
        verifiedAtMs: i64,
    ) -> Result<LocalModelVerifyResult, LocalModelDownloadError> {
        let modelId = modelId.trim();
        let version = version.trim();
        let mut registry = self.readRegistry()?;
        let index = registry
            .installedModels
            .iter()
            .position(|model| model.manifest.id == modelId && model.manifest.version == version)
            .ok_or_else(|| LocalModelDownloadError::ModelNotInstalled {
                modelId: modelId.to_string(),
                version: version.to_string(),
            })?;
        let installDir = self.runtimeStoragePath(&registry.installedModels[index].storagePath)?;
        let manifest = registry.installedModels[index].manifest.clone();
        validateManifestStorageSegments(&manifest)?;
        let verifiedBytes = self.verifyInstalledFiles(&manifest, &installDir)?;
        registry.installedModels[index].verifiedAtMs = Some(verifiedAtMs);
        let installedModel = registry.installedModels[index].clone();
        self.writeRegistry(&registry)?;
        Ok(LocalModelVerifyResult {
            installedModel,
            verifiedBytes,
        })
    }

    /// Deletes one installed model directory and removes its registry record.
    pub fn deleteInstalledModel(
        &self,
        modelId: &str,
        version: &str,
    ) -> Result<LocalModelDeleteResult, LocalModelDownloadError> {
        let modelId = modelId.trim();
        let version = version.trim();
        let mut registry = self.readRegistry()?;
        let installedModel = registry
            .getInstalledModel(modelId, version)
            .cloned()
            .ok_or_else(|| LocalModelDownloadError::ModelNotInstalled {
                modelId: modelId.to_string(),
                version: version.to_string(),
            })?;
        let installDir = self.runtimeStoragePath(&installedModel.storagePath)?;
        self.removePath(&installDir)?;
        registry.remove(modelId, version);
        self.writeRegistry(&registry)?;
        Ok(LocalModelDeleteResult {
            removedStoragePath: installedModel.storagePath.clone(),
            removedModel: installedModel,
        })
    }

    /// Reads the local model registry snapshot from disk.
    pub fn readRegistry(&self) -> Result<LocalModelRegistrySnapshot, LocalModelDownloadError> {
        self.registryStore
            .read()
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))
    }

    /// Writes the supplied local model registry snapshot to disk.
    pub fn writeRegistry(
        &self,
        snapshot: &LocalModelRegistrySnapshot,
    ) -> Result<(), LocalModelDownloadError> {
        self.registryStore
            .write(snapshot)
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))
    }

    /// Verifies that the installer has a runtime storage host.
    fn createDirectory(&self, path: &Path) -> Result<(), LocalModelDownloadError> {
        self.storageHost
            .as_ref()
            .ok_or_else(|| LocalModelDownloadError::Storage(storagePathString(path)))?;
        Ok(())
    }

    /// Removes temporary model targets after a stopped host download.
    fn removeDownloadedTargets(
        &self,
        targets: &[(LocalModelFile, PathBuf, PathBuf)],
    ) -> Result<(), LocalModelDownloadError> {
        for (_, tempTarget, _) in targets {
            self.removePath(tempTarget)?;
        }
        Ok(())
    }

    /// Removes temporary archive targets after a stopped archive installation.
    fn removeArchiveTargets(
        &self,
        targets: &[(LocalModelArchive, PathBuf)],
    ) -> Result<(), LocalModelDownloadError> {
        for (_, archivePath) in targets {
            self.removePath(archivePath)?;
        }
        Ok(())
    }

    /// Removes one file or directory path from local model staging.
    fn removePath(&self, path: &Path) -> Result<(), LocalModelDownloadError> {
        self.storageHost
            .as_ref()
            .ok_or_else(|| LocalModelDownloadError::Storage(storagePathString(path)))?
            .delete(&storagePathString(path), true)
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))
    }

    /// Verifies all files declared by one installed manifest.
    fn verifyInstalledFiles(
        &self,
        manifest: &LocalModelManifest,
        installDir: &Path,
    ) -> Result<u64, LocalModelDownloadError> {
        let mut verifiedBytes = 0u64;
        for file in &manifest.files {
            let target = installDir.join(normalizedRelativePath(&file.relativePath)?);
            verifiedBytes += self.verifyInstalledFile(file, &target)?;
        }
        Ok(verifiedBytes)
    }

    /// Verifies one installed file by size and SHA-256 checksum.
    fn verifyInstalledFile(
        &self,
        file: &LocalModelFile,
        target: &Path,
    ) -> Result<u64, LocalModelDownloadError> {
        let storageHost = self
            .storageHost
            .as_ref()
            .ok_or_else(|| LocalModelDownloadError::Storage(storagePathString(target)))?;
        verifyFileBytes(
            file,
            &storageHost
                .readBytes(&storagePathString(target))
                .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?,
        )
    }

    /// Verifies one downloaded archive by size and SHA-256 checksum.
    fn verifyDownloadedArchive(
        &self,
        archive: &LocalModelArchive,
        archivePath: &Path,
    ) -> Result<(), LocalModelDownloadError> {
        let storageHost = self
            .storageHost
            .as_ref()
            .ok_or_else(|| LocalModelDownloadError::Storage(storagePathString(archivePath)))?;
        verifyArchiveBytes(
            archive,
            &storageHost
                .readBytes(&storagePathString(archivePath))
                .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?,
        )
    }

    /// Extracts one verified model archive into a staging directory.
    fn extractModelArchive(
        &self,
        archivePath: &Path,
        stagingDir: &Path,
        archiveFormat: &LocalModelArchiveFormat,
    ) -> Result<(), LocalModelDownloadError> {
        let storageHost = self
            .storageHost
            .as_ref()
            .ok_or_else(|| LocalModelDownloadError::Storage(storagePathString(archivePath)))?;
        match archiveFormat {
            LocalModelArchiveFormat::TarBz2 => {
                extractTarBz2ArchiveToStorage(archivePath, stagingDir, storageHost)
            }
        }
    }

    /// Replaces the target file with a verified temporary download.
    fn replaceFileWithVerifiedDownload(
        &self,
        tempTarget: &Path,
        target: &Path,
    ) -> Result<(), LocalModelDownloadError> {
        let storageHost = self
            .storageHost
            .as_ref()
            .ok_or_else(|| LocalModelDownloadError::Storage(storagePathString(tempTarget)))?;
        let content = storageHost
            .readBytes(&storagePathString(tempTarget))
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
        storageHost
            .writeBytes(&storagePathString(target), &content)
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
        self.removePath(tempTarget)
    }

    /// Promotes a verified staging directory into the final model install path.
    fn commitStagingDirectory(
        &self,
        stagingDir: &Path,
        installDir: &Path,
    ) -> Result<(), LocalModelDownloadError> {
        let storageHost = self
            .storageHost
            .as_ref()
            .ok_or_else(|| LocalModelDownloadError::Storage(storagePathString(installDir)))?;
        self.removePath(installDir)?;
        copyStorageDirectory(storageHost, stagingDir, installDir)?;
        self.removePath(stagingDir)
    }

    /// Maps a runtime storage path to the installer runtime root.
    fn runtimeStoragePath(&self, storagePath: &str) -> Result<PathBuf, LocalModelDownloadError> {
        runtimeLayoutPath(&self.runtimeRoot, storagePath)
    }
}

/// Maps a runtime-layout path into a native path under the runtime root.
fn runtimeLayoutPath(
    runtimeRoot: &Path,
    storagePath: &str,
) -> Result<PathBuf, LocalModelDownloadError> {
    let storagePath = storagePath.trim();
    let runtimePrefix = "runtime/";
    if !storagePath.starts_with(runtimePrefix) {
        return Err(LocalModelDownloadError::InvalidStoragePath(
            storagePath.to_string(),
        ));
    }
    let relative = storagePath
        .trim_start_matches(runtimePrefix)
        .replace('/', std::path::MAIN_SEPARATOR_STR);
    Ok(runtimeRoot.join(relative))
}

/// Validates model id, version, and manifest file paths before installation.
fn validateManifestStorageSegments(
    manifest: &LocalModelManifest,
) -> Result<(), LocalModelDownloadError> {
    validatedStorageSegment("modelId", &manifest.id)?;
    validatedStorageSegment("version", &manifest.version)?;
    for file in &manifest.files {
        normalizedRelativePath(&file.relativePath)?;
    }
    match &manifest.installSource {
        LocalModelInstallSource::Files => {}
        LocalModelInstallSource::Archives { archives } => {
            for archive in archives {
                validatedStorageSegment("archiveId", &archive.archiveId)?;
                normalizedRelativePath(&archive.relativePath)?;
            }
        }
    }
    Ok(())
}

/// Maps host download progress into model installation progress events.
fn modelDownloadProgressCallback(
    onProgress: LocalModelDownloadProgressCallback,
) -> operit_host_api::HttpDownloadProgressCallback {
    Arc::new(move |progress: HttpDownloadProgress| match progress.state {
        HttpDownloadProgressState::Started => {
            onProgress(LocalModelDownloadProgress::FileStarted {
                relativePath: progress.fileId,
                totalFileBytes: progress.fileTotalBytes,
            });
        }
        HttpDownloadProgressState::Downloading => {
            onProgress(LocalModelDownloadProgress::FileProgress {
                relativePath: progress.fileId,
                fileBytes: progress.fileDownloadedBytes,
                totalFileBytes: progress.fileTotalBytes,
                downloadedBytes: progress.downloadedBytes,
                totalBytes: progress.totalBytes,
            });
        }
        HttpDownloadProgressState::Completed => {
            onProgress(LocalModelDownloadProgress::FileDownloaded {
                relativePath: progress.fileId,
                downloadedBytes: progress.downloadedBytes,
                totalBytes: progress.totalBytes,
            });
        }
    })
}

/// Converts one native target path into the host download contract representation.
fn nativePathString(path: &Path) -> Result<String, LocalModelDownloadError> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| LocalModelDownloadError::InvalidFileName {
            path: path.to_string_lossy().to_string(),
        })
}

/// Builds the temporary download path for one archive.
fn archiveDownloadPath(
    parent: &Path,
    archive: &LocalModelArchive,
) -> Result<PathBuf, LocalModelDownloadError> {
    let archiveId = validatedStorageSegment("archiveId", &archive.archiveId)?;
    Ok(parent.join(format!(".{archiveId}.archive.part")))
}

/// Removes one file or directory path from native local model staging.
#[cfg(test)]
fn removeNativePath(path: &Path) -> Result<(), LocalModelDownloadError> {
    if path.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
    } else if path.exists() {
        fs::remove_file(path)
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
    }
    Ok(())
}

/// Verifies one native file by size and SHA-256 checksum.
#[cfg(test)]
fn verifyNativeFile(file: &LocalModelFile, target: &Path) -> Result<u64, LocalModelDownloadError> {
    let mut input = fs::File::open(target)
        .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
    verifyFileReader(file, &mut input)
}

/// Verifies one in-memory file by size and SHA-256 checksum.
fn verifyFileBytes(file: &LocalModelFile, bytes: &[u8]) -> Result<u64, LocalModelDownloadError> {
    if bytes.len() as u64 != file.byteSize {
        return Err(LocalModelDownloadError::SizeMismatch {
            path: file.relativePath.clone(),
        });
    }
    verifyFileReader(file, &mut Cursor::new(bytes))
}

/// Verifies one file stream by size and SHA-256 checksum.
fn verifyFileReader(
    file: &LocalModelFile,
    input: &mut dyn Read,
) -> Result<u64, LocalModelDownloadError> {
    let mut hasher = Sha256::new();
    let mut fileBytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        fileBytes += read as u64;
    }
    if fileBytes != file.byteSize {
        return Err(LocalModelDownloadError::SizeMismatch {
            path: file.relativePath.clone(),
        });
    }
    let calculated = format!("{:x}", hasher.finalize());
    if !calculated.eq_ignore_ascii_case(file.sha256.trim()) {
        return Err(LocalModelDownloadError::ChecksumMismatch {
            path: file.relativePath.clone(),
        });
    }
    Ok(fileBytes)
}

/// Verifies one native archive by size and SHA-256 checksum.
#[cfg(test)]
fn verifyNativeArchive(
    archive: &LocalModelArchive,
    archivePath: &Path,
) -> Result<(), LocalModelDownloadError> {
    let metadata = fs::metadata(archivePath)
        .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
    if metadata.len() != archive.byteSize {
        return Err(LocalModelDownloadError::SizeMismatch {
            path: archive.relativePath.clone(),
        });
    }
    let mut input = fs::File::open(archivePath)
        .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
    verifyArchiveReader(archive, &mut input)
}

/// Verifies one in-memory archive by size and SHA-256 checksum.
fn verifyArchiveBytes(
    archive: &LocalModelArchive,
    bytes: &[u8],
) -> Result<(), LocalModelDownloadError> {
    if bytes.len() as u64 != archive.byteSize {
        return Err(LocalModelDownloadError::SizeMismatch {
            path: archive.relativePath.clone(),
        });
    }
    verifyArchiveReader(archive, &mut Cursor::new(bytes))
}

/// Verifies one archive stream by SHA-256 checksum.
fn verifyArchiveReader(
    archive: &LocalModelArchive,
    input: &mut dyn Read,
) -> Result<(), LocalModelDownloadError> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let calculated = format!("{:x}", hasher.finalize());
    if !calculated.eq_ignore_ascii_case(archive.sha256.trim()) {
        return Err(LocalModelDownloadError::ChecksumMismatch {
            path: archive.relativePath.clone(),
        });
    }
    Ok(())
}

/// Extracts a Tar+Bzip2 model archive into a native staging directory.
#[cfg(test)]
fn extractTarBz2ArchiveToNative(
    archivePath: &Path,
    stagingDir: &Path,
) -> Result<(), LocalModelDownloadError> {
    let file = fs::File::open(archivePath)
        .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
    let decoder = DecoderReader::new(file);
    let mut archive = tar::Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
    for entry in entries {
        let mut entry =
            entry.map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
        let path = entry
            .path()
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
        validateArchivePath(&path)?;
        entry
            .unpack_in(stagingDir)
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
    }
    Ok(())
}

/// Extracts a Tar+Bzip2 model archive into runtime storage.
fn extractTarBz2ArchiveToStorage(
    archivePath: &Path,
    stagingDir: &Path,
    storageHost: &Arc<dyn RuntimeStorageHost>,
) -> Result<(), LocalModelDownloadError> {
    let archiveBytes = storageHost
        .readBytes(&storagePathString(archivePath))
        .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
    let decoder = DecoderReader::new(Cursor::new(archiveBytes));
    let mut archive = tar::Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
    for entry in entries {
        let mut entry =
            entry.map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
        let path = entry
            .path()
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?
            .to_path_buf();
        validateArchivePath(&path)?;
        if entry.header().entry_type().is_dir() {
            continue;
        }
        let mut content = Vec::new();
        entry
            .read_to_end(&mut content)
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
        let target = stagingDir.join(path);
        storageHost
            .writeBytes(&storagePathString(&target), &content)
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
    }
    Ok(())
}

/// Rejects archive paths that can escape the extraction directory.
fn validateArchivePath(path: &Path) -> Result<(), LocalModelDownloadError> {
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(LocalModelDownloadError::InvalidArchiveEntry {
            path: path.to_string_lossy().to_string(),
        });
    }
    Ok(())
}

/// Builds the temporary download path for one target file.
fn downloadTempPath(target: &Path) -> Result<PathBuf, LocalModelDownloadError> {
    let fileName = target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| LocalModelDownloadError::InvalidFileName {
            path: target.to_string_lossy().to_string(),
        })?;
    Ok(target.with_file_name(format!("{fileName}.download")))
}

/// Replaces a native target file with a verified temporary download.
#[cfg(test)]
fn replaceNativeFileWithVerifiedDownload(
    tempTarget: &Path,
    target: &Path,
) -> Result<(), LocalModelDownloadError> {
    if target.exists() {
        fs::remove_file(target)
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
    }
    fs::rename(tempTarget, target)
        .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))
}

/// Copies every staged runtime-storage file into a final directory.
fn copyStorageDirectory(
    storageHost: &Arc<dyn RuntimeStorageHost>,
    sourceDir: &Path,
    targetDir: &Path,
) -> Result<(), LocalModelDownloadError> {
    let sourcePrefix = storagePathString(sourceDir);
    let sourcePrefixWithSlash = format!("{}/", sourcePrefix.trim_end_matches('/'));
    let entries = storageHost
        .list(&sourcePrefix)
        .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
    for entry in entries {
        if entry.isDirectory {
            continue;
        }
        let relative = entry
            .path
            .strip_prefix(&sourcePrefixWithSlash)
            .ok_or_else(|| LocalModelDownloadError::Storage(entry.path.clone()))?;
        let targetPath = storagePathString(&targetDir.join(relative));
        let content = storageHost
            .readBytes(&entry.path)
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
        storageHost
            .writeBytes(&targetPath, &content)
            .map_err(|error| LocalModelDownloadError::Storage(error.to_string()))?;
    }
    Ok(())
}

/// Converts a path into the virtual runtime-storage representation.
fn storagePathString(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Normalizes a manifest relative file path for local installation.
fn normalizedRelativePath(relativePath: &str) -> Result<PathBuf, LocalModelDownloadError> {
    let trimmed = relativePath.trim().replace('\\', "/");
    let mut result = PathBuf::new();
    for segment in trimmed.split('/') {
        let value = validatedStorageSegment("relativePath", segment)?;
        result.push(value);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalModelManifest::{
        LocalEngineKind, LocalModelArchive, LocalModelArchiveFormat, LocalModelInstallSource,
        LocalModelKind, LocalModelSource, LocalModelSourceKind,
    };
    use operit_host_api::{
        HostError, HostResult, HttpDownloadFileResult, HttpDownloadProgressCallback,
        HttpDownloadResult, HttpRequestData, HttpResponseData,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    const FIXTURE_BYTES: &[u8] = b"hello local model";
    const FIXTURE_SHA256: &str = "54607fbe414e6ffb88d6dd460c9f53efed016290ded1342fdc0656ab61b9ed75";

    #[derive(Clone, Debug)]
    struct TestHttpHost;

    #[derive(Clone, Debug)]
    struct FixtureArchiveHttpHost {
        archivePath: PathBuf,
    }

    impl HttpHost for TestHttpHost {
        /// Rejects buffered HTTP requests in registry-only tests.
        fn executeHttpRequest(&self, _request: HttpRequestData) -> HostResult<HttpResponseData> {
            Err(HostError::new("test HTTP request is not configured"))
        }

        /// Rejects downloads in registry-only tests.
        fn downloadFiles(
            &self,
            _request: HttpDownloadRequest,
            _control: HttpDownloadControl,
            _onProgress: HttpDownloadProgressCallback,
        ) -> HostResult<HttpDownloadResult> {
            Err(HostError::new("test HTTP download is not configured"))
        }
    }

    impl HttpHost for FixtureArchiveHttpHost {
        /// Rejects buffered HTTP requests in archive installer tests.
        fn executeHttpRequest(&self, _request: HttpRequestData) -> HostResult<HttpResponseData> {
            Err(HostError::new("test HTTP request is not configured"))
        }

        /// Copies one prepared archive into the requested download target.
        fn downloadFiles(
            &self,
            request: HttpDownloadRequest,
            _control: HttpDownloadControl,
            onProgress: HttpDownloadProgressCallback,
        ) -> HostResult<HttpDownloadResult> {
            let mut results = Vec::new();
            let totalBytes = fs::metadata(&self.archivePath)
                .map_err(|error| HostError::new(error.to_string()))?
                .len();
            for file in &request.files {
                onProgress(HttpDownloadProgress {
                    downloadId: request.downloadId.clone(),
                    fileId: file.fileId.clone(),
                    state: HttpDownloadProgressState::Started,
                    fileDownloadedBytes: 0,
                    fileTotalBytes: file.expectedBytes,
                    downloadedBytes: 0,
                    totalBytes,
                    completedFiles: 0,
                    totalFiles: request.files.len(),
                });
                fs::copy(&self.archivePath, &file.targetPath)
                    .map_err(|error| HostError::new(error.to_string()))?;
                onProgress(HttpDownloadProgress {
                    downloadId: request.downloadId.clone(),
                    fileId: file.fileId.clone(),
                    state: HttpDownloadProgressState::Completed,
                    fileDownloadedBytes: totalBytes,
                    fileTotalBytes: file.expectedBytes,
                    downloadedBytes: totalBytes,
                    totalBytes,
                    completedFiles: 1,
                    totalFiles: request.files.len(),
                });
                results.push(HttpDownloadFileResult {
                    fileId: file.fileId.clone(),
                    finalUrl: file.url.clone(),
                    targetPath: file.targetPath.clone(),
                    downloadedBytes: totalBytes,
                });
            }
            Ok(HttpDownloadResult {
                downloadId: request.downloadId,
                files: results,
                downloadedBytes: totalBytes,
            })
        }
    }

    /// Verifies registry-only installer use is valid inside an active Tokio runtime.
    #[test]
    fn registryAccessDoesNotCreateAnHttpRuntime() {
        let root = uniqueTempDir("tokio-registry");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime.block_on(async {
            let installer = installerForRoot(&root);
            assert_eq!(
                installer.readRegistry().unwrap(),
                LocalModelRegistrySnapshot::empty()
            );
        });
        fs::remove_dir_all(root).unwrap();
    }

    /// Verifies an installed model and persists the verification timestamp.
    #[test]
    fn verifyInstalledModelUpdatesRegistry() {
        let root = uniqueTempDir("verify");
        let installer = installerForRoot(&root);
        let manifest = testManifest();
        let storagePath = buildLocalModelStoragePath(
            &manifest.kind,
            &manifest.engine,
            &manifest.id,
            &manifest.version,
        )
        .unwrap();
        let installDir = installer.runtimeStoragePath(&storagePath).unwrap();
        fs::create_dir_all(&installDir).unwrap();
        fs::write(installDir.join("model.bin"), FIXTURE_BYTES).unwrap();
        installer
            .writeRegistry(&LocalModelRegistrySnapshot {
                installedModels: vec![InstalledLocalModel {
                    manifest,
                    storagePath,
                    installedAtMs: 100,
                    verifiedAtMs: None,
                }],
                ..LocalModelRegistrySnapshot::empty()
            })
            .unwrap();

        let result = installer
            .verifyInstalledModel("test-local-model", "v1", 200)
            .unwrap();

        assert_eq!(result.verifiedBytes, FIXTURE_BYTES.len() as u64);
        assert_eq!(result.installedModel.verifiedAtMs, Some(200));
        fs::remove_dir_all(&root).unwrap();
    }

    /// Verifies deleting an installed model removes files and registry state.
    #[test]
    fn deleteInstalledModelRemovesFilesAndRegistryRecord() {
        let root = uniqueTempDir("delete");
        let installer = installerForRoot(&root);
        let manifest = testManifest();
        let storagePath = buildLocalModelStoragePath(
            &manifest.kind,
            &manifest.engine,
            &manifest.id,
            &manifest.version,
        )
        .unwrap();
        let installDir = installer.runtimeStoragePath(&storagePath).unwrap();
        fs::create_dir_all(&installDir).unwrap();
        fs::write(installDir.join("model.bin"), FIXTURE_BYTES).unwrap();
        installer
            .writeRegistry(&LocalModelRegistrySnapshot {
                installedModels: vec![InstalledLocalModel {
                    manifest,
                    storagePath: storagePath.clone(),
                    installedAtMs: 100,
                    verifiedAtMs: Some(100),
                }],
                ..LocalModelRegistrySnapshot::empty()
            })
            .unwrap();

        let result = installer
            .deleteInstalledModel("test-local-model", "v1")
            .unwrap();
        let registry = installer.readRegistry().unwrap();

        assert_eq!(result.removedStoragePath, storagePath);
        assert_eq!(registry.installedModels.len(), 0);
        assert!(!installDir.exists());
        fs::remove_dir_all(&root).unwrap();
    }

    /// Verifies archive model installation extracts and verifies declared files.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn installArchiveModelExtractsAndVerifiesDeclaredFiles() {
        let root = uniqueTempDir("archive-install");
        let archivePath = root.join("source.tar.bz2");
        writeFixtureArchive(&archivePath);
        let archiveBytes = fs::metadata(&archivePath).unwrap().len();
        let archiveSha256 = fileSha256(&archivePath);
        let manifest = archiveManifest(archiveBytes, &archiveSha256);
        assert_eq!(manifest.declaredByteSize(), archiveBytes);
        let installer = LocalModelInstaller::new(
            root.to_path_buf(),
            root.join("config/preferences/local_model_registry.preferences.json"),
            Arc::new(FixtureArchiveHttpHost {
                archivePath: archivePath.clone(),
            }),
        )
        .unwrap();

        let result = installer
            .install(
                LocalModelInstallRequest {
                    manifest,
                    installedAtMs: 300,
                },
                HttpDownloadControl::new(),
                Arc::new(|_| {}),
            )
            .unwrap();
        let installDir = installer
            .runtimeStoragePath(&result.installedModel.storagePath)
            .unwrap();

        assert_eq!(result.downloadedBytes, archiveBytes);
        assert_eq!(
            fs::read(installDir.join("archive-root/model.bin")).unwrap(),
            FIXTURE_BYTES
        );
        assert!(installer
            .readRegistry()
            .unwrap()
            .getInstalledModel("test-archive-model", "v1")
            .is_some());
        fs::remove_dir_all(&root).unwrap();
    }

    /// Builds a local model installer fixture rooted under a temporary directory.
    fn installerForRoot(root: &Path) -> LocalModelInstaller {
        LocalModelInstaller::new(
            root.to_path_buf(),
            root.join("config/preferences/local_model_registry.preferences.json"),
            Arc::new(TestHttpHost),
        )
        .unwrap()
    }

    /// Builds a local model manifest fixture for installer tests.
    fn testManifest() -> LocalModelManifest {
        LocalModelManifest {
            id: "test-local-model".to_string(),
            version: "v1".to_string(),
            displayName: "Test Local Model".to_string(),
            description: "test model".to_string(),
            kind: LocalModelKind::SpeechToText,
            engine: LocalEngineKind::SherpaNcnn,
            license: "test".to_string(),
            homepage: "https://example.test/model".to_string(),
            languages: vec!["en".to_string()],
            tags: vec!["test".to_string()],
            engineRequirement: None,
            driver: None,
            sources: vec![LocalModelSource {
                id: "main".to_string(),
                kind: LocalModelSourceKind::DirectHttp,
                repository: "test".to_string(),
                revision: "v1".to_string(),
                baseUrl: "https://example.test/model".to_string(),
            }],
            installSource: LocalModelInstallSource::Files,
            files: vec![LocalModelFile {
                relativePath: "model.bin".to_string(),
                sha256: FIXTURE_SHA256.to_string(),
                byteSize: FIXTURE_BYTES.len() as u64,
                sourceId: "main".to_string(),
            }],
        }
    }

    /// Builds a local archive manifest fixture for installer tests.
    #[cfg(not(target_arch = "wasm32"))]
    fn archiveManifest(archiveBytes: u64, archiveSha256: &str) -> LocalModelManifest {
        LocalModelManifest {
            id: "test-archive-model".to_string(),
            version: "v1".to_string(),
            displayName: "Test Archive Model".to_string(),
            description: "test archive model".to_string(),
            kind: LocalModelKind::TextToSpeech,
            engine: LocalEngineKind::SherpaOnnx,
            license: "test".to_string(),
            homepage: "https://example.test/archive".to_string(),
            languages: vec!["en".to_string()],
            tags: vec!["test".to_string()],
            engineRequirement: None,
            driver: None,
            sources: vec![LocalModelSource {
                id: "archive-source".to_string(),
                kind: LocalModelSourceKind::DirectHttp,
                repository: "test".to_string(),
                revision: "v1".to_string(),
                baseUrl: "https://example.test/archive".to_string(),
            }],
            installSource: LocalModelInstallSource::Archives {
                archives: vec![LocalModelArchive {
                    archiveId: "archive".to_string(),
                    relativePath: "archive.tar.bz2".to_string(),
                    sha256: archiveSha256.to_string(),
                    byteSize: archiveBytes,
                    sourceId: "archive-source".to_string(),
                    archiveFormat: LocalModelArchiveFormat::TarBz2,
                }],
            },
            files: vec![LocalModelFile {
                relativePath: "archive-root/model.bin".to_string(),
                sha256: FIXTURE_SHA256.to_string(),
                byteSize: FIXTURE_BYTES.len() as u64,
                sourceId: "archive-source".to_string(),
            }],
        }
    }

    /// Writes a compressed tar archive containing one model fixture file.
    #[cfg(not(target_arch = "wasm32"))]
    fn writeFixtureArchive(archivePath: &Path) {
        use bzip2::write::BzEncoder;
        use bzip2::Compression;
        use tar::{Builder, Header};

        let file = fs::File::create(archivePath).unwrap();
        let encoder = BzEncoder::new(file, Compression::best());
        let mut builder = Builder::new(encoder);
        let mut header = Header::new_gnu();
        header.set_size(FIXTURE_BYTES.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "archive-root/model.bin", FIXTURE_BYTES)
            .unwrap();
        builder.finish().unwrap();
    }

    /// Returns the SHA-256 digest for one local fixture file.
    #[cfg(not(target_arch = "wasm32"))]
    fn fileSha256(path: &Path) -> String {
        let mut input = fs::File::open(path).unwrap();
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = input.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        format!("{:x}", hasher.finalize())
    }

    /// Creates a unique temporary directory for one installer test.
    fn uniqueTempDir(label: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "operit-local-models-{label}-{}-{now}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }
}
