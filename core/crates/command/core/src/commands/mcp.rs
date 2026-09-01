use crate::commands::util::{parse_bool_arg, read_content_arg};
use crate::output::CoreCommandOutput;
use operit_host_api::HostManager::HostManager;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::data::preferences::ApiPreferences::ApiPreferences;
use operit_tools::tools::mcp::MCPManager::MCPManager;
use operit_tools::tools::mcp_runtime::plugins::MCPBridge::MCPBridge;
use operit_tools::tools::mcp_runtime::plugins::MCPStarter::{MCPStarter, StartStatus};
use operit_tools::tools::mcp_runtime::MCPLocalServer::{
    MCPLocalServer, PluginMetadata, ServerStatus,
};
use operit_tools::tools::mcp_runtime::MCPRepository::MCPRepository;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use serde_json::Value;
use std::collections::BTreeMap;

/// Runs MCP server management commands.
pub fn run_mcp_command(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let context = application.hostManager.clone();
    let command = args.first();
    match command.map(String::as_str) {
        Some("dir") => print_mcp_dir(context, output),
        Some("list") => list_mcp_servers(context, output),
        Some("show") => show_mcp_server(
            context,
            required_arg(args, 1, "operit2 mcp show <id>")?,
            output,
        ),
        Some("import") => import_mcp_config(context, args, output),
        Some("export") => export_mcp_config(context, output),
        Some("remove") => remove_mcp_server(context, args, output),
        Some("enable") => set_mcp_enabled(context, args, true, output),
        Some("disable") => set_mcp_enabled(context, args, false, output),
        Some("start") => start_mcp_server(application, args, output),
        Some("kill") => kill_mcp_server(application, args, output),
        Some("tools") => print_mcp_tools(context, args, output),
        Some("config") => print_mcp_config(context, args, output),
        Some("config-set") => save_mcp_config(context, args, output),
        Some("local-set") => save_local_mcp_server(context, args, output),
        Some("install-github") => install_mcp_from_github(application, args, output),
        Some("install-zip") => install_mcp_from_zip(application, args, output),
        Some("meta") => print_mcp_metadata(context, args, output),
        Some("meta-set") => save_mcp_metadata(context, args, output),
        Some("describe") => generate_mcp_description(application, args, output),
        Some(_) | None => {
            print_mcp_usage(output);
            Ok(())
        }
    }
}

fn print_mcp_dir(context: HostManager, output: &mut CoreCommandOutput) -> Result<(), String> {
    let server = mcp_local_server(&context);
    let configDir = server.getConfigDirectory();
    let configFile = server.getConfigFilePath();
    output.push_stdout_line(format!("MCP config directory: {configDir}"));
    output.push_stdout_line(format!("MCP config file: {configFile}"));
    output.setJsonStdout(serde_json::json!({
        "configDir": configDir,
        "configFile": configFile,
    }));
    Ok(())
}

/// Lists configured MCP servers.
fn list_mcp_servers(context: HostManager, output: &mut CoreCommandOutput) -> Result<(), String> {
    let server = mcp_local_server(&context);
    let servers = server.getAllMCPServers();
    let metadata = server.getAllPluginMetadata();
    let status = server.getAllServerStatus();
    let mut items = Vec::new();
    output.push_stdout_line(format!("MCP servers: {}", servers.len()));
    for (serverId, serverConfig) in servers {
        let enabled = server.isServerEnabled(&serverId);
        let name = metadata.get(&serverId).map(|item| item.name.clone());
        let toolCount = status
            .get(&serverId)
            .and_then(|item| item.cachedTools.as_ref())
            .map(Vec::len);
        output.push_stdout_line(format!(
            "- {} — {} — command: {} {} — name: {} — tools: {}",
            serverId,
            enabled,
            serverConfig.command,
            serverConfig.args.join(" "),
            format_optional_string(name.as_ref()),
            format_optional_usize(toolCount)
        ));
        items.push(serde_json::json!({
            "id": serverId,
            "enabled": enabled,
            "command": serverConfig.command,
            "args": serverConfig.args,
            "url": serverConfig.url,
            "type": serverConfig.r#type,
            "headerKeys": serverConfig.headers.keys().cloned().collect::<Vec<_>>(),
            "envKeys": serverConfig.env.keys().cloned().collect::<Vec<_>>(),
            "autoApprove": serverConfig.autoApprove,
            "name": name,
            "tools": toolCount,
        }));
    }
    output.setJsonStdout(serde_json::Value::Array(items));
    Ok(())
}

/// Shows one configured MCP server.
fn show_mcp_server(
    context: HostManager,
    id: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let server = mcp_local_server(&context);
    let serverConfig = server
        .getMCPServer(id)
        .ok_or_else(|| format!("MCP server not found: {id}"))?;
    let enabled = server.isServerEnabled(id);
    let headerKeys = serverConfig.headers.keys().cloned().collect::<Vec<_>>();
    let envKeys = serverConfig.env.keys().cloned().collect::<Vec<_>>();
    let metadata = server.getPluginMetadata(id);
    let status = server.getServerStatus(id);
    output.push_stdout_line(format!("MCP server: {id}"));
    output.push_stdout_line(format!("Enabled: {enabled}"));
    output.push_stdout_line(format!("Command: {}", serverConfig.command));
    output.push_stdout_line(format!("Arguments: {}", serverConfig.args.join(" ")));
    if let Some(url) = &serverConfig.url {
        output.push_stdout_line(format!("URL: {url}"));
    }
    if let Some(serverType) = &serverConfig.r#type {
        output.push_stdout_line(format!("Type: {serverType}"));
    }
    output.push_stdout_line(format!("Header keys: {}", headerKeys.join(", ")));
    output.push_stdout_line(format!("Environment keys: {}", envKeys.join(", ")));
    output.push_stdout_line(format!(
        "Auto approve: {}",
        serverConfig.autoApprove.join(", ")
    ));
    print_optional_metadata(metadata.as_ref(), output);
    print_optional_status(status.as_ref(), output);
    output.setJsonStdout(serde_json::json!({
        "id": id,
        "enabled": enabled,
        "command": serverConfig.command,
        "args": serverConfig.args,
        "url": serverConfig.url,
        "type": serverConfig.r#type,
        "headerKeys": headerKeys,
        "envKeys": envKeys,
        "autoApprove": serverConfig.autoApprove,
        "metadata": metadata,
        "status": status,
    }));
    Ok(())
}

/// Imports MCP server configuration from JSON input.
fn import_mcp_config(
    context: HostManager,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let configArg = required_arg(args, 1, "operit2 mcp import <json-or-@file>")?;
    let configJson = read_content_arg(configArg)?;
    let count = mcp_local_server(&context).mergeConfigFromJson(&configJson)?;
    output.push_stdout_line(format!("MCP servers imported: {count}"));
    output.setJsonStdout(serde_json::json!({"imported": count}));
    Ok(())
}

/// Exports MCP server configuration as JSON.
fn export_mcp_config(context: HostManager, output: &mut CoreCommandOutput) -> Result<(), String> {
    let configJson = mcp_local_server(&context).exportConfigAsJson();
    let configValue =
        serde_json::from_str::<Value>(&configJson).map_err(|error| error.to_string())?;
    output.push_stdout_line(configJson);
    output.setJsonStdout(configValue);
    Ok(())
}

/// Removes one MCP server.
fn remove_mcp_server(
    context: HostManager,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let id = required_arg(args, 1, "operit2 mcp remove <id>")?;
    mcp_local_server(&context).removeMCPServer(id)?;
    output.push_stdout_line(format!("MCP server removed: {id}"));
    output.setJsonStdout(serde_json::json!({"id": id, "removed": true}));
    Ok(())
}

/// Sets the enabled state for one MCP server.
fn set_mcp_enabled(
    context: HostManager,
    args: &[String],
    enabled: bool,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let usage = if enabled {
        "operit2 mcp enable <id>"
    } else {
        "operit2 mcp disable <id>"
    };
    let id = required_arg(args, 1, usage)?;
    mcp_local_server(&context).setServerEnabled(id, enabled)?;
    output.push_stdout_line(format!("MCP server {}: {id}", enabled_status(enabled)));
    output.setJsonStdout(serde_json::json!({"id": id, "enabled": enabled}));
    Ok(())
}

/// Starts one MCP server and prints startup progress.
fn start_mcp_server(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let context = application.hostManager.clone();
    let id = required_arg(args, 1, "operit2 mcp start <id>")?;
    require_mcp_server(&context, id)?;
    let timeoutSeconds = ApiPreferences::getInstance()
        .getMcpStartupTimeoutSeconds()
        .map_err(|error| error.to_string())?;
    let timeoutMs = timeoutSeconds.max(1) as u64 * 1000;
    let starter = MCPStarter::new(context.clone(), application.toolHandler.runtimeSupport());
    let mut statuses = Vec::new();
    let started = starter.startPluginWithTimeout(id, timeoutMs, |status| {
        statuses.push(status);
    });
    let mut statusItems = Vec::new();
    for status in &statuses {
        print_start_status(status, output);
        statusItems.push(start_status_json(status));
    }
    if !started {
        return Err(format!("MCP start failed: {id}"));
    }
    mcp_local_server(&context).updateServerStatus(
        id.to_string(),
        None,
        None,
        Some(current_time_millis()),
        None,
    )?;
    output.push_stdout_line(format!("MCP server started: {id}"));
    output.setJsonStdout(serde_json::json!({
        "id": id,
        "started": true,
        "timeoutMs": timeoutMs,
        "statuses": statusItems,
    }));
    Ok(())
}

/// Stops one MCP server and unregisters its tools.
fn kill_mcp_server(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let context = application.hostManager.clone();
    let id = required_arg(args, 1, "operit2 mcp kill <id>")?;
    require_mcp_server(&context, id)?;

    let bridgeResult = MCPBridge::getInstance(&context).unregisterMcpService(id);
    require_bridge_success(&bridgeResult)?;

    MCPManager::getInstance(context.clone()).unregisterServer(id);
    let mut toolHandler = application.toolHandler.clone();
    let removedTools = toolHandler.unregisterMcpServerTools(id);
    let removedPackage = toolHandler.unregisterMcpServerPackage(id);
    mcp_local_server(&context).updateServerStatus(
        id.to_string(),
        None,
        None,
        None,
        Some(current_time_millis()),
    )?;
    output.push_stdout_line(format!("MCP server stopped: {id}"));
    output.push_stdout_line(format!("Unregistered tools: {removedTools}"));
    output.push_stdout_line(format!("Unregistered package: {removedPackage}"));
    output.setJsonStdout(serde_json::json!({
        "id": id,
        "killed": true,
        "unregisteredTools": removedTools,
        "unregisteredPackage": removedPackage,
    }));
    Ok(())
}

/// Prints cached tools for one MCP server.
fn print_mcp_tools(
    context: HostManager,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let id = required_arg(args, 1, "operit2 mcp tools <id>")?;
    let tools = mcp_local_server(&context)
        .getCachedTools(id)
        .ok_or_else(|| format!("MCP tools not cached: {id}"))?;
    output.push_stdout_line(format!("MCP tools for {id}: {}", tools.len()));
    for tool in &tools {
        output.push_stdout_line(format!(
            "- {} — {} — schema: {}",
            tool.name, tool.description, tool.inputSchema
        ));
    }
    output.setJsonStdout(serde_json::json!({
        "id": id,
        "tools": tools,
    }));
    Ok(())
}

/// Prints saved plugin configuration for one MCP server.
fn print_mcp_config(
    context: HostManager,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let id = required_arg(args, 1, "operit2 mcp config <id>")?;
    require_mcp_server(&context, id)?;
    let config = mcp_local_server(&context).getPluginConfig(id);
    output.push_stdout_line(config.clone());
    output.setJsonStdout(serde_json::json!({
        "id": id,
        "config": config,
    }));
    Ok(())
}

/// Saves plugin configuration for one MCP server.
fn save_mcp_config(
    context: HostManager,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let id = required_arg(args, 1, "operit2 mcp config-set <id> <json-or-@file>")?;
    let configArg = required_arg(args, 2, "operit2 mcp config-set <id> <json-or-@file>")?;
    let configJson = read_content_arg(configArg)?;
    let saved = mcp_local_server(&context).savePluginConfig(id, &configJson)?;
    if !saved {
        return Err(format!("MCP config did not contain server: {id}"));
    }
    output.push_stdout_line(format!("MCP config saved: {id}"));
    output.setJsonStdout(serde_json::json!({"id": id, "configSaved": true}));
    Ok(())
}

/// Saves one local MCP server definition.
fn save_local_mcp_server(
    context: HostManager,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let usage = "operit2 mcp local-set <id> [--disabled true|false] [--env KEY=VALUE] [--approve TOOL] -- <command> [args...]";
    let id = required_arg(args, 1, usage)?;
    let parsed = parse_local_set_args(&args[2..], usage)?;
    let command = parsed.command.clone();
    let commandArgs = parsed.args.clone();
    let envKeys = parsed.env.keys().cloned().collect::<Vec<_>>();
    let disabled = parsed.disabled;
    let autoApprove = parsed.autoApprove.clone();
    mcp_local_server(&context).addOrUpdateMCPServer(
        id.to_string(),
        parsed.command,
        parsed.args,
        parsed.env,
        parsed.disabled,
        parsed.autoApprove,
    )?;
    output.push_stdout_line(format!("Local MCP server saved: {id}"));
    output.push_stdout_line(format!("Command: {command} {}", commandArgs.join(" ")));
    output.setJsonStdout(serde_json::json!({
        "id": id,
        "localSaved": true,
        "command": command,
        "args": commandArgs,
        "envKeys": envKeys,
        "disabled": disabled,
        "autoApprove": autoApprove,
    }));
    Ok(())
}

/// Installs an MCP server from a GitHub repository.
fn install_mcp_from_github(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let context = application.hostManager.clone();
    let usage = "operit2 mcp install-github <id> <repo-url> <name> <description-or-@file> <author> <version> [config-or-@file]";
    let id = required_arg(args, 1, usage)?.to_string();
    let repoUrl = required_arg(args, 2, usage)?.to_string();
    let metadata = metadata_from_install_args(args, usage)?;
    let mcpConfig = optional_content_arg(args.get(7))?;
    match MCPRepository::getInstance(&context, application.toolHandler.runtimeSupport())
        .installMCPServerWithObject(
            id.clone(),
            repoUrl.clone(),
            metadata.clone(),
            mcpConfig,
            |_| {},
        ) {
        operit_tools::tools::mcp_runtime::MCPRepository::InstallResult::Success { pluginPath } => {
            output.push_stdout_line(format!("MCP server installed: {id}"));
            output.push_stdout_line(format!("Path: {pluginPath}"));
            output.setJsonStdout(serde_json::json!({
                "id": id,
                "repoUrl": repoUrl,
                "metadata": metadata,
                "path": pluginPath,
                "installed": true,
            }));
            Ok(())
        }
        operit_tools::tools::mcp_runtime::MCPRepository::InstallResult::Error { message } => {
            Err(message)
        }
    }
}

/// Installs an MCP server from a zip archive.
fn install_mcp_from_zip(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let context = application.hostManager.clone();
    let usage = "operit2 mcp install-zip <id> <zip-path> <name> <description-or-@file> <author> <version> [config-or-@file]";
    let id = required_arg(args, 1, usage)?.to_string();
    let zipPath = required_arg(args, 2, usage)?.to_string();
    let metadata = metadata_from_install_args(args, usage)?;
    let mcpConfig = optional_content_arg(args.get(7))?;
    match MCPRepository::getInstance(&context, application.toolHandler.runtimeSupport())
        .installMCPServerFromZip(
            id.clone(),
            zipPath.clone(),
            metadata.clone(),
            mcpConfig,
            |_| {},
        ) {
        operit_tools::tools::mcp_runtime::MCPRepository::InstallResult::Success { pluginPath } => {
            output.push_stdout_line(format!("MCP server installed: {id}"));
            output.push_stdout_line(format!("Path: {pluginPath}"));
            output.setJsonStdout(serde_json::json!({
                "id": id,
                "zipPath": zipPath,
                "metadata": metadata,
                "path": pluginPath,
                "installed": true,
            }));
            Ok(())
        }
        operit_tools::tools::mcp_runtime::MCPRepository::InstallResult::Error { message } => {
            Err(message)
        }
    }
}

/// Prints plugin metadata for one MCP server.
fn print_mcp_metadata(
    context: HostManager,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let id = required_arg(args, 1, "operit2 mcp meta <id>")?;
    let metadata = mcp_local_server(&context)
        .getPluginMetadata(id)
        .ok_or_else(|| format!("MCP metadata not found: {id}"))?;
    output.push_stdout_line(format!("MCP metadata: {id}"));
    output.push_stdout_line(format!("Name: {}", metadata.name));
    output.push_stdout_line(format!("Description: {}", metadata.description));
    output.push_stdout_line(format!("Author: {}", metadata.author));
    output.push_stdout_line(format!("Version: {}", metadata.version));
    output.setJsonStdout(serde_json::json!({"id": id, "metadata": metadata}));
    Ok(())
}

/// Saves plugin metadata for one MCP server.
fn save_mcp_metadata(
    context: HostManager,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let usage = "operit2 mcp meta-set <id> <name> <description-or-@file> <author> <version>";
    let id = required_arg(args, 1, usage)?;
    require_mcp_server(&context, id)?;
    let name = required_arg(args, 2, usage)?.to_string();
    let description = read_content_arg(required_arg(args, 3, usage)?)?;
    let author = required_arg(args, 4, usage)?.to_string();
    let version = required_arg(args, 5, usage)?.to_string();
    let metadata = PluginMetadata {
        name,
        description,
        author,
        version,
    };
    mcp_local_server(&context).addOrUpdatePluginMetadata(id, metadata.clone())?;
    output.push_stdout_line(format!("MCP metadata saved: {id}"));
    output
        .setJsonStdout(serde_json::json!({"id": id, "metadata": metadata, "metadataSaved": true}));
    Ok(())
}

/// Generates and saves an MCP description.
fn generate_mcp_description(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let context = application.hostManager.clone();
    let id = required_arg(args, 1, "operit2 mcp describe <id>")?;
    let metadata = mcp_local_server(&context)
        .getPluginMetadata(id)
        .ok_or_else(|| format!("MCP metadata not found: {id}"))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    let description = runtime.block_on(
        MCPRepository::getInstance(&context, application.toolHandler.runtimeSupport())
            .generatePluginDescription(id, &metadata.name),
    )?;
    mcp_local_server(&context).addOrUpdatePluginMetadata(
        id,
        PluginMetadata {
            name: metadata.name,
            description: description.clone(),
            author: metadata.author,
            version: metadata.version,
        },
    )?;
    output.push_stdout_line(description.clone());
    output.setJsonStdout(serde_json::json!({
        "id": id,
        "description": description,
    }));
    Ok(())
}

struct LocalSetArgs {
    command: String,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    disabled: bool,
    autoApprove: Vec<String>,
}

fn parse_local_set_args(args: &[String], usage: &str) -> Result<LocalSetArgs, String> {
    let mut env = BTreeMap::new();
    let mut disabled = false;
    let mut autoApprove = Vec::new();
    let mut commandStart = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--" => {
                commandStart = Some(index + 1);
                break;
            }
            "--disabled" => {
                disabled = parse_bool_arg(args.get(index + 1), usage)?;
                index += 2;
            }
            "--env" => {
                let (key, value) = parse_key_value(required_arg(args, index + 1, usage)?)?;
                env.insert(key, value);
                index += 2;
            }
            "--approve" => {
                autoApprove.push(required_arg(args, index + 1, usage)?.to_string());
                index += 2;
            }
            _ => return Err(usage.to_string()),
        }
    }
    let start = commandStart.ok_or_else(|| usage.to_string())?;
    let command = required_arg(args, start, usage)?.to_string();
    let commandArgs = args[start + 1..].to_vec();
    Ok(LocalSetArgs {
        command,
        args: commandArgs,
        env,
        disabled,
        autoApprove,
    })
}

fn parse_key_value(value: &str) -> Result<(String, String), String> {
    let separator = value
        .find('=')
        .ok_or_else(|| format!("invalid KEY=VALUE: {value}"))?;
    let key = value[..separator].trim().to_string();
    if key.is_empty() {
        return Err(format!("invalid KEY=VALUE: {value}"));
    }
    Ok((key, value[separator + 1..].to_string()))
}

fn metadata_from_install_args(args: &[String], usage: &str) -> Result<PluginMetadata, String> {
    Ok(PluginMetadata {
        name: required_arg(args, 3, usage)?.to_string(),
        description: read_content_arg(required_arg(args, 4, usage)?)?,
        author: required_arg(args, 5, usage)?.to_string(),
        version: required_arg(args, 6, usage)?.to_string(),
    })
}

fn optional_content_arg(value: Option<&String>) -> Result<String, String> {
    value
        .map(|item| read_content_arg(item))
        .transpose()
        .map(|item| item.unwrap_or_default())
}

fn require_mcp_server(context: &HostManager, id: &str) -> Result<(), String> {
    mcp_local_server(context)
        .getMCPServer(id)
        .map(|_| ())
        .ok_or_else(|| format!("MCP server not found: {id}"))
}

/// Prints MCP metadata when it exists.
fn print_optional_metadata(metadata: Option<&PluginMetadata>, output: &mut CoreCommandOutput) {
    if let Some(metadata) = metadata {
        output.push_stdout_line(format!("Name: {}", metadata.name));
        output.push_stdout_line(format!("Description: {}", metadata.description));
        output.push_stdout_line(format!("Author: {}", metadata.author));
        output.push_stdout_line(format!("Version: {}", metadata.version));
    }
}

/// Prints MCP runtime status when it exists.
fn print_optional_status(status: Option<&ServerStatus>, output: &mut CoreCommandOutput) {
    if let Some(status) = status {
        output.push_stdout_line(format!("Last start: {}", status.lastStartTime));
        output.push_stdout_line(format!("Last stop: {}", status.lastStopTime));
        if let Some(errorMessage) = &status.errorMessage {
            output.push_stdout_line(format!("Error: {errorMessage}"));
        }
        if let Some(tools) = status.cachedTools.as_ref() {
            output.push_stdout_line(format!("Cached tools: {}", tools.len()));
        }
    }
}

/// Prints one MCP startup status message.
fn print_start_status(status: &StartStatus, output: &mut CoreCommandOutput) {
    match status {
        StartStatus::InProgress(message) => {
            output.push_stdout_line(format!("In progress: {message}"))
        }
        StartStatus::Success(message) => output.push_stdout_line(format!("Success: {message}")),
        StartStatus::Error(message) => output.push_stdout_line(format!("Error: {message}")),
        StartStatus::TerminalServiceUnavailable(message) => {
            output.push_stdout_line(format!("Terminal service unavailable: {message}"))
        }
        StartStatus::PnpmMissing(message) => {
            output.push_stdout_line(format!("pnpm missing: {message}"))
        }
    }
}

/// Builds a JSON object for one MCP startup status.
fn start_status_json(status: &StartStatus) -> Value {
    match status {
        StartStatus::InProgress(message) => {
            serde_json::json!({"status": "in_progress", "message": message})
        }
        StartStatus::Success(message) => {
            serde_json::json!({"status": "success", "message": message})
        }
        StartStatus::Error(message) => serde_json::json!({"status": "error", "message": message}),
        StartStatus::TerminalServiceUnavailable(message) => {
            serde_json::json!({"status": "terminal_service_unavailable", "message": message})
        }
        StartStatus::PnpmMissing(message) => {
            serde_json::json!({"status": "pnpm_missing", "message": message})
        }
    }
}

/// Formats an optional string reference for CLI text.
fn format_optional_string(value: Option<&String>) -> String {
    match value {
        Some(value) => value.clone(),
        None => "-".to_string(),
    }
}

/// Formats an optional usize for CLI text.
fn format_optional_usize(value: Option<usize>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "-".to_string(),
    }
}

/// Formats an enabled state as CLI text.
fn enabled_status(enabled: bool) -> &'static str {
    if enabled {
        "enabled"
    } else {
        "disabled"
    }
}

/// Checks whether an MCP bridge command succeeded.
fn require_bridge_success(value: &Value) -> Result<(), String> {
    match value.get("success").and_then(Value::as_bool) {
        Some(true) => Ok(()),
        _ => match value
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
        {
            Some(message) => Err(message.to_string()),
            None => Err("MCP bridge command failed".to_string()),
        },
    }
}

/// Returns the current Unix timestamp in milliseconds.
fn current_time_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time must be after UNIX_EPOCH")
        .as_millis() as i64
}

/// Reads a required command argument.
fn required_arg<'a>(args: &'a [String], index: usize, usage: &str) -> Result<&'a String, String> {
    args.get(index).ok_or_else(|| format!("usage: {usage}"))
}

/// Creates an MCP local server repository.
fn mcp_local_server(context: &HostManager) -> MCPLocalServer {
    MCPLocalServer::getInstance(context)
}

/// Prints MCP command usage.
fn print_mcp_usage(output: &mut CoreCommandOutput) {
    let lines = vec![
        "operit2 mcp dir",
        "operit2 mcp list",
        "operit2 mcp show <id>",
        "operit2 mcp import <json-or-@file>",
        "operit2 mcp export",
        "operit2 mcp remove <id>",
        "operit2 mcp enable <id>",
        "operit2 mcp disable <id>",
        "operit2 mcp start <id>",
        "operit2 mcp kill <id>",
        "operit2 mcp tools <id>",
        "operit2 mcp config <id>",
        "operit2 mcp config-set <id> <json-or-@file>",
        "operit2 mcp local-set <id> [--disabled true|false] [--env KEY=VALUE] [--approve TOOL] -- <command> [args...]",
        "operit2 mcp install-github <id> <repo-url> <name> <description-or-@file> <author> <version> [config-or-@file]",
        "operit2 mcp install-zip <id> <zip-path> <name> <description-or-@file> <author> <version> [config-or-@file]",
        "operit2 mcp meta <id>",
        "operit2 mcp meta-set <id> <name> <description-or-@file> <author> <version>",
        "operit2 mcp describe <id>",
    ];
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(serde_json::json!({"usage": lines}));
}
