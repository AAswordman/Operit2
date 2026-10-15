//! In-memory authoring validation, without a ZIP or install authority.
//! Media goes through the exact L4 implementation used by package import.

use crate::{
    json, media, metadata_for_path, normalize_draft, sha256_hex, validate_theme_resource_path,
    ManifestFile, ThemeFileValidation, ThemeMediaValidation, ThemeTemplateTrust, ThemeSpecV1,
    ThemeValidationError, ValidationStage, MAX_EXPANDED_BYTES, MAX_FONTS, MAX_MANIFEST_BYTES,
    MAX_RESOURCES,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub struct ThemePreviewAsset {
    pub path: String,
    pub media_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemePreviewValidationReceipt {
    pub validator_version: u32,
    pub theme_sha256: String,
    pub files: Vec<ThemeFileValidation>,
    pub media: Vec<ThemeMediaValidation>,
    pub trust: ThemeTemplateTrust,
}

/// Private construction prevents a caller-provided hash from granting media
/// authority. This is neither a rendered receipt nor an installable package.
#[derive(Debug)]
pub struct ValidatedThemePreview {
    theme: ThemeSpecV1,
    receipt: ThemePreviewValidationReceipt,
    files: BTreeMap<String, Vec<u8>>,
}

impl ValidatedThemePreview {
    pub fn theme(&self) -> &ThemeSpecV1 { &self.theme }
    pub fn receipt(&self) -> &ThemePreviewValidationReceipt { &self.receipt }
    pub fn file_bytes(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(Vec::as_slice)
    }
}

pub fn validate_theme_preview_draft(
    theme_json: &[u8], assets: &[ThemePreviewAsset],
) -> Result<ValidatedThemePreview, ThemeValidationError> {
    let theme = normalize_draft(theme_json)?;
    if assets.len() > MAX_RESOURCES {
        return Err(preview_error("assets", "asset entry budget exceeded"));
    }
    let mut paths = BTreeSet::new();
    let mut total = 0usize;
    let mut fonts = 0usize;
    let mut declarations = Vec::new();
    // Check all sizes/identities before copying bytes or entering a decoder.
    for asset in assets {
        validate_theme_resource_path(&asset.path)?;
        let (media_type, limit) = metadata_for_path(&asset.path)?;
        if !asset.path.starts_with("assets/") || media_type != asset.media_type
            || asset.bytes.is_empty() || asset.bytes.len() > limit
            || !paths.insert(asset.path.clone())
        {
            return Err(preview_error(&asset.path, "asset type, size or identity is invalid"));
        }
        total = total.checked_add(asset.bytes.len())
            .ok_or_else(|| preview_error("assets", "asset byte count overflow"))?;
        if asset.path.starts_with("assets/fonts/") { fonts += 1; }
        if total > MAX_EXPANDED_BYTES || fonts > MAX_FONTS {
            return Err(preview_error("assets", "asset byte or font budget exceeded"));
        }
        declarations.push(ManifestFile {
            path: asset.path.clone(), media_type: asset.media_type.clone(),
        });
    }
    if paths != theme.asset_paths() {
        return Err(preview_error("assets", "asset bytes must exactly match normalized theme references"));
    }
    declarations.sort_by(|left, right| left.path.cmp(&right.path));
    let files = assets.iter().map(|asset| (asset.path.clone(), asset.bytes.as_slice()))
        .collect::<BTreeMap<_, _>>();
    let media = media::validate_media_files(&files, &declarations)?;
    media::validate_bubble_pixels(&theme, &media)?;
    let receipt = ThemePreviewValidationReceipt {
        validator_version: 1,
        theme_sha256: theme.sha256(),
        files: declarations.into_iter().map(|file| ThemeFileValidation {
            bytes: files[&file.path].len(), sha256: sha256_hex(files[&file.path]), path: file.path,
        }).collect(),
        media,
        trust: ThemeTemplateTrust::Untrusted,
    };
    // Seal after decoder work so validation's peak memory does not include
    // another full asset copy alongside the image decoder allocation.
    let sealed_files = assets.iter().map(|asset| (asset.path.clone(), asset.bytes.clone())).collect();
    Ok(ValidatedThemePreview { theme, receipt, files: sealed_files })
}

/// Browser transport carries a small, closed directory and one binary buffer.
/// No base64 expansion or JSON integer array of image/font bytes is needed.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AssetTransfer {
    path: String,
    media_type: String,
    offset: usize,
    bytes: usize,
}

pub(crate) fn parse_asset_transfer(
    directory_json: &[u8], payload: &[u8],
) -> Result<Vec<ThemePreviewAsset>, ThemeValidationError> {
    if payload.len() > MAX_EXPANDED_BYTES {
        return Err(preview_error("assets", "binary asset payload exceeds the byte budget"));
    }
    let directory: Vec<AssetTransfer> = serde_json::from_value(
        json::parse(directory_json, MAX_MANIFEST_BYTES, "assets")?,
    ).map_err(|error| ThemeValidationError::contract("assets", error.to_string()))?;
    if directory.len() > MAX_RESOURCES {
        return Err(preview_error("assets", "asset entry budget exceeded"));
    }
    let mut end = 0usize;
    let mut paths = BTreeSet::new();
    let mut fonts = 0;
    for entry in &directory {
        validate_theme_resource_path(&entry.path)?;
        let (media_type, limit) = metadata_for_path(&entry.path)?;
        if !entry.path.starts_with("assets/") || entry.media_type != media_type
            || !paths.insert(&entry.path) || entry.bytes == 0 || entry.bytes > limit
            || entry.offset != end
        {
            return Err(preview_error(&entry.path, "asset directory must be unique, typed and contiguous"));
        }
        if entry.path.starts_with("assets/fonts/") { fonts += 1; }
        end = end.checked_add(entry.bytes)
            .ok_or_else(|| preview_error(&entry.path, "asset range overflow"))?;
        if end > payload.len() {
            return Err(preview_error(&entry.path, "asset range exceeds the binary payload"));
        }
    }
    if end != payload.len() || fonts > MAX_FONTS {
        return Err(preview_error("assets", "binary payload has unreferenced bytes or too many fonts"));
    }
    Ok(directory.into_iter().map(|entry| ThemePreviewAsset {
        bytes: payload[entry.offset..entry.offset + entry.bytes].to_vec(),
        path: entry.path, media_type: entry.media_type,
    }).collect())
}

pub fn validate_theme_preview_transfer(
    theme_json: &[u8], directory_json: &[u8], payload: &[u8],
) -> Result<ValidatedThemePreview, ThemeValidationError> {
    // Fail on an invalid/oversized draft before duplicating the media payload.
    normalize_draft(theme_json)?;
    validate_theme_preview_draft(theme_json, &parse_asset_transfer(directory_json, payload)?)
}

fn preview_error(path: &str, message: &str) -> ThemeValidationError {
    ThemeValidationError::new("theme-preview-input-invalid", ValidationStage::Integrity, path, message)
}
