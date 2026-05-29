use crate::commands::util::read_content_arg;
use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::core::tools::AIToolHandler::AIToolHandler;
use operit_runtime::data::mcp::MCPLocalServer::MCPLocalServer;

pub fn run_mcp_command(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_mcp_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "dir" => {
            let server = mcp_local_server(&context);
            output.push_stdout_line(format!("configDir={}", server.getConfigDirectory()));
            output.push_stdout_line(format!("configFile={}", server.getConfigFilePath()));
            Ok(())
        }
        "list" => list_mcp_servers(context, output),
        "show" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 mcp show <name>".to_string())?;
            show_mcp_server(context, name, output)
        }
        "import" => {
            let configArg = args
                .get(1)
                .ok_or_else(|| "usage: operit2 mcp import <json-or-@file>".to_string())?;
            let configJson = read_content_arg(configArg)?;
            let count = mcp_local_server(&context).mergeConfigFromJson(&configJson)?;
            output.push_stdout_line(format!("imported={count}"));
            Ok(())
        }
        "enable" => set_mcp_enabled(context, args.get(1), true, output),
        "disable" => set_mcp_enabled(context, args.get(1), false, output),
        "start" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 mcp start <name>".to_string())?;
            let packageManager = AIToolHandler::getInstance(context).getOrCreatePackageManager();
            let mut guard = packageManager
                .lock()
                .expect("package manager mutex poisoned");
            output.push_stdout_line(guard.useMCPServer(name));
            Ok(())
        }
        "cached" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 mcp cached <name>".to_string())?;
            if let Some(tools) = mcp_local_server(&context).getCachedTools(name) {
                for tool in tools {
                    output.push_stdout_line(format!(
                        "{}\t{}\t{}",
                        tool.name, tool.description, tool.inputSchema
                    ));
                }
            }
            Ok(())
        }
        "export" => {
            output.push_stdout_line(mcp_local_server(&context).exportConfigAsJson());
            Ok(())
        }
        _ => {
            print_mcp_usage(output);
            Ok(())
        }
    }
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
        let metaName = match metadata.get(&serverId) {
            Some(item) => item.name.as_str(),
            None => "",
        };
        let cachedTools = match status
            .get(&serverId)
            .and_then(|item| item.cachedTools.as_ref())
        {
            Some(tools) => tools.len(),
            None => 0,
        };
        output.push_stdout_line(format!(
            "{}\tenabled={}\tcommand={}\targs={}\tname={}\ttoolsCached={}",
            serverId,
            server.isServerEnabled(&serverId),
            serverConfig.command,
            serverConfig.args.join(" "),
            metaName,
            cachedTools
        ));
    }
    for (pluginId, meta) in metadata {
        if meta.r#type == "remote" {
            let endpoint = match meta.endpoint {
                Some(value) => value,
                None => String::new(),
            };
            output.push_stdout_line(format!(
                "{}\tenabled={}\tremote={}\tname={}",
                pluginId,
                server.isServerEnabled(&pluginId),
                endpoint,
                meta.name
            ));
        }
    }
    Ok(())
}

fn show_mcp_server(
    context: OperitApplicationContext,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let server = mcp_local_server(&context);
    if let Some(serverConfig) = server.getMCPServer(name) {
        output.push_stdout_line(format!("id={name}"));
        output.push_stdout_line(format!("enabled={}", server.isServerEnabled(name)));
        output.push_stdout_line(format!("command={}", serverConfig.command));
        output.push_stdout_line(format!("args={}", serverConfig.args.join(" ")));
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
    }
    if let Some(metadata) = server.getPluginMetadata(name) {
        let endpoint = match metadata.endpoint {
            Some(value) => value,
            None => String::new(),
        };
        let connectionType = match metadata.connectionType {
            Some(value) => value,
            None => String::new(),
        };
        output.push_stdout_line(format!("metadataId={}", metadata.id));
        output.push_stdout_line(format!("name={}", metadata.name));
        output.push_stdout_line(format!("description={}", metadata.description));
        output.push_stdout_line(format!("type={}", metadata.r#type));
        output.push_stdout_line(format!("endpoint={endpoint}"));
        output.push_stdout_line(format!("connectionType={connectionType}"));
    }
    if let Some(status) = server.getServerStatus(name) {
        let errorMessage = match status.errorMessage {
            Some(value) => value,
            None => String::new(),
        };
        let cachedTools = match status.cachedTools.as_ref() {
            Some(tools) => tools.len(),
            None => 0,
        };
        output.push_stdout_line(format!("lastStartTime={}", status.lastStartTime));
        output.push_stdout_line(format!("lastStopTime={}", status.lastStopTime));
        output.push_stdout_line(format!("errorMessage={errorMessage}"));
        output.push_stdout_line(format!("cachedTools={cachedTools}"));
    }
    Ok(())
}

fn set_mcp_enabled(
    context: OperitApplicationContext,
    name: Option<&String>,
    enabled: bool,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let name = name.ok_or_else(|| {
        if enabled {
            "usage: operit2 mcp enable <name>".to_string()
        } else {
            "usage: operit2 mcp disable <name>".to_string()
        }
    })?;
    mcp_local_server(&context).setServerEnabled(name, enabled)?;
    if enabled {
        output.push_stdout_line(format!("enabled={name}"));
    } else {
        output.push_stdout_line(format!("disabled={name}"));
    }
    Ok(())
}

fn mcp_local_server(context: &OperitApplicationContext) -> MCPLocalServer {
    MCPLocalServer::getInstance(context)
}

fn print_mcp_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 mcp dir");
    output.push_stdout_line("operit2 mcp list");
    output.push_stdout_line("operit2 mcp show <name>");
    output.push_stdout_line("operit2 mcp import <json-or-@file>");
    output.push_stdout_line("operit2 mcp enable <name>");
    output.push_stdout_line("operit2 mcp disable <name>");
    output.push_stdout_line("operit2 mcp start <name>");
    output.push_stdout_line("operit2 mcp cached <name>");
    output.push_stdout_line("operit2 mcp export");
}
