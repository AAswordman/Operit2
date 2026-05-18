use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct TextStreamRevisionTracker {
    content_buffer: String,
    savepoints: BTreeMap<String, String>,
}

impl TextStreamRevisionTracker {
    pub fn new(initial_content: impl Into<String>) -> Self {
        Self {
            content_buffer: initial_content.into(),
            savepoints: BTreeMap::new(),
        }
    }

    pub fn current_content(&self) -> String {
        self.content_buffer.clone()
    }

    pub fn append(&mut self, chunk: &str) -> String {
        self.content_buffer.push_str(chunk);
        self.content_buffer.clone()
    }

    pub fn savepoint(&mut self, id: &str) {
        self.savepoints
            .insert(id.to_string(), self.content_buffer.clone());
    }

    pub fn rollback(&mut self, id: &str) -> Option<String> {
        let snapshot = self.savepoints.get(id)?.clone();
        self.content_buffer.clear();
        self.content_buffer.push_str(&snapshot);
        Some(snapshot)
    }

    pub fn replace(&mut self, content: &str) {
        self.content_buffer.clear();
        self.content_buffer.push_str(content);
    }
}
