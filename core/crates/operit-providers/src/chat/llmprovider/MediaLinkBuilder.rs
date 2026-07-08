pub struct MediaLinkBuilder;

impl MediaLinkBuilder {
    pub fn image(id: &str) -> String {
        format!("<link type=\"image\" id=\"{}\"></link>", id)
    }

    pub fn audio(id: &str) -> String {
        format!("<link type=\"audio\" id=\"{}\"></link>", id)
    }

    pub fn video(id: &str) -> String {
        format!("<link type=\"video\" id=\"{}\"></link>", id)
    }
}
