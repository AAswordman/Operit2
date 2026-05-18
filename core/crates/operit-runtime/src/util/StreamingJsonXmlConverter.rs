#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Event {
    Tag { text: String },
    Content { text: String },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum State {
    WaitBrace,
    WaitKeyQuote,
    ReadKey,
    WaitColon,
    WaitValue,
    ReadString,
    ReadPrimitive,
    Escape,
    UnicodeEscape,
    WaitComma,
}

#[derive(Debug, Clone)]
pub struct StreamingJsonXmlConverter {
    state: State,
    buffer: String,
    unicode_count: usize,
    primitive_nesting_depth: i32,
    primitive_in_string: bool,
    primitive_escape: bool,
    key_escape: bool,
    reading_complex_value: bool,
    has_open_param: bool,
}

impl Default for StreamingJsonXmlConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingJsonXmlConverter {
    pub fn new() -> Self {
        Self {
            state: State::WaitBrace,
            buffer: String::new(),
            unicode_count: 0,
            primitive_nesting_depth: 0,
            primitive_in_string: false,
            primitive_escape: false,
            key_escape: false,
            reading_complex_value: false,
            has_open_param: false,
        }
    }

    pub fn has_unfinished_param(&self) -> bool {
        self.has_open_param
    }

    pub fn feed(&mut self, chunk: &str) -> Vec<Event> {
        let mut events = Vec::new();
        for c in chunk.chars() {
            match self.state {
                State::WaitBrace => {
                    if c == '{' {
                        self.state = State::WaitKeyQuote;
                    }
                }
                State::WaitKeyQuote => {
                    if c == '"' {
                        self.state = State::ReadKey;
                        self.key_escape = false;
                        self.buffer.clear();
                    } else if c == '}' {
                        self.state = State::WaitBrace;
                    }
                }
                State::ReadKey => {
                    if self.key_escape {
                        self.buffer.push(c);
                        self.key_escape = false;
                    } else {
                        match c {
                            '\\' => self.key_escape = true,
                            '"' => {
                                events.push(Event::Tag {
                                    text: format!("\n  <param name=\"{}\">", self.buffer),
                                });
                                self.has_open_param = true;
                                self.state = State::WaitColon;
                            }
                            _ => self.buffer.push(c),
                        }
                    }
                }
                State::WaitColon => {
                    if c == ':' {
                        self.state = State::WaitValue;
                    }
                }
                State::WaitValue => {
                    if !c.is_whitespace() {
                        if c == '"' {
                            self.state = State::ReadString;
                        } else {
                            self.state = State::ReadPrimitive;
                            self.buffer.clear();
                            self.buffer.push(c);
                            self.reading_complex_value = c == '[' || c == '{';
                            self.primitive_nesting_depth = if self.reading_complex_value { 1 } else { 0 };
                            self.primitive_in_string = false;
                            self.primitive_escape = false;
                        }
                    }
                }
                State::ReadString => match c {
                    '"' => {
                        self.state = State::WaitComma;
                        events.push(Event::Tag {
                            text: "</param>".to_string(),
                        });
                        self.has_open_param = false;
                    }
                    '\\' => self.state = State::Escape,
                    _ => events.push(Event::Content {
                        text: escape_xml(&c.to_string()),
                    }),
                },
                State::Escape => {
                    if c == 'u' {
                        self.state = State::UnicodeEscape;
                        self.unicode_count = 0;
                        self.buffer.clear();
                    } else {
                        let unescaped = match c {
                            'n' => "\n".to_string(),
                            'r' => "\r".to_string(),
                            't' => "\t".to_string(),
                            'b' => "\u{0008}".to_string(),
                            'f' => "\u{000c}".to_string(),
                            '"' => "\"".to_string(),
                            '\\' => "\\".to_string(),
                            '/' => "/".to_string(),
                            _ => c.to_string(),
                        };
                        events.push(Event::Content {
                            text: escape_xml(&unescaped),
                        });
                        self.state = State::ReadString;
                    }
                }
                State::UnicodeEscape => {
                    self.buffer.push(c);
                    self.unicode_count += 1;
                    if self.unicode_count == 4 {
                        if let Ok(code) = u32::from_str_radix(&self.buffer, 16) {
                            if let Some(decoded) = char::from_u32(code) {
                                events.push(Event::Content {
                                    text: escape_xml(&decoded.to_string()),
                                });
                            }
                        }
                        self.state = State::ReadString;
                    }
                }
                State::ReadPrimitive => self.read_primitive_char(c, &mut events),
                State::WaitComma => {
                    if c == ',' {
                        self.state = State::WaitKeyQuote;
                    } else if c == '}' {
                        self.state = State::WaitBrace;
                    }
                }
            }
        }
        events
    }

    pub fn flush(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        if self.can_finalize_primitive_on_flush() {
            self.emit_primitive_param(&mut events);
        }
        events
    }

    fn read_primitive_char(&mut self, c: char, events: &mut Vec<Event>) {
        if self.reading_complex_value {
            if self.primitive_in_string {
                self.buffer.push(c);
                if self.primitive_escape {
                    self.primitive_escape = false;
                } else if c == '\\' {
                    self.primitive_escape = true;
                } else if c == '"' {
                    self.primitive_in_string = false;
                }
            } else {
                match c {
                    '"' => {
                        self.primitive_in_string = true;
                        self.buffer.push(c);
                    }
                    '[' | '{' => {
                        self.primitive_nesting_depth += 1;
                        self.buffer.push(c);
                    }
                    ']' | '}' => {
                        self.primitive_nesting_depth -= 1;
                        self.buffer.push(c);
                        if self.primitive_nesting_depth == 0 {
                            self.emit_primitive_param(events);
                            self.state = State::WaitComma;
                        }
                    }
                    _ => self.buffer.push(c),
                }
            }
        } else if c == ',' || c == '}' || c.is_whitespace() {
            self.emit_primitive_param(events);
            self.state = if c == ',' {
                State::WaitKeyQuote
            } else if c == '}' {
                State::WaitBrace
            } else {
                State::WaitComma
            };
        } else {
            self.buffer.push(c);
        }
    }

    fn reset_primitive_tracking(&mut self) {
        self.primitive_nesting_depth = 0;
        self.primitive_in_string = false;
        self.primitive_escape = false;
        self.reading_complex_value = false;
    }

    fn emit_primitive_param(&mut self, events: &mut Vec<Event>) {
        events.push(Event::Content {
            text: escape_xml(&self.buffer),
        });
        events.push(Event::Tag {
            text: "</param>".to_string(),
        });
        self.has_open_param = false;
        self.buffer.clear();
        self.reset_primitive_tracking();
    }

    fn can_finalize_primitive_on_flush(&self) -> bool {
        self.state == State::ReadPrimitive
            && !self.buffer.is_empty()
            && (!self.reading_complex_value
                || (self.primitive_nesting_depth == 0
                    && !self.primitive_in_string
                    && !self.primitive_escape))
    }
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
