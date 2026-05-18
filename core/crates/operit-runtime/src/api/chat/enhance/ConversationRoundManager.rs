pub struct ConversationRoundManager {
    current_round: i32,
    raw_content: String,
}

impl ConversationRoundManager {
    pub fn new() -> Self {
        Self {
            current_round: 0,
            raw_content: String::new(),
        }
    }

    pub fn initialize_new_conversation(&mut self) {
        self.current_round = 0;
        self.raw_content.clear();
    }

    pub fn update_content(&mut self, content: String) -> String {
        self.raw_content = content;
        self.raw_content.clone()
    }

    pub fn start_new_round(&mut self) -> i32 {
        self.current_round += 1;
        self.current_round
    }

    pub fn append_content(&mut self, content: &str) -> String {
        self.raw_content.push_str(content);
        self.raw_content.clone()
    }

    pub fn get_display_content(&self) -> String {
        self.raw_content.clone()
    }

    pub fn get_current_round_content(&self) -> String {
        self.raw_content.clone()
    }

    pub fn get_raw_content(&self) -> String {
        self.raw_content.clone()
    }

    pub fn get_current_round(&self) -> i32 {
        self.current_round
    }

    pub fn clear_content(&mut self) {
        self.raw_content.clear();
    }
}
