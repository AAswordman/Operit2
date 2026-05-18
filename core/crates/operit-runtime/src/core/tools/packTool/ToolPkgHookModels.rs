use super::super::super::chat::hooks::PromptTurn::PromptTurnKind;

pub struct ToolPkgXmlRenderHookComposeDslResult {
    pub dsl: String,
}

pub struct ToolPkgXmlRenderHookObjectResult {
    pub object_json: String,
}

pub struct ToolPkgToolLifecycleEventPayload {
    pub event: String,
    pub tool_name: String,
}

pub struct ToolPkgPromptTurn {
    pub kind: PromptTurnKind,
    pub content: String,
    pub tool_name: Option<String>,
}

pub struct ToolPkgPromptHookObjectResult {
    pub raw_input: Option<String>,
    pub processed_input: Option<String>,
    pub system_prompt: Option<String>,
    pub tool_prompt: Option<String>,
}
