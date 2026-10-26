//! Theme template definitions, renderer media checks, and ToolPkg authoring support.
//!
//! This crate performs no I/O, Host calls, runtime startup, or theme application.
//! Renderer evidence applies only to declared template resources. ToolPkg loading,
//! executable protection and installation remain the shared package system's responsibility.

#![forbid(unsafe_code)]

mod error;
mod export;
mod json;
mod media;
mod preview;
mod schema;
mod toolpkg;

pub use toolpkg::{
    toolpkg_manifest_value, validate_authored_toolpkg_theme_templates, validate_declared_theme_templates, validate_toolpkg_theme_template,
    validate_toolpkg_theme_templates,
};

#[cfg(feature = "wasm")]
mod wasm;

pub use error::{ThemeValidationError, ValidationStage};
pub use export::{
    export_toolpkg_from_theme, export_toolpkg_from_theme_transfer, ThemeExportAsset,
    ThemeExportDraft,
};
pub use media::ThemeMediaValidation;
pub use preview::{
    validate_theme_preview_draft, validate_theme_preview_transfer, ThemePreviewAsset,
    ThemePreviewValidationReceipt, ValidatedThemePreview,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

pub const THEME_TEMPLATE_SCHEMA: &str = include_str!("../schema/theme-template.schema.json");
pub const THEME_SCHEMA_V1: &str = include_str!("../schema/theme-v1.schema.json");
pub const MAX_ARCHIVE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_EXPANDED_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;
pub const MAX_THEME_BYTES: usize = 256 * 1024;
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
pub const MAX_FONT_TABLES: usize = 64;
pub const MAX_FONT_GLYPHS: usize = 65_535;
pub const MAX_VALIDATOR_MEMORY_BYTES: usize = 512 * 1024 * 1024;
pub const MAX_VALIDATOR_TASK_MILLIS: usize = 15_000;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeTemplateMetadata {
    pub theme_id: String,
    pub package_id: String,
    pub template_id: String,
    pub path: String,
    pub preview: String,
    pub version: String,
    pub display_name: String,
    pub author: Author,
    pub license: Option<String>,
    pub files: Vec<ManifestFile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Author {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestFile {
    pub path: String,
    pub media_type: String,
}

#[derive(Debug, Clone)]
pub struct ThemeSpecV1 {
    value: Value,
}

impl ThemeSpecV1 {
    /// Returns the checked visual definition for renderer transport.
    pub fn as_json(&self) -> &Value {
        &self.value
    }

    /// v1 canonical representation: sorted object keys, compact UTF-8 JSON,
    /// integral numeric visual tokens represented as integers (including -0).
    /// No claims about arbitrary JSON or RFC 8785 attestation payloads are made.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        write_canonical(&self.value, &mut bytes);
        bytes
    }

    /// Hashes the canonical visual definition.
    pub fn sha256(&self) -> String {
        sha256_hex(&self.canonical_bytes())
    }

    /// Collects the visual asset references owned by this template.
    pub fn asset_paths(&self) -> BTreeSet<String> {
        [
            "/background/asset",
            "/chat/user/image/asset",
            "/chat/ai/image/asset",
            "/input/image/asset",
            "/input/settingsAsset",
            "/input/attachmentAsset",
            "/input/micAsset",
            "/input/sendAsset",
            "/input/stopAsset",
            "/typography/global",
            "/typography/user",
            "/typography/ai",
        ]
        .iter()
        .filter_map(|pointer| self.value.pointer(pointer).and_then(Value::as_str))
        .filter(|value| value.starts_with("assets/"))
        .map(str::to_owned)
        .collect()
    }
}

/// Only the contract and file set are checked. Never use this as a media-safety
/// token, install receipt, official badge, or evidence of ZIP validation.
#[derive(Debug, Clone)]
pub struct ContractCheckedTheme {
    manifest: ThemeTemplateMetadata,
    theme: ThemeSpecV1,
    manifest_sha256: String,
}

/// One validated theme template contributed by an ordinary ToolPkg.
/// Installation and all other package capabilities belong to the ToolPkg loader.
/// In-memory authoring uses the separately scoped ValidatedThemePreview type.
#[derive(Debug, Clone)]
pub struct ValidatedThemeTemplate {
    archive_sha256: String,
    archive_bytes: usize,
    expanded_bytes: usize,
    manifest: ThemeTemplateMetadata,
    theme: ThemeSpecV1,
    files: BTreeMap<String, Vec<u8>>,
    media: Vec<ThemeMediaValidation>,
    receipt: ThemeValidationReceipt,
}

/// Deterministic renderer evidence scoped to one declared template.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeValidationReceipt {
    pub validator_version: u32,
    pub theme_id: String,
    pub archive_sha256: String,
    pub archive_bytes: usize,
    pub expanded_bytes: usize,
    pub manifest_sha256: String,
    pub theme_sha256: String,
    pub files: Vec<ThemeFileValidation>,
    pub media: Vec<ThemeMediaValidation>,
    pub trust: ThemeTemplateTrust,
}

/// Byte/structure validation never asserts publisher identity or curation.
/// The origin-policy and attestation layer will add verified trust separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemeTemplateTrust {
    Untrusted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeFileValidation {
    pub path: String,
    pub bytes: usize,
    pub sha256: String,
}

impl ValidatedThemeTemplate {
    /// Returns the digest of the contributing ToolPkg archive.
    pub fn archive_sha256(&self) -> &str {
        &self.archive_sha256
    }
    /// Returns the contributing ToolPkg archive byte length.
    pub fn archive_bytes(&self) -> usize {
        self.archive_bytes
    }
    /// Returns the expanded byte length of this template's resources.
    pub fn expanded_bytes(&self) -> usize {
        self.expanded_bytes
    }
    /// Returns template identity and ordinary ToolPkg display metadata.
    pub fn metadata(&self) -> &ThemeTemplateMetadata {
        &self.manifest
    }
    /// Returns the checked visual definition.
    pub fn theme(&self) -> &ThemeSpecV1 {
        &self.theme
    }
    /// Returns media evidence for the template resources.
    pub fn media(&self) -> &[ThemeMediaValidation] {
        &self.media
    }
    /// Returns renderer evidence scoped to this template.
    pub fn receipt(&self) -> &ThemeValidationReceipt {
        &self.receipt
    }

    /// Returns bytes only for a path already covered by the validation receipt.
    pub fn file_bytes(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(Vec::as_slice)
    }
}

impl ContractCheckedTheme {
    /// Returns template identity and ordinary ToolPkg display metadata.
    pub fn metadata(&self) -> &ThemeTemplateMetadata {
        &self.manifest
    }
    /// Returns the checked visual definition.
    pub fn theme(&self) -> &ThemeSpecV1 {
        &self.theme
    }
    /// Returns the contributing ToolPkg manifest digest.
    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }
}

/// Computes a lowercase SHA-256 digest for exact resource bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Loads the trusted visual definition schema once.
fn theme_schema_v1() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| serde_json::from_str(THEME_SCHEMA_V1).expect("trusted theme schema"))
}

/// Checks one declared template's media metadata and local resource budgets.
fn check_template_metadata(manifest: &ThemeTemplateMetadata) -> Result<(), ThemeValidationError> {
    let mut paths = BTreeSet::new();
    let mut resources = 0;
    let mut fonts = 0;
    for file in &manifest.files {
        validate_theme_resource_path(&file.path)?;
        if !paths.insert(file.path.clone()) {
            return Err(integrity(&file.path, "duplicate template resource"));
        }
        let (media_type, _) = metadata_for_path(&file.path)?;
        if file.media_type != media_type {
            return Err(ThemeValidationError::contract(
                &file.path,
                "declared media type is invalid",
            ));
        }
        if file.path.starts_with("assets/") {
            resources += 1;
        }
        if file.path.starts_with("assets/fonts/") {
            fonts += 1;
        }
    }
    if resources > MAX_RESOURCES || fonts > MAX_FONTS {
        return Err(ThemeValidationError::contract(
            "themes/files",
            "template exceeds resource budget",
        ));
    }
    for required in ["theme.json", "preview.png"] {
        if !paths.contains(required) {
            return Err(integrity(required, "required template resource is missing"));
        }
    }
    Ok(())
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

/// Validates a persisted palette against the same complete color contract used by templates.
pub fn validate_palette(value: &Value) -> Result<(), ThemeValidationError> {
    let definition = theme_schema_v1();
    schema::validate(&definition, &definition["properties"]["palette"], value, "palette", true)
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

/// Seals the checked visual definition after verifying resource spellings.
fn finish_spec(value: Value) -> Result<ThemeSpecV1, ThemeValidationError> {
    let spec = ThemeSpecV1 { value };
    for path in spec.asset_paths() {
        validate_theme_resource_path(&path)?;
    }
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
                if index > 0 {
                    bytes.push(b',');
                }
                bytes.extend(serde_json::to_vec(key).expect("checked JSON key"));
                bytes.push(b':');
                write_canonical(child, bytes);
            }
            bytes.push(b'}');
        }
        Value::Array(array) => {
            bytes.push(b'[');
            for (index, child) in array.iter().enumerate() {
                if index > 0 {
                    bytes.push(b',');
                }
                write_canonical(child, bytes);
            }
            bytes.push(b']');
        }
        _ => bytes.extend(serde_json::to_vec(value).expect("checked JSON scalar")),
    }
}

/// Checks slice relationships that cannot be expressed by individual schema fields.
fn check_visual_relations(value: &Value) -> Result<(), ThemeValidationError> {
    for (base, image) in [
        ("/chat/user/image", &value["chat"]["user"]["image"]),
        ("/chat/ai/image", &value["chat"]["ai"]["image"]),
        ("/input/image", &value["input"]["image"]),
    ] {
        if image.is_null() {
            continue;
        }
        for axis in ["x", "y"] {
            let start = image["slice"][format!("{axis}Start")].as_f64().unwrap();
            let end = image["slice"][format!("{axis}End")].as_f64().unwrap();
            if end < start + 0.01 {
                return Err(ThemeValidationError::contract(
                    &format!("theme.json{base}/slice/{axis}End"),
                    "slice end must be at least start + 0.01",
                ));
            }
        }
    }
    Ok(())
}

/// Validates only the resources referenced by one theme template.
pub fn check_theme_template_files(
    files: &BTreeMap<String, Vec<u8>>,
    manifest: ThemeTemplateMetadata,
    manifest_bytes: &[u8],
) -> Result<ContractCheckedTheme, ThemeValidationError> {
    check_template_metadata(&manifest)?;
    let mut expanded = 0usize;
    for file in &manifest.files {
        let bytes = files
            .get(&file.path)
            .ok_or_else(|| integrity(&file.path, "declared template resource is missing"))?;
        let (_, limit) = metadata_for_path(&file.path)?;
        if bytes.is_empty() || bytes.len() > limit {
            return Err(integrity(&file.path, "resource is empty or exceeds the file size limit"));
        }
        expanded = expanded.checked_add(bytes.len())
            .ok_or_else(|| integrity(&file.path, "resource size overflow"))?;
        if expanded > MAX_EXPANDED_BYTES {
            return Err(integrity("themes/files", "template exceeds the total resource size limit"));
        }
        if file.media_type == "text/plain" {
            std::str::from_utf8(bytes).map_err(|_| integrity(&file.path, "text must be UTF-8"))?;
        }
    }
    let theme = parse_theme(&files["theme.json"])?;
    let declared_assets: BTreeSet<_> = manifest
        .files
        .iter()
        .filter(|file| file.path.starts_with("assets/"))
        .map(|file| file.path.clone())
        .collect();
    if theme.asset_paths() != declared_assets {
        return Err(integrity(
            "theme.json",
            "referenced assets must match the template resource declarations",
        ));
    }
    Ok(ContractCheckedTheme {
        manifest,
        theme,
        manifest_sha256: sha256_hex(manifest_bytes),
    })
}

/// Produces renderer evidence for one template without validating unrelated package capabilities.
fn validate_template(
    archive_bytes: &[u8],
    manifest: ThemeTemplateMetadata,
    manifest_bytes: &[u8],
    files: BTreeMap<String, Vec<u8>>,
) -> Result<ValidatedThemeTemplate, ThemeValidationError> {
    let checked = check_theme_template_files(&files, manifest, manifest_bytes)?;
    let media = media::validate_media_files(&files, &checked.manifest.files)?;
    media::validate_bubble_pixels(&checked.theme, &media)?;
    let expanded_bytes = files.values().map(Vec::len).sum();
    let receipt = ThemeValidationReceipt {
        validator_version: 1,
        theme_id: checked.manifest.theme_id.clone(),
        archive_sha256: sha256_hex(archive_bytes),
        archive_bytes: archive_bytes.len(),
        expanded_bytes,
        manifest_sha256: checked.manifest_sha256.clone(),
        theme_sha256: checked.theme.sha256(),
        files: checked
            .manifest
            .files
            .iter()
            .map(|file| ThemeFileValidation {
                path: file.path.clone(),
                bytes: files[&file.path].len(),
                sha256: sha256_hex(&files[&file.path]),
            })
            .collect(),
        media: media.clone(),
        trust: ThemeTemplateTrust::Untrusted,
    };
    Ok(ValidatedThemeTemplate {
        archive_sha256: receipt.archive_sha256.clone(),
        archive_bytes: archive_bytes.len(),
        expanded_bytes,
        manifest: checked.manifest,
        theme: checked.theme,
        files,
        media,
        receipt,
    })
}

/// Paths are rejected verbatim, not normalized, sanitized, or resolved on disk.
pub fn validate_theme_resource_path(path: &str) -> Result<(), ThemeValidationError> {
    if path == ".operit/market-install.seal" {
        return Ok(());
    }
    if path.is_empty() || path.len() > 128 || !path.is_ascii() || path.split('/').count() > 4 {
        return Err(integrity(path, "path exceeds v1 naming budget"));
    }
    for part in path.split('/') {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with('.')
            || !part.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-' || b == b'.'
            })
        {
            return Err(integrity(path, "invalid path component"));
        }
        let stem = part.split('.').next().unwrap();
        if [
            "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
            "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
        ]
        .contains(&stem)
        {
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
    if !pattern.is_match(path) {
        return Err(integrity(path, "unregistered path spelling"));
    }
    metadata_for_path(path)?;
    Ok(())
}

/// Returns the renderer media type and byte budget for a template-local resource.
fn metadata_for_path(path: &str) -> Result<(&'static str, usize), ThemeValidationError> {
    match path {
        "manifest.json" => Ok(("application/json", MAX_MANIFEST_BYTES)),
        ".operit/market-install.seal" => Ok(("application/octet-stream", MAX_MANIFEST_BYTES)),
        "theme.json" => Ok(("application/json", MAX_THEME_BYTES)),
        "preview.png" => Ok(("image/png", MAX_PREVIEW_BYTES)),
        _ if path.starts_with("assets/images/") && path.ends_with(".png") => {
            Ok(("image/png", MAX_IMAGE_BYTES))
        }
        _ if path.starts_with("assets/images/")
            && (path.ends_with(".jpg") || path.ends_with(".jpeg")) =>
        {
            Ok(("image/jpeg", MAX_IMAGE_BYTES))
        }
        _ if path.starts_with("assets/images/") && path.ends_with(".svg") => {
            Ok(("image/svg+xml", MAX_SVG_BYTES))
        }
        _ if path.starts_with("assets/fonts/") && path.ends_with(".ttf") => {
            Ok(("font/ttf", MAX_FONT_BYTES))
        }
        _ if path.starts_with("licenses/") && path.ends_with(".txt") => {
            Ok(("text/plain", MAX_LICENSE_BYTES))
        }
        _ => Err(integrity(path, "unregistered package path or file type")),
    }
}

/// Creates a resource integrity error with the offending template path.
fn integrity(path: &str, message: &str) -> ThemeValidationError {
    ThemeValidationError::new(
        "theme-package-integrity-invalid",
        ValidationStage::Integrity,
        path,
        message,
    )
}
