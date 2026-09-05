use std::sync::OnceLock;

use operit_util::ImagePoolManager::ImagePoolManager;
use regex::Regex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaLink {
    pub link_type: String,
    pub id: String,
    pub base64_data: String,
    pub mime_type: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageLink {
    pub link_type: String,
    pub id: String,
    pub base64_data: String,
    pub mime_type: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaLinkTag {
    pub link_type: String,
    pub id: String,
}

pub struct MediaLinkParser;

impl MediaLinkParser {
    /// Extracts registered image payloads referenced by media-link tags.
    pub fn extract_image_links(message: &str) -> Vec<ImageLink> {
        let mut links = Vec::new();
        for tag in Self::extract_media_link_tags(message)
            .into_iter()
            .filter(|tag| tag.link_type == "image")
        {
            let Some(image_data) = ImagePoolManager::get_image(&tag.id) else {
                continue;
            };
            links.push(ImageLink {
                link_type: tag.link_type,
                id: tag.id,
                base64_data: image_data.base64,
                mime_type: image_data.mime_type,
            });
        }
        links
    }

    /// Extracts image link ids in first-seen order.
    pub fn extract_image_link_ids(message: &str) -> Vec<String> {
        Self::extract_media_link_tags(message)
            .into_iter()
            .filter(|tag| tag.link_type == "image")
            .map(|tag| tag.id)
            .collect()
    }

    /// Removes image media-link tags while preserving other media-link tags.
    pub fn remove_image_links(message: &str) -> String {
        Self::replace_links(message, |tag| {
            if tag.link_type == "image" {
                String::new()
            } else {
                tag.raw.to_string()
            }
        })
    }

    /// Replaces image media-link tags while preserving other media-link tags.
    pub fn replace_image_links(message: &str, replacer: impl Fn(&str) -> String) -> String {
        Self::replace_links(message, |tag| {
            if tag.link_type == "image" {
                if tag.id == "error" {
                    String::new()
                } else {
                    replacer(&tag.id)
                }
            } else {
                tag.raw.to_string()
            }
        })
    }

    /// Reports whether the message contains at least one image media-link tag.
    pub fn has_image_links(message: &str) -> bool {
        parsed_link_tags(message)
            .iter()
            .any(|tag| tag.link_type == "image")
    }

    /// Extracts non-image media payload references from recognized link tags.
    pub fn extract_media_links(message: &str) -> Vec<MediaLink> {
        Self::extract_media_link_tags(message)
            .into_iter()
            .filter(|tag| tag.link_type == "audio" || tag.link_type == "video")
            .map(|tag| MediaLink {
                link_type: tag.link_type,
                id: tag.id,
                base64_data: String::new(),
                mime_type: String::new(),
            })
            .collect()
    }

    /// Extracts recognized media-link tags in first-seen order.
    pub fn extract_media_link_tags(message: &str) -> Vec<MediaLinkTag> {
        let mut tags = Vec::new();
        let mut seen = Vec::<(String, String)>::new();
        for tag in parsed_link_tags(message) {
            if tag.id == "error"
                || !matches!(tag.link_type.as_str(), "image" | "audio" | "video")
                || seen
                    .iter()
                    .any(|(seen_type, seen_id)| seen_type == &tag.link_type && seen_id == &tag.id)
            {
                continue;
            }
            seen.push((tag.link_type.clone(), tag.id.clone()));
            tags.push(MediaLinkTag {
                link_type: tag.link_type,
                id: tag.id,
            });
        }
        tags
    }

    /// Replaces non-image media-link tags using the supplied transformer.
    pub fn replace_media_links(message: &str, replacer: impl Fn(&str, &str) -> String) -> String {
        Self::replace_links(message, |tag| {
            if tag.link_type == "audio" || tag.link_type == "video" {
                if tag.id == "error" {
                    String::new()
                } else {
                    replacer(&tag.link_type, &tag.id)
                }
            } else {
                tag.raw.to_string()
            }
        })
    }

    /// Removes non-image media-link tags while preserving image media-link tags.
    pub fn remove_media_links(message: &str) -> String {
        Self::replace_media_links(message, |_, _| String::new())
    }

    /// Reports whether the message contains audio or video media-link tags.
    pub fn has_media_links(message: &str) -> bool {
        parsed_link_tags(message)
            .iter()
            .any(|tag| tag.link_type == "audio" || tag.link_type == "video")
    }

    /// Replaces recognized link tags in one pass.
    fn replace_links(message: &str, replacer: impl Fn(&ParsedLinkTag<'_>) -> String) -> String {
        let mut result = String::new();
        let mut cursor = 0;
        for tag in parsed_link_tags(message) {
            result.push_str(&message[cursor..tag.start]);
            result.push_str(&replacer(&tag));
            cursor = tag.end;
        }
        result.push_str(&message[cursor..]);
        result
    }
}

struct ParsedLinkTag<'a> {
    raw: &'a str,
    start: usize,
    end: usize,
    link_type: String,
    id: String,
}

/// Returns the compiled regex used to find link start tags.
fn link_start_tag_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?is)<link\b[^>]*>"#).expect("media-link start tag regex must compile")
    })
}

/// Returns the compiled regex used to find link close tags.
fn link_close_tag_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?is)</link\s*>"#).expect("media-link close tag regex must compile")
    })
}

/// Returns the compiled regex used to read link tag attributes.
fn link_attr_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?i)\b([A-Za-z_:-]+)\s*=\s*\\*["']?([^"'\\\s>]+)\\*["']?"#)
            .expect("media-link attribute regex must compile")
    })
}

/// Parses complete link tags and their relevant attributes.
fn parsed_link_tags(message: &str) -> Vec<ParsedLinkTag<'_>> {
    let mut tags = Vec::new();
    let mut cursor = 0;
    while let Some(open_tag) = link_start_tag_regex().find_at(message, cursor) {
        let open_text = open_tag.as_str();
        let raw_end = if is_self_closing_link_start(open_text) {
            open_tag.end()
        } else {
            let Some(close_tag) = link_close_tag_regex().find_at(message, open_tag.end()) else {
                cursor = open_tag.end();
                continue;
            };
            close_tag.end()
        };
        let raw = &message[open_tag.start()..raw_end];
        cursor = raw_end;
        let Some((link_type, id)) = parse_link_tag(open_text) else {
            continue;
        };
        tags.push(ParsedLinkTag {
            raw,
            start: open_tag.start(),
            end: raw_end,
            link_type,
            id,
        });
    }
    tags
}

/// Reports whether a link start tag is self-closing.
fn is_self_closing_link_start(tag_text: &str) -> bool {
    tag_text
        .trim_end()
        .trim_end_matches('>')
        .trim_end()
        .ends_with('/')
}

/// Parses the type and id attributes from one link tag.
fn parse_link_tag(tag_text: &str) -> Option<(String, String)> {
    let mut link_type = None;
    let mut id = None;
    for capture in link_attr_regex().captures_iter(tag_text) {
        let name = capture.get(1)?.as_str().to_ascii_lowercase();
        let value = capture
            .get(2)?
            .as_str()
            .trim_end_matches('/')
            .trim_end_matches('\\')
            .to_string();
        match name.as_str() {
            "type" => link_type = Some(value.to_ascii_lowercase()),
            "id" => id = Some(value),
            _ => {}
        }
    }
    Some((link_type?, id?))
}

#[cfg(test)]
mod tests {
    use super::MediaLinkParser;
    use operit_util::ImagePoolManager::ImagePoolManager;

    /// Verifies image tags resolve through the image pool.
    #[test]
    fn extractImageLinksReadsRegisteredImageData() {
        ImagePoolManager::clear();
        let image_id = ImagePoolManager::add_image_bytes(
            b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR\x00\x00\x00\x01\x00\x00\x00\x01",
            Some("image/png"),
            None,
        );
        let message = format!("before <link type=\"image\" id=\"{image_id}\"></link> after");

        let links = MediaLinkParser::extract_image_links(&message);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].id, image_id);
        assert_eq!(links[0].mime_type, "image/png");
        assert!(!links[0].base64_data.is_empty());
    }

    /// Verifies self-closing tags are parsed and only image tags are removed.
    #[test]
    fn selfClosingImageTagsAreRecognizedAndRemovedSelectively() {
        let message =
            "a <link type=\"image\" id=\"img1\"/> b <link type=\"audio\" id=\"aud1\"></link>";

        assert_eq!(
            MediaLinkParser::extract_image_link_ids(message),
            vec!["img1".to_string()]
        );
        assert_eq!(
            MediaLinkParser::remove_image_links(message),
            "a  b <link type=\"audio\" id=\"aud1\"></link>"
        );
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
}
