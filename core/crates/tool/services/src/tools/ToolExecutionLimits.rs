pub struct ToolExecutionLimits;

impl ToolExecutionLimits {
    pub const MAX_FILE_READ_BYTES: usize = 32_000;
    pub const DEFAULT_FILE_READ_PART_LINES: usize = 200;
    pub const MAX_TEXT_RESULT_LENGTH: usize = 5_000;
    /// Maximum decoded image size inlined into model context by MCP tool results;
    /// larger payloads persist to the caller's file directory for on-demand reads.
    pub const MAX_INLINE_IMAGE_BYTES: usize = 512 * 1024;
    /// Bounds one serialized tool result in Unicode scalar values, including its XML wrapper.
    pub const MAX_SINGLE_TOOL_RESULT_MESSAGE_CHARS: usize = Self::MAX_FILE_READ_BYTES * 2;
}
