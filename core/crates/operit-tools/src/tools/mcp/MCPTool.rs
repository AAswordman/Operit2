use operit_tools::tools::mcp::MCPToolParameter::MCPToolParameter;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MCPTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<MCPToolParameter>,
}
