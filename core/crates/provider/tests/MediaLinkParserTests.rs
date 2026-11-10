use super::MediaLinkParser;
use operit_util::ImagePoolManager::ImagePoolManager;
use operit_util::MediaPoolManager::MediaPoolManager;

/// Verifies image tags resolve through the image pool.
#[test]
fn extractImageLinksReadsRegisteredImageData() {
    let image_id = ImagePoolManager::add_image_bytes(
        include_bytes!("fixtures/pixel.png"),
        Some("image/png"),
        None,
    );
    let message = format!("before <link type=\"image\" id=\"{image_id}\"></link> after");

    let links = MediaLinkParser::extract_image_links(&message);

    assert_eq!(links.len(), 1);
    assert_eq!(links[0].id, image_id);
    assert_eq!(links[0].mime_type, "image/png");
    assert!(!links[0].base64_data.is_empty());
    ImagePoolManager::remove_image(&image_id);
}

/// Verifies self-closing tags are parsed and only image tags are removed.
#[test]
fn selfClosingImageTagsAreRecognizedAndRemovedSelectively() {
    let message = "a <link type=\"image\" id=\"img1\"/> b <link type=\"audio\" id=\"aud1\"></link>";

    assert_eq!(
        MediaLinkParser::extract_image_link_ids(message),
        vec!["img1".to_string()]
    );
    assert_eq!(
        MediaLinkParser::remove_image_links(message),
        "a  b <link type=\"audio\" id=\"aud1\"></link>"
    );
}

/// Verifies file links preserve their decoded filename and encounter order.
#[test]
fn fileLinksPreserveDecodedFilename() {
    let message = concat!(
        "<link filename=\"report&amp;one.pdf\" id=\"f1\" type=\"file\">",
        "PDF</link><link type=\"audio\" id=\"a1\">Audio</link>"
    );

    let tags = MediaLinkParser::extract_media_link_tags(message);

    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0].link_type, "file");
    assert_eq!(tags[0].id, "f1");
    assert_eq!(tags[0].file_name.as_deref(), Some("report&one.pdf"));
    assert_eq!(tags[1].link_type, "audio");
    assert_eq!(tags[1].file_name, None);
    assert_eq!(MediaLinkParser::remove_media_links(message), "");
}

/// Verifies error image tags are detected and removed without producing ids.
#[test]
fn errorImageTagsAreDetectedAndRemoved() {
    let message = "a <link type=\"image\" id=\"error\"></link> b";

    assert!(MediaLinkParser::has_image_links(message));
    assert!(MediaLinkParser::extract_image_link_ids(message).is_empty());
    assert_eq!(MediaLinkParser::remove_image_links(message), "a  b");
    assert_eq!(
        MediaLinkParser::replace_image_links(message, |_| "x".to_string()),
        "a  b"
    );
}

/// Verifies audio and video tags resolve through the media pool.
#[test]
fn extractMediaLinksReadsRegisteredMediaData() {
    let audio_id = MediaPoolManager::add_media_bytes(b"audio", "audio/mpeg");
    let video_id = MediaPoolManager::add_media_bytes(b"video", "video/mp4");
    let message = format!(
        "<link type=\"audio\" id=\"{audio_id}\"></link> <link type=\"video\" id=\"{video_id}\"/>"
    );

    let links = MediaLinkParser::extract_media_links(&message);

    assert_eq!(links.len(), 2);
    assert_eq!(links[0].id, audio_id);
    assert_eq!(links[0].mime_type, "audio/mpeg");
    assert_eq!(links[0].base64_data, "YXVkaW8=");
    assert_eq!(links[1].id, video_id);
    assert_eq!(links[1].mime_type, "video/mp4");
    assert_eq!(links[1].base64_data, "dmlkZW8=");
    MediaPoolManager::remove_media(&links[0].id);
    MediaPoolManager::remove_media(&links[1].id);
}
