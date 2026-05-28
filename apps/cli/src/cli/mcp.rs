use super::core::CliCore;
use super::*;

pub(super) async fn run_mcp_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_mcp_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "dir" => {
            println!(
                "configDir={}",
                core.mcp_local_server()
                    .getConfigDirectory()
                    .await
                    .map_err(|error| error.to_string())?
            );
            println!(
                "configFile={}",
                core.mcp_local_server()
                    .getConfigFilePath()
                    .await
                    .map_err(|error| error.to_string())?
            );
        }
        "list" => {
            let servers = core
                .mcp_local_server()
                .getAllMCPServers()
                .await
                .map_err(|error| error.to_string())?;
            let metadata = core
                .mcp_local_server()
                .getAllPluginMetadata()
                .await
                .map_err(|error| error.to_string())?;
            let status = core
                .mcp_local_server()
                .getAllServerStatus()
                .await
                .map_err(|error| error.to_string())?;
            for (serverId, serverConfig) in servers {
                let meta = metadata.get(&serverId);
                let state = status.get(&serverId);
                println!(
                    "{}\tenabled={}\tcommand={}\targs={}\tname={}\ttoolsCached={}",
                    serverId,
                    core.mcp_local_server()
                        .isServerEnabled(&serverId)
                        .await
                        .map_err(|error| error.to_string())?,
                    serverConfig.command,
                    serverConfig.args.join(" "),
                    meta.map(|item| item.name.as_str()).unwrap_or(""),
                    state
                        .and_then(|item| item.cachedTools.as_ref())
                        .map(|tools| tools.len())
                        .unwrap_or(0)
                );
            }
            for (pluginId, meta) in metadata {
                if meta.r#type == "remote" {
                    println!(
                        "{}\tenabled={}\tremote={}\tname={}",
                        pluginId,
                        core.mcp_local_server()
                            .isServerEnabled(&pluginId)
                            .await
                            .map_err(|error| error.to_string())?,
                        meta.endpoint.unwrap_or_default(),
                        meta.name
                    );
                }
            }
        }
        "show" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 mcp show <name>".to_string())?;
            if let Some(serverConfig) = core
                .mcp_local_server()
                .getMCPServer(name)
                .await
                .map_err(|error| error.to_string())?
            {
                println!("id={name}");
                println!(
                    "enabled={}",
                    core.mcp_local_server()
                        .isServerEnabled(name)
                        .await
                        .map_err(|error| error.to_string())?
                );
                println!("command={}", serverConfig.command);
                println!("args={}", serverConfig.args.join(" "));
                println!(
                    "envKeys={}",
                    serverConfig
                        .env
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(",")
                );
                println!("autoApprove={}", serverConfig.autoApprove.join(","));
            }
            if let Some(metadata) = core
                .mcp_local_server()
                .getPluginMetadata(name)
                .await
                .map_err(|error| error.to_string())?
            {
                println!("metadataId={}", metadata.id);
                println!("name={}", metadata.name);
                println!("description={}", metadata.description);
                println!("type={}", metadata.r#type);
                println!("endpoint={}", metadata.endpoint.unwrap_or_default());
                println!(
                    "connectionType={}",
                    metadata.connectionType.unwrap_or_default()
                );
            }
            if let Some(status) = core
                .mcp_local_server()
                .getServerStatus(name)
                .await
                .map_err(|error| error.to_string())?
            {
                println!("lastStartTime={}", status.lastStartTime);
                println!("lastStopTime={}", status.lastStopTime);
                println!("errorMessage={}", status.errorMessage.unwrap_or_default());
                println!(
                    "cachedTools={}",
                    status
                        .cachedTools
                        .as_ref()
                        .map(|tools| tools.len())
                        .unwrap_or(0)
                );
            }
        }
        "import" => {
            let configArg = args
                .get(1)
                .ok_or_else(|| "usage: operit2 mcp import <json-or-@file>".to_string())?;
            let configJson = read_skill_content_arg(configArg)?;
            let count = core
                .mcp_local_server()
                .mergeConfigFromJson(&configJson)
                .await
                .map_err(|error| error.to_string())?;
            println!("imported={count}");
        }
        "enable" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 mcp enable <name>".to_string())?;
            core.mcp_local_server()
                .setServerEnabled(name, true)
                .await
                .map_err(|error| error.to_string())?;
            println!("enabled={name}");
        }
        "disable" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 mcp disable <name>".to_string())?;
            core.mcp_local_server()
                .setServerEnabled(name, false)
                .await
                .map_err(|error| error.to_string())?;
            println!("disabled={name}");
        }
        "start" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 mcp start <name>".to_string())?;
            println!(
                "{}",
                core.permissions_pack_tool_package_manager()
                    .useMCPServer(name)
                    .await
                    .map_err(|error| error.to_string())?
            );
        }
        "cached" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 mcp cached <name>".to_string())?;
            for tool in core
                .mcp_local_server()
                .getCachedTools(name)
                .await
                .map_err(|error| error.to_string())?
                .unwrap_or_default()
            {
                println!("{}\t{}\t{}", tool.name, tool.description, tool.inputSchema);
            }
        }
        "export" => {
            println!(
                "{}",
                core.mcp_local_server()
                    .exportConfigAsJson()
                    .await
                    .map_err(|error| error.to_string())?
            );
        }
        _ => print_mcp_usage(),
    }
    Ok(())
}
