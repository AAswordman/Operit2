use crate::util::stream::plugins::BaseJsonPlugin::{BaseJsonPlugin, JsonEmitMode};
use crate::util::stream::plugins::StreamPlugin::{PluginState, StreamPlugin};

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonEmitAll;

impl JsonEmitMode for JsonEmitAll {
    fn should_emit(&self, _c: char, _in_string: bool) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "StreamJsonPlugin"
    }
}

#[derive(Debug, Clone)]
pub struct StreamJsonPlugin {
    inner: BaseJsonPlugin<JsonEmitAll>,
}

impl Default for StreamJsonPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamJsonPlugin {
    pub fn new() -> Self {
        Self {
            inner: BaseJsonPlugin::new(JsonEmitAll),
        }
    }
}

impl StreamPlugin for StreamJsonPlugin {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn state(&self) -> PluginState {
        self.inner.state()
    }

    fn process_char(&mut self, c: char, at_start_of_line: bool) -> bool {
        self.inner.process_char(c, at_start_of_line)
    }

    fn init_plugin(&mut self) -> bool {
        self.inner.init_plugin()
    }

    fn destroy(&mut self) {
        self.inner.destroy();
    }

    fn reset(&mut self) {
        self.inner.reset();
    }
}
