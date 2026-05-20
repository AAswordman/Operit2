use crate::util::stream::plugins::StreamPlugin::{PluginState, StreamPlugin};

fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownFencedCodeBlockPlugin {
    include_fences: bool,
    state: PluginState,
    fence_len: usize,
    is_matching_end_fence: bool,
    has_started_matching_fence: bool,
}

impl StreamMarkdownFencedCodeBlockPlugin {
    pub fn new(include_fences: bool) -> Self {
        let mut value = Self {
            include_fences,
            state: PluginState::Idle,
            fence_len: 0,
            is_matching_end_fence: false,
            has_started_matching_fence: false,
        };
        value.reset();
        value
    }
}

impl Default for StreamMarkdownFencedCodeBlockPlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamPlugin for StreamMarkdownFencedCodeBlockPlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownFencedCodeBlockPlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, _at_start_of_line: bool) -> bool {
        if self.state == PluginState::Processing {
            if _at_start_of_line {
                self.is_matching_end_fence = true;
                self.has_started_matching_fence = false;
            }

            if self.is_matching_end_fence {
                if !self.has_started_matching_fence {
                    if c == ' ' {
                        return self.include_fences;
                    }
                    self.has_started_matching_fence = true;
                }

                if c == '`' {
                    self.fence_len += 1;
                    return self.include_fences;
                }

                if c == '\n' {
                    if self.fence_len >= 3 {
                        self.reset();
                        return self.include_fences;
                    }
                    self.is_matching_end_fence = false;
                    self.fence_len = 0;
                    return true;
                }

                self.is_matching_end_fence = false;
                self.fence_len = 0;
                return true;
            }

            return true;
        }

        if self.state == PluginState::Idle {
            if c == '`' {
                self.state = PluginState::Trying;
                self.fence_len = 1;
                return self.include_fences;
            }
            return true;
        }

        if self.state == PluginState::Trying {
            if c == '`' {
                self.fence_len += 1;
                return self.include_fences;
            }
            if c == '\n' {
                if self.fence_len >= 3 {
                    self.state = PluginState::Processing;
                    self.is_matching_end_fence = false;
                    self.has_started_matching_fence = false;
                    self.fence_len = 0;
                    return self.include_fences;
                }
                self.reset();
                return true;
            }
            if self.fence_len < 3 {
                self.reset();
                return true;
            }
            return self.include_fences;
        }

        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.fence_len = 0;
        self.is_matching_end_fence = false;
        self.has_started_matching_fence = false;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownInlineCodePlugin {
    include_ticks: bool,
    state: PluginState,
    tick_len: usize,
    end_match: usize,
}

impl StreamMarkdownInlineCodePlugin {
    pub fn new(include_ticks: bool) -> Self {
        let mut value = Self {
            include_ticks,
            state: PluginState::Idle,
            tick_len: 0,
            end_match: 0,
        };
        value.reset();
        value
    }
}

impl Default for StreamMarkdownInlineCodePlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamPlugin for StreamMarkdownInlineCodePlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownInlineCodePlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, _at_start_of_line: bool) -> bool {
        if self.state == PluginState::Processing && c == '\n' {
            self.reset();
            return true;
        }

        if self.state == PluginState::Processing {
            if c == '`' {
                self.end_match += 1;
                if self.end_match == self.tick_len {
                    self.reset();
                    return self.include_ticks;
                }
                return self.include_ticks;
            }
            self.end_match = 0;
            return true;
        }

        if c == '`' {
            if self.state == PluginState::Idle {
                self.state = PluginState::Trying;
                self.tick_len = 1;
                return self.include_ticks;
            }
            if self.state == PluginState::Trying {
                self.reset();
                return true;
            }
        }

        if self.state == PluginState::Trying {
            if c != '`' && c != '\n' {
                self.state = PluginState::Processing;
                self.end_match = 0;
                return true;
            }
            if c == '\n' {
                self.reset();
                return true;
            }
        }

        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.tick_len = 0;
        self.end_match = 0;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownBoldPlugin {
    include_asterisks: bool,
    state: PluginState,
    start_match: usize,
    end_match: usize,
}

impl StreamMarkdownBoldPlugin {
    pub fn new(include_asterisks: bool) -> Self {
        let mut value = Self {
            include_asterisks,
            state: PluginState::Idle,
            start_match: 0,
            end_match: 0,
        };
        value.reset();
        value
    }
}

impl Default for StreamMarkdownBoldPlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamPlugin for StreamMarkdownBoldPlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownBoldPlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, _at_start_of_line: bool) -> bool {
        if self.state == PluginState::Processing {
            if c == '*' {
                self.end_match += 1;
                if self.end_match == 2 {
                    self.reset();
                    return self.include_asterisks;
                }
                return self.include_asterisks;
            }
            self.end_match = 0;
            return true;
        }

        if self.state == PluginState::Idle {
            if c == '*' {
                self.state = PluginState::Trying;
                self.start_match = 1;
                return self.include_asterisks;
            }
            return true;
        }

        if self.state == PluginState::Trying {
            if self.start_match == 1 {
                if c == '*' {
                    self.start_match = 2;
                    return self.include_asterisks;
                }
                self.reset();
                return true;
            }
            if self.start_match == 2 {
                if c != '*' && c != '\n' {
                    self.state = PluginState::Processing;
                    self.start_match = 0;
                    self.end_match = 0;
                    return true;
                }
                self.reset();
                return true;
            }
        }

        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.start_match = 0;
        self.end_match = 0;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownItalicPlugin {
    include_asterisks: bool,
    state: PluginState,
    start_match: usize,
    last_char: Option<char>,
}

impl StreamMarkdownItalicPlugin {
    pub fn new(include_asterisks: bool) -> Self {
        let mut value = Self {
            include_asterisks,
            state: PluginState::Idle,
            start_match: 0,
            last_char: None,
        };
        value.reset();
        value
    }
}

impl Default for StreamMarkdownItalicPlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamPlugin for StreamMarkdownItalicPlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownItalicPlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, _at_start_of_line: bool) -> bool {
        if self.last_char == Some('*') && c == '*' {
            self.last_char = None;
            self.reset();
            return true;
        }
        self.last_char = Some(c);

        if self.state == PluginState::Processing {
            if c == '\n' {
                self.reset();
                return true;
            }
            if c == '*' {
                self.reset();
                return self.include_asterisks;
            }
            return true;
        }

        if c == '*' {
            self.state = PluginState::Trying;
            self.start_match = 1;
            return self.include_asterisks;
        }

        if self.state == PluginState::Trying {
            if c != '*' && c != '\n' && c != ' ' {
                self.state = PluginState::Processing;
                return true;
            }
            self.reset();
            return true;
        }

        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.start_match = 0;
        self.last_char = None;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownHeaderPlugin {
    include_marker: bool,
    state: PluginState,
    hash_count: usize,
    in_match: bool,
}

impl StreamMarkdownHeaderPlugin {
    pub fn new(include_marker: bool) -> Self {
        let mut value = Self {
            include_marker,
            state: PluginState::Idle,
            hash_count: 0,
            in_match: false,
        };
        value.reset();
        value
    }
}

impl Default for StreamMarkdownHeaderPlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamPlugin for StreamMarkdownHeaderPlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownHeaderPlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, at_start_of_line: bool) -> bool {
        if self.state == PluginState::Processing {
            if c == '\n' {
                self.reset();
            }
            return true;
        }

        if at_start_of_line {
            self.in_match = true;
            self.hash_count = 0;
            self.state = PluginState::Idle;
        }

        if !self.in_match && self.state != PluginState::Trying {
            return true;
        }

        if c == '#' {
            self.hash_count += 1;
            self.state = PluginState::Trying;
            return self.include_marker;
        }

        if c == ' ' && (1..=6).contains(&self.hash_count) {
            self.state = PluginState::Processing;
            self.in_match = false;
            return self.include_marker;
        }

        self.reset();
        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.hash_count = 0;
        self.in_match = false;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownLinkPlugin {
    state: PluginState,
    phase: usize,
}

impl Default for StreamMarkdownLinkPlugin {
    fn default() -> Self {
        let mut value = Self {
            state: PluginState::Idle,
            phase: 0,
        };
        value.reset();
        value
    }
}

impl StreamPlugin for StreamMarkdownLinkPlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownLinkPlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, _at_start_of_line: bool) -> bool {
        if self.state == PluginState::Idle {
            if c == '[' {
                self.state = PluginState::Trying;
                self.phase = 1;
            }
            return true;
        }

        if self.state == PluginState::Trying || self.state == PluginState::Processing {
            if c == '\n' {
                self.reset();
                return true;
            }
            if self.phase == 1 {
                if c == ']' {
                    self.phase = 2;
                    self.state = PluginState::Processing;
                }
                return true;
            }
            if self.phase == 2 {
                if c == '(' {
                    self.phase = 3;
                    return true;
                }
                self.reset();
                return true;
            }
            if self.phase == 3 {
                if c == ')' {
                    self.reset();
                    return true;
                }
                return true;
            }
        }

        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.phase = 0;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownBlockQuotePlugin {
    include_marker: bool,
    state: PluginState,
    match_index: usize,
}

impl StreamMarkdownBlockQuotePlugin {
    pub fn new(include_marker: bool) -> Self {
        let mut value = Self {
            include_marker,
            state: PluginState::Idle,
            match_index: 0,
        };
        value.reset();
        value
    }
}

impl Default for StreamMarkdownBlockQuotePlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamPlugin for StreamMarkdownBlockQuotePlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownBlockQuotePlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, at_start_of_line: bool) -> bool {
        if c == '\n' {
            if self.state == PluginState::Processing {
                self.state = PluginState::WaitFor;
            } else {
                self.reset();
            }
            return true;
        }

        if self.state == PluginState::WaitFor && at_start_of_line {
            if c == '>' {
                self.state = PluginState::Processing;
                self.match_index = 1;
                return true;
            }
            self.reset();
            return true;
        }

        if at_start_of_line {
            if self.match_index == 0 {
                if c == '>' {
                    self.match_index = 1;
                    self.state = PluginState::Trying;
                    return self.include_marker;
                }
                return true;
            }
            if self.match_index == 1 {
                if c == ' ' {
                    self.state = PluginState::Processing;
                    self.match_index = 0;
                    return self.include_marker;
                }
                self.reset();
                return true;
            }
        }

        if self.state == PluginState::Processing {
            return true;
        }

        if self.state == PluginState::Trying && self.match_index == 1 {
            if c == ' ' {
                self.state = PluginState::Processing;
                self.match_index = 0;
                return self.include_marker;
            }
            self.reset();
            return true;
        }

        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.match_index = 0;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownHorizontalRulePlugin {
    include_marker: bool,
    state: PluginState,
    current_marker: Option<char>,
    marker_count: usize,
}

impl StreamMarkdownHorizontalRulePlugin {
    pub fn new(include_marker: bool) -> Self {
        let mut value = Self {
            include_marker,
            state: PluginState::Idle,
            current_marker: None,
            marker_count: 0,
        };
        value.reset();
        value
    }
}

impl Default for StreamMarkdownHorizontalRulePlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamPlugin for StreamMarkdownHorizontalRulePlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownHorizontalRulePlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, at_start_of_line: bool) -> bool {
        if c == '\n' {
            let is_match = (self.state == PluginState::Trying
                || self.state == PluginState::Processing)
                && self.marker_count >= 3;
            let should_emit = is_match && self.include_marker;
            self.reset();
            return if is_match { should_emit } else { true };
        }

        if self.state == PluginState::Idle {
            if at_start_of_line && matches!(c, '-' | '*' | '_') {
                self.state = PluginState::Trying;
                self.current_marker = Some(c);
                self.marker_count = 1;
                return self.include_marker;
            }
            return true;
        }

        if self.current_marker == Some(c) || c == ' ' || c == '\t' {
            if self.current_marker == Some(c) {
                self.marker_count += 1;
            }
            if self.marker_count >= 3 {
                self.state = PluginState::Processing;
            }
            return self.include_marker;
        }

        self.reset();
        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.current_marker = None;
        self.marker_count = 0;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownOrderedListPlugin {
    include_marker: bool,
    state: PluginState,
    match_state: usize,
}

impl StreamMarkdownOrderedListPlugin {
    pub fn new(include_marker: bool) -> Self {
        let mut value = Self {
            include_marker,
            state: PluginState::Idle,
            match_state: 0,
        };
        value.reset();
        value
    }
}

impl Default for StreamMarkdownOrderedListPlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamPlugin for StreamMarkdownOrderedListPlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownOrderedListPlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, at_start_of_line: bool) -> bool {
        if self.state == PluginState::Processing {
            if c == '\n' {
                self.reset();
            }
            return true;
        }

        if at_start_of_line {
            self.match_state = 0;
            self.state = PluginState::Idle;
        }

        if !at_start_of_line && self.state != PluginState::Trying {
            return true;
        }

        if self.match_state == 0 {
            if is_digit(c) {
                self.state = PluginState::Trying;
                self.match_state = 1;
                return self.include_marker;
            }
            self.reset();
            return true;
        }

        if self.match_state == 1 {
            if is_digit(c) {
                self.state = PluginState::Trying;
                return self.include_marker;
            }
            if c == '.' {
                self.match_state = 2;
                self.state = PluginState::Trying;
                return self.include_marker;
            }
            self.reset();
            return true;
        }

        if self.match_state == 2 {
            if c == ' ' {
                self.state = PluginState::Processing;
                self.match_state = 0;
                return self.include_marker;
            }
            self.reset();
            return true;
        }

        self.reset();
        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.match_state = 0;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownUnorderedListPlugin {
    include_marker: bool,
    state: PluginState,
    match_state: usize,
}

impl StreamMarkdownUnorderedListPlugin {
    pub fn new(include_marker: bool) -> Self {
        let mut value = Self {
            include_marker,
            state: PluginState::Idle,
            match_state: 0,
        };
        value.reset();
        value
    }
}

impl Default for StreamMarkdownUnorderedListPlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamPlugin for StreamMarkdownUnorderedListPlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownUnorderedListPlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, at_start_of_line: bool) -> bool {
        if self.state == PluginState::Processing {
            if c == '\n' {
                self.reset();
            }
            return true;
        }

        if at_start_of_line {
            self.match_state = 0;
            self.state = PluginState::Idle;
        }

        if !at_start_of_line && self.state != PluginState::Trying {
            return true;
        }

        if self.match_state == 0 {
            if matches!(c, '-' | '+' | '*') {
                self.state = PluginState::Trying;
                self.match_state = 1;
                return self.include_marker;
            }
            self.reset();
            return true;
        }

        if self.match_state == 1 {
            if c == ' ' {
                self.state = PluginState::Processing;
                self.match_state = 0;
                return self.include_marker;
            }
            self.reset();
            return true;
        }

        self.reset();
        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.match_state = 0;
    }
}

macro_rules! double_delimiter_plugin {
    ($type_name:ident, $start_char:expr, $end_char:expr, $field_name:ident) => {
        #[derive(Debug, Clone)]
        pub struct $type_name {
            $field_name: bool,
            state: PluginState,
            start_state: usize,
            end_state: usize,
        }

        impl $type_name {
            pub fn new($field_name: bool) -> Self {
                let mut value = Self {
                    $field_name,
                    state: PluginState::Idle,
                    start_state: 0,
                    end_state: 0,
                };
                value.reset();
                value
            }
        }

        impl Default for $type_name {
            fn default() -> Self {
                Self::new(true)
            }
        }

        impl StreamPlugin for $type_name {
            fn name(&self) -> &'static str {
                stringify!($type_name)
            }

            fn state(&self) -> PluginState {
                self.state
            }

            fn process_char(&mut self, c: char, _at_start_of_line: bool) -> bool {
                if self.state == PluginState::Processing {
                    if self.end_state == 0 {
                        if c == $end_char {
                            self.end_state = 1;
                            return self.$field_name;
                        }
                        return true;
                    }
                    if self.end_state == 1 {
                        if c == $end_char {
                            self.reset();
                            return self.$field_name;
                        }
                        self.end_state = 0;
                        return true;
                    }
                    self.end_state = 0;
                    return true;
                }

                if self.start_state == 0 {
                    if c == $start_char {
                        self.start_state = 1;
                        self.state = PluginState::Trying;
                        return self.$field_name;
                    }
                    return true;
                }
                if self.start_state == 1 {
                    if c == $start_char {
                        self.start_state = 2;
                        self.state = PluginState::Trying;
                        return self.$field_name;
                    }
                    self.reset();
                    return true;
                }
                if self.start_state == 2 {
                    if c != $start_char && c != '\n' {
                        self.state = PluginState::Processing;
                        self.start_state = 0;
                        self.end_state = 0;
                        return true;
                    }
                    self.reset();
                    return true;
                }

                self.reset();
                true
            }

            fn init_plugin(&mut self) -> bool {
                self.reset();
                true
            }

            fn destroy(&mut self) {}

            fn reset(&mut self) {
                self.state = PluginState::Idle;
                self.start_state = 0;
                self.end_state = 0;
            }
        }
    };
}

double_delimiter_plugin!(
    StreamMarkdownStrikethroughPlugin,
    '~',
    '~',
    include_delimiters
);
double_delimiter_plugin!(
    StreamMarkdownUnderlinePlugin,
    '_',
    '_',
    include_delimiters
);

#[derive(Debug, Clone)]
pub struct StreamMarkdownInlineLaTeXPlugin {
    include_delimiters: bool,
    state: PluginState,
    start_state: usize,
    end_state: usize,
}

impl StreamMarkdownInlineLaTeXPlugin {
    pub fn new(include_delimiters: bool) -> Self {
        let mut value = Self {
            include_delimiters,
            state: PluginState::Idle,
            start_state: 0,
            end_state: 0,
        };
        value.reset();
        value
    }
}

impl Default for StreamMarkdownInlineLaTeXPlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamPlugin for StreamMarkdownInlineLaTeXPlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownInlineLaTeXPlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, _at_start_of_line: bool) -> bool {
        if self.state == PluginState::Processing {
            if self.end_state == 0 && c == '$' {
                self.end_state = 1;
                self.reset();
                return self.include_delimiters;
            }
            return true;
        }

        if self.start_state == 0 {
            if c == '$' {
                self.start_state = 1;
                self.state = PluginState::Trying;
                return self.include_delimiters;
            }
            return true;
        }

        if self.start_state == 1 {
            if c != '$' && c != '\n' {
                self.state = PluginState::Processing;
                self.start_state = 0;
                self.end_state = 0;
                return true;
            }
            self.reset();
            return true;
        }

        self.reset();
        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.start_state = 0;
        self.end_state = 0;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownInlineParenLaTeXPlugin {
    include_delimiters: bool,
    state: PluginState,
    start_state: usize,
    end_state: usize,
}

impl StreamMarkdownInlineParenLaTeXPlugin {
    pub fn new(include_delimiters: bool) -> Self {
        let mut value = Self {
            include_delimiters,
            state: PluginState::Idle,
            start_state: 0,
            end_state: 0,
        };
        value.reset();
        value
    }
}

impl Default for StreamMarkdownInlineParenLaTeXPlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamPlugin for StreamMarkdownInlineParenLaTeXPlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownInlineParenLaTeXPlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, _at_start_of_line: bool) -> bool {
        if self.state == PluginState::Processing {
            if self.end_state == 0 {
                if c == '\\' {
                    self.end_state = 1;
                    return self.include_delimiters;
                }
                return true;
            }
            if self.end_state == 1 {
                if c == ')' {
                    self.reset();
                    return self.include_delimiters;
                }
                self.end_state = 0;
                return true;
            }
            self.end_state = 0;
            return true;
        }

        if self.start_state == 0 {
            if c == '\\' {
                self.start_state = 1;
                self.state = PluginState::Trying;
                return self.include_delimiters;
            }
            return true;
        }
        if self.start_state == 1 {
            if c == '(' {
                self.start_state = 2;
                self.state = PluginState::Trying;
                return self.include_delimiters;
            }
            self.reset();
            return true;
        }
        if self.start_state == 2 {
            if c != '\n' {
                self.state = PluginState::Processing;
                self.start_state = 0;
                self.end_state = 0;
                return true;
            }
            self.reset();
            return true;
        }

        self.reset();
        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.start_state = 0;
        self.end_state = 0;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownBlockLaTeXPlugin {
    include_delimiters: bool,
    state: PluginState,
    start_state: usize,
    end_state: usize,
}

impl StreamMarkdownBlockLaTeXPlugin {
    pub fn new(include_delimiters: bool) -> Self {
        let mut value = Self {
            include_delimiters,
            state: PluginState::Idle,
            start_state: 0,
            end_state: 0,
        };
        value.reset();
        value
    }
}

impl Default for StreamMarkdownBlockLaTeXPlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamPlugin for StreamMarkdownBlockLaTeXPlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownBlockLaTeXPlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, _at_start_of_line: bool) -> bool {
        if self.state == PluginState::Processing {
            if self.end_state == 0 {
                if c == '$' {
                    self.end_state = 1;
                    return self.include_delimiters;
                }
                return true;
            }
            if self.end_state == 1 {
                if c == '$' {
                    self.reset();
                    return self.include_delimiters;
                }
                self.end_state = 0;
                return true;
            }
            self.end_state = 0;
            return true;
        }

        if self.start_state == 0 {
            if c == '$' {
                self.start_state = 1;
                self.state = PluginState::Trying;
                return self.include_delimiters;
            }
            return true;
        }
        if self.start_state == 1 {
            if c == '$' {
                self.state = PluginState::Processing;
                self.start_state = 0;
                self.end_state = 0;
                return self.include_delimiters;
            }
            self.reset();
            return true;
        }

        self.reset();
        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.start_state = 0;
        self.end_state = 0;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownBlockBracketLaTeXPlugin {
    include_delimiters: bool,
    state: PluginState,
    start_state: usize,
    end_state: usize,
}

impl StreamMarkdownBlockBracketLaTeXPlugin {
    pub fn new(include_delimiters: bool) -> Self {
        let mut value = Self {
            include_delimiters,
            state: PluginState::Idle,
            start_state: 0,
            end_state: 0,
        };
        value.reset();
        value
    }
}

impl Default for StreamMarkdownBlockBracketLaTeXPlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamPlugin for StreamMarkdownBlockBracketLaTeXPlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownBlockBracketLaTeXPlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, _at_start_of_line: bool) -> bool {
        if self.state == PluginState::Processing {
            if self.end_state == 0 {
                if c == '\\' {
                    self.end_state = 1;
                    return self.include_delimiters;
                }
                return true;
            }
            if self.end_state == 1 {
                if c == ']' {
                    self.reset();
                    return self.include_delimiters;
                }
                self.end_state = 0;
                return true;
            }
            self.end_state = 0;
            return true;
        }

        if self.start_state == 0 {
            if c == '\\' {
                self.start_state = 1;
                self.state = PluginState::Trying;
                return self.include_delimiters;
            }
            return true;
        }
        if self.start_state == 1 {
            if c == '[' {
                self.state = PluginState::Processing;
                self.start_state = 0;
                self.end_state = 0;
                return self.include_delimiters;
            }
            self.reset();
            return true;
        }

        self.reset();
        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.start_state = 0;
        self.end_state = 0;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownImagePlugin {
    include_delimiters: bool,
    state: PluginState,
    phase: usize,
}

impl StreamMarkdownImagePlugin {
    pub fn new(include_delimiters: bool) -> Self {
        let mut value = Self {
            include_delimiters,
            state: PluginState::Idle,
            phase: 0,
        };
        value.reset();
        value
    }
}

impl Default for StreamMarkdownImagePlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamPlugin for StreamMarkdownImagePlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownImagePlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, _at_start_of_line: bool) -> bool {
        if self.state == PluginState::Idle {
            if c == '!' {
                self.state = PluginState::Trying;
                self.phase = 1;
                return self.include_delimiters;
            }
            return true;
        }

        if self.state == PluginState::Trying || self.state == PluginState::Processing {
            if c == '\n' {
                self.reset();
                return true;
            }
            if self.phase == 1 {
                if c == '[' {
                    self.phase = 2;
                    self.state = PluginState::Processing;
                    return self.include_delimiters;
                }
                self.reset();
                return true;
            }
            if self.phase == 2 {
                if c == ']' {
                    self.phase = 3;
                    return self.include_delimiters;
                }
                return self.include_delimiters;
            }
            if self.phase == 3 {
                if c == '(' {
                    self.phase = 4;
                    return self.include_delimiters;
                }
                self.reset();
                return true;
            }
            if self.phase == 4 {
                if c == ')' {
                    self.reset();
                    return self.include_delimiters;
                }
                return self.include_delimiters;
            }
        }

        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.phase = 0;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownTablePlugin {
    include_delimiters: bool,
    state: PluginState,
    table_row_count: usize,
    found_header_separator: bool,
    header_sep_match_state: usize,
}

impl StreamMarkdownTablePlugin {
    pub fn new(include_delimiters: bool) -> Self {
        let mut value = Self {
            include_delimiters,
            state: PluginState::Idle,
            table_row_count: 0,
            found_header_separator: false,
            header_sep_match_state: 0,
        };
        value.reset();
        value
    }
}

impl Default for StreamMarkdownTablePlugin {
    fn default() -> Self {
        Self::new(true)
    }
}

impl StreamPlugin for StreamMarkdownTablePlugin {
    fn name(&self) -> &'static str {
        "StreamMarkdownTablePlugin"
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, at_start_of_line: bool) -> bool {
        if c == '\n' {
            if self.state == PluginState::Processing {
                self.state = PluginState::WaitFor;
            }
            return true;
        }

        if self.state == PluginState::WaitFor && at_start_of_line {
            if c == '|' {
                self.state = PluginState::Processing;
                self.table_row_count += 1;
                self.header_sep_match_state = 0;
                return self.include_delimiters;
            }
            if matches!(c, '$' | '`' | '#' | '>' | '*' | '-' | '+') {
                self.reset();
                return true;
            }
            self.reset();
            return true;
        }

        if at_start_of_line {
            if c == '|' {
                if self.state == PluginState::Idle {
                    self.state = PluginState::Processing;
                    self.table_row_count = 1;
                    self.found_header_separator = false;
                } else if self.state == PluginState::Processing {
                    self.table_row_count += 1;
                }
                self.header_sep_match_state = 0;
                return self.include_delimiters;
            }
            if self.state == PluginState::Processing {
                self.reset();
            }
            return true;
        }

        if self.state == PluginState::Processing {
            if self.table_row_count == 2 && !self.found_header_separator {
                if self.header_sep_match_state == 0 {
                    self.header_sep_match_state = 1;
                }
                let _ = matches!(c, '|' | '-' | ':' | ' ' | '\t');
            }
            if self.include_delimiters {
                return true;
            }
            return c != '|';
        }

        true
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.table_row_count = 0;
        self.found_header_separator = false;
        self.header_sep_match_state = 0;
    }
}
