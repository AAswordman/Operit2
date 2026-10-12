//! Runtime-independent theme v1 archive, contract, integrity and media checks.
//!
//! This crate performs no I/O, Host calls, runtime startup, or theme application.
//! `ContractCheckedTheme` is NOT an installable/fully validated archive. The ZIP,
//! media and origin-policy gates must run before a caller may render or install it.

#![forbid(unsafe_code)]

mod error;
mod archive;
mod export;
mod json;
mod media;
mod preview;
mod schema;

#[cfg(feature = "wasm")]
mod wasm;

pub use error::{ThemeValidationError, ValidationStage};
pub use archive::{inspect_theme_archive, InspectedThemeArchive};
pub use export::{export_theme_package, export_theme_package_from_transfer, ThemeExportAsset, ThemeExportDraft};
pub use media::ThemeMediaValidation;
pub use preview::{validate_theme_preview_draft, validate_theme_preview_transfer,
    ThemePreviewAsset, ThemePreviewValidationReceipt, ValidatedThemePreview};
use semver::Version;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Mutex, OnceLock};

pub const MANIFEST_SCHEMA_V1: &str = include_str!("../schema/manifest-v1.schema.json");
pub const THEME_SCHEMA_V1: &str = include_str!("../schema/theme-v1.schema.json");
pub const MAX_ARCHIVE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_EXPANDED_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;
pub const MAX_THEME_BYTES: usize = 256 * 1024;
pub const MAX_FILES: usize = 64;
pub const MAX_RESOURCES: usize = 24;
pub const MAX_FONTS: usize = 3;
pub const MAX_IMAGE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_SVG_BYTES: usize = 1024 * 1024;
pub const MAX_FONT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_LICENSE_BYTES: usize = 128 * 1024;
pub const MAX_PREVIEW_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_IMAGE_DIMENSION: usize = 4096;
pub const MAX_IMAGE_PIXELS: usize = 16_777_216;
pub const MAX_TOTAL_IMAGE_PIXELS: usize = 64_000_000;
pub const PREVIEW_WIDTH: usize = 1280;
pub const PREVIEW_HEIGHT: usize = 800;
pub const MAX_COMPRESSION_RATIO: usize = 100;
pub const MAX_FONT_TABLES: usize = 64;
pub const MAX_FONT_GLYPHS: usize = 65_535;
pub const MAX_VALIDATOR_MEMORY_BYTES: usize = 512 * 1024 * 1024;
pub const MAX_VALIDATOR_TASK_MILLIS: usize = 15_000;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestV1 {
    pub format: String,
    pub format_version: u32,
    pub theme_api_version: u32,
    pub theme_id: String,
    pub version: String,
    pub display_name: String,
    pub author: Author,
    pub license: String,
    pub min_app_version: String,
    pub platforms: Vec<String>,
    pub files: Vec<ManifestFile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_app_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_with: Option<CreatedWith>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub asset_licenses: Vec<AssetLicense>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Author { pub name: String }

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestFile {
    pub path: String,
    pub media_type: String,
    pub bytes: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreatedWith {
    pub studio_version: String,
    pub renderer_commit: String,
    pub flutter_revision: String,
    pub fixture_version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssetLicense { pub asset: String, pub license: String, pub text: String }

#[derive(Debug, Clone)]
pub struct ThemeSpecV1 { value: Value }

impl ThemeSpecV1 {
    pub fn as_json(&self) -> &Value { &self.value }

    /// v1 canonical representation: sorted object keys, compact UTF-8 JSON,
    /// integral numeric visual tokens represented as integers (including -0).
    /// No claims about arbitrary JSON or RFC 8785 attestation payloads are made.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        write_canonical(&self.value, &mut bytes);
        bytes
    }

    pub fn sha256(&self) -> String { sha256_hex(&self.canonical_bytes()) }

        pub fn asset_paths(&self) -> BTreeSet<String> {
        ["/background/asset", "/chat/user/image/asset", "/chat/ai/image/asset",
         "/input/image/asset", "/input/settingsAsset", "/input/attachmentAsset", "/input/micAsset", "/input/sendAsset", "/input/stopAsset",
         "/typography/global", "/typography/user", "/typography/ai"]
            .iter().filter_map(|pointer| self.value.pointer(pointer).and_then(Value::as_str))
            .filter(|value| value.starts_with("assets/"))
            .map(str::to_owned).collect()
    }
}

/// Only the contract and file set are checked. Never use this as a media-safety
/// token, install receipt, official badge, or evidence of ZIP validation.
#[derive(Debug, Clone)]
pub struct ContractCheckedTheme {
    manifest: ManifestV1,
    theme: ThemeSpecV1,
    manifest_sha256: String,
}

/// The archive package value allowed to cross validation into an installer or
/// package preview. Construction is private and requires L0-L4 validation.
/// In-memory authoring uses the separately scoped ValidatedThemePreview type.
#[derive(Debug, Clone)]
pub struct ValidatedThemePackage {
    sealed_archive: Vec<u8>,
    archive_sha256: String,
    archive_bytes: usize,
    expanded_bytes: usize,
    manifest: ManifestV1,
    theme: ThemeSpecV1,
    files: BTreeMap<String, Vec<u8>>,
    media: Vec<ThemeMediaValidation>,
    receipt: ThemeValidationReceipt,
}

/// Deterministic evidence produced by the complete package validator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeValidationReceipt {
    pub validator_version: u32,
    pub archive_sha256: String,
    pub archive_bytes: usize,
    pub expanded_bytes: usize,
    pub manifest_sha256: String,
    pub theme_sha256: String,
    pub files: Vec<ThemeFileValidation>,
    pub media: Vec<ThemeMediaValidation>,
    pub trust: ThemePackageTrust,
}

/// Byte/structure validation never asserts publisher identity or curation.
/// The origin-policy and attestation layer will add verified trust separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemePackageTrust {
    Untrusted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeFileValidation {
    pub path: String,
    pub bytes: usize,
    pub sha256: String,
}

impl ValidatedThemePackage {
    /// The exact original ZIP whose digest appears in this package's receipt.
    /// Installers must persist these bytes, not an author-supplied pathname.
    pub fn sealed_archive_bytes(&self) -> &[u8] { &self.sealed_archive }
    pub fn archive_sha256(&self) -> &str { &self.archive_sha256 }
    pub fn archive_bytes(&self) -> usize { self.archive_bytes }
    pub fn expanded_bytes(&self) -> usize { self.expanded_bytes }
    pub fn manifest(&self) -> &ManifestV1 { &self.manifest }
    pub fn theme(&self) -> &ThemeSpecV1 { &self.theme }
    pub fn media(&self) -> &[ThemeMediaValidation] { &self.media }
    pub fn receipt(&self) -> &ThemeValidationReceipt { &self.receipt }

    /// Returns bytes only for a path already covered by the validation receipt.
    pub fn file_bytes(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(Vec::as_slice)
    }
}

impl ContractCheckedTheme {
    pub fn manifest(&self) -> &ManifestV1 { &self.manifest }
    pub fn theme(&self) -> &ThemeSpecV1 { &self.theme }
    pub fn manifest_sha256(&self) -> &str { &self.manifest_sha256 }
}

pub fn sha256_hex(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }

fn manifest_schema_v1() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| serde_json::from_str(MANIFEST_SCHEMA_V1).expect("trusted manifest schema"))
}

fn theme_schema_v1() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| serde_json::from_str(THEME_SCHEMA_V1).expect("trusted theme schema"))
}

pub fn parse_manifest(bytes: &[u8], app_version: &str) -> Result<ManifestV1, ThemeValidationError> {
    let value = json::parse(bytes, MAX_MANIFEST_BYTES, "manifest.json")?;
    let definition = manifest_schema_v1();
    schema::validate(&definition, &definition, &value, "manifest.json", true)?;
    let manifest: ManifestV1 = serde_json::from_value(value)
        .map_err(|e| ThemeValidationError::contract("manifest.json", e.to_string()))?;
    validate_versions(&manifest, app_version)?;
    let mut paths = BTreeSet::new();
    let mut expanded = bytes.len();
    let mut resources = 0;
    let mut fonts = 0;
    for file in &manifest.files {
        validate_package_path(&file.path)?;
        if !paths.insert(file.path.clone()) {
            return Err(integrity(&file.path, "duplicate manifest file"));
        }
        let (media_type, limit) = metadata_for_path(&file.path)?;
        if file.media_type != media_type || file.bytes > limit {
            return Err(ThemeValidationError::contract(&file.path, "media type or declared byte budget is invalid"));
        }
        expanded = expanded.checked_add(file.bytes)
            .ok_or_else(|| integrity("manifest.json/files", "expanded byte count overflow"))?;
        if file.path.starts_with("assets/") { resources += 1; }
        if file.path.starts_with("assets/fonts/") { fonts += 1; }
    }
    if expanded > MAX_EXPANDED_BYTES || resources > MAX_RESOURCES || fonts > MAX_FONTS {
        return Err(ThemeValidationError::contract("manifest.json/files", "package exceeds v1 resource budget"));
    }
    for required in ["theme.json", "preview.png", "licenses/theme.txt"] {
        if !paths.contains(required) { return Err(integrity(required, "required package file is missing")); }
    }
    Ok(manifest)
}

/// Packages must contain the complete expanded visual state. Missing tokens are
/// rejected, even when the draft editor has a schema-defined default for them.
pub fn parse_theme(bytes: &[u8]) -> Result<ThemeSpecV1, ThemeValidationError> {
    let mut value = json::parse(bytes, MAX_THEME_BYTES, "theme.json")?;
    let definition = theme_schema_v1();
    schema::validate(&definition, &definition, &value, "theme.json", true)?;
    schema::normalize(&definition, &definition, &mut value);
    check_visual_relations(&value)?;
    finish_spec(value)
}

/// The only place where omitted authoring tokens receive versioned defaults.
/// Explicit illegal values, unknown fields, and bad resource paths are errors.
pub fn normalize_draft(bytes: &[u8]) -> Result<ThemeSpecV1, ThemeValidationError> {
    let mut value = json::parse(bytes, MAX_THEME_BYTES, "theme.json")?;
    let definition = theme_schema_v1();
    schema::validate(&definition, &definition, &value, "theme.json", false)?;
    schema::normalize(&definition, &definition, &mut value);
    schema::validate(&definition, &definition, &value, "theme.json", true)?;
    check_visual_relations(&value)?;
    finish_spec(value)
}

fn finish_spec(value: Value) -> Result<ThemeSpecV1, ThemeValidationError> {
    let spec = ThemeSpecV1 { value };
    for path in spec.asset_paths() { validate_package_path(&path)?; }
    Ok(spec)
}

/// Sorting is explicit, even if another workspace crate enables preserve_order
/// on serde_json. The digest may not depend on Cargo feature unification.
fn write_canonical(value: &Value, bytes: &mut Vec<u8>) {
    match value {
        Value::Object(object) => {
            bytes.push(b'{');
            let sorted: BTreeMap<_, _> = object.iter().collect();
            for (index, (key, child)) in sorted.into_iter().enumerate() {
                if index > 0 { bytes.push(b','); }
                bytes.extend(serde_json::to_vec(key).expect("checked JSON key"));
                bytes.push(b':');
                write_canonical(child, bytes);
            }
            bytes.push(b'}');
        }
        Value::Array(array) => {
            bytes.push(b'[');
            for (index, child) in array.iter().enumerate() {
                if index > 0 { bytes.push(b','); }
                write_canonical(child, bytes);
            }
            bytes.push(b']');
        }
        _ => bytes.extend(serde_json::to_vec(value).expect("checked JSON scalar")),
    }
}

fn check_visual_relations(value: &Value) -> Result<(), ThemeValidationError> {
    for (base, image) in [
        ("/chat/user/image", &value["chat"]["user"]["image"]),
        ("/chat/ai/image", &value["chat"]["ai"]["image"]),
        ("/input/image", &value["input"]["image"]),
    ] {
        if image.is_null() { continue; }
        for axis in ["x", "y"] {
            let start = image["slice"][format!("{axis}Start")].as_f64().unwrap();
            let end = image["slice"][format!("{axis}End")].as_f64().unwrap();
            if end < start + 0.01 {
                return Err(ThemeValidationError::contract(&format!("theme.json{base}/slice/{axis}End"),
                    "slice end must be at least start + 0.01"));
            }
        }
    }
    Ok(())
}

/// Files must come from a separately bounded archive inspector. This function
/// does not read or extract ZIPs and cannot prove decoder/trust safety.
pub fn check_file_set(
    files: &BTreeMap<String, Vec<u8>>, app_version: &str,
) -> Result<ContractCheckedTheme, ThemeValidationError> {
    if files.len() > MAX_FILES { return Err(integrity("", "too many package files")); }
    let mut total = 0usize;
    for (path, bytes) in files {
        let bytes = bytes.as_slice();
        validate_package_path(path)?;
        total = total.checked_add(bytes.len()).ok_or_else(|| integrity(path, "byte count overflow"))?;
        if total > MAX_EXPANDED_BYTES { return Err(integrity(path, "expanded bytes exceed budget")); }
    }
    let manifest_bytes = files.get("manifest.json").ok_or_else(|| integrity("manifest.json", "manifest is missing"))?.as_slice();
    let manifest = parse_manifest(manifest_bytes, app_version)?;
    let mut declared = BTreeSet::from(["manifest.json".to_owned()]);
    for file in &manifest.files { declared.insert(file.path.clone()); }
    if declared != files.keys().cloned().collect::<BTreeSet<_>>() {
        return Err(integrity("manifest.json/files", "actual files must exactly match manifest.files"));
    }
    for file in &manifest.files {
        let bytes = files[&file.path].as_slice();
        if bytes.len() != file.bytes || sha256_hex(bytes) != file.sha256 {
            return Err(integrity(&file.path, "file byte count or SHA-256 differs from manifest"));
        }
        if file.media_type == "text/plain" {
            std::str::from_utf8(bytes).map_err(|_| integrity(&file.path, "license text must be UTF-8"))?;
        }
    }
    let theme = parse_theme(files["theme.json"].as_slice())?;
    let assets = theme.asset_paths();
    let declared_assets: BTreeSet<_> = manifest.files.iter().filter(|f| f.path.starts_with("assets/"))
        .map(|f| f.path.clone()).collect();
    if assets != declared_assets {
        return Err(integrity("theme.json", "referenced assets must exactly match packaged assets"));
    }
    let mut licensed_assets = BTreeSet::new();
    let mut used_licenses = BTreeSet::from(["licenses/theme.txt".to_owned()]);
    for license in &manifest.asset_licenses {
        validate_package_path(&license.asset)?;
        validate_package_path(&license.text)?;
        if !assets.contains(&license.asset) || !declared.contains(&license.text) || !licensed_assets.insert(license.asset.clone()) {
            return Err(integrity(&license.asset, "asset license is duplicate or refers to missing content"));
        }
        used_licenses.insert(license.text.clone());
    }
    if assets != licensed_assets {
        return Err(ThemeValidationError::new("theme-asset-license-missing", ValidationStage::Integrity,
            "manifest.json/assetLicenses", "every asset needs an SPDX ID and packaged license text"));
    }
    let actual_licenses: BTreeSet<_> = declared.iter().filter(|p| p.starts_with("licenses/")).cloned().collect();
    if actual_licenses != used_licenses { return Err(integrity("licenses", "unreferenced license file")); }
    Ok(ContractCheckedTheme { manifest, theme, manifest_sha256: sha256_hex(manifest_bytes) })
}

/// Runs the fail-closed L0-L4 pipeline in one operation.  No filesystem or
/// network access occurs here; the caller owns the input byte lifetime and must
/// decide separately whether to import, preview or apply the validated package.
fn validated_package_cache() -> &'static Mutex<VecDeque<(String, String, ValidatedThemePackage)>> {
    static CACHE: OnceLock<Mutex<VecDeque<(String, String, ValidatedThemePackage)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(VecDeque::with_capacity(4)))
}

pub fn validate_theme_package(
    archive_bytes: &[u8],
    app_version: &str,
) -> Result<ValidatedThemePackage, ThemeValidationError> {
    if !archive_bytes.is_empty() && archive_bytes.len() <= MAX_ARCHIVE_BYTES {
        let digest = sha256_hex(archive_bytes);
        if let Ok(cache) = validated_package_cache().lock() {
            if let Some((_, _, cached)) = cache.iter().find(|(sha, ver, pkg)| {
                sha == &digest && ver == app_version && pkg.sealed_archive_bytes() == archive_bytes
            }) {
                return Ok(cached.clone());
            }
        }
    }
    let inspected = inspect_theme_archive(archive_bytes)?;
    let checked = check_file_set(&inspected.files, app_version)?;
    let media = media::validate_media_files(&inspected.files, &checked.manifest.files)?;
    media::validate_bubble_pixels(&checked.theme, &media)?;
    let declared_hashes: BTreeMap<&str, &str> = checked
        .manifest
        .files
        .iter()
        .map(|file| (file.path.as_str(), file.sha256.as_str()))
        .collect();
    let mut files = Vec::with_capacity(inspected.files.len());
    for (path, bytes) in &inspected.files {
        let digest = if path == "manifest.json" {
            checked.manifest_sha256.clone()
        } else {
            declared_hashes
                .get(path.as_str())
                .map(|s| (*s).to_owned())
                .unwrap_or_else(|| sha256_hex(bytes))
        };
        files.push(ThemeFileValidation {
            path: path.clone(),
            bytes: bytes.len(),
            sha256: digest,
        });
    }
    let receipt = ThemeValidationReceipt {
        validator_version: 1,
        archive_sha256: inspected.archive_sha256.clone(),
        archive_bytes: inspected.archive_bytes,
        expanded_bytes: inspected.expanded_bytes,
        manifest_sha256: checked.manifest_sha256.clone(),
        theme_sha256: checked.theme.sha256(),
        files,
        media: media.clone(),
        trust: ThemePackageTrust::Untrusted,
    };
    let validated = ValidatedThemePackage {
        sealed_archive: archive_bytes.to_vec(),
        archive_sha256: inspected.archive_sha256.clone(),
        archive_bytes: inspected.archive_bytes,
        expanded_bytes: inspected.expanded_bytes,
        manifest: checked.manifest,
        theme: checked.theme,
        files: inspected.files,
        media,
        receipt,
    };
    if let Ok(mut cache) = validated_package_cache().lock() {
        cache.retain(|(sha, ver, _)| !(sha == &validated.archive_sha256 && ver == app_version));
        if cache.len() >= 4 {
            cache.pop_front();
        }
        cache.push_back((
            validated.archive_sha256.clone(),
            app_version.to_owned(),
            validated.clone(),
        ));
    }
    Ok(validated)
}

/// Enforces an external download/confirmation identity before running the
/// package pipeline. An expected SHA-256 is not an identity or review proof.
pub fn validate_theme_package_with_expectation(
    archive_bytes: &[u8], app_version: &str, expected_bytes: usize, expected_sha256: &str,
) -> Result<ValidatedThemePackage, ThemeValidationError> {
    if expected_sha256.len() != 64
        || !expected_sha256.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || archive_bytes.len() != expected_bytes || sha256_hex(archive_bytes) != expected_sha256
    {
        return Err(ThemeValidationError::new("theme-download-integrity-invalid",
            ValidationStage::Input, "", "archive differs from the confirmed byte count or SHA-256"));
    }
    validate_theme_package(archive_bytes, app_version)
}

/// Paths are rejected verbatim, not normalized, sanitized, or resolved on disk.
pub fn validate_package_path(path: &str) -> Result<(), ThemeValidationError> {
    if path.is_empty() || path.len() > 128 || !path.is_ascii() || path.split('/').count() > 4 {
        return Err(integrity(path, "path exceeds v1 naming budget"));
    }
    for part in path.split('/') {
        if part.is_empty() || part == "." || part == ".." || part.ends_with('.') ||
            !part.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-' || b == b'.') {
            return Err(integrity(path, "invalid path component"));
        }
        let stem = part.split('.').next().unwrap();
        if ["con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9",
            "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9"].contains(&stem) {
            return Err(integrity(path, "Windows reserved path name"));
        }
    }
    static PATH_PATTERN: OnceLock<Regex> = OnceLock::new();
    let pattern = PATH_PATTERN.get_or_init(|| {
        Regex::new(concat!(
            r"^(manifest\.json|theme\.json|preview\.png|",
            r"assets/images/([a-z0-9_-]+/)?[a-z0-9_-]+\.(png|jpg|jpeg|svg)|",
            r"assets/fonts/([a-z0-9_-]+/)?[a-z0-9_-]+\.ttf|",
            r"licenses/([a-z0-9_-]+/)?[a-z0-9_-]+\.txt)$",
        ))
        .expect("trusted path expression")
    });
    if !pattern.is_match(path) { return Err(integrity(path, "unregistered path spelling")); }
    metadata_for_path(path)?;
    Ok(())
}

fn metadata_for_path(path: &str) -> Result<(&'static str, usize), ThemeValidationError> {
    match path {
        "manifest.json" => Ok(("application/json", MAX_MANIFEST_BYTES)),
        "theme.json" => Ok(("application/json", MAX_THEME_BYTES)),
        "preview.png" => Ok(("image/png", MAX_PREVIEW_BYTES)),
        _ if path.starts_with("assets/images/") && path.ends_with(".png") => Ok(("image/png", MAX_IMAGE_BYTES)),
        _ if path.starts_with("assets/images/") && (path.ends_with(".jpg") || path.ends_with(".jpeg")) => Ok(("image/jpeg", MAX_IMAGE_BYTES)),
        _ if path.starts_with("assets/images/") && path.ends_with(".svg") => Ok(("image/svg+xml", MAX_SVG_BYTES)),
        _ if path.starts_with("assets/fonts/") && path.ends_with(".ttf") => Ok(("font/ttf", MAX_FONT_BYTES)),
        _ if path.starts_with("licenses/") && path.ends_with(".txt") => Ok(("text/plain", MAX_LICENSE_BYTES)),
        _ => Err(integrity(path, "unregistered package path or file type")),
    }
}

fn validate_versions(manifest: &ManifestV1, app_version: &str) -> Result<(), ThemeValidationError> {
    let parse = |value: &str, path: &str| Version::parse(value)
        .map_err(|_| ThemeValidationError::contract(path, "complete SemVer is required"));
    parse(&manifest.version, "manifest.json/version")?;
    let minimum = parse(&manifest.min_app_version, "manifest.json/minAppVersion")?;
    let current = parse(app_version, "appVersion")?;
    if current.cmp_precedence(&minimum).is_lt() {
        return Err(ThemeValidationError::new("theme-compatibility-invalid", ValidationStage::Contract,
            "manifest.json/minAppVersion", "this theme requires a newer Core version"));
    }
    if let Some(maximum) = &manifest.max_app_version {
        let maximum = parse(maximum, "manifest.json/maxAppVersion")?;
        if maximum.cmp_precedence(&minimum).is_lt() || current.cmp_precedence(&maximum).is_gt() {
            return Err(ThemeValidationError::new("theme-compatibility-invalid", ValidationStage::Contract,
                "manifest.json/maxAppVersion", "Core version is outside the theme compatibility interval"));
        }
    }
    Ok(())
}

fn integrity(path: &str, message: &str) -> ThemeValidationError {
    ThemeValidationError::new("theme-package-integrity-invalid", ValidationStage::Integrity, path, message)
}
