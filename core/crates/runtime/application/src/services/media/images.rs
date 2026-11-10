//! Device-independent image transformations over the process-wide image pool.
//! Callers must authorize access before requesting a preview. No chat or Edge types live here.
use base64::{engine::general_purpose::STANDARD, Engine};
use image::ImageReader;
use operit_util::ImagePoolManager::ImagePoolManager;
use std::{
    collections::VecDeque,
    io::Cursor,
    sync::{Arc, Mutex, OnceLock},
};

const CACHE_ENTRIES: usize = 4;
const MAX_SOURCE_BYTES: usize = 8 * 1024 * 1024;

/// Uses the HEIF library to prepare pixels for the existing image delivery service.
pub(crate) fn heif_display_bytes(bytes: &[u8]) -> Result<Vec<u8>, String> {
    use image::ImageEncoder;

    let options = heic_rs::DecodeOptions::default()
        .with_layout(heic_rs::PixelLayout::Rgba8)
        .with_max_pixels(Some(64 * 1024 * 1024));
    let decoded = heic_rs::decode(bytes, &options)
        .map_err(|error| format!("Cannot decode HEIF image: {error}"))?;
    let context = heic_rs::context::Context::open(bytes)
        .map_err(|error| format!("Cannot read HEIF metadata: {error}"))?;
    let properties = context.props(context.meta.primary)
        .map_err(|error| format!("Cannot read HEIF image properties: {error}"))?;
    let mut png = Vec::new();
    let mut encoder = image::codecs::png::PngEncoder::new(&mut png);
    if let Some(profile) = context.icc(&properties) {
        encoder.set_icc_profile(profile.to_vec())
            .map_err(|error| format!("Cannot preserve HEIF color profile: {error}"))?;
    }
    encoder.write_image(
        &decoded.data,
        decoded.width,
        decoded.height,
        image::ExtendedColorType::Rgba8,
    ).map_err(|error| format!("Cannot encode HEIF display image: {error}"))?;
    Ok(png)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb565Le,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewOptions {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
}

impl PreviewOptions {
    fn validate(self) -> Result<(), String> {
        if self.width == 0 || self.height == 0 || self.width > 512 || self.height > 512 {
            return Err("Preview dimensions must be within 1..=512".into());
        }
        Ok(())
    }
}

pub struct PreviewChunk {
    pub width: usize,
    pub height: usize,
    pub offset: usize,
    pub bytes: Vec<u8>,
}

struct PreparedImage {
    id: String,
    options: PreviewOptions,
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

// Shared across chat runtime slots, like ImagePoolManager itself. Bounded to 2 MiB
// of cached pixel data. Decoding never holds this mutex.
static CACHE: OnceLock<Mutex<VecDeque<Arc<PreparedImage>>>> = OnceLock::new();

pub fn preview_chunk(
    id: &str,
    options: PreviewOptions,
    offset: usize,
    chunk_size: usize,
) -> Result<PreviewChunk, String> {
    options.validate()?;
    if chunk_size == 0
        || chunk_size > 16 * 1024
        || chunk_size % 2 != 0
        || offset % 2 != 0
        || offset >= options.width as usize * options.height as usize * 2
    {
        return Err("Invalid preview chunk range".into());
    }
    // Check the source even on a cache hit: expired images must not stay readable.
    let source = ImagePoolManager::get_image(id).ok_or("原图已过期，请重新发送图片")?;
    let cache = CACHE.get_or_init(|| Mutex::new(VecDeque::new()));
    let cached = {
        let cache = cache.lock().map_err(|_| "图片预览缓存不可用")?;
        cache
            .iter()
            .find(|image| image.id == id && image.options == options)
            .cloned()
    };
    let image = if let Some(image) = cached {
        image
    } else {
        if source.base64.len() > MAX_SOURCE_BYTES.div_ceil(3) * 4 {
            return Err("原图超过 8 MiB 预览限制".into());
        }
        let bytes = STANDARD
            .decode(&source.base64)
            .map_err(|_| "图片数据无效")?;
        let (width, height, pixels) = prepare(&bytes, options)?;
        let image = Arc::new(PreparedImage {
            id: id.into(),
            options,
            width,
            height,
            pixels,
        });
        let mut cache = cache.lock().map_err(|_| "图片预览缓存不可用")?;
        // Concurrent requests may prepare the same image; keep only one cache entry.
        cache.retain(|entry| entry.id != id || entry.options != options);
        while cache.len() >= CACHE_ENTRIES {
            cache.pop_front();
        }
        cache.push_back(image.clone());
        image
    };
    if offset >= image.pixels.len() {
        return Err("图片分块越界".into());
    }
    let end = (offset + chunk_size).min(image.pixels.len());
    Ok(PreviewChunk {
        width: image.width,
        height: image.height,
        offset,
        bytes: image.pixels[offset..end].to_vec(),
    })
}

/// Validate an upload and return its opaque pool ID, not chat markup.
pub fn register(bytes: &[u8], mime: &str) -> Result<String, String> {
    if !matches!(mime, "image/png" | "image/jpeg") || bytes.is_empty() || bytes.len() > 512 * 1024 {
        return Err("只支持小于 512 KiB 的 PNG/JPEG 图片".into());
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| "图片格式无效")?;
    if reader.format()
        != Some(if mime == "image/png" {
            image::ImageFormat::Png
        } else {
            image::ImageFormat::Jpeg
        })
    {
        return Err("图片类型和实际内容不符".into());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().map_err(|_| "图片损坏，无法读取")?;
    if image.width() > 8192 || image.height() > 8192 {
        return Err("图片尺寸超过 8192 × 8192".into());
    }
    let id = ImagePoolManager::add_image_bytes(bytes, Some(mime), None);
    if id == "error" {
        return Err("图片注册失败".into());
    }
    Ok(id)
}

fn prepare(bytes: &[u8], options: PreviewOptions) -> Result<(usize, usize, Vec<u8>), String> {
    if bytes.is_empty() || bytes.len() > MAX_SOURCE_BYTES {
        return Err("图片为空或超过预览限制".into());
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|_| "图片损坏或格式不受支持（支持 PNG/JPEG）")?
        .thumbnail(options.width, options.height)
        .to_rgba8();
    let mut pixels = Vec::with_capacity(image.width() as usize * image.height() as usize * 2);
    for pixel in image.pixels() {
        let a = pixel[3] as u16;
        let channel = |i: usize| (pixel[i] as u16 * a + 255 * (255 - a)) / 255;
        let rgb = ((channel(0) >> 3) << 11) | ((channel(1) >> 2) << 5) | (channel(2) >> 3);
        pixels.extend_from_slice(&rgb.to_le_bytes());
    }
    Ok((image.width() as usize, image.height() as usize, pixels))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png() -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(256, 128, image::Rgba([255, 0, 0, 255]));
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        bytes.into_inner()
    }

    #[test]
    fn previews_are_parameterized_bounded_and_expire_with_the_source() {
        let bytes = png();
        let id = register(&bytes, "image/png").unwrap();
        let options = PreviewOptions {
            width: 128,
            height: 96,
            format: PixelFormat::Rgb565Le,
        };
        let first = preview_chunk(&id, options, 0, 1024).unwrap();
        assert_eq!(
            (first.width, first.height, first.bytes.len()),
            (128, 64, 1024)
        );
        assert_eq!(&first.bytes[..2], &[0, 248]);
        let other = preview_chunk(
            &id,
            PreviewOptions {
                width: 32,
                height: 32,
                ..options
            },
            0,
            1024,
        )
        .unwrap();
        assert_eq!((other.width, other.height), (32, 16));
        let again = preview_chunk(&id, options, 1024, 1024).unwrap();
        assert_eq!((again.width, again.offset), (128, 1024));
        let last = preview_chunk(&id, options, 16382, 1024).unwrap();
        assert_eq!(last.bytes.len(), 2);
        for (offset, size) in [(1, 1024), (0, 0), (0, 3), (0, 16386), (16384, 1024)] {
            assert!(preview_chunk(&id, options, offset, size).is_err());
        }
        for width in [0, 513, u32::MAX] {
            assert!(preview_chunk(&id, PreviewOptions { width, ..options }, 0, 1024).is_err());
        }
        ImagePoolManager::remove_image(&id);
        assert!(preview_chunk(&id, options, 0, 1024).is_err());
    }

    #[test]
    fn uploads_validate_content_and_return_ids_not_markup() {
        let bytes = png();
        assert!(register(&bytes, "image/jpeg").is_err());
        assert!(register(b"bad image", "image/png").is_err());
        assert!(register(&vec![0; 512 * 1024 + 1], "image/png").is_err());
        let id = register(&bytes, "image/png").unwrap();
        assert!(!id.contains('<'));
        assert_eq!(
            ImagePoolManager::get_image(&id).unwrap().mime_type,
            "image/png"
        );
        ImagePoolManager::remove_image(&id);
    }

    #[test]
    fn transparent_pixels_are_composited_on_white() {
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 0]));
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        let (_, _, pixels) = prepare(
            bytes.get_ref(),
            PreviewOptions {
                width: 1,
                height: 1,
                format: PixelFormat::Rgb565Le,
            },
        )
        .unwrap();
        assert_eq!(pixels, [255, 255]);
    }
}
