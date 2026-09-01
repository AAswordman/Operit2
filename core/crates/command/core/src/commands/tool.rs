use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_tools::tools::AIToolHandler::{AIToolHandler, ToolRegistrationVisibility};
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{AITool, ToolParameter};

/// Runs tool inspection and execution commands.
pub fn run_tool_command(
    application: &OperitApplication,
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
            list_tools(&application.toolHandler, scope, output)
        }
        "show" => {
            let tool_name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tool show <tool-name>".to_string())?;
            show_tool(&application.toolHandler, tool_name, output)
        }
        "exec" => {
            let tool_name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tool exec <tool-name> <params-json>".to_string())?;
            let params_json = args
                .get(2)
                .ok_or_else(|| "usage: operit2 tool exec <tool-name> <params-json>".to_string())?;
            exec_tool(
                application.toolHandler.clone(),
                tool_name,
                params_json,
                output,
            )
        }
        _ => {
            print_tool_usage(output);
            Ok(())
        }
    }
}

fn list_tools(
    handler: &AIToolHandler,
    scope: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let names = match scope {
        "public" => handler.getPublicToolNames(),
        "internal" => handler.getInternalToolNames(),
        "all" => handler.getAllToolNames(),
        _ => return Err("usage: operit2 tool list <public|internal|all>".to_string()),
    };
    let mut items = Vec::new();
    output.push_stdout_line(format!("Tools ({scope}): {}", names.len()));
    for name in names {
        let visibility: Option<ToolRegistrationVisibility> = handler.getToolVisibility(&name);
        let visibilityName = format_tool_visibility(visibility);
        output.push_stdout_line(format!("- {name} ({visibilityName})"));
        items.push(serde_json::json!({
            "name": name,
            "visibility": visibilityName,
        }));
    }
    output.setJsonStdout(serde_json::json!({"scope": scope, "tools": items}));
    Ok(())
}

fn show_tool(
    handler: &AIToolHandler,
    tool_name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let registered = handler.hasToolExecutor(tool_name);
    let visibility: Option<ToolRegistrationVisibility> = handler.getToolVisibility(tool_name);
    let visibilityName = format_tool_visibility(visibility);
    output.push_stdout_line(format!("Tool: {tool_name}"));
    output.push_stdout_line(format!("Registered: {registered}"));
    output.push_stdout_line(format!("Visibility: {visibilityName}"));
    output.setJsonStdout(serde_json::json!({
        "name": tool_name,
        "registered": registered,
        "visibility": visibilityName,
    }));
    Ok(())
}

/// Formats a registered tool visibility value for command output.
fn format_tool_visibility(visibility: Option<ToolRegistrationVisibility>) -> String {
    match visibility {
        Some(value) => format!("{value:?}"),
        None => "none".to_string(),
    }
}

pub fn exec_tool(
    mut handler: AIToolHandler,
    tool_name: &str,
    params_json: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let tool = AITool {
        name: tool_name.to_string(),
        parameters: parse_tool_parameters_json(params_json)?,
    };
    let result = handler.executeTool(tool);
    print_tool_execution_result(&result, output)
}

/// Parses a JSON object into tool parameters.
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

/// Prints a tool execution result and records its JSON object.
fn print_tool_execution_result(
    result: &ToolResult,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let resultText = result.result.toString();
    output.push_stdout_line(format!("Tool: {}", result.toolName));
    output.push_stdout_line(format!("Success: {}", result.success));
    output.setJsonStdout(serde_json::to_value(result).map_err(|error| error.to_string())?);
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

/// Prints tool command usage.
fn print_tool_usage(output: &mut CoreCommandOutput) {
    let lines = vec![
        "operit2 tool list <public|internal|all>",
        "operit2 tool show <tool-name>",
        "operit2 tool exec <tool-name> <params-json>",
    ];
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(serde_json::json!({"usage": lines}));
}
