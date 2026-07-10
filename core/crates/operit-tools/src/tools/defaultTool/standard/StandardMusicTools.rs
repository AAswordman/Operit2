use std::sync::Arc;

use operit_host_api::{AudioPlaybackHost, HostError, MusicPlaybackRequest};

use operit_tools::tools::ToolResultDataClasses::{
    stringResultData, MusicPlaybackResultData, ToolResultData,
};
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{
    AITool, ToolAccessSpec, ToolBoundary, ToolEffect, ToolExecutor, ToolValidationResult,
};

#[derive(Clone)]
pub struct StandardMusicTools {
    audioPlaybackHost: Option<Arc<dyn AudioPlaybackHost>>,
}

#[derive(Clone, Copy)]
pub enum MusicToolOperation {
    Play,
    Pause,
    Resume,
    Stop,
    Seek,
    SetVolume,
    Status,
}

#[derive(Clone)]
pub struct MusicToolExecutor {
    pub tools: StandardMusicTools,
    pub operation: MusicToolOperation,
}

impl StandardMusicTools {
    pub fn new(audioPlaybackHost: Option<Arc<dyn AudioPlaybackHost>>) -> Self {
        Self { audioPlaybackHost }
    }

    #[allow(non_snake_case)]
    fn host(&self) -> Result<&dyn AudioPlaybackHost, HostError> {
        self.audioPlaybackHost
            .as_deref()
            .ok_or_else(|| HostError::new("AudioPlaybackHost is not registered for this runtime."))
    }

    #[allow(non_snake_case)]
    fn play(&self, tool: &AITool) -> ToolResult {
        let request = MusicPlaybackRequest {
            source: parameterValue(tool, "source"),
            sourceType: parameterValue(tool, "source_type"),
            title: optionalNonEmptyParameterValue(tool, "title"),
            artist: optionalNonEmptyParameterValue(tool, "artist"),
            loopPlayback: booleanParameterValue(tool, "loop", false),
            volume: numberParameterValue(tool, "volume", 1.0),
            startPositionMs: integerParameterValue(tool, "start_position_ms", 0),
        };
        match self.host().and_then(|host| host.playMusic(request)) {
            Ok(data) => toolSuccessData(
                tool,
                ToolResultData::MusicPlaybackResultData(MusicPlaybackResultData::from(data)),
            ),
            Err(error) => toolError(tool, format!("Error playing music: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn pause(&self, tool: &AITool) -> ToolResult {
        match self.host().and_then(|host| host.pauseMusic()) {
            Ok(data) => toolSuccessData(
                tool,
                ToolResultData::MusicPlaybackResultData(MusicPlaybackResultData::from(data)),
            ),
            Err(error) => toolError(tool, format!("Error pausing music: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn resume(&self, tool: &AITool) -> ToolResult {
        match self.host().and_then(|host| host.resumeMusic()) {
            Ok(data) => toolSuccessData(
                tool,
                ToolResultData::MusicPlaybackResultData(MusicPlaybackResultData::from(data)),
            ),
            Err(error) => toolError(tool, format!("Error resuming music: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn stop(&self, tool: &AITool) -> ToolResult {
        match self.host().and_then(|host| host.stopMusic()) {
            Ok(data) => toolSuccessData(
                tool,
                ToolResultData::MusicPlaybackResultData(MusicPlaybackResultData::from(data)),
            ),
            Err(error) => toolError(tool, format!("Error stopping music: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn seek(&self, tool: &AITool) -> ToolResult {
        let positionMs = integerParameterValue(tool, "position_ms", 0);
        match self.host().and_then(|host| host.seekMusic(positionMs)) {
            Ok(data) => toolSuccessData(
                tool,
                ToolResultData::MusicPlaybackResultData(MusicPlaybackResultData::from(data)),
            ),
            Err(error) => toolError(tool, format!("Error seeking music: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn setVolume(&self, tool: &AITool) -> ToolResult {
        let volume = numberParameterValue(tool, "volume", 1.0);
        match self.host().and_then(|host| host.setMusicVolume(volume)) {
            Ok(data) => toolSuccessData(
                tool,
                ToolResultData::MusicPlaybackResultData(MusicPlaybackResultData::from(data)),
            ),
            Err(error) => toolError(
                tool,
                format!("Error setting music volume: {}", error.message),
            ),
        }
    }

    #[allow(non_snake_case)]
    fn status(&self, tool: &AITool) -> ToolResult {
        match self.host().and_then(|host| host.musicStatus()) {
            Ok(data) => toolSuccessData(
                tool,
                ToolResultData::MusicPlaybackResultData(MusicPlaybackResultData::from(data)),
            ),
            Err(error) => toolError(
                tool,
                format!("Error getting music status: {}", error.message),
            ),
        }
    }
}

impl ToolExecutor for MusicToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        validateMusicTool(self.operation, tool)
    }

    fn accessSpec(&self, _tool: &AITool) -> Result<ToolAccessSpec, String> {
        let effect = match self.operation {
            MusicToolOperation::Status => ToolEffect::READ,
            MusicToolOperation::Play
            | MusicToolOperation::Pause
            | MusicToolOperation::Resume
            | MusicToolOperation::Stop
            | MusicToolOperation::Seek
            | MusicToolOperation::SetVolume => ToolEffect::WRITE,
        };
        Ok(ToolAccessSpec {
            effect,
            boundary: ToolBoundary::None,
        })
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        let result = match self.operation {
            MusicToolOperation::Play => self.tools.play(tool),
            MusicToolOperation::Pause => self.tools.pause(tool),
            MusicToolOperation::Resume => self.tools.resume(tool),
            MusicToolOperation::Stop => self.tools.stop(tool),
            MusicToolOperation::Seek => self.tools.seek(tool),
            MusicToolOperation::SetVolume => self.tools.setVolume(tool),
            MusicToolOperation::Status => self.tools.status(tool),
        };
        vec![result]
    }
}

#[allow(non_snake_case)]
fn validateMusicTool(operation: MusicToolOperation, tool: &AITool) -> ToolValidationResult {
    let invalid = |message: &str| ToolValidationResult {
        valid: false,
        errorMessage: message.to_string(),
    };
    match operation {
        MusicToolOperation::Play => {
            if parameterValue(tool, "source").is_empty() {
                return invalid("source is required.");
            }
            let sourceType = parameterValue(tool, "source_type");
            if !matches!(sourceType.as_str(), "path" | "url" | "uri") {
                return invalid("source_type must be path, url, or uri.");
            }
            if invalidNumberParameter(tool, "volume") {
                return invalid("volume must be a number.");
            }
            if invalidIntegerParameter(tool, "start_position_ms") {
                return invalid("start_position_ms must be an integer.");
            }
        }
        MusicToolOperation::Seek => {
            if parameterValue(tool, "position_ms").is_empty() {
                return invalid("position_ms is required.");
            }
            if invalidIntegerParameter(tool, "position_ms") {
                return invalid("position_ms must be an integer.");
            }
        }
        MusicToolOperation::SetVolume => {
            if parameterValue(tool, "volume").is_empty() {
                return invalid("volume is required.");
            }
            if invalidNumberParameter(tool, "volume") {
                return invalid("volume must be a number.");
            }
        }
        MusicToolOperation::Pause
        | MusicToolOperation::Resume
        | MusicToolOperation::Stop
        | MusicToolOperation::Status => {}
    }
    ToolValidationResult {
        valid: true,
        errorMessage: String::new(),
    }
}

#[allow(non_snake_case)]
fn optionalParameterValue(tool: &AITool, name: &str) -> Option<String> {
    tool.parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .map(|parameter| parameter.value.trim().to_string())
}

#[allow(non_snake_case)]
fn parameterValue(tool: &AITool, name: &str) -> String {
    optionalParameterValue(tool, name).unwrap_or_default()
}

#[allow(non_snake_case)]
fn optionalNonEmptyParameterValue(tool: &AITool, name: &str) -> Option<String> {
    optionalParameterValue(tool, name).filter(|value| !value.is_empty())
}

#[allow(non_snake_case)]
fn booleanParameterValue(tool: &AITool, name: &str, defaultValue: bool) -> bool {
    match optionalParameterValue(tool, name) {
        Some(value) if matches!(value.as_str(), "true" | "1" | "yes" | "y" | "on") => true,
        Some(value) if matches!(value.as_str(), "false" | "0" | "no" | "n" | "off") => false,
        Some(_) => defaultValue,
        None => defaultValue,
    }
}

#[allow(non_snake_case)]
fn integerParameterValue(tool: &AITool, name: &str, defaultValue: i64) -> i64 {
    optionalParameterValue(tool, name)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .parse::<i64>()
                .expect("integer parameter must be validated")
        })
        .unwrap_or(defaultValue)
}

#[allow(non_snake_case)]
fn numberParameterValue(tool: &AITool, name: &str, defaultValue: f64) -> f64 {
    optionalParameterValue(tool, name)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .parse::<f64>()
                .expect("number parameter must be validated")
        })
        .unwrap_or(defaultValue)
}

#[allow(non_snake_case)]
fn invalidIntegerParameter(tool: &AITool, name: &str) -> bool {
    optionalParameterValue(tool, name)
        .filter(|value| !value.is_empty())
        .is_some_and(|value| value.parse::<i64>().is_err())
}

#[allow(non_snake_case)]
fn invalidNumberParameter(tool: &AITool, name: &str) -> bool {
    optionalParameterValue(tool, name)
        .filter(|value| !value.is_empty())
        .is_some_and(|value| value.parse::<f64>().is_err())
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
fn toolError(tool: &AITool, error: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: stringResultData(""),
        error: Some(error),
    }
}
