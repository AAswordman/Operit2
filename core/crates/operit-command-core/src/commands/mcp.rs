use crate::commands::util::{parse_bool_arg, read_content_arg};
use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::core::tools::AIToolHandler::AIToolHandler;
use operit_runtime::data::mcp::MCPLocalServer::{MCPLocalServer, PluginMetadata};
use operit_runtime::data::mcp::MCPRepository::MCPRepository;
use std::collections::BTreeMap;

pub fn run_mcp_command(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
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
        Some("start") => start_mcp_server(context, args, output),
        Some("tools") => print_mcp_tools(context, args, output),
        Some("config") => print_mcp_config(context, args, output),
        Some("config-set") => save_mcp_config(context, args, output),
        Some("local-set") => save_local_mcp_server(context, args, output),
        Some("install-github") => install_mcp_from_github(context, args, output),
        Some("install-zip") => install_mcp_from_zip(context, args, output),
        Some("meta") => print_mcp_metadata(context, args, output),
        Some("meta-set") => save_mcp_metadata(context, args, output),
        Some("describe") => generate_mcp_description(context, args, output),
        Some(_) | None => {
            print_mcp_usage(output);
            Ok(())
        }
    }
}

fn print_mcp_dir(
    context: OperitApplicationContext,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let server = mcp_local_server(&context);
    output.push_stdout_line(format!("configDir={}", server.getConfigDirectory()));
    output.push_stdout_line(format!("configFile={}", server.getConfigFilePath()));
    Ok(())
}

fn list_mcp_servers(
    context: OperitApplicationContext,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let server = mcp_local_server(&context);
    let servers = server.getAllMCPServers();
    let metadata = server.getAllPluginMetadata();
    let status = server.getAllServerStatus();
    for (serverId, serverConfig) in servers {
        let mut line = format!(
            "{}\tenabled={}\tcommand={}\targs={}",
            serverId,
            server.isServerEnabled(&serverId),
            serverConfig.command,
            serverConfig.args.join(" ")
        );
        if let Some(item) = metadata.get(&serverId) {
            line.push_str(&format!("\tname={}", item.name));
        }
        if let Some(tools) = status
            .get(&serverId)
            .and_then(|item| item.cachedTools.as_ref())
        {
            line.push_str(&format!("\ttools={}", tools.len()));
        }
        output.push_stdout_line(line);
    }
    Ok(())
}

fn show_mcp_server(
    context: OperitApplicationContext,
    id: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let server = mcp_local_server(&context);
    let serverConfig = server
        .getMCPServer(id)
        .ok_or_else(|| format!("MCP server not found: {id}"))?;
    output.push_stdout_line(format!("id={id}"));
    output.push_stdout_line(format!("enabled={}", server.isServerEnabled(id)));
    output.push_stdout_line(format!("command={}", serverConfig.command));
    output.push_stdout_line(format!("args={}", serverConfig.args.join(" ")));
    if let Some(url) = serverConfig.url {
        output.push_stdout_line(format!("url={url}"));
    }
    if let Some(serverType) = serverConfig.r#type {
        output.push_stdout_line(format!("type={serverType}"));
    }
    output.push_stdout_line(format!(
        "headerKeys={}",
        serverConfig
            .headers
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    ));
    output.push_stdout_line(format!(
        "envKeys={}",
        serverConfig
            .env
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    ));
    output.push_stdout_line(format!(
        "autoApprove={}",
        serverConfig.autoApprove.join(",")
    ));
    print_optional_metadata(&server, id, output);
    print_optional_status(&server, id, output);
    Ok(())
}

fn import_mcp_config(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let configArg = required_arg(args, 1, "operit2 mcp import <json-or-@file>")?;
    let configJson = read_content_arg(configArg)?;
    let count = mcp_local_server(&context).mergeConfigFromJson(&configJson)?;
    output.push_stdout_line(format!("imported={count}"));
    Ok(())
}

fn export_mcp_config(
    context: OperitApplicationContext,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    output.push_stdout_line(mcp_local_server(&context).exportConfigAsJson());
    Ok(())
}

fn remove_mcp_server(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let id = required_arg(args, 1, "operit2 mcp remove <id>")?;
    mcp_local_server(&context).removeMCPServer(id)?;
    output.push_stdout_line(format!("removed={id}"));
    Ok(())
}

fn set_mcp_enabled(
    context: OperitApplicationContext,
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
    if enabled {
        output.push_stdout_line(format!("enabled={id}"));
    } else {
        output.push_stdout_line(format!("disabled={id}"));
    }
    Ok(())
}

fn start_mcp_server(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let id = required_arg(args, 1, "operit2 mcp start <id>")?;
    let packageManager = AIToolHandler::getInstance(context).getOrCreatePackageManager();
    let mut guard = packageManager
        .lock()
        .expect("package manager mutex poisoned");
    output.push_stdout_line(guard.useMCPServer(id));
    Ok(())
}

fn print_mcp_tools(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let id = required_arg(args, 1, "operit2 mcp tools <id>")?;
    let tools = mcp_local_server(&context)
        .getCachedTools(id)
        .ok_or_else(|| format!("MCP tools not cached: {id}"))?;
    for tool in tools {
        output.push_stdout_line(format!(
            "{}\t{}\t{}",
            tool.name, tool.description, tool.inputSchema
        ));
    }
    Ok(())
}

fn print_mcp_config(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let id = required_arg(args, 1, "operit2 mcp config <id>")?;
    require_mcp_server(&context, id)?;
    output.push_stdout_line(mcp_local_server(&context).getPluginConfig(id));
    Ok(())
}

fn save_mcp_config(
    context: OperitApplicationContext,
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
    output.push_stdout_line(format!("configSaved={id}"));
    Ok(())
}

fn save_local_mcp_server(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let usage = "operit2 mcp local-set <id> [--disabled true|false] [--env KEY=VALUE] [--approve TOOL] -- <command> [args...]";
    let id = required_arg(args, 1, usage)?;
    let parsed = parse_local_set_args(&args[2..], usage)?;
    mcp_local_server(&context).addOrUpdateMCPServer(
        id.to_string(),
        parsed.command,
        parsed.args,
        parsed.env,
        parsed.disabled,
        parsed.autoApprove,
    )?;
    output.push_stdout_line(format!("localSaved={id}"));
    Ok(())
}

fn install_mcp_from_github(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let usage = "operit2 mcp install-github <id> <repo-url> <name> <description-or-@file> <author> <version> [config-or-@file]";
    let id = required_arg(args, 1, usage)?.to_string();
    let repoUrl = required_arg(args, 2, usage)?.to_string();
    let metadata = metadata_from_install_args(args, usage)?;
    let mcpConfig = optional_content_arg(args.get(7))?;
    match MCPRepository::getInstance(&context).installMCPServerWithObject(
        id.clone(),
        repoUrl,
        metadata,
        mcpConfig,
        |_| {},
    ) {
        operit_runtime::data::mcp::MCPRepository::InstallResult::Success { pluginPath } => {
            output.push_stdout_line(format!("installed={id}"));
            output.push_stdout_line(format!("path={pluginPath}"));
            Ok(())
        }
        operit_runtime::data::mcp::MCPRepository::InstallResult::Error { message } => Err(message),
    }
}

fn install_mcp_from_zip(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let usage = "operit2 mcp install-zip <id> <zip-path> <name> <description-or-@file> <author> <version> [config-or-@file]";
    let id = required_arg(args, 1, usage)?.to_string();
    let zipPath = required_arg(args, 2, usage)?.to_string();
    let metadata = metadata_from_install_args(args, usage)?;
    let mcpConfig = optional_content_arg(args.get(7))?;
    match MCPRepository::getInstance(&context).installMCPServerFromZip(
        id.clone(),
        zipPath,
        metadata,
        mcpConfig,
        |_| {},
    ) {
        operit_runtime::data::mcp::MCPRepository::InstallResult::Success { pluginPath } => {
            output.push_stdout_line(format!("installed={id}"));
            output.push_stdout_line(format!("path={pluginPath}"));
            Ok(())
        }
        operit_runtime::data::mcp::MCPRepository::InstallResult::Error { message } => Err(message),
    }
}

fn print_mcp_metadata(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let id = required_arg(args, 1, "operit2 mcp meta <id>")?;
    let metadata = mcp_local_server(&context)
        .getPluginMetadata(id)
        .ok_or_else(|| format!("MCP metadata not found: {id}"))?;
    output.push_stdout_line(format!("name={}", metadata.name));
    output.push_stdout_line(format!("description={}", metadata.description));
    output.push_stdout_line(format!("author={}", metadata.author));
    output.push_stdout_line(format!("version={}", metadata.version));
    Ok(())
}

fn save_mcp_metadata(
    context: OperitApplicationContext,
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
    mcp_local_server(&context).addOrUpdatePluginMetadata(
        id,
        PluginMetadata {
            name,
            description,
            author,
            version,
        },
    )?;
    output.push_stdout_line(format!("metadataSaved={id}"));
    Ok(())
}

fn generate_mcp_description(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let id = required_arg(args, 1, "operit2 mcp describe <id>")?;
    let metadata = mcp_local_server(&context)
        .getPluginMetadata(id)
        .ok_or_else(|| format!("MCP metadata not found: {id}"))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    let description = runtime.block_on(
        MCPRepository::getInstance(&context).generatePluginDescription(id, &metadata.name),
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
    output.push_stdout_line(description);
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

fn require_mcp_server(context: &OperitApplicationContext, id: &str) -> Result<(), String> {
    mcp_local_server(context)
        .getMCPServer(id)
        .map(|_| ())
        .ok_or_else(|| format!("MCP server not found: {id}"))
}

fn print_optional_metadata(server: &MCPLocalServer, id: &str, output: &mut CoreCommandOutput) {
    if let Some(metadata) = server.getPluginMetadata(id) {
        output.push_stdout_line(format!("name={}", metadata.name));
        output.push_stdout_line(format!("description={}", metadata.description));
        output.push_stdout_line(format!("author={}", metadata.author));
        output.push_stdout_line(format!("version={}", metadata.version));
    }
}

fn print_optional_status(server: &MCPLocalServer, id: &str, output: &mut CoreCommandOutput) {
    if let Some(status) = server.getServerStatus(id) {
        output.push_stdout_line(format!("lastStartTime={}", status.lastStartTime));
        output.push_stdout_line(format!("lastStopTime={}", status.lastStopTime));
        if let Some(errorMessage) = status.errorMessage {
            output.push_stdout_line(format!("errorMessage={errorMessage}"));
        }
        if let Some(tools) = status.cachedTools.as_ref() {
            output.push_stdout_line(format!("tools={}", tools.len()));
        }
    }
}

fn required_arg<'a>(args: &'a [String], index: usize, usage: &str) -> Result<&'a String, String> {
    args.get(index).ok_or_else(|| format!("usage: {usage}"))
}

fn mcp_local_server(context: &OperitApplicationContext) -> MCPLocalServer {
    MCPLocalServer::getInstance(context)
}

fn print_mcp_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 mcp dir");
    output.push_stdout_line("operit2 mcp list");
    output.push_stdout_line("operit2 mcp show <id>");
    output.push_stdout_line("operit2 mcp import <json-or-@file>");
    output.push_stdout_line("operit2 mcp export");
    output.push_stdout_line("operit2 mcp remove <id>");
    output.push_stdout_line("operit2 mcp enable <id>");
    output.push_stdout_line("operit2 mcp disable <id>");
    output.push_stdout_line("operit2 mcp start <id>");
    output.push_stdout_line("operit2 mcp tools <id>");
    output.push_stdout_line("operit2 mcp config <id>");
    output.push_stdout_line("operit2 mcp config-set <id> <json-or-@file>");
    output.push_stdout_line("operit2 mcp local-set <id> [--disabled true|false] [--env KEY=VALUE] [--approve TOOL] -- <command> [args...]");
    output.push_stdout_line("operit2 mcp install-github <id> <repo-url> <name> <description-or-@file> <author> <version> [config-or-@file]");
    output.push_stdout_line("operit2 mcp install-zip <id> <zip-path> <name> <description-or-@file> <author> <version> [config-or-@file]");
    output.push_stdout_line("operit2 mcp meta <id>");
    output.push_stdout_line(
        "operit2 mcp meta-set <id> <name> <description-or-@file> <author> <version>",
    );
    output.push_stdout_line("operit2 mcp describe <id>");
}
