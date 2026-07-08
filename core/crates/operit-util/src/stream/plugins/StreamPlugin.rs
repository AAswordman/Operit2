#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PluginState {
    Idle,
    Trying,
    Processing,
    WaitFor,
}

pub trait StreamPlugin {
    fn name(&self) -> &'static str;
    fn state(&self) -> PluginState;
    fn process_char(&mut self, c: char, at_start_of_line: bool) -> bool;
    fn init_plugin(&mut self) -> bool;
    fn destroy(&mut self);
    fn reset(&mut self);
}
