use serde_json::Value;

use operit_host_api::HostManager::HostManager;
use operit_tools::tools::mcp::MCPServerConfig::MCPServerConfig;
use operit_tools::tools::mcp::MCPTool::MCPTool;
use operit_tools::tools::mcp::MCPToolParameter::MCPToolParameter;
use operit_tools::tools::ToolPackage::{
    LocalizedText, PackageTool, PackageToolParameter, ToolPackage,
};
use crate::tools::mcp_runtime::plugins::MCPBridgeClient::MCPBridgeClient;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MCPPackage {
    pub serverConfig: MCPServerConfig,
    pub mcpTools: Vec<MCPTool>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoadResult {
    pub mcpPackage: Option<MCPPackage>,
    pub errorMessage: Option<String>,
}

impl MCPPackage {
    #[allow(non_snake_case)]
    pub fn fromServer(
        context: &HostManager,
        serverConfig: MCPServerConfig,
    ) -> Option<MCPPackage> {
        Self::loadFromServer(context, serverConfig).mcpPackage
    }

    #[allow(non_snake_case)]
    pub fn loadFromServer(
        context: &HostManager,
        serverConfig: MCPServerConfig,
    ) -> LoadResult {
        let bridgeClient = MCPBridgeClient::new(context.clone(), serverConfig.name.clone());
        if !bridgeClient.connect() {
            return LoadResult {
                mcpPackage: None,
                errorMessage: Some(
                    bridgeClient
                        .getLastConnectionFailureDetail()
                        .unwrap_or_else(|| {
                            "Connection failed, but no detailed reason was reported.".to_string()
                        }),
                ),
            };
        }

        let jsonTools = bridgeClient.getTools();
        if jsonTools.is_empty() {
            return LoadResult {
                mcpPackage: Some(MCPPackage {
                    serverConfig,
                    mcpTools: Vec::new(),
                }),
                errorMessage: None,
            };
        }

        let mcpTools = jsonTools
            .into_iter()
            .filter_map(parseMcpTool)
            .collect::<Vec<_>>();
        LoadResult {
            mcpPackage: Some(MCPPackage {
                serverConfig,
                mcpTools,
            }),
            errorMessage: None,
        }
    }

    #[allow(non_snake_case)]
    pub fn toToolPackage(&self) -> ToolPackage {
        let tools = self
            .mcpTools
            .iter()
            .map(|mcpTool| {
                let parameters = mcpTool
                    .parameters
                    .iter()
                    .map(|mcpParam| PackageToolParameter {
                        name: mcpParam.name.clone(),
                        description: LocalizedText {
                            values: std::collections::HashMap::from([(
                                "default".to_string(),
                                mcpParam.description.clone(),
                            )]),
                        },
                        parameter_type: mcpParam.parameter_type.clone(),
                        required: mcpParam.required,
                    })
                    .collect::<Vec<_>>();
                PackageTool {
                    name: mcpTool.name.clone(),
                    description: LocalizedText {
                        values: std::collections::HashMap::from([(
                            "default".to_string(),
                            mcpTool.description.clone(),
                        )]),
                    },
                    parameters,
                    script: self.generateScriptPlaceholder(&self.serverConfig.name, &mcpTool.name),
                    advice: false,
                }
            })
            .collect::<Vec<_>>();

        ToolPackage {
            name: self.serverConfig.name.clone(),
            description: LocalizedText {
                values: std::collections::HashMap::from([(
                    "default".to_string(),
                    self.serverConfig.description.clone(),
                )]),
            },
            tools,
            category: "MCP".to_string(),
            ..ToolPackage::default()
        }
    }

    #[allow(non_snake_case)]
    fn generateScriptPlaceholder(&self, serverName: &str, toolName: &str) -> String {
        format!(
            "/* MCPJS\n{{\n    \"serverName\": \"{}\",\n    \"toolName\": \"{}\",\n    \"endpoint\": \"{}\"\n}}\n*/\n// MCP tool placeholder",
            serverName, toolName, self.serverConfig.endpoint
        )
    }
}

#[allow(non_snake_case)]
fn parseMcpTool(jsonTool: Value) -> Option<MCPTool> {
    let name = jsonTool.get("name").and_then(Value::as_str)?.to_string();
    if name.is_empty() {
        return None;
    }
    let description = jsonTool
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let inputSchema = jsonTool.get("inputSchema");
    let properties = inputSchema
        .and_then(|schema| schema.get("properties"))
        .and_then(Value::as_object);
    let required = inputSchema
        .and_then(|schema| schema.get("required"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<std::collections::BTreeSet<_>>()
        })
        .unwrap_or_default();
    let parameters = properties
        .map(|properties| {
            properties
                .iter()
                .filter_map(|(paramName, paramObj)| {
                    let paramObject = paramObj.as_object()?;
                    Some(MCPToolParameter {
                        name: paramName.clone(),
                        description: paramObject
                            .get("description")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        parameter_type: paramObject
                            .get("type")
                            .and_then(Value::as_str)
                            .unwrap_or("string")
                            .to_string(),
                        required: required.contains(paramName),
                        defaultValue: paramObject
                            .get("default")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some(MCPTool {
        name,
        description,
        parameters,
    })
}
