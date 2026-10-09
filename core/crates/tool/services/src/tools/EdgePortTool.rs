//! Core plugin-to-Edge port execution; no second plugin runtime or transport client.
use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::{
    AITool, AsyncToolExecutor, ToolAccessSpec, ToolBoundary, ToolEffect, ToolInvocationFuture,
    ToolValidationResult,
};
use crate::runtime_support::ToolRuntimeSupport;
use operit_link::{CoreCallRequest, toCoreValue};
use operit_plugin_sdk::js_sdk::edge::{EdgeInterfaceInfo, IoInterfaceInfo};
use operit_plugin_sdk::js_sdk::results::{EdgePortResultData, ToolResultData, stringResultData};
use std::sync::Arc;

pub const MAX_ARGS_BYTES: usize = 1024;
pub const MAX_REPLY_BYTES: usize = 4096;

pub struct EdgePortToolExecutor {
    pub io: bool,
    pub runtime: Arc<dyn ToolRuntimeSupport>,
}

fn parameter<'a>(tool: &'a AITool, key: &str) -> Result<&'a str, String> {
    let mut values = tool.parameters.iter().filter(|p| p.name == key);
    let value = values
        .next()
        .ok_or_else(|| format!("Missing parameter: {key}"))?;
    if values.next().is_some() {
        return Err(format!("Duplicate parameter: {key}"));
    }
    Ok(&value.value)
}
fn identifier(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && !value.chars().any(|c| c.is_whitespace() || c.is_control())
}
/// Maps only known hardware entrypoints. User data cannot select a Core RPC or route kind.
fn prepare(tool: &AITool, io: bool) -> Result<(String, CoreCallRequest, ToolEffect), String> {
    if tool
        .parameters
        .iter()
        .any(|p| !matches!(p.name.as_str(), "node_id" | "interface_info" | "args"))
    {
        return Err("Unknown Edge port parameter".into());
    }
    let node = parameter(tool, "node_id")?;
    if !identifier(node, 128) {
        return Err("Invalid edge node ID".into());
    }
    let info = parameter(tool, "interface_info")?;
    if info.len() > 512 {
        return Err("Edge interface information exceeds 512 bytes".into());
    }
    let args: serde_json::Value = if tool.parameters.iter().any(|p| p.name == "args") {
        let text = parameter(tool, "args")?;
        if text.len() > MAX_ARGS_BYTES {
            return Err("Edge arguments exceed 1024 bytes".into());
        }
        serde_json::from_str(text).map_err(|_| "Invalid Edge arguments JSON")?
    } else {
        serde_json::json!({})
    };
    if !args.is_object() {
        return Err("Edge arguments must be a JSON object".into());
    }
    let (target, method, args, effect) = if io {
        let interface: IoInterfaceInfo =
            serde_json::from_str(info).map_err(|_| "Invalid I/O interface information")?;
        if interface.port != "gpio" {
            return Err("Unsupported I/O port; serial/UART is not implemented yet".into());
        }
        let pin = args["pin"]
            .as_u64()
            .filter(|p| *p <= 255)
            .ok_or("GPIO pin must be an integer between 0 and 255")?;
        let (method, args, effect) = match interface.operation.as_str() {
            "read" if args.as_object().unwrap().len() == 1 => (
                "getDigitalOutput",
                serde_json::json!({"pin":pin}),
                ToolEffect::READ,
            ),
            "write" if args.as_object().unwrap().len() == 2 => {
                let level = args["level"]
                    .as_bool()
                    .ok_or("GPIO level must be a boolean")?;
                (
                    "setDigitalOutput",
                    serde_json::json!({"pin":pin,"level":level}),
                    ToolEffect::WRITE,
                )
            }
            _ => return Err("Unsupported GPIO operation or unexpected GPIO arguments".into()),
        };
        ("edge.deviceIo", method, args, effect)
    } else {
        let interface: EdgeInterfaceInfo =
            serde_json::from_str(info).map_err(|_| "Invalid Edge interface information")?;
        if !identifier(&interface.pluginId, 108) || !identifier(&interface.action, 64) {
            return Err("Invalid Edge plugin ID or action".into());
        }
        (
            "edge.plugins",
            "invoke",
            serde_json::json!({"pluginId":interface.pluginId,"action":interface.action,"args":args}),
            ToolEffect::WRITE,
        )
    };
    let request = CoreCallRequest::new(
        operit_link::nextCoreRouteRequestId("edgePort"),
        target,
        method,
        toCoreValue(args).map_err(|_| "Edge arguments cannot be encoded")?,
    );
    Ok((node.to_owned(), request, effect))
}
fn failure(tool: &AITool, error: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: stringResultData(""),
        error: Some(error.chars().take(256).collect()),
    }
}
impl AsyncToolExecutor for EdgePortToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        match prepare(tool, self.io) {
            Ok(_) => ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            },
            Err(error) => ToolValidationResult {
                valid: false,
                errorMessage: error,
            },
        }
    }
    fn accessSpec(&self, tool: &AITool) -> Result<ToolAccessSpec, String> {
        let (_, _, effect) = prepare(tool, self.io)?;
        Ok(ToolAccessSpec {
            effect,
            boundary: ToolBoundary::None,
        })
    }
    fn invokeAndStreamAsync<'a>(&'a mut self, tool: &'a AITool) -> ToolInvocationFuture<'a> {
        Box::pin(async move {
            let result = match prepare(tool, self.io) {
                Ok((node, request, _)) => {
                    match self.runtime.executeEdgeTool(node.clone(), request).await {
                        Ok(data) => {
                            if serde_json::to_vec(&data)
                                .map(|v| v.len() > MAX_REPLY_BYTES)
                                .unwrap_or(true)
                            {
                                failure(tool,"Edge result exceeds 4096 bytes; chunked/streaming I/O is not implemented yet".into())
                            } else {
                                ToolResult {
                                    toolName: tool.name.clone(),
                                    success: true,
                                    result: ToolResultData::EdgePortResultData(
                                        EdgePortResultData { nodeId: node, data },
                                    ),
                                    error: None,
                                }
                            }
                        }
                        Err(error) => failure(tool, error),
                    }
                }
                Err(error) => failure(tool, error),
            };
            vec![result]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolExecutionManager::ToolParameter;
    fn tool(io: bool, info: serde_json::Value, args: serde_json::Value) -> AITool {
        AITool {
            name: if io { "io_execute" } else { "edge_execute" }.into(),
            parameters: vec![
                ToolParameter {
                    name: "node_id".into(),
                    value: "edge-board".into(),
                },
                ToolParameter {
                    name: "interface_info".into(),
                    value: info.to_string(),
                },
                ToolParameter {
                    name: "args".into(),
                    value: args.to_string(),
                },
            ],
        }
    }
    #[test]
    fn explicit_node_ports_only_and_correct_effect() {
        let t = tool(
            false,
            serde_json::json!({"pluginId":"device.status","action":"read"}),
            serde_json::json!({"text":"hello"}),
        );
        let (node, req, effect) = prepare(&t, false).unwrap();
        assert_eq!(node, "edge-board");
        assert_eq!(req.target, "edge.plugins");
        assert_eq!(req.methodName, "invoke");
        assert_eq!(effect, ToolEffect::WRITE);
        assert_eq!(req.args,toCoreValue(serde_json::json!({"pluginId":"device.status","action":"read","args":{"text":"hello"}})).unwrap());
        let t = tool(
            true,
            serde_json::json!({"port":"gpio","operation":"read"}),
            serde_json::json!({"pin":2}),
        );
        let (_, req, effect) = prepare(&t, true).unwrap();
        assert_eq!(req.methodName, "getDigitalOutput");
        assert_eq!(effect, ToolEffect::READ);
        let t = tool(
            true,
            serde_json::json!({"port":"gpio","operation":"write"}),
            serde_json::json!({"pin":2,"level":true}),
        );
        let (_, req, effect) = prepare(&t, true).unwrap();
        assert_eq!(req.methodName, "setDigitalOutput");
        assert_eq!(effect, ToolEffect::WRITE);
    }
    #[test]
    fn optional_object_args_and_duplicate_parameters_are_checked() {
        let mut t = tool(
            false,
            serde_json::json!({"pluginId":"device.status","action":"read"}),
            serde_json::json!({}),
        );
        t.parameters.pop();
        let (_, request, _) = prepare(&t, false).unwrap();
        assert_eq!(
            request.args,
            toCoreValue(serde_json::json!({"pluginId":"device.status","action":"read","args":{}}))
                .unwrap()
        );
        t.parameters.push(ToolParameter {
            name: "args".into(),
            value: "".into(),
        });
        assert!(
            prepare(&t, false).is_err(),
            "explicit blank args is not a JSON object"
        );
        t.parameters.last_mut().unwrap().value = "[]".into();
        assert!(prepare(&t, false).is_err());
        t.parameters.last_mut().unwrap().value = "{}".into();
        t.parameters.push(t.parameters[0].clone());
        assert!(prepare(&t, false).is_err());
    }

    #[test]
    fn rejects_route_injection_missing_ports_and_oversized_payloads() {
        for info in [
            serde_json::json!({"target":"core.internal","method":"erase"}),
            serde_json::json!({"pluginId":"ok","action":"read","target":"core.internal"}),
        ] {
            assert!(prepare(&tool(false, info, serde_json::json!({})), false).is_err());
        }
        for (info, args) in [
            (
                serde_json::json!({"port":"serial","operation":"read"}),
                serde_json::json!({"pin":2}),
            ),
            (
                serde_json::json!({"port":"gpio","operation":"write"}),
                serde_json::json!({"pin":2,"level":"true"}),
            ),
            (
                serde_json::json!({"port":"gpio","operation":"read"}),
                serde_json::json!({"pin":-1}),
            ),
            (
                serde_json::json!({"port":"gpio","operation":"read"}),
                serde_json::json!({"pin":2,"target":"core.internal"}),
            ),
        ] {
            assert!(prepare(&tool(true, info, args), true).is_err());
        }
        let mut t = tool(
            false,
            serde_json::json!({"pluginId":"ok","action":"read"}),
            serde_json::json!({"text":"x".repeat(1024)}),
        );
        assert!(prepare(&t, false).is_err());
        t.parameters[2].value = "{}".into();
        t.parameters[0].value = "".into();
        assert!(prepare(&t, false).is_err());
    }
}
