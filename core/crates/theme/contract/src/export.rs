//! Deterministic in-memory theme package export for Studio and trusted tooling.
//!
//! The exporter accepts declarative bytes only. It does not read paths, fetch
//! URLs, execute code, or decide whether a publisher is official. Its output
//! is immediately sent through the same package validator used by import.

use crate::{
    check_file_set, json, metadata_for_path, normalize_draft, preview::parse_asset_transfer,
    sha256_hex, validate_package_path, validate_theme_package, AssetLicense,
    CreatedWith, ManifestFile, ManifestV1, ThemeSpecV1, ThemeValidationError, ValidationStage,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeExportAsset {
    pub path: String,
    pub media_type: String,
    pub bytes: Vec<u8>,
    pub license: String,
    pub license_path: String,
    pub license_text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeExportDraft {
    pub theme_id: String,
    pub version: String,
    pub display_name: String,
    pub author_name: String,
    pub license: String,
    pub min_app_version: String,
    pub platforms: Vec<String>,
    pub theme_json: Vec<u8>,
    pub preview_png: Vec<u8>,
    pub theme_license_text: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub max_app_version: Option<String>,
    #[serde(default)]
    pub created_with: Option<CreatedWith>,
    #[serde(default)]
    pub assets: Vec<ThemeExportAsset>,
}

/// Exports one complete deterministic archive and validates that archive
/// through the import pipeline before returning it.
pub fn export_theme_package(draft: &ThemeExportDraft) -> Result<Vec<u8>, ThemeValidationError> {
    let theme = normalize_draft(&draft.theme_json)?;
    validate_export_metadata(draft)?;
    let files = build_export_files(draft, &theme)?;
    let manifest = build_manifest(draft, &files)?;
    let manifest_bytes =
        serde_json::to_vec(&manifest).map_err(|error| export_error("", error.to_string()))?;
    let mut inspected_files: BTreeMap<String, Vec<u8>> = files.clone();
    inspected_files.insert("manifest.json".into(), manifest_bytes.clone());
    // Validate metadata/licenses before allocating a ZIP or entering L4.
    check_file_set(&inspected_files, &draft.min_app_version)?;
    let total: usize = inspected_files.values().map(|bytes| bytes.len()).sum();
    if total > crate::MAX_ARCHIVE_BYTES {
        return Err(export_error("", "Stored export exceeds the archive byte budget"));
    }
    drop(inspected_files);

    let mut archive_bytes = Cursor::new(Vec::new());
    {
        let mut writer = ZipWriter::new(&mut archive_bytes);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(
                DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0)
                    .map_err(|error| export_error("", error.to_string()))?,
            )
            .unix_permissions(0o100644);
        write_entry(&mut writer, "manifest.json", &manifest_bytes, options)?;
        write_entry(&mut writer, "theme.json", &files["theme.json"], options)?;
        write_entry(&mut writer, "preview.png", &files["preview.png"], options)?;
        for path in files.keys().filter(|path| path.starts_with("assets/")) {
            write_entry(&mut writer, path, &files[path], options)?;
        }
        for path in files.keys().filter(|path| path.starts_with("licenses/")) {
            write_entry(&mut writer, path, &files[path], options)?;
        }
        writer
            .finish()
            .map_err(|error| export_error("", error.to_string()))?;
    }
    let archive = archive_bytes.into_inner();
    validate_theme_package(&archive, &draft.min_app_version)?;
    Ok(archive)
}

fn validate_export_metadata(draft: &ThemeExportDraft) -> Result<(), ThemeValidationError> {
    if draft.author_name.is_empty()
        || draft.display_name.is_empty()
        || draft.theme_license_text.is_empty()
        || draft.theme_license_text.len() > crate::MAX_LICENSE_BYTES
        || draft.platforms.len() != 7
        || draft.assets.len() > crate::MAX_RESOURCES
        || draft.preview_png.is_empty()
        || draft.preview_png.len() > crate::MAX_PREVIEW_BYTES
    {
        return Err(export_error(
            "",
            "export metadata is incomplete or exceeds the draft budget",
        ));
    }
    let mut bytes = draft.preview_png.len() + draft.theme_json.len() + draft.theme_license_text.len();
    let mut fonts = 0;
    for asset in &draft.assets {
        validate_package_path(&asset.path)?;
        validate_package_path(&asset.license_path)?;
        let (media_type, limit) = metadata_for_path(&asset.path)?;
        if !asset.path.starts_with("assets/") || asset.media_type != media_type
            || asset.bytes.len() > limit || asset.bytes.is_empty()
            || !asset.license_path.starts_with("licenses/")
            || asset.license_text.is_empty() || asset.license_text.len() > crate::MAX_LICENSE_BYTES
        {
            return Err(export_error(
                &asset.path,
                "asset bytes, media type or packaged license is invalid",
            ));
        }
        bytes = bytes.checked_add(asset.bytes.len())
            .and_then(|count| count.checked_add(asset.license_text.len()))
            .ok_or_else(|| export_error("", "export byte count overflow"))?;
        if asset.path.starts_with("assets/fonts/") { fonts += 1; }
    }
    if bytes > crate::MAX_EXPANDED_BYTES || fonts > crate::MAX_FONTS {
        return Err(export_error("", "export asset byte or font budget exceeded"));
    }
    Ok(())
}

fn build_export_files(
    draft: &ThemeExportDraft,
    theme: &ThemeSpecV1,
) -> Result<BTreeMap<String, Vec<u8>>, ThemeValidationError> {
    let theme_bytes = theme.canonical_bytes();
    let mut files = BTreeMap::from([
        ("theme.json".to_string(), theme_bytes),
        ("preview.png".to_string(), draft.preview_png.clone()),
        (
            "licenses/theme.txt".to_string(),
            draft.theme_license_text.as_bytes().to_vec(),
        ),
    ]);
    let referenced = theme.asset_paths();
    let mut assets = BTreeSet::new();
    let mut licenses = BTreeMap::<String, Vec<u8>>::new();
    for asset in &draft.assets {
        let (expected_media_type, _) = metadata_for_path(&asset.path)?;
        if expected_media_type != asset.media_type || !assets.insert(asset.path.clone()) {
            return Err(export_error(
                &asset.path,
                "asset path or media type is duplicated or invalid",
            ));
        }
        if !referenced.contains(&asset.path) {
            return Err(export_error(
                &asset.path,
                "export contains an asset not referenced by theme.json",
            ));
        }
        files.insert(asset.path.clone(), asset.bytes.clone());
        let license_bytes = asset.license_text.as_bytes().to_vec();
        if let Some(existing) = files.get(&asset.license_path) {
            if existing != &license_bytes {
                return Err(export_error(
                    &asset.license_path,
                    "asset license path conflicts with an existing packaged license",
                ));
            }
        }
        match licenses.get(&asset.license_path) {
            Some(existing) if existing != &license_bytes => {
                return Err(export_error(
                    &asset.license_path,
                    "one license path has conflicting text",
                ));
            }
            _ => {
                licenses.insert(asset.license_path.clone(), license_bytes);
            }
        }
    }
    if assets != referenced {
        return Err(export_error(
            "theme.json",
            "every referenced asset must be supplied exactly once",
        ));
    }
    for (path, bytes) in licenses {
        files.insert(path, bytes);
    }
    Ok(files)
}

fn build_manifest(
    draft: &ThemeExportDraft,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<ManifestV1, ThemeValidationError> {
    let mut manifest_files = Vec::new();
    for (path, bytes) in files {
        let (media_type, _) = metadata_for_path(path)?;
        manifest_files.push(ManifestFile {
            path: path.clone(),
            media_type: media_type.to_string(),
            bytes: bytes.len(),
            sha256: sha256_hex(bytes),
        });
    }
    let mut asset_licenses = Vec::new();
    for asset in &draft.assets {
        asset_licenses.push(AssetLicense {
            asset: asset.path.clone(),
            license: asset.license.clone(),
            text: asset.license_path.clone(),
        });
    }
    asset_licenses.sort_by(|left, right| left.asset.cmp(&right.asset));
    let mut platforms = draft.platforms.clone();
    platforms.sort();
    Ok(ManifestV1 {
        format: "operit2.theme".to_string(),
        format_version: 1,
        theme_api_version: 1,
        theme_id: draft.theme_id.clone(),
        version: draft.version.clone(),
        display_name: draft.display_name.clone(),
        author: crate::Author {
            name: draft.author_name.clone(),
        },
        license: draft.license.clone(),
        min_app_version: draft.min_app_version.clone(),
        platforms,
        files: manifest_files,
        description: draft.description.clone(),
        max_app_version: draft.max_app_version.clone(),
        created_with: draft.created_with.clone(),
        asset_licenses,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExportMetadata {
    theme_id: String,
    version: String,
    display_name: String,
    author_name: String,
    license: String,
    min_app_version: String,
    platforms: Vec<String>,
    theme_license_text: String,
    description: Option<String>,
    max_app_version: Option<String>,
    created_with: Option<CreatedWith>,
    asset_licenses: Vec<ExportLicense>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExportLicense {
    path: String,
    license: String,
    license_path: String,
    license_text: String,
}

/// The browser sends bounded JSON metadata separately from binary media. It
/// never serializes an entire font as a JSON integer array to pass the ABI.
pub fn export_theme_package_from_transfer(
    metadata_json: &[u8], theme_json: &[u8], preview_png: &[u8],
    directory_json: &[u8], payload: &[u8],
) -> Result<Vec<u8>, ThemeValidationError> {
    normalize_draft(theme_json)?;
    if preview_png.is_empty() || preview_png.len() > crate::MAX_PREVIEW_BYTES {
        return Err(export_error("preview.png", "preview byte budget exceeded"));
    }
    let metadata: ExportMetadata = serde_json::from_value(json::parse(
        metadata_json,
        crate::MAX_MANIFEST_BYTES + (crate::MAX_RESOURCES + 1) * crate::MAX_LICENSE_BYTES,
        "exportMetadata",
    )?).map_err(|error| ThemeValidationError::contract("exportMetadata", error.to_string()))?;
    let assets = parse_asset_transfer(directory_json, payload)?;
    let mut licenses = BTreeMap::new();
    for license in metadata.asset_licenses {
        if licenses.insert(license.path.clone(), license).is_some() {
            return Err(export_error("assetLicenses", "duplicate asset license"));
        }
    }
    let mut export_assets = Vec::new();
    for asset in assets {
        let license = licenses.remove(&asset.path)
            .ok_or_else(|| export_error(&asset.path, "asset license is missing"))?;
        export_assets.push(ThemeExportAsset {
            path: asset.path, media_type: asset.media_type, bytes: asset.bytes,
            license: license.license, license_path: license.license_path,
            license_text: license.license_text,
        });
    }
    if !licenses.is_empty() {
        return Err(export_error("assetLicenses", "unreferenced asset license"));
    }
    export_theme_package(&ThemeExportDraft {
        theme_id: metadata.theme_id, version: metadata.version,
        display_name: metadata.display_name, author_name: metadata.author_name,
        license: metadata.license, min_app_version: metadata.min_app_version,
        platforms: metadata.platforms, theme_json: theme_json.to_vec(),
        preview_png: preview_png.to_vec(), theme_license_text: metadata.theme_license_text,
        description: metadata.description, max_app_version: metadata.max_app_version,
        created_with: metadata.created_with, assets: export_assets,
    })
}

fn write_entry(
    writer: &mut ZipWriter<&mut Cursor<Vec<u8>>>,
    path: &str,
    bytes: &[u8],
    options: SimpleFileOptions,
) -> Result<(), ThemeValidationError> {
    writer
        .start_file(path, options)
        .map_err(|error| export_error(path, error.to_string()))?;
    writer
        .write_all(bytes)
        .map_err(|error| export_error(path, error.to_string()))
}

fn export_error(path: &str, message: impl Into<String>) -> ThemeValidationError {
    ThemeValidationError::new(
        "theme-export-invalid",
        ValidationStage::Integrity,
        path,
        message,
    )
}
