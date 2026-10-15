use operit_plugin_theme::*;
use serde_json::json;
use std::io::Cursor;

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(width, height, image::Rgba([40, 60, 80, 255]))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}

/// Supplies the ToolPkg metadata and the agreement text referenced by its manifest.
fn metadata() -> serde_json::Value {
    json!({
        "toolpkgId": "dev.operit.binary", "version": "1.0.0", "displayName": "Binary",
        "authorName": "Example Author", "agreementText": "Example agreement", "description": null,
    })
}

#[test]
fn preview_owns_normalization_media_and_receipt() {
    let image = png(32, 32);
    let draft = br#"{"background":{"asset":"assets/images/bg.png"}}"#;
    let preview = validate_theme_preview_draft(
        draft,
        &[ThemePreviewAsset {
            path: "assets/images/bg.png".into(),
            media_type: "image/png".into(),
            bytes: image.clone(),
        }],
    )
    .unwrap();
    assert_eq!(preview.receipt().trust, ThemeTemplateTrust::Untrusted);
    assert_eq!(
        preview.receipt().theme_sha256,
        normalize_draft(draft).unwrap().sha256()
    );
    assert_eq!(preview.receipt().files[0].sha256, sha256_hex(&image));
    assert_eq!(preview.file_bytes("assets/images/bg.png").unwrap(), image);
    assert!(matches!(
        preview.receipt().media[0],
        ThemeMediaValidation::Image {
            frames: 1,
            width: 32,
            height: 32,
            ..
        }
    ));
}

#[test]
fn static_svg_assets_are_structurally_checked_and_hash_bound() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16"><path d="M1 1 L15 15" fill="#334455"/></svg>"##.to_vec();
    let theme = br#"{"background":{"asset":"assets/images/mark.svg"}}"#;
    let preview = validate_theme_preview_draft(
        theme,
        &[ThemePreviewAsset {
            path: "assets/images/mark.svg".into(),
            media_type: "image/svg+xml".into(),
            bytes: svg.clone(),
        }],
    )
    .unwrap();
    assert_eq!(preview.file_bytes("assets/images/mark.svg").unwrap(), svg);
    assert!(matches!(
        preview.receipt().media[0],
        ThemeMediaValidation::SvgImage {
            width: 16,
            height: 16,
            ..
        }
    ));
}

#[test]
fn svg_active_content_external_references_and_unbounded_paths_fail_closed() {
    let theme = br#"{"background":{"asset":"assets/images/unsafe.svg"}}"#;
    for payload in [
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><script>alert(1)</script></svg>"#.as_slice(),
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" onload="alert(1)"/>"#,
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><image href="https://example.test/x.png"/></svg>"#,
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" style="fill:red"/>"#,
        br#"<!DOCTYPE svg [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">&xxe;</svg>"#,
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><foreignObject/></svg>"#,
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M0 0 X1 1"/></svg>"#,
    ] {
        let result = validate_theme_preview_draft(theme, &[ThemePreviewAsset {
            path: "assets/images/unsafe.svg".into(),
            media_type: "image/svg+xml".into(),
            bytes: payload.to_vec(),
        }]);
        assert!(result.is_err(), "accepted unsafe SVG: {}", String::from_utf8_lossy(payload));
    }
}

#[test]
fn static_svg_theme_export_import_uses_the_same_l4_validator() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16"><path d="M1 1 L15 15" fill="#334455"/></svg>"##;
    let theme = br#"{"background":{"asset":"assets/images/mark.svg"}}"#;
    let directory = serde_json::to_vec(&json!([{
        "path":"assets/images/mark.svg",
        "mediaType":"image/svg+xml",
        "offset":0,
        "bytes":svg.len()
    }]))
    .unwrap();
    let mut metadata = metadata();
    let archive = export_toolpkg_from_theme_transfer(
        &serde_json::to_vec(&metadata).unwrap(),
        theme,
        &png(1280, 800),
        &directory,
        svg,
    )
    .unwrap();
    let imported = validate_toolpkg_theme_templates(&archive)
        .map(|mut templates| templates.remove(0))
        .unwrap();
    assert_eq!(imported.file_bytes("assets/images/mark.svg").unwrap(), svg);
    assert!(matches!(imported.receipt().media.iter().find(|media| {
        matches!(media, ThemeMediaValidation::SvgImage { path, .. } if path == "assets/images/mark.svg")
    }), Some(ThemeMediaValidation::SvgImage { width: 16, height: 16, .. })));
}

#[test]
fn native_preview_retains_the_exact_checked_bytes() {
    let mut assets = vec![ThemePreviewAsset {
        path: "assets/images/bg.png".into(),
        media_type: "image/png".into(),
        bytes: png(32, 32),
    }];
    let preview = validate_theme_preview_draft(
        br#"{"background":{"asset":"assets/images/bg.png"}}"#,
        &assets,
    )
    .unwrap();
    let digest = preview.receipt().files[0].sha256.clone();
    assets[0].bytes.fill(0);
    assert_eq!(
        sha256_hex(preview.file_bytes("assets/images/bg.png").unwrap()),
        digest
    );
}

#[test]
fn matching_hashes_do_not_make_fake_media_renderable() {
    let bytes = b"not an image";
    let directory = json!([{"path":"assets/images/bg.png","mediaType":"image/png","offset":0,"bytes":bytes.len()}]);
    let error = validate_theme_preview_transfer(
        br#"{"background":{"asset":"assets/images/bg.png"}}"#,
        &serde_json::to_vec(&directory).unwrap(),
        bytes,
    )
    .unwrap_err();
    assert_eq!(error.stage, ValidationStage::Media);
    assert_eq!(error.code, "theme-media-invalid");
}

#[test]
fn binary_directory_rejects_aliases_gaps_overlaps_truncation_and_unknown_keys() {
    let bytes = png(32, 32);
    let draft = br#"{"background":{"asset":"assets/images/bg.png"}}"#;
    let row = json!({"path":"assets/images/bg.png","mediaType":"image/png","offset":0,"bytes":bytes.len()});
    let mut wrong_offset = row.clone();
    wrong_offset["offset"] = json!(1);
    let mut wrong_length = row.clone();
    wrong_length["bytes"] = json!(bytes.len() + 1);
    let mut unknown = row.clone();
    unknown["execute"] = json!("script");
    let mut wrong_mime = row.clone();
    wrong_mime["mediaType"] = json!("font/ttf");
    let mut traversal = row.clone();
    traversal["path"] = json!("assets/images/../bg.png");
    for directory in [
        json!([wrong_offset]),
        json!([wrong_length]),
        json!([unknown]),
        json!([wrong_mime]),
        json!([traversal]),
        json!([row.clone(), row.clone()]),
    ] {
        assert!(validate_theme_preview_transfer(
            draft,
            &serde_json::to_vec(&directory).unwrap(),
            &bytes
        )
        .is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(validate_theme_preview_transfer(
        draft,
        &serde_json::to_vec(&json!([row])).unwrap(),
        &trailing
    )
    .is_err());
    assert!(validate_theme_preview_transfer(draft,
        br#"[{"path":"assets/images/bg.png","path":"assets/images/other.png","mediaType":"image/png","offset":0,"bytes":1}]"#,
        &[0],
    ).is_err());
}

#[test]
fn preview_checks_exact_resource_closure_and_bubble_pixel_relations() {
    let assets = vec![ThemePreviewAsset {
        path: "assets/images/bubble.png".into(),
        media_type: "image/png".into(),
        bytes: png(1, 1),
    }];
    assert!(validate_theme_preview_draft(br#"{}"#, &assets).is_err());
    let draft = br#"{"chat":{"user":{"image":{"asset":"assets/images/bubble.png"}}}}"#;
    assert!(validate_theme_preview_draft(draft, &[]).is_err());
    assert_eq!(
        validate_theme_preview_draft(draft, &assets)
            .unwrap_err()
            .stage,
        ValidationStage::Media
    );
}

#[test]
fn binary_export_is_deterministic_and_round_trips_through_import() {
    let draft = br#"{}"#;
    let preview = png(1280, 800);
    let meta = serde_json::to_vec(&metadata()).unwrap();
    let first = export_toolpkg_from_theme_transfer(&meta, draft, &preview, b"[]", &[]).unwrap();
    let second = export_toolpkg_from_theme_transfer(&meta, draft, &preview, b"[]", &[]).unwrap();
    assert_eq!(first, second);
    let package = validate_toolpkg_theme_templates(&first)
        .map(|mut templates| templates.remove(0))
        .unwrap();
    assert_eq!(
        package.theme().sha256(),
        normalize_draft(draft).unwrap().sha256()
    );
    assert_eq!(package.receipt().trust, ThemeTemplateTrust::Untrusted);
    let mut unknown = metadata();
    unknown["script"] = json!("alert()");
    assert!(export_toolpkg_from_theme_transfer(
        &serde_json::to_vec(&unknown).unwrap(),
        draft,
        &preview,
        b"[]",
        &[]
    )
    .is_err());
}

#[test]
fn export_supports_real_fonts_without_a_json_integer_array_budget() {
    let font = include_bytes!(
        "../../../../../apps/flutter/packages/operit_ui/assets/fonts/noto-sans-sc.ttf"
    );
    assert!(font.len() > 2 * 1024 * 1024);
    let theme = br#"{"typography":{"global":"assets/fonts/custom.ttf"}}"#;
    let directory = json!([{"path":"assets/fonts/custom.ttf","mediaType":"font/ttf","offset":0,"bytes":font.len()}]);
    let mut meta = metadata();
    let archive = export_toolpkg_from_theme_transfer(
        &serde_json::to_vec(&meta).unwrap(),
        theme,
        &png(1280, 800),
        &serde_json::to_vec(&directory).unwrap(),
        font,
    )
    .unwrap();
    let package = validate_toolpkg_theme_templates(&archive)
        .map(|mut templates| templates.remove(0))
        .unwrap();
    assert_eq!(package.file_bytes("assets/fonts/custom.ttf").unwrap(), font);
}

/// Rejects theme-specific legal fields instead of treating them as alternative ToolPkg metadata.
#[test]
fn theme_export_rejects_removed_protocol_and_compatibility_fields() {
    for field in [
        "license",
        "assetLicenses",
        "platforms",
        "minAppVersion",
        "maxAppVersion",
        "createdWith",
    ] {
        let mut meta = metadata();
        meta[field] = json!("obsolete");
        assert!(
            export_toolpkg_from_theme_transfer(
                &serde_json::to_vec(&meta).unwrap(),
                br#"{}"#,
                &png(1280, 800),
                b"[]",
                &[],
            )
            .is_err(),
            "{field}"
        );
    }
}

/// Validates rewritten nested manifests and preserves ordinary package identities without extra theme rules.
#[test]
fn final_archive_validation_accepts_nested_manifest_and_ordinary_id() {
    use std::io::Read;
    let mut meta = metadata();
    meta["toolpkgId"] = json!("com.operit.character_cards");
    meta["displayName"] = json!("");
    meta["authorName"] = json!("");
    let bytes = export_toolpkg_from_theme_transfer(
        &serde_json::to_vec(&meta).unwrap(), b"{}", &png(1280,800), b"[]", b"",
    ).unwrap();
    let mut input = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..input.len() {
        let mut file = input.by_index(index).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        let path = if file.name() == "manifest.json" { "bundle/manifest.hjson".to_owned() } else { format!("bundle/{}", file.name()) };
        output.start_file(path, zip::write::SimpleFileOptions::default()).unwrap();
        std::io::Write::write_all(&mut output,&data).unwrap();
    }
    let archive = output.finish().unwrap().into_inner();
    let templates = validate_authored_toolpkg_theme_templates(&archive,"bundle/manifest.hjson").unwrap();
    assert_eq!(templates.len(),1);
    assert_eq!(templates[0].metadata().package_id,"com.operit.character_cards");
}

/// Requires untouched contributions to remain valid before returning the merged archive to Studio.
#[test]
fn final_archive_validation_rejects_missing_resources_in_another_template() {
    use std::io::Read;
    let bytes = export_toolpkg_from_theme_transfer(
        &serde_json::to_vec(&metadata()).unwrap(), b"{}", &png(1280,800), b"[]", b"",
    ).unwrap();
    let mut input = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..input.len() {
        let mut file = input.by_index(index).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        if file.name() == "manifest.json" {
            let mut manifest: serde_json::Value = serde_json::from_slice(&data).unwrap();
            let mut other = manifest["themes"][0].clone();
            other["id"] = json!("other");
            other["files"][0]["path"] = json!("assets/images/missing.png");
            manifest["themes"].as_array_mut().unwrap().push(other);
            data = serde_json::to_vec(&manifest).unwrap();
        }
        output.start_file(file.name(),zip::write::SimpleFileOptions::default()).unwrap();
        std::io::Write::write_all(&mut output,&data).unwrap();
    }
    let archive = output.finish().unwrap().into_inner();
    assert!(validate_authored_toolpkg_theme_templates(&archive,"manifest.json").is_err());
}

/// Exported contributions only ask authors to identify their files and media types.
#[test]
fn exported_manifest_omits_resource_sizes_and_hashes() {
    let bytes = export_toolpkg_from_theme_transfer(
        &serde_json::to_vec(&metadata()).unwrap(), b"{}", &png(1280,800), b"[]", b"",
    ).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let manifest: serde_json::Value = serde_json::from_reader(archive.by_name("manifest.json").unwrap()).unwrap();
    for file in manifest["themes"][0]["files"].as_array().unwrap() {
        assert_eq!(file.as_object().unwrap().len(), 2);
        assert!(file.get("bytes").is_none());
        assert!(file.get("sha256").is_none());
    }
}
