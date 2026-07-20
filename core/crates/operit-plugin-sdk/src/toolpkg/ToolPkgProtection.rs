#![allow(non_snake_case)]

use crate::toolpkg::ToolPkgParser::ToolPkgArchiveParser;
use chacha20poly1305::{
    aead::{AeadInPlace, KeyInit},
    ChaCha20Poly1305, Key, Nonce, Tag,
};
#[cfg(not(target_arch = "wasm32"))]
use rquickjs::{CatchResultExt, Context as QuickJsContext, Runtime as QuickJsRuntime};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;

pub const PROTECTION_ID: &str = "operit-protected";
pub const MARKET_ONLY_PROTECTION_ID: &str = "operit-market-only";
pub const MARKET_INSTALL_SEAL_ENTRY_NAME: &str = ".operit/market-install.seal";

const MAGIC: &[u8; 8] = b"OPTPROTA";
const MARKET_ONLY_MAGIC: &[u8; 8] = b"OPMKTPKG";
const MARKET_ARCHIVE_MAGIC: &[u8; 8] = b"OPMARCH1";
const MARKET_INSTALL_SEAL_MAGIC: &[u8; 8] = b"OPMINST1";
const MARKET_ARCHIVE_FORMAT_VERSION: u8 = 1;
const NONCE_SIZE: usize = 12;
const TAG_SIZE: usize = 16;
const SHA256_SIZE: usize = 32;
const HEADER_SIZE: usize = MAGIC.len() + NONCE_SIZE + TAG_SIZE;
pub const MARKET_ONLY_PROTECTION_HEADER_SIZE: usize = MARKET_ONLY_MAGIC.len() + SHA256_SIZE;
const MARKET_ONLY_HEADER_SIZE: usize = MARKET_ONLY_PROTECTION_HEADER_SIZE + NONCE_SIZE + TAG_SIZE;
const MARKET_ONLY_PREFIX_SIZE: usize = MARKET_ONLY_PROTECTION_HEADER_SIZE;
const MARKET_ARCHIVE_AUTH_PREFIX_SIZE: usize = MARKET_ARCHIVE_MAGIC.len() + 1 + 8;
const MARKET_ARCHIVE_HEADER_SIZE: usize = MARKET_ARCHIVE_AUTH_PREFIX_SIZE + SHA256_SIZE;
pub const MARKET_INSTALLATION_ID_SIZE: usize = 16;
const DEFAULT_SCRIPT_ENTRY_NAME: &str = "artifact.js";
const TERSER_BUNDLE: &str = include_str!("vendor/terser.bundle.min.js");
const MINIFIER_BOOTSTRAP: &str = r#"
(function(root) {
    if (!root.Terser || typeof root.Terser.minify_sync !== "function") {
        throw new Error("Terser minify_sync is not available");
    }

    root.__operitToolPkgAstMinify = function(source, entryName) {
        var normalizedEntryName = String(entryName);
        var result = root.Terser.minify_sync(String(source), {
            ecma: 2020,
            module: /\.mjs$/i.test(normalizedEntryName),
            compress: false,
            mangle: false,
            format: {
                comments: false
            },
            sourceMap: false
        });

        if (!result || typeof result.code !== "string") {
            throw new Error("Terser did not return minified code for " + normalizedEntryName);
        }
        return result.code;
    };
})(typeof globalThis !== "undefined" ? globalThis : this);
"#;

#[cfg(operit_private_toolpkg_protection)]
mod privateMaterial {
    include!(concat!(env!("OUT_DIR"), "/private_toolpkg_protection.rs"));
}

const DECOY_PRIVATE_SIZE: usize = 32;
#[used]
static DECOY_ALPHA: [u8; DECOY_PRIVATE_SIZE] = [
    0x4d, 0xd1, 0x18, 0x72, 0xb4, 0x2e, 0x90, 0x5b, 0xc3, 0x0f, 0x69, 0xa2, 0x37, 0x8c, 0x51, 0xfe,
    0x26, 0x9a, 0x44, 0xbd, 0x07, 0xe3, 0x5e, 0x71, 0xd8, 0x13, 0xaf, 0x62, 0x3c, 0x95, 0x0a, 0xf0,
];
#[used]
static DECOY_BETA: [u8; DECOY_PRIVATE_SIZE] = [
    0x96, 0x23, 0xfa, 0x4c, 0x10, 0x87, 0x3d, 0xe2, 0x58, 0xb1, 0x06, 0xcd, 0x79, 0x34, 0xae, 0x65,
    0xdb, 0x40, 0x1c, 0x93, 0x6f, 0x28, 0xc6, 0x05, 0xba, 0x77, 0x4e, 0xd0, 0x19, 0x82, 0x3a, 0xe5,
];
#[used]
static DECOY_GAMMA: [u8; DECOY_PRIVATE_SIZE] = [
    0x21, 0xa9, 0x56, 0xc8, 0x3f, 0x04, 0xde, 0x75, 0x1b, 0x84, 0xf2, 0x49, 0x90, 0x2d, 0xb7, 0x6a,
    0x0e, 0xd4, 0x63, 0x18, 0xc1, 0x5a, 0x8f, 0x32, 0xe9, 0x07, 0x74, 0xbc, 0x45, 0x9e, 0x20, 0xd7,
];
#[used]
static DECOY_DELTA: [u8; DECOY_PRIVATE_SIZE] = [
    0x93, 0x59, 0xd1, 0x49, 0x39, 0x2f, 0x70, 0x0d, 0xa0, 0xa1, 0x8f, 0x6b, 0x82, 0x1c, 0x70, 0x8a,
    0xb4, 0xf4, 0x9d, 0x89, 0x8a, 0x0e, 0xb0, 0xe8, 0x02, 0x9b, 0x15, 0xdf, 0x17, 0xe7, 0xa1, 0x3b,
];
#[used]
static DECOY_PERMUTATION: [u8; DECOY_PRIVATE_SIZE] = [
    15, 2, 29, 7, 22, 11, 0, 18, 31, 5, 26, 9, 20, 1, 28, 13, 24, 4, 17, 30, 8, 21, 12, 27, 3, 19,
    6, 25, 10, 16, 14, 23,
];
#[used]
static DECOY_SELECTOR: [u8; DECOY_PRIVATE_SIZE] = [
    3, 1, 2, 0, 1, 3, 0, 2, 3, 0, 2, 1, 0, 3, 1, 2, 2, 0, 3, 1, 0, 2, 1, 3, 3, 1, 0, 2, 1, 3, 2, 0,
];
#[used]
static DECOY_MIXER: [u8; DECOY_PRIVATE_SIZE] = [
    0x3d, 0x8c, 0x61, 0x27, 0xe4, 0x59, 0x12, 0xa6, 0x74, 0xc0, 0x35, 0x9b, 0x48, 0xf1, 0x06, 0xda,
    0x57, 0x2c, 0xe8, 0x31, 0x9f, 0x64, 0x0b, 0xb5, 0x42, 0xd9, 0x1e, 0x83, 0x6c, 0x25, 0xfa, 0x50,
];

#[allow(non_snake_case)]
/// Returns whether bytes use the Operit 1 protected artifact envelope.
pub fn isProtected(bytes: &[u8]) -> bool {
    bytes.len() >= MAGIC.len() && &bytes[..MAGIC.len()] == MAGIC
}

#[allow(non_snake_case)]
/// Returns whether bytes use the authenticated marketplace-only entry envelope.
pub fn isMarketOnlyProtected(bytes: &[u8]) -> bool {
    bytes.len() >= MARKET_ONLY_PREFIX_SIZE && &bytes[..MARKET_ONLY_MAGIC.len()] == MARKET_ONLY_MAGIC
}

#[allow(non_snake_case)]
/// Returns whether bytes use either supported encrypted ToolPkg entry envelope.
pub fn isProtectedEntry(bytes: &[u8]) -> bool {
    isProtected(bytes) || isMarketOnlyProtected(bytes)
}

#[allow(non_snake_case)]
/// Returns whether bytes begin with the signed marketplace archive envelope.
pub fn isMarketArchive(bytes: &[u8]) -> bool {
    bytes.len() >= MARKET_ARCHIVE_MAGIC.len()
        && &bytes[..MARKET_ARCHIVE_MAGIC.len()] == MARKET_ARCHIVE_MAGIC
}

#[allow(non_snake_case)]
/// Computes the immutable marketplace-only policy digest embedded in every protected entry.
pub fn marketOnlyPolicyDigest(toolpkgId: &str, version: &str) -> [u8; SHA256_SIZE] {
    sha256(
        format!(
            "toolpkg_id={}\nversion={}\nmarket_only=true",
            toolpkgId.trim(),
            version.trim()
        )
        .as_bytes(),
    )
}

#[allow(non_snake_case)]
/// Verifies that a marketplace-only entry carries one exact manifest policy digest.
pub fn hasMarketOnlyPolicyDigest(bytes: &[u8], expectedDigest: &[u8; SHA256_SIZE]) -> bool {
    isMarketOnlyProtected(bytes)
        && constantTimeEquals(
            &bytes[MARKET_ONLY_MAGIC.len()..MARKET_ONLY_PREFIX_SIZE],
            expectedDigest,
        )
}

#[allow(non_snake_case)]
/// Decrypts protected bytes and returns plain bytes for unprotected input.
pub fn decryptIfNeeded(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if isMarketOnlyProtected(bytes) {
        decryptMarket(bytes)
    } else if isProtected(bytes) {
        decrypt(bytes)
    } else {
        Ok(bytes.to_vec())
    }
}

#[allow(non_snake_case)]
/// Decrypts protected bytes if needed and decodes the result as UTF-8.
pub fn decodeUtf8(bytes: &[u8]) -> Result<String, String> {
    String::from_utf8(decryptIfNeeded(bytes)?).map_err(|e| e.to_string())
}

#[allow(non_snake_case)]
/// Returns whether a protection secret is configured for this process or build.
pub fn isSecretConfigured() -> bool {
    true
}

/// Encrypts one byte slice with the Operit 1 protected artifact envelope.
pub fn encrypt(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.is_empty() {
        return Err("Cannot protect empty content".to_string());
    }
    if isProtected(bytes) {
        return Ok(bytes.to_vec());
    }
    let mut key = deriveAeadKey();
    let nonce = randomNonce();
    let associatedData = buildAssociatedData(&nonce);
    let mut ciphertext = bytes.to_vec();
    let tag = encryptDetached(&key, &nonce, &associatedData, &mut ciphertext)?;
    let mut output = Vec::with_capacity(HEADER_SIZE + ciphertext.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(tag.as_slice());
    output.extend_from_slice(&ciphertext);
    clearKeyMaterial(&mut key);
    Ok(output)
}

#[allow(non_snake_case)]
/// Encrypts one ToolPkg entry with its immutable marketplace-only policy digest.
pub fn encryptMarket(bytes: &[u8], policyDigest: &[u8; SHA256_SIZE]) -> Result<Vec<u8>, String> {
    if bytes.is_empty() {
        return Err("Cannot protect empty content".to_string());
    }
    let mut key = deriveAeadKey();
    let nonce = randomNonce();
    let associatedData = buildMarketAssociatedData(policyDigest, &nonce);
    let mut ciphertext = bytes.to_vec();
    let tag = encryptDetached(&key, &nonce, &associatedData, &mut ciphertext)?;
    let mut output = Vec::with_capacity(MARKET_ONLY_HEADER_SIZE + ciphertext.len());
    output.extend_from_slice(MARKET_ONLY_MAGIC);
    output.extend_from_slice(policyDigest);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(tag.as_slice());
    output.extend_from_slice(&ciphertext);
    clearKeyMaterial(&mut key);
    Ok(output)
}

#[allow(non_snake_case)]
/// Wraps a protected ToolPkg ZIP in the authenticated marketplace archive envelope.
pub fn wrapMarketArchive(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.is_empty() {
        return Err("Cannot wrap an empty ToolPkg archive".to_string());
    }
    let mut authenticated = Vec::with_capacity(MARKET_ARCHIVE_AUTH_PREFIX_SIZE + bytes.len());
    authenticated.extend_from_slice(MARKET_ARCHIVE_MAGIC);
    authenticated.push(MARKET_ARCHIVE_FORMAT_VERSION);
    authenticated.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    authenticated.extend_from_slice(bytes);
    let mut macKey = deriveMarketArchiveMacKey();
    let tag = hmacSha256(&macKey, &authenticated);
    let mut output = Vec::with_capacity(MARKET_ARCHIVE_HEADER_SIZE + bytes.len());
    output.extend_from_slice(&authenticated[..MARKET_ARCHIVE_AUTH_PREFIX_SIZE]);
    output.extend_from_slice(&tag);
    output.extend_from_slice(bytes);
    clearKeyMaterial(&mut macKey);
    authenticated.fill(0);
    Ok(output)
}

#[allow(non_snake_case)]
/// Authenticates and unwraps one marketplace archive into its raw ToolPkg ZIP bytes.
pub fn unwrapMarketArchive(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() < MARKET_ARCHIVE_HEADER_SIZE
        || !isMarketArchive(bytes)
        || bytes[MARKET_ARCHIVE_MAGIC.len()] != MARKET_ARCHIVE_FORMAT_VERSION
    {
        return Err("Not an Operit market ToolPkg archive".to_string());
    }
    let declaredSizeOffset = MARKET_ARCHIVE_MAGIC.len() + 1;
    let declaredSize = u64::from_le_bytes(
        bytes[declaredSizeOffset..MARKET_ARCHIVE_AUTH_PREFIX_SIZE]
            .try_into()
            .map_err(|_| "Market archive length authentication failed".to_string())?,
    );
    let payload = &bytes[MARKET_ARCHIVE_HEADER_SIZE..];
    if declaredSize != payload.len() as u64 {
        return Err("Market archive length authentication failed".to_string());
    }
    let mut authenticated = Vec::with_capacity(MARKET_ARCHIVE_AUTH_PREFIX_SIZE + payload.len());
    authenticated.extend_from_slice(&bytes[..MARKET_ARCHIVE_AUTH_PREFIX_SIZE]);
    authenticated.extend_from_slice(payload);
    let mut macKey = deriveMarketArchiveMacKey();
    let expectedTag = hmacSha256(&macKey, &authenticated);
    let result = if constantTimeEquals(
        &bytes[MARKET_ARCHIVE_AUTH_PREFIX_SIZE..MARKET_ARCHIVE_HEADER_SIZE],
        &expectedTag,
    ) {
        Ok(payload.to_vec())
    } else {
        Err("Market archive authentication failed".to_string())
    };
    clearKeyMaterial(&mut macKey);
    authenticated.fill(0);
    result
}

#[allow(non_snake_case)]
/// Returns whether a raw ToolPkg ZIP contains at least one protected non-manifest entry.
pub fn toolPkgArchiveContainsProtectedEntries(bytes: &[u8]) -> Result<bool, String> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
    let manifestPreview =
        ToolPkgArchiveParser::readToolPkgManifestPreview(&mut archive, &entryIndex)
            .ok_or_else(|| "manifest.hjson or manifest.json not found".to_string())?;
    for entryName in &entryIndex.entryNames {
        if entryName.eq_ignore_ascii_case(&manifestPreview.entryName) {
            continue;
        }
        if entryName.eq_ignore_ascii_case(MARKET_INSTALL_SEAL_ENTRY_NAME) {
            continue;
        }
        let header = ToolPkgArchiveParser::readZipEntryPrefix(
            &mut archive,
            &entryIndex,
            entryName,
            MARKET_ONLY_PROTECTION_HEADER_SIZE,
        )
        .ok_or_else(|| format!("Unable to read ToolPkg entry '{entryName}'"))?;
        if isProtectedEntry(&header) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[allow(non_snake_case)]
/// Creates a random client installation identifier used to bind local market package seals.
pub fn createMarketInstallationId() -> [u8; MARKET_INSTALLATION_ID_SIZE] {
    let uuidBytes = *uuid::Uuid::new_v4().as_bytes();
    let mut installationId = [0u8; MARKET_INSTALLATION_ID_SIZE];
    installationId.copy_from_slice(&uuidBytes);
    installationId
}

#[allow(non_snake_case)]
/// Adds a device-bound authenticated installation seal to one verified raw marketplace ToolPkg ZIP.
pub fn attachMarketInstallSeal(
    rawArchive: &[u8],
    installationId: &[u8; MARKET_INSTALLATION_ID_SIZE],
) -> Result<Vec<u8>, String> {
    let entries = readMarketInstallArchiveEntries(rawArchive, true)?;
    let archiveDigest = marketInstallArchiveDigest(&entries);
    let mut payload = Vec::with_capacity(installationId.len() + archiveDigest.len());
    payload.extend_from_slice(installationId);
    payload.extend_from_slice(&archiveDigest);
    let wrappedPayload = wrapMarketArchive(&payload)?;
    payload.fill(0);
    let mut seal = Vec::with_capacity(MARKET_INSTALL_SEAL_MAGIC.len() + wrappedPayload.len());
    seal.extend_from_slice(MARKET_INSTALL_SEAL_MAGIC);
    seal.extend_from_slice(&wrappedPayload);
    writeMarketInstallArchiveEntries(&entries, &seal)
}

#[allow(non_snake_case)]
/// Verifies that one installed ToolPkg ZIP has an unmodified local market installation seal.
pub fn verifyMarketInstallSeal(
    archiveBytes: &[u8],
    installationId: &[u8; MARKET_INSTALLATION_ID_SIZE],
) -> Result<bool, String> {
    let entries = readMarketInstallArchiveEntries(archiveBytes, false)?;
    let seals = entries
        .iter()
        .filter(|entry| {
            entry
                .name
                .eq_ignore_ascii_case(MARKET_INSTALL_SEAL_ENTRY_NAME)
        })
        .collect::<Vec<_>>();
    if seals.len() != 1
        || seals[0].isDirectory
        || !hasPrefix(&seals[0].content, MARKET_INSTALL_SEAL_MAGIC)
    {
        return Ok(false);
    }
    let payload = unwrapMarketArchive(&seals[0].content[MARKET_INSTALL_SEAL_MAGIC.len()..])?;
    if payload.len() != MARKET_INSTALLATION_ID_SIZE + SHA256_SIZE {
        return Ok(false);
    }
    let archiveDigest = marketInstallArchiveDigest(&entries);
    Ok(
        constantTimeEquals(&payload[..MARKET_INSTALLATION_ID_SIZE], installationId)
            && constantTimeEquals(&payload[MARKET_INSTALLATION_ID_SIZE..], &archiveDigest),
    )
}

#[allow(non_snake_case)]
/// Protects one JavaScript or ToolPkg artifact supplied as bytes.
pub fn protectArtifactBytes(sourceBytes: &[u8], isToolPkg: bool) -> Result<Vec<u8>, String> {
    protectArtifactNamedBytes(sourceBytes, DEFAULT_SCRIPT_ENTRY_NAME, isToolPkg)
}

#[allow(non_snake_case)]
/// Protects one named JavaScript or ToolPkg artifact supplied as bytes.
pub fn protectArtifactNamedBytes(
    sourceBytes: &[u8],
    sourceEntryName: &str,
    isToolPkg: bool,
) -> Result<Vec<u8>, String> {
    let mut minifier = ToolPkgJsAstMinifier::new()?;
    if isToolPkg {
        minifyToolPkgArchive(sourceBytes, &mut minifier)
    } else {
        astMinifyBytes(sourceBytes, sourceEntryName, &mut minifier)
    }
}

/// Decrypts one protected Operit 1 artifact payload.
fn decrypt(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() < HEADER_SIZE || &bytes[..MAGIC.len()] != MAGIC {
        return Err("Not an Operit protected payload".to_string());
    }
    let mut key = deriveAeadKey();
    let mut nonce = [0u8; NONCE_SIZE];
    nonce.copy_from_slice(&bytes[MAGIC.len()..MAGIC.len() + NONCE_SIZE]);
    let providedTag = &bytes[MAGIC.len() + NONCE_SIZE..HEADER_SIZE];
    let ciphertext = &bytes[HEADER_SIZE..];
    let associatedData = buildAssociatedData(&nonce);
    let mut plaintext = ciphertext.to_vec();
    let result = decryptDetached(&key, &nonce, &associatedData, &mut plaintext, providedTag)
        .map(|()| plaintext);
    clearKeyMaterial(&mut key);
    result
}

/// Decrypts one marketplace-only ToolPkg entry after authenticating its policy-bound header.
fn decryptMarket(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() < MARKET_ONLY_HEADER_SIZE || !isMarketOnlyProtected(bytes) {
        return Err("Not an Operit market protected payload".to_string());
    }
    let policyDigest: [u8; SHA256_SIZE] = bytes[MARKET_ONLY_MAGIC.len()..MARKET_ONLY_PREFIX_SIZE]
        .try_into()
        .map_err(|_| "Market protected payload is malformed".to_string())?;
    let nonceOffset = MARKET_ONLY_PREFIX_SIZE;
    let mut nonce = [0u8; NONCE_SIZE];
    nonce.copy_from_slice(&bytes[nonceOffset..nonceOffset + NONCE_SIZE]);
    let providedTag = &bytes[nonceOffset + NONCE_SIZE..MARKET_ONLY_HEADER_SIZE];
    let associatedData = buildMarketAssociatedData(&policyDigest, &nonce);
    let mut plaintext = bytes[MARKET_ONLY_HEADER_SIZE..].to_vec();
    let mut key = deriveAeadKey();
    let result = decryptDetached(&key, &nonce, &associatedData, &mut plaintext, providedTag)
        .map(|()| plaintext);
    clearKeyMaterial(&mut key);
    result
}

#[allow(non_snake_case)]
/// AST-minifies executable ToolPkg entries while preserving the standard ZIP structure.
fn minifyToolPkgArchive(
    sourceBytes: &[u8],
    minifier: &mut ToolPkgJsAstMinifier,
) -> Result<Vec<u8>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(sourceBytes)).map_err(|e| e.to_string())?;
    let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
    let mPreview = ToolPkgArchiveParser::readToolPkgManifestPreview(&mut archive, &entryIndex)
        .ok_or_else(|| "manifest.hjson or manifest.json not found".to_string())?;
    let manifestBasePath = mPreview
        .entryName
        .rsplit_once('/')
        .map(|(basePath, _)| basePath)
        .unwrap_or("")
        .to_string();
    let manifestEntryName = ToolPkgArchiveParser::normalizeZipEntryPath(&mPreview.entryName)
        .ok_or_else(|| "Invalid toolpkg manifest entry name".to_string())?;
    let mut astMinifiedEntryNames = BTreeSet::new();
    let mut resourceEntryRoots = BTreeSet::new();
    if let Some(mainEntry) = ToolPkgArchiveParser::resolveManifestRelativeZipEntryPath(
        &manifestBasePath,
        &mPreview.manifest.main,
    ) {
        astMinifiedEntryNames.insert(mainEntry);
    }
    for subpackage in &mPreview.manifest.subpackages {
        if let Some(entry) = ToolPkgArchiveParser::resolveManifestRelativeZipEntryPath(
            &manifestBasePath,
            &subpackage.entry,
        ) {
            astMinifiedEntryNames.insert(entry);
        }
    }
    for resource in &mPreview.manifest.resources {
        if let Some(root) = ToolPkgArchiveParser::resolveManifestRelativeResourcePath(
            &manifestBasePath,
            &resource.path,
        ) {
            resourceEntryRoots.insert(root);
        }
    }
    let mut out = Vec::new();
    {
        let mut w = zip::ZipWriter::new(Cursor::new(&mut out));
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = entry.name().to_string();
            let mut options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            if let Some(lastModified) = entry.last_modified() {
                options = options.last_modified_time(lastModified);
            }
            if entry.is_dir() {
                w.add_directory(name, options).map_err(|e| e.to_string())?;
                continue;
            }
            let mut orig = Vec::new();
            entry.read_to_end(&mut orig).map_err(|e| e.to_string())?;
            let norm = ToolPkgArchiveParser::normalizeZipEntryPath(&name);
            let data = match norm.as_deref() {
                None => orig,
                Some(norm) if norm == manifestEntryName => orig,
                Some(norm)
                    if shouldAstMinifyToolPkgEntry(
                        norm,
                        &astMinifiedEntryNames,
                        &resourceEntryRoots,
                    ) =>
                {
                    astMinifyBytes(&orig, norm, minifier)?
                }
                Some(_) => orig,
            };
            w.start_file(name, options).map_err(|e| e.to_string())?;
            w.write_all(&data).map_err(|e| e.to_string())?;
        }
        w.finish().map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// Returns whether a normalized ToolPkg archive entry should be AST-minified.
fn shouldAstMinifyToolPkgEntry(
    norm: &str,
    astMinifiedEntryNames: &BTreeSet<String>,
    resourceEntryRoots: &BTreeSet<String>,
) -> bool {
    astMinifiedEntryNames.contains(norm)
        && !resourceEntryRoots
            .iter()
            .any(|root| norm == root || norm.starts_with(&format!("{root}/")))
}

/// Holds one ZIP entry while a marketplace installation seal is being attached or verified.
struct MarketInstallArchiveEntry {
    name: String,
    lastModified: Option<zip::DateTime>,
    isDirectory: bool,
    content: Vec<u8>,
}

/// Reads a ToolPkg ZIP into normalized entries and optionally rejects an existing installation seal.
fn readMarketInstallArchiveEntries(
    bytes: &[u8],
    rejectExistingSeal: bool,
) -> Result<Vec<MarketInstallArchiveEntry>, String> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let name = ToolPkgArchiveParser::normalizeZipEntryPath(entry.name())
            .ok_or_else(|| format!("Invalid ToolPkg ZIP entry: {}", entry.name()))?;
        if rejectExistingSeal && name.eq_ignore_ascii_case(MARKET_INSTALL_SEAL_ENTRY_NAME) {
            return Err("ToolPkg already contains a market installation seal".to_string());
        }
        let isDirectory = entry.is_dir();
        let mut content = Vec::new();
        if !isDirectory {
            entry
                .read_to_end(&mut content)
                .map_err(|error| error.to_string())?;
        }
        entries.push(MarketInstallArchiveEntry {
            name,
            lastModified: entry.last_modified(),
            isDirectory,
            content,
        });
    }
    Ok(entries)
}

/// Computes the canonical digest of every non-directory ToolPkg entry except the installation seal.
fn marketInstallArchiveDigest(entries: &[MarketInstallArchiveEntry]) -> [u8; SHA256_SIZE] {
    let mut ordered = entries
        .iter()
        .filter(|entry| !entry.isDirectory)
        .filter(|entry| {
            !entry
                .name
                .eq_ignore_ascii_case(MARKET_INSTALL_SEAL_ENTRY_NAME)
        })
        .collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
    let mut digest = Sha256::new();
    for entry in ordered {
        digest.update(entry.name.as_bytes());
        digest.update([0]);
        digest.update((entry.content.len() as u64).to_be_bytes());
        digest.update(&entry.content);
    }
    let mut output = [0u8; SHA256_SIZE];
    output.copy_from_slice(&digest.finalize());
    output
}

/// Writes normalized ToolPkg entries and one installation seal back to a compressed ZIP archive.
fn writeMarketInstallArchiveEntries(
    entries: &[MarketInstallArchiveEntry],
    seal: &[u8],
) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    {
        let mut archive = zip::ZipWriter::new(Cursor::new(&mut output));
        for entry in entries {
            let mut options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            if let Some(lastModified) = entry.lastModified.clone() {
                options = options.last_modified_time(lastModified);
            }
            if entry.isDirectory {
                archive
                    .add_directory(entry.name.clone(), options)
                    .map_err(|error| error.to_string())?;
                continue;
            }
            archive
                .start_file(entry.name.clone(), options)
                .map_err(|error| error.to_string())?;
            archive
                .write_all(&entry.content)
                .map_err(|error| error.to_string())?;
        }
        archive
            .start_file(
                MARKET_INSTALL_SEAL_ENTRY_NAME,
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            )
            .map_err(|error| error.to_string())?;
        archive.write_all(seal).map_err(|error| error.to_string())?;
        archive.finish().map_err(|error| error.to_string())?;
    }
    Ok(output)
}

/// Tests whether bytes begin with an exact binary marker.
fn hasPrefix(bytes: &[u8], prefix: &[u8]) -> bool {
    bytes.len() >= prefix.len() && &bytes[..prefix.len()] == prefix
}

#[allow(non_snake_case)]
/// AST-minifies UTF-8 JavaScript-like bytes through the bundled Terser parser/printer.
fn astMinifyBytes(
    bytes: &[u8],
    entryName: &str,
    minifier: &mut ToolPkgJsAstMinifier,
) -> Result<Vec<u8>, String> {
    let source = String::from_utf8(bytes.to_vec()).map_err(|error| error.to_string())?;
    let minified = astMinifySourcePreservingMetadata(&source, entryName, minifier)?;
    Ok(minified.into_bytes())
}

#[allow(non_snake_case)]
/// Preserves the package metadata block and AST-minifies the executable body.
fn astMinifySourcePreservingMetadata(
    source: &str,
    entryName: &str,
    minifier: &mut ToolPkgJsAstMinifier,
) -> Result<String, String> {
    if let Some((metadataBlock, body)) = splitLeadingMetadataBlock(source) {
        let body = body.trim();
        if body.is_empty() {
            return Err(format!(
                "JavaScript body after METADATA is empty for {entryName}"
            ));
        }
        let minifiedBody = minifier.minify(body, entryName)?;
        return Ok(format!("{metadataBlock}{minifiedBody}"));
    }
    minifier.minify(source, entryName)
}

#[allow(non_snake_case)]
/// Splits one leading standalone package metadata block from its executable body.
fn splitLeadingMetadataBlock(source: &str) -> Option<(&str, &str)> {
    let trimmed = source.trim_start();
    let leadingWhitespaceSize = source.len() - trimmed.len();
    if !trimmed.starts_with("/*") {
        return None;
    }
    let commentBody = &trimmed[2..];
    let label = commentBody.trim_start();
    if !startsWithMetadataLabel(label) {
        return None;
    }
    let commentEnd = trimmed.find("*/")? + 2;
    let metadataEnd = leadingWhitespaceSize + commentEnd;
    Some((&source[..metadataEnd], &source[metadataEnd..]))
}

#[allow(non_snake_case)]
/// Returns whether the comment body starts with the exact METADATA marker.
fn startsWithMetadataLabel(commentBody: &str) -> bool {
    let Some(afterLabel) = commentBody.strip_prefix("METADATA") else {
        return false;
    };
    match afterLabel.chars().next() {
        Some(ch) => ch.is_whitespace() || ch == '*',
        None => true,
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct ToolPkgJsAstMinifier {
    _runtime: QuickJsRuntime,
    context: QuickJsContext,
}

#[cfg(not(target_arch = "wasm32"))]
impl ToolPkgJsAstMinifier {
    /// Creates a QuickJS-backed Terser minifier instance.
    fn new() -> Result<Self, String> {
        if TERSER_BUNDLE.trim().is_empty() {
            return Err("Terser bundle is empty".to_string());
        }
        let runtime = QuickJsRuntime::new().map_err(|error| error.to_string())?;
        let context = QuickJsContext::full(&runtime).map_err(|error| error.to_string())?;
        let minifier = Self {
            _runtime: runtime,
            context,
        };
        minifier.evalVoid(TERSER_BUNDLE)?;
        minifier.evalVoid(MINIFIER_BOOTSTRAP)?;
        Ok(minifier)
    }

    /// Minifies one JavaScript source string for a named entry.
    fn minify(&mut self, source: &str, entryName: &str) -> Result<String, String> {
        if source.is_empty() {
            return Err(format!(
                "Cannot AST-minify empty JavaScript entry: {entryName}"
            ));
        }
        if entryName.trim().is_empty() {
            return Err("JavaScript entry name is required for AST minification".to_string());
        }
        let sourceJson = serde_json::to_string(source).map_err(|error| error.to_string())?;
        let entryNameJson = serde_json::to_string(entryName).map_err(|error| error.to_string())?;
        let script = format!("__operitToolPkgAstMinify({sourceJson},{entryNameJson});");
        let minified = self.evalString(&script)?;
        if minified.is_empty() {
            return Err(format!("AST-minified output is empty for {entryName}"));
        }
        Ok(minified)
    }

    /// Evaluates one QuickJS script and discards its result.
    fn evalVoid(&self, script: &str) -> Result<(), String> {
        let wrapped = format!("{script}\nvoid 0;");
        self.context.with(|ctx| {
            ctx.eval::<(), _>(wrapped.as_str())
                .catch(&ctx)
                .map_err(|error| error.to_string())
        })
    }

    /// Evaluates one QuickJS script and returns a string result.
    fn evalString(&self, script: &str) -> Result<String, String> {
        self.context.with(|ctx| {
            ctx.eval::<String, _>(script)
                .catch(&ctx)
                .map_err(|error| error.to_string())
        })
    }
}

#[cfg(target_arch = "wasm32")]
struct ToolPkgJsAstMinifier;

#[cfg(target_arch = "wasm32")]
impl ToolPkgJsAstMinifier {
    /// Reports that artifact protection is unavailable in wasm32 builds.
    fn new() -> Result<Self, String> {
        Err("ToolPkg artifact protection is not available on wasm32".to_string())
    }

    /// Reports that JavaScript AST minification is unavailable in wasm32 builds.
    fn minify(&mut self, _source: &str, entryName: &str) -> Result<String, String> {
        Err(format!(
            "ToolPkg JavaScript AST minification is not available for {entryName} on wasm32"
        ))
    }
}

/// Reconstructs the release secret from generated private material or the public decoy material.
fn protectionKey() -> Vec<u8> {
    #[cfg(operit_private_toolpkg_protection)]
    {
        return privateMaterial::loadPrivateProtectionSecret();
    }
    #[cfg(not(operit_private_toolpkg_protection))]
    {
        decoyProtectionKey()
    }
}

/// Reconstructs the public non-production key through the same volatile share topology.
#[cfg(not(operit_private_toolpkg_protection))]
#[inline(never)]
fn decoyProtectionKey() -> Vec<u8> {
    let mut output = Vec::with_capacity(DECOY_PRIVATE_SIZE);
    for logicalIndex in 0..DECOY_PRIVATE_SIZE {
        let physicalIndex = readDecoyByte(&DECOY_PERMUTATION, logicalIndex) as usize;
        let alpha = readDecoyByte(&DECOY_ALPHA, physicalIndex);
        let beta = readDecoyByte(&DECOY_BETA, physicalIndex);
        let gamma = readDecoyByte(&DECOY_GAMMA, physicalIndex);
        let delta = readDecoyByte(&DECOY_DELTA, physicalIndex);
        let mixer = readDecoyByte(&DECOY_MIXER, logicalIndex);
        let selector = readDecoyByte(&DECOY_SELECTOR, logicalIndex) & 3;
        let value = match selector {
            0 => alpha ^ beta ^ gamma ^ delta ^ mixer,
            1 => alpha ^ delta ^ beta ^ mixer ^ gamma,
            2 => alpha ^ mixer ^ beta ^ gamma ^ delta,
            _ => alpha ^ gamma ^ beta ^ delta ^ mixer,
        };
        output.push(std::hint::black_box(value));
    }
    output
}

/// Reads one public decoy share through a volatile access boundary.
#[cfg(not(operit_private_toolpkg_protection))]
#[inline(never)]
fn readDecoyByte(values: &[u8; DECOY_PRIVATE_SIZE], index: usize) -> u8 {
    unsafe { core::ptr::read_volatile(values.as_ptr().add(index)) }
}

#[allow(non_snake_case)]
/// Derives the ChaCha20-Poly1305 key exactly like the Operit 1 native layer.
fn deriveAeadKey() -> [u8; SHA256_SIZE] {
    deriveKey(
        b"operit-toolpkg-protection-aead-salt",
        b"operit-toolpkg-chacha20-poly1305\x01",
    )
}

#[allow(non_snake_case)]
/// Derives the HMAC key used by the signed marketplace archive envelope.
fn deriveMarketArchiveMacKey() -> [u8; SHA256_SIZE] {
    deriveKey(
        b"operit-toolpkg-market-archive-mac-salt",
        b"operit-toolpkg-market-archive-hmac-sha256-v1\x01",
    )
}

#[allow(non_snake_case)]
/// Performs the two HMAC operations used by the fixed one-block HKDF derivation contract.
fn deriveKey(salt: &[u8], info: &[u8]) -> [u8; SHA256_SIZE] {
    let mut secret = protectionKey();
    let mut prk = hmacSha256(salt, &secret);
    let derived = hmacSha256(&prk, info);
    secret.fill(0);
    clearKeyMaterial(&mut prk);
    derived
}

#[allow(non_snake_case)]
/// Creates a 12-byte nonce from a UUID v4 random source.
fn randomNonce() -> [u8; NONCE_SIZE] {
    let uuidBytes = *uuid::Uuid::new_v4().as_bytes();
    let mut nonce = [0u8; NONCE_SIZE];
    nonce.copy_from_slice(&uuidBytes[..NONCE_SIZE]);
    nonce
}

#[allow(non_snake_case)]
/// Computes HMAC-SHA256 with a compact local implementation.
fn hmacSha256(key: &[u8], msg: &[u8]) -> [u8; SHA256_SIZE] {
    let mut bk = [0u8; 64];
    if key.len() > bk.len() {
        bk[..SHA256_SIZE].copy_from_slice(&sha256(key));
    } else {
        bk[..key.len()].copy_from_slice(key);
    }
    let mut inner = Vec::with_capacity(bk.len() + msg.len());
    let mut outer = Vec::with_capacity(bk.len() + SHA256_SIZE);
    for b in bk {
        inner.push(b ^ 0x36);
        outer.push(b ^ 0x5c);
    }
    inner.extend_from_slice(msg);
    let mut innerHash = sha256(&inner);
    outer.extend_from_slice(&innerHash);
    let result = sha256(&outer);
    bk.fill(0);
    inner.fill(0);
    outer.fill(0);
    clearKeyMaterial(&mut innerHash);
    result
}

/// Computes a SHA-256 digest.
fn sha256(bytes: &[u8]) -> [u8; SHA256_SIZE] {
    let d = Sha256::digest(bytes);
    let mut o = [0u8; SHA256_SIZE];
    o.copy_from_slice(&d);
    o
}

#[allow(non_snake_case)]
/// Builds the authenticated data used by Operit 1 protected artifacts.
fn buildAssociatedData(nonce: &[u8; NONCE_SIZE]) -> Vec<u8> {
    let mut associatedData = Vec::with_capacity(MAGIC.len() + nonce.len());
    associatedData.extend_from_slice(MAGIC);
    associatedData.extend_from_slice(nonce);
    associatedData
}

#[allow(non_snake_case)]
/// Builds the policy-bound authenticated data used by marketplace-only ToolPkg entries.
fn buildMarketAssociatedData(
    policyDigest: &[u8; SHA256_SIZE],
    nonce: &[u8; NONCE_SIZE],
) -> Vec<u8> {
    let mut associatedData =
        Vec::with_capacity(MARKET_ONLY_MAGIC.len() + policyDigest.len() + nonce.len());
    associatedData.extend_from_slice(MARKET_ONLY_MAGIC);
    associatedData.extend_from_slice(policyDigest);
    associatedData.extend_from_slice(nonce);
    associatedData
}

#[allow(non_snake_case)]
/// Compares two authentication values without an early byte mismatch exit.
fn constantTimeEquals(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (leftByte, rightByte) in left.iter().zip(right) {
        difference |= leftByte ^ rightByte;
    }
    difference == 0
}

#[allow(non_snake_case)]
/// Clears a fixed-size key buffer through volatile writes before it is released.
fn clearKeyMaterial(material: &mut [u8; SHA256_SIZE]) {
    for value in material {
        unsafe {
            core::ptr::write_volatile(value, 0);
        }
    }
}

#[allow(non_snake_case)]
/// Encrypts a buffer in place and returns the detached ChaCha20-Poly1305 tag.
fn encryptDetached(
    key: &[u8; SHA256_SIZE],
    nonce: &[u8; NONCE_SIZE],
    associatedData: &[u8],
    buffer: &mut Vec<u8>,
) -> Result<Tag, String> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    cipher
        .encrypt_in_place_detached(Nonce::from_slice(nonce), associatedData, buffer)
        .map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
/// Decrypts a buffer in place after authenticating the detached tag.
fn decryptDetached(
    key: &[u8; SHA256_SIZE],
    nonce: &[u8; NONCE_SIZE],
    associatedData: &[u8],
    buffer: &mut Vec<u8>,
    tag: &[u8],
) -> Result<(), String> {
    if tag.len() != TAG_SIZE {
        return Err("Protected payload authentication failed".to_string());
    }
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    cipher
        .decrypt_in_place_detached(
            Nonce::from_slice(nonce),
            associatedData,
            buffer,
            Tag::from_slice(tag),
        )
        .map_err(|_| "Protected payload authentication failed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    /// Verifies standalone scripts remain plain while their executable code is AST-minified.
    fn standalone_script_is_plain_and_ast_minified() {
        let source = br#"
            // comment must disappear
            export function keepExternalName(value) {
                return value + 1;
            }
        "#;
        let minified = protectArtifactNamedBytes(source, "core.mjs", false)
            .expect("standalone script should be minified");
        let minified = String::from_utf8(minified).expect("minified script should be UTF-8");
        assert!(minified.contains("export function keepExternalName(value){return value+1}"));
        assert!(!minified.contains("comment must disappear"));
        assert!(!minified.contains('\n'));
    }

    #[test]
    /// Verifies standalone package metadata remains parseable after AST minification.
    fn standalone_package_metadata_survives_ast_minification() {
        let source = br#"/* METADATA
{
  name: protected_package
  displayName: Protected Package
  tools: [
    {
      name: inspect
      description: Inspect text
      parameters: [
        { name: text, description: Text, type: string, required: true }
      ]
    }
  ]
}
*/
// body comment must disappear
exports.inspect = function(params) {
    return "metadata-flow:" + params.text;
};
"#;
        let minified = protectArtifactNamedBytes(source, "protected_package.js", false)
            .expect("standalone package should be minified");
        let minified = String::from_utf8(minified).expect("minified package should be UTF-8");
        assert!(minified.starts_with("/* METADATA"));
        assert!(minified.contains("name: protected_package"));
        assert!(minified
            .contains("exports.inspect=function(params){return\"metadata-flow:\"+params.text};"));
        assert!(!minified.contains("body comment must disappear"));

        let package = crate::JsPackageLoader::JsPackageLoader::parse(&minified)
            .expect("minified standalone package should parse");
        assert_eq!(package.name, "protected_package");
        assert_eq!(package.tools.len(), 1);
        assert_eq!(package.tools[0].name, "inspect");
    }

    #[test]
    /// Verifies ToolPkg publication preserves a normal ZIP, manifest bytes, and resource bytes.
    fn toolpkg_publication_minifies_only_executable_entries() {
        let manifest = br#"{
            "toolpkg_id": "minify-test",
            "version": "1.0.0",
            "main": "main.js",
            "resources": [
                {"key": "web", "path": "assets", "mime": "vnd.android.document/directory"}
            ]
        }"#;
        let mut source_bytes = Vec::new();
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut source_bytes));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("manifest.json", options)
            .expect("manifest entry should start");
        zip.write_all(manifest).expect("manifest should be written");
        zip.start_file("main.js", options)
            .expect("main entry should start");
        zip.write_all(b"// comment\nglobalThis.main = true;")
            .expect("main should be written");
        zip.start_file("assets/app.js", options)
            .expect("resource entry should start");
        zip.write_all(b"window.asset = true;")
            .expect("resource should be written");
        zip.start_file("modules/core.wasm", options)
            .expect("wasm entry should start");
        zip.write_all(b"\0asm\x01\0\0\0")
            .expect("wasm should be written");
        zip.finish().expect("test zip should finish");

        let minified =
            protectArtifactBytes(&source_bytes, true).expect("ToolPkg should be minified");
        assert!(minified.starts_with(b"PK"));
        assert!(!isMarketArchive(&minified));

        let mut output = zip::ZipArchive::new(Cursor::new(minified))
            .expect("minified ToolPkg should be a standard ZIP");
        let mut manifest_bytes = Vec::new();
        output
            .by_name("manifest.json")
            .expect("manifest entry should exist")
            .read_to_end(&mut manifest_bytes)
            .expect("manifest entry should read");
        let mut main_bytes = Vec::new();
        output
            .by_name("main.js")
            .expect("main entry should exist")
            .read_to_end(&mut main_bytes)
            .expect("main entry should read");
        let mut resource_bytes = Vec::new();
        output
            .by_name("assets/app.js")
            .expect("resource entry should exist")
            .read_to_end(&mut resource_bytes)
            .expect("resource entry should read");
        let mut wasm_bytes = Vec::new();
        output
            .by_name("modules/core.wasm")
            .expect("wasm entry should exist")
            .read_to_end(&mut wasm_bytes)
            .expect("wasm entry should read");

        assert_eq!(manifest_bytes, manifest);
        assert_eq!(main_bytes, b"globalThis.main=true;");
        assert_eq!(resource_bytes, b"window.asset = true;");
        assert_eq!(wasm_bytes, b"\0asm\x01\0\0\0");
    }
}
