//! Thin browser binding to the same native contract implementation.
//! It emits deterministic bytes but does not approve packages or install them.
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = normalizeThemeDraft)]
pub fn normalize_theme_draft(bytes: &[u8]) -> String {
    let response = match crate::normalize_draft(bytes) {
        Ok(theme) => serde_json::json!({
            "ok": true,
            "theme": theme.as_json(),
            "canonicalThemeJson": String::from_utf8(theme.canonical_bytes())
                .expect("normalized theme JSON must be UTF-8"),
            "themeSha256": theme.sha256(),
        }),
        Err(error) => serde_json::json!({"ok": false, "errors": [error]}),
    };
    serde_json::to_string(&response).expect("contract response must serialize")
}

#[wasm_bindgen(js_name = exportToolPkgFromTheme)]
pub fn export_toolpkg_from_theme(
    metadata_json: &[u8], theme_json: &[u8], preview_png: &[u8],
    directory_json: &[u8], payload: &[u8],
) -> Result<Vec<u8>, JsValue> {
    crate::export_toolpkg_from_theme_transfer(
        metadata_json, theme_json, preview_png, directory_json, payload,
    ).map_err(|error| JsValue::from_str(
        &serde_json::to_string(&error).expect("contract error must serialize"),
    ))
}

#[wasm_bindgen(js_name = validateThemePreviewDraft)]
pub fn validate_theme_preview_draft(
    theme_json: &[u8], directory_json: &[u8], payload: &[u8],
) -> String {
    let response = match crate::validate_theme_preview_transfer(theme_json, directory_json, payload) {
        Ok(preview) => serde_json::json!({
            "ok": true,
            "canonicalThemeJson": String::from_utf8(preview.theme().canonical_bytes())
                .expect("normalized theme JSON must be UTF-8"),
            "themeSha256": preview.theme().sha256(),
            "receipt": preview.receipt(),
        }),
        Err(error) => serde_json::json!({"ok": false, "errors": [error]}),
    };
    serde_json::to_string(&response).expect("contract response must serialize")
}

/// Checks the complete merged ToolPkg before Studio exposes its download.
#[wasm_bindgen(js_name = validateAuthoredToolPkg)]
pub fn validate_authored_toolpkg(archive_bytes: &[u8], manifest_path: &str) -> String {
    let response = match crate::toolpkg::validate_authored_toolpkg_theme_templates(archive_bytes, manifest_path) {
        Ok(templates) => serde_json::json!({
            "ok": true,
            "receipts": templates.iter().map(|template| template.receipt().clone()).collect::<Vec<_>>(),
        }),
        Err(error) => serde_json::json!({"ok": false, "errors": [error]}),
    };
    serde_json::to_string(&response).expect("ToolPkg validation response must serialize")
}
