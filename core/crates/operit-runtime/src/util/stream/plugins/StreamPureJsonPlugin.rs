use crate::util::stream::plugins::BaseJsonPlugin::{BaseJsonPlugin, JsonEmitMode};
use crate::util::stream::plugins::StreamPlugin::{PluginState, StreamPlugin};

#[derive(Debug, Clone, Copy, Default)]
pub struct PureJsonEmitMode;

impl JsonEmitMode for PureJsonEmitMode {
    fn should_emit(&self, c: char, in_string: bool) -> bool {
        if in_string {
            !matches!(c, '\\' | '"')
        } else {
            !matches!(c, '{' | '}' | '[' | ']' | ':' | ',') && !c.is_whitespace()
        }
    }

    fn name(&self) -> &'static str {
        "StreamPureJsonPlugin"
    }
}

#[derive(Debug, Clone)]
pub struct StreamPureJsonPlugin {
    inner: BaseJsonPlugin<PureJsonEmitMode>,
}

impl Default for StreamPureJsonPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamPureJsonPlugin {
    pub fn new() -> Self {
        Self {
            inner: BaseJsonPlugin::new(PureJsonEmitMode),
        }
    }
}

impl StreamPlugin for StreamPureJsonPlugin {
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
