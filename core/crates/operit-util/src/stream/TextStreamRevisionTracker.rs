use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct TextStreamRevisionTracker {
    content_buffer: String,
    savepoints: BTreeMap<String, usize>,
}

impl TextStreamRevisionTracker {
    /// Creates a revision tracker with optional initial content.
    pub fn new(initial_content: impl Into<String>) -> Self {
        Self {
            content_buffer: initial_content.into(),
            savepoints: BTreeMap::new(),
        }
    }

    /// Borrows the currently accumulated content without cloning it.
    pub fn current_content(&self) -> &str {
        &self.content_buffer
    }

    /// Appends one streamed chunk and borrows the updated content.
    pub fn append(&mut self, chunk: &str) -> &str {
        self.content_buffer.push_str(chunk);
        &self.content_buffer
    }

    /// Records the current UTF-8 byte boundary for a later destructive rollback.
    pub fn savepoint(&mut self, id: &str) {
        self.savepoints
            .insert(id.to_string(), self.content_buffer.len());
    }

    /// Truncates content to a savepoint and borrows the resulting content.
    pub fn rollback(&mut self, id: &str) -> Option<&str> {
        let position = self.savepoints.get(id).copied()?;
        self.content_buffer.truncate(position);
        self.savepoints.retain(|_, saved| *saved <= position);
        Some(&self.content_buffer)
    }

    /// Replaces the accumulated content and clears savepoints from the old text.
    pub fn replace(&mut self, content: &str) {
        self.content_buffer.clear();
        self.content_buffer.push_str(content);
        self.savepoints.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::TextStreamRevisionTracker;

    /// Verifies that appending mutates one buffer and exposes a borrowed view.
    #[test]
    fn append_borrows_updated_content() {
        let mut tracker = TextStreamRevisionTracker::new("a");

        assert_eq!(tracker.append("b"), "ab");
        assert_eq!(tracker.append("c"), "abc");
        assert_eq!(tracker.current_content(), "abc");
    }

    /// Verifies that rollback truncates to the recorded byte boundary.
    #[test]
    fn rollback_truncates_to_savepoint() {
        let mut tracker = TextStreamRevisionTracker::new("");
        let _ = tracker.append("before");
        tracker.savepoint("retry");
        let _ = tracker.append(" discarded");

        assert_eq!(tracker.rollback("retry"), Some("before"));
    }

    /// Verifies that savepoints inside a discarded suffix are invalidated.
    #[test]
    fn rollback_invalidates_discarded_savepoints() {
        let mut tracker = TextStreamRevisionTracker::new("");
        let _ = tracker.append("a");
        tracker.savepoint("outer");
        let _ = tracker.append("b");
        tracker.savepoint("inner");
        let _ = tracker.append("c");

        let _ = tracker.rollback("outer");

        assert_eq!(tracker.rollback("inner"), None);
    }
}
