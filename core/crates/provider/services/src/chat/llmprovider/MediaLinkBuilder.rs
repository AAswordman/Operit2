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
}
