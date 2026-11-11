#![allow(non_snake_case)]

mod commands;
mod output;

pub use output::CoreCommandOutput;

/// Creates an application from the provided context and runs a core command.
pub async fn run_core_command_with_context(
    context: operit_host_api::HostManager::HostManager,
    args: &[String],
) -> Result<CoreCommandOutput, String> {
    let mut application =
        operit_runtime::core::application::OperitApplication::OperitApplication::newWithContext(
            context,
        );
    application.onCreate()?;
    run_core_command(&mut application, args).await
}

/// Runs a core command against an already initialized application.
pub async fn run_core_command(
    application: &mut operit_runtime::core::application::OperitApplication::OperitApplication,
    args: &[String],
) -> Result<CoreCommandOutput, String> {
    let jsonMode = args.iter().any(|arg| arg == "--json");
    let mut commandArgs = args
        .iter()
        .filter(|arg| arg.as_str() != "--json")
        .cloned()
        .collect::<Vec<_>>();
    let directory = if commandArgs.first().map(String::as_str) == Some("--cwd") {
        let value = commandArgs.get(1).ok_or("--cwd requires a VFS directory")?;
        let value = application
            .hostManager
            .vfs()?
            .resolveResourcePath(None, value)?;
        commandArgs.drain(..2);
        Some(value)
    } else {
        None
    };
    let mut output = CoreCommandOutput::new();
    output.setJsonMode(jsonMode);
    if let Some(directory) = directory {
        use operit_tools::ToolExecutionManager::{
            ToolExecutionManager, ToolExposureMode, ToolRuntimeContext,
        };
        let mut context =
            ToolExecutionManager::currentToolRuntimeContext().unwrap_or(ToolRuntimeContext {
                callerChatId: None,
                callerParticipantId: None,
                workspacePath: None,
                workspaceFolders: Vec::new(),
                toolExposureMode: ToolExposureMode::CLI,
                extensionOwner: "core:cli".into(),
                executionData: serde_json::Map::new(),
            });
        context.workspacePath = Some(directory.clone());
        context.workspaceFolders = vec![directory];
        ToolExecutionManager::scopeToolRuntimeContext(
            context,
            commands::run_core_command(application, &commandArgs, &mut output),
        )
        .await?;
    } else {
        commands::run_core_command(application, &commandArgs, &mut output).await?;
    }
    output.finalizeJson()?;
    Ok(output)
}
