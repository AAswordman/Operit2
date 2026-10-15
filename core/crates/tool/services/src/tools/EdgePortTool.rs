//! Core plugin-to-Edge port execution; no second plugin runtime or transport client.
use crate::runtime_support::ToolRuntimeSupport;
use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::{
    AITool, AsyncToolExecutor, ToolAccessSpec, ToolBoundary, ToolEffect, ToolInvocationFuture,
    ToolValidationResult,
};
use operit_link::{toCoreValue, CoreCallRequest};
use operit_plugin_sdk::javascript::{
    JsDeviceToolAction, JsDeviceToolCall, JsToolCallRequest, JsToolCallResultData,
};
use operit_plugin_sdk::js_sdk::edge::{EdgeInterfaceInfo, IoInterfaceInfo};
use operit_plugin_sdk::js_sdk::results::{stringResultData, EdgePortResultData, ToolResultData};
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
/// Maps native device calls. Device selection and routing remain transport-owned.
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
        if !identifier(&interface.target, 108) || !identifier(&interface.method, 64)
            || !interface.target.starts_with("edge.") || interface.target == "edge.plugins" {
            return Err("Invalid native Edge service target or method".into());
        }
        // Native requests carry their arguments directly, without a plugin envelope.
        let request = CoreCallRequest::new(
            operit_link::nextCoreRouteRequestId("edgePort"), interface.target,
            interface.method, toCoreValue(args).map_err(|_| "Edge arguments cannot be encoded")?,
        );
        return Ok((node.to_owned(), request, ToolEffect::WRITE));
    };
    let request = CoreCallRequest::new(
        operit_link::nextCoreRouteRequestId("edgePort"),
        target,
        method,
        toCoreValue(args).map_err(|_| "Edge arguments cannot be encoded")?,
    );
    Ok((node.to_owned(), request, effect))
}
/// Maps a device-selected hardware call to the validated AI/CLI firmware surface.
/// Device selection replaces node_id rather than permitting a second destination.
pub fn prepare_device_port(
    node: &str,
    request: &JsToolCallRequest,
) -> Result<(CoreCallRequest, ToolEffect), String> {
    let io = match request.qualified_tool_name().as_str() {
        "edge_execute" => false,
        "io_execute" => true,
        _ => return Err("Only Edge.execute and Io.execute select hardware ports".into()),
    };
    if request.parameters.contains_key("node_id") {
        return Err("A device-selected hardware call cannot override node_id".into());
    }
    let mut tool = crate::tools::DeviceTools::tool_from_request(request);
    tool.parameters
        .push(crate::ToolExecutionManager::ToolParameter {
            name: "node_id".into(),
            value: node.into(),
        });
    let (_, call, effect) = prepare(&tool, io)?;
    Ok((call, effect))
}

/// Enforces the same result limits for AI/CLI and device-selected firmware calls.
pub fn port_result(node: String, data: serde_json::Value) -> Result<EdgePortResultData, String> {
    if serde_json::to_vec(&data)
        .map(|v| v.len() > MAX_REPLY_BYTES)
        .unwrap_or(true)
    {
        return Err(
            "Edge result exceeds 4096 bytes; chunked/streaming I/O is not implemented yet".into(),
        );
    }
    Ok(EdgePortResultData { nodeId: node, data })
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
    fn invokeAndStreamAsync<'a>(&'a mut self, tool: &'a AITool, context: &'a crate::ToolExecutionManager::ToolInvocationContext) -> ToolInvocationFuture<'a> {
        Box::pin(async move {
            let result = match prepare(tool, self.io) {
                Ok((node, _, _)) => {
                    let routeState = self.runtime.coreNodeRouteState().ok();
                    let node = match canonicalEdgeNodeReference(routeState.as_ref(), &node) {
                        Ok(canonicalNode) => canonicalNode,
                        Err(error) => return vec![failure(tool, error)],
                    };
                    let request = JsToolCallRequest {
                        tool_type: "default".into(),
                        tool_name: tool.name.clone(),
                        parameters: tool
                            .parameters
                            .iter()
                            .filter(|p| p.name != "node_id")
                            .map(|p| {
                                (
                                    p.name.clone(),
                                    serde_json::from_str(&p.value)
                                        .unwrap_or_else(|_| p.value.clone().into()),
                                )
                            })
                            .collect(),
                        device: Some(JsDeviceToolCall {
                            node_id: node,
                            invocation_id: operit_link::nextCoreRouteRequestId("devicePort"),
                            action: JsDeviceToolAction::Port,
                            timeout_ms: 120_000,
                        }),
                    };
                    let result = self.runtime.executeDeviceTool(request, None).await;
                    if !result.success {
                        failure(
                            tool,
                            result.error.unwrap_or_else(|| "Device port failed".into()),
                        )
                    } else if let JsToolCallResultData::Value(value) = result.data {
                        match serde_json::from_value(value) {
                            Ok(port) => ToolResult {
                                toolName: tool.name.clone(),
                                success: true,
                                result: ToolResultData::EdgePortResultData(port),
                                error: None,
                            },
                            Err(error) => {
                                failure(tool, format!("Invalid device port result: {error}"))
                            }
                        }
                    } else {
                        failure(tool, "Invalid binary hardware result".into())
                    }
                }
                Err(error) => failure(tool, error),
            };
            vec![result]
        })
    }
}

/// Canonicalizes one Edge node reference through the live route snapshot.
///
/// A unique name or id prefix becomes the canonical node id. An ambiguous
/// reference fails instead of guessing. Without a snapshot, or for a reference
/// the snapshot cannot see, the raw value keeps the previous pass-through: the
/// router still owns membership, policy, and reachability errors for it.
#[allow(non_snake_case)]
fn canonicalEdgeNodeReference(
    routeState: Option<&crate::runtime_support::RuntimeCoreNodeRouteState>,
    node: &str,
) -> Result<String, String> {
    use crate::runtime_support::{formatCoreNodeReferenceCandidates, CoreNodeReferenceMatch};
    let Some(state) = routeState else {
        return Ok(node.to_string());
    };
    match state.resolveNodeReference(node) {
        CoreNodeReferenceMatch::Resolved(resolved) => Ok(resolved.nodeId.clone()),
        CoreNodeReferenceMatch::Unknown => Ok(node.to_string()),
        CoreNodeReferenceMatch::Ambiguous(candidates) => Err(format!(
            "Edge node reference is ambiguous: {node}; candidates: {}",
            formatCoreNodeReferenceCandidates(&candidates)
        )),
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
    fn device_ports_validate_the_selected_destination_and_result_bound() {
        let mut request = JsToolCallRequest {
            tool_type: "default".into(),
            tool_name: "edge_execute".into(),
            parameters: std::collections::BTreeMap::from([
                (
                    "interface_info".into(),
                    serde_json::json!({"target":"edge.status","method":"read"}),
                ),
                ("args".into(), serde_json::json!({})),
            ]),
            device: None,
        };
        assert!(prepare_device_port("edge-node", &request).is_ok());
        request
            .parameters
            .insert("node_id".into(), "another-node".into());
        assert!(prepare_device_port("edge-node", &request)
            .unwrap_err()
            .contains("override"));
        request.parameters.remove("node_id");
        request.tool_name = "other_package:edge_execute".into();
        assert!(prepare_device_port("edge-node", &request).is_err());
        assert!(port_result("edge-node".into(), serde_json::json!({"ok":true})).is_ok());
        assert!(port_result(
            "edge-node".into(),
            serde_json::json!({"data":"x".repeat(MAX_REPLY_BYTES)})
        )
        .is_err());
    }

    #[test]
    fn explicit_node_ports_only_and_correct_effect() {
        let t = tool(
            false,
            serde_json::json!({"target":"edge.status","method":"read"}),
            serde_json::json!({"text":"hello"}),
        );
        let (node, req, effect) = prepare(&t, false).unwrap();
        assert_eq!(node, "edge-board");
        assert_eq!(req.target, "edge.status");
        assert_eq!(req.methodName, "read");
        assert_eq!(effect, ToolEffect::WRITE);
        assert_eq!(req.args,toCoreValue(serde_json::json!({"text":"hello"})).unwrap());
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
            serde_json::json!({"target":"edge.status","method":"read"}),
            serde_json::json!({}),
        );
        t.parameters.pop();
        let (_, request, _) = prepare(&t, false).unwrap();
        assert_eq!(
            request.args,
            toCoreValue(serde_json::json!({}))
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
            serde_json::json!({"target":"edge.status","method":"read","routeKind":"space"}),
            serde_json::json!({"pluginId":"device.status","action":"read"}),
            serde_json::json!({"target":"edge.plugins","method":"invoke"}),
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
            serde_json::json!({"target":"ok","method":"read"}),
            serde_json::json!({"text":"x".repeat(1024)}),
        );
        assert!(prepare(&t, false).is_err());
        t.parameters[2].value = "{}".into();
        t.parameters[0].value = "".into();
        assert!(prepare(&t, false).is_err());
    }

    fn routeState(nodes: Vec<(&str, &str)>) -> crate::runtime_support::RuntimeCoreNodeRouteState {
        crate::runtime_support::RuntimeCoreNodeRouteState {
            currentNodeId: "local".to_string(),
            nodes: nodes
                .into_iter()
                .map(
                    |(nodeId, displayName)| crate::runtime_support::RuntimeCoreNodeStatus {
                        nodeId: nodeId.to_string(),
                        displayName: displayName.to_string(),
                        userName: String::new(),
                        platform: "esp32".to_string(),
                        model: String::new(),
                        reachable: true,
                        canExecute: false,
                        relayHops: None,
                        relayPath: None,
                    },
                )
                .collect(),
        }
    }

    #[test]
    fn edge_node_names_and_prefixes_resolve_to_canonical_ids() {
        let state = routeState(vec![
            ("core-1791589326795-13765763d4b74d4ab76648f26ee874be", "cas"),
            (
                "core-1791589400000-58f0d4ee0efc47f59a1f1b4d514b7af0",
                "edge-board",
            ),
        ]);
        assert_eq!(
            canonicalEdgeNodeReference(Some(&state), "edge-board").unwrap(),
            "core-1791589400000-58f0d4ee0efc47f59a1f1b4d514b7af0"
        );
        assert_eq!(
            canonicalEdgeNodeReference(Some(&state), "core-17915894").unwrap(),
            "core-1791589400000-58f0d4ee0efc47f59a1f1b4d514b7af0"
        );
    }

    #[test]
    fn unseen_edge_references_keep_the_router_pass_through() {
        let state = routeState(vec![("core-1", "cas")]);
        assert_eq!(
            canonicalEdgeNodeReference(Some(&state), "core-remote-edge").unwrap(),
            "core-remote-edge"
        );
        assert_eq!(
            canonicalEdgeNodeReference(None, "edge-two").unwrap(),
            "edge-two"
        );
    }

    #[test]
    fn ambiguous_edge_names_fail_with_candidates() {
        let state = routeState(vec![("core-1", "board"), ("core-2", "board")]);
        let error = canonicalEdgeNodeReference(Some(&state), "board").unwrap_err();
        assert!(error.contains("ambiguous"), "{error}");
        assert!(error.contains("board (core-1)"), "{error}");
        assert!(error.contains("board (core-2)"), "{error}");
    }
}
