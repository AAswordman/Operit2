pub struct ToolProgressEvent {
    pub tool_name: String,
    pub message: String,
}

pub struct ToolProgressBus;

impl ToolProgressBus {
    pub const SUMMARY_PROGRESS_TOOL_NAME: &'static str = "__SUMMARY__";

    pub fn clear() {}
}
