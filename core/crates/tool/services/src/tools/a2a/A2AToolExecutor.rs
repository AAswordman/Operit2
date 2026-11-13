use super::A2ARepository::{build_operation, A2ARepository, PACKAGE_PREFIX};
use crate::tools::ToolResultDataClasses::stringResultData;
use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::{
    AITool, AsyncToolExecutor, ToolAccessSpec, ToolBoundary, ToolEffect, ToolInvocationContext,
    ToolInvocationFuture, ToolValidationResult,
};
use operit_host_api::HostManager::HostManager;
use serde_json::Value;

pub struct A2AToolExecutor {
    host: HostManager,
}
impl A2AToolExecutor {
    pub fn new(host: HostManager) -> Self {
        Self { host }
    }
}

fn target(tool: &AITool) -> Result<(&str, &str, Value), String> {
    let (package, operation) = tool
        .name
        .split_once(':')
        .ok_or("A2A tool must use package:operation")?;
    let id = package
        .strip_prefix(PACKAGE_PREFIX)
        .ok_or("Invalid A2A package")?;
    let mut args = serde_json::Map::new();
    for p in &tool.parameters {
        if !matches!(p.name.as_str(), "message" | "task_id" | "context_id") {
            return Err(format!("Unknown A2A parameter: {}", p.name));
        }
        if args
            .insert(p.name.clone(), Value::String(p.value.clone()))
            .is_some()
        {
            return Err(format!("Duplicate A2A parameter: {}", p.name));
        }
    }
    let args = Value::Object(args);
    build_operation("1.0", operation, &args)?;
    Ok((id, operation, args))
}
impl AsyncToolExecutor for A2AToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        match target(tool) {
            Ok(_) => ToolValidationResult {
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
        let (_, operation, _) = target(tool)?;
        Ok(ToolAccessSpec {
            effect: if operation == "get_task" {
                ToolEffect::READ
            } else {
                ToolEffect::WRITE
            },
            boundary: ToolBoundary::None,
        })
    }
    /// Executes an independent request through the owning resource services.
    fn invokeAndStreamAsync<'a>(
        &'a self,
        tool: &'a AITool,
        _context: &'a ToolInvocationContext,
    ) -> ToolInvocationFuture<'a> {
        Box::pin(async move {
            let result = async {
                let (id, operation, args) = target(tool)?;
                A2ARepository::new(self.host.clone())?
                    .invoke(id, operation, args)
                    .await
            }
            .await;
            vec![match result {
                Ok(value) => ToolResult {
                    toolName: tool.name.clone(),
                    success: true,
                    result: stringResultData(serde_json::to_string(&value).unwrap_or_default()),
                    error: None,
                },
                Err(error) => ToolResult {
                    toolName: tool.name.clone(),
                    success: false,
                    result: stringResultData(error.clone()),
                    error: Some(error),
                },
            }]
        })
    }
}
