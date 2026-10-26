//! Registers image, audio, and video bytes in the process pools used by media links.
use crate::ImagePoolManager::{decode_base64, ImagePoolManager};
use crate::MediaPoolManager::MediaPoolManager;

const MAX_AV_BYTES: usize = 20 * 1024 * 1024;

/// One pool entry that can be turned into a canonical media link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredMedia {
    pub id: String,
    pub mime_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

/// Registers already decoded media bytes.
pub fn register_media_bytes(
    kind: &str,
    bytes: &[u8],
    mime_type: Option<&str>,
) -> Result<RegisteredMedia, String> {
    if bytes.is_empty() {
        return Err("media bytes must not be empty".to_string());
    }
    match kind {
        "image" => register_image_bytes(bytes, mime_type),
        "audio" | "video" => register_av_bytes(kind, bytes, required_mime(kind, mime_type)?),
        _ => Err("media type must be image, audio, or video".to_string()),
    }
}

/// Registers base64 media, including an optional data URL prefix.
pub fn register_media_base64(
    kind: &str,
    base64: &str,
    mime_type: Option<&str>,
) -> Result<RegisteredMedia, String> {
    let (bytes, header_mime) = decode_media_base64(base64)?;
    let mime = mime_type
        .filter(|value| !value.trim().is_empty())
        .or(header_mime.as_deref());
    register_media_bytes(kind, &bytes, mime)
}

fn register_image_bytes(bytes: &[u8], mime_type: Option<&str>) -> Result<RegisteredMedia, String> {
    let id = ImagePoolManager::add_image_bytes(bytes, mime_type, None);
    if id == "error" {
        return Err("image registration failed".to_string());
    }
    let image = ImagePoolManager::get_image(&id).ok_or("image registration failed")?;
    Ok(RegisteredMedia {
        id,
        mime_type: image.mime_type,
        width: Some(image.width),
        height: Some(image.height),
    })
}

fn register_av_bytes(kind: &str, bytes: &[u8], mime_type: &str) -> Result<RegisteredMedia, String> {
    let prefix = format!("{kind}/");
    if !mime_type.starts_with(&prefix) {
        return Err(format!("{kind} MIME type must start with {prefix}"));
    }
    if bytes.len() > MAX_AV_BYTES {
        return Err(format!("{kind} input is too large"));
    }
    let id = MediaPoolManager::add_media_bytes(bytes, mime_type);
    if id == "error" {
        return Err(format!("{kind} registration failed"));
    }
    let media = MediaPoolManager::get_media(&id).ok_or(format!("{kind} registration failed"))?;
    Ok(RegisteredMedia {
        id,
        mime_type: media.mime_type,
        width: None,
        height: None,
    })
}

fn required_mime<'a>(kind: &str, mime_type: Option<&'a str>) -> Result<&'a str, String> {
    mime_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{kind} registration requires a MIME type"))
}

fn decode_media_base64(input: &str) -> Result<(Vec<u8>, Option<String>), String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("media bytes must not be empty".to_string());
    }
    let (payload, header_mime) = if let Some(rest) = trimmed.strip_prefix("data:") {
        let (header, data) = rest
            .split_once(',')
            .ok_or("media data URL is missing its payload")?;
        let mime = header.split(';').next().unwrap_or("").trim();
        (
            data,
            if mime.is_empty() {
                None
            } else {
                Some(mime.to_string())
            },
        )
    } else {
        (trimmed, None)
    };
    let bytes = decode_base64(payload);
    if bytes.is_empty() {
        return Err("media bytes must not be empty".to_string());
    }
    Ok((bytes, header_mime))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png() -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR".to_vec();
        bytes.extend_from_slice(&2u32.to_be_bytes());
        bytes.extend_from_slice(&3u32.to_be_bytes());
        bytes
    }

    #[test]
    fn image_registration_keeps_detected_type_and_dimensions() {
        let registered = register_media_bytes("image", &png(), None).expect("png");
        assert_eq!(registered.mime_type, "image/png");
        assert_eq!(registered.width, Some(2));
        assert_eq!(registered.height, Some(3));
        assert_ne!(registered.id, "error");
        ImagePoolManager::remove_image(&registered.id);
    }

    #[test]
    fn image_data_url_uses_its_payload() {
        let payload = encode_png_base64();
        let registered = register_media_base64(
            "image",
            &format!("data:image/png;base64,{payload}"),
            None,
        )
        .expect("data url");
        assert_eq!(registered.mime_type, "image/png");
        ImagePoolManager::remove_image(&registered.id);
    }

    #[test]
    fn audio_and_video_require_matching_mime_types() {
        let audio = register_media_bytes("audio", b"audio", Some("audio/mpeg")).expect("audio");
        assert_eq!(audio.mime_type, "audio/mpeg");
        assert_eq!(audio.width, None);
        MediaPoolManager::remove_media(&audio.id);
        let video = register_media_base64("video", "dmlkZW8=", Some("video/mp4")).expect("video");
        assert_eq!(video.mime_type, "video/mp4");
        MediaPoolManager::remove_media(&video.id);
        assert!(register_media_bytes("audio", b"audio", None)
            .unwrap_err()
            .contains("MIME"));
        assert!(register_media_bytes("video", b"video", Some("audio/mpeg"))
            .unwrap_err()
            .contains("video/"));
        assert!(register_media_bytes("file", b"x", None)
            .unwrap_err()
            .contains("image, audio, or video"));
        assert!(register_media_base64("image", "   ", None)
            .unwrap_err()
            .contains("must not be empty"));
    }

    fn encode_png_base64() -> String {
        crate::ImagePoolManager::encode_base64(&png())
    }
}
