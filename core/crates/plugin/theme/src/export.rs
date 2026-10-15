//! Deterministic in-memory ToolPkg export with a theme template for Studio and trusted tooling.
//!
//! The exporter accepts declarative bytes only. It does not read paths, fetch
//! URLs, execute code, or decide whether a publisher is official. Its output
//! is immediately checked against its declared template resources.

use crate::{
    check_theme_template_files, json, metadata_for_path, normalize_draft,
    preview::parse_asset_transfer, validate_theme_resource_path, ManifestFile,
    ThemeSpecV1, ThemeTemplateMetadata, ThemeValidationError, ValidationStage,
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeExportDraft {
    pub toolpkg_id: String,
    pub version: String,
    pub display_name: String,
    pub author_name: String,
    pub theme_json: Vec<u8>,
    pub preview_png: Vec<u8>,
    pub agreement_text: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub assets: Vec<ThemeExportAsset>,
}

/// Exports one complete deterministic archive and validates that archive
/// against its declared template before returning it.
pub fn export_toolpkg_from_theme(
    draft: &ThemeExportDraft,
) -> Result<Vec<u8>, ThemeValidationError> {
    let theme = normalize_draft(&draft.theme_json)?;
    validate_export_metadata(draft)?;
    let files = build_export_files(draft, &theme)?;
    let manifest = build_manifest(draft, &files)?;
    let manifest_bytes = serde_json::to_vec(&crate::toolpkg_manifest_value(&manifest))
        .map_err(|error| export_error("", error.to_string()))?;
    let mut inspected_files: BTreeMap<String, Vec<u8>> = files.clone();
    inspected_files.insert("manifest.json".into(), manifest_bytes.clone());
    // Validate metadata/licenses before allocating a ZIP or entering L4.
    check_theme_template_files(&files, manifest.clone(), &manifest_bytes)?;
    let total: usize = inspected_files.values().map(|bytes| bytes.len()).sum();
    if total > crate::MAX_ARCHIVE_BYTES {
        return Err(export_error(
            "",
            "Stored export exceeds the archive byte budget",
        ));
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
    crate::validate_toolpkg_theme_templates(&archive)?;
    Ok(archive)
}

fn validate_export_metadata(draft: &ThemeExportDraft) -> Result<(), ThemeValidationError> {
    if draft.toolpkg_id.trim().is_empty()
        || draft.agreement_text.len() > crate::MAX_LICENSE_BYTES
        || draft.assets.len() > crate::MAX_RESOURCES
        || draft.preview_png.is_empty()
        || draft.preview_png.len() > crate::MAX_PREVIEW_BYTES
    {
        return Err(export_error(
            "",
            "export metadata is incomplete or exceeds the draft budget",
        ));
    }
    let mut bytes = draft.preview_png.len() + draft.theme_json.len() + draft.agreement_text.len();
    let mut fonts = 0;
    for asset in &draft.assets {
        validate_theme_resource_path(&asset.path)?;
        let (media_type, limit) = metadata_for_path(&asset.path)?;
        if !asset.path.starts_with("assets/")
            || asset.media_type != media_type
            || asset.bytes.len() > limit
            || asset.bytes.is_empty()
        {
            return Err(export_error(
                &asset.path,
                "asset bytes or media type is invalid",
            ));
        }
        bytes = bytes
            .checked_add(asset.bytes.len())
            .ok_or_else(|| export_error("", "export byte count overflow"))?;
        if asset.path.starts_with("assets/fonts/") {
            fonts += 1;
        }
    }
    if bytes > crate::MAX_EXPANDED_BYTES || fonts > crate::MAX_FONTS {
        return Err(export_error(
            "",
            "export asset byte or font budget exceeded",
        ));
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
    ]);
    if !draft.agreement_text.is_empty() {
        files.insert(
            "licenses/agreement.txt".into(),
            draft.agreement_text.as_bytes().to_vec(),
        );
    }
    let referenced = theme.asset_paths();
    let mut assets = BTreeSet::new();
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
    }
    if assets != referenced {
        return Err(export_error(
            "theme.json",
            "every referenced asset must be supplied exactly once",
        ));
    }
    Ok(files)
}

fn build_manifest(
    draft: &ThemeExportDraft,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<ThemeTemplateMetadata, ThemeValidationError> {
    let mut manifest_files = Vec::new();
    for path in files.keys() {
        if path.starts_with("licenses/") {
            continue;
        }
        let (media_type, _) = metadata_for_path(path)?;
        manifest_files.push(ManifestFile {
            path: path.clone(),
            media_type: media_type.to_string(),
        });
    }
    Ok(ThemeTemplateMetadata {
        theme_id: format!("{}:default", draft.toolpkg_id),
        package_id: draft.toolpkg_id.clone(),
        template_id: "default".into(),
        path: "theme.json".into(),
        preview: "preview.png".into(),
        version: draft.version.clone(),
        display_name: draft.display_name.clone(),
        author: crate::Author {
            name: draft.author_name.clone(),
        },
        license: if draft.agreement_text.is_empty() {
            None
        } else {
            Some("licenses/agreement.txt".into())
        },
        files: manifest_files,
        description: draft.description.clone(),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExportMetadata {
    toolpkg_id: String,
    version: String,
    display_name: String,
    author_name: String,
    agreement_text: String,
    description: Option<String>,
}

/// The browser sends bounded JSON metadata separately from binary media. It
/// never serializes an entire font as a JSON integer array to pass the ABI.
pub fn export_toolpkg_from_theme_transfer(
    metadata_json: &[u8],
    theme_json: &[u8],
    preview_png: &[u8],
    directory_json: &[u8],
    payload: &[u8],
) -> Result<Vec<u8>, ThemeValidationError> {
    normalize_draft(theme_json)?;
    if preview_png.is_empty() || preview_png.len() > crate::MAX_PREVIEW_BYTES {
        return Err(export_error("preview.png", "preview byte budget exceeded"));
    }
    let metadata: ExportMetadata = serde_json::from_value(json::parse(
        metadata_json,
        crate::MAX_MANIFEST_BYTES + (crate::MAX_RESOURCES + 1) * crate::MAX_LICENSE_BYTES,
        "exportMetadata",
    )?)
    .map_err(|error| ThemeValidationError::contract("exportMetadata", error.to_string()))?;
    let assets = parse_asset_transfer(directory_json, payload)?;
    let export_assets = assets
        .into_iter()
        .map(|asset| ThemeExportAsset {
            path: asset.path,
            media_type: asset.media_type,
            bytes: asset.bytes,
        })
        .collect();
    export_toolpkg_from_theme(&ThemeExportDraft {
        toolpkg_id: metadata.toolpkg_id,
        version: metadata.version,
        display_name: metadata.display_name,
        author_name: metadata.author_name,
        theme_json: theme_json.to_vec(),
        preview_png: preview_png.to_vec(),
        agreement_text: metadata.agreement_text,
        description: metadata.description,
        assets: export_assets,
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
