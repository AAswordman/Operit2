#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiReference {
    pub title: String,
    pub url: String,
}

pub struct ReferenceManager;

impl ReferenceManager {
    pub fn extractReferences(content: &str) -> Vec<AiReference> {
        let mut refs = Vec::new();
        let mut cursor = 0;
        while let Some(open_rel) = content[cursor..].find('[') {
            let title_start = cursor + open_rel + 1;
            let Some(close_rel) = content[title_start..].find("](") else {
                break;
            };
            let title_end = title_start + close_rel;
            let url_start = title_end + 2;
            let Some(url_end_rel) = content[url_start..].find(')') else {
                break;
            };
            let url_end = url_start + url_end_rel;
            let title = &content[title_start..title_end];
            let url = &content[url_start..url_end];
            if url.starts_with("http://") || url.starts_with("https://") {
                refs.push(AiReference {
                    title: title.to_string(),
                    url: url.to_string(),
                });
            }
            cursor = url_end + 1;
        }
        refs
    }
}
