use super::*;
use image::{
    codecs::jpeg::JpegEncoder, DynamicImage, ImageEncoder, ImageFormat, RgbImage, RgbaImage,
};
use std::io::Cursor;

/// Encodes real pixels so registration tests exercise complete image decoding.
fn encode(image: &DynamicImage, format: ImageFormat) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, format).unwrap();
    bytes.into_inner()
}

/// Retrieves and removes one entry without clearing other tests' pool entries.
fn take(id: &str) -> ImageData {
    assert_ne!(id, "error");
    let image = ImagePoolManager::get_image(id).unwrap();
    ImagePoolManager::remove_image(id);
    let bytes = STANDARD.decode(&image.base64).unwrap();
    let format = image::guess_format(&bytes).unwrap();
    assert_eq!(image.mime_type, format.to_mime_type());
    let decoded = image::load_from_memory(&bytes).unwrap();
    assert_eq!(
        (image.width, image.height),
        (decoded.width() as i32, decoded.height() as i32)
    );
    image
}

/// Keeps transparent pixels intact and ignores an incorrect source MIME declaration.
#[test]
fn transparent_input_is_reencoded_as_png() {
    let pixels =
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 3, image::Rgba([30, 60, 90, 100])));
    let bytes = encode(&pixels, ImageFormat::Png);
    let image = take(&ImagePoolManager::add_image_bytes(
        &bytes,
        Some("image/jpg"),
        None,
    ));
    assert_eq!(image.mime_type, "image/png");
    assert_eq!(
        image::load_from_memory(&STANDARD.decode(image.base64).unwrap())
            .unwrap()
            .into_rgba8(),
        pixels.into_rgba8()
    );
}

/// Shrinks an oversized photograph and sends a valid JPEG with canonical MIME.
#[test]
fn opaque_input_is_limited_to_2048_and_reencoded_as_jpeg() {
    let pixels =
        DynamicImage::ImageRgb8(RgbImage::from_pixel(3072, 1536, image::Rgb([40, 80, 120])));
    let bytes = encode(&pixels, ImageFormat::Bmp);
    let image = take(&ImagePoolManager::add_image_bytes(
        &bytes,
        Some("image/bmp"),
        None,
    ));
    assert_eq!((image.width, image.height), (2048, 1024));
    assert_eq!(image.mime_type, "image/jpeg");
    assert!(STANDARD.decode(image.base64).unwrap().len() < bytes.len() / 10);
}

/// Applies scaling and an explicit output format to both registration entry points.
#[test]
fn base64_and_bytes_share_options_and_output() {
    let pixels = DynamicImage::ImageRgb8(RgbImage::from_pixel(80, 40, image::Rgb([20, 40, 60])));
    let bytes = encode(&pixels, ImageFormat::Png);
    let options = ImageRegistrationOptions {
        scale_percent: Some(50),
        max_long_edge: Some(20),
        output_format: ImageOutputFormat::PNG,
        ..Default::default()
    };
    let binary = take(&ImagePoolManager::add_image_bytes(
        &bytes,
        None,
        Some(options.clone()),
    ));
    let data_url = format!("data:image/jpeg;base64,\n{}\n", STANDARD.encode(bytes));
    let base64 = take(&ImagePoolManager::add_image_from_base64(
        &data_url,
        "image/jpeg",
        Some(options),
    ));
    assert_eq!(binary, base64);
    assert_eq!((binary.width, binary.height), (20, 10));
    assert_eq!(binary.mime_type, "image/png");
}

/// Converts common non-provider image formats into a decodable PNG or JPEG.
#[test]
fn gif_webp_bmp_and_ico_are_normalized() {
    for format in [
        ImageFormat::Gif,
        ImageFormat::WebP,
        ImageFormat::Bmp,
        ImageFormat::Ico,
    ] {
        let pixels =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 2, image::Rgba([0, 255, 0, 128])));
        let bytes = encode(&pixels, format);
        let image = take(&ImagePoolManager::add_image_bytes(
            &bytes,
            Some(format.to_mime_type()),
            None,
        ));
        assert!(matches!(
            image.mime_type.as_str(),
            "image/png" | "image/jpeg"
        ));
        assert_eq!((image.width, image.height), (4, 2));
    }
}

/// Converts a real HEVC still image to JPEG without using a platform codec.
#[test]
fn heif_input_is_normalized_for_providers() {
    let bytes = include_bytes!("../tests/fixtures/heif-checker-64.heic");
    let image = take(&ImagePoolManager::add_image_bytes(
        bytes,
        Some("image/heic"),
        None,
    ));
    assert_eq!(image.mime_type, "image/jpeg");
    assert_eq!((image.width, image.height), (64, 64));
    assert_eq!(
        ImagePoolManager::add_image_bytes(&bytes[..32], Some("image/heic"), None),
        "error"
    );
}

/// Flattens transparent pixels on white when JPEG is explicitly requested.
#[test]
fn forced_jpeg_composites_transparency_on_white() {
    let pixels = DynamicImage::ImageRgba8(RgbaImage::from_pixel(8, 8, image::Rgba([255, 0, 0, 0])));
    let options = ImageRegistrationOptions {
        output_format: ImageOutputFormat::JPEG,
        ..Default::default()
    };
    let image = take(&ImagePoolManager::add_image_bytes(
        &encode(&pixels, ImageFormat::Png),
        None,
        Some(options),
    ));
    let rgb = image::load_from_memory(&STANDARD.decode(image.base64).unwrap())
        .unwrap()
        .into_rgb8();
    assert!(rgb
        .pixels()
        .all(|pixel| pixel.0.iter().all(|channel| *channel >= 250)));
}

/// Honors JPEG quality rather than silently ignoring registration options.
#[test]
fn jpeg_quality_controls_encoded_size() {
    let pixels = DynamicImage::ImageRgb8(RgbImage::from_fn(64, 64, |x, y| {
        image::Rgb([
            (x * 17 + y * 31) as u8,
            (x * 43 + y) as u8,
            (x + y * 73) as u8,
        ])
    }));
    let bytes = encode(&pixels, ImageFormat::Png);
    let sizes: Vec<_> = [10, 95]
        .into_iter()
        .map(|quality| {
            let options = ImageRegistrationOptions {
                jpeg_quality: Some(quality),
                ..Default::default()
            };
            take(&ImagePoolManager::add_image_bytes(
                &bytes,
                None,
                Some(options),
            ))
            .base64
            .len()
        })
        .collect();
    assert!(sizes[0] < sizes[1]);
}

/// Rotates JPEG pixels exactly once and strips orientation metadata from the output.
#[test]
fn exif_orientation_is_normalized_and_can_be_disabled() {
    let pixels = RgbImage::from_fn(16, 8, |x, _| {
        if x < 8 {
            image::Rgb([255, 0, 0])
        } else {
            image::Rgb([0, 0, 255])
        }
    });
    let mut exif = b"II\x2a\x00\x08\x00\x00\x00\x01\x00\x12\x01\x03\x00\x01\x00\x00\x00\x06\x00\x00\x00\x00\x00\x00\x00".to_vec();
    for orientation in 1..=8 {
        exif[18] = orientation;
        let mut bytes = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut bytes, 95);
        encoder.set_exif_metadata(exif.clone()).unwrap();
        encoder.encode_image(&pixels).unwrap();
        let options = ImageRegistrationOptions {
            output_format: ImageOutputFormat::PNG,
            ..Default::default()
        };
        let normalized = take(&ImagePoolManager::add_image_bytes(
            &bytes,
            None,
            Some(options.clone()),
        ));
        let mut expected = image::load_from_memory(&bytes).unwrap();
        expected.apply_orientation(image::metadata::Orientation::from_exif(orientation).unwrap());
        let actual = image::load_from_memory(&STANDARD.decode(normalized.base64).unwrap()).unwrap();
        assert_eq!(actual.into_rgba8(), expected.into_rgba8());
        let unchanged = take(&ImagePoolManager::add_image_bytes(
            &bytes,
            None,
            Some(ImageRegistrationOptions {
                normalize_exif: false,
                ..options
            }),
        ));
        assert_eq!((unchanged.width, unchanged.height), (16, 8));
    }
}

/// Rejects corrupt images, incomplete headers, and malformed Base64 before pooling.
#[test]
fn invalid_images_are_not_registered() {
    for bytes in [
        b"not an image".as_slice(),
        b"\x89PNG\r\n\x1a\n",
        b"\xff\xd8\xff",
        b"",
    ] {
        assert_eq!(
            ImagePoolManager::add_image_bytes(bytes, Some("image/png"), None),
            "error"
        );
    }
    for base64 in [
        "!!!!",
        "aW1hZ2U=",
        "data:image/png,AAAA",
        "data:image/png;base64",
        "",
    ] {
        assert_eq!(
            ImagePoolManager::add_image_from_base64(base64, "image/png", None),
            "error"
        );
    }
}

/// Refuses oversized input and decoded dimensions before allocating full pixel buffers.
#[test]
fn oversized_inputs_are_rejected_before_decoding() {
    assert_eq!(
        ImagePoolManager::add_image_bytes(&vec![0; 32 * 1024 * 1024 + 1], None, None),
        "error"
    );
    // A BMP header can declare huge dimensions while its pixel data remains tiny.
    let pixels = DynamicImage::ImageRgb8(RgbImage::new(1, 1));
    let mut bytes = encode(&pixels, ImageFormat::Bmp);
    bytes[18..22].copy_from_slice(&8193i32.to_le_bytes());
    bytes[22..26].copy_from_slice(&8193i32.to_le_bytes());
    assert_eq!(
        ImagePoolManager::add_image_bytes(&bytes, None, None),
        "error"
    );
}
