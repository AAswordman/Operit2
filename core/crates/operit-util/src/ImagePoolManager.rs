use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::AppLogger::AppLogger;

const TAG: &str = "ImagePoolManager";
const DEFAULT_SCALE_PERCENT: i32 = 100;
const DEFAULT_JPEG_QUALITY: i32 = 85;
const DEFAULT_MAX_LONG_EDGE: i32 = 2048;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
/// Output format used when registering images for model input.
pub enum ImageOutputFormat {
    PNG,
    JPEG,
    AUTO,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
/// Options controlling image registration and preprocessing.
pub struct ImageRegistrationOptions {
    pub scale_percent: Option<i32>,
    pub output_format: ImageOutputFormat,
    pub jpeg_quality: Option<i32>,
    pub normalize_exif: bool,
    pub max_long_edge: Option<i32>,
}

impl Default for ImageRegistrationOptions {
    fn default() -> Self {
        Self {
            scale_percent: Some(DEFAULT_SCALE_PERCENT),
            output_format: ImageOutputFormat::AUTO,
            jpeg_quality: Some(DEFAULT_JPEG_QUALITY),
            normalize_exif: true,
            max_long_edge: Some(DEFAULT_MAX_LONG_EDGE),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
/// Registered image payload and metadata.
pub struct ImageData {
    pub base64: String,
    pub mime_type: String,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug)]
struct PoolState {
    max_pool_size: usize,
    image_pool: HashMap<String, ImageData>,
    order: VecDeque<String>,
}

impl Default for PoolState {
    fn default() -> Self {
        Self {
            max_pool_size: 20,
            image_pool: HashMap::new(),
            order: VecDeque::new(),
        }
    }
}

static STATE: OnceLock<Mutex<PoolState>> = OnceLock::new();

fn state() -> &'static Mutex<PoolState> {
    STATE.get_or_init(|| Mutex::new(PoolState::default()))
}

/// Process-wide image pool used by chat and tool execution.
pub struct ImagePoolManager;

impl ImagePoolManager {
    /// Returns default options used for image registration.
    pub fn default_registration_options() -> ImageRegistrationOptions {
        ImageRegistrationOptions::default()
    }

    /// Sets the maximum number of images kept in memory.
    pub fn set_max_pool_size(value: usize) {
        if value > 0 {
            state()
                .lock()
                .expect("ImagePool mutex poisoned")
                .max_pool_size = value;
            AppLogger::d(TAG, &format!("pool size limit updated: {value}"));
        }
    }

    /// Registers image bytes for model input.
    pub fn add_image_bytes(
        bytes: &[u8],
        mime_type: Option<&str>,
        _options: Option<ImageRegistrationOptions>,
    ) -> String {
        if bytes.is_empty() {
            AppLogger::e(TAG, "image bytes must not be empty");
            return "error".to_string();
        }
        let mime_type = mime_type_from_bytes(&bytes)
            .or_else(|| mime_type.map(str::to_string))
            .unwrap_or_else(|| "image/png".to_string());
        let (width, height) = image_dimensions(&bytes).unwrap_or((0, 0));
        Self::insert(ImageData {
            base64: encode_base64(&bytes),
            mime_type,
            width,
            height,
        })
    }

    /// Registers an image from base64 data.
    pub fn add_image_from_base64(
        base64: &str,
        mime_type: &str,
        _options: Option<ImageRegistrationOptions>,
    ) -> String {
        let (normalized_base64, normalized_mime) = normalize_base64_input(base64, mime_type);
        let bytes = decode_base64(&normalized_base64);
        let (width, height) = image_dimensions(&bytes).unwrap_or((0, 0));
        Self::insert(ImageData {
            base64: encode_base64(&bytes),
            mime_type: mime_type_from_bytes(&bytes).unwrap_or(normalized_mime),
            width,
            height,
        })
    }

    /// Returns registered image data by id.
    pub fn get_image(id: &str) -> Option<ImageData> {
        if let Some(data) = state()
            .lock()
            .expect("ImagePool mutex poisoned")
            .image_pool
            .get(id)
            .cloned()
        {
            return Some(data);
        }
        None
    }

    /// Returns the MIME type for a registered image.
    pub fn get_image_mime_type(id: &str) -> Option<String> {
        Self::get_image(id).map(|image| image.mime_type)
    }

    /// Removes one image from the pool.
    pub fn remove_image(id: &str) {
        let mut guard = state().lock().expect("ImagePool mutex poisoned");
        guard.image_pool.remove(id);
        guard.order.retain(|item| item != id);
    }

    /// Clears all images from the pool and cache directory.
    pub fn clear() {
        let mut guard = state().lock().expect("ImagePool mutex poisoned");
        guard.image_pool.clear();
        guard.order.clear();
    }

    /// Returns the current number of registered images.
    pub fn size() -> usize {
        state()
            .lock()
            .expect("ImagePool mutex poisoned")
            .image_pool
            .len()
    }

    fn insert(data: ImageData) -> String {
        let id = new_id();
        let mut guard = state().lock().expect("ImagePool mutex poisoned");
        touch_locked(&mut guard, &id);
        guard.image_pool.insert(id.clone(), data.clone());
        trim_locked(&mut guard);
        id
    }
}

fn touch_locked(state: &mut PoolState, id: &str) {
    state.order.retain(|item| item != id);
    state.order.push_back(id.to_string());
}

fn trim_locked(state: &mut PoolState) {
    while state.image_pool.len() > state.max_pool_size {
        if let Some(id) = state.order.pop_front() {
            state.image_pool.remove(&id);
        } else {
            break;
        }
    }
}

fn normalize_base64_input(base64: &str, mime_type: &str) -> (String, String) {
    let trimmed = base64.trim();
    if let Some(rest) = trimmed.strip_prefix("data:") {
        if let Some(comma) = rest.find(',') {
            let header = &rest[..comma];
            let header_mime = header.split(';').next().unwrap_or("").trim();
            return (
                rest[comma + 1..].to_string(),
                if header_mime.is_empty() {
                    mime_type_or_png(mime_type)
                } else {
                    header_mime.to_string()
                },
            );
        }
    }
    (trimmed.to_string(), mime_type_or_png(mime_type))
}

fn mime_type_or_png(mime_type: &str) -> String {
    if mime_type.trim().is_empty() {
        "image/png".to_string()
    } else {
        mime_type.to_string()
    }
}

fn mime_type_from_bytes(bytes: &[u8]) -> Option<String> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png".to_string())
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg".to_string())
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif".to_string())
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp".to_string())
    } else {
        None
    }
}

fn image_dimensions(bytes: &[u8]) -> Option<(i32, i32)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") && bytes.len() >= 24 {
        let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as i32;
        let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]) as i32;
        return Some((width, height));
    }
    jpeg_dimensions(bytes)
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(i32, i32)> {
    let mut index = 2;
    while index + 9 < bytes.len() {
        if bytes[index] != 0xFF {
            index += 1;
            continue;
        }
        let marker = bytes[index + 1];
        let length = u16::from_be_bytes([bytes[index + 2], bytes[index + 3]]) as usize;
        if matches!(marker, 0xC0 | 0xC1 | 0xC2 | 0xC3) {
            let height = u16::from_be_bytes([bytes[index + 5], bytes[index + 6]]) as i32;
            let width = u16::from_be_bytes([bytes[index + 7], bytes[index + 8]]) as i32;
            return Some((width, height));
        }
        index += 2 + length;
    }
    None
}

fn new_id() -> String {
    let millis = operit_host_api::TimeUtils::currentTimeMillisU128();
    format!("{millis:x}")
}

pub(crate) fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut index = 0;
    while index < bytes.len() {
        let b0 = bytes[index];
        let b1 = *bytes.get(index + 1).unwrap_or(&0);
        let b2 = *bytes.get(index + 2).unwrap_or(&0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0b11) << 4) | (b1 >> 4)) as usize] as char);
        if index + 1 < bytes.len() {
            out.push(TABLE[(((b1 & 0b1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if index + 2 < bytes.len() {
            out.push(TABLE[(b2 & 0b111111) as usize] as char);
        } else {
            out.push('=');
        }
        index += 3;
    }
    out
}

pub(crate) fn decode_base64(input: &str) -> Vec<u8> {
    let mut buffer = 0u32;
    let mut bits = 0u8;
    let mut out = Vec::new();
    for byte in input.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        if byte == b'=' {
            break;
        }
        let Some(value) = base64_value(byte) else {
            continue;
        };
        buffer = (buffer << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    out
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}
