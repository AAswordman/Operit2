//! L4 checks for the image and font formats allowed by theme v1.
//!
//! The ZIP and manifest layers only establish where bytes came from. This
//! module establishes that the bytes are the declared media, are structurally
//! bounded, and can be consumed by a complete decoder/parser walk.

use crate::{sha256_hex, ManifestFile, ThemeSpecV1, ThemeValidationError, ValidationStage};
use crate::{MAX_FONT_BYTES, MAX_FONT_GLYPHS, MAX_FONT_TABLES, MAX_IMAGE_DIMENSION,
    MAX_IMAGE_PIXELS, MAX_TOTAL_IMAGE_PIXELS, MAX_SVG_BYTES, PREVIEW_HEIGHT, PREVIEW_WIDTH};
use image::{ImageFormat, ImageReader};
use serde::Serialize;
use std::collections::{BTreeSet, VecDeque};
use std::io::Cursor;
use std::sync::{Mutex, OnceLock};
use ttf_parser::{Face, GlyphId, OutlineBuilder};

const MAX_IMAGE_CHUNKS: usize = 4_096;
const MAX_CMAP_SUBTABLES: usize = 64;
const MAX_CMAP_CODEPOINTS: usize = 1_000_000;
const MAX_GLYPH_EVENTS: usize = 32_768;
const MAX_TOTAL_GLYPH_EVENTS: usize = 8_000_000;

/// Evidence that a declared media file passed the L4 decoder/parser walk.
/// This type deliberately contains no resource bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ThemeMediaValidation {
    Image {
        path: String,
        media_type: String,
        bytes: usize,
        sha256: String,
        width: u32,
        height: u32,
        pixels: usize,
        frames: u32,
    },
    TrueTypeFont {
        path: String,
        media_type: String,
        bytes: usize,
        sha256: String,
        tables: usize,
        glyphs: usize,
        units_per_em: u16,
    },
    SvgImage {
        path: String,
        media_type: String,
        bytes: usize,
        sha256: String,
        width: u32,
        height: u32,
        pixels: usize,
    },
}

type MediaCacheKey = (String, String, usize, String);

fn media_validation_cache() -> &'static Mutex<VecDeque<(MediaCacheKey, ThemeMediaValidation)>> {
    static CACHE: OnceLock<Mutex<VecDeque<(MediaCacheKey, ThemeMediaValidation)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(VecDeque::with_capacity(32)))
}

pub(crate) fn validate_media_files<B: AsRef<[u8]>>(
    files: &std::collections::BTreeMap<String, B>,
    manifest_files: &[ManifestFile],
) -> Result<Vec<ThemeMediaValidation>, ThemeValidationError> {
    let mut receipts = Vec::new();
    let mut total_image_pixels = 0usize;
    for manifest_file in manifest_files {
        let Some(bytes) = files.get(&manifest_file.path) else {
            return Err(media_error(&manifest_file.path, "declared media bytes are missing"));
        };
        let bytes = bytes.as_ref();
        match manifest_file.media_type.as_str() {
            "application/json" | "text/plain" => continue,
            "image/png" | "image/jpeg" | "font/ttf" | "image/svg+xml" => {}
            other => {
                return Err(media_error(
                    &manifest_file.path,
                    format!("unsupported declared media type: {other}"),
                ))
            }
        }
        let digest = sha256_hex(bytes);
        let cache_key: MediaCacheKey = (
            manifest_file.path.clone(),
            manifest_file.media_type.clone(),
            bytes.len(),
            digest.clone(),
        );
        let cached_receipt = media_validation_cache()
            .lock()
            .ok()
            .and_then(|cache| cache.iter().find(|(k, _)| k == &cache_key).map(|(_, v)| v.clone()));
        let receipt = if let Some(cached) = cached_receipt {
            cached
        } else {
            let computed = match manifest_file.media_type.as_str() {
                "image/png" | "image/jpeg" => {
                    let (width, height, frames) = validate_image(
                        &manifest_file.path,
                        bytes,
                        &manifest_file.media_type,
                    )?;
                    let pixels = (width as usize)
                        .checked_mul(height as usize)
                        .ok_or_else(|| media_error(&manifest_file.path, "image pixel count overflows usize"))?;
                    ThemeMediaValidation::Image {
                        path: manifest_file.path.clone(),
                        media_type: manifest_file.media_type.clone(),
                        bytes: bytes.len(),
                        sha256: digest,
                        width,
                        height,
                        pixels,
                        frames,
                    }
                }
                "font/ttf" => {
                    let summary = validate_true_type(&manifest_file.path, bytes)?;
                    ThemeMediaValidation::TrueTypeFont {
                        path: manifest_file.path.clone(),
                        media_type: manifest_file.media_type.clone(),
                        bytes: bytes.len(),
                        sha256: digest,
                        tables: summary.tables,
                        glyphs: summary.glyphs,
                        units_per_em: summary.units_per_em,
                    }
                }
                "image/svg+xml" => {
                    let (width, height) = validate_svg(&manifest_file.path, bytes)?;
                    let pixels = (width as usize)
                        .checked_mul(height as usize)
                        .ok_or_else(|| media_error(&manifest_file.path, "SVG pixel count overflows usize"))?;
                    ThemeMediaValidation::SvgImage {
                        path: manifest_file.path.clone(),
                        media_type: manifest_file.media_type.clone(),
                        bytes: bytes.len(),
                        sha256: digest,
                        width,
                        height,
                        pixels,
                    }
                }
                _ => unreachable!(),
            };
            if let Ok(mut cache) = media_validation_cache().lock() {
                cache.retain(|(k, _)| k != &cache_key);
                if cache.len() >= 32 {
                    cache.pop_front();
                }
                cache.push_back((cache_key, computed.clone()));
            }
            computed
        };
        match &receipt {
            ThemeMediaValidation::Image { path, width, height, pixels, .. } => {
                if path == "preview.png"
                    && (*width as usize != PREVIEW_WIDTH || *height as usize != PREVIEW_HEIGHT)
                {
                    return Err(media_error(path, "preview.png must be exactly 1280 x 800"));
                }
                total_image_pixels = total_image_pixels
                    .checked_add(*pixels)
                    .ok_or_else(|| media_error(path, "total image pixel count overflows usize"))?;
                if total_image_pixels > MAX_TOTAL_IMAGE_PIXELS {
                    return Err(media_error(path, "total image pixel budget exceeded"));
                }
            }
            ThemeMediaValidation::SvgImage { path, pixels, .. } => {
                total_image_pixels = total_image_pixels
                    .checked_add(*pixels)
                    .ok_or_else(|| media_error(path, "total image pixel count overflows usize"))?;
                if total_image_pixels > MAX_TOTAL_IMAGE_PIXELS {
                    return Err(media_error(path, "total image pixel budget exceeded"));
                }
            }
            ThemeMediaValidation::TrueTypeFont { .. } => {}
        }
        receipts.push(receipt);
    }
    receipts.sort_by(|left, right| media_path(left).cmp(media_path(right)));
    Ok(receipts)
}

fn media_path(receipt: &ThemeMediaValidation) -> &str {
    match receipt {
        ThemeMediaValidation::Image { path, .. }
        | ThemeMediaValidation::TrueTypeFont { path, .. }
        | ThemeMediaValidation::SvgImage { path, .. } => path,
    }
}

fn validate_image(path: &str, bytes: &[u8], media_type: &str) -> Result<(u32, u32, u32), ThemeValidationError> {
    if bytes.is_empty() { return Err(media_error(path, "image is empty")); }
    let (format, container_dimensions) = match media_type {
        "image/png" => (ImageFormat::Png, validate_png_container(path, bytes)?),
        "image/jpeg" => (ImageFormat::Jpeg, validate_jpeg_container(path, bytes)?),
        _ => return Err(media_error(path, "image media type is not allowed")),
    };
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| media_error(path, format!("image format cannot be identified: {error}")))?;
    if reader.format() != Some(format) {
        return Err(media_error(path, "image extension, media type and decoder format differ"));
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION as u32);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION as u32);
    limits.max_alloc = Some(192 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|error| media_error(path, format!("image decode failed: {error}")))?;
    let width = decoded.width();
    let height = decoded.height();
    if width == 0 || height == 0 || width as usize > MAX_IMAGE_DIMENSION || height as usize > MAX_IMAGE_DIMENSION {
        return Err(media_error(path, "decoded image dimensions exceed v1 bounds"));
    }
    let pixels = (width as usize).checked_mul(height as usize)
        .ok_or_else(|| media_error(path, "decoded image pixel count overflows usize"))?;
    if pixels > MAX_IMAGE_PIXELS { return Err(media_error(path, "decoded image pixel budget exceeded")); }
    if let Some((container_width, container_height)) = container_dimensions {
        if width != container_width || height != container_height {
            return Err(media_error(path, "decoded dimensions differ from the image container"));
        }
    }
    Ok((width, height, 1))
}

const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";
const MAX_SVG_ELEMENTS: usize = 512;
const MAX_SVG_DEPTH: usize = 32;
const MAX_SVG_PATH_CHARS: usize = 65_536;
const MAX_SVG_RASTER_DIMENSION: u32 = 4096;

/// Validates a bounded, static SVG subset before the Flutter renderer sees it.
/// The accepted document contains vector shapes only: no script, CSS blocks,
/// network/file references, embedded documents, filters, fonts or animation.
fn validate_svg(path: &str, bytes: &[u8]) -> Result<(u32, u32), ThemeValidationError> {
    if bytes.is_empty() || bytes.len() > MAX_SVG_BYTES {
        return Err(media_error(path, "SVG byte budget exceeded"));
    }
    let source = std::str::from_utf8(bytes)
        .map_err(|_| media_error(path, "SVG must be valid UTF-8"))?;
    // XML DTD declarations are case-sensitive. Reject them before invoking the
    // XML parser so entity declarations and external identifiers are inert.
    if source.contains("<!DOCTYPE") || source.contains("<!ENTITY") {
        return Err(media_error(path, "SVG DTD and entity declarations are not allowed"));
    }
    let trimmed = source.trim_start_matches('\u{feff}').trim_start();
    let body = if let Some(xml_declaration) = trimmed.strip_prefix("<?xml ") {
        let end = xml_declaration.find("?>")
            .ok_or_else(|| media_error(path, "SVG XML declaration is incomplete"))?;
        let mut attributes = xml_declaration[..end].split_ascii_whitespace();
        if !matches!(attributes.next(), Some("version=\"1.0\"" | "version='1.0'")) {
            return Err(media_error(path, "SVG XML declaration version is not allowed"));
        }
        for attribute in attributes {
            if !matches!(attribute,
                "encoding=\"UTF-8\"" | "encoding=\"utf-8\"" |
                "encoding='UTF-8'" | "encoding='utf-8'" |
                "standalone=\"yes\"" | "standalone=\"no\"" |
                "standalone='yes'" | "standalone='no'")
            {
                return Err(media_error(path, "SVG XML declaration attribute is not allowed"));
            }
        }
        &xml_declaration[end + 2..]
    } else if trimmed.starts_with("<?") {
        return Err(media_error(path, "SVG processing instructions are not allowed"));
    } else {
        trimmed
    };
    if body.contains("<?") {
        return Err(media_error(path, "SVG processing instructions are not allowed"));
    }
    if source.bytes().filter(|byte| *byte == b'<').count() > MAX_SVG_ELEMENTS * 2 + 2 {
        return Err(media_error(path, "SVG XML token budget exceeded"));
    }
    let document = roxmltree::Document::parse(source)
        .map_err(|error| media_error(path, format!("SVG XML is invalid: {error}")))?;
    let root = document.root_element();
    if root.tag_name().name() != "svg" || root.tag_name().namespace() != Some(SVG_NAMESPACE) {
        return Err(media_error(path, "SVG root must declare the standard SVG namespace"));
    }
    let view_box = root.attribute("viewBox")
        .ok_or_else(|| media_error(path, "SVG viewBox is required"))?;
    let values = parse_svg_numbers(path, view_box, 4, false)?;
    let view_width = values[2];
    let view_height = values[3];
    if values[0].abs() > 1_000_000.0 || values[1].abs() > 1_000_000.0
        || view_width <= 0.0 || view_height <= 0.0
        || view_width > 1_000_000.0 || view_height > 1_000_000.0
    {
        return Err(media_error(path, "SVG viewBox dimensions are outside v1 bounds"));
    }
    let root_width = root.attribute("width");
    let root_height = root.attribute("height");
    if root_width.is_some() != root_height.is_some() {
        return Err(media_error(path, "SVG width and height must be declared together"));
    }
    let intrinsic_width = root_width
        .map(|value| parse_svg_length(path, value, false))
        .transpose()?
        .unwrap_or(view_width);
    let intrinsic_height = root_height
        .map(|value| parse_svg_length(path, value, false))
        .transpose()?
        .unwrap_or(view_height);
    if intrinsic_width <= 0.0 || intrinsic_height <= 0.0
        || intrinsic_width > MAX_SVG_RASTER_DIMENSION as f64
        || intrinsic_height > MAX_SVG_RASTER_DIMENSION as f64
    {
        return Err(media_error(path, "SVG intrinsic dimensions are outside v1 bounds"));
    }
    if let Some(value) = root.attribute("preserveAspectRatio") {
        if !matches!(value,
            "none" | "xMinYMin meet" | "xMidYMin meet" | "xMaxYMin meet" |
            "xMinYMid meet" | "xMidYMid meet" | "xMaxYMid meet" |
            "xMinYMax meet" | "xMidYMax meet" | "xMaxYMax meet" |
            "xMinYMin slice" | "xMidYMin slice" | "xMaxYMin slice" |
            "xMinYMid slice" | "xMidYMid slice" | "xMaxYMid slice" |
            "xMinYMax slice" | "xMidYMax slice" | "xMaxYMax slice")
        {
            return Err(media_error(path, "SVG preserveAspectRatio value is not allowed"));
        }
    }

    let mut element_count = 0usize;
    let mut ids = BTreeSet::new();
    let mut references = Vec::new();
    let mut stack = vec![(root, 1usize)];
    while let Some((node, depth)) = stack.pop() {
        if node.is_element() {
            element_count += 1;
            if element_count > MAX_SVG_ELEMENTS || depth > MAX_SVG_DEPTH {
                return Err(media_error(path, "SVG element or nesting budget exceeded"));
            }
            if node.tag_name().namespace() != Some(SVG_NAMESPACE) {
                return Err(media_error(path, "SVG element uses an unregistered namespace"));
            }
            let tag = node.tag_name().name();
            if !matches!(tag,
                "svg" | "g" | "defs" | "path" | "rect" | "circle" | "ellipse" |
                "line" | "polyline" | "polygon" | "linearGradient" | "radialGradient" |
                "stop" | "clipPath" | "title" | "desc")
            {
                return Err(media_error(path, format!("SVG element is outside the static subset: {tag}")));
            }
            for attribute in node.attributes() {
                let name = attribute.name();
                let value = attribute.value();
                if value.len() > MAX_SVG_PATH_CHARS
                    || !svg_attribute_allowed(tag, name)
                {
                    return Err(media_error(path, format!("SVG attribute is outside the static subset: {name}")));
                }
                if name == "id" {
                    if !valid_svg_id(value) || !ids.insert(value.to_string()) {
                        return Err(media_error(path, "SVG IDs must be unique bounded identifiers"));
                    }
                }
                validate_svg_attribute_value(path, name, value, &mut references)?;
            }
            if tag == "path" {
                validate_svg_path_data(path, node.attribute("d").unwrap_or(""))?;
            }
            if tag == "polyline" || tag == "polygon" {
                let points = parse_svg_numeric_list(path, node.attribute("points").unwrap_or(""), false)?;
                if points.len() < 4 || points.len() % 2 != 0 {
                    return Err(media_error(path, "SVG points must contain at least two coordinate pairs"));
                }
            }
            if tag == "linearGradient" || tag == "radialGradient" {
                if let Some(units) = node.attribute("gradientUnits") {
                    if !matches!(units, "objectBoundingBox" | "userSpaceOnUse") {
                        return Err(media_error(path, "SVG gradientUnits is invalid"));
                    }
                }
            }
            if tag == "clipPath" {
                if let Some(units) = node.attribute("clipPathUnits") {
                    if !matches!(units, "objectBoundingBox" | "userSpaceOnUse") {
                        return Err(media_error(path, "SVG clipPathUnits is invalid"));
                    }
                }
            }
        } else if node.is_pi() {
            return Err(media_error(path, "SVG processing instructions are not allowed"));
        } else if node.is_text() {
            let text = node.text().unwrap_or_default();
            if !text.trim().is_empty() {
                let parent = node.parent().map(|parent| parent.tag_name().name());
                if !matches!(parent, Some("title" | "desc")) || text.len() > 512 {
                    return Err(media_error(path, "SVG text is only allowed in bounded title or desc nodes"));
                }
            }
        }
        for child in node.children() {
            stack.push((child, if node.is_element() { depth + 1 } else { depth }));
        }
    }
    for reference in references {
        if !ids.contains(&reference) {
            return Err(media_error(path, "SVG local paint or clip reference is unresolved"));
        }
    }

    let width = intrinsic_width.ceil() as u32;
    let height = intrinsic_height.ceil() as u32;
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| media_error(path, "SVG pixel count overflows usize"))?;
    if pixels > MAX_IMAGE_PIXELS {
        return Err(media_error(path, "SVG pixel budget exceeded"));
    }
    Ok((width, height))
}

fn svg_attribute_allowed(tag: &str, name: &str) -> bool {
    if name == "id" || name == "transform" || name == "opacity" {
        return true;
    }
    match tag {
        "svg" => matches!(name, "xmlns" | "viewBox" | "width" | "height" | "version" | "preserveAspectRatio" | "fill" | "fill-opacity" | "stroke" | "stroke-opacity"),
        "g" => matches!(name, "fill" | "fill-opacity" | "fill-rule" | "stroke" | "stroke-opacity" | "stroke-width" | "stroke-linecap" | "stroke-linejoin" | "stroke-miterlimit" | "clip-path"),
        "defs" => false,
        "path" => matches!(name, "d" | "fill" | "fill-opacity" | "fill-rule" | "stroke" | "stroke-opacity" | "stroke-width" | "stroke-linecap" | "stroke-linejoin" | "stroke-miterlimit" | "stroke-dasharray" | "stroke-dashoffset" | "clip-path"),
        "rect" => matches!(name, "x" | "y" | "width" | "height" | "rx" | "ry" | "fill" | "fill-opacity" | "fill-rule" | "stroke" | "stroke-opacity" | "stroke-width" | "stroke-linecap" | "stroke-linejoin" | "stroke-miterlimit" | "clip-path"),
        "circle" => matches!(name, "cx" | "cy" | "r" | "fill" | "fill-opacity" | "fill-rule" | "stroke" | "stroke-opacity" | "stroke-width" | "stroke-linecap" | "stroke-linejoin" | "stroke-miterlimit" | "clip-path"),
        "ellipse" => matches!(name, "cx" | "cy" | "rx" | "ry" | "fill" | "fill-opacity" | "fill-rule" | "stroke" | "stroke-opacity" | "stroke-width" | "stroke-linecap" | "stroke-linejoin" | "stroke-miterlimit" | "clip-path"),
        "line" => matches!(name, "x1" | "y1" | "x2" | "y2" | "stroke" | "stroke-opacity" | "stroke-width" | "stroke-linecap" | "stroke-linejoin" | "stroke-miterlimit" | "clip-path"),
        "polyline" | "polygon" => matches!(name, "points" | "fill" | "fill-opacity" | "fill-rule" | "stroke" | "stroke-opacity" | "stroke-width" | "stroke-linecap" | "stroke-linejoin" | "stroke-miterlimit" | "clip-path"),
        "linearGradient" => matches!(name, "x1" | "y1" | "x2" | "y2" | "gradientUnits" | "gradientTransform" | "spreadMethod"),
        "radialGradient" => matches!(name, "cx" | "cy" | "r" | "fx" | "fy" | "fr" | "gradientUnits" | "gradientTransform" | "spreadMethod"),
        "stop" => matches!(name, "offset" | "stop-color" | "stop-opacity"),
        "clipPath" => matches!(name, "clipPathUnits"),
        "title" | "desc" => false,
        _ => false,
    }
}

fn validate_svg_attribute_value(
    path: &str,
    name: &str,
    value: &str,
    references: &mut Vec<String>,
) -> Result<(), ThemeValidationError> {
    match name {
        "d" => {}
        "points" => {}
        "transform" | "gradientTransform" => validate_svg_transform(path, value)?,
        "fill" | "stroke" | "stop-color" => {
            if let Some(id) = value.strip_prefix("url(#").and_then(|rest| rest.strip_suffix(')')) {
                if !valid_svg_id(id) { return Err(media_error(path, "SVG paint reference is invalid")); }
                references.push(id.to_string());
            } else if !valid_svg_color(value) {
                return Err(media_error(path, "SVG paint must be a literal color or local reference"));
            }
        }
        "clip-path" => {
            let Some(id) = value.strip_prefix("url(#").and_then(|rest| rest.strip_suffix(')')) else {
                return Err(media_error(path, "SVG clip-path must be a local reference"));
            };
            if !valid_svg_id(id) { return Err(media_error(path, "SVG clip-path reference is invalid")); }
            references.push(id.to_string());
        }
        "fill-rule" => if !matches!(value, "nonzero" | "evenodd") { return Err(media_error(path, "SVG fill-rule is invalid")); },
        "stroke-linecap" => if !matches!(value, "butt" | "round" | "square") { return Err(media_error(path, "SVG stroke-linecap is invalid")); },
        "stroke-linejoin" => if !matches!(value, "miter" | "round" | "bevel") { return Err(media_error(path, "SVG stroke-linejoin is invalid")); },
        "spreadMethod" => if !matches!(value, "pad" | "reflect" | "repeat") { return Err(media_error(path, "SVG spreadMethod is invalid")); },
        "gradientUnits" | "clipPathUnits" => if !matches!(value, "objectBoundingBox" | "userSpaceOnUse") { return Err(media_error(path, "SVG units value is invalid")); },
        "preserveAspectRatio" => {}
        "version" => if value != "1.1" && value != "2.0" { return Err(media_error(path, "SVG version is invalid")); },
        "id" => if !valid_svg_id(value) { return Err(media_error(path, "SVG ID is invalid")); },
        "width" | "height" => { parse_svg_length(path, value, false)?; }
        "offset" => { parse_svg_length(path, value, true)?; }
        "opacity" | "fill-opacity" | "stroke-opacity" | "stop-opacity" => {
            let number = parse_svg_length(path, value, true)?;
            let normalized = if value.ends_with('%') { number / 100.0 } else { number };
            if !(0.0..=1.0).contains(&normalized) { return Err(media_error(path, "SVG opacity is outside [0, 1]")); }
        }
        "stroke-width" | "stroke-miterlimit" | "r" | "rx" | "ry" | "fr" => {
            if parse_svg_length(path, value, false)? < 0.0 { return Err(media_error(path, "SVG lengths cannot be negative")); }
        }
        "viewBox" => { parse_svg_numbers(path, value, 4, false)?; }
        "x" | "y" | "cx" | "cy" | "x1" | "x2" | "y1" | "y2" | "fx" | "fy" => { parse_svg_length(path, value, true)?; }
        "stroke-dasharray" => { parse_svg_numeric_list(path, value, true)?; }
        "stroke-dashoffset" => { parse_svg_length(path, value, true)?; }
        _ => return Err(media_error(path, format!("SVG attribute is not implemented: {name}"))),
    }
    Ok(())
}

fn parse_svg_length(path: &str, source: &str, allow_percent: bool) -> Result<f64, ThemeValidationError> {
    let value = source.trim();
    let number = if let Some(value) = value.strip_suffix("px") { value }
        else if allow_percent { value.strip_suffix('%').unwrap_or(value) } else { value };
    let parsed = number.parse::<f64>().map_err(|_| media_error(path, "SVG numeric value is malformed"))?;
    if !parsed.is_finite() || parsed.abs() > 1_000_000.0 {
        return Err(media_error(path, "SVG numeric value exceeds its finite bound"));
    }
    Ok(parsed)
}

fn parse_svg_numeric_list(path: &str, source: &str, allow_percent: bool) -> Result<Vec<f64>, ThemeValidationError> {
    let mut values = Vec::new();
    for value in source.split(|character: char| character.is_ascii_whitespace() || character == ',').filter(|part| !part.is_empty()) {
        values.push(parse_svg_length(path, value, allow_percent)?);
        if values.len() > 256 { return Err(media_error(path, "SVG numeric list exceeds v1 bounds")); }
    }
    if values.is_empty() { return Err(media_error(path, "SVG numeric list cannot be empty")); }
    Ok(values)
}

fn parse_svg_numbers(path: &str, source: &str, expected: usize, allow_percent: bool) -> Result<Vec<f64>, ThemeValidationError> {
    let values = parse_svg_numeric_list(path, source, allow_percent)?;
    if values.len() != expected { return Err(media_error(path, "SVG numeric tuple has the wrong arity")); }
    Ok(values)
}

fn valid_svg_id(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && value.len() <= 64
        && chars.all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
}

fn valid_svg_color(value: &str) -> bool {
    if matches!(value, "none" | "currentColor" | "black" | "white" | "red" | "green" | "blue" | "transparent") {
        return true;
    }
    let Some(hex) = value.strip_prefix('#') else { return false; };
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_svg_transform(path: &str, source: &str) -> Result<(), ThemeValidationError> {
    let mut rest = source.trim();
    if rest.is_empty() { return Err(media_error(path, "SVG transform is empty")); }
    while !rest.is_empty() {
        let open = rest.find('(').ok_or_else(|| media_error(path, "SVG transform syntax is invalid"))?;
        let name = rest[..open].trim();
        let close = rest[open + 1..].find(')').map(|index| open + 1 + index)
            .ok_or_else(|| media_error(path, "SVG transform syntax is invalid"))?;
        let args = parse_svg_numeric_list(path, &rest[open + 1..close], false)?;
        let valid_arity = match name {
            "matrix" => args.len() == 6,
            "translate" | "scale" => matches!(args.len(), 1 | 2),
            "rotate" => matches!(args.len(), 1 | 3),
            "skewX" | "skewY" => args.len() == 1,
            _ => false,
        };
        if !valid_arity { return Err(media_error(path, "SVG transform operation or arity is not allowed")); }
        rest = rest[close + 1..].trim_start();
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum SvgPathToken { Command(u8), Number(f64) }

fn validate_svg_path_data(path: &str, source: &str) -> Result<(), ThemeValidationError> {
    if source.is_empty() || source.len() > MAX_SVG_PATH_CHARS {
        return Err(media_error(path, "SVG path data is empty or exceeds v1 bounds"));
    }
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        if bytes[offset].is_ascii_whitespace() || bytes[offset] == b',' { offset += 1; continue; }
        if bytes[offset].is_ascii_alphabetic() {
            if !matches!(bytes[offset], b'M' | b'm' | b'L' | b'l' | b'H' | b'h' | b'V' | b'v' |
                b'C' | b'c' | b'S' | b's' | b'Q' | b'q' | b'T' | b't' | b'A' | b'a' | b'Z' | b'z')
            {
                return Err(media_error(path, "SVG path command is outside the static subset"));
            }
            tokens.push(SvgPathToken::Command(bytes[offset]));
            offset += 1;
            continue;
        }
        let start = offset;
        if matches!(bytes[offset], b'+' | b'-') { offset += 1; }
        let mut digits = 0usize;
        while offset < bytes.len() && bytes[offset].is_ascii_digit() { digits += 1; offset += 1; }
        if offset < bytes.len() && bytes[offset] == b'.' {
            offset += 1;
            while offset < bytes.len() && bytes[offset].is_ascii_digit() { digits += 1; offset += 1; }
        }
        if digits == 0 { return Err(media_error(path, "SVG path number is malformed")); }
        if offset < bytes.len() && matches!(bytes[offset], b'e' | b'E') {
            offset += 1;
            if offset < bytes.len() && matches!(bytes[offset], b'+' | b'-') { offset += 1; }
            let exponent_start = offset;
            while offset < bytes.len() && bytes[offset].is_ascii_digit() { offset += 1; }
            if offset == exponent_start { return Err(media_error(path, "SVG path exponent is malformed")); }
        }
        let number = source[start..offset].parse::<f64>()
            .map_err(|_| media_error(path, "SVG path number is malformed"))?;
        if !number.is_finite() || number.abs() > 1_000_000.0 {
            return Err(media_error(path, "SVG path number exceeds its finite bound"));
        }
        tokens.push(SvgPathToken::Number(number));
        if tokens.len() > 16_384 { return Err(media_error(path, "SVG path token budget exceeded")); }
    }
    let mut index = 0usize;
    let mut command = None;
    while index < tokens.len() {
        if let SvgPathToken::Command(next) = tokens[index] {
            command = Some(next);
            index += 1;
            if matches!(next, b'Z' | b'z') { command = None; continue; }
        }
        let current = command.ok_or_else(|| media_error(path, "SVG path numbers have no command"))?;
        let arity = match current.to_ascii_uppercase() {
            b'M' | b'L' | b'T' => 2,
            b'H' | b'V' => 1,
            b'S' | b'Q' => 4,
            b'C' => 6,
            b'A' => 7,
            _ => return Err(media_error(path, "SVG path command arity is invalid")),
        };
        let start = index;
        while index < tokens.len() && matches!(tokens[index], SvgPathToken::Number(_)) { index += 1; }
        let count = index - start;
        if count == 0 || count % arity != 0 || (matches!(current, b'M' | b'm') && count < 2) {
            return Err(media_error(path, "SVG path command has incomplete coordinates"));
        }
        if matches!(current, b'A' | b'a') {
            for group in tokens[start..index].chunks_exact(7) {
                let values = group.iter().map(|token| match token { SvgPathToken::Number(value) => *value, _ => unreachable!() }).collect::<Vec<_>>();
                if values[0] < 0.0 || values[1] < 0.0 || !matches!(values[3], 0.0 | 1.0) || !matches!(values[4], 0.0 | 1.0) {
                    return Err(media_error(path, "SVG arc radii or flags are invalid"));
                }
            }
        }
    }
    Ok(())
}

fn validate_png_container(path: &str, bytes: &[u8]) -> Result<Option<(u32, u32)>, ThemeValidationError> {
    const SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 8 || &bytes[..8] != SIGNATURE { return Err(media_error(path, "PNG signature is invalid")); }
    let mut offset = 8usize;
    let mut chunks = 0usize;
    let mut saw_ihdr = false;
    let mut saw_idat = false;
    let mut saw_iend = false;
    let mut ended_idat = false;
    let mut dimensions = None;
    while offset < bytes.len() {
        chunks += 1;
        if chunks > MAX_IMAGE_CHUNKS || offset.checked_add(12).is_none() || offset + 12 > bytes.len() {
            return Err(media_error(path, "PNG chunk structure exceeds bounds"));
        }
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let kind = &bytes[offset + 4..offset + 8];
        let payload_start = offset + 8;
        let payload_end = payload_start.checked_add(length).ok_or_else(|| media_error(path, "PNG chunk length overflows usize"))?;
        let next = payload_end.checked_add(4).ok_or_else(|| media_error(path, "PNG CRC offset overflows usize"))?;
        if next > bytes.len() || !kind.iter().all(|byte| byte.is_ascii_alphabetic()) || !kind[2].is_ascii_uppercase() {
            return Err(media_error(path, "PNG chunk is truncated or has an invalid type"));
        }
        let crc = u32::from_be_bytes(bytes[payload_end..next].try_into().unwrap());
        if crc32fast::hash(&bytes[offset + 4..payload_end]) != crc { return Err(media_error(path, "PNG chunk CRC is invalid")); }
        if !saw_ihdr && kind != b"IHDR" { return Err(media_error(path, "PNG must begin with IHDR")); }
        match kind {
            b"IHDR" => {
                if saw_ihdr || length != 13 { return Err(media_error(path, "PNG IHDR is missing, duplicated or malformed")); }
                let width = u32::from_be_bytes(bytes[payload_start..payload_start + 4].try_into().unwrap());
                let height = u32::from_be_bytes(bytes[payload_start + 4..payload_start + 8].try_into().unwrap());
                if width == 0 || height == 0 || width as usize > MAX_IMAGE_DIMENSION || height as usize > MAX_IMAGE_DIMENSION {
                    return Err(media_error(path, "PNG dimensions exceed v1 bounds"));
                }
                dimensions = Some((width, height));
                saw_ihdr = true;
            }
            b"IDAT" => {
                if ended_idat { return Err(media_error(path, "PNG IDAT chunks must be contiguous")); }
                saw_idat = true;
            }
            b"acTL" | b"fcTL" | b"fdAT" => return Err(media_error(path, "animated PNG is not allowed in theme v1")),
            b"IEND" => {
                if length != 0 || saw_iend || !saw_idat || next != bytes.len() { return Err(media_error(path, "PNG IEND placement is invalid")); }
                saw_iend = true;
            }
            b"iCCP" | b"zTXt" | b"iTXt" => return Err(media_error(path, "compressed PNG metadata is not allowed")),
            _ if kind[0].is_ascii_uppercase() => return Err(media_error(path, "PNG contains an unknown critical chunk")),
            _ => {}
        }
        if saw_idat && kind != b"IDAT" { ended_idat = true; }
        offset = next;
        if saw_iend { break; }
    }
    if !saw_ihdr || !saw_idat || !saw_iend { return Err(media_error(path, "PNG is missing IHDR, IDAT or IEND")); }
    Ok(dimensions)
}

fn validate_jpeg_container(path: &str, bytes: &[u8]) -> Result<Option<(u32, u32)>, ThemeValidationError> {
    if bytes.len() < 4 || bytes[..2] != [0xff, 0xd8] { return Err(media_error(path, "JPEG SOI marker is invalid")); }
    let mut offset = 2usize;
    let mut saw_sof = false;
    let mut saw_sos = false;
    let mut scans = 0usize;
    let mut segments = 0usize;
    let mut dimensions = None;
    loop {
        if offset >= bytes.len() || bytes[offset] != 0xff { return Err(media_error(path, "JPEG marker stream is truncated")); }
        while offset < bytes.len() && bytes[offset] == 0xff { offset += 1; }
        if offset >= bytes.len() { return Err(media_error(path, "JPEG marker is truncated")); }
        let marker = bytes[offset]; offset += 1; segments += 1;
        if segments > MAX_IMAGE_CHUNKS || marker == 0 || marker == 0xd8 || (0xd0..=0xd7).contains(&marker) {
            return Err(media_error(path, "JPEG marker budget or placement is invalid"));
        }
        if marker == 0xd9 {
            if !saw_sof || !saw_sos || offset != bytes.len() { return Err(media_error(path, "JPEG must end at EOI after a complete scan")); }
            return Ok(dimensions);
        }
        if offset + 2 > bytes.len() { return Err(media_error(path, "JPEG segment length is truncated")); }
        let length = u16::from_be_bytes(bytes[offset..offset + 2].try_into().unwrap()) as usize;
        if length < 2 || offset.checked_add(length).is_none() || offset + length > bytes.len() { return Err(media_error(path, "JPEG segment length is invalid")); }
        let payload = offset + 2; let end = offset + length;
        if (0xe0..=0xef).contains(&marker) && (bytes[payload..end].starts_with(b"MPF\0") || bytes[payload..end].starts_with(b"Exif\0\0") || bytes[payload..end].starts_with(b"ICC_PROFILE\0")) {
            return Err(media_error(path, "JPEG multi-picture and decoder-dependent metadata are not allowed"));
        }
        if marker == 0xda {
            let components = *bytes.get(payload).ok_or_else(|| media_error(path, "JPEG scan header is truncated"))? as usize;
            scans += 1;
            if !saw_sof || components == 0 || components > 4 || length != 6 + components * 2 || scans > 128 { return Err(media_error(path, "JPEG scan header is invalid")); }
            saw_sos = true; offset = end; scan_jpeg_entropy(path, bytes, &mut offset)?; continue;
        }
        if is_jpeg_sof(marker) {
            let components = *bytes.get(payload + 5).ok_or_else(|| media_error(path, "JPEG frame header is truncated"))? as usize;
            if saw_sof || !matches!(marker, 0xc0 | 0xc2) || components == 0 || components > 4 || length != 8 + components * 3 || bytes[payload] != 8 { return Err(media_error(path, "JPEG frame header is invalid")); }
            let height = u16::from_be_bytes(bytes[payload + 1..payload + 3].try_into().unwrap()) as u32;
            let width = u16::from_be_bytes(bytes[payload + 3..payload + 5].try_into().unwrap()) as u32;
            if width == 0 || height == 0 || width as usize > MAX_IMAGE_DIMENSION || height as usize > MAX_IMAGE_DIMENSION { return Err(media_error(path, "JPEG dimensions exceed v1 bounds")); }
            dimensions = Some((width, height)); saw_sof = true;
        }
        offset = end;
    }
}

fn scan_jpeg_entropy(path: &str, bytes: &[u8], offset: &mut usize) -> Result<(), ThemeValidationError> {
    while *offset < bytes.len() {
        if bytes[*offset] != 0xff { *offset += 1; continue; }
        let marker_start = *offset;
        while *offset < bytes.len() && bytes[*offset] == 0xff { *offset += 1; }
        if *offset >= bytes.len() { return Err(media_error(path, "JPEG entropy-coded scan is truncated")); }
        let marker = bytes[*offset];
        if marker == 0 || (0xd0..=0xd7).contains(&marker) { *offset += 1; continue; }
        *offset = marker_start; return Ok(());
    }
    Err(media_error(path, "JPEG scan has no terminating marker"))
}

fn is_jpeg_sof(marker: u8) -> bool { matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) }

struct FontSummary { tables: usize, glyphs: usize, units_per_em: u16 }
struct FontRecord { tag: [u8; 4], offset: usize, padded_end: usize, length: usize }

fn validate_true_type(path: &str, bytes: &[u8]) -> Result<FontSummary, ThemeValidationError> {
    if bytes.is_empty() || bytes.len() > MAX_FONT_BYTES || bytes.len() < 12 { return Err(font_error(path, "font byte budget or offset table bounds exceeded")); }
    if be_u32(bytes, 0, path)? != 0x0001_0000 { return Err(font_error(path, "only static TrueType outlines are allowed")); }
    let count = be_u16(bytes, 4, path)? as usize;
    if count == 0 || count > MAX_FONT_TABLES { return Err(font_error(path, "font table count is outside v1 bounds")); }
    let directory_end = 12usize.checked_add(count.checked_mul(16).ok_or_else(|| font_error(path, "font directory length overflows usize"))?).ok_or_else(|| font_error(path, "font directory length overflows usize"))?;
    if directory_end > bytes.len() { return Err(font_error(path, "font table directory is truncated")); }
    let power = 1usize << count.ilog2();
    if be_u16(bytes, 6, path)? as usize != power * 16 || be_u16(bytes, 8, path)? as usize != count.ilog2() as usize || be_u16(bytes, 10, path)? as usize != count * 16 - power * 16 { return Err(font_error(path, "font offset table search fields are invalid")); }
    let mut records = Vec::with_capacity(count);
    for index in 0..count {
        let start = 12 + index * 16; let tag: [u8; 4] = bytes[start..start + 4].try_into().unwrap();
        if !tag.iter().all(|byte| byte.is_ascii_graphic() || *byte == b' ') || records.last().is_some_and(|record: &FontRecord| record.tag >= tag) { return Err(font_error(path, "font table tags are invalid or unsorted")); }
        let checksum = be_u32(bytes, start + 4, path)?; let offset = be_u32(bytes, start + 8, path)? as usize; let length = be_u32(bytes, start + 12, path)? as usize;
        let padded = length.checked_add(3).map(|value| value & !3).ok_or_else(|| font_error(path, "font table padded length overflows usize"))?;
        let end = offset.checked_add(length).ok_or_else(|| font_error(path, "font table range overflows usize"))?; let padded_end = offset.checked_add(padded).ok_or_else(|| font_error(path, "font table padded range overflows usize"))?;
        if length == 0 || offset < directory_end || offset % 4 != 0 || end > bytes.len() || padded_end > bytes.len() { return Err(font_error(path, "font table range is invalid")); }
        if font_checksum(&bytes[offset..end], tag == *b"head") != checksum || bytes[end..padded_end].iter().any(|byte| *byte != 0) { return Err(font_error(path, "font table checksum or padding is invalid")); }
        if records.iter().any(|record: &FontRecord| offset < record.padded_end && record.offset < padded_end) { return Err(font_error(path, "font tables overlap")); }
        records.push(FontRecord { tag, offset, padded_end, length });
    }
    let mut by_offset = records.iter().collect::<Vec<_>>(); by_offset.sort_by_key(|record| record.offset); let mut previous_end = directory_end;
    for record in by_offset { if bytes[previous_end..record.offset].iter().any(|byte| *byte != 0) { return Err(font_error(path, "font has nonzero unregistered bytes")); } previous_end = record.padded_end; }
    if previous_end != bytes.len() || font_checksum(bytes, false) != 0xb1b0_afba { return Err(font_error(path, "font whole-file checksum or trailing padding is invalid")); }
    let unsupported: [[u8; 4]; 17] = [*b"CFF ", *b"CFF2", *b"COLR", *b"CPAL", *b"SVG ", *b"sbix", *b"CBDT", *b"CBLC", *b"EBDT", *b"EBLC", *b"fvar", *b"gvar", *b"avar", *b"HVAR", *b"MVAR", *b"VVAR", *b"cvar"];
    if unsupported.iter().any(|tag| records.iter().any(|record| record.tag == *tag)) { return Err(font_error(path, "CFF, variable, color and bitmap font tables are not allowed")); }
    let required: [[u8; 4]; 7] = [*b"head", *b"maxp", *b"hhea", *b"hmtx", *b"loca", *b"glyf", *b"cmap"];
    for required in required { if !records.iter().any(|record| record.tag == required) { return Err(font_error(path, format!("required font table {} is missing", String::from_utf8_lossy(&required)))); } }
    let head = font_table_bytes(path, bytes, &records, b"head")?;
    let maxp = font_table_bytes(path, bytes, &records, b"maxp")?;
    let hhea = font_table_bytes(path, bytes, &records, b"hhea")?;
    let hmtx = font_table_bytes(path, bytes, &records, b"hmtx")?;
    let loca = font_table_bytes(path, bytes, &records, b"loca")?;
    let glyf = font_table_bytes(path, bytes, &records, b"glyf")?;
    if head.len() < 54 || be_u32(head, 0, path)? != 0x0001_0000 || be_u32(head, 12, path)? != 0x5f0f_3cf5 { return Err(font_error(path, "head table is invalid")); }
    let units = be_u16(head, 18, path)?; let loca_format = be_i16(head, 50, path)?;
    if !(16..=16_384).contains(&units) || !matches!(loca_format, 0 | 1) || be_i16(head, 52, path)? != 0 { return Err(font_error(path, "head metrics or loca format is invalid")); }
    if maxp.len() < 6 || be_u32(maxp, 0, path)? != 0x0001_0000 { return Err(font_error(path, "maxp table is not a TrueType glyph table")); }
    let glyphs = be_u16(maxp, 4, path)? as usize; if glyphs == 0 || glyphs > MAX_FONT_GLYPHS { return Err(font_error(path, "font glyph count is outside v1 bounds")); }
    if hhea.len() < 36 || be_u32(hhea, 0, path)? != 0x0001_0000 || be_i16(hhea, 32, path)? != 0 { return Err(font_error(path, "hhea table is invalid")); }
    let metrics = be_u16(hhea, 34, path)? as usize;
    if metrics == 0 || metrics > glyphs { return Err(font_error(path, "hhea numberOfLongHorMetrics is invalid")); }
    let required_hmtx = metrics.checked_mul(4).and_then(|value| value.checked_add((glyphs - metrics).checked_mul(2)?)).ok_or_else(|| font_error(path, "hmtx length overflows usize"))?;
    if hmtx.len() < required_hmtx { return Err(font_error(path, "hmtx table is truncated")); }
    let offsets = parse_loca(path, loca, glyf.len(), glyphs, loca_format)?;
    let face = Face::parse(bytes, 0).map_err(|error| font_error(path, format!("TrueType parser rejected the face: {error:?}")))?;
    let parsed = face.tables(); if parsed.glyf.is_none() || parsed.hmtx.is_none() || parsed.cmap.is_none() || face.is_variable() { return Err(font_error(path, "TrueType parser did not accept the required static tables")); }
    for (tag, parsed_ok) in [
        (*b"name", parsed.name.is_some()), (*b"OS/2", parsed.os2.is_some()),
        (*b"post", parsed.post.is_some()), (*b"GDEF", parsed.gdef.is_some()),
        (*b"GPOS", parsed.gpos.is_some()), (*b"GSUB", parsed.gsub.is_some()),
        (*b"STAT", parsed.stat.is_some()), (*b"kern", parsed.kern.is_some()),
        (*b"vhea", parsed.vhea.is_some()), (*b"vmtx", parsed.vmtx.is_some()),
    ] {
        if records.iter().any(|record| record.tag == tag) && !parsed_ok {
            return Err(font_error(path, format!("TrueType parser omitted malformed optional table {}", String::from_utf8_lossy(&tag))));
        }
    }
    validate_cmap(path, parsed.cmap.as_ref().unwrap(), glyphs)?; validate_glyphs(path, &face, glyf, &offsets, glyphs)?;
    Ok(FontSummary { tables: count, glyphs, units_per_em: units })
}

fn parse_loca(path: &str, table: &[u8], glyf_len: usize, glyphs: usize, format: i16) -> Result<Vec<usize>, ThemeValidationError> {
    let width = if format == 0 { 2 } else { 4 }; let required = (glyphs + 1).checked_mul(width).ok_or_else(|| font_error(path, "loca length overflows usize"))?; if table.len() < required { return Err(font_error(path, "loca table is truncated")); }
    let mut offsets = Vec::with_capacity(glyphs + 1); let mut previous = 0usize;
    for index in 0..=glyphs { let value = if width == 2 { (be_u16(table, index * 2, path)? as usize) * 2 } else { be_u32(table, index * 4, path)? as usize }; if value < previous || value > glyf_len { return Err(font_error(path, "loca glyph range is invalid")); } offsets.push(value); previous = value; }
    Ok(offsets)
}

fn font_table_bytes<'a>(
    path: &str,
    bytes: &'a [u8],
    records: &[FontRecord],
    tag: &[u8; 4],
) -> Result<&'a [u8], ThemeValidationError> {
    let record = records
        .iter()
        .find(|record| record.tag == *tag)
        .ok_or_else(|| font_error(path, "required font table is missing"))?;
    Ok(&bytes[record.offset..record.offset + record.length])
}

#[derive(Default)]
struct OutlineBudget { events: usize, total: usize, finite: bool }
impl OutlineBudget { fn point(&mut self, x: f32, y: f32) { self.events += 1; self.total += 1; self.finite &= x.is_finite() && y.is_finite() && x.abs() <= 1_000_000.0 && y.abs() <= 1_000_000.0; } }
impl OutlineBuilder for OutlineBudget {
    fn move_to(&mut self, x: f32, y: f32) { self.point(x, y); }
    fn line_to(&mut self, x: f32, y: f32) { self.point(x, y); }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) { self.point(x1, y1); self.point(x, y); }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) { self.point(x1, y1); self.point(x2, y2); self.point(x, y); }
    fn close(&mut self) { self.events += 1; }
}

fn validate_glyphs(path: &str, face: &Face<'_>, glyf: &[u8], offsets: &[usize], glyphs: usize) -> Result<(), ThemeValidationError> {
    let mut budget = OutlineBudget { finite: true, ..Default::default() };
    for index in 0..glyphs {
        let start = offsets[index]; let end = offsets[index + 1]; if start == end { continue; }
        if end - start < 10 { return Err(font_error(path, "glyph header is truncated")); }
        let contours = be_i16(glyf, start, path)?; if contours < -1 { return Err(font_error(path, "glyph numberOfContours is invalid")); }
        let before = budget.events; let outline = face.outline_glyph(GlyphId(index as u16), &mut budget);
        if (contours != 0 && outline.is_none()) || !budget.finite || budget.events - before > MAX_GLYPH_EVENTS || budget.total > MAX_TOTAL_GLYPH_EVENTS { return Err(font_error(path, "glyph outline is malformed or exceeds the traversal budget")); }
    }
    Ok(())
}

fn validate_cmap(path: &str, cmap: &ttf_parser::cmap::Table<'_>, glyphs: usize) -> Result<(), ThemeValidationError> {
    if cmap.subtables.len() == 0 || cmap.subtables.len() as usize > MAX_CMAP_SUBTABLES { return Err(font_error(path, "cmap subtable count is invalid")); }
    let mut unicode = false;
    for index in 0..cmap.subtables.len() {
        let subtable = cmap.subtables.get(index).ok_or_else(|| font_error(path, "cmap subtable failed structural parsing"))?; unicode |= subtable.is_unicode(); let mut checked = 0usize; let mut invalid = false;
        subtable.codepoints(|codepoint| { checked += 1; if checked > MAX_CMAP_CODEPOINTS || subtable.glyph_index(codepoint).is_some_and(|id| id.0 as usize >= glyphs) { invalid = true; } });
        if invalid { return Err(font_error(path, "cmap is malformed or references a missing glyph")); }
    }
    if !unicode { return Err(font_error(path, "cmap has no Unicode subtable")); }
    Ok(())
}

fn font_checksum(bytes: &[u8], zero_adjustment: bool) -> u32 {
    let mut sum = 0u32;
    let chunks = bytes.chunks_exact(4);
    let rem = chunks.remainder();
    for (index, chunk) in chunks.enumerate() {
        if zero_adjustment && index == 2 {
            continue;
        }
        sum = sum.wrapping_add(u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    if !rem.is_empty() {
        let mut word = [0u8; 4];
        word[..rem.len()].copy_from_slice(rem);
        sum = sum.wrapping_add(u32::from_be_bytes(word));
    }
    sum
}
fn be_u16(bytes: &[u8], offset: usize, path: &str) -> Result<u16, ThemeValidationError> { let end = offset.checked_add(2).ok_or_else(|| font_error(path, "font offset overflows usize"))?; let slice = bytes.get(offset..end).ok_or_else(|| font_error(path, "font table field is truncated"))?; Ok(u16::from_be_bytes(slice.try_into().unwrap())) }
fn be_i16(bytes: &[u8], offset: usize, path: &str) -> Result<i16, ThemeValidationError> { Ok(be_u16(bytes, offset, path)? as i16) }
fn be_u32(bytes: &[u8], offset: usize, path: &str) -> Result<u32, ThemeValidationError> { let end = offset.checked_add(4).ok_or_else(|| font_error(path, "font offset overflows usize"))?; let slice = bytes.get(offset..end).ok_or_else(|| font_error(path, "font table field is truncated"))?; Ok(u32::from_be_bytes(slice.try_into().unwrap())) }

pub(crate) fn validate_bubble_pixels(theme: &ThemeSpecV1, media: &[ThemeMediaValidation]) -> Result<(), ThemeValidationError> {
    for image in [
        &theme.as_json()["chat"]["user"]["image"],
        &theme.as_json()["chat"]["ai"]["image"],
        &theme.as_json()["input"]["image"],
    ] {
        if image.is_null() { continue; }
        let path = image["asset"].as_str().expect("checked bubble image asset");
        let (width, height) = media.iter().find_map(|entry| match entry {
            ThemeMediaValidation::Image { path: candidate, width, height, .. }
            | ThemeMediaValidation::SvgImage { path: candidate, width, height, .. } if candidate == path => Some((*width, *height)),
            _ => None,
        }).ok_or_else(|| media_error(path, "bubble image has no validated dimensions"))?;
        let crop = [image["crop"]["left"].as_f64().unwrap(), image["crop"]["top"].as_f64().unwrap(), image["crop"]["right"].as_f64().unwrap(), image["crop"]["bottom"].as_f64().unwrap()];
        for (axis, length, leading, trailing) in [("x", width, crop[0], crop[2]), ("y", height, crop[1], crop[3])] {
            let start = (length as f64 * leading).round() as i64;
            let end = (length as f64 * (1.0 - trailing)).round() as i64;
            let cropped = end - start;
            if start < 0 || cropped < 3 || end > length as i64 {
                return Err(media_error(path, "bubble crop must leave at least three source pixels on each axis"));
            }
            let slice_start = (cropped as f64 * image["slice"][format!("{axis}Start")].as_f64().unwrap()).round() as i64;
            let slice_end = (cropped as f64 * image["slice"][format!("{axis}End")].as_f64().unwrap()).round() as i64;
            if slice_start < 1 || slice_end <= slice_start || slice_end >= cropped {
                return Err(media_error(path, "rounded nine-slice caps and center must each contain source pixels"));
            }
        }
    }
    Ok(())
}

fn font_error(path: &str, message: impl Into<String>) -> ThemeValidationError { ThemeValidationError::new("theme-font-invalid", ValidationStage::Media, path, message) }
fn media_error(path: &str, message: impl Into<String>) -> ThemeValidationError { ThemeValidationError::new("theme-media-invalid", ValidationStage::Media, path, message) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_builtin_fonts_pass_the_same_l4_walk_as_imported_fonts() {
        for (path, bytes) in [
            ("assets/fonts/cascadia-mono.ttf", include_bytes!("../../../../../apps/flutter/packages/operit_ui/assets/fonts/cascadia-mono.ttf").as_slice()),
            ("assets/fonts/noto-sans-sc.ttf", include_bytes!("../../../../../apps/flutter/packages/operit_ui/assets/fonts/noto-sans-sc.ttf").as_slice()),
        ] {
            let summary = validate_true_type(path, bytes).unwrap();
            assert!(summary.tables <= MAX_FONT_TABLES);
            assert!(summary.glyphs <= MAX_FONT_GLYPHS);
        }
    }
}
