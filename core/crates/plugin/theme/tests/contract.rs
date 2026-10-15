use operit_plugin_theme::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

const FIXTURE: &[u8] =
    include_bytes!("../../../../../apps/flutter/packages/operit_ui/assets/fixtures/theme-v1.json");

#[test]
fn defaults_equal_renderer_fixture() {
    let draft = normalize_draft(br#"{}"#).unwrap();
    let fixture = parse_theme(FIXTURE).unwrap();
    assert_eq!(draft.canonical_bytes(), fixture.canonical_bytes());
    assert_eq!(draft.sha256(), fixture.sha256());
}

#[test]
fn rejects_duplicates_unknown_fields_and_user_controls() {
    for bad in [
        r#"{"background":{},"background":{}}"#,
        r#"{"chat":{"style":"bubble","style":"cursor"}}"#,
        r#"{"showThinkingProcess":false}"#,
        r#"{"fontScale":2}"#,
        r#"{"chat":{"selector":"body"}}"#,
        r#"{"themeApiVersion":1}"#,
        r#"{"themeApiVersion":2}"#,
    ] {
        assert!(normalize_draft(bad.as_bytes()).is_err(), "{bad}");
    }
}

#[test]
fn packages_are_full_but_drafts_can_omit_versioned_defaults() {
    assert!(parse_theme(br#"{}"#).is_err());
    assert!(normalize_draft(br#"{}"#).is_ok());
    assert!(normalize_draft(br#"{}"#).is_ok());
    assert!(normalize_draft(br#"{"background":{"opacity":2}}"#).is_err());
}

#[test]
fn rejects_remote_paths_and_invalid_slicing_instead_of_clamping() {
    for path in [
        "https://example.test/bg.png",
        "../bg.png",
        "C:/bg.png",
        "runtime/data/bg.png",
    ] {
        let bad = json!({"background":{"asset":path}});
        assert!(normalize_draft(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    let bad = json!({"chat":{"user":{"image":{
        "asset":"assets/images/bubble.png","slice":{"xStart":0.8,"xEnd":0.4}
    }}}});
    let error = normalize_draft(&serde_json::to_vec(&bad).unwrap()).unwrap_err();
    assert!(error.path.ends_with("/slice/xEnd"));
}

#[test]
fn rejects_path_aliases_traversal_and_executable_content() {
    for path in [
        "../theme.json",
        "assets/images/../bg.png",
        "/preview.png",
        "assets\\images\\x.png",
        "assets/images/BG.png",
        "assets/images/con.png",
        "assets/images/x.png.",
        "assets/images/x.png:stream",
        "assets//images/x.png",
        "theme.css",
        "script.js",
        "assets/images/x.zip",
    ] {
        assert!(validate_theme_resource_path(path).is_err(), "{path}");
    }
    assert!(validate_theme_resource_path("assets/images/wallpaper.png").is_ok());
    assert!(validate_theme_resource_path("assets/images/wallpaper.svg").is_ok());
}

fn files() -> BTreeMap<String, Vec<u8>> {
    // Deliberately not a decoded PNG. This demonstrates that an L3 receipt is
    // not a media-validation or installation capability.
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::from([
        ("theme.json".into(), FIXTURE.to_vec()),
        ("preview.png".into(), b"not a decoded image".to_vec()),
        ("licenses/theme.txt".into(), b"Example license".to_vec()),
    ]);
    let entries: Vec<Value> = files
        .keys()
        .map(|path| {
            json!({
                "path":path,"mediaType":match path.as_str() {
                    "theme.json"=>"application/json","preview.png"=>"image/png",_=>"text/plain"
                }
            })
        })
        .collect();
    let manifest = json!({"schema_version":1,"api_version":"2.0.0",
        "toolpkg_id":"dev.operit.example","version":"1.0.0","display_name":"Example",
        "author":["Example Author"],"license":"licenses/theme.txt",
        "themes":[{"id":"default","path":"theme.json","preview":"preview.png","files":entries}]});
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    files
}

/// Constructs contribution metadata for focused resource integrity tests.
fn check_files(
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<ContractCheckedTheme, ThemeValidationError> {
    let value: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    let metadata = ThemeTemplateMetadata {
        theme_id: "dev.operit.example:default".into(),
        package_id: "dev.operit.example".into(),
        template_id: "default".into(),
        path: "theme.json".into(),
        preview: "preview.png".into(),
        version: value["version"].as_str().unwrap().into(),
        display_name: "Example".into(),
        author: Author {
            name: "Author".into(),
        },
        license: Some("licenses/theme.txt".into()),
        files: serde_json::from_value(value["themes"][0]["files"].clone()).unwrap(),
        description: None,
    };
    check_theme_template_files(files, metadata, &files["manifest.json"])
}

/// Checks declared resources without imposing an exact file set on the ToolPkg.
#[test]
fn checks_resource_presence_without_author_supplied_sizes_or_hashes() {
    let original = files();
    let checked = check_files(&original).unwrap();
    assert_eq!(
        checked.theme().sha256(),
        parse_theme(FIXTURE).unwrap().sha256()
    );
    let mut extra = original.clone();
    extra.insert("scripts/main.js".into(), b"export default {}".to_vec());
    assert!(check_files(&extra).is_ok());
    let mut tampered = original.clone();
    tampered.get_mut("preview.png").unwrap().push(0);
    assert!(check_files(&tampered).is_ok());
    let mut missing = original;
    missing.remove("preview.png");
    assert!(check_files(&missing).is_err());
}

/// Keeps agreement, version and compatibility metadata out of the template declaration.
#[test]
fn themes_use_only_toolpkg_metadata_and_license_reference() {
    let fixture = complete_files(png_preview());
    let template = validate_toolpkg_theme_template(
        &archive_from_files(&fixture),
        "dev.operit.example:default",
    )
    .unwrap();
    assert_eq!(
        template.metadata().license.as_deref(),
        Some("licenses/theme.txt")
    );
    let value: Value = serde_json::from_slice(&fixture["manifest.json"]).unwrap();
    for field in [
        "platforms",
        "minAppVersion",
        "maxAppVersion",
        "license",
        "themeApiVersion",
        "version",
    ] {
        assert!(value["themes"][0].get(field).is_none(), "{field}");
    }
}

#[test]
fn rejects_deep_trailing_and_oversized_json() {
    let deep = format!("{}0{}", "[".repeat(34), "]".repeat(34));
    assert!(normalize_draft(deep.as_bytes()).is_err());
    assert!(normalize_draft(br#"{} false"#).is_err());
    let bytes = vec![b' '; MAX_THEME_BYTES + 1];
    assert_eq!(
        normalize_draft(&bytes).unwrap_err().code,
        "theme-input-budget-exceeded"
    );
}

fn zip_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut output = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut output);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        for (name, bytes) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }
    output.into_inner()
}

/// zip 2.x deliberately refuses duplicate names while writing. Build one
/// valid entry, then duplicate its raw local/central records so the archive
/// inspector receives the hostile input that it must reject.
fn zip_with_duplicate(name: &str, first: &[u8], second: &[u8]) -> Vec<u8> {
    let original = zip_with(&[(name, first)]);
    let eocd = original.len() - 22;
    let central_start =
        u32::from_le_bytes(original[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    let central_size =
        u32::from_le_bytes(original[eocd + 12..eocd + 16].try_into().unwrap()) as usize;
    let local = &original[..central_start];
    let central = &original[central_start..eocd];
    assert_eq!(central.len(), central_size);
    let mut duplicate_local = zip_with(&[(name, second)]);
    let duplicate_eocd = duplicate_local.len() - 22;
    let duplicate_central_start = u32::from_le_bytes(
        duplicate_local[duplicate_eocd + 16..duplicate_eocd + 20]
            .try_into()
            .unwrap(),
    ) as usize;
    duplicate_local.truncate(duplicate_central_start);
    let duplicate_central = &original[central_start..eocd];
    let mut duplicate_central = duplicate_central.to_vec();
    duplicate_central[42..46].copy_from_slice(&(central_start as u32).to_le_bytes());
    let mut result = Vec::new();
    result.extend_from_slice(local);
    result.extend_from_slice(&duplicate_local);
    let new_central_start = result.len();
    result.extend_from_slice(central);
    result.extend_from_slice(&duplicate_central);
    let new_central_size = central_size * 2;
    let mut new_eocd = original[eocd..].to_vec();
    new_eocd[8..10].copy_from_slice(&2u16.to_le_bytes());
    new_eocd[10..12].copy_from_slice(&2u16.to_le_bytes());
    new_eocd[12..16].copy_from_slice(&(new_central_size as u32).to_le_bytes());
    new_eocd[16..20].copy_from_slice(&(new_central_start as u32).to_le_bytes());
    result.extend_from_slice(&new_eocd);
    result
}

fn png_preview() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(1280, 800, image::Rgba([18, 24, 32, 255]));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}

fn complete_files(preview: Vec<u8>) -> BTreeMap<String, Vec<u8>> {
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::from([
        ("theme.json".into(), FIXTURE.to_vec()),
        ("preview.png".into(), preview),
        ("licenses/theme.txt".into(), b"Example license".to_vec()),
    ]);
    let entries: Vec<Value> = files
        .keys()
        .map(|path| {
            json!({
                "path":path,
                "mediaType":match path.as_str() {
                    "theme.json"=>"application/json",
                    "preview.png"=>"image/png",
                    _=>"text/plain"
                }
            })
        })
        .collect();
    let manifest = json!({"schema_version":1,"api_version":"2.0.0",

        "toolpkg_id":"dev.operit.example",
        "version":"1.0.0",
        "display_name":"Example",
        "author":["Example Author"],
        "license":"licenses/theme.txt",
        "themes":[{"id":"default","path":"theme.json","preview":"preview.png","files":entries}]
    });
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    files
}

fn archive_from_files(files: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut output = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut output);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        for (path, bytes) in files {
            writer.start_file(path, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }
    output.into_inner()
}

#[test]
fn complete_package_runs_l4_and_returns_non_installing_receipt() {
    let files = complete_files(png_preview());
    let archive = archive_from_files(&files);
    let package = validate_toolpkg_theme_template(&archive, "dev.operit.example:default").unwrap();
    assert_eq!(package.archive_sha256(), sha256_hex(&archive));
    assert_eq!(package.metadata().package_id, "dev.operit.example");
    assert_eq!(package.receipt().trust, ThemeTemplateTrust::Untrusted);
    assert_eq!(
        package.file_bytes("preview.png").unwrap().len(),
        files["preview.png"].len()
    );
}

#[test]
fn complete_package_rejects_invalid_media() {
    let files = complete_files(b"not a PNG".to_vec());
    let archive = archive_from_files(&files);
    let error =
        validate_toolpkg_theme_template(&archive, "dev.operit.example:default").unwrap_err();
    assert_eq!(error.code, "theme-media-invalid");
    assert_eq!(error.stage, ValidationStage::Media);
}

#[test]
fn exporter_is_deterministic_and_exports_a_package_that_import_accepts() {
    let draft = ThemeExportDraft {
        toolpkg_id: "dev.operit.example".into(),
        version: "1.0.0".into(),
        display_name: "Example".into(),
        author_name: "Example Author".into(),
        theme_json: FIXTURE.to_vec(),
        preview_png: png_preview(),
        agreement_text: "Example license".into(),
        description: None,

        assets: Vec::new(),
    };
    let first = export_toolpkg_from_theme(&draft).unwrap();
    let second = export_toolpkg_from_theme(&draft).unwrap();
    assert_eq!(first, second);
    let package = validate_toolpkg_theme_template(&first, "dev.operit.example:default").unwrap();
    assert_eq!(package.metadata().package_id, draft.toolpkg_id);
}

/// Allows executable resources and multiple distinct templates in the same ordinary ToolPkg.
#[test]
fn mixed_toolpkg_contributes_multiple_templates() {
    let mut files = complete_files(png_preview());
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["main"] = json!("scripts/main.js");
    manifest["version"] = json!("1");
    files.insert("scripts/main.js".into(), b"export default {}".to_vec());
    files.insert(
        "templates/dark/definition.json".into(),
        files["theme.json"].clone(),
    );
    files.insert("screenshots/dark.png".into(), files["preview.png"].clone());
    let mut dark = manifest["themes"][0].clone();
    dark["id"] = json!("dark");
    dark["path"] = json!("templates/dark/definition.json");
    dark["preview"] = json!("screenshots/dark.png");
    dark["files"]
        .as_array_mut()
        .unwrap()
        .retain(|file| file["mediaType"] != "text/plain");
    manifest["themes"].as_array_mut().unwrap().push(dark);
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    let bytes = archive_from_files(&files);
    let templates = validate_toolpkg_theme_templates(&bytes).unwrap();
    assert_eq!(templates.len(), 2);
    assert_eq!(templates[1].metadata().theme_id, "dev.operit.example:dark");
    assert_eq!(templates[1].metadata().version, "1");
    assert!(templates[1].file_bytes("scripts/main.js").is_none());
    assert!(validate_toolpkg_theme_template(&bytes, "dev.operit.example:missing").is_err());
}

/// Empty contribution lists are ordinary ToolPkgs and do not need theme resources.
#[test]
fn toolpkg_without_theme_declarations_has_no_theme_templates() {
    let archive = zip_with(&[
        (
            "manifest.json",
            br#"{"toolpkg_id":"dev.operit.tools","main":"main.js"}"#,
        ),
        ("main.js", b"export default {}"),
    ]);
    assert!(validate_toolpkg_theme_templates(&archive)
        .unwrap()
        .is_empty());
}

/// Package agreement references are optional and never template-local requirements.
#[test]
fn template_does_not_require_package_agreement() {
    let mut files = complete_files(png_preview());
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest.as_object_mut().unwrap().remove("license");
    manifest["themes"][0]["files"]
        .as_array_mut()
        .unwrap()
        .retain(|file| file["mediaType"] != "text/plain");
    files.remove("licenses/theme.txt");
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    let templates = validate_toolpkg_theme_templates(&archive_from_files(&files)).unwrap();
    assert!(templates[0].metadata().license.is_none());
}

/// Rejects traversal in the declared definition independently of unrelated archive contents.
#[test]
fn rejects_unsafe_template_references() {
    let mut files = complete_files(png_preview());
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["themes"][0]["path"] = json!("../theme.json");
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    assert!(validate_toolpkg_theme_templates(&archive_from_files(&files)).is_err());
}

/// Validates shipped templates against their actual resources, including declared lengths and hashes.
#[test]
fn bundled_external_theme_templates_load_without_sizes_or_hashes() {
    let external = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../plugins/packages/external");
    let mut checked = 0;
    for entry in std::fs::read_dir(external).unwrap() {
        let package = entry.unwrap().path();
        let manifest = package.join("manifest.json");
        if !manifest.is_file() {
            continue;
        }
        let bytes = std::fs::read(&manifest).unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        let Some(themes) = value["themes"].as_array() else {
            continue;
        };
        let mut files = BTreeMap::from([("manifest.json".into(), bytes)]);
        for theme in themes {
            let theme_path = std::path::Path::new(theme["path"].as_str().unwrap());
            for resource in theme["files"].as_array().unwrap() {
                let name = resource["path"].as_str().unwrap();
                let path = match name {
                    "theme.json" => theme_path.to_path_buf(),
                    "preview.png" => theme["preview"].as_str().unwrap().into(),
                    _ => theme_path.parent().unwrap().join(name),
                };
                files.insert(
                    path.to_str().unwrap().to_string(),
                    std::fs::read(package.join(&path)).unwrap(),
                );
            }
        }
        let actual = validate_toolpkg_theme_templates(&archive_from_files(&files))
            .unwrap_or_else(|error| panic!("{}: {error}", manifest.display()));
        assert_eq!(actual.len(), themes.len());
        checked += actual.len();
    }
    assert!(checked >= 2, "shipped theme templates must be checked");
}

/// Old, stale bookkeeping fields must not prevent loading an otherwise valid theme.
#[test]
fn ignores_legacy_resource_sizes_and_hashes_and_computes_receipts_from_files() {
    let mut files = complete_files(png_preview());
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    for file in manifest["themes"][0]["files"].as_array_mut().unwrap() {
        file["bytes"] = json!(1);
        file["sha256"] = json!("obsolete digest");
    }
    files.insert("manifest.json".into(), serde_json::to_vec(&manifest).unwrap());
    let templates = validate_toolpkg_theme_templates(&archive_from_files(&files)).unwrap();
    for receipt in &templates[0].receipt().files {
        assert_eq!(receipt.bytes, files[&receipt.path].len());
        assert_eq!(receipt.sha256, sha256_hex(&files[&receipt.path]));
    }
    let serialized = serde_json::to_value(&templates[0].metadata().files).unwrap();
    for file in serialized.as_array().unwrap() {
        assert_eq!(file.as_object().unwrap().len(), 2);
        assert!(file.get("bytes").is_none());
        assert!(file.get("sha256").is_none());
    }
}

/// Limits apply to actual resources even without author-supplied bookkeeping.
#[test]
fn rejects_empty_and_oversized_actual_resources() {
    let mut resources = files();
    resources.insert("preview.png".into(), Vec::new());
    assert!(check_files(&resources).is_err());
    resources.insert("preview.png".into(), vec![0; MAX_IMAGE_BYTES + 1]);
    assert!(check_files(&resources).is_err());
}
