#![allow(non_snake_case)]

use crate::JsPackageLoader::JsPackageLoader;
use crate::toolpkg::ToolPkgParser::{ToolPkgArchiveParser, ToolPkgMarketOrigin};
use chacha20poly1305::{
    aead::{AeadInPlace, KeyInit},
    ChaCha20Poly1305, Key, Nonce, Tag,
};
#[cfg(not(target_arch = "wasm32"))]
use rquickjs::{CatchResultExt, Context as QuickJsContext, Runtime as QuickJsRuntime};
use sha2::{Digest, Sha256};
use regex::Regex;
use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;

pub const PROTECTION_ID: &str = "operit-protected";
pub const MARKET_ONLY_PROTECTION_ID: &str = "operit-market-only";
pub const MARKET_INSTALL_SEAL_ENTRY_NAME: &str = ".operit/market-install.seal";
pub const MARKET_ORIGIN_CAPTURE_METHOD: &str = "_m";
pub const SCRIPT_MARKET_ORIGIN_METADATA_KEY: &str = "__operit_market_origin";

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
const MARKET_ORIGIN_XOR_KEY: u8 = 0x5a;
const MARKET_ORIGIN_METADATA_PREFIX: &str = "xor-v1:";
const TOOLPKG_SIMILARITY_FRAGMENT_SIZE: usize = 24;
const MINIFIER_MAX_STACK_SIZE: usize = 8 * 1024 * 1024;
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
            compress: {
                defaults: true,
                passes: 3,
                toplevel: true
            },
            mangle: {
                toplevel: true,
                keep_classnames: false,
                keep_fnames: false
            },
            format: {
                comments: false,
                ascii_only: true
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
    protectArtifactNamedBytesWithMarketOrigin(sourceBytes, sourceEntryName, isToolPkg, None)
}

#[allow(non_snake_case)]
/// Protects one named artifact and embeds its verified marketplace origin when provided.
pub fn protectArtifactNamedBytesWithMarketOrigin(
    sourceBytes: &[u8],
    sourceEntryName: &str,
    isToolPkg: bool,
    scriptMarketOrigin: Option<&ToolPkgMarketOrigin>,
) -> Result<Vec<u8>, String> {
    if isToolPkg {
        let mut minifier = ToolPkgJsAstMinifier::new()?;
        minifyToolPkgArchive(sourceBytes, &mut minifier)
    } else {
        let source = String::from_utf8(sourceBytes.to_vec()).map_err(|error| error.to_string())?;
        let source = match scriptMarketOrigin {
            Some(origin) => injectScriptMarketOriginIntoMetadata(&source, origin)?,
            None => source,
        };
        let mut minifier = ToolPkgJsAstMinifier::new()?;
        astMinifyBytes(source.as_bytes(), sourceEntryName, &mut minifier, None)
    }
}

/// Processes one publish artifact with mandatory marketplace provenance and an explicit minification mode.
pub fn processArtifactNamedBytesWithMarketOrigin(
    sourceBytes: &[u8],
    sourceEntryName: &str,
    isToolPkg: bool,
    marketOrigin: &ToolPkgMarketOrigin,
    minify: bool,
) -> Result<Vec<u8>, String> {
    if isToolPkg {
        if minify {
            let mut minifier = ToolPkgJsAstMinifier::new()?;
            minifyToolPkgArchiveWithMarketOrigin(sourceBytes, &mut minifier, marketOrigin)
        } else {
            injectToolPkgMarketOrigin(sourceBytes, marketOrigin)
        }
    } else {
        let source = String::from_utf8(sourceBytes.to_vec()).map_err(|error| error.to_string())?;
        let source = injectScriptMarketOriginIntoMetadata(&source, marketOrigin)?;
        if !minify {
            return Ok(source.into_bytes());
        }
        let mut minifier = ToolPkgJsAstMinifier::new()?;
        astMinifyBytes(source.as_bytes(), sourceEntryName, &mut minifier, None)
    }
}

/// Contains transparent code-overlap evidence between two ToolPkg archives.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolPkgJavaScriptSimilarity {
    /// Counts unique canonical-code fingerprints in the reference archive.
    pub referenceFingerprintCount: usize,
    /// Counts unique canonical-code fingerprints in the candidate archive.
    pub candidateFingerprintCount: usize,
    /// Counts fingerprints present in both archives.
    pub sharedFingerprintCount: usize,
    /// Reports the fraction of reference fingerprints found in the candidate archive.
    pub referenceCoverage: f64,
    /// Reports the fraction of candidate fingerprints found in the reference archive.
    pub candidateCoverage: f64,
    /// Reports the harmonic mean of both directional coverage values.
    pub score: f64,
}

/// Compares executable ToolPkg code after deterministic AST canonicalization.
///
/// The result is reproducible evidence for complaint review: it reports copied
/// canonical code fragments but does not establish authorship or make a removal decision.
#[allow(non_snake_case)]
pub fn compareToolPkgJavaScriptSimilarity(
    referenceArchiveBytes: &[u8],
    candidateArchiveBytes: &[u8],
) -> Result<ToolPkgJavaScriptSimilarity, String> {
    let mut minifier = ToolPkgJsAstMinifier::new()?;
    let referenceFingerprints = canonicalToolPkgFingerprints(referenceArchiveBytes, &mut minifier)?;
    let candidateFingerprints = canonicalToolPkgFingerprints(candidateArchiveBytes, &mut minifier)?;
    let sharedFingerprintCount = referenceFingerprints
        .intersection(&candidateFingerprints)
        .count();
    let referenceCoverage = sharedFingerprintCount as f64 / referenceFingerprints.len() as f64;
    let candidateCoverage = sharedFingerprintCount as f64 / candidateFingerprints.len() as f64;
    let score = if sharedFingerprintCount == 0 {
        0.0
    } else {
        2.0 * referenceCoverage * candidateCoverage / (referenceCoverage + candidateCoverage)
    };

    Ok(ToolPkgJavaScriptSimilarity {
        referenceFingerprintCount: referenceFingerprints.len(),
        candidateFingerprintCount: candidateFingerprints.len(),
        sharedFingerprintCount,
        referenceCoverage,
        candidateCoverage,
        score,
    })
}

/// Produces canonical fragment fingerprints for every declared executable ToolPkg entry.
fn canonicalToolPkgFingerprints(
    archiveBytes: &[u8],
    minifier: &mut ToolPkgJsAstMinifier,
) -> Result<BTreeSet<[u8; SHA256_SIZE]>, String> {
    let executableEntries = readToolPkgExecutableEntries(archiveBytes)?;
    let mut fingerprints = BTreeSet::new();
    for (entryName, source) in executableEntries {
        let canonicalSource = astMinifySourcePreservingMetadata(&source, &entryName, minifier)?;
        let canonicalSource = stripCanonicalMarketOriginInvocations(&canonicalSource)?;
        fingerprints.extend(buildJavaScriptFragmentFingerprints(&canonicalSource));
    }
    if fingerprints.is_empty() {
        return Err("ToolPkg executable entries contain no comparable JavaScript code".to_string());
    }
    Ok(fingerprints)
}

/// Reads the main and declared subpackage scripts from one ToolPkg archive.
fn readToolPkgExecutableEntries(archiveBytes: &[u8]) -> Result<Vec<(String, String)>, String> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(archiveBytes)).map_err(|error| error.to_string())?;
    let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
    let manifestPreview =
        ToolPkgArchiveParser::readToolPkgManifestPreview(&mut archive, &entryIndex)
            .ok_or_else(|| "manifest.hjson or manifest.json not found".to_string())?;
    let manifestBasePath = manifestPreview
        .entryName
        .rsplit_once('/')
        .map(|(basePath, _)| basePath)
        .unwrap_or("");
    let mainEntry = ToolPkgArchiveParser::resolveManifestRelativeZipEntryPath(
        manifestBasePath,
        &manifestPreview.manifest.main,
    )
    .ok_or_else(|| "ToolPkg manifest.main is required for similarity comparison".to_string())?;
    let mut entryNames = BTreeSet::from([mainEntry]);
    for subpackage in &manifestPreview.manifest.subpackages {
        let entryName = ToolPkgArchiveParser::resolveManifestRelativeZipEntryPath(
            manifestBasePath,
            &subpackage.entry,
        )
        .ok_or_else(|| {
            format!(
                "ToolPkg subpackage '{}' has an invalid entry path",
                subpackage.id
            )
        })?;
        entryNames.insert(entryName);
    }
    entryNames
        .into_iter()
        .map(|entryName| {
            let source =
                ToolPkgArchiveParser::readZipEntryText(&mut archive, &entryIndex, &entryName)
                    .ok_or_else(|| {
                        format!("Unable to read ToolPkg executable entry '{entryName}'")
                    })?;
            Ok((entryName, source))
        })
        .collect()
}

/// Builds order-independent hashes for fixed-size canonical JavaScript fragments.
fn buildJavaScriptFragmentFingerprints(source: &str) -> BTreeSet<[u8; SHA256_SIZE]> {
    let sourceBytes = source.as_bytes();
    if sourceBytes.is_empty() {
        return BTreeSet::new();
    }
    let fragmentSize = sourceBytes.len().min(TOOLPKG_SIMILARITY_FRAGMENT_SIZE);
    let mut fingerprints = BTreeSet::new();
    for fragment in sourceBytes.windows(fragmentSize) {
        fingerprints.insert(sha256(fragment));
    }
    fingerprints
}

/// Removes the structured market-origin calls added to published ToolPkg main entries.
fn stripCanonicalMarketOriginInvocations(source: &str) -> Result<String, String> {
    let bytes = source.as_bytes();
    let prefix = format!("ToolPkg.{MARKET_ORIGIN_CAPTURE_METHOD}(");
    let prefixBytes = prefix.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut offset = 0usize;
    while offset < bytes.len() {
        if matches!(bytes[offset], b'\'' | b'\"' | b'`') {
            let end = findJavaScriptQuotedEnd(bytes, offset, bytes[offset])?;
            output.extend_from_slice(&bytes[offset..end]);
            offset = end;
            continue;
        }
        let hasBoundary = offset == 0 || !isJavaScriptNameByte(bytes[offset - 1]);
        if hasBoundary && bytes[offset..].starts_with(prefixBytes) {
            let openParen = offset + prefixBytes.len() - 1;
            offset = findJavaScriptCallEnd(bytes, openParen)?;
            if bytes.get(offset) == Some(&b';') {
                offset += 1;
            }
            continue;
        }
        output.push(bytes[offset]);
        offset += 1;
    }
    String::from_utf8(output).map_err(|error| error.to_string())
}

/// Returns whether one byte can continue a JavaScript identifier or property name.
fn isJavaScriptNameByte(value: u8) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, b'_' | b'$')
}

/// Finds the exclusive end of one quoted JavaScript token.
fn findJavaScriptQuotedEnd(bytes: &[u8], quoteStart: usize, quote: u8) -> Result<usize, String> {
    let mut offset = quoteStart + 1;
    while offset < bytes.len() {
        match bytes[offset] {
            b'\\' => offset += 2,
            value if value == quote => return Ok(offset + 1),
            _ => offset += 1,
        }
    }
    Err("Canonical JavaScript contains an unterminated quoted token".to_string())
}

/// Finds the exclusive end of one balanced JavaScript call expression.
fn findJavaScriptCallEnd(bytes: &[u8], openParen: usize) -> Result<usize, String> {
    let mut depth = 0usize;
    let mut offset = openParen;
    while offset < bytes.len() {
        match bytes[offset] {
            b'\'' | b'\"' | b'`' => {
                offset = findJavaScriptQuotedEnd(bytes, offset, bytes[offset])?;
                continue;
            }
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(offset + 1);
                }
            }
            _ => {}
        }
        offset += 1;
    }
    Err("Canonical JavaScript contains an unterminated market-origin call".to_string())
}

/// Builds the encoded marketplace origin call injected into the ToolPkg main entry.
fn buildMarketOriginInvocation(
    toolpkgId: &str,
    version: &str,
    author: &[String],
) -> Result<String, String> {
    let origin = ToolPkgMarketOrigin {
        market: "Operit".to_string(),
        toolpkgId: toolpkgId.to_string(),
        version: version.to_string(),
        author: author.to_vec(),
    };
    let encoded = encodeMarketOriginBytes(&origin)?;
    let encodedJson = serde_json::to_string(&encoded).map_err(|error| error.to_string())?;
    Ok(format!(
        "ToolPkg.{MARKET_ORIGIN_CAPTURE_METHOD}({encodedJson},{MARKET_ORIGIN_XOR_KEY});"
    ))
}

/// Encodes marketplace provenance for a standalone script metadata field.
pub fn encodeMarketOriginForMetadata(origin: &ToolPkgMarketOrigin) -> Result<String, String> {
    let encoded = encodeMarketOriginBytes(origin)?;
    Ok(format!(
        "{MARKET_ORIGIN_METADATA_PREFIX}{}",
        encoded
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    ))
}

/// Decodes and validates marketplace provenance stored in standalone script metadata.
pub fn decodeMarketOriginFromMetadata(
    value: &str,
    packageId: &str,
) -> Option<ToolPkgMarketOrigin> {
    let encoded = value.trim().strip_prefix(MARKET_ORIGIN_METADATA_PREFIX)?;
    if encoded.trim().is_empty() {
        return None;
    }
    let mut bytes = Vec::new();
    for item in encoded.split(',') {
        bytes.push(item.trim().parse::<u8>().ok()? ^ MARKET_ORIGIN_XOR_KEY);
    }
    let payload = String::from_utf8(bytes).ok()?;
    let origin = serde_json::from_str::<ToolPkgMarketOrigin>(&payload).ok()?;
    validateMarketOrigin(origin, packageId)
}

/// Reads a validated marketplace provenance record from standalone script metadata.
pub fn readScriptMarketOrigin(script: &str, packageId: &str) -> Option<ToolPkgMarketOrigin> {
    let metadata = JsPackageLoader::extract_metadata(script);
    let metadata = JsPackageLoader::parse_metadata_object(&metadata).ok()?;
    let encoded = metadata
        .get(SCRIPT_MARKET_ORIGIN_METADATA_KEY)
        .and_then(serde_json::Value::as_str)?;
    decodeMarketOriginFromMetadata(encoded, packageId)
}

/// Adds encoded marketplace provenance to a script's required leading metadata block.
fn injectScriptMarketOriginIntoMetadata(
    source: &str,
    marketOrigin: &ToolPkgMarketOrigin,
) -> Result<String, String> {
    let Some((_, body)) = splitLeadingMetadataBlock(source) else {
        return Err("JavaScript package METADATA block is required for marketplace origin".to_string());
    };
    let metadata = JsPackageLoader::extract_metadata(source);
    let mut metadata = JsPackageLoader::parse_metadata_object(&metadata)?;
    metadata.insert(
        SCRIPT_MARKET_ORIGIN_METADATA_KEY.to_string(),
        serde_json::Value::String(encodeMarketOriginForMetadata(marketOrigin)?),
    );
    let serialized = serde_json::to_string(&metadata).map_err(|error| error.to_string())?;
    Ok(format!("/* METADATA\n{serialized}\n*/{body}"))
}

/// Serializes a market-origin record as the shared ASCII XOR byte payload.
fn encodeMarketOriginBytes(origin: &ToolPkgMarketOrigin) -> Result<Vec<u8>, String> {
    let payloadJson = serde_json::to_string(origin).map_err(|error| error.to_string())?;
    let asciiPayload = payloadJson.chars().fold(String::new(), |mut output, character| {
        if character.is_ascii() {
            output.push(character);
        } else {
            let mut utf16Units = [0u16; 2];
            for unit in character.encode_utf16(&mut utf16Units) {
                output.push_str("\\u");
                output.push_str(&format!("{:04x}", *unit));
            }
        }
        output
    });
    Ok(asciiPayload
        .into_bytes()
        .into_iter()
        .map(|value| value ^ MARKET_ORIGIN_XOR_KEY)
        .collect())
}

/// Accepts only complete Operit provenance records that match the installed package ID.
fn validateMarketOrigin(
    origin: ToolPkgMarketOrigin,
    packageId: &str,
) -> Option<ToolPkgMarketOrigin> {
    let toolpkgId = origin.toolpkgId.trim();
    let version = origin.version.trim();
    if origin.market != "Operit" || toolpkgId != packageId.trim() || version.is_empty() {
        return None;
    }
    Some(ToolPkgMarketOrigin {
        market: origin.market,
        toolpkgId: toolpkgId.to_string(),
        version: version.to_string(),
        author: origin
            .author
            .into_iter()
            .map(|author| author.trim().to_string())
            .filter(|author| !author.is_empty())
            .collect(),
    })
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
/// Preserves the legacy ToolPkg minifier entry point using manifest-declared author metadata.
fn minifyToolPkgArchive(
    sourceBytes: &[u8],
    minifier: &mut ToolPkgJsAstMinifier,
) -> Result<Vec<u8>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(sourceBytes)).map_err(|e| e.to_string())?;
    let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
    let manifestPreview = ToolPkgArchiveParser::readToolPkgManifestPreview(&mut archive, &entryIndex)
        .ok_or_else(|| "manifest.hjson or manifest.json not found".to_string())?;
    let marketOrigin = ToolPkgMarketOrigin {
        market: "Operit".to_string(),
        toolpkgId: manifestPreview.manifest.toolpkgId.clone(),
        version: manifestPreview.manifest.version.clone(),
        author: manifestPreview.manifest.author.clone(),
    };
    minifyToolPkgArchiveWithMarketOrigin(sourceBytes, minifier, &marketOrigin)
}

/// AST-minifies executable ToolPkg entries while preserving the standard ZIP structure.
fn minifyToolPkgArchiveWithMarketOrigin(
    sourceBytes: &[u8],
    minifier: &mut ToolPkgJsAstMinifier,
    marketOrigin: &ToolPkgMarketOrigin,
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
    let mainEntry = ToolPkgArchiveParser::resolveManifestRelativeZipEntryPath(
        &manifestBasePath,
        &mPreview.manifest.main,
    );
    let marketOriginInvocation = buildMarketOriginInvocation(
        &marketOrigin.toolpkgId,
        &marketOrigin.version,
        &marketOrigin.author,
    )?;
    if let Some(mainEntryPath) = &mainEntry {
        astMinifiedEntryNames.insert(mainEntryPath.clone());
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
    for module in &mPreview.manifest.wasmModules {
        if let Some(path) = ToolPkgArchiveParser::resolveManifestRelativeResourcePath(
            &manifestBasePath,
            &module.path,
        ) {
            resourceEntryRoots.insert(path);
        }
    }
    let reachableEntryNames = collectReachableToolPkgEntries(
        sourceBytes,
        &manifestEntryName,
        &astMinifiedEntryNames,
        &resourceEntryRoots,
    )?;
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
            let normalizedName = ToolPkgArchiveParser::normalizeZipEntryPath(&name);
            if normalizedName
                .as_ref()
                .map(|value| !reachableEntryNames.contains(value))
                .unwrap_or(true)
            {
                continue;
            }
            if entry.is_dir() {
                w.add_directory(name, options).map_err(|e| e.to_string())?;
                continue;
            }
            let mut orig = Vec::new();
            entry.read_to_end(&mut orig).map_err(|e| e.to_string())?;
            let norm = normalizedName;
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
                    astMinifyBytes(
                        &orig,
                        norm,
                        minifier,
                        (mainEntry.as_deref() == Some(norm))
                            .then_some(marketOriginInvocation.as_str()),
                    )?
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

/// Injects marketplace provenance into the ToolPkg main entry without changing executable source formatting.
fn injectToolPkgMarketOrigin(
    sourceBytes: &[u8],
    marketOrigin: &ToolPkgMarketOrigin,
) -> Result<Vec<u8>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(sourceBytes)).map_err(|e| e.to_string())?;
    let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
    let manifestPreview = ToolPkgArchiveParser::readToolPkgManifestPreview(&mut archive, &entryIndex)
        .ok_or_else(|| "manifest.hjson or manifest.json not found".to_string())?;
    let manifestBasePath = manifestPreview
        .entryName
        .rsplit_once('/')
        .map(|(basePath, _)| basePath)
        .unwrap_or("");
    let mainEntry = ToolPkgArchiveParser::resolveManifestRelativeZipEntryPath(
        manifestBasePath,
        &manifestPreview.manifest.main,
    )
    .ok_or_else(|| "ToolPkg manifest.main is required".to_string())?;
    let marketOriginInvocation = buildMarketOriginInvocation(
        &marketOrigin.toolpkgId,
        &marketOrigin.version,
        &marketOrigin.author,
    )?;
    let mut out = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(Cursor::new(&mut out));
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
            let name = entry.name().to_string();
            let mut options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            if let Some(lastModified) = entry.last_modified() {
                options = options.last_modified_time(lastModified);
            }
            if entry.is_dir() {
                writer
                    .add_directory(name, options)
                    .map_err(|error| error.to_string())?;
                continue;
            }
            let mut original = Vec::new();
            entry
                .read_to_end(&mut original)
                .map_err(|error| error.to_string())?;
            let data = match ToolPkgArchiveParser::normalizeZipEntryPath(&name).as_deref() {
                Some(normalizedName) if normalizedName == mainEntry => {
                    format!(
                        "{}\n{}\n",
                        String::from_utf8(original).map_err(|error| error.to_string())?,
                        marketOriginInvocation
                    )
                    .into_bytes()
                }
                _ => original,
            };
            writer
                .start_file(name, options)
                .map_err(|error| error.to_string())?;
            writer.write_all(&data).map_err(|error| error.to_string())?;
        }
        writer.finish().map_err(|error| error.to_string())?;
    }
    Ok(out)
}

/// Collects manifest, entry, resource, and static relative-module dependencies for one ToolPkg archive.
fn collectReachableToolPkgEntries(
    sourceBytes: &[u8],
    manifestEntryName: &str,
    executableEntryNames: &BTreeSet<String>,
    resourceEntryRoots: &BTreeSet<String>,
) -> Result<BTreeSet<String>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(sourceBytes)).map_err(|e| e.to_string())?;
    let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
    let entryNames = entryIndex.entryNames.clone();
    let mut reachable = BTreeSet::from([manifestEntryName.to_string()]);
    reachable.extend(executableEntryNames.iter().cloned());
    for root in resourceEntryRoots {
        reachable.extend(
            entryNames
                .iter()
                .filter(|name| *name == root || name.starts_with(&format!("{}/", root)))
                .cloned(),
        );
    }

    let modulePattern = Regex::new(
        r#"(?:require\s*\(\s*["']([^"']+)["']\s*\)|(?:from|import)\s*["']([^"']+)["'])"#,
    )
    .map_err(|error| error.to_string())?;
    let mut pending = executableEntryNames.iter().cloned().collect::<Vec<_>>();
    while let Some(currentName) = pending.pop() {
        if !isJavaScriptEntry(&currentName) {
            continue;
        }
        let Some(source) = ToolPkgArchiveParser::readZipEntryText(
            &mut archive,
            &entryIndex,
            &currentName,
        ) else {
            continue;
        };
        for captures in modulePattern.captures_iter(&source) {
            let specifier = captures
                .get(1)
                .or_else(|| captures.get(2))
                .map(|value| value.as_str())
                .unwrap_or("");
            let Some(resolved) = resolveToolPkgModuleEntry(&currentName, specifier, &entryNames)
            else {
                continue;
            };
            if reachable.insert(resolved.clone()) {
                pending.push(resolved);
            }
        }
    }
    Ok(reachable)
}

/// Resolves a static relative JavaScript module reference against archive entries.
fn resolveToolPkgModuleEntry(
    currentName: &str,
    specifier: &str,
    entryNames: &BTreeSet<String>,
) -> Option<String> {
    if !specifier.starts_with('.') {
        return None;
    }
    let mut segments = currentName
        .rsplit_once('/')
        .map(|(base, _)| base)
        .unwrap_or("")
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    for segment in specifier.replace('\\', "/").split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            value => segments.push(value.to_string()),
        }
    }
    let modulePath = segments.join("/");
    [
        modulePath.clone(),
        format!("{}.js", modulePath),
        format!("{}.mjs", modulePath),
        format!("{}.cjs", modulePath),
        format!("{}.json", modulePath),
        format!("{}/index.js", modulePath),
    ]
    .into_iter()
    .find(|candidate| entryNames.contains(candidate))
}

/// Returns whether an archive entry contains JavaScript executable source.
fn isJavaScriptEntry(name: &str) -> bool {
    matches!(
        name.rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())
            .as_deref(),
        Some("js" | "mjs" | "cjs" | "ts" | "jsx" | "tsx")
    )
}

/// Returns whether a normalized ToolPkg archive entry should be AST-minified.
fn shouldAstMinifyToolPkgEntry(
    norm: &str,
    astMinifiedEntryNames: &BTreeSet<String>,
    resourceEntryRoots: &BTreeSet<String>,
) -> bool {
    (astMinifiedEntryNames.contains(norm) || isJavaScriptEntry(norm))
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
    marketOriginInvocation: Option<&str>,
) -> Result<Vec<u8>, String> {
    let source = String::from_utf8(bytes.to_vec()).map_err(|error| error.to_string())?;
    let source = marketOriginInvocation
        .map(|invocation| format!("{source}\n{invocation}\n"))
        .unwrap_or(source);
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
        // Terser's scope analysis needs more stack than QuickJS's default for published packages.
        runtime.set_max_stack_size(MINIFIER_MAX_STACK_SIZE);
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

    /// Builds a minimal ToolPkg archive with one main JavaScript entry.
    fn buildSimilarityTestToolPkg(mainScript: &str) -> Vec<u8> {
        let manifest = br#"{
            "toolpkg_id": "similarity-test",
            "version": "1.0.0",
            "main": "main.js"
        }"#;
        let mut archiveBytes = Vec::new();
        let mut archive = zip::ZipWriter::new(Cursor::new(&mut archiveBytes));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        archive
            .start_file("manifest.json", options)
            .expect("manifest entry should start");
        archive
            .write_all(manifest)
            .expect("manifest should be written");
        archive
            .start_file("main.js", options)
            .expect("main entry should start");
        archive
            .write_all(mainScript.as_bytes())
            .expect("main script should be written");
        archive.finish().expect("test archive should finish");
        archiveBytes
    }

    #[test]
    /// Verifies standalone scripts retain public exports while their executable code is optimized.
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
        assert!(minified.contains("export function keepExternalName"));
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
        assert!(minified.contains("exports.inspect="));
        assert!(!minified.contains("body comment must disappear"));

        let package = crate::JsPackageLoader::JsPackageLoader::parse(&minified)
            .expect("minified standalone package should parse");
        assert_eq!(package.name, "protected_package");
        assert_eq!(package.tools.len(), 1);
        assert_eq!(package.tools[0].name, "inspect");
    }

    #[test]
    /// Verifies direct marketplace uploads always carry the authenticated author provenance.
    fn publish_processing_injects_script_provenance_with_or_without_minification() {
        let source = r#"/* METADATA
{
  "name": "uuid_generator"
}
*/
const internalUuidPrefix = "uuid:";
exports.generate = function generateUuid(value) {
    return internalUuidPrefix + value.trim();
};
"#;
        let origin = ToolPkgMarketOrigin {
            market: "Operit".to_string(),
            toolpkgId: "uuid_generator".to_string(),
            version: "1.0.2".to_string(),
            author: vec!["authenticated-publisher".to_string()],
        };

        let unminified = processArtifactNamedBytesWithMarketOrigin(
            source.as_bytes(),
            "uuid_generator.js",
            false,
            &origin,
            false,
        )
        .expect("unminified script processing should succeed");
        let unminified = String::from_utf8(unminified).expect("processed script should be UTF-8");
        assert!(unminified.contains("internalUuidPrefix"));
        assert_eq!(
            readScriptMarketOrigin(&unminified, "uuid_generator"),
            Some(origin.clone())
        );

        let minified = processArtifactNamedBytesWithMarketOrigin(
            source.as_bytes(),
            "uuid_generator.js",
            false,
            &origin,
            true,
        )
        .expect("minified script processing should succeed");
        let minified = String::from_utf8(minified).expect("processed script should be UTF-8");
        assert!(!minified.contains("internalUuidPrefix"));
        assert_eq!(
            readScriptMarketOrigin(&minified, "uuid_generator"),
            Some(origin)
        );
    }

    #[test]
    /// Verifies ToolPkg publication preserves all files normally and keeps only reachable files when minified.
    fn publish_processing_marks_toolpkg_and_trims_to_reachable_entries() {
        let manifest = br#"{
            "toolpkg_id": "tree-publish-test",
            "version": "1.0.2",
            "author": ["manifest-author"],
            "main": "main.js",
            "subpackages": [{"id": "child", "entry": "modules/child.js"}],
            "resources": [{"key": "web", "path": "assets", "mime": "vnd.android.document/directory"}],
            "wasm_modules": [{"id": "core", "path": "native/core.wasm", "exports": ["run"]}]
        }"#;
        let mut source = Vec::new();
        let mut archive = zip::ZipWriter::new(Cursor::new(&mut source));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, bytes) in [
            ("manifest.json", manifest.as_slice()),
            (
                "main.js",
                b"const helper = require('./lib/used'); globalThis.main = helper.value;".as_slice(),
            ),
            (
                "modules/child.js",
                b"const helper = require('../lib/child-used'); globalThis.child = helper.value;"
                    .as_slice(),
            ),
            ("lib/used.js", b"exports.value = 'main';".as_slice()),
            ("lib/child-used.js", b"exports.value = 'child';".as_slice()),
            ("assets/site.js", b"window.asset = true;".as_slice()),
            ("native/core.wasm", b"\0asm\x01\0\0\0".as_slice()),
            ("src/private.js", b"const privateSource = true;".as_slice()),
            ("unused.js", b"const unusedEntry = true;".as_slice()),
            ("main.js.map", b"{}".as_slice()),
        ] {
            archive
                .start_file(name, options)
                .expect("test archive entry should start");
            archive
                .write_all(bytes)
                .expect("test archive entry should be written");
        }
        archive.finish().expect("test archive should finish");

        let origin = ToolPkgMarketOrigin {
            market: "Operit".to_string(),
            toolpkgId: "tree-publish-test".to_string(),
            version: "1.0.2".to_string(),
            author: vec!["authenticated-publisher".to_string()],
        };
        let expectedInvocation = buildMarketOriginInvocation(
            &origin.toolpkgId,
            &origin.version,
            &origin.author,
        )
        .expect("market origin invocation should be built");

        let unminified = processArtifactNamedBytesWithMarketOrigin(
            &source,
            "tree-publish-test.toolpkg",
            true,
            &origin,
            false,
        )
        .expect("unminified ToolPkg processing should succeed");
        let mut unminified = zip::ZipArchive::new(Cursor::new(unminified))
            .expect("unminified result should remain a ZIP");
        assert!(unminified.by_name("src/private.js").is_ok());
        let mut unminifiedMain = String::new();
        unminified
            .by_name("main.js")
            .expect("main entry should remain")
            .read_to_string(&mut unminifiedMain)
            .expect("main entry should be text");
        assert!(unminifiedMain.contains(&expectedInvocation));

        let minified = processArtifactNamedBytesWithMarketOrigin(
            &source,
            "tree-publish-test.toolpkg",
            true,
            &origin,
            true,
        )
        .expect("minified ToolPkg processing should succeed");
        let mut minified =
            zip::ZipArchive::new(Cursor::new(minified)).expect("minified result should remain a ZIP");
        for retained in [
            "manifest.json",
            "main.js",
            "modules/child.js",
            "lib/used.js",
            "lib/child-used.js",
            "assets/site.js",
            "native/core.wasm",
        ] {
            assert!(minified.by_name(retained).is_ok(), "{retained} should remain");
        }
        for removed in ["src/private.js", "unused.js", "main.js.map"] {
            assert!(minified.by_name(removed).is_err(), "{removed} should be removed");
        }
        let mut minifiedMain = String::new();
        minified
            .by_name("main.js")
            .expect("main entry should remain")
            .read_to_string(&mut minifiedMain)
            .expect("main entry should be text");
        assert!(minifiedMain.contains(&expectedInvocation));
    }

    #[test]
    /// Verifies release optimization removes private names without changing a CommonJS export.
    fn release_optimization_preserves_commonjs_export_and_removes_private_names() {
        let source = r#"
            // This implementation detail must not be retained in a published artifact.
            const internalPrefixForUuidGenerator = "normalized:";
            function normalizeUserSuppliedUuid(sourceText) {
                return sourceText.trim().toLowerCase();
            }
            exports.generate_uuid = function generateUuidImplementation(params) {
                const suppliedText = params.text;
                return internalPrefixForUuidGenerator + normalizeUserSuppliedUuid(suppliedText);
            };
        "#;
        let protected = protectArtifactNamedBytes(source.as_bytes(), "uuid_generator.js", false)
            .expect("standalone script should be optimized");
        let protected = String::from_utf8(protected).expect("optimized script should be UTF-8");

        assert!(protected.len() < source.len());
        assert!(protected.contains("exports.generate_uuid="));
        assert!(!protected.contains("internalPrefixForUuidGenerator"));
        assert!(!protected.contains("normalizeUserSuppliedUuid"));
        assert!(!protected.contains("generateUuidImplementation"));

        let mut minifier = ToolPkgJsAstMinifier::new().expect("QuickJS minifier should initialize");
        let result = minifier
            .evalString(&format!(
                "globalThis.exports={{}};{protected}JSON.stringify(exports.generate_uuid({{text:\"  A1B2-C3D4  \"}}));"
            ))
            .expect("optimized CommonJS export should execute");
        assert_eq!(result, "\"normalized:a1b2-c3d4\"");
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
            ],
            "wasm_modules": [
                {"id": "core", "path": "modules/core.wasm", "exports": ["run"]}
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

        let mut output = zip::ZipArchive::new(Cursor::new(&minified))
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
        let main_text = String::from_utf8_lossy(&main_bytes);
        assert!(main_text.contains("ToolPkg._m("));
        assert!(!main_text.contains("\"market\":\"Operit\""));
        assert_eq!(resource_bytes, b"window.asset = true;");
        assert_eq!(wasm_bytes, b"\0asm\x01\0\0\0");
    }

    #[test]
    /// Verifies comments and formatting do not change ToolPkg code similarity.
    fn toolpkg_similarity_ignores_comments_and_formatting() {
        let reference = buildSimilarityTestToolPkg(
            r#"
                // This comment is not code.
                function formatLabel(value) {
                    return "label:" + value;
                }
                globalThis.render = formatLabel;
            "#,
        );
        let candidate = buildSimilarityTestToolPkg(
            r#"function formatLabel(value){return"label:"+value}globalThis.render=formatLabel;"#,
        );

        let similarity = compareToolPkgJavaScriptSimilarity(&reference, &candidate)
            .expect("formatted copies should be comparable");

        assert_eq!(similarity.referenceCoverage, 1.0);
        assert_eq!(similarity.candidateCoverage, 1.0);
        assert_eq!(similarity.score, 1.0);
    }

    #[test]
    /// Verifies publication-minified ToolPkg code remains strongly comparable to its source archive.
    fn toolpkg_similarity_matches_published_minified_source() {
        let source = buildSimilarityTestToolPkg(
            r#"
                function normalizeName(value) { return value.trim().toLowerCase(); }
                function createGreeting(value) { return "Hello, " + normalizeName(value); }
                function createSummary(value) { return { greeting: createGreeting(value), size: value.length }; }
                globalThis.createSummary = createSummary;
            "#,
        );
        let published = protectArtifactBytes(&source, true).expect("ToolPkg should be minified");

        let similarity = compareToolPkgJavaScriptSimilarity(&source, &published)
            .expect("source and published package should be comparable");

        assert!(similarity.referenceCoverage > 0.98, "{similarity:?}");
        assert!(similarity.candidateCoverage > 0.98, "{similarity:?}");
        assert!(similarity.score > 0.98, "{similarity:?}");
    }

    #[test]
    /// Verifies a local function change preserves evidence from the unchanged code.
    fn toolpkg_similarity_survives_a_local_code_change() {
        let reference = buildSimilarityTestToolPkg(
            r#"
                function normalizeName(value) { return value.trim().toLowerCase(); }
                function createGreeting(value) { return "Hello, " + normalizeName(value); }
                function createSummary(value) { return { greeting: createGreeting(value), size: value.length }; }
                function createUrl(value) { return "https://example.test/" + normalizeName(value); }
                globalThis.createSummary = createSummary;
            "#,
        );
        let modified = buildSimilarityTestToolPkg(
            r#"
                function normalizeName(value) { return value.trim().toLowerCase(); }
                function createGreeting(value) { return "Welcome, " + normalizeName(value).toUpperCase(); }
                function createSummary(value) { return { greeting: createGreeting(value), size: value.length }; }
                function createUrl(value) { return "https://example.test/" + normalizeName(value); }
                globalThis.createSummary = createSummary;
            "#,
        );
        let published = protectArtifactBytes(&modified, true).expect("ToolPkg should be minified");

        let similarity = compareToolPkgJavaScriptSimilarity(&reference, &published)
            .expect("modified package should be comparable");

        assert!(similarity.referenceCoverage > 0.3, "{similarity:?}");
        assert!(similarity.candidateCoverage > 0.25, "{similarity:?}");
        assert!(similarity.score > 0.25, "{similarity:?}");
    }

    #[test]
    /// Verifies broader edits lower the score while preserving overlap from unchanged structure.
    fn toolpkg_similarity_reports_broader_code_changes() {
        let reference = buildSimilarityTestToolPkg(
            r#"
                function normalizeName(value) { return value.trim().toLowerCase(); }
                function createGreeting(value) { return "Hello, " + normalizeName(value); }
                function createSummary(value) { return { greeting: createGreeting(value), size: value.length }; }
                function createUrl(value) { return "https://example.test/" + normalizeName(value); }
                globalThis.createSummary = createSummary;
            "#,
        );
        let modified = buildSimilarityTestToolPkg(
            r#"
                function normalizeName(value) { return String(value).trim().replace(/\s+/g, "-"); }
                function createGreeting(value) { return "Welcome back, " + normalizeName(value); }
                function createSummary(value) { return { message: createGreeting(value), length: value.length, url: createUrl(value) }; }
                function createUrl(value) { return "https://operit.test/items/" + normalizeName(value); }
                globalThis.createSummary = createSummary;
            "#,
        );
        let published = protectArtifactBytes(&modified, true).expect("ToolPkg should be minified");

        let similarity = compareToolPkgJavaScriptSimilarity(&reference, &published)
            .expect("broadly modified package should be comparable");

        assert!(similarity.referenceCoverage > 0.1, "{similarity:?}");
        assert!(similarity.score < 0.1, "{similarity:?}");
    }

    #[test]
    /// Verifies unrelated ToolPkg scripts do not produce a material similarity score.
    fn toolpkg_similarity_rejects_unrelated_code() {
        let reference = buildSimilarityTestToolPkg(
            r#"
                function normalizeName(value) { return value.trim().toLowerCase(); }
                function createGreeting(value) { return "Hello, " + normalizeName(value); }
                function createSummary(value) { return { greeting: createGreeting(value), size: value.length }; }
                globalThis.createSummary = createSummary;
            "#,
        );
        let unrelated = buildSimilarityTestToolPkg(
            r#"
                class TemperatureWindow {
                    constructor(minimum, maximum) { this.minimum = minimum; this.maximum = maximum; }
                    contains(value) { return value >= this.minimum && value <= this.maximum; }
                }
                globalThis.TemperatureWindow = TemperatureWindow;
            "#,
        );
        let published = protectArtifactBytes(&unrelated, true).expect("ToolPkg should be minified");

        let similarity = compareToolPkgJavaScriptSimilarity(&reference, &published)
            .expect("unrelated package should be comparable");

        assert!(similarity.referenceCoverage < 0.1, "{similarity:?}");
        assert!(similarity.candidateCoverage < 0.1, "{similarity:?}");
        assert!(similarity.score < 0.1, "{similarity:?}");
    }
}
