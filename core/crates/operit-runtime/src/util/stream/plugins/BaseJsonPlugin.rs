use crate::util::stream::Stream::StreamLogger;
use crate::util::stream::plugins::StreamPlugin::{PluginState, StreamPlugin};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum JsonType {
    None,
    Object,
    Array,
}

pub trait JsonEmitMode {
    fn should_emit(&self, c: char, in_string: bool) -> bool;
    fn name(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub struct BaseJsonPlugin<M>
where
    M: JsonEmitMode,
{
    pub state: PluginState,
    open_brace_count: i32,
    open_bracket_count: i32,
    pub in_string: bool,
    is_escaped: bool,
    json_type: JsonType,
    mode: M,
}

impl<M> BaseJsonPlugin<M>
where
    M: JsonEmitMode,
{
    pub fn new(mode: M) -> Self {
        let mut plugin = Self {
            state: PluginState::Idle,
            open_brace_count: 0,
            open_bracket_count: 0,
            in_string: false,
            is_escaped: false,
            json_type: JsonType::None,
            mode,
        };
        plugin.reset();
        plugin
    }

    fn handle_char_in_processing(&mut self, c: char) -> bool {
        if self.in_string {
            if self.is_escaped {
                self.is_escaped = false;
            } else {
                match c {
                    '\\' => self.is_escaped = true,
                    '"' => self.in_string = false,
                    _ => {}
                }
            }
        } else {
            match c {
                '"' => self.in_string = true,
                '{' if self.json_type == JsonType::Object => self.open_brace_count += 1,
                '[' if self.json_type == JsonType::Array => self.open_bracket_count += 1,
                '}' if self.json_type == JsonType::Object => {
                    self.open_brace_count -= 1;
                    if self.open_brace_count == 0 && self.open_bracket_count == 0 {
                        self.finish_processing();
                    }
                }
                ']' if self.json_type == JsonType::Array => {
                    self.open_bracket_count -= 1;
                    if self.open_bracket_count == 0 && self.open_brace_count == 0 {
                        self.finish_processing();
                    }
                }
                _ => {}
            }
        }
        self.mode.should_emit(c, self.in_string)
    }

    fn finish_processing(&mut self) {
        StreamLogger::d(self.mode.name(), "JSON structure complete.");
        self.reset();
    }
}

impl<M> StreamPlugin for BaseJsonPlugin<M>
where
    M: JsonEmitMode,
{
    fn name(&self) -> &'static str {
        self.mode.name()
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn process_char(&mut self, c: char, _at_start_of_line: bool) -> bool {
        if self.state == PluginState::Idle {
            match c {
                '{' => {
                    self.state = PluginState::Processing;
                    self.json_type = JsonType::Object;
                    self.open_brace_count = 1;
                    return self.mode.should_emit(c, self.in_string);
                }
                '[' => {
                    self.state = PluginState::Processing;
                    self.json_type = JsonType::Array;
                    self.open_bracket_count = 1;
                    return self.mode.should_emit(c, self.in_string);
                }
                _ => return false,
            }
        }
        if self.state == PluginState::Processing {
            return self.handle_char_in_processing(c);
        }
        false
    }

    fn init_plugin(&mut self) -> bool {
        self.reset();
        true
    }

    fn destroy(&mut self) {}

    fn reset(&mut self) {
        self.state = PluginState::Idle;
        self.open_brace_count = 0;
        self.open_bracket_count = 0;
        self.in_string = false;
        self.is_escaped = false;
        self.json_type = JsonType::None;
    }
}
