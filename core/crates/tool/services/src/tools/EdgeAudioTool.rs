//! Plugin-facing controls; recording bytes never enter generic Edge port calls.
use crate::runtime_support::{edge_audio::EdgeAudioRegistry, ToolRuntimeSupport};
use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::{
    AITool, AsyncToolExecutor, ToolAccessSpec, ToolBoundary, ToolEffect, ToolInvocationFuture,
    ToolValidationResult,
};
use operit_host_api::AudioInputFormat;
use operit_link::{toCoreValue, CoreCallRequest};
use operit_plugin_sdk::js_sdk::{edge::EdgeAudioInputOptions, results::*};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub enum EdgeAudioOperation {
    List,
    Open,
    Read,
    Close,
}
pub struct EdgeAudioToolExecutor {
    pub operation: EdgeAudioOperation,
    pub runtime: Arc<dyn ToolRuntimeSupport>,
}
fn parameter<'a>(tool: &'a AITool, name: &str) -> Result<&'a str, String> {
    let mut values = tool.parameters.iter().filter(|p| p.name == name);
    let value = values.next().ok_or_else(|| format!("Missing {name}"))?;
    if values.next().is_some() {
        return Err(format!("Duplicate {name}"));
    }
    Ok(&value.value)
}
fn validate(tool: &AITool, operation: EdgeAudioOperation) -> Result<(), String> {
    let keys: &[&str] = match operation {
        EdgeAudioOperation::List => &["node_id"],
        // Both generated JS and Rust SDK bindings flatten an `options` argument.
        EdgeAudioOperation::Open => &["node_id", "input_id", "format", "max_duration_ms"],
        EdgeAudioOperation::Read | EdgeAudioOperation::Close => &["stream_id"],
    };
    let minimum = if matches!(operation, EdgeAudioOperation::Open) {
        2
    } else {
        1
    };
    if tool.parameters.len() < minimum
        || tool.parameters.len() > keys.len()
        || tool
            .parameters
            .iter()
            .any(|p| !keys.contains(&p.name.as_str()))
    {
        return Err("Unexpected audio parameters".into());
    }
    for parameterValue in &tool.parameters {
        let value = parameter(tool, &parameterValue.name)?;
        if value.is_empty() || value.len() > 1024 {
            return Err(format!("Invalid {}", parameterValue.name));
        }
    }
    if keys.contains(&"node_id") {
        let id = parameter(tool, "node_id")?;
        if id.len() > 128 || id.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err("Invalid Edge node ID".into());
        }
    } else if uuid::Uuid::parse_str(parameter(tool, "stream_id")?).is_err() {
        return Err("Invalid audio stream ID".into());
    }
    if matches!(operation, EdgeAudioOperation::Open) {
        options(tool)?;
    }
    Ok(())
}
fn options(tool: &AITool) -> Result<(EdgeAudioInputOptions, AudioInputFormat, u32), String> {
    let inputId = parameter(tool, "input_id")?.to_string();
    let format = if tool.parameters.iter().any(|p| p.name == "format") {
        serde_json::from_str(parameter(tool, "format")?).map_err(|e| e.to_string())?
    } else {
        None
    };
    let maxDurationMs = if tool.parameters.iter().any(|p| p.name == "max_duration_ms") {
        serde_json::from_str(parameter(tool, "max_duration_ms")?).map_err(|e| e.to_string())?
    } else {
        None
    };
    let options = EdgeAudioInputOptions {
        inputId,
        format,
        maxDurationMs,
    };
    if options.inputId.is_empty() || options.inputId.len() > 128 {
        return Err("Invalid audio input ID".into());
    }
    let format: AudioInputFormat = match &options.format {
        Some(format) => {
            serde_json::from_value(serde_json::to_value(format).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?
        }
        None => AudioInputFormat::default(),
    };
    format.validate()?;
    let duration = options.maxDurationMs.unwrap_or(60_000);
    if duration == 0 || duration > 300_000 {
        return Err("maxDurationMs must be 1..300000".into());
    }
    Ok((options, format, duration))
}
/// Cancellation before returning a stream revokes its ingress reservation.
struct OpenGuard {
    registry: Arc<EdgeAudioRegistry>,
    id: String,
    returned: bool,
}
impl Drop for OpenGuard {
    fn drop(&mut self) {
        if !self.returned {
            self.registry.close(&self.id);
        }
    }
}
impl EdgeAudioToolExecutor {
    async fn call(
        &self,
        node: &str,
        method: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.runtime
            .executeEdgeTool(
                node.into(),
                CoreCallRequest::new(
                    format!("audio-{}", uuid::Uuid::new_v4()),
                    "edge.audio",
                    method,
                    toCoreValue(args).map_err(|e| e.to_string())?,
                ),
            )
            .await
    }
    async fn execute(&self, tool: &AITool) -> Result<ToolResultData, String> {
        validate(tool, self.operation)?;
        match self.operation {
            EdgeAudioOperation::List => {
                let node = parameter(tool, "node_id")?;
                let inputs = self.call(node, "listInputs", serde_json::json!({})).await?;
                let inputs: Vec<EdgeAudioInputDevice> =
                    serde_json::from_value(inputs).map_err(|e| e.to_string())?;
                if inputs.len() > 16 {
                    return Err("Too many audio inputs".into());
                }
                Ok(ToolResultData::EdgeAudioInputsResultData(
                    EdgeAudioInputsResultData {
                        nodeId: node.into(),
                        inputs,
                    },
                ))
            }
            EdgeAudioOperation::Open => {
                let node = parameter(tool, "node_id")?;
                let (options, format, duration) = options(tool)?;
                let registry = self.runtime.edgeAudioRegistry()?;
                let id = registry.register(node.into(), format.clone(), duration)?;
                let mut guard = OpenGuard {
                    registry,
                    id: id.clone(),
                    returned: false,
                };
                let ack = self
                    .call(
                        node,
                        "startInput",
                        serde_json::json!({ "streamId":id,
                    "inputId":options.inputId, "format":format, "maxDurationMs":duration }),
                    )
                    .await?;
                if ack["streamId"].as_str() != Some(&id) {
                    return Err("Audio start acknowledgement mismatch".into());
                }
                guard.returned = true;
                Ok(ToolResultData::EdgeAudioStreamResultData(
                    EdgeAudioStreamResultData {
                        nodeId: node.into(),
                        streamId: id,
                        format: serde_json::from_value(
                            serde_json::to_value(format).map_err(|e| e.to_string())?,
                        )
                        .map_err(|e| e.to_string())?,
                        maxDurationMs: duration,
                    },
                ))
            }
            EdgeAudioOperation::Read => Ok(ToolResultData::EdgeAudioReadResultData(
                self.runtime
                    .edgeAudioRegistry()?
                    .read(parameter(tool, "stream_id")?)
                    .await?,
            )),
            EdgeAudioOperation::Close => {
                let id = parameter(tool, "stream_id")?;
                let node = self.runtime.edgeAudioRegistry()?.close(id);
                if let Some(node) = node {
                    self.call(&node, "stopInput", serde_json::json!({"streamId":id}))
                        .await?;
                }
                Ok(ToolResultData::BooleanResultData(BooleanResultData {
                    value: true,
                }))
            }
        }
    }
}
impl AsyncToolExecutor for EdgeAudioToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        match validate(tool, self.operation) {
            Ok(()) => ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            },
            Err(errorMessage) => ToolValidationResult {
                valid: false,
                errorMessage,
            },
        }
    }
    fn accessSpec(&self, tool: &AITool) -> Result<ToolAccessSpec, String> {
        validate(tool, self.operation)?;
        Ok(ToolAccessSpec {
            effect: match self.operation {
                EdgeAudioOperation::List | EdgeAudioOperation::Read => ToolEffect::READ,
                _ => ToolEffect::WRITE,
            },
            boundary: ToolBoundary::None,
        })
    }
    fn invokeAndStreamAsync<'a>(&'a mut self, tool: &'a AITool) -> ToolInvocationFuture<'a> {
        Box::pin(async move {
            vec![match self.execute(tool).await {
                Ok(result) => ToolResult {
                    toolName: tool.name.clone(),
                    success: true,
                    result,
                    error: None,
                },
                Err(error) => ToolResult {
                    toolName: tool.name.clone(),
                    success: false,
                    result: stringResultData(String::new()),
                    error: Some(error),
                },
            }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolExecutionManager::ToolParameter;
    fn invocation(fields: &[(&str, &str)]) -> AITool {
        AITool {
            name: "edge_open_audio_input".into(),
            parameters: fields
                .iter()
                .map(|(name, value)| ToolParameter {
                    name: (*name).into(),
                    value: (*value).into(),
                })
                .collect(),
        }
    }
    #[test]
    fn audio_options_match_generated_flattened_sdk_parameters() {
        let tool = invocation(&[
            ("node_id", "edge"),
            ("input_id", "mic0"),
            (
                "format",
                r#"{"encoding":"pcm_s16le","sampleRateHz":16000,"channels":1}"#,
            ),
            ("max_duration_ms", "20000"),
        ]);
        validate(&tool, EdgeAudioOperation::Open).unwrap();
        let (options, format, duration) = options(&tool).unwrap();
        assert_eq!(options.inputId, "mic0");
        assert_eq!(format, AudioInputFormat::default());
        assert_eq!(duration, 20_000);
        let default = invocation(&[("node_id", "edge"), ("input_id", "mic0")]);
        validate(&default, EdgeAudioOperation::Open).unwrap();
        assert_eq!(super::options(&default).unwrap().2, 60_000);
        // Rust SDK Option fields serialize as null; JS omits absent fields.
        let nulls = invocation(&[
            ("node_id", "edge"),
            ("input_id", "mic0"),
            ("format", "null"),
            ("max_duration_ms", "null"),
        ]);
        validate(&nulls, EdgeAudioOperation::Open).unwrap();
    }
    #[test]
    fn audio_options_reject_routes_duplicates_and_invalid_duration() {
        for fields in [
            vec![
                ("node_id", "edge"),
                ("input_id", "mic0"),
                ("receiver_node_id", "other-core"),
            ],
            vec![
                ("node_id", "edge"),
                ("input_id", "mic0"),
                ("max_duration_ms", "0"),
            ],
            vec![
                ("node_id", "edge"),
                ("input_id", "mic0"),
                ("max_duration_ms", "300001"),
            ],
            vec![
                ("node_id", "edge"),
                ("input_id", "mic0"),
                ("node_id", "other-edge"),
            ],
            vec![("node_id", "edge"), ("options", r#"{"inputId":"mic0"}"#)],
        ] {
            assert!(validate(&invocation(&fields), EdgeAudioOperation::Open).is_err());
        }
    }
    #[tokio::test]
    async fn audio_open_cancellation_revokes_reserved_ingress() {
        let registry = Arc::new(EdgeAudioRegistry::new());
        let id = registry
            .register("edge".into(), AudioInputFormat::default(), 1000)
            .unwrap();
        drop(OpenGuard {
            registry: registry.clone(),
            id: id.clone(),
            returned: false,
        });
        assert!(registry.read(&id).await.is_err());
    }
}
