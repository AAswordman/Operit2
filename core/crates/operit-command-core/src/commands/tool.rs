use crate::output::CoreCommandOutput;
use operit_runtime::api::chat::enhance::ConversationMarkupManager::ToolResult;
use operit_runtime::api::chat::enhance::ToolExecutionManager::{AITool, ToolParameter};
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::core::tools::AIToolHandler::{AIToolHandler, ToolRegistrationVisibility};
use operit_runtime::core::tools::ToolPermissionSystem::{PermissionLevel, ToolPermissionSystem};

pub fn run_tool_command(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_tool_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "list" => {
            let scope = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tool list <public|internal|all>".to_string())?;
            list_tools(context, scope, output)
        }
        "show" => {
            let tool_name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tool show <tool-name>".to_string())?;
            show_tool(context, tool_name, output)
        }
        "exec" => {
            let tool_name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tool exec <tool-name> <params-json>".to_string())?;
            let params_json = args
                .get(2)
                .ok_or_else(|| "usage: operit2 tool exec <tool-name> <params-json>".to_string())?;
            exec_tool(context, tool_name, params_json, output)
        }
        _ => {
            print_tool_usage(output);
            Ok(())
        }
    }
}

fn list_tools(
    context: OperitApplicationContext,
    scope: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let handler = tool_handler(&context);
    let names = match scope {
        "public" => handler.getPublicToolNames(),
        "internal" => handler.getInternalToolNames(),
        "all" => handler.getAllToolNames(),
        _ => return Err("usage: operit2 tool list <public|internal|all>".to_string()),
    };
    for name in names {
        let visibility: Option<ToolRegistrationVisibility> = handler.getToolVisibility(&name);
        output.push_stdout_line(format!(
            "{}\tvisibility={}",
            name,
            format_tool_visibility(visibility)
        ));
    }
    Ok(())
}

fn show_tool(
    context: OperitApplicationContext,
    tool_name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let handler = tool_handler(&context);
    output.push_stdout_line(format!("name={tool_name}"));
    output.push_stdout_line(format!("registered={}", handler.hasToolExecutor(tool_name)));
    let visibility: Option<ToolRegistrationVisibility> = handler.getToolVisibility(tool_name);
    output.push_stdout_line(format!("visibility={}", format_tool_visibility(visibility)));

    let permission_system = ToolPermissionSystem::getInstance();
    let permission: PermissionLevel = permission_system
        .getToolPermission(tool_name)
        .map_err(|error| error.to_string())?;
    output.push_stdout_line(format!("permission={}", permission.name()));
    let override_permission: Option<PermissionLevel> = permission_system
        .getToolPermissionOverride(tool_name)
        .map_err(|error| error.to_string())?;
    output.push_stdout_line(format!(
        "permissionOverride={}",
        format_permission_override(override_permission)
    ));
    Ok(())
}

fn format_tool_visibility(visibility: Option<ToolRegistrationVisibility>) -> String {
    match visibility {
        Some(value) => format!("{value:?}"),
        None => "none".to_string(),
    }
}

fn format_permission_override(permission: Option<PermissionLevel>) -> String {
    match permission {
        Some(level) => level.name().to_string(),
        None => "none".to_string(),
    }
}

pub fn exec_tool(
    context: OperitApplicationContext,
    tool_name: &str,
    params_json: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let mut handler = tool_handler(&context);
    let tool = AITool {
        name: tool_name.to_string(),
        parameters: parse_tool_parameters_json(params_json)?,
    };
    let result = handler.executeTool(tool);
    print_tool_execution_result(&result, output)
}

fn parse_tool_parameters_json(value: &str) -> Result<Vec<ToolParameter>, String> {
    let object = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(value)
        .map_err(|error| error.to_string())?;
    Ok(object
        .into_iter()
        .map(|(name, value)| ToolParameter {
            name,
            value: match value {
                serde_json::Value::String(value) => value,
                other => other.to_string(),
            },
        })
        .collect())
}

fn print_tool_execution_result(
    result: &ToolResult,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    output.push_stdout_line(format!("toolName={}", result.toolName));
    output.push_stdout_line(format!("success={}", result.success));
    let resultText = result.result.toString();
    if result.success {
        output.push_stdout_line(&resultText);
        Ok(())
    } else {
        if !resultText.trim().is_empty() {
            output.push_stdout_line(&resultText);
        }
        match result.error.clone() {
            Some(error) => Err(error),
            None => Err("tool execution failed without error message".to_string()),
        }
    }
}

fn tool_handler(context: &OperitApplicationContext) -> AIToolHandler {
    AIToolHandler::getInstance(context.clone())
}

fn print_tool_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 tool list <public|internal|all>");
    output.push_stdout_line("operit2 tool show <tool-name>");
    output.push_stdout_line("operit2 tool exec <tool-name> <params-json>");
}
