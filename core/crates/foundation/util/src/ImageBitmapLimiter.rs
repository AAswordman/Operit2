//! Pure image preprocessing shared by every model-input registration path.
use std::io::Cursor;

use image::{
    codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage, ImageDecoder, ImageFormat,
    ImageReader, RgbImage, RgbaImage,
};

use crate::ImagePoolManager::{
    ImageOutputFormat, ImageRegistrationOptions, DEFAULT_JPEG_QUALITY, DEFAULT_MAX_LONG_EDGE,
    DEFAULT_SCALE_PERCENT,
};

pub(crate) const MAX_SOURCE_BYTES: usize = 32 * 1024 * 1024;
const MAX_PIXELS: u64 = 64 * 1024 * 1024;
const MAX_DECODE_BYTES: u64 = 256 * 1024 * 1024;

/// Encoded model input whose metadata describes the final pixels and bytes.
pub(crate) struct PreparedImage {
    pub bytes: Vec<u8>,
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Default)]
/// Stateless preprocessing over image bytes, independent of hosts and providers.
pub struct ImageBitmapLimiter;

impl ImageBitmapLimiter {
    /// Decodes, orients, resizes, and encodes a single still image for model input.
    pub(crate) fn prepare(
        bytes: &[u8],
        options: &ImageRegistrationOptions,
    ) -> Result<PreparedImage, String> {
        if bytes.is_empty() || bytes.len() > MAX_SOURCE_BYTES {
            return Err("Image input must contain 1 byte to 32 MiB".into());
        }
        let mut image = decode(bytes, options.normalize_exif)?;
        let scale = options
            .scale_percent
            .unwrap_or(DEFAULT_SCALE_PERCENT)
            .clamp(1, 100) as u64;
        let mut width = (u64::from(image.width()) * scale / 100).max(1) as u32;
        let mut height = (u64::from(image.height()) * scale / 100).max(1) as u32;
        let limit = options.max_long_edge.unwrap_or(DEFAULT_MAX_LONG_EDGE);
        if limit > 0 {
            let long_edge = width.max(height);
            if long_edge > limit as u32 {
                width = (u64::from(width) * limit as u64 / u64::from(long_edge)).max(1) as u32;
                height = (u64::from(height) * limit as u64 / u64::from(long_edge)).max(1) as u32;
            }
        }
        if (width, height) != (image.width(), image.height()) {
            image = image.resize_exact(width, height, FilterType::Triangle);
        }
        let format = match options.output_format {
            ImageOutputFormat::PNG => ImageFormat::Png,
            ImageOutputFormat::JPEG => ImageFormat::Jpeg,
            ImageOutputFormat::AUTO => {
                if image.has_alpha() {
                    ImageFormat::Png
                } else {
                    ImageFormat::Jpeg
                }
            }
        };
        let mut output = Cursor::new(Vec::new());
        if format == ImageFormat::Jpeg {
            let rgb = flatten_on_white(image);
            let quality = options
                .jpeg_quality
                .unwrap_or(DEFAULT_JPEG_QUALITY)
                .clamp(1, 100) as u8;
            JpegEncoder::new_with_quality(&mut output, quality)
                .encode_image(&rgb)
                .map_err(|error| format!("Cannot encode JPEG image: {error}"))?;
        } else {
            DynamicImage::ImageRgba8(image.into_rgba8())
                .write_to(&mut output, format)
                .map_err(|error| format!("Cannot encode PNG image: {error}"))?;
        }
        Ok(PreparedImage {
            bytes: output.into_inner(),
            format,
            width,
            height,
        })
    }
}

/// Selects the decoder by file content, without relying on the supplied MIME type.
fn decode(bytes: &[u8], normalize_exif: bool) -> Result<DynamicImage, String> {
    if bytes.get(4..8) == Some(b"ftyp") {
        let info =
            heic_rs::probe(bytes).map_err(|error| format!("Cannot inspect HEIF image: {error}"))?;
        let layout = if info.has_alpha {
            heic_rs::PixelLayout::Rgba8
        } else {
            heic_rs::PixelLayout::Rgb8
        };
        let options = heic_rs::DecodeOptions::default()
            .with_layout(layout)
            .with_transforms(normalize_exif)
            .with_max_pixels(Some(MAX_PIXELS));
        let decoded = heic_rs::decode(bytes, &options)
            .map_err(|error| format!("Cannot decode HEIF image: {error}"))?;
        return match layout {
            heic_rs::PixelLayout::Rgba8 => {
                RgbaImage::from_raw(decoded.width, decoded.height, decoded.data)
                    .map(DynamicImage::ImageRgba8)
            }
            _ => RgbImage::from_raw(decoded.width, decoded.height, decoded.data)
                .map(DynamicImage::ImageRgb8),
        }
        .ok_or_else(|| "Invalid HEIF pixel buffer".into());
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| format!("Cannot identify image format: {error}"))?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|error| format!("Cannot open image decoder: {error}"))?;
    let (width, height) = decoder.dimensions();
    if width == 0
        || height == 0
        || u64::from(width) * u64::from(height) > MAX_PIXELS
        || decoder.total_bytes() > MAX_DECODE_BYTES
    {
        return Err("Image exceeds the 64 megapixel or 256 MiB decode limit".into());
    }
    let orientation = if normalize_exif {
        Some(
            decoder
                .orientation()
                .map_err(|error| format!("Cannot read image orientation: {error}"))?,
        )
    } else {
        None
    };
    let mut image = DynamicImage::from_decoder(decoder)
        .map_err(|error| format!("Cannot decode image: {error}"))?;
    if let Some(orientation) = orientation {
        image.apply_orientation(orientation);
    }
    Ok(image)
}

/// Composites alpha on white before JPEG encoding, matching the Kotlin pipeline.
fn flatten_on_white(image: DynamicImage) -> RgbImage {
    if !image.has_alpha() {
        return image.into_rgb8();
    }
    let rgba = image.into_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let pixel = rgba.get_pixel(x, y);
        let alpha = u32::from(pixel[3]);
        image::Rgb(std::array::from_fn(|channel| {
            ((u32::from(pixel[channel]) * alpha + 255 * (255 - alpha) + 127) / 255) as u8
        }))
    })
}
