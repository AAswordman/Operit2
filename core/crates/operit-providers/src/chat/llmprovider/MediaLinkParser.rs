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
    pub fn extract_image_links(message: &str) -> Vec<ImageLink> {
        Self::extract_media_links(message)
            .into_iter()
            .filter(|link| link.link_type == "image")
            .map(|link| ImageLink {
                link_type: link.link_type,
                id: link.id,
                base64_data: link.base64_data,
                mime_type: link.mime_type,
            })
            .collect()
    }

    pub fn extract_image_link_ids(message: &str) -> Vec<String> {
        Self::extract_media_link_tags(message)
            .into_iter()
            .filter(|tag| tag.link_type == "image")
            .map(|tag| tag.id)
            .collect()
    }

    pub fn remove_image_links(message: &str) -> String {
        Self::remove_media_links(message)
    }

    pub fn replace_image_links(message: &str, replacer: impl Fn(&str) -> String) -> String {
        Self::replace_media_links(message, |link_type, id| {
            if link_type == "image" {
                replacer(id)
            } else {
                format!("<link type=\"{}\" id=\"{}\"></link>", link_type, id)
            }
        })
    }

    pub fn has_image_links(message: &str) -> bool {
        Self::has_media_links(message)
            && Self::extract_media_link_tags(message)
                .iter()
                .any(|tag| tag.link_type == "image")
    }

    pub fn extract_media_links(message: &str) -> Vec<MediaLink> {
        Self::extract_media_link_tags(message)
            .into_iter()
            .map(|tag| MediaLink {
                link_type: tag.link_type,
                id: tag.id,
                base64_data: String::new(),
                mime_type: String::new(),
            })
            .collect()
    }

    pub fn extract_media_link_tags(message: &str) -> Vec<MediaLinkTag> {
        let mut tags = Vec::new();
        let mut seen = Vec::<(String, String)>::new();
        let mut cursor = 0;
        while let Some(start_rel) = message[cursor..].find("<link") {
            let start = cursor + start_rel;
            let end = match message[start..].find("</link>") {
                Some(value) => start + value + "</link>".len(),
                None => break,
            };
            let tag_text = &message[start..end];
            if let Some((link_type, id)) = parse_link_tag(tag_text) {
                if id != "error"
                    && matches!(link_type.as_str(), "image" | "audio" | "video")
                    && !seen
                        .iter()
                        .any(|(seen_type, seen_id)| seen_type == &link_type && seen_id == &id)
                {
                    seen.push((link_type.clone(), id.clone()));
                    tags.push(MediaLinkTag { link_type, id });
                }
            }
            cursor = end;
        }
        tags
    }

    pub fn replace_media_links(message: &str, replacer: impl Fn(&str, &str) -> String) -> String {
        let mut result = String::new();
        let mut cursor = 0;
        while let Some(start_rel) = message[cursor..].find("<link") {
            let start = cursor + start_rel;
            result.push_str(&message[cursor..start]);
            let end = match message[start..].find("</link>") {
                Some(value) => start + value + "</link>".len(),
                None => {
                    result.push_str(&message[start..]);
                    return result;
                }
            };
            let tag_text = &message[start..end];
            if let Some((link_type, id)) = parse_link_tag(tag_text) {
                result.push_str(&replacer(&link_type, &id));
            } else {
                result.push_str(tag_text);
            }
            cursor = end;
        }
        result.push_str(&message[cursor..]);
        result
    }

    pub fn remove_media_links(message: &str) -> String {
        Self::replace_media_links(message, |_, _| String::new())
    }

    pub fn has_media_links(message: &str) -> bool {
        message.contains("<link") && message.contains("</link>")
    }
}

fn parse_link_tag(tag_text: &str) -> Option<(String, String)> {
    let type_value = extract_attr(tag_text, "type")?;
    let id_value = extract_attr(tag_text, "id")?;
    Some((type_value, id_value))
}

fn extract_attr(source: &str, attribute_name: &str) -> Option<String> {
    let attr_start = source.find(attribute_name)? + attribute_name.len();
    let after_name = source[attr_start..].trim_start();
    let after_equals = after_name.strip_prefix('=')?.trim_start();
    let after_escape = after_equals.strip_prefix('\\').unwrap_or(after_equals);
    let quote = after_escape.chars().next()?;
    match quote {
        '"' | '\'' => {
            let body = &after_escape[quote.len_utf8()..];
            let end = body.find(quote)?;
            Some(body[..end].trim_end_matches('\\').to_string())
        }
        _ => {
            let end = after_escape
                .find(|ch: char| ch.is_whitespace() || ch == '>')
                .unwrap_or(after_escape.len());
            Some(after_escape[..end].trim_end_matches('/').to_string())
        }
    }
}
