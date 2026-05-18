use crate::util::stream::plugins::StreamPlugin::{PluginState, StreamPlugin};

#[derive(Debug, Clone)]
struct DelimitedPlugin {
    name: &'static str,
    start: &'static str,
    end: &'static str,
    include_delimiters: bool,
    line_bound: bool,
    at_line_start: bool,
    state: PluginState,
    start_buffer: String,
    end_buffer: String,
}

impl DelimitedPlugin {
    fn new(
        name: &'static str,
        start: &'static str,
        end: &'static str,
        include_delimiters: bool,
        line_bound: bool,
        at_line_start: bool,
    ) -> Self {
        Self {
            name,
            start,
            end,
            include_delimiters,
            line_bound,
            at_line_start,
            state: PluginState::Idle,
            start_buffer: String::new(),
            end_buffer: String::new(),
        }
    }

    fn process(&mut self, c: char, at_start_of_line: bool) -> bool {
        if self.state == PluginState::Processing {
            if self.line_bound && c == '\n' {
                self.reset();
                return true;
            }
            self.end_buffer.push(c);
            if self.end.starts_with(&self.end_buffer) {
                if self.end_buffer == self.end {
                    self.reset();
                }
                return self.include_delimiters;
            }
            self.end_buffer.clear();
            if self.end.starts_with(c) {
                self.end_buffer.push(c);
                return self.include_delimiters;
            }
            return true;
        }

        if self.at_line_start && !at_start_of_line && self.state == PluginState::Idle {
            return true;
        }

        self.start_buffer.push(c);
        if self.start.starts_with(&self.start_buffer) {
            self.state = PluginState::Trying;
            if self.start_buffer == self.start {
                self.state = PluginState::Processing;
                self.start_buffer.clear();
                self.end_buffer.clear();
            }
            return self.include_delimiters;
        }

        self.reset();
        true
    }

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.start_buffer.clear();
        self.end_buffer.clear();
    }
}

macro_rules! delimited_plugin {
    ($type_name:ident, $start:expr, $end:expr, $field_name:ident, $line_bound:expr, $at_line_start:expr) => {
        #[derive(Debug, Clone)]
        pub struct $type_name {
            inner: DelimitedPlugin,
        }

        impl $type_name {
            pub fn new($field_name: bool) -> Self {
                Self {
                    inner: DelimitedPlugin::new(
                        stringify!($type_name),
                        $start,
                        $end,
                        $field_name,
                        $line_bound,
                        $at_line_start,
                    ),
                }
            }
        }

        impl Default for $type_name {
            fn default() -> Self {
                Self::new(true)
            }
        }

        impl StreamPlugin for $type_name {
            fn name(&self) -> &'static str {
                self.inner.name
            }

            fn state(&self) -> PluginState {
                self.inner.state
            }

            fn process_char(&mut self, c: char, at_start_of_line: bool) -> bool {
                self.inner.process(c, at_start_of_line)
            }

            fn init_plugin(&mut self) -> bool {
                self.reset();
                true
            }

            fn destroy(&mut self) {}

            fn reset(&mut self) {
                self.inner.reset();
            }
        }
    };
}

delimited_plugin!(StreamMarkdownInlineCodePlugin, "`", "`", include_ticks, true, false);
delimited_plugin!(StreamMarkdownBoldPlugin, "**", "**", include_asterisks, true, false);
delimited_plugin!(StreamMarkdownItalicPlugin, "*", "*", include_asterisks, true, false);
delimited_plugin!(StreamMarkdownStrikethroughPlugin, "~~", "~~", include_delimiters, true, false);
delimited_plugin!(StreamMarkdownUnderlinePlugin, "__", "__", include_delimiters, true, false);
delimited_plugin!(StreamMarkdownInlineLaTeXPlugin, "$", "$", include_delimiters, true, false);
delimited_plugin!(StreamMarkdownInlineParenLaTeXPlugin, "\\(", "\\)", include_delimiters, true, false);
delimited_plugin!(StreamMarkdownBlockLaTeXPlugin, "$$", "$$", include_delimiters, false, false);
delimited_plugin!(StreamMarkdownBlockBracketLaTeXPlugin, "\\[", "\\]", include_delimiters, false, false);

#[derive(Debug, Clone)]
pub struct StreamMarkdownFencedCodeBlockPlugin {
    include_fences: bool,
    state: PluginState,
    opening_fence: String,
    line_buffer: String,
}

impl StreamMarkdownFencedCodeBlockPlugin {
    pub fn new(include_fences: bool) -> Self {
        Self {
            include_fences,
            state: PluginState::Idle,
            opening_fence: String::new(),
            line_buffer: String::new(),
        }
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

    fn process_char(&mut self, c: char, at_start_of_line: bool) -> bool {
        if self.state == PluginState::Processing {
            if at_start_of_line {
                self.line_buffer.clear();
            }
            self.line_buffer.push(c);
            if self.line_buffer.trim_start().starts_with(&self.opening_fence)
                && self.line_buffer.trim_end() == self.opening_fence
            {
                self.reset();
                return self.include_fences;
            }
            return true;
        }

        if at_start_of_line && c == '`' {
            self.opening_fence.push(c);
            self.state = PluginState::Trying;
            return self.include_fences;
        }
        if self.state == PluginState::Trying {
            if c == '`' {
                self.opening_fence.push(c);
                return self.include_fences;
            }
            if self.opening_fence.len() >= 3 {
                self.state = PluginState::Processing;
                return true;
            }
            self.reset();
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
        self.opening_fence.clear();
        self.line_buffer.clear();
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownHeaderPlugin {
    include_marker: bool,
    state: PluginState,
    hash_count: usize,
}

impl StreamMarkdownHeaderPlugin {
    pub fn new(include_marker: bool) -> Self {
        Self {
            include_marker,
            state: PluginState::Idle,
            hash_count: 0,
        }
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
        if at_start_of_line && c == '#' {
            self.state = PluginState::Trying;
            self.hash_count = 1;
            return self.include_marker;
        }
        if self.state == PluginState::Trying {
            if c == '#' {
                self.hash_count += 1;
                return self.include_marker;
            }
            if c == ' ' && (1..=6).contains(&self.hash_count) {
                self.state = PluginState::Processing;
                return self.include_marker;
            }
            self.reset();
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
        self.hash_count = 0;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownLinkPlugin {
    state: PluginState,
    stage: LinkStage,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum LinkStage {
    WaitText,
    WaitParen,
    WaitUrl,
}

impl Default for StreamMarkdownLinkPlugin {
    fn default() -> Self {
        Self {
            state: PluginState::Idle,
            stage: LinkStage::WaitText,
        }
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
        match self.state {
            PluginState::Idle if c == '[' => {
                self.state = PluginState::Trying;
                self.stage = LinkStage::WaitText;
            }
            PluginState::Trying | PluginState::Processing => match self.stage {
                LinkStage::WaitText if c == ']' => self.stage = LinkStage::WaitParen,
                LinkStage::WaitParen if c == '(' => {
                    self.stage = LinkStage::WaitUrl;
                    self.state = PluginState::Processing;
                }
                LinkStage::WaitUrl if c == ')' => self.reset(),
                _ if c == '\n' => self.reset(),
                _ => {}
            },
            _ => {}
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
        self.stage = LinkStage::WaitText;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownImagePlugin {
    include_delimiters: bool,
    state: PluginState,
    buffer: String,
}

impl StreamMarkdownImagePlugin {
    pub fn new(include_delimiters: bool) -> Self {
        Self {
            include_delimiters,
            state: PluginState::Idle,
            buffer: String::new(),
        }
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
        self.buffer.push(c);
        if self.state == PluginState::Idle && self.buffer.ends_with("![") {
            self.state = PluginState::Processing;
        }
        if self.state == PluginState::Processing {
            if c == ')' || c == '\n' {
                self.reset();
            }
            return self.include_delimiters;
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
        self.buffer.clear();
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownBlockQuotePlugin {
    include_marker: bool,
    state: PluginState,
}

impl StreamMarkdownBlockQuotePlugin {
    pub fn new(include_marker: bool) -> Self {
        Self {
            include_marker,
            state: PluginState::Idle,
        }
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
        if self.state == PluginState::Processing {
            if c == '\n' {
                self.reset();
            }
            return true;
        }
        if at_start_of_line && c == '>' {
            self.state = PluginState::Processing;
            return self.include_marker;
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
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownHorizontalRulePlugin {
    include_marker: bool,
    state: PluginState,
    marker: Option<char>,
    count: usize,
}

impl StreamMarkdownHorizontalRulePlugin {
    pub fn new(include_marker: bool) -> Self {
        Self {
            include_marker,
            state: PluginState::Idle,
            marker: None,
            count: 0,
        }
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
        if at_start_of_line && matches!(c, '-' | '*' | '_') {
            self.state = PluginState::Trying;
            self.marker = Some(c);
            self.count = 1;
            return self.include_marker;
        }
        if self.state == PluginState::Trying {
            if Some(c) == self.marker {
                self.count += 1;
                if self.count >= 3 {
                    self.state = PluginState::Processing;
                }
                return self.include_marker;
            }
            if c == '\n' && self.count >= 3 {
                self.reset();
                return self.include_marker;
            }
            if c != ' ' {
                self.reset();
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
        self.marker = None;
        self.count = 0;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownOrderedListPlugin {
    include_marker: bool,
    state: PluginState,
    seen_digit: bool,
    seen_dot: bool,
}

impl StreamMarkdownOrderedListPlugin {
    pub fn new(include_marker: bool) -> Self {
        Self {
            include_marker,
            state: PluginState::Idle,
            seen_digit: false,
            seen_dot: false,
        }
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
        if at_start_of_line && c.is_ascii_digit() {
            self.state = PluginState::Trying;
            self.seen_digit = true;
            return self.include_marker;
        }
        if self.state == PluginState::Trying {
            if c.is_ascii_digit() {
                return self.include_marker;
            }
            if c == '.' && self.seen_digit {
                self.seen_dot = true;
                return self.include_marker;
            }
            if c == ' ' && self.seen_dot {
                self.state = PluginState::Processing;
                return self.include_marker;
            }
            self.reset();
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
        self.seen_digit = false;
        self.seen_dot = false;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownUnorderedListPlugin {
    include_marker: bool,
    state: PluginState,
    marker_seen: bool,
}

impl StreamMarkdownUnorderedListPlugin {
    pub fn new(include_marker: bool) -> Self {
        Self {
            include_marker,
            state: PluginState::Idle,
            marker_seen: false,
        }
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
        if at_start_of_line && matches!(c, '-' | '+' | '*') {
            self.state = PluginState::Trying;
            self.marker_seen = true;
            return self.include_marker;
        }
        if self.state == PluginState::Trying {
            if c == ' ' && self.marker_seen {
                self.state = PluginState::Processing;
                return self.include_marker;
            }
            self.reset();
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
        self.marker_seen = false;
    }
}

#[derive(Debug, Clone)]
pub struct StreamMarkdownTablePlugin {
    include_delimiters: bool,
    state: PluginState,
    table_row_count: usize,
    found_header_separator: bool,
}

impl StreamMarkdownTablePlugin {
    pub fn new(include_delimiters: bool) -> Self {
        Self {
            include_delimiters,
            state: PluginState::Idle,
            table_row_count: 0,
            found_header_separator: false,
        }
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
        if c == '\n' && self.state == PluginState::Processing {
            self.state = PluginState::WaitFor;
            return true;
        }
        if self.state == PluginState::WaitFor {
            if at_start_of_line && c == '|' {
                self.state = PluginState::Processing;
                self.table_row_count += 1;
                return self.include_delimiters;
            }
            self.reset();
            return true;
        }
        if at_start_of_line && c == '|' {
            self.state = PluginState::Processing;
            self.table_row_count += 1;
            return self.include_delimiters;
        }
        if self.state == PluginState::Processing {
            if self.table_row_count == 2 && matches!(c, '-' | ':' | ' ' | '|') {
                self.found_header_separator = true;
            }
            return self.include_delimiters || c != '|';
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
    }
}
