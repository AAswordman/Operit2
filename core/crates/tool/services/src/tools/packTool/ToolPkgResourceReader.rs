use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use operit_vfs::VisualFileSystem::VisualFileSystem;
use operit_plugin_sdk::toolpkg::ToolPkgParser::{
    ToolPkgArchiveParser, ToolPkgContainerRuntime, ToolPkgSourceType,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

use super::text_resource_cache::{read_text_resources, ToolPkgTextResourceCache};
use super::{bundledPluginAssetByName, vfsPathString, javaStringHashCodeHex};
use crate::runtime_support::ToolRuntimeSupport;

use super::TOOLPKG_CACHE_SIGNATURE_FILE;

/// Captures resource capabilities without retaining or copying the package manager.
#[derive(Clone)]
#[allow(non_snake_case)]
pub(super) struct ToolPkgResourceReader {
    pub(super) storePaths: RuntimeStorePaths,
    pub(super) filesystem: Arc<VisualFileSystem>,
    pub(super) runtimeSupport: Arc<dyn ToolRuntimeSupport>,
    pub(super) toolPkgCacheLock: Arc<Mutex<()>>,
    pub(super) toolPkgTextResourceCache: Arc<ToolPkgTextResourceCache>,
}

/// Reads only requested entries from the existing extracted revision cache.
struct RegistrationResources {
    filesystem: Arc<VisualFileSystem>,
    root: PathBuf,
    index: operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgEntryIndex,
    package: String,
}

impl operit_plugin_sdk::javascript::ToolPkgTextResourceHost for RegistrationResources {
    /// Resolves archive spelling before reading and decrypting one cached text entry.
    fn read_toolpkg_text_resource(&self, package: &str, path: &str) -> Result<Option<String>, String> {
        if package != self.package { return Err("Registration resource owner mismatch".into()); }
        let Some(entry) = self.index.resolveEntryName(path) else { return Ok(None); };
        let bytes = self.filesystem.readFileBytes(&vfsPathString(&self.root.join(entry)))?;
        operit_plugin_sdk::toolpkg::ToolPkgProtection::decodeUtf8(&bytes).map(Some)
    }
}

impl ToolPkgResourceReader {
    /// Reuses the normal extracted cache instead of building a registration-only text snapshot.
    pub(super) fn registrationResources(
        &self, runtime: &ToolPkgContainerRuntime, bytes: &[u8],
    ) -> Result<Arc<dyn operit_plugin_sdk::javascript::ToolPkgTextResourceHost>, String> {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|error| error.to_string())?;
        let index = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
        let signature = self.buildToolPkgCacheSignatureForRuntime(runtime)
            .ok_or("Registration source is unavailable")?;
        let _guard = self.toolPkgCacheLock.lock().map_err(|error| error.to_string())?;
        let root = self.ensureToolPkgCacheDirLocked(
            &runtime.packageName, &signature, &runtime.mainEntry,
            |destination| ToolPkgArchiveParser::extractZipEntriesFromAssetBytes(bytes, self.filesystem.as_ref(), &vfsPathString(destination)),
        ).ok_or("Cannot prepare package resource cache")?;
        Ok(Arc::new(RegistrationResources { filesystem: self.filesystem.clone(), root, index, package: runtime.packageName.clone() }))
    }
    /// Returns the virtual cache root for package resources.
    #[allow(non_snake_case)]
    fn toolPkgCacheRootDir(&self) -> PathBuf {
        let dir = self.storePaths.toolpkg_cache_dir();
        let _ = self.filesystem.makeDirectory(&vfsPathString(&dir), true);
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
    pub(super) fn deleteToolPkgCacheDir(&self, packageName: &str) -> Result<(), String> {
        self.toolPkgTextResourceCache.invalidate(packageName);
        let _guard = self
            .toolPkgCacheLock
            .lock()
            .map_err(|error| error.to_string())?;
        self.deleteToolPkgCacheDirLocked(packageName)
    }

    /// Removes extracted resources while the cache lock is held.
    #[allow(non_snake_case)]
    fn deleteToolPkgCacheDirLocked(&self, packageName: &str) -> Result<(), String> {
        let path = vfsPathString(&self.storePaths.toolpkg_cache_dir()
            .join(Self::toolPkgCacheDirName(packageName)));
        if self.filesystem.fileExists(&path)?.exists {
            self.filesystem.deleteFile(&path, true)?;
        }
        Ok(())
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
            .filesystem
            .fileExists(&vfsPathString(&cacheDir))
            .map(|info| info.exists && info.isDirectory)
            .unwrap_or(false);
        let signatureFileExists = self
            .filesystem
            .fileExists(&vfsPathString(&signatureFile))
            .map(|info| info.exists && !info.isDirectory)
            .unwrap_or(false);
        let signatureMatches = if signatureFileExists {
            self.filesystem
                .readFile(&vfsPathString(&signatureFile))
                .map(|text| text == signature)
                .unwrap_or(false)
        } else {
            false
        };
        let mainScriptFile = cacheDir.join(mainEntry);
        let mainScriptExists = mainEntry.is_empty() || self
            .filesystem
            .fileExists(&vfsPathString(&mainScriptFile))
            .map(|info| info.exists && !info.isDirectory)
            .unwrap_or(false);

        if cacheDirExists && signatureFileExists && signatureMatches && mainScriptExists {
            return Some(cacheDir);
        }

        self.deleteToolPkgCacheDirLocked(packageName).ok()?;
        if self
            .filesystem
            .makeDirectory(&vfsPathString(&cacheDir), true)
            .is_err()
        {
            return None;
        }
        if !extractArchive(&cacheDir) {
            let _ = self.deleteToolPkgCacheDirLocked(packageName);
            return None;
        }
        if self
            .filesystem
            .writeFile(&vfsPathString(&signatureFile), signature, false)
            .is_err()
        {
            let _ = self.deleteToolPkgCacheDirLocked(packageName);
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
                let metadata = self.filesystem.fileInfo(sourcePath).ok()?;
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
                    .filesystem
                    .fileExists(&runtime.sourcePath)
                    .map(|info| info.exists && info.isDirectory)
                    .unwrap_or(false)
                {
                    return self
                        .filesystem
                        .copyFile(&runtime.sourcePath, &vfsPathString(destinationDir), true)
                        .is_ok();
                }
                ToolPkgArchiveParser::extractZipEntriesFromExternal(
                    self.filesystem.as_ref(),
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
                    self.filesystem.as_ref(),
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
                            .filesystem
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
                    read_text_resources(self.filesystem.as_ref(), &cacheDir)
                })?;
        Ok(resources)
    }

    /// Loads only the declared main entry through the revision-scoped text cache.
    #[allow(non_snake_case)]
    pub(super) fn mainScript(&self, runtime: &ToolPkgContainerRuntime) -> Result<String, String> {
        self.textResource(runtime, &runtime.mainEntry)
    }

    /// Caches one requested text resource from the shared extracted revision.
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
                format!(
                    "ToolPkg package source is unavailable: {}",
                    runtime.packageName
                )
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

    /// Reads runtime text through the same extracted revision cache used by registration.
    #[allow(non_snake_case)]
    fn readSourceText(&self, runtime: &ToolPkgContainerRuntime, path: &str) -> Result<String, String> {
        let directory = if runtime.sourceType != ToolPkgSourceType::ASSET
            && self.filesystem.fileExists(&runtime.sourcePath)?.isDirectory {
            PathBuf::from(&runtime.sourcePath)
        } else {
            self.ensureToolPkgCacheLocked(runtime).ok_or("ToolPkg resource cache is unavailable")?
        };
        let bytes = self.filesystem.readFileBytes(&vfsPathString(&directory.join(path)))?;
        operit_plugin_sdk::toolpkg::ToolPkgProtection::decodeUtf8(&bytes)
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
            .filesystem
            .fileExists(&vfsPathString(&resourceFile))
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
            .filesystem
            .fileExists(&vfsPathString(&resourceFile))
            .map(|info| info.exists && !info.isDirectory)
            .unwrap_or(false)
        {
            return None;
        }
        let bytes = self
            .filesystem
            .readFileBytes(&vfsPathString(&resourceFile))
            .ok()?;
        operit_plugin_sdk::toolpkg::ToolPkgProtection::decryptIfNeeded(&bytes).ok()
    }
}
