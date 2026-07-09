pub struct ConversationRoundManager {
    current_round: i32,
    raw_content: String,
}

impl ConversationRoundManager {
    /// Creates an empty conversation round manager.
    pub fn new() -> Self {
        Self {
            current_round: 0,
            raw_content: String::new(),
        }
    }

    /// Resets the manager to the beginning of a new conversation.
    pub fn initialize_new_conversation(&mut self) {
        self.current_round = 0;
        self.raw_content.clear();
    }

    /// Replaces the raw content for the current conversation round.
    pub fn update_content(&mut self, content: String) -> String {
        self.raw_content = content;
        self.raw_content.clone()
    }

    /// Advances to the next conversation round and returns its index.
    pub fn start_new_round(&mut self) -> i32 {
        self.current_round += 1;
        self.current_round
    }

    /// Appends text to the raw content and returns the updated content.
    pub fn append_content(&mut self, content: &str) -> String {
        self.raw_content.push_str(content);
        self.raw_content.clone()
    }

    /// Returns the current content prepared for display.
    pub fn get_display_content(&self) -> String {
        self.raw_content.clone()
    }

    /// Returns the raw content for the current conversation round.
    pub fn get_current_round_content(&self) -> String {
        self.raw_content.clone()
    }

    /// Returns the full raw conversation content buffer.
    pub fn get_raw_content(&self) -> String {
        self.raw_content.clone()
    }

    /// Returns the current conversation round index.
    pub fn get_current_round(&self) -> i32 {
        self.current_round
    }

    /// Clears the raw content for the current conversation round.
    pub fn clear_content(&mut self) {
        self.raw_content.clear();
    }
}
