use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputProcessingState {
    Idle,
    Processing {
        message: String,
    },
    Connecting {
        message: String,
    },
    Receiving {
        message: String,
    },
    ExecutingTool {
        toolName: String,
    },
    ToolProgress {
        toolName: String,
        progress: f32,
        message: String,
    },
    ProcessingToolResult {
        toolName: String,
    },
    Summarizing {
        message: String,
    },
    ExecutingPlan {
        message: String,
    },
    Completed,
    Error {
        message: String,
    },
}

impl InputProcessingState {
    pub fn toolProgress(toolName: String, progress: f32) -> Self {
        Self::ToolProgress {
            toolName,
            progress,
            message: String::new(),
        }
    }
}
