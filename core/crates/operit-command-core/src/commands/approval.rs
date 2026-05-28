use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::core::tools::ToolPermissionSystem::{
    PermissionLevel, ToolPermissionSystem,
};

pub fn run_approval_command(
    _context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_approval_usage(output);
        return Ok(());
    }
    let permissionSystem = ToolPermissionSystem::getInstance();
    match args[0].as_str() {
        "status" => {
            let master = permissionSystem
                .getMasterSwitch()
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("master={}", master.name()));
            let overrides = permissionSystem
                .getToolPermissionOverrides()
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("overrides={}", overrides.len()));
            Ok(())
        }
        "list" => {
            let overrides = permissionSystem
                .getToolPermissionOverrides()
                .map_err(|error| error.to_string())?;
            for (toolName, level) in overrides {
                output.push_stdout_line(format!("{toolName}\t{}", level.name()));
            }
            Ok(())
        }
        "allow" | "ask" | "forbid" => {
            let level = parse_permission_level_arg(Some(args[0].as_str()))?;
            permissionSystem
                .saveMasterSwitch(level.clone())
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("master={}", level.name()));
            Ok(())
        }
        "tool" => {
            let toolName = args.get(1).ok_or_else(|| {
                "usage: operit2 approval tool <tool-name> <allow|ask|forbid|clear>".to_string()
            })?;
            match args.get(2).map(String::as_str) {
                Some("clear") => {
                    permissionSystem
                        .clearToolPermission(toolName)
                        .map_err(|error| error.to_string())?;
                    output.push_stdout_line(format!("cleared={toolName}"));
                    Ok(())
                }
                value @ (Some("allow") | Some("ask") | Some("forbid")) => {
                    let level = parse_permission_level_arg(value)?;
                    permissionSystem
                        .saveToolPermission(toolName, level.clone())
                        .map_err(|error| error.to_string())?;
                    output.push_stdout_line(format!("{toolName}={}", level.name()));
                    Ok(())
                }
                _ => Err(
                    "usage: operit2 approval tool <tool-name> <allow|ask|forbid|clear>"
                        .to_string(),
                ),
            }
        }
        _ => {
            print_approval_usage(output);
            Ok(())
        }
    }
}

fn parse_permission_level_arg(value: Option<&str>) -> Result<PermissionLevel, String> {
    match value {
        Some("allow") | Some("ALLOW") => Ok(PermissionLevel::ALLOW),
        Some("ask") | Some("ASK") => Ok(PermissionLevel::ASK),
        Some("forbid") | Some("FORBID") => Ok(PermissionLevel::FORBID),
        _ => Err("expected allow, ask, or forbid".to_string()),
    }
}

fn print_approval_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 approval status");
    output.push_stdout_line("operit2 approval list");
    output.push_stdout_line("operit2 approval <allow|ask|forbid>");
    output.push_stdout_line("operit2 approval tool <tool-name> <allow|ask|forbid|clear>");
}
