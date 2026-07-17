#![allow(non_snake_case)]

use crate::toolpkg::ToolPkgParser::ToolPkgArchiveParser;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;

pub const PROTECTION_ID: &str = "operit-protected-v1";

const MAGIC: &[u8; 8] = b"OPTPROT1";
const NONCE_SIZE: usize = 16;
const TAG_SIZE: usize = 32;
const HEADER_SIZE: usize = MAGIC.len() + NONCE_SIZE + TAG_SIZE;
const STREAM_DOMAIN: &[u8] = b"operit-toolpkg-stream-v1";
const SECRET_KEY: &str = "OPERIT_TOOLPKG_PROTECTION_SECRET";

#[allow(non_snake_case)]
pub fn isProtected(bytes: &[u8]) -> bool {
    bytes.len() >= MAGIC.len() && &bytes[..MAGIC.len()] == MAGIC
}

#[allow(non_snake_case)]
pub fn decryptIfNeeded(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if isProtected(bytes) {
        decrypt(bytes)
    } else {
        Ok(bytes.to_vec())
    }
}

#[allow(non_snake_case)]
pub fn decodeUtf8(bytes: &[u8]) -> Result<String, String> {
    String::from_utf8(decryptIfNeeded(bytes)?).map_err(|e| e.to_string())
}

#[allow(non_snake_case)]
pub fn isSecretConfigured() -> bool {
    protectionKey().is_some()
}

pub fn encrypt(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.is_empty() {
        return Err("Cannot protect empty content".to_string());
    }
    if isProtected(bytes) {
        return Ok(bytes.to_vec());
    }
    let key =
        protectionKey().ok_or_else(|| "ToolPkg protection secret is not configured".to_string())?;
    let nonce = *uuid::Uuid::new_v4().as_bytes();
    let ciphertext = xorWithKeystream(bytes, &key, &nonce);
    let tag = hmacSha256(&key, &buildTagMessage(&nonce, &ciphertext));
    let mut output = Vec::with_capacity(HEADER_SIZE + ciphertext.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&tag);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

#[allow(non_snake_case)]
/// Protects one JavaScript or ToolPkg artifact supplied as bytes.
pub fn protectArtifactBytes(sourceBytes: &[u8], isToolPkg: bool) -> Result<Vec<u8>, String> {
    if !isSecretConfigured() {
        return Err("ToolPkg protection secret is not configured".to_string());
    }
    if isToolPkg {
        protectToolPkgArchive(sourceBytes)
    } else {
        encrypt(sourceBytes)
    }
}

fn decrypt(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let key =
        protectionKey().ok_or_else(|| "ToolPkg protection secret is not configured".to_string())?;
    if bytes.len() < HEADER_SIZE || &bytes[..MAGIC.len()] != MAGIC {
        return Err("Not an Operit protected payload".to_string());
    }
    let mut nonce = [0u8; NONCE_SIZE];
    nonce.copy_from_slice(&bytes[MAGIC.len()..MAGIC.len() + NONCE_SIZE]);
    let providedTag = &bytes[MAGIC.len() + NONCE_SIZE..HEADER_SIZE];
    let ciphertext = &bytes[HEADER_SIZE..];
    let expectedTag = hmacSha256(&key, &buildTagMessage(&nonce, ciphertext));
    if !constantTimeEquals(providedTag, &expectedTag) {
        return Err("Protected payload authentication failed".to_string());
    }
    Ok(xorWithKeystream(ciphertext, &key, &nonce))
}

#[allow(non_snake_case)]
fn protectToolPkgArchive(sourceBytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(sourceBytes)).map_err(|e| e.to_string())?;
    let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
    let mPreview = ToolPkgArchiveParser::readToolPkgManifestPreview(&mut archive, &entryIndex)
        .ok_or_else(|| "manifest.hjson or manifest.json not found".to_string())?;
    let manifestBasePath = mPreview
        .entryName
        .rsplit_once('/')
        .map(|(basePath, _)| basePath.to_string())
        .unwrap_or_default();
    let mut protectedEntryNames = BTreeSet::new();
    let mut plainResourceEntryRoots = BTreeSet::new();
    if let Some(entry) = ToolPkgArchiveParser::resolveManifestRelativeZipEntryPath(
        &manifestBasePath,
        &mPreview.manifest.main,
    ) {
        protectedEntryNames.insert(entry);
    }
    for subpackage in &mPreview.manifest.subpackages {
        if let Some(entry) = ToolPkgArchiveParser::resolveManifestRelativeZipEntryPath(
            &manifestBasePath,
            &subpackage.entry,
        ) {
            protectedEntryNames.insert(entry);
        }
    }
    for resource in &mPreview.manifest.resources {
        if let Some(root) = ToolPkgArchiveParser::resolveManifestRelativeResourcePath(
            &manifestBasePath,
            &resource.path,
        ) {
            plainResourceEntryRoots.insert(root);
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
            let Some(norm) = ToolPkgArchiveParser::normalizeZipEntryPath(&name) else {
                continue;
            };
            let mut orig = Vec::new();
            entry.read_to_end(&mut orig).map_err(|e| e.to_string())?;
            let data = if shouldProtect(&norm, &protectedEntryNames, &plainResourceEntryRoots) {
                encrypt(&orig)?
            } else {
                orig
            };
            w.start_file(name, options).map_err(|e| e.to_string())?;
            w.write_all(&data).map_err(|e| e.to_string())?;
        }
        w.finish().map_err(|e| e.to_string())?;
    }
    Ok(out)
}

fn shouldProtect(norm: &str, prot: &BTreeSet<String>, plain: &BTreeSet<String>) -> bool {
    if plain
        .iter()
        .any(|r| norm == r || norm.starts_with(&format!("{r}/")))
    {
        return false;
    }
    if prot.contains(norm) {
        return true;
    }
    matches!(
        norm.rsplit_once('.')
            .map(|(_, ext)| ext.to_ascii_lowercase())
            .as_deref(),
        Some("js" | "mjs" | "cjs" | "ts" | "jsx" | "tsx")
    )
}

fn protectionKey() -> Option<Vec<u8>> {
    if let Ok(v) = std::env::var(SECRET_KEY) {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.as_bytes().to_vec());
        }
    }
    if let Some(v) = option_env!("OPERIT_TOOLPKG_PROTECTION_SECRET") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.as_bytes().to_vec());
        }
    }
    None
}

fn hmacSha256(key: &[u8], msg: &[u8]) -> [u8; TAG_SIZE] {
    let mut bk = [0u8; 64];
    if key.len() > bk.len() {
        bk.copy_from_slice(&sha256(key));
    } else {
        bk[..key.len()].copy_from_slice(key);
    }
    let mut inner = Vec::with_capacity(bk.len() + msg.len());
    let mut outer = Vec::with_capacity(bk.len() + TAG_SIZE);
    for b in bk {
        inner.push(b ^ 0x36);
        outer.push(b ^ 0x5c);
    }
    inner.extend_from_slice(msg);
    outer.extend_from_slice(&sha256(&inner));
    sha256(&outer)
}

fn sha256(bytes: &[u8]) -> [u8; TAG_SIZE] {
    let d = Sha256::digest(bytes);
    let mut o = [0u8; TAG_SIZE];
    o.copy_from_slice(&d);
    o
}

fn buildTagMessage(nonce: &[u8; NONCE_SIZE], ct: &[u8]) -> Vec<u8> {
    let mut m = Vec::with_capacity(MAGIC.len() + nonce.len() + ct.len());
    m.extend_from_slice(MAGIC);
    m.extend_from_slice(nonce);
    m.extend_from_slice(ct);
    m
}

fn keystreamBlock(key: &[u8], nonce: &[u8; NONCE_SIZE], counter: u64) -> [u8; TAG_SIZE] {
    let mut m = Vec::with_capacity(STREAM_DOMAIN.len() + nonce.len() + 8);
    m.extend_from_slice(STREAM_DOMAIN);
    m.extend_from_slice(nonce);
    m.extend_from_slice(&counter.to_be_bytes());
    hmacSha256(key, &m)
}

fn xorWithKeystream(input: &[u8], key: &[u8], nonce: &[u8; NONCE_SIZE]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut counter = 0u64;
    let mut off = 0usize;
    while off < input.len() {
        let block = keystreamBlock(key, nonce, counter);
        counter += 1;
        let cnt = block.len().min(input.len() - off);
        for i in 0..cnt {
            out.push(input[off + i] ^ block[i]);
        }
        off += cnt;
    }
    out
}

fn constantTimeEquals(l: &[u8], r: &[u8]) -> bool {
    if l.len() != r.len() {
        return false;
    }
    let mut d = 0u8;
    for i in 0..l.len() {
        d |= l[i] ^ r[i];
    }
    d == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::sync::Mutex;

    static SECRET_ENV_LOCK: Mutex<()> = Mutex::new(());

    fn withTestSecret<T>(run: impl FnOnce() -> T) -> T {
        let _guard = SECRET_ENV_LOCK.lock().expect("secret env lock poisoned");
        let previous = std::env::var(SECRET_KEY).ok();
        std::env::set_var(SECRET_KEY, "operit-toolpkg-unit-test-secret");
        let result = run();
        if let Some(value) = previous {
            std::env::set_var(SECRET_KEY, value);
        } else {
            std::env::remove_var(SECRET_KEY);
        }
        result
    }

    #[test]
    fn magic_check() {
        assert!(isProtected(b"OPTPROT1abc"));
        assert!(!isProtected(b"OPTPROT0abc"));
        assert!(!isProtected(b"short"));
    }

    #[test]
    fn encryption_round_trips_with_configured_secret() {
        withTestSecret(|| {
            let plaintext = b"console.log('hello')";
            let encrypted = encrypt(plaintext).expect("content should encrypt");
            assert!(isProtected(&encrypted));
            assert_eq!(
                decryptIfNeeded(&encrypted).expect("content should decrypt"),
                plaintext
            );
        });
    }

    #[test]
    fn resource_plain_override() {
        let mut p = BTreeSet::new();
        p.insert("dist/main.js".to_string());
        let mut r = BTreeSet::new();
        r.insert("assets".to_string());
        assert!(!shouldProtect("assets/web/app.js", &p, &r));
        assert!(shouldProtect("dist/main.js", &p, &r));
    }

    #[test]
    fn toolpkg_protection_keeps_manifest_resources_plain() {
        withTestSecret(|| {
            let mut sourceBytes = Vec::new();
            let mut zip = zip::ZipWriter::new(Cursor::new(&mut sourceBytes));
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("manifest.json", options)
                .expect("manifest entry should start");
            zip.write_all(
                br#"{
                    "toolpkg_id": "protection-test",
                    "version": "1.0.0",
                    "main": "main.js",
                    "display_name": "Protection Test",
                    "description": "Protection Test",
                    "author": ["Operit"],
                    "resources": [
                        {"key": "web", "path": "assets", "mime": "vnd.android.document/directory"}
                    ]
                }"#,
            )
            .expect("manifest should be written");
            zip.start_file("main.js", options)
                .expect("main entry should start");
            zip.write_all(b"globalThis.main = true;")
                .expect("main should be written");
            zip.start_file("lib/helper.js", options)
                .expect("helper entry should start");
            zip.write_all(b"globalThis.helper = true;")
                .expect("helper should be written");
            zip.start_file("assets/app.js", options)
                .expect("asset entry should start");
            zip.write_all(b"window.asset = true;")
                .expect("asset should be written");
            zip.finish().expect("test zip should finish");

            let protectedBytes =
                protectArtifactBytes(&sourceBytes, true).expect("toolpkg should be protected");

            let cursor = Cursor::new(protectedBytes);
            let mut protectedZip =
                zip::ZipArchive::new(cursor).expect("protected toolpkg should be a zip");
            let mut mainBytes = Vec::new();
            protectedZip
                .by_name("main.js")
                .expect("main entry should exist")
                .read_to_end(&mut mainBytes)
                .expect("main entry should read");
            let mut helperBytes = Vec::new();
            protectedZip
                .by_name("lib/helper.js")
                .expect("helper entry should exist")
                .read_to_end(&mut helperBytes)
                .expect("helper entry should read");
            let mut assetBytes = Vec::new();
            protectedZip
                .by_name("assets/app.js")
                .expect("asset entry should exist")
                .read_to_end(&mut assetBytes)
                .expect("asset entry should read");

            assert!(isProtected(&mainBytes));
            assert!(isProtected(&helperBytes));
            assert!(!isProtected(&assetBytes));
            assert_eq!(
                decodeUtf8(&mainBytes).expect("main should decrypt"),
                "globalThis.main = true;"
            );
            assert_eq!(assetBytes, b"window.asset = true;");
        });
    }
}
