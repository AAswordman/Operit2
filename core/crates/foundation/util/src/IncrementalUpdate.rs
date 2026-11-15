//! Byte-exact package reconstruction over Host HTTP, VFS and transactional storage.
use crate::GithubReleaseUtil::{
    downloadOptionalUpdatePackage, downloadUpdatePackage, ensureDownloadActive, updateHttpRequest,
    validateDownloadedUpdatePackage, validateUpdatePackageSource, DownloadAttempt,
    FullUpdateProgressEvent, FullUpdateStage, FullUpdateTarget, GithubReleaseUtil, ReleaseInfo,
};
use crate::UpdateHttp::{requestOptional, UpdateHttpEvent};
use operit_host_api::{
    FileSource, HostManager::HostManager, HttpDownloadControl, RuntimeStorageWriteSession,
};
use operit_vfs::VisualFileSystem::VisualFileSystem;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Read};
use std::sync::{Arc, Mutex};

const BUFFER: usize = 1024 * 1024;
const MAX_PACKAGE: u64 = 16 * 1024 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateManifest {
    pub schema: u32,
    pub version: String,
    pub assetName: String,
    pub sha256: String,
    pub size: u64,
    pub patches: Vec<PackagePatch>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackagePatch {
    pub baseVersion: String,
    pub baseSha256: String,
    pub baseSize: u64,
    pub assetName: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FullTransferPolicy {
    version: String,
    assetName: String,
}

/// Rejects malformed hashes before they enter comparisons or storage paths.
fn validHash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Decodes the exact SHA-256 digest syntax returned by GitHub's release API.
fn githubHash(value: &str) -> Result<&str, String> {
    value
        .strip_prefix("sha256:")
        .filter(|hash| validHash(hash))
        .ok_or_else(|| "Release asset has no valid GitHub SHA-256 digest".into())
}

/// Hashes bounded file ranges and notices cancellation during local work.
pub fn hashSource(
    source: &dyn FileSource,
    control: &HttpDownloadControl,
) -> Result<(String, u64), String> {
    let size = u64::try_from(source.fileInfo().map_err(|e| e.to_string())?.size)
        .map_err(|_| "Invalid package size")?;
    if size == 0 || size > MAX_PACKAGE {
        return Err("Package size exceeds update limits".into());
    }
    let mut hash = Sha256::new();
    let mut offset = 0;
    while offset < size {
        ensureDownloadActive(control)?;
        let length = (size - offset).min(BUFFER as u64) as usize;
        let bytes = source
            .readBytesRange(offset, length)
            .map_err(|e| e.to_string())?;
        if bytes.len() != length {
            return Err("Package changed during verification".into());
        }
        hash.update(&bytes);
        offset += length as u64;
    }
    Ok((format!("{:x}", hash.finalize()), size))
}

/// Binds all manifest fields and patch names to the selected GitHub release asset.
pub fn validateManifest(manifest: &UpdateManifest, release: &ReleaseInfo) -> Result<(), String> {
    if !matches!(manifest.schema, 1 | 2 | 3)
        || manifest.version != release.version
        || manifest.assetName != release.assetName
        || manifest.size == 0
        || manifest.size > MAX_PACKAGE
        || !validHash(&manifest.sha256)
        || manifest.patches.len() > 64
    {
        return Err("Update manifest does not match selected package".into());
    }
    if release.fullPackageAvailable {
        let packageHash = githubHash(
            release
                .packageSha256
                .as_deref()
                .ok_or("Package digest is missing")?,
        )?;
        if manifest.sha256 != packageHash {
            return Err("Manifest package digest disagrees with GitHub".into());
        }
    }
    let mut bases = std::collections::HashSet::new();
    for patch in &manifest.patches {
        let expected = format!("{}.from-{}.patch.gz", release.assetName, patch.baseSha256);
        if !validHash(&patch.baseSha256)
            || !validHash(&patch.sha256)
            || patch.assetName != expected
            || patch.baseSize == 0
            || patch.baseSize > MAX_PACKAGE
            || patch.size == 0
            || patch.size > MAX_PACKAGE
            || !bases.insert(&patch.baseSha256)
        {
            return Err("Invalid incremental package declaration".into());
        }
        if crate::GithubReleaseUtil::GithubReleaseUtil::compareVersions(
            &patch.baseVersion,
            &manifest.version,
        )? >= 0
        {
            return Err("Incremental base must be older than its target".into());
        }
    }
    Ok(())
}

/// Fetches a small manifest and authenticates its bytes against GitHub metadata.
async fn fetchManifest(
    release: &ReleaseInfo,
    context: &HostManager,
    control: HttpDownloadControl,
) -> Result<Option<UpdateManifest>, String> {
    let Some(asset) = &release.updateManifest else {
        return Ok(None);
    };
    validateUpdatePackageSource(&release.downloadUrl, &release.assetName)?;
    validateUpdatePackageSource(&asset.url, &asset.name)?;
    let tag = asset
        .url
        .rsplit('/')
        .nth(1)
        .ok_or("Missing manifest release tag")?;
    if asset.name != format!("{}.update.json", release.assetName)
        || tag.strip_prefix('v').unwrap_or(tag) != release.version
    {
        return Err("Manifest source does not match selected release".into());
    }
    let expectedHash = githubHash(&asset.digest)?;
    let sources =
        crate::UpdateSources::ranked(&asset.url, &asset.name, context, control.clone(), true)
            .await?;
    let mut lastError = None;
    for source in sources {
        let result: Result<Option<UpdateManifest>, String> = async {
            let body = Arc::new(Mutex::new(Vec::new()));
            let sink = body.clone();
            let mut request = updateHttpRequest(&source, "GET");
            request.connectTimeoutSeconds = 8;
            request.readTimeoutSeconds = 15;
            let present = requestOptional(
                context
                    .httpHost
                    .clone()
                    .ok_or("HttpHost is required for updates")?,
                request,
                control.clone(),
                move |event| {
                    if let UpdateHttpEvent::Bytes(bytes) = event {
                        let mut body = sink.lock().map_err(|e| e.to_string())?;
                        if body.len().saturating_add(bytes.len()) > 4 * 1024 * 1024 {
                            return Err("Update manifest exceeds size limit".into());
                        }
                        body.extend_from_slice(&bytes);
                    }
                    Ok(())
                },
            )
            .await?;
            if !present {
                return Ok(None);
            }
            let bytes = body.lock().map_err(|e| e.to_string())?;
            if format!("{:x}", Sha256::digest(&*bytes)) != expectedHash {
                return Err("Update manifest SHA-256 mismatch".into());
            }
            let manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            validateManifest(&manifest, release)?;
            Ok(Some(manifest))
        }
        .await;
        match result {
            Ok(Some(manifest)) => return Ok(Some(manifest)),
            Ok(None) => {}
            Err(error) => {
                ensureDownloadActive(&control)?;
                crate::AppLogger::AppLogger::w(
                    "ApplicationUpdate",
                    &format!("Manifest source failed: {source}: {error}"),
                );
                lastError = Some(error);
            }
        }
    }
    ensureDownloadActive(&control)?;
    match lastError {
        Some(error) => Err(error),
        None => Ok(None),
    }
}

struct SourceReader<'a> {
    source: &'a dyn FileSource,
    offset: u64,
    size: u64,
}
impl Read for SourceReader<'_> {
    /// Adapts bounded Host reads to the streaming gzip decoder.
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let length = (self.size - self.offset).min(buffer.len() as u64) as usize;
        if length == 0 {
            return Ok(0);
        }
        let bytes = self
            .source
            .readBytesRange(self.offset, length)
            .map_err(io::Error::other)?;
        if bytes.len() != length {
            return Err(io::Error::other("Truncated patch source"));
        }
        buffer[..length].copy_from_slice(&bytes);
        self.offset += length as u64;
        Ok(length)
    }
}

struct PendingWrite(Option<Box<dyn RuntimeStorageWriteSession>>);
impl Drop for PendingWrite {
    /// Discards an incomplete reconstruction on error, cancellation or unwinding.
    fn drop(&mut self) {
        if let Some(writer) = self.0.take() {
            let _ = writer.discard();
        }
    }
}

/// Reads a fixed-width integer without accepting truncated instructions.
fn integer(reader: &mut impl Read) -> Result<u64, String> {
    let mut bytes = [0; 8];
    reader.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    Ok(u64::from_le_bytes(bytes))
}

/// Authenticates sources and reconstructs the original signed installer, including CAB-aware deltas.
pub fn applyPatch(
    base: &dyn FileSource,
    patch: &dyn FileSource,
    manifest: &UpdateManifest,
    declaration: &PackagePatch,
    context: &HostManager,
    destination: &str,
    control: &HttpDownloadControl,
) -> Result<(), String> {
    if hashSource(base, control)? != (declaration.baseSha256.clone(), declaration.baseSize)
        || hashSource(patch, control)? != (declaration.sha256.clone(), declaration.size)
    {
        return Err("Incremental source SHA-256 mismatch".into());
    }
    let source = SourceReader {
        source: patch,
        offset: 0,
        size: declaration.size,
    };
    let mut decoder = flate2::read::MultiGzDecoder::new(source);
    let mut magic = [0; 9];
    decoder.read_exact(&mut magic).map_err(|e| e.to_string())?;
    let residuals = &magic == b"O2PATCH2\0" && matches!(manifest.schema, 2 | 3);
    let cabinets = &magic == b"O2PATCH3\0" && manifest.schema == 3;
    if !(cabinets || residuals || (&magic == b"O2PATCH1\0" && manifest.schema == 1))
        || integer(&mut decoder)? != declaration.baseSize
        || integer(&mut decoder)? != manifest.size
    {
        return Err("Invalid incremental package header".into());
    }
    let mut hashes = [0; 64];
    decoder.read_exact(&mut hashes).map_err(|e| e.to_string())?;
    let encoded = hashes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if encoded != format!("{}{}", declaration.baseSha256, manifest.sha256) {
        return Err("Patch header disagrees with authenticated manifest".into());
    }
    if cabinets {
        return applyCabinetPatch(base, &mut decoder, manifest, context, destination, control);
    }
    applyInstructions(
        base,
        &mut decoder,
        declaration.baseSize,
        manifest.size,
        &manifest.sha256,
        residuals,
        Some(&manifest.assetName),
        context,
        destination,
        control,
    )
}

/// Reconstructs expanded payloads in a disposable job and retains only the verified original MSI.
fn applyCabinetPatch(
    base: &dyn FileSource,
    decoder: &mut impl Read,
    manifest: &UpdateManifest,
    context: &HostManager,
    destination: &str,
    control: &HttpDownloadControl,
) -> Result<(), String> {
    use operit_host_api::ApplicationUpdateTransform::{ExpandMsiCabinets, RestoreMsiCabinets};
    let mut length = [0; 4];
    decoder.read_exact(&mut length).map_err(|e| e.to_string())?;
    let length = u32::from_le_bytes(length) as usize;
    if length > 4 * BUFFER {
        return Err("MSI recipe exceeds limit".into());
    }
    let mut recipe = vec![0; length];
    decoder.read_exact(&mut recipe).map_err(|e| e.to_string())?;
    let oldSize = integer(decoder)?;
    let newSize = integer(decoder)?;
    if oldSize == 0 || newSize == 0 || oldSize > MAX_PACKAGE || newSize > MAX_PACKAGE {
        return Err("Expanded package exceeds update limits".into());
    }
    let mut hashes = [0; 64];
    decoder.read_exact(&mut hashes).map_err(|e| e.to_string())?;
    let oldHash = hashes[..32]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let newHash = hashes[32..]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let host = context
        .systemOperationHost
        .as_ref()
        .ok_or("Installer transform unavailable")?;
    let directory = format!(
        "{}/{}",
        crate::RuntimeStorageLayout::RUNTIME_APPLICATION_UPDATES_DIR_PATH,
        uuid::Uuid::new_v4()
    );
    let storage = context
        .runtimeStorageHost
        .clone()
        .ok_or("Runtime storage unavailable")?;
    let _scratch = DownloadAttempt {
        storage: storage.clone(),
        path: directory.clone(),
        published: false,
    };
    let vfs = context.vfs()?;
    let old = vfs.file(&VisualFileSystem::runtimePath(&format!(
        "{directory}/base"
    ))?)?;
    host.transformApplicationUpdate(base, &old, ExpandMsiCabinets { recipe }, control)
        .map_err(|e| e.to_string())?;
    if hashSource(&old, control)? != (oldHash, oldSize) {
        return Err("Expanded base package SHA-256 mismatch".into());
    }
    let expandedPath = format!("{directory}/target");
    applyInstructions(
        &old,
        decoder,
        oldSize,
        newSize,
        &newHash,
        true,
        None,
        context,
        &expandedPath,
        control,
    )?;
    storage
        .delete(&format!("{directory}/base"), false)
        .map_err(|e| e.to_string())?;
    let expanded = vfs.file(&VisualFileSystem::runtimePath(&expandedPath)?)?;
    let restored = vfs.file(&VisualFileSystem::runtimePath(&format!(
        "{directory}/restored.msi"
    ))?)?;
    host.transformApplicationUpdate(&expanded, &restored, RestoreMsiCabinets, control)
        .map_err(|e| e.to_string())?;
    storage
        .delete(&expandedPath, false)
        .map_err(|e| e.to_string())?;
    copyPackage(
        &restored,
        destination,
        &manifest.assetName,
        Some(&(manifest.sha256.clone(), manifest.size)),
        context,
        control,
    )?;
    Ok(())
}

/// Applies bounded copy, literal and residual operations to original or expanded package bytes.
fn applyInstructions(
    base: &dyn FileSource,
    decoder: &mut impl Read,
    baseSize: u64,
    targetSize: u64,
    targetHash: &str,
    residuals: bool,
    assetName: Option<&str>,
    context: &HostManager,
    destination: &str,
    control: &HttpDownloadControl,
) -> Result<(), String> {
    let writes = context
        .runtimeStorageWriteHost
        .as_ref()
        .ok_or("Update writer unavailable")?;
    let mut writer = PendingWrite(Some(
        writes
            .createWriteSession(destination)
            .map_err(|e| e.to_string())?,
    ));
    let mut hash = Sha256::new();
    let mut written = 0u64;
    let mut prefix = Vec::with_capacity(8);
    let mut ended = false;
    for _ in 0..4_000_000 {
        ensureDownloadActive(control)?;
        let mut opcode = [0];
        decoder.read_exact(&mut opcode).map_err(|e| e.to_string())?;
        if opcode[0] == 0 {
            ended = true;
            break;
        }
        let (offset, length) = match opcode[0] {
            1 => (integer(decoder)?, integer(decoder)?),
            3 if residuals => (integer(decoder)?, integer(decoder)?),
            2 => (0, integer(decoder)?),
            _ => return Err("Invalid patch instruction".into()),
        };
        if length == 0
            || length > targetSize.saturating_sub(written)
            || (opcode[0] != 2 && (offset > baseSize || length > baseSize - offset))
        {
            return Err("Patch instruction exceeds package bounds".into());
        }
        let mut done = 0;
        while done < length {
            ensureDownloadActive(control)?;
            let count = (length - done).min(BUFFER as u64) as usize;
            let bytes = if opcode[0] != 2 {
                let mut bytes = base
                    .readBytesRange(offset + done, count)
                    .map_err(|e| e.to_string())?;
                if bytes.len() != count {
                    return Err("Truncated base package".into());
                }
                if opcode[0] == 3 {
                    let mut residual = vec![0; count];
                    decoder
                        .read_exact(&mut residual)
                        .map_err(|e| e.to_string())?;
                    for (byte, diff) in bytes.iter_mut().zip(residual) {
                        *byte = byte.wrapping_add(diff);
                    }
                }
                bytes
            } else {
                let mut bytes = vec![0; count];
                decoder.read_exact(&mut bytes).map_err(|e| e.to_string())?;
                bytes
            };
            prefix.extend_from_slice(&bytes[..bytes.len().min(8 - prefix.len())]);
            hash.update(&bytes);
            writer
                .0
                .as_mut()
                .expect("pending update")
                .writeChunk(&bytes)
                .map_err(|e| e.to_string())?;
            done += count as u64;
        }
        written += length;
    }
    let mut trailing = [0];
    if !ended
        || written != targetSize
        || decoder.read(&mut trailing).map_err(|e| e.to_string())? != 0
        || format!("{:x}", hash.finalize()) != targetHash
    {
        return Err("Reconstructed package SHA-256 or length mismatch".into());
    }
    if let Some(assetName) = assetName {
        validateDownloadedUpdatePackage(&prefix, assetName)?;
    }
    ensureDownloadActive(control)?;
    writer
        .0
        .take()
        .expect("pending update")
        .commit()
        .map_err(|e| e.to_string())
}

/// Copies bounded package bytes transactionally, validating the copied container and optional digest.
fn copyPackage(
    source: &dyn FileSource,
    destination: &str,
    assetName: &str,
    expected: Option<&(String, u64)>,
    context: &HostManager,
    control: &HttpDownloadControl,
) -> Result<(String, u64), String> {
    let size = u64::try_from(source.fileInfo().map_err(|e| e.to_string())?.size)
        .map_err(|_| "Invalid package size")?;
    if size == 0 || size > MAX_PACKAGE {
        return Err("Package size exceeds update limits".into());
    }
    validateDownloadedUpdatePackage(
        &source
            .readBytesRange(0, size.min(8) as usize)
            .map_err(|e| e.to_string())?,
        assetName,
    )?;
    let mut writer = PendingWrite(Some(
        context
            .runtimeStorageWriteHost
            .as_ref()
            .ok_or("Update writer unavailable")?
            .createWriteSession(destination)
            .map_err(|e| e.to_string())?,
    ));
    let mut hash = Sha256::new();
    let mut prefix = Vec::with_capacity(8);
    let mut offset = 0u64;
    while offset < size {
        ensureDownloadActive(control)?;
        let count = (size - offset).min(BUFFER as u64) as usize;
        let bytes = source
            .readBytesRange(offset, count)
            .map_err(|e| e.to_string())?;
        if bytes.len() != count {
            return Err("Package changed during cache write".into());
        }
        prefix.extend_from_slice(&bytes[..bytes.len().min(8 - prefix.len())]);
        hash.update(&bytes);
        writer
            .0
            .as_mut()
            .expect("pending cache")
            .writeChunk(&bytes)
            .map_err(|e| e.to_string())?;
        offset += count as u64;
    }
    validateDownloadedUpdatePackage(&prefix, assetName)?;
    let identity = (format!("{:x}", hash.finalize()), size);
    if expected.is_some_and(|expected| expected != &identity) {
        return Err("Package changed during import".into());
    }
    ensureDownloadActive(control)?;
    writer
        .0
        .take()
        .expect("pending cache")
        .commit()
        .map_err(|e| e.to_string())?;
    Ok(identity)
}

/// Retains selected originals independently of releases while pruning obsolete packages in this family.
fn retainSource(
    source: &dyn FileSource,
    _currentAsset: &str,
    version: &str,
    assetName: &str,
    expected: Option<&(String, u64)>,
    context: &HostManager,
    control: &HttpDownloadControl,
) -> Result<(), String> {
    let root = packageCacheDirectory(version, assetName)?;
    let destination = format!("{root}/{assetName}");
    copyPackage(source, &destination, assetName, expected, context, control)?;
    let storage = context
        .runtimeStorageHost
        .as_ref()
        .ok_or("Runtime storage unavailable")?;
    for entry in storage.list(&root).map_err(|e| e.to_string())? {
        if !entry.isDirectory
            && entry.path != destination
            && !entry
                .path
                .rsplit('/')
                .next()
                .unwrap_or("")
                .starts_with("installer")
        {
            storage
                .delete(&entry.path, false)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub struct ImportedPackage {
    pub version: String,
    pub packagePath: Option<String>,
}

/// Imports a user-selected original installer without requiring release metadata or network access.
pub fn importLocalPackage(
    sourcePath: &str,
    currentVersion: &str,
    target: &FullUpdateTarget,
    context: &HostManager,
    control: &HttpDownloadControl,
) -> Result<ImportedPackage, String> {
    ensureDownloadActive(control)?;
    let currentAsset = target.assetNameForVersion(currentVersion)?;
    let marker = format!("-{currentVersion}.");
    let (family, extension) = currentAsset
        .split_once(&marker)
        .ok_or("Invalid current package identity")?;
    let name = sourcePath
        .rsplit('/')
        .next()
        .ok_or("Missing package filename")?;
    let version = name
        .strip_prefix(&format!("{family}-"))
        .and_then(|name| name.strip_suffix(&format!(".{extension}")))
        .ok_or(
            "Select the original installer for this platform, architecture and package format",
        )?;
    if target.assetNameForVersion(version)? != name {
        return Err("Package filename does not match this installation".into());
    }
    let comparison = GithubReleaseUtil::compareVersions(version, currentVersion)?;
    if comparison < 0 {
        return Err("An older application package cannot be imported".into());
    }
    let vfs = context.vfs()?;
    let source = vfs.file(sourcePath)?;
    if comparison == 0 {
        retainSource(
            &source,
            &currentAsset,
            version,
            name,
            None,
            context,
            control,
        )?;
        return Ok(ImportedPackage {
            version: version.into(),
            packagePath: None,
        });
    }
    retainSource(
        &source,
        &currentAsset,
        version,
        name,
        None,
        context,
        control,
    )?;
    Ok(ImportedPackage {
        version: version.into(),
        packagePath: Some(format!("{}/{name}", packageCacheDirectory(version, name)?)),
    })
}

/// Identifies one product, ABI, distribution and installer format independently of its version.
fn packageFamily(release: &ReleaseInfo) -> Result<String, String> {
    let marker = format!("-{}.", release.version);
    let (family, extension) = release
        .assetName
        .split_once(&marker)
        .ok_or("Invalid versioned package name")?;
    Ok(format!("{family}.{extension}"))
}

/// Separates package caches by product, ABI, distribution and installer format.
fn cacheDirectory(release: &ReleaseInfo) -> Result<String, String> {
    packageCacheDirectory(&release.version, &release.assetName)
}

/// Uses the same package identity for online downloads and manually supplied originals.
fn packageCacheDirectory(version: &str, assetName: &str) -> Result<String, String> {
    let marker = format!("-{version}.");
    let (family, extension) = assetName
        .split_once(&marker)
        .ok_or("Invalid versioned package name")?;
    Ok(format!(
        "{}/{family}.{extension}",
        crate::RuntimeStorageLayout::RUNTIME_APPLICATION_UPDATE_BASES_DIR_PATH,
    ))
}

/// Keeps one durable decision per package family without adding state to synchronized data.
fn policyPath(release: &ReleaseInfo) -> Result<String, String> {
    Ok(format!(
        "{}/{}.json",
        crate::RuntimeStorageLayout::RUNTIME_APPLICATION_UPDATE_STATE_DIR_PATH,
        packageFamily(release)?
    ))
}

/// Applies a previous incremental failure only to its exact target version and package.
fn requiresFullTransfer(release: &ReleaseInfo, context: &HostManager) -> Result<bool, String> {
    let storage = context
        .runtimeStorageHost
        .as_ref()
        .ok_or("Runtime storage unavailable")?;
    let path = policyPath(release)?;
    if !storage.exists(&path).map_err(|error| error.to_string())? {
        return Ok(false);
    }
    let bytes = storage
        .readBytes(&path)
        .map_err(|error| error.to_string())?;
    let policy: FullTransferPolicy =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    Ok(policy.version == release.version && policy.assetName == release.assetName)
}

/// Atomically records a failed target while preserving cancellation and the original error.
fn recordIncrementalFailure(
    release: &ReleaseInfo,
    context: &HostManager,
    control: &HttpDownloadControl,
    error: String,
) -> String {
    if control.isCancelled() {
        return "UPDATE_CANCELLED".into();
    }
    let save = || -> Result<(), String> {
        let bytes = serde_json::to_vec(&FullTransferPolicy {
            version: release.version.clone(),
            assetName: release.assetName.clone(),
        })
        .map_err(|error| error.to_string())?;
        let mut writer = PendingWrite(Some(
            context
                .runtimeStorageWriteHost
                .as_ref()
                .ok_or("Update writer unavailable")?
                .createWriteSession(&policyPath(release)?)
                .map_err(|error| error.to_string())?,
        ));
        writer
            .0
            .as_mut()
            .expect("pending update policy")
            .writeChunk(&bytes)
            .map_err(|error| error.to_string())?;
        ensureDownloadActive(control)?;
        writer
            .0
            .take()
            .expect("pending update policy")
            .commit()
            .map_err(|error| error.to_string())
    };
    let saved = save();
    if control.isCancelled() {
        return "UPDATE_CANCELLED".into();
    }
    match saved {
        Ok(()) => {
            format!("{error}; the next attempt for this version will download the full package")
        }
        Err(policyError) => {
            format!("{error}; could not record the full-download decision: {policyError}")
        }
    }
}

/// One authoritative transfer decision shared by interactive and unattended clients.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateMode {
    Full,
    Incremental,
    NeedsBase,
    Unavailable,
}

pub struct UpdatePlan {
    pub mode: UpdateMode,
    pub fullAvailable: bool,
    pub incrementalAvailable: bool,
    pub baseVersion: Option<String>,
    pub fullBytes: Option<u64>,
    pub incrementalBytes: Option<u64>,
    release: ReleaseInfo,
    currentAsset: String,
    manifest: Option<UpdateManifest>,
    candidates: Vec<usize>,
    selected: Option<usize>,
    base: Option<Box<dyn FileSource>>,
}
impl UpdatePlan {
    /// Applies the user's explicit full-package choice without inventing a missing asset.
    pub fn chooseFull(&mut self) {
        self.mode = if self.fullAvailable {
            UpdateMode::Full
        } else {
            UpdateMode::Unavailable
        };
        self.base = None;
    }

    /// Verifies a selected original against this plan before retaining it or downloading patches.
    pub fn supplyBase(
        &mut self,
        path: &str,
        context: &HostManager,
        control: &HttpDownloadControl,
    ) -> Result<(), String> {
        if self.mode != UpdateMode::NeedsBase {
            return Err("Update is not awaiting a base package".into());
        }
        let source = context.vfs()?.file(path)?;
        let identity = hashSource(&source, control)?;
        let manifest = self
            .manifest
            .as_ref()
            .ok_or("Missing authenticated update manifest")?;
        let index = self.candidates.iter().copied().find(|index| {
            let patch = &manifest.patches[*index];
            identity == (patch.baseSha256.clone(), patch.baseSize)
        }).ok_or("UPDATE_BASE_MISMATCH: selected installer does not match the required version, platform, architecture or original bytes")?;
        retainSource(
            &source,
            &self.currentAsset,
            &manifest.patches[index].baseVersion,
            &self.currentAsset,
            Some(&identity),
            context,
            control,
        )?;
        let cached = format!("{}/{}", cacheDirectory(&self.release)?, self.currentAsset);
        self.base = Some(Box::new(
            context
                .vfs()?
                .file(&VisualFileSystem::runtimePath(&cached)?)?,
        ));
        self.selected = Some(index);
        self.incrementalBytes = Some(manifest.patches[index].size);
        self.mode = UpdateMode::Incremental;
        Ok(())
    }
}

/// Resolves an installed or retained original through the Host without prompting the UI.
fn automaticBase(
    release: &ReleaseInfo,
    currentAsset: &str,
    context: &HostManager,
) -> Result<Option<Box<dyn FileSource>>, String> {
    let source = match &context.systemOperationHost {
        Some(host) => host
            .applicationUpdateBase()
            .map_err(|error| error.to_string())?,
        None => operit_host_api::ApplicationUpdateBase::RetainedPackage,
    };
    match source {
        operit_host_api::ApplicationUpdateBase::InstalledPackage(source) => Ok(Some(source)),
        operit_host_api::ApplicationUpdateBase::Unavailable => Ok(None),
        operit_host_api::ApplicationUpdateBase::RetainedPackage => {
            let cache = format!("{}/{currentAsset}", cacheDirectory(release)?);
            if context
                .runtimeStorageHost
                .as_ref()
                .ok_or("Runtime storage unavailable")?
                .exists(&cache)
                .map_err(|error| error.to_string())?
            {
                Ok(Some(Box::new(
                    context
                        .vfs()?
                        .file(&VisualFileSystem::runtimePath(&cache)?)?,
                )))
            } else {
                Ok(None)
            }
        }
    }
}

/// Calculates complete-package and applicable direct-patch availability before any package transfer.
pub async fn planUpdate(
    release: ReleaseInfo,
    currentAsset: String,
    context: &HostManager,
    control: HttpDownloadControl,
) -> Result<UpdatePlan, String> {
    validateUpdatePackageSource(&release.downloadUrl, &release.assetName)?;
    ensureDownloadActive(&control)?;
    let manifest = if requiresFullTransfer(&release, context)? {
        None
    } else {
        fetchManifest(&release, context, control.clone()).await?
    };
    let candidates: Vec<usize> = manifest
        .as_ref()
        .map(|manifest| {
            manifest
                .patches
                .iter()
                .enumerate()
                .filter(|(_, patch)| {
                    patch.size < manifest.size
                        && release
                            .patchAssetNames
                            .iter()
                            .any(|name| name == &patch.assetName)
                        && release.assetName.replace(
                            &format!("-{}.", release.version),
                            &format!("-{}.", patch.baseVersion),
                        ) == currentAsset
                })
                .map(|(index, _)| index)
                .collect()
        })
        .unwrap_or_default();
    let mut plan = UpdatePlan {
        mode: if release.fullPackageAvailable {
            UpdateMode::Full
        } else {
            UpdateMode::Unavailable
        },
        fullAvailable: release.fullPackageAvailable,
        incrementalAvailable: !candidates.is_empty(),
        baseVersion: candidates.first().map(|index| {
            manifest.as_ref().unwrap().patches[*index]
                .baseVersion
                .clone()
        }),
        fullBytes: manifest.as_ref().map(|m| m.size),
        incrementalBytes: candidates
            .iter()
            .map(|index| manifest.as_ref().unwrap().patches[*index].size)
            .min(),
        release,
        currentAsset,
        manifest,
        candidates,
        selected: None,
        base: None,
    };
    if preparedPackage(&plan.release, plan.manifest.as_ref(), context, &control)?.is_some() {
        // The complete verified target is already local, so no baseline prompt is necessary.
        plan.mode = UpdateMode::Full;
        return Ok(plan);
    }
    if plan.candidates.is_empty() {
        return Ok(plan);
    }
    let select = || -> Result<Option<(Box<dyn FileSource>, usize)>, String> {
        let Some(base) = automaticBase(&plan.release, &plan.currentAsset, context)? else {
            return Ok(None);
        };
        let (hash, size) = hashSource(base.as_ref(), &control)?;
        let manifest = plan.manifest.as_ref().unwrap();
        let index = plan
            .candidates
            .iter()
            .copied()
            .find(|index| {
                let patch = &manifest.patches[*index];
                patch.baseSha256 == hash && patch.baseSize == size
            })
            .ok_or("Installed or retained base does not match the incremental manifest")?;
        Ok(Some((base, index)))
    };
    match select()
        .map_err(|error| recordIncrementalFailure(&plan.release, context, &control, error))?
    {
        Some((base, index)) => {
            plan.incrementalBytes = Some(plan.manifest.as_ref().unwrap().patches[index].size);
            plan.base = Some(base);
            plan.selected = Some(index);
            plan.mode = UpdateMode::Incremental;
        }
        None => plan.mode = UpdateMode::NeedsBase,
    }
    Ok(plan)
}

/// Executes the exact planned choice; a missing full package is never requested.
pub async fn executeUpdate<F>(
    plan: UpdatePlan,
    context: &HostManager,
    control: HttpDownloadControl,
    onEvent: F,
) -> Result<String, String>
where
    F: Fn(FullUpdateProgressEvent) + Send + Sync + 'static,
{
    if let Some(path) = preparedPackage(&plan.release, plan.manifest.as_ref(), context, &control)? {
        ensureDownloadActive(&control)?;
        onEvent(FullUpdateProgressEvent::StageChanged {
            stage: FullUpdateStage::Ready,
            message: "Verified update package is ready".into(),
        });
        return Ok(path);
    }
    if plan.mode == UpdateMode::NeedsBase {
        return Err("UPDATE_BASE_REQUIRED".into());
    }
    if plan.mode == UpdateMode::Unavailable {
        return Err("UPDATE_UNAVAILABLE".into());
    }
    let UpdatePlan {
        release,
        currentAsset,
        manifest,
        base,
        selected,
        mode,
        ..
    } = plan;
    ensureDownloadActive(&control)?;
    let storage = context
        .runtimeStorageHost
        .clone()
        .ok_or("Runtime storage unavailable")?;
    let directory = format!(
        "{}/{}",
        crate::RuntimeStorageLayout::RUNTIME_APPLICATION_UPDATES_DIR_PATH,
        uuid::Uuid::new_v4()
    );
    let _attempt = DownloadAttempt {
        storage: storage.clone(),
        path: directory.clone(),
        published: false,
    };
    let destination = format!("{directory}/{}", release.assetName);
    let events = Arc::new(onEvent);
    let reconstructed = if mode == UpdateMode::Incremental {
        let manifest = manifest.as_ref().ok_or("Missing update manifest")?;
        let patch = &manifest.patches[selected.ok_or("Missing patch selection")?];
        let url = format!(
            "{}/{}",
            release
                .updateManifest
                .as_ref()
                .ok_or("Missing authenticated manifest source")?
                .url
                .rsplit_once('/')
                .ok_or("Invalid release URL")?
                .0,
            patch.assetName
        );
        validateUpdatePackageSource(&url, &patch.assetName)?;
        let patchPath = format!("{directory}/{}", patch.assetName);
        events(FullUpdateProgressEvent::StageChanged {
            stage: FullUpdateStage::DownloadingPackage,
            message: "Downloading incremental update".into(),
        });
        let progress = events.clone();
        let present = downloadOptionalUpdatePackage(
            &url,
            &patchPath,
            &patch.assetName,
            context,
            control.clone(),
            Some(&patch.sha256),
            move |event| progress(event),
        )
        .await
        .map_err(|error| {
            if error.starts_with("UPDATE_INTEGRITY:") {
                recordIncrementalFailure(&release, context, &control, error)
            } else {
                error
            }
        })?;
        if present {
            events(FullUpdateProgressEvent::StageChanged {
                stage: FullUpdateStage::ReconstructingPackage,
                message: "Reconstructing and verifying update package".into(),
            });
            let reconstruct = || -> Result<(), String> {
                let source = context
                    .vfs()?
                    .file(&VisualFileSystem::runtimePath(&patchPath)?)?;
                applyPatch(
                    base.as_ref().ok_or("Missing verified base")?.as_ref(),
                    &source,
                    manifest,
                    patch,
                    context,
                    &destination,
                    &control,
                )
            };
            reconstruct()
                .map_err(|error| recordIncrementalFailure(&release, context, &control, error))?;
            storage
                .delete(&patchPath, false)
                .map_err(|e| e.to_string())?;
        }
        present
    } else {
        false
    };
    if !reconstructed {
        if !release.fullPackageAvailable {
            return Err("UPDATE_UNAVAILABLE: incremental asset is no longer available and no full installer was published".into());
        }
        events(FullUpdateProgressEvent::StageChanged {
            stage: FullUpdateStage::DownloadingPackage,
            message: "Downloading full update package".into(),
        });
        let progress = events.clone();
        let expectedHash = release
            .packageSha256
            .as_deref()
            .map(githubHash)
            .transpose()?
            .or_else(|| manifest.as_ref().map(|manifest| manifest.sha256.as_str()));
        downloadUpdatePackage(
            &release.downloadUrl,
            &destination,
            &release.assetName,
            context,
            control.clone(),
            expectedHash,
            move |event| progress(event),
        )
        .await?;
        let source = context
            .vfs()?
            .file(&VisualFileSystem::runtimePath(&destination)?)?;
        let (hash, size) = hashSource(&source, &control)?;
        if let Some(digest) = &release.packageSha256 {
            if hash != githubHash(digest)? {
                return Err("Full package SHA-256 mismatch".into());
            }
        }
        if let Some(manifest) = &manifest {
            if hash != manifest.sha256 || size != manifest.size {
                return Err("Full package disagrees with manifest".into());
            }
        }
    }
    let source = context
        .vfs()?
        .file(&VisualFileSystem::runtimePath(&destination)?)?;
    retainSource(
        &source,
        &currentAsset,
        &release.version,
        &release.assetName,
        None,
        context,
        &control,
    )?;
    ensureDownloadActive(&control)?;
    events(FullUpdateProgressEvent::StageChanged {
        stage: FullUpdateStage::Ready,
        message: "Verified update package is ready".into(),
    });
    Ok(format!(
        "{}/{}",
        cacheDirectory(&release)?,
        release.assetName
    ))
}

/// Runs the shared plan for unattended clients that cannot request an original from the user.
pub async fn prepareUpdate<F>(
    release: ReleaseInfo,
    currentAsset: String,
    context: &HostManager,
    control: HttpDownloadControl,
    onEvent: F,
) -> Result<String, String>
where
    F: Fn(FullUpdateProgressEvent) + Send + Sync + 'static,
{
    let mut plan = planUpdate(release, currentAsset, context, control.clone()).await?;
    if plan.mode == UpdateMode::NeedsBase {
        plan.chooseFull();
    }
    executeUpdate(plan, context, control, onEvent).await
}

/// Reuses an authenticated complete target already retained by an earlier attempt.
fn preparedPackage(
    release: &ReleaseInfo,
    manifest: Option<&UpdateManifest>,
    context: &HostManager,
    control: &HttpDownloadControl,
) -> Result<Option<String>, String> {
    let path = format!("{}/{}", cacheDirectory(release)?, release.assetName);
    let storage = context
        .runtimeStorageHost
        .as_ref()
        .ok_or("Runtime storage unavailable")?;
    if !storage.exists(&path).map_err(|e| e.to_string())? {
        return Ok(None);
    }
    let expected = match manifest {
        Some(manifest) => manifest.sha256.as_str(),
        None => match &release.packageSha256 {
            Some(hash) => githubHash(hash)?,
            None => return Ok(None),
        },
    };
    let source = context
        .vfs()?
        .file(&VisualFileSystem::runtimePath(&path)?)?;
    let (hash, size) = hashSource(&source, control)?;
    if hash != expected || manifest.is_some_and(|m| m.size != size) {
        storage.delete(&path, false).map_err(|e| e.to_string())?;
        return Err("Retained update package failed verification".into());
    }
    Ok(Some(path))
}

/// Collects abandoned downloads and obsolete packages before any update operation can start.
pub fn cleanupUpdateCache(
    currentVersion: &str,
    currentAsset: &str,
    context: &HostManager,
) -> Result<(), String> {
    let storage = context
        .runtimeStorageHost
        .as_ref()
        .ok_or("Runtime storage unavailable")?;
    let jobs = crate::RuntimeStorageLayout::RUNTIME_APPLICATION_UPDATES_DIR_PATH;
    for entry in storage.list(jobs).map_err(|e| e.to_string())? {
        if entry.isDirectory
            && entry
                .path
                .rsplit('/')
                .next()
                .is_some_and(|name| uuid::Uuid::parse_str(name).is_ok())
        {
            // Legacy installers used this directory; leave their source until the installation is resolved.
            if storage
                .list(&entry.path)
                .map_err(|e| e.to_string())?
                .iter()
                .any(|file| file.path.ends_with(".ready"))
            {
                continue;
            }
            storage
                .delete(&entry.path, true)
                .map_err(|e| e.to_string())?;
        }
    }
    let root = packageCacheDirectory(currentVersion, currentAsset)?;
    let (family, extension) = currentAsset
        .split_once(&format!("-{currentVersion}."))
        .ok_or("Invalid current package identity")?;
    let installedBase = match &context.systemOperationHost {
        Some(host) => !matches!(
            host.applicationUpdateBase().map_err(|e| e.to_string())?,
            operit_host_api::ApplicationUpdateBase::RetainedPackage
        ),
        None => false,
    };
    let finished = storage
        .exists(&format!("{root}/installer.finished"))
        .map_err(|e| e.to_string())?;
    for entry in storage.list(&root).map_err(|e| e.to_string())? {
        if entry.isDirectory {
            continue;
        }
        let name = entry
            .path
            .rsplit('/')
            .next()
            .ok_or("Invalid cached package path")?;
        let obsolete = match name
            .strip_prefix(&format!("{family}-"))
            .and_then(|name| name.strip_suffix(&format!(".{extension}")))
        {
            Some(version) => {
                let comparison = GithubReleaseUtil::compareVersions(version, currentVersion)?;
                comparison < 0 || (comparison == 0 && installedBase)
            }
            None => finished && name.starts_with("installer") && name != "installer.finished",
        };
        if obsolete {
            storage
                .delete(&entry.path, false)
                .map_err(|e| e.to_string())?;
        }
    }
    if finished {
        storage
            .delete(&format!("{root}/installer.finished"), false)
            .map_err(|e| e.to_string())?;
    }
    let policy = format!(
        "{}/{}.{}.json",
        crate::RuntimeStorageLayout::RUNTIME_APPLICATION_UPDATE_STATE_DIR_PATH,
        family,
        extension
    );
    if storage.exists(&policy).map_err(|e| e.to_string())? {
        let state: FullTransferPolicy =
            serde_json::from_slice(&storage.readBytes(&policy).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if GithubReleaseUtil::compareVersions(&state.version, currentVersion)? <= 0 {
            storage.delete(&policy, false).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
