use crate::output::CoreCommandOutput;
use operit_host_api::HostManager::HostManager;
use operit_tools::tools::ToolPermissionSystem::{AiPermissionMode, ToolPermissionSystem};

pub fn run_approval_command(
    _context: HostManager,
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
            let mode = permissionSystem
                .getAiPermissionMode()
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Approval mode: {}", mode.name()));
            output.setJsonStdout(serde_json::json!({"mode": mode.name()}));
            Ok(())
        }
        "read-only" | "workspace-write" | "full" => {
            let mode = parse_ai_permission_mode_arg(Some(args[0].as_str()))?;
            permissionSystem
                .saveAiPermissionMode(mode.clone())
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Approval mode updated: {}", mode.name()));
            output.setJsonStdout(serde_json::json!({"mode": mode.name(), "updated": true}));
            Ok(())
        }
        _ => {
            print_approval_usage(output);
            Ok(())
        }
    }
}

/// Parses an AI permission mode from a command argument.
fn parse_ai_permission_mode_arg(value: Option<&str>) -> Result<AiPermissionMode, String> {
    match value {
        Some("read-only") | Some("READ_ONLY") | Some("ReadOnly") => Ok(AiPermissionMode::ReadOnly),
        Some("workspace-write") | Some("WORKSPACE_WRITE") | Some("WorkspaceWrite") => {
            Ok(AiPermissionMode::WorkspaceWrite)
        }
        Some("full") | Some("FULL") | Some("Full") => Ok(AiPermissionMode::Full),
        _ => Err("expected read-only, workspace-write, or full".to_string()),
    }
}

/// Prints approval command usage.
fn print_approval_usage(output: &mut CoreCommandOutput) {
    let lines = vec![
        "operit2 approval status",
        "operit2 approval <read-only|workspace-write|full>",
    ];
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(serde_json::json!({"usage": lines}));
}
