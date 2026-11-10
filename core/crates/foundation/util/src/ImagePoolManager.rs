use operit_host_api::AtomicCounter::AtomicU64;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock};

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};

use crate::AppLogger::AppLogger;
use crate::ImageBitmapLimiter::{ImageBitmapLimiter, MAX_SOURCE_BYTES};

const TAG: &str = "ImagePoolManager";
pub(crate) const DEFAULT_SCALE_PERCENT: i32 = 100;
pub(crate) const DEFAULT_JPEG_QUALITY: i32 = 85;
pub(crate) const DEFAULT_MAX_LONG_EDGE: i32 = 2048;

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
    /// Matches the default Kotlin registration policy.
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
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

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
            AppLogger::v_with_level(
                TAG,
                &format!("pool size limit updated: {value}"),
                crate::AppLogger::VERBOSE_LEVEL_4,
            );
        }
    }

    /// Returns the maximum number of images retained in the pool.
    pub fn max_pool_size() -> usize {
        state()
            .lock()
            .expect("ImagePool mutex poisoned")
            .max_pool_size
    }

    /// Registers normalized model input; the encoded bytes determine the final MIME type.
    pub fn add_image_bytes(
        bytes: &[u8],
        _mime_type: Option<&str>,
        options: Option<ImageRegistrationOptions>,
    ) -> String {
        let image = match ImageBitmapLimiter::prepare(bytes, &options.unwrap_or_default()) {
            Ok(image) => image,
            Err(error) => {
                AppLogger::e(TAG, &error);
                return "error".to_string();
            }
        };
        Self::insert(ImageData {
            base64: encode_base64(&image.bytes),
            mime_type: image.format.to_mime_type().to_string(),
            width: image.width as i32,
            height: image.height as i32,
        })
    }

    /// Registers an image from base64 data.
    pub fn add_image_from_base64(
        base64: &str,
        mime_type: &str,
        options: Option<ImageRegistrationOptions>,
    ) -> String {
        match decode_image_base64(base64) {
            Ok(bytes) => Self::add_image_bytes(&bytes, Some(mime_type), options),
            Err(error) => {
                AppLogger::e(TAG, &error);
                "error".to_string()
            }
        }
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

/// Decodes raw Base64 or a Base64 data URL without accepting malformed payloads.
fn decode_image_base64(input: &str) -> Result<Vec<u8>, String> {
    let trimmed = input.trim();
    if trimmed.len() > MAX_SOURCE_BYTES.div_ceil(3) * 4 + 1024 {
        return Err("Image Base64 input exceeds 32 MiB".into());
    }
    let payload = if let Some(rest) = trimmed.strip_prefix("data:") {
        let (header, payload) = rest
            .split_once(',')
            .ok_or("Image data URL is missing its payload")?;
        if !header.split(';').any(|part| part == "base64") {
            return Err("Image data URL must use Base64 encoding".into());
        }
        payload
    } else {
        trimmed
    };
    let compact: Vec<u8> = payload
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    STANDARD
        .decode(compact)
        .map_err(|error| format!("Invalid image Base64: {error}"))
}

/// Allocates a process-unique pool identifier.
fn new_id() -> String {
    let millis = operit_host_api::TimeUtils::currentTimeMillisU128();
    let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    format!("{millis:x}-{sequence:x}")
}

/// Encodes media bytes as unwrapped standard Base64.
pub(crate) fn encode_base64(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

/// Decodes the legacy audio/video Base64 representation.
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

/// Maps one standard Base64 digit to its numeric value.
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

#[cfg(test)]
#[path = "ImagePoolManagerTests.rs"]
mod tests;
