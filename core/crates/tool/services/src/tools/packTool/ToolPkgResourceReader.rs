use std::collections::BTreeMap;
use std::io::{Cursor, Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use operit_host_api::FileSystemHost;
use operit_plugin_sdk::toolpkg::ToolPkgParser::{
    ToolPkgArchiveParser, ToolPkgContainerRuntime, ToolPkgSourceType,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

use super::text_resource_cache::{read_text_resources, ToolPkgTextResourceCache};
use super::{bundledPluginAssetByName, hostPath, javaStringHashCodeHex};
use crate::runtime_support::ToolRuntimeSupport;

use super::TOOLPKG_CACHE_SIGNATURE_FILE;

/// Captures resource capabilities without retaining or copying the package manager.
#[derive(Clone)]
#[allow(non_snake_case)]
pub(super) struct ToolPkgResourceReader {
    pub(super) storePaths: RuntimeStorePaths,
    pub(super) fileSystemHost: Arc<dyn FileSystemHost>,
    pub(super) runtimeSupport: Arc<dyn ToolRuntimeSupport>,
    pub(super) toolPkgCacheLock: Arc<Mutex<()>>,
    pub(super) toolPkgTextResourceCache: Arc<ToolPkgTextResourceCache>,
}

impl ToolPkgResourceReader {
    /// Returns the host cache root for package resources.
    #[allow(non_snake_case)]
    fn toolPkgCacheRootDir(&self) -> PathBuf {
        let dir = self.storePaths.toolpkg_cache_dir();
        let _ = self.fileSystemHost.makeDirectory(&hostPath(&dir), true);
        dir
    }

    /// Builds the stable directory name for one package.
    #[allow(non_snake_case)]
    fn toolPkgCacheDirName(packageName: &str) -> String {
        let normalized = packageName.trim();
        let safeName = normalized
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                    ch
                } else {
                    '_'
                }
            })
            .collect::<String>();
        let safeName = if safeName.trim().is_empty() {
            "toolpkg".to_string()
        } else {
            safeName
        };
        let hash = javaStringHashCodeHex(normalized);
        format!("{safeName}-{hash}")
    }

    /// Returns the cache directory for one package.
    #[allow(non_snake_case)]
    fn toolPkgCacheDir(&self, packageName: &str) -> PathBuf {
        self.toolPkgCacheRootDir()
            .join(Self::toolPkgCacheDirName(packageName))
    }

    /// Invalidates text snapshots and removes extracted resources.
    #[allow(non_snake_case)]
    pub(super) fn deleteToolPkgCacheDir(&self, packageName: &str) {
        self.toolPkgTextResourceCache.invalidate(packageName);
        let _guard = self
            .toolPkgCacheLock
            .lock()
            .expect("toolpkg cache mutex poisoned");
        self.deleteToolPkgCacheDirLocked(packageName);
    }

    /// Removes extracted resources while the cache lock is held.
    #[allow(non_snake_case)]
    fn deleteToolPkgCacheDirLocked(&self, packageName: &str) {
        let dir = self.toolPkgCacheDir(packageName);
        let _ = self.fileSystemHost.deleteFile(&hostPath(&dir), true);
    }

    /// Validates or extracts a package revision under the shared cache lock.
    #[allow(non_snake_case)]
    fn ensureToolPkgCacheDirLocked<FExtractArchive>(
        &self,
        packageName: &str,
        signature: &str,
        mainEntry: &str,
        extractArchive: FExtractArchive,
    ) -> Option<PathBuf>
    where
        FExtractArchive: Fn(&Path) -> bool,
    {
        let cacheDir = self.toolPkgCacheDir(packageName);
        let signatureFile = cacheDir.join(TOOLPKG_CACHE_SIGNATURE_FILE);
        let cacheDirExists = self
            .fileSystemHost
            .fileExists(&hostPath(&cacheDir))
            .map(|info| info.exists && info.isDirectory)
            .unwrap_or(false);
        let signatureFileExists = self
            .fileSystemHost
            .fileExists(&hostPath(&signatureFile))
            .map(|info| info.exists && !info.isDirectory)
            .unwrap_or(false);
        let signatureMatches = if signatureFileExists {
            self.fileSystemHost
                .readFile(&hostPath(&signatureFile))
                .map(|text| text == signature)
                .unwrap_or(false)
        } else {
            false
        };
        let mainScriptFile = cacheDir.join(mainEntry);
        let mainScriptExists = self
            .fileSystemHost
            .fileExists(&hostPath(&mainScriptFile))
            .map(|info| info.exists && !info.isDirectory)
            .unwrap_or(false);

        if cacheDirExists && signatureFileExists && signatureMatches && mainScriptExists {
            return Some(cacheDir);
        }

        self.deleteToolPkgCacheDirLocked(packageName);
        if self
            .fileSystemHost
            .makeDirectory(&hostPath(&cacheDir), true)
            .is_err()
        {
            return None;
        }
        if !extractArchive(&cacheDir) {
            self.deleteToolPkgCacheDirLocked(packageName);
            return None;
        }
        if self
            .fileSystemHost
            .writeFile(&hostPath(&signatureFile), signature, false)
            .is_err()
        {
            self.deleteToolPkgCacheDirLocked(packageName);
            return None;
        }
        Some(cacheDir)
    }

    /// Validates mutable sources and reuses build-time fingerprints for immutable assets.
    /// Performs the resource cache operation.
    #[allow(non_snake_case)]
    fn buildToolPkgCacheSignature(
        &self,
        sourceType: &ToolPkgSourceType,
        sourcePath: &str,
        version: &str,
        mainEntry: &str,
    ) -> Option<String> {
        match sourceType {
            ToolPkgSourceType::EXTERNAL | ToolPkgSourceType::MARKET => {
                let sourceFile = PathBuf::from(sourcePath);
                let metadata = self.fileSystemHost.fileInfo(sourcePath).ok()?;
                if !metadata.exists {
                    return None;
                }
                Some(format!(
                    "{}|{}|{}|{}|{}|{}",
                    if *sourceType == ToolPkgSourceType::MARKET {
                        "market"
                    } else {
                        "external"
                    },
                    sourceFile.to_string_lossy(),
                    metadata.size,
                    metadata.lastModified,
                    version,
                    mainEntry
                ))
            }
            ToolPkgSourceType::ASSET => {
                let asset = bundledPluginAssetByName(self.runtimeSupport.as_ref(), sourcePath)?;
                Some(format!(
                    "asset|{}|{}|{}|{}|{}",
                    sourcePath,
                    asset.bytes.len(),
                    asset.sha256,
                    version,
                    mainEntry
                ))
            }
        }
    }

    /// Computes the source signature for one captured package revision.
    #[allow(non_snake_case)]
    fn buildToolPkgCacheSignatureForRuntime(
        &self,
        runtime: &ToolPkgContainerRuntime,
    ) -> Option<String> {
        self.buildToolPkgCacheSignature(
            &runtime.sourceType,
            &runtime.sourcePath,
            &runtime.version,
            &runtime.mainEntry,
        )
    }

    /// Extracts the captured archive using host filesystem capabilities.
    #[allow(non_snake_case)]
    fn extractToolPkgArchive(
        &self,
        runtime: &ToolPkgContainerRuntime,
        destinationDir: &Path,
    ) -> bool {
        match runtime.sourceType {
            ToolPkgSourceType::EXTERNAL | ToolPkgSourceType::MARKET => {
                if self
                    .fileSystemHost
                    .fileExists(&runtime.sourcePath)
                    .map(|info| info.exists && info.isDirectory)
                    .unwrap_or(false)
                {
                    return self
                        .fileSystemHost
                        .copyFile(&runtime.sourcePath, &hostPath(destinationDir), true)
                        .is_ok();
                }
                ToolPkgArchiveParser::extractZipEntriesFromExternal(
                    self.fileSystemHost.as_ref(),
                    &runtime.sourcePath,
                    &destinationDir.to_string_lossy(),
                )
            }
            ToolPkgSourceType::ASSET => {
                let Some(asset) =
                    bundledPluginAssetByName(self.runtimeSupport.as_ref(), &runtime.sourcePath)
                else {
                    return false;
                };
                ToolPkgArchiveParser::extractZipEntriesFromAssetBytes(
                    asset.bytes,
                    self.fileSystemHost.as_ref(),
                    &destinationDir.to_string_lossy(),
                )
            }
        }
    }

    /// Ensures the captured package revision has an extracted cache.
    #[allow(non_snake_case)]
    pub(super) fn ensureToolPkgCache(&self, runtime: &ToolPkgContainerRuntime) -> Option<PathBuf> {
        let _guard = self
            .toolPkgCacheLock
            .lock()
            .expect("toolpkg cache mutex poisoned");
        self.ensureToolPkgCacheLocked(runtime)
    }

    /// Validates the cache while its files are protected from invalidation.
    fn ensureToolPkgCacheLocked(&self, runtime: &ToolPkgContainerRuntime) -> Option<PathBuf> {
        let signature = self.buildToolPkgCacheSignatureForRuntime(runtime)?;
        self.ensureToolPkgCacheDirLocked(
            &runtime.packageName,
            &signature,
            &runtime.mainEntry,
            |destinationDir| self.extractToolPkgArchive(runtime, destinationDir),
        )
    }

    /// Shares immutable text resources across sessions of the same package revision.
    #[allow(non_snake_case)]
    pub fn toolPkgTextResources(
        &self,
        runtime: &ToolPkgContainerRuntime,
    ) -> Result<Arc<BTreeMap<String, String>>, String> {
        let packageName = &runtime.packageName;
        let signature = self
            .buildToolPkgCacheSignatureForRuntime(&runtime)
            .ok_or_else(|| format!("ToolPkg package source is unavailable: {packageName}"))?;
        let (resources, _) =
            self.toolPkgTextResourceCache
                .get_or_load(&packageName, &signature, || {
                    let _guard = self
                        .toolPkgCacheLock
                        .lock()
                        .expect("toolpkg cache mutex poisoned");
                    // Validate/extract once for this batch. Reading each cached file must
                    // not rehash the archive or recheck its on-disk signature.
                    let directorySource = runtime.sourceType != ToolPkgSourceType::ASSET
                        && self
                            .fileSystemHost
                            .fileExists(&runtime.sourcePath)
                            .map_err(|error| error.to_string())?
                            .isDirectory;
                    let cacheDir = if directorySource {
                        // Reload directory sources from their actual files; changing a
                        // child need not change the source directory's cached signature.
                        PathBuf::from(&runtime.sourcePath)
                    } else {
                        self.ensureToolPkgCacheDirLocked(
                            &packageName,
                            &signature,
                            &runtime.mainEntry,
                            |destination| self.extractToolPkgArchive(&runtime, destination),
                        )
                        .ok_or_else(|| {
                            format!("ToolPkg package cache is unavailable: {packageName}")
                        })?
                    };
                    read_text_resources(self.fileSystemHost.as_ref(), &cacheDir)
                })?;
        Ok(resources)
    }

    /// Loads only the declared main entry through the revision-scoped text cache.
    #[allow(non_snake_case)]
    pub(super) fn mainScript(&self, runtime: &ToolPkgContainerRuntime) -> Result<String, String> {
        self.textResource(runtime, &runtime.mainEntry)
    }

    /// Caches one requested resource without extracting or scanning the rest of the package.
    #[allow(non_snake_case)]
    fn textResource(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resourcePath: &str,
    ) -> Result<String, String> {
        let path = ToolPkgArchiveParser::normalizeResourcePath(resourcePath)
            .ok_or_else(|| format!("Invalid ToolPkg resource path: {resourcePath}"))?;
        let signature = self
            .buildToolPkgCacheSignatureForRuntime(runtime)
            .ok_or_else(|| {
                format!("ToolPkg package source is unavailable: {}", runtime.packageName)
            })?;
        let (text, _) = self.toolPkgTextResourceCache.get_or_load_text(
            &runtime.packageName,
            &signature,
            &path.to_ascii_lowercase(),
            || {
                let _guard = self
                    .toolPkgCacheLock
                    .lock()
                    .map_err(|error| error.to_string())?;
                self.readSourceText(runtime, &path)
            },
        )?;
        Ok(text)
    }

    /// Reads an exact source entry through host capabilities, borrowing immutable archive bytes.
    #[allow(non_snake_case)]
    fn readSourceText(
        &self,
        runtime: &ToolPkgContainerRuntime,
        path: &str,
    ) -> Result<String, String> {
        match runtime.sourceType {
            ToolPkgSourceType::ASSET => {
                let asset =
                    bundledPluginAssetByName(self.runtimeSupport.as_ref(), &runtime.sourcePath)
                        .ok_or_else(|| {
                            format!("ToolPkg asset is unavailable: {}", runtime.sourcePath)
                        })?;
                Self::readArchiveText(Cursor::new(asset.bytes), path)
            }
            ToolPkgSourceType::EXTERNAL | ToolPkgSourceType::MARKET => {
                let source = self
                    .fileSystemHost
                    .fileExists(&runtime.sourcePath)
                    .map_err(|error| error.to_string())?;
                if !source.exists {
                    return Err(format!(
                        "ToolPkg package source is unavailable: {}",
                        runtime.sourcePath,
                    ));
                }
                if source.isDirectory {
                    let resource = PathBuf::from(&runtime.sourcePath).join(path);
                    let bytes = self
                        .fileSystemHost
                        .readFileBytes(&hostPath(&resource))
                        .map_err(|error| format!("Cannot read ToolPkg resource {path}: {error}"))?;
                    return operit_plugin_sdk::toolpkg::ToolPkgProtection::decodeUtf8(&bytes)
                        .map_err(|error| format!("Cannot decode ToolPkg resource {path}: {error}"));
                }
                let bytes = self
                    .fileSystemHost
                    .readFileBytes(&runtime.sourcePath)
                    .map_err(|error| {
                        format!("Cannot read ToolPkg archive {}: {error}", runtime.sourcePath)
                    })?;
                Self::readArchiveText(Cursor::new(bytes), path)
            }
        }
    }

    /// Decompresses and decodes exactly one archive entry without creating an extracted directory.
    #[allow(non_snake_case)]
    fn readArchiveText<R: Read + Seek>(reader: R, path: &str) -> Result<String, String> {
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|error| format!("Cannot open ToolPkg archive: {error}"))?;
        let mut entry = archive.by_name(path)
            .map_err(|error| format!("Cannot open ToolPkg resource {path}: {error}"))?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)
            .map_err(|error| format!("Cannot read ToolPkg resource {path}: {error}"))?;
        operit_plugin_sdk::toolpkg::ToolPkgProtection::decodeUtf8(&bytes)
            .map_err(|error| format!("Cannot decode ToolPkg resource {path}: {error}"))
    }

    /// Resolves a normalized resource path inside the validated cache.
    #[allow(non_snake_case)]
    pub(crate) fn resolveToolPkgResourceFile(
        &self,
        runtime: &ToolPkgContainerRuntime,
        normalizedResourcePath: &str,
    ) -> Option<PathBuf> {
        let _guard = self
            .toolPkgCacheLock
            .lock()
            .expect("toolpkg cache mutex poisoned");
        self.resolveToolPkgResourceFileLocked(runtime, normalizedResourcePath)
    }

    /// Resolves the captured resource while the shared cache lock is held.
    fn resolveToolPkgResourceFileLocked(
        &self,
        runtime: &ToolPkgContainerRuntime,
        normalizedResourcePath: &str,
    ) -> Option<PathBuf> {
        let normalizedPath = ToolPkgArchiveParser::normalizeResourcePath(normalizedResourcePath)?;
        let cacheDir = self.ensureToolPkgCacheLocked(runtime)?;
        let resourceFile = cacheDir.join(normalizedPath);
        if !self
            .fileSystemHost
            .fileExists(&hostPath(&resourceFile))
            .map(|info| info.exists)
            .unwrap_or(false)
        {
            return None;
        }
        Some(resourceFile)
    }

    /// Reads and decrypts a cached resource through the host filesystem.
    #[allow(non_snake_case)]
    pub(crate) fn readToolPkgResourceBytes(
        &self,
        runtime: &ToolPkgContainerRuntime,
        normalizedResourcePath: &str,
    ) -> Option<Vec<u8>> {
        let _guard = self
            .toolPkgCacheLock
            .lock()
            .expect("toolpkg cache mutex poisoned");
        let resourceFile =
            self.resolveToolPkgResourceFileLocked(runtime, normalizedResourcePath)?;
        if !self
            .fileSystemHost
            .fileExists(&hostPath(&resourceFile))
            .map(|info| info.exists && !info.isDirectory)
            .unwrap_or(false)
        {
            return None;
        }
        let bytes = self
            .fileSystemHost
            .readFileBytes(&hostPath(&resourceFile))
            .ok()?;
        operit_plugin_sdk::toolpkg::ToolPkgProtection::decryptIfNeeded(&bytes).ok()
    }
}
