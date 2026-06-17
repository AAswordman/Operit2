use std::sync::Arc;

use operit_host_api::{
    HiddenTerminalCommandOutput, TerminalCloseOutput, TerminalCommandOutput, TerminalHost,
    TerminalInfo, TerminalScreenOutput, TerminalSessionInfo,
};

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{AITool, ToolExecutor, ToolValidationResult};
use crate::core::tools::ToolResultDataClasses::{
    HiddenTerminalCommandResultData, StringResultData, TerminalCommandResultData,
    TerminalInfoResultData, TerminalSessionCloseResultData, TerminalSessionCreationResultData,
    TerminalSessionScreenResultData, TerminalStreamEventData, TerminalTypeInfoData, ToolResultData,
};

const TERMINAL_SESSION_TIMEOUT_MS: u64 = 1800000;
const HIDDEN_TERMINAL_TIMEOUT_MS: u64 = 120000;

#[derive(Clone)]
pub struct StandardTerminalTools {
    pub terminalHost: Option<Arc<dyn TerminalHost>>,
}

#[derive(Clone, Copy)]
pub enum TerminalToolOperation {
    GetTerminalInfo,
    CreateSession,
    ExecuteInSession,
    ExecuteInSessionStreaming,
    ExecuteHiddenCommand,
    InputInSession,
    CloseSession,
    GetSessionScreen,
}

#[derive(Clone)]
pub struct TerminalToolExecutor {
    pub tools: StandardTerminalTools,
    pub operation: TerminalToolOperation,
}

impl StandardTerminalTools {
    pub fn new(terminalHost: Option<Arc<dyn TerminalHost>>) -> Self {
        Self { terminalHost }
    }

    #[allow(non_snake_case)]
    pub fn getTerminalInfo(&self, tool: &AITool) -> ToolResult {
        match self.host().and_then(|host| host.terminalInfo()) {
            Ok(data) => toolSuccessData(
                tool,
                ToolResultData::TerminalInfoResultData(terminalInfoResultData(&data)),
            ),
            Err(error) => toolError(
                tool,
                format!("Error getting terminal info: {}", error.message),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn createOrGetSession(&self, tool: &AITool) -> ToolResult {
        let sessionName = parameterValue(tool, "session_name");
        let terminalType = optionalParameterValue(tool, "type")
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
        match self
            .host()
            .and_then(|host| host.createOrGetSession(&sessionName, &terminalType))
        {
            Ok(data) => toolSuccessData(
                tool,
                ToolResultData::TerminalSessionCreationResultData(
                    terminalSessionCreationResultData(&data),
                ),
            ),
            Err(error) => toolError(
                tool,
                format!(
                    "Error creating or getting terminal session: {}",
                    error.message
                ),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn executeCommandInSession(&self, tool: &AITool) -> ToolResult {
        let sessionId = parameterValue(tool, "session_id");
        let command = parameterValue(tool, "command");
        let timeoutMs = timeoutParameterValue(tool, "timeout_ms", TERMINAL_SESSION_TIMEOUT_MS);
        match self
            .host()
            .and_then(|host| host.executeInSession(&sessionId, &command, timeoutMs))
        {
            Ok(data) => toolSuccessData(
                tool,
                ToolResultData::TerminalCommandResultData(terminalCommandResultData(&data)),
            ),
            Err(error) => toolError(
                tool,
                format!("Error executing terminal command: {}", error.message),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn executeCommandInSessionStream(&self, tool: &AITool) -> Vec<ToolResult> {
        let sessionId = parameterValue(tool, "session_id");
        let command = parameterValue(tool, "command");
        let timeoutMs = timeoutParameterValue(tool, "timeout_ms", TERMINAL_SESSION_TIMEOUT_MS);
        let startData = TerminalStreamEventData {
            r#type: "start".to_string(),
            command: command.clone(),
            sessionId: sessionId.clone(),
            chunk: None,
            chunkIndex: Some(0),
            receivedChars: Some(0),
        };
        let start = ToolResult {
            toolName: tool.name.clone(),
            success: true,
            result: ToolResultData::TerminalStreamEventData(startData),
            error: Some(String::new()),
        };
        match self
            .host()
            .and_then(|host| host.executeInSession(&sessionId, &command, timeoutMs))
        {
            Ok(data) => vec![
                start,
                toolSuccessData(
                    tool,
                    ToolResultData::TerminalCommandResultData(terminalCommandResultData(&data)),
                ),
            ],
            Err(error) => vec![
                start,
                toolError(
                    tool,
                    format!("Error executing terminal command: {}", error.message),
                ),
            ],
        }
    }

    #[allow(non_snake_case)]
    pub fn executeHiddenCommand(&self, tool: &AITool) -> ToolResult {
        let command = parameterValue(tool, "command");
        let terminalType = optionalParameterValue(tool, "type")
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
        let executorKey = stringParameterValue(tool, "executor_key", "default");
        let timeoutMs = timeoutParameterValue(tool, "timeout_ms", HIDDEN_TERMINAL_TIMEOUT_MS);
        match self.host().and_then(|host| {
            host.executeHiddenCommand(&command, &terminalType, &executorKey, timeoutMs)
        }) {
            Ok(data) => {
                if data.exitCode == 0 || data.timedOut {
                    toolSuccessData(
                        tool,
                        ToolResultData::HiddenTerminalCommandResultData(
                            hiddenTerminalCommandResultData(&data),
                        ),
                    )
                } else {
                    toolError(
                        tool,
                        format!(
                            "Error executing hidden terminal command: state=EXITED, error=exitCode={}\n{}",
                            data.exitCode,
                            data.output.trim()
                        ),
                    )
                }
            }
            Err(error) => toolError(
                tool,
                format!("Error executing hidden terminal command: {}", error.message),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn inputInSession(&self, tool: &AITool) -> ToolResult {
        let sessionId = parameterValue(tool, "session_id");
        let input = optionalParameterValue(tool, "input");
        let control = optionalParameterValue(tool, "control");
        match self
            .host()
            .and_then(|host| host.inputInSession(&sessionId, input.as_deref(), control.as_deref()))
        {
            Ok(data) => toolSuccessStringData(
                tool,
                StringResultData {
                    value: format!(
                        "Terminal input sent to session {}. Accepted chars: {}",
                        data.sessionId, data.acceptedChars
                    ),
                },
            ),
            Err(error) => toolError(tool, error.message),
        }
    }

    #[allow(non_snake_case)]
    pub fn closeSession(&self, tool: &AITool) -> ToolResult {
        let sessionId = parameterValue(tool, "session_id");
        match self.host().and_then(|host| host.closeSession(&sessionId)) {
            Ok(data) => toolSuccessData(
                tool,
                ToolResultData::TerminalSessionCloseResultData(terminalSessionCloseResultData(
                    &data,
                )),
            ),
            Err(error) => toolError(
                tool,
                format!(
                    "Error closing terminal session {}: {}",
                    sessionId, error.message
                ),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn getSessionScreen(&self, tool: &AITool) -> ToolResult {
        let sessionId = parameterValue(tool, "session_id");
        match self
            .host()
            .and_then(|host| host.getSessionScreen(&sessionId))
        {
            Ok(data) => toolSuccessData(
                tool,
                ToolResultData::TerminalSessionScreenResultData(terminalSessionScreenResultData(
                    &data,
                )),
            ),
            Err(error) => toolError(
                tool,
                format!("Error getting terminal session screen: {}", error.message),
            ),
        }
    }

    fn host(&self) -> Result<&dyn TerminalHost, operit_host_api::HostError> {
        self.terminalHost.as_deref().ok_or_else(|| {
            operit_host_api::HostError::new("TerminalHost is not registered for this runtime.")
        })
    }
}

impl ToolExecutor for TerminalToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        validateTerminalTool(self.operation, tool)
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        match self.operation {
            TerminalToolOperation::GetTerminalInfo => vec![self.tools.getTerminalInfo(tool)],
            TerminalToolOperation::CreateSession => vec![self.tools.createOrGetSession(tool)],
            TerminalToolOperation::ExecuteInSession => {
                vec![self.tools.executeCommandInSession(tool)]
            }
            TerminalToolOperation::ExecuteInSessionStreaming => {
                self.tools.executeCommandInSessionStream(tool)
            }
            TerminalToolOperation::ExecuteHiddenCommand => {
                vec![self.tools.executeHiddenCommand(tool)]
            }
            TerminalToolOperation::InputInSession => vec![self.tools.inputInSession(tool)],
            TerminalToolOperation::CloseSession => vec![self.tools.closeSession(tool)],
            TerminalToolOperation::GetSessionScreen => vec![self.tools.getSessionScreen(tool)],
        }
    }
}

#[allow(non_snake_case)]
fn validateTerminalTool(operation: TerminalToolOperation, tool: &AITool) -> ToolValidationResult {
    let invalid = |message: &str| ToolValidationResult {
        valid: false,
        errorMessage: message.to_string(),
    };
    match operation {
        TerminalToolOperation::ExecuteInSession
        | TerminalToolOperation::ExecuteInSessionStreaming
        | TerminalToolOperation::ExecuteHiddenCommand => {
            if parameterValue(tool, "command").is_empty() {
                return invalid("Command parameter is required");
            }
        }
        TerminalToolOperation::CreateSession => {
            if parameterValue(tool, "session_name").is_empty() {
                return invalid("session_name is required.");
            }
        }
        TerminalToolOperation::InputInSession => {
            if parameterValue(tool, "session_id").is_empty() {
                return invalid("session_id is required.");
            }
            if !hasParameter(tool, "input") && optionalParameterValue(tool, "control").is_none() {
                return invalid("At least one of input or control is required.");
            }
        }
        TerminalToolOperation::CloseSession | TerminalToolOperation::GetSessionScreen => {
            if parameterValue(tool, "session_id").is_empty() {
                return invalid("session_id is required.");
            }
        }
        TerminalToolOperation::GetTerminalInfo => {}
    }
    match operation {
        TerminalToolOperation::ExecuteInSession
        | TerminalToolOperation::ExecuteInSessionStreaming
        | TerminalToolOperation::ExecuteHiddenCommand => {
            if optionalParameterValue(tool, "timeout_ms")
                .as_deref()
                .is_some_and(|value| value.parse::<u64>().is_err())
            {
                return invalid("timeout_ms must be an integer.");
            }
        }
        _ => {}
    }
    ToolValidationResult {
        valid: true,
        errorMessage: String::new(),
    }
}

#[allow(non_snake_case)]
fn terminalCommandResultData(data: &TerminalCommandOutput) -> TerminalCommandResultData {
    TerminalCommandResultData {
        command: data.command.clone(),
        output: data.output.clone(),
        exitCode: data.exitCode,
        sessionId: data.sessionId.clone(),
        timedOut: data.timedOut,
    }
}

#[allow(non_snake_case)]
fn hiddenTerminalCommandResultData(
    data: &HiddenTerminalCommandOutput,
) -> HiddenTerminalCommandResultData {
    HiddenTerminalCommandResultData {
        command: data.command.clone(),
        output: data.output.clone(),
        exitCode: data.exitCode,
        executorKey: data.executorKey.clone(),
        timedOut: data.timedOut,
    }
}

#[allow(non_snake_case)]
fn terminalSessionCreationResultData(
    data: &TerminalSessionInfo,
) -> TerminalSessionCreationResultData {
    TerminalSessionCreationResultData {
        sessionId: data.sessionId.clone(),
        sessionName: data.sessionName.clone(),
        isNewSession: data.isNewSession,
    }
}

#[allow(non_snake_case)]
fn terminalSessionCloseResultData(data: &TerminalCloseOutput) -> TerminalSessionCloseResultData {
    TerminalSessionCloseResultData {
        sessionId: data.sessionId.clone(),
        success: data.success,
        message: data.message.clone(),
    }
}

#[allow(non_snake_case)]
fn terminalSessionScreenResultData(data: &TerminalScreenOutput) -> TerminalSessionScreenResultData {
    TerminalSessionScreenResultData {
        sessionId: data.sessionId.clone(),
        rows: data.rows,
        cols: data.cols,
        content: data.content.clone(),
        commandRunning: data.commandRunning,
    }
}

#[allow(non_snake_case)]
fn terminalInfoResultData(data: &TerminalInfo) -> TerminalInfoResultData {
    let types = data
        .types
        .iter()
        .map(|terminalType| TerminalTypeInfoData {
            terminalType: terminalType.terminalType.clone(),
            available: terminalType.available,
            description: terminalType.description.clone(),
        })
        .collect::<Vec<_>>();
    TerminalInfoResultData {
        platform: data.platform.clone(),
        defaultType: data.defaultType.clone(),
        types,
    }
}

#[allow(non_snake_case)]
fn toolSuccess(tool: &AITool, result: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: ToolResultData::StringResultData(StringResultData { value: result }),
        error: None,
    }
}

#[allow(non_snake_case)]
fn toolSuccessData(tool: &AITool, data: ToolResultData) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: data,
        error: None,
    }
}

#[allow(non_snake_case)]
fn toolSuccessStringData(tool: &AITool, data: StringResultData) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: ToolResultData::StringResultData(data),
        error: None,
    }
}

#[allow(non_snake_case)]
fn toolError(tool: &AITool, error: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: ToolResultData::StringResultData(StringResultData {
            value: String::new(),
        }),
        error: Some(error),
    }
}

#[allow(non_snake_case)]
fn parameterValue(tool: &AITool, name: &str) -> String {
    optionalParameterValue(tool, name)
        .map(|value| value.trim().to_string())
        .unwrap_or_default()
}

#[allow(non_snake_case)]
fn optionalParameterValue(tool: &AITool, name: &str) -> Option<String> {
    tool.parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .map(|parameter| parameter.value.clone())
}

#[allow(non_snake_case)]
fn hasParameter(tool: &AITool, name: &str) -> bool {
    tool.parameters
        .iter()
        .any(|parameter| parameter.name == name)
}

#[allow(non_snake_case)]
fn stringParameterValue(tool: &AITool, name: &str, defaultValue: &str) -> String {
    match optionalParameterValue(tool, name)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        Some(value) => value,
        None => defaultValue.to_string(),
    }
}

#[allow(non_snake_case)]
fn timeoutParameterValue(tool: &AITool, name: &str, defaultValue: u64) -> u64 {
    match optionalParameterValue(tool, name)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        Some(value) => value
            .parse::<u64>()
            .expect("timeout_ms must be validated before terminal tool execution"),
        None => defaultValue,
    }
}
