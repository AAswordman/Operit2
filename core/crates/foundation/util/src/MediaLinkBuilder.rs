/// Canonical builders for the media-link tags parsed by provider transports.
///
/// Media-link tags embed pool-backed payloads (images, audio, video, files)
/// inside plain message text. Every producer of these tags — chat message
/// building, connection testing, and tool result rendering — must go through
/// this builder so the tag shape stays identical to the provider-side parser.
pub struct MediaLinkBuilder;

impl MediaLinkBuilder {
    /// Builds a canonical image media-link tag.
    pub fn image(id: &str) -> String {
        format!("<link type=\"image\" id=\"{}\"></link>", id)
    }

    /// Builds a canonical audio media-link tag.
    pub fn audio(id: &str) -> String {
        format!("<link type=\"audio\" id=\"{}\"></link>", id)
    }

    /// Builds a canonical video media-link tag.
    pub fn video(id: &str) -> String {
        format!("<link type=\"video\" id=\"{}\"></link>", id)
    }

    /// Builds a canonical file media-link tag with an escaped filename.
    pub fn file(id: &str, file_name: &str) -> String {
        format!(
            "<link type=\"file\" id=\"{}\" filename=\"{}\"></link>",
            id,
            escape_xml_attribute(file_name)
        )
    }
}

/// Escapes XML attribute characters used in canonical media-link filenames.
fn escape_xml_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::MediaLinkBuilder;

    /// Keeps image tags aligned with the provider-side media-link parser shape.
    #[test]
    fn image_tag_matches_parser_contract() {
        assert_eq!(
            MediaLinkBuilder::image("abc-123"),
            "<link type=\"image\" id=\"abc-123\"></link>"
        );
    }

    /// Escapes filename characters that would break attribute parsing.
    #[test]
    fn file_tag_escapes_attribute_characters() {
        assert_eq!(
            MediaLinkBuilder::file("id1", "a\"b<c>d&e.txt"),
            "<link type=\"file\" id=\"id1\" filename=\"a&quot;b&lt;c&gt;d&amp;e.txt\"></link>"
        );
    }
}
