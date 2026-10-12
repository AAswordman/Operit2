use operit_theme_contract::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

const FIXTURE: &[u8] = include_bytes!("../../../../../apps/flutter/packages/operit_theme/assets/fixtures/theme-v1.json");

#[test]
fn defaults_equal_renderer_fixture() {
    let draft = normalize_draft(br#"{"themeApiVersion":1}"#).unwrap();
    let fixture = parse_theme(FIXTURE).unwrap();
    assert_eq!(draft.canonical_bytes(), fixture.canonical_bytes());
    assert_eq!(draft.sha256(), fixture.sha256());
}

#[test]
fn rejects_duplicates_unknown_fields_and_user_controls() {
    for bad in [
        r#"{"themeApiVersion":1,"themeApiVersion":1}"#,
        r#"{"themeApiVersion":1,"chat":{"style":"bubble","style":"cursor"}}"#,
        r#"{"themeApiVersion":1,"showThinkingProcess":false}"#,
        r#"{"themeApiVersion":1,"fontScale":2}"#,
        r#"{"themeApiVersion":1,"chat":{"selector":"body"}}"#,
        r#"{"themeApiVersion":1.0}"#,
        r#"{"themeApiVersion":2}"#,
    ] { assert!(normalize_draft(bad.as_bytes()).is_err(), "{bad}"); }
}

#[test]
fn packages_are_full_but_drafts_can_omit_versioned_defaults() {
    assert!(parse_theme(br#"{"themeApiVersion":1}"#).is_err());
    assert!(normalize_draft(br#"{"themeApiVersion":1}"#).is_ok());
    assert!(normalize_draft(br#"{}"#).is_err());
    assert!(normalize_draft(br#"{"themeApiVersion":1,"background":{"opacity":2}}"#).is_err());
}

#[test]
fn rejects_remote_paths_and_invalid_slicing_instead_of_clamping() {
    for path in ["https://example.test/bg.png", "../bg.png", "C:/bg.png", "runtime/data/bg.png"] {
        let bad = json!({"themeApiVersion":1,"background":{"asset":path}});
        assert!(normalize_draft(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    let bad = json!({"themeApiVersion":1,"chat":{"user":{"image":{
        "asset":"assets/images/bubble.png","slice":{"xStart":0.8,"xEnd":0.4}
    }}}});
    let error = normalize_draft(&serde_json::to_vec(&bad).unwrap()).unwrap_err();
    assert!(error.path.ends_with("/slice/xEnd"));
}

#[test]
fn rejects_path_aliases_traversal_and_executable_content() {
    for path in ["../theme.json", "assets/images/../bg.png", "/preview.png", "assets\\images\\x.png",
        "assets/images/BG.png", "assets/images/con.png", "assets/images/x.png.", "assets/images/x.png:stream",
        "assets//images/x.png", "theme.css", "script.js", "assets/images/x.zip"] {
        assert!(validate_package_path(path).is_err(), "{path}");
    }
    assert!(validate_package_path("assets/images/wallpaper.png").is_ok());
    assert!(validate_package_path("assets/images/wallpaper.svg").is_ok());
}

fn files() -> BTreeMap<String, Vec<u8>> {
    // Deliberately not a decoded PNG. This demonstrates that an L3 receipt is
    // not a media-validation or installation capability.
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::from([
        ("theme.json".into(), FIXTURE.to_vec()),
        ("preview.png".into(), b"not a decoded image".to_vec()),
        ("licenses/theme.txt".into(), b"Example license".to_vec()),
    ]);
    let entries: Vec<Value> = files.iter().map(|(path, bytes)| json!({
        "path":path,"mediaType":match path.as_str() {
            "theme.json"=>"application/json","preview.png"=>"image/png",_=>"text/plain"
        },"bytes":bytes.len(),"sha256":sha256_hex(bytes)
    })).collect();
    let manifest = json!({"format":"operit2.theme","formatVersion":1,"themeApiVersion":1,
        "themeId":"dev.operit.example","version":"1.0.0","displayName":"Example",
        "author":{"name":"Example Author"},"license":"MIT","minAppVersion":"0.0.1-preview.13",
        "platforms":["android","ios","windows","macos","linux","ohos","web"],"files":entries});
    files.insert("manifest.json".into(), serde_json::to_vec(&manifest).unwrap());
    files
}

#[test]
fn checks_exact_sets_bytes_and_hashes() {
    let original = files();
    let checked = check_file_set(&original, "0.0.1-preview.13").unwrap();
    assert_eq!(checked.theme().sha256(), parse_theme(FIXTURE).unwrap().sha256());
    let mut extra = original.clone();
    extra.insert("licenses/extra.txt".into(), b"extra".to_vec());
    assert!(check_file_set(&extra, "0.0.1-preview.13").is_err());
    let mut tampered = original.clone();
    tampered.get_mut("preview.png").unwrap().push(0);
    assert!(check_file_set(&tampered, "0.0.1-preview.13").is_err());
    let mut missing = original;
    missing.remove("preview.png");
    assert!(check_file_set(&missing, "0.0.1-preview.13").is_err());
}

#[test]
fn compares_full_core_prerelease_versions() {
    let files = files();
    let manifest = &files["manifest.json"];
    assert!(parse_manifest(manifest, "0.0.1-preview.12").is_err());
    assert!(parse_manifest(manifest, "0.0.1-preview.13+windows.17").is_ok());
    assert!(parse_manifest(manifest, "0.0.1").is_ok());
}

#[test]
fn rejects_deep_trailing_and_oversized_json() {
    let deep = format!("{}0{}", "[".repeat(34), "]".repeat(34));
    assert!(normalize_draft(deep.as_bytes()).is_err());
    assert!(normalize_draft(br#"{"themeApiVersion":1} false"#).is_err());
    let bytes = vec![b' '; MAX_THEME_BYTES + 1];
    assert_eq!(normalize_draft(&bytes).unwrap_err().code, "theme-input-budget-exceeded");
}

fn zip_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut output = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut output);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored);
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
    let central_start = u32::from_le_bytes(original[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    let central_size = u32::from_le_bytes(original[eocd + 12..eocd + 16].try_into().unwrap()) as usize;
    let local = &original[..central_start];
    let central = &original[central_start..eocd];
    assert_eq!(central.len(), central_size);
    let mut duplicate_local = zip_with(&[(name, second)]);
    let duplicate_eocd = duplicate_local.len() - 22;
    let duplicate_central_start = u32::from_le_bytes(
        duplicate_local[duplicate_eocd + 16..duplicate_eocd + 20].try_into().unwrap(),
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
    let entries: Vec<Value> = files.iter().map(|(path, bytes)| json!({
        "path":path,
        "mediaType":match path.as_str() {
            "theme.json"=>"application/json",
            "preview.png"=>"image/png",
            _=>"text/plain"
        },
        "bytes":bytes.len(),
        "sha256":sha256_hex(bytes)
    })).collect();
    let manifest = json!({
        "format":"operit2.theme",
        "formatVersion":1,
        "themeApiVersion":1,
        "themeId":"dev.operit.example",
        "version":"1.0.0",
        "displayName":"Example",
        "author":{"name":"Example Author"},
        "license":"MIT",
        "minAppVersion":"0.0.1-preview.13",
        "platforms":["android","ios","windows","macos","linux","ohos","web"],
        "files":entries
    });
    files.insert("manifest.json".into(), serde_json::to_vec(&manifest).unwrap());
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
    let package = validate_theme_package(&archive, "0.0.1-preview.13").unwrap();
    assert_eq!(package.archive_sha256(), sha256_hex(&archive));
    assert_eq!(package.manifest().theme_id, "dev.operit.example");
    assert_eq!(package.receipt().trust, ThemePackageTrust::Untrusted);
    assert_eq!(package.file_bytes("preview.png").unwrap().len(), files["preview.png"].len());
    assert_eq!(package.sealed_archive_bytes(), archive.as_slice());
}

#[test]
fn complete_package_rejects_media_after_hash_and_contract_checks() {
    let files = complete_files(b"not a PNG".to_vec());
    let archive = archive_from_files(&files);
    let error = validate_theme_package(&archive, "0.0.1-preview.13").unwrap_err();
    assert_eq!(error.code, "theme-media-invalid");
    assert_eq!(error.stage, ValidationStage::Media);
}

#[test]
fn exporter_is_deterministic_and_exports_a_package_that_import_accepts() {
    let draft = ThemeExportDraft {
        theme_id: "dev.operit.example".into(),
        version: "1.0.0".into(),
        display_name: "Example".into(),
        author_name: "Example Author".into(),
        license: "MIT".into(),
        min_app_version: "0.0.1-preview.13".into(),
        platforms: vec![
            "android".into(),
            "ios".into(),
            "windows".into(),
            "macos".into(),
            "linux".into(),
            "ohos".into(),
            "web".into(),
        ],
        theme_json: FIXTURE.to_vec(),
        preview_png: png_preview(),
        theme_license_text: "Example license".into(),
        description: None,
        max_app_version: None,
        created_with: None,
        assets: Vec::new(),
    };
    let first = export_theme_package(&draft).unwrap();
    let second = export_theme_package(&draft).unwrap();
    assert_eq!(first, second);
    let package = validate_theme_package(&first, "0.0.1-preview.13").unwrap();
    assert_eq!(package.manifest().theme_id, draft.theme_id);
}

#[test]
fn bounded_archive_inspection_is_fail_closed() {
    let archive = zip_with(&[
        ("theme.json", b"{}"),
        ("preview.png", b"png"),
        ("licenses/theme.txt", b"license"),
    ]);
    let inspected = inspect_theme_archive(&archive).unwrap();
    assert_eq!(inspected.archive_bytes, archive.len());
    assert_eq!(inspected.files.len(), 3);
    assert_eq!(inspected.archive_sha256, sha256_hex(&archive));

    for entries in [
        vec![("../theme.json", b"bad".as_slice())],
        vec![("theme.css", b"body".as_slice())],
        vec![("assets\\images\\x.png", b"bad".as_slice())],
        vec![("script.js", b"throw".as_slice())],
    ] {
        assert!(inspect_theme_archive(&zip_with(&entries)).is_err());
    }
    assert!(inspect_theme_archive(&zip_with_duplicate("theme.json", b"one", b"two")).is_err());
}

#[test]
fn archive_rejects_compression_bombs_before_contract_parsing() {
    let bomb = vec![0u8; 2 * 1024 * 1024];
    let archive = {
        let mut output = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut output);
            let options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Deflated);
            writer.start_file("assets/images/bomb.png", options).unwrap();
            writer.write_all(&bomb).unwrap();
            writer.finish().unwrap();
        }
        output.into_inner()
    };
    let error = inspect_theme_archive(&archive).unwrap_err();
    assert_eq!(error.code, "theme-package-archive-invalid");
}
