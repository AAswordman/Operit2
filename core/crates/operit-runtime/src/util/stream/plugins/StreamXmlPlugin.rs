use crate::util::ChatMarkupRegex::ChatMarkupRegex;
use crate::util::stream::plugins::StreamPlugin::{PluginState, StreamPlugin};

#[derive(Debug, Clone)]
pub struct StreamXmlPlugin {
    include_tags_in_output: bool,
    state: PluginState,
    candidate: String,
    active_tag_name: Option<String>,
    closing_candidate: String,
    allow_start_after_end_tag: bool,
    allow_start_after_punctuation: bool,
    last_char: char,
}

impl Default for StreamXmlPlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamXmlPlugin {
    pub fn new(include_tags_in_output: bool) -> Self {
        Self {
            include_tags_in_output,
            state: PluginState::Idle,
            candidate: String::new(),
            active_tag_name: None,
            closing_candidate: String::new(),
            allow_start_after_end_tag: false,
            allow_start_after_punctuation: false,
            last_char: '\0',
        }
    }

    fn finish(&mut self, c: char, result: bool) -> bool {
        self.last_char = c;
        result
    }

    fn handle_default_character(&mut self, c: char) -> bool {
        self.update_punctuation_allowance(c);
        true
    }

    fn update_punctuation_allowance(&mut self, c: char) {
        if punctuation_triggers().contains(&c) || is_emoji_trigger(c) {
            self.allow_start_after_punctuation = true;
        } else if c == ' ' || c == '\t' || is_emoji_continuation_char(c) {
        } else {
            self.allow_start_after_punctuation = false;
        }
    }
}

impl StreamPlugin for StreamXmlPlugin {
    fn name(&self) -> &'static str {
        "StreamXmlPlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, at_start_of_line: bool) -> bool {
        if self.state == PluginState::Processing {
            self.closing_candidate.push(c);
            let end_tag = format!("</{}>", self.active_tag_name.as_deref().unwrap_or(""));
            if end_tag.starts_with(&self.closing_candidate) {
                if self.closing_candidate == end_tag {
                    self.allow_start_after_end_tag = true;
                    self.allow_start_after_punctuation = false;
                    self.reset();
                    self.allow_start_after_end_tag = true;
                }
                return self.finish(c, self.include_tags_in_output);
            }
            self.closing_candidate.clear();
            if c == '<' {
                self.closing_candidate.push(c);
            }
            return self.finish(c, true);
        }

        if self.state == PluginState::Idle && !at_start_of_line {
            let allow_start = self.allow_start_after_end_tag || self.allow_start_after_punctuation;
            if !allow_start {
                let result = self.handle_default_character(c);
                return self.finish(c, result);
            }
            if c == ' ' || c == '\t' || is_emoji_continuation_char(c) {
                let result = self.handle_default_character(c);
                return self.finish(c, result);
            }
        }

        self.candidate.push(c);
        if !self.candidate.starts_with('<') {
            self.candidate.clear();
            let result = self.handle_default_character(c);
            return self.finish(c, result);
        }

        if !self.candidate.contains('>') {
            self.state = PluginState::Trying;
            return self.finish(c, self.include_tags_in_output);
        }

        let tag_name = ChatMarkupRegex::extract_opening_tag_name(&self.candidate);
        if let Some(tag_name) = tag_name {
            if self.last_char == '/' || self.candidate.trim_end().ends_with("/>") {
                self.reset();
                return self.finish(c, true);
            }
            self.state = PluginState::Processing;
            self.active_tag_name = Some(tag_name);
            self.closing_candidate.clear();
            self.candidate.clear();
            self.allow_start_after_end_tag = false;
            self.allow_start_after_punctuation = false;
            self.finish(c, self.include_tags_in_output)
        } else {
            self.reset();
            self.finish(c, true)
        }
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.candidate.clear();
        self.active_tag_name = None;
        self.closing_candidate.clear();
        self.last_char = '\0';
    }
}

fn punctuation_triggers() -> Vec<char> {
    vec![
        '，', '。', '？', '！', '：', '（', '）', '【', '】', '《', '》', ':', ',', '.', '?', '!',
        '~', '～',
    ]
}

fn is_emoji_trigger(c: char) -> bool {
    ('\u{1F300}'..='\u{1FAFF}').contains(&c) || ('\u{2600}'..='\u{27BF}').contains(&c)
}

fn is_emoji_continuation_char(c: char) -> bool {
    matches!(c, '\u{200D}' | '\u{FE0E}' | '\u{FE0F}' | '\u{20E3}')
}
