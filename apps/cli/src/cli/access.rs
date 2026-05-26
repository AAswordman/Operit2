use super::core::CliCore;
use super::*;

pub(super) async fn run_approval_command(
    core: &mut CliCore,
    args: &[String],
) -> Result<(), String> {
    if args.is_empty() {
        print_approval_usage();
        return Ok(());
    }
    match args[0].as_str() {
        "status" => {
            let master = core
                .permissions_tool_permission_system()
                .getMasterSwitch()
                .await
                .map_err(|error| error.to_string())?;
            println!("master={}", master.name());
            let overrides = core
                .permissions_tool_permission_system()
                .getToolPermissionOverrides()
                .await
                .map_err(|error| error.to_string())?;
            println!("overrides={}", overrides.len());
        }
        "list" => {
            let overrides = core
                .permissions_tool_permission_system()
                .getToolPermissionOverrides()
                .await
                .map_err(|error| error.to_string())?;
            for (toolName, level) in overrides {
                println!("{toolName}\t{}", level.name());
            }
        }
        "allow" | "ask" | "forbid" => {
            let level = parse_permission_level_arg(Some(args[0].as_str()))?;
            core.permissions_tool_permission_system()
                .saveMasterSwitch(level.clone())
                .await
                .map_err(|error| error.to_string())?;
            println!("master={}", level.name());
        }
        "tool" => {
            let toolName = args.get(1).ok_or_else(|| {
                "usage: operit2 approval tool <tool-name> <allow|ask|forbid|clear>".to_string()
            })?;
            match args.get(2).map(String::as_str) {
                Some("clear") => {
                    core.permissions_tool_permission_system()
                        .clearToolPermission(toolName)
                        .await
                        .map_err(|error| error.to_string())?;
                    println!("cleared={toolName}");
                }
                value @ (Some("allow") | Some("ask") | Some("forbid")) => {
                    let level = parse_permission_level_arg(value)?;
                    core.permissions_tool_permission_system()
                        .saveToolPermission(toolName, level.clone())
                        .await
                        .map_err(|error| error.to_string())?;
                    println!("{toolName}={}", level.name());
                }
                _ => {
                    return Err(
                        "usage: operit2 approval tool <tool-name> <allow|ask|forbid|clear>"
                            .to_string(),
                    );
                }
            }
        }
        _ => print_approval_usage(),
    }
    Ok(())
}

pub(super) async fn run_tool_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_tool_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "list" => {
            let scope = args.get(1).map(String::as_str).unwrap_or("public");
            let names = match scope {
                "public" => core
                    .permissions_a_itool_handler()
                    .getPublicToolNames()
                    .await
                    .map_err(|error| error.to_string())?,
                "internal" => core
                    .permissions_a_itool_handler()
                    .getInternalToolNames()
                    .await
                    .map_err(|error| error.to_string())?,
                "all" => core
                    .permissions_a_itool_handler()
                    .getAllToolNames()
                    .await
                    .map_err(|error| error.to_string())?,
                _ => return Err("usage: operit2 tool list [public|internal|all]".to_string()),
            };
            for name in names {
                let visibility = match core
                    .permissions_a_itool_handler()
                    .getToolVisibility(&name)
                    .await
                    .map_err(|error| error.to_string())?
                {
                    Some(value) => format!("{value:?}"),
                    None => "none".to_string(),
                };
                println!("{name}\tvisibility={visibility}");
            }
        }
        "show" => {
            let toolName = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tool show <tool-name>".to_string())?;
            println!("name={toolName}");
            println!(
                "registered={}",
                core.permissions_a_itool_handler()
                    .hasToolExecutor(toolName)
                    .await
                    .map_err(|error| error.to_string())?
            );
            match core
                .permissions_a_itool_handler()
                .getToolVisibility(toolName)
                .await
                .map_err(|error| error.to_string())?
            {
                Some(visibility) => println!("visibility={visibility:?}"),
                None => println!("visibility=none"),
            }
            let permission = core
                .permissions_tool_permission_system()
                .getToolPermission(toolName)
                .await
                .map_err(|error| error.to_string())?;
            let overridePermission = core
                .permissions_tool_permission_system()
                .getToolPermissionOverride(toolName)
                .await
                .map_err(|error| error.to_string())?;
            println!("permission={}", permission.name());
            match overridePermission {
                Some(level) => println!("permissionOverride={}", level.name()),
                None => println!("permissionOverride=none"),
            }
        }
        "exec" => {
            let toolName = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tool exec <tool-name> <params-json>".to_string())?;
            let paramsJson = args
                .get(2)
                .ok_or_else(|| "usage: operit2 tool exec <tool-name> <params-json>".to_string())?;
            let tool = AITool {
                name: toolName.clone(),
                parameters: parse_tool_parameters_json(paramsJson)?,
            };
            let result = core
                .permissions_a_itool_handler()
                .executeTool(tool)
                .await
                .map_err(|error| error.to_string())?;
            print_tool_execution_result(&result)?;
        }
        _ => print_tool_usage(),
    }
    Ok(())
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
    result: &operit_runtime::api::chat::enhance::ConversationMarkupManager::ToolResult,
) -> Result<(), String> {
    println!("toolName={}", result.toolName);
    println!("success={}", result.success);
    if result.success {
        println!("{}", result.result);
        Ok(())
    } else {
        if !result.result.trim().is_empty() {
            println!("{}", result.result);
        }
        match result.error.clone() {
            Some(error) => Err(error),
            None => Err("tool execution failed without error message".to_string()),
        }
    }
}
