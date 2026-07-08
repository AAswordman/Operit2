pub struct ToolExecutionLimits;

impl ToolExecutionLimits {
    pub const MAX_FILE_READ_BYTES: usize = 32_000;
    pub const DEFAULT_FILE_READ_PART_LINES: usize = 200;
    pub const MAX_TEXT_RESULT_LENGTH: usize = 5_000;
    pub const MAX_FINAL_TOOL_RESULT_MESSAGE_CHARS: usize = Self::MAX_FILE_READ_BYTES * 2;
}
