use std::collections::BTreeMap;

use crate::output::CoreCommandOutput;
use operit_host_api::HostManager::HostManager;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::services::ChatServiceCore::ChatServiceCore;
use operit_runtime::ui::features::chat::webview::workspace::WorkspaceUtils;
use operit_tools::files::PathMapper::PathMapper;
use operit_tools::files::VisualFileSystem::VisualFileSystem;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{AITool, ToolParameter};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Runs a synchronous action against the local main chat runtime core.
fn with_main_chat_core<R>(
    application: &OperitApplication,
    action: impl FnOnce(&mut ChatServiceCore) -> R,
) -> Result<R, String> {
    let mut holder = application
        .chatRuntimeHolder
        .try_lock()
        .map_err(|_| "Chat runtime holder is busy".to_string())?;
    Ok(action(holder.getCore(ChatRuntimeSlot::MAIN)))
}

/// Runs workspace path, binding, shortcut, and workspace tool commands.
pub fn run_workspace_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_workspace_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "default-path" => default_workspace_path(application, &args[1..], output),
        "create-default" => create_default_workspace(application, &args[1..], output),
        "bind-default" => bind_default_workspace(application, &args[1..], output),
        "bind" => bind_workspace(application, &args[1..], output),
        "unbind" => unbind_workspace(application, &args[1..], output),
        "list" => list_workspaces(application, output),
        "chats" => list_workspace_chats(application, &args[1..], output),
        "commands" => list_workspace_commands(application, &args[1..], output),
        "commands-path" => list_workspace_commands_path(application, &args[1..], output),
        "run" => run_workspace_shortcut(application, &args[1..], output),
        "run-path" => run_workspace_shortcut_path(application, &args[1..], output),
        _ => {
            print_workspace_usage(output);
            Ok(())
        }
    }
}

/// Prints the default workspace path for one chat id.
fn default_workspace_path(
    _application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 workspace default-path <chat-id>".to_string())?;
    let path = PathMapper::workspacePath(chatId)?;
    output.push_stdout_line(format!("Default workspace path: {path}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "workspacePath": path
    }));
    Ok(())
}

/// Creates the default workspace directory for one chat id.
fn create_default_workspace(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let (chatId, projectType) = parse_default_workspace_args(
        args,
        "operit2 workspace create-default <chat-id> [project-type]",
    )?;
    let _ = application;
    let workspacePath =
        WorkspaceUtils::createAndGetDefaultWorkspace(chatId.clone(), projectType.clone())?;
    output.push_stdout_line(format!("Created default workspace for {chatId}"));
    output.push_stdout_line(format!("Workspace: {workspacePath}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "projectType": projectType,
        "workspacePath": workspacePath,
        "created": true
    }));
    Ok(())
}

/// Creates and binds the default workspace for one chat id.
fn bind_default_workspace(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let (chatId, projectType) = parse_default_workspace_args(
        args,
        "operit2 workspace bind-default <chat-id> [project-type]",
    )?;
    let workspacePath = WorkspaceUtils::createAndGetDefaultWorkspace(chatId.clone(), projectType)?;
    with_main_chat_core(application, |core| {
        core.bindChatToWorkspace(chatId.clone(), workspacePath.clone())
    })??;
    output.push_stdout_line(format!("Bound workspace for {chatId}"));
    output.push_stdout_line(format!("Workspace: {workspacePath}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "workspacePath": workspacePath,
        "bound": true
    }));
    Ok(())
}

/// Binds one chat id to an explicit workspace path.
fn bind_workspace(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 workspace bind <chat-id> <workspace>".to_string())?
        .clone();
    let workspace = args
        .get(1)
        .cloned()
        .and_then(nonBlankString)
        .ok_or_else(|| "usage: operit2 workspace bind <chat-id> <workspace>".to_string())?;
    let workspace = PathMapper::normalizeWorkspaceBindingPath(&workspace)?;
    with_main_chat_core(application, |core| {
        core.bindChatToWorkspace(chatId.clone(), workspace.clone())
    })??;
    output.push_stdout_line(format!("Bound workspace for {chatId}"));
    output.push_stdout_line(format!("Workspace: {workspace}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "workspacePath": workspace,
        "bound": true
    }));
    Ok(())
}

/// Removes the workspace binding from one chat id.
fn unbind_workspace(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 workspace unbind <chat-id>".to_string())?
        .clone();
    with_main_chat_core(application, |core| {
        core.unbindChatFromWorkspace(chatId.clone())
    })?;
    output.push_stdout_line(format!("Unbound workspace for {chatId}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "unbound": true
    }));
    Ok(())
}

/// Lists all workspace bindings grouped by workspace path.
fn list_workspaces(
    application: &mut OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let mut workspaces = BTreeMap::<String, usize>::new();
    let chats = with_main_chat_core(application, |core| core.chatHistoriesFlow().value())?;
    for chat in chats {
        let Some(workspace) = chat.workspace else {
            continue;
        };
        let entry = workspaces.entry(workspace).or_insert(0);
        *entry += 1;
    }
    output.push_stdout_line(format!("Workspaces: {}", workspaces.len()));
    for (workspace, chatCount) in &workspaces {
        output.push_stdout_line(format!("- {workspace} | chats: {chatCount}"));
    }
    output.setJsonStdout(json!({ "workspaces": workspaces }));
    Ok(())
}

/// Lists chats bound to one workspace path.
fn list_workspace_chats(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let workspace = args
        .get(0)
        .cloned()
        .and_then(nonBlankString)
        .ok_or_else(|| "usage: operit2 workspace chats <workspace>".to_string())?;
    let chats = with_main_chat_core(application, |core| core.chatHistoriesFlow().value())?
        .into_iter()
        .filter(|chat| chat.workspace.as_deref() == Some(workspace.as_str()))
        .collect::<Vec<_>>();
    output.push_stdout_line(format!("Chats in workspace: {}", chats.len()));
    output.push_stdout_line(format!("Workspace: {workspace}"));
    for chat in &chats {
        output.push_stdout_line(format!("- {} | {}", chat.id, chat.title));
    }
    output.setJsonStdout(json!({
        "workspacePath": workspace,
        "chats": chats
    }));
    Ok(())
}

/// Lists workspace commands configured for a chat workspace.
fn list_workspace_commands(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 workspace commands <chat-id>".to_string())?;
    let workspacePath = workspace_path_for_chat(application, chatId)?;
    list_commands_at_path(
        &application.hostManager,
        Some(chatId),
        &workspacePath,
        output,
    )
}

/// Lists workspace commands configured at an explicit workspace path.
fn list_workspace_commands_path(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let workspacePath = args
        .get(0)
        .cloned()
        .and_then(nonBlankString)
        .ok_or_else(|| "usage: operit2 workspace commands-path <workspace>".to_string())?;
    let workspacePath = PathMapper::normalizeWorkspaceBindingPath(&workspacePath)?;
    list_commands_at_path(&application.hostManager, None, &workspacePath, output)
}

/// Runs a configured workspace shortcut for a chat workspace.
fn run_workspace_shortcut(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 workspace run <chat-id> <command-id>".to_string())?;
    let commandId = args
        .get(1)
        .ok_or_else(|| "usage: operit2 workspace run <chat-id> <command-id>".to_string())?;
    let workspacePath = workspace_path_for_chat(application, chatId)?;
    run_command_at_path(application, Some(chatId), &workspacePath, commandId, output)
}

/// Runs a configured workspace shortcut at an explicit workspace path.
fn run_workspace_shortcut_path(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let workspacePath = args
        .get(0)
        .cloned()
        .and_then(nonBlankString)
        .ok_or_else(|| "usage: operit2 workspace run-path <workspace> <command-id>".to_string())?;
    let workspacePath = PathMapper::normalizeWorkspaceBindingPath(&workspacePath)?;
    let commandId = args
        .get(1)
        .ok_or_else(|| "usage: operit2 workspace run-path <workspace> <command-id>".to_string())?;
    run_command_at_path(application, None, &workspacePath, commandId, output)
}

/// Resolves the workspace path currently bound to one chat.
fn workspace_path_for_chat(
    application: &mut OperitApplication,
    chatId: &str,
) -> Result<String, String> {
    let chat = with_main_chat_core(application, |core| {
        core.chatHistoriesFlow()
            .value()
            .into_iter()
            .find(|chat| chat.id == chatId)
            .ok_or_else(|| format!("chat not found: {chatId}"))
    })??;
    chat.workspace
        .and_then(nonBlankString)
        .ok_or_else(|| format!("chat has no workspace: {chatId}"))
}

#[allow(non_snake_case)]
/// Creates a visual filesystem rooted in the configured runtime and workspace storage.
fn vfsForWorkspace(context: &HostManager) -> Result<VisualFileSystem, String> {
    let runtimeStorageHost = context
        .runtimeStorageHost
        .as_ref()
        .ok_or_else(|| "RuntimeStorageHost is not configured for workspace commands".to_string())?;
    let runtimeStoreRoot = runtimeStorageHost.runtimeRootDir().ok_or_else(|| {
        "RuntimeStorageHost runtime root is not configured for workspace commands".to_string()
    })?;
    let workspaceCollectionRoot = runtimeStorageHost.workspaceRootDir().ok_or_else(|| {
        "RuntimeStorageHost workspace root is not configured for workspace commands".to_string()
    })?;
    Ok(VisualFileSystem::new(
        context
            .fileSystemHost
            .clone()
            .ok_or_else(|| "FileSystemHost is not registered for workspace commands".to_string())?,
        PathMapper::new(runtimeStoreRoot, workspaceCollectionRoot),
    ))
}

/// Lists commands from one workspace configuration file.
fn list_commands_at_path(
    context: &HostManager,
    chatId: Option<&str>,
    workspacePath: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let vfs = vfsForWorkspace(context)?;
    let config = WorkspaceConfigReader::readConfig(&vfs, workspacePath)?;
    output.push_stdout_line(format!("Workspace commands: {}", config.commands.len()));
    output.push_stdout_line(format!("Workspace: {workspacePath}"));
    for command in &config.commands {
        output.push_stdout_line(format!(
            "- {} | {} | {} | working dir: {} | shell: {} | dedicated: {}",
            command.id,
            command.label,
            command.kind(),
            command.workingDir,
            command.shell,
            command.usesDedicatedSession
        ));
    }
    output.setJsonStdout(json!({
        "chatId": chatId,
        "workspacePath": workspacePath,
        "commands": config.commands
    }));
    Ok(())
}

/// Runs one command from a workspace configuration file.
fn run_command_at_path(
    application: &OperitApplication,
    chatId: Option<&str>,
    workspacePath: &str,
    commandId: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let context = application.hostManager.clone();
    let vfs = vfsForWorkspace(&context)?;
    let config = WorkspaceConfigReader::readConfig(&vfs, workspacePath)?;
    let command = config
        .commands
        .into_iter()
        .find(|command| command.id == commandId)
        .ok_or_else(|| format!("workspace command not found: {commandId}"))?;

    let toolName = command.tool.clone().and_then(nonBlankString);
    if let Some(toolName) = toolName {
        return execute_workspace_tool(
            application.toolHandler.clone(),
            &command,
            chatId,
            workspacePath,
            &toolName,
            output,
        );
    }

    let commandText = command
        .command
        .clone()
        .and_then(nonBlankString)
        .ok_or_else(|| "No command/tool configured".to_string())?;
    execute_workspace_shell_command(
        &context,
        chatId,
        workspacePath,
        &command,
        &commandText,
        output,
    )
}

/// Executes one workspace command via the tool runtime.
fn execute_workspace_tool(
    mut handler: AIToolHandler,
    command: &CommandConfig,
    chatId: Option<&str>,
    workspacePath: &str,
    toolName: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let mut parameters = Vec::new();
    for (name, value) in &command.toolParameters {
        parameters.push(ToolParameter {
            name: name.clone(),
            value: resolve_workspace_tool_parameter_value(name, value, workspacePath)?,
        });
    }

    let result = handler.executeTool(AITool {
        name: toolName.to_string(),
        parameters,
    });
    print_tool_execution_result(&result, command, chatId, workspacePath, output)
}

/// Executes one workspace command via the terminal host.
fn execute_workspace_shell_command(
    context: &HostManager,
    chatId: Option<&str>,
    workspacePath: &str,
    command: &CommandConfig,
    commandText: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let terminalHost = context
        .terminalHost
        .clone()
        .ok_or_else(|| "TerminalHost is not registered for this runtime.".to_string())?;
    let vfs = vfsForWorkspace(context)?;
    let workingDir = workspace_command_working_dir(&vfs, workspacePath, &command.workingDir)?;
    let sessionName = workspace_command_session_name(workspacePath, command);
    let session = terminalHost
        .createOrGetSession(&sessionName)
        .map_err(|error| {
            format!(
                "failed to create workspace terminal session: {}",
                error.message
            )
        })?;
    let cdCommand = format!("cd {}", shell_quote(&workingDir));
    terminalHost
        .executeInSession(&session.sessionId, &cdCommand, 120000)
        .map_err(|error| format!("failed to enter workspace directory: {}", error.message))?;
    let commandOutput = terminalHost
        .executeInSession(&session.sessionId, commandText, 1800000)
        .map_err(|error| format!("failed to execute workspace command: {}", error.message))?;
    let commandOutputText = commandOutput.output.clone();
    if !commandOutputText.is_empty() {
        output.push_stdout(commandOutputText.clone());
    }
    output.push_stdout_line(format!("Exit code: {}", commandOutput.exitCode));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "workspacePath": workspacePath,
        "commandId": &command.id,
        "label": &command.label,
        "kind": command.kind(),
        "command": commandText,
        "workingDir": &command.workingDir,
        "resolvedWorkingDir": &workingDir,
        "sessionName": &sessionName,
        "sessionId": &commandOutput.sessionId,
        "output": &commandOutputText,
        "exitCode": commandOutput.exitCode,
        "platform": &commandOutput.platform,
        "terminal": &commandOutput.terminal,
        "terminalType": &commandOutput.terminalType,
        "timedOut": commandOutput.timedOut
    }));
    if commandOutput.exitCode == 0 || commandOutput.timedOut {
        Ok(())
    } else {
        Err(format!(
            "workspace command failed with exitCode={}",
            commandOutput.exitCode
        ))
    }
}

/// Resolves one workspace tool parameter value.
fn resolve_workspace_tool_parameter_value(
    name: &str,
    rawValue: &str,
    workspacePath: &str,
) -> Result<String, String> {
    let expanded = rawValue
        .replace("$WORKSPACE", workspacePath)
        .replace("${WORKSPACE}", workspacePath);

    if !is_path_like_tool_parameter(name) {
        return Ok(expanded);
    }

    let trimmed = expanded.trim();
    if trimmed.is_empty() || hasUriScheme(trimmed) {
        return Ok(expanded);
    }

    if startsWithHostDrivePath(trimmed) {
        return Err(format!(
            "workspace tool parameter `{name}` must use a VFS path; use /mnt/windows/<drive>/... for Windows host paths"
        ));
    }

    if PathMapper::normalizeVfsPath(trimmed).is_ok() {
        return Ok(trimmed.to_string());
    }

    PathMapper::joinVfsPath(workspacePath, trimmed)
}

/// Returns whether a tool parameter name represents a filesystem path.
fn is_path_like_tool_parameter(name: &str) -> bool {
    name.split(['_', '-'])
        .map(|part| part.to_ascii_lowercase())
        .any(|part| matches!(part.as_str(), "path" | "file" | "dir" | "directory"))
}

#[allow(non_snake_case)]
/// Returns whether a text value begins with a URI scheme marker.
fn hasUriScheme(value: &str) -> bool {
    let Some(colonIndex) = value.find(':') else {
        return false;
    };
    if colonIndex == 0 {
        return false;
    }
    let scheme = &value[..colonIndex];
    if !scheme
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.'))
    {
        return false;
    }
    let bytes = value.as_bytes();
    bytes.get(colonIndex + 1) == Some(&b'/') && bytes.get(colonIndex + 2) == Some(&b'/')
}

#[allow(non_snake_case)]
/// Returns whether a text value begins with a Windows drive prefix.
fn startsWithHostDrivePath(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// Prints one workspace tool execution result.
fn print_tool_execution_result(
    result: &ToolResult,
    command: &CommandConfig,
    chatId: Option<&str>,
    workspacePath: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    output.push_stdout_line(format!("Tool: {}", result.toolName));
    output.push_stdout_line(format!("Success: {}", result.success));
    let resultText = result.result.toString();
    output.setJsonStdout(json!({
        "chatId": chatId,
        "workspacePath": workspacePath,
        "commandId": &command.id,
        "label": &command.label,
        "kind": command.kind(),
        "toolName": &result.toolName,
        "success": result.success,
        "result": &result.result,
        "resultText": &resultText,
        "error": &result.error
    }));
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

/// Resolves the physical working directory for a workspace command.
fn workspace_command_working_dir(
    vfs: &VisualFileSystem,
    workspacePath: &str,
    workingDir: &str,
) -> Result<String, String> {
    let trimmed = workingDir.trim();
    let workingDirPath = if trimmed.is_empty() || trimmed == "." {
        workspacePath.to_string()
    } else if startsWithHostDrivePath(trimmed) {
        return Err(
            "workspace command workingDir must use a VFS path or a path relative to the workspace"
                .to_string(),
        );
    } else if PathMapper::normalizeVfsPath(trimmed).is_ok() {
        trimmed.to_string()
    } else {
        PathMapper::joinVfsPath(workspacePath, trimmed)?
    };
    Ok(vfs.resolvePath(&workingDirPath)?.physicalPath)
}

/// Builds the terminal session name for a workspace command.
fn workspace_command_session_name(workspacePath: &str, command: &CommandConfig) -> String {
    if let Some(sessionTitle) = command.sessionTitle.clone().and_then(nonBlankString) {
        return sessionTitle;
    }
    let name = workspacePath
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| workspacePath.to_string());
    if command.usesDedicatedSession {
        format!("Workspace: {name}: {}", command.id)
    } else {
        format!("Workspace: {name}")
    }
}

/// Quotes a path for the terminal shell used by workspace commands.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Parses arguments shared by default workspace commands.
fn parse_default_workspace_args(
    args: &[String],
    usage: &str,
) -> Result<(String, Option<String>), String> {
    let chatId = args.get(0).cloned().ok_or_else(|| usage.to_string())?;
    let projectType = args.get(1).cloned().and_then(nonBlankString);
    Ok((chatId, projectType))
}

/// Converts non-empty text to an owned string.
fn nonBlankString(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Prints workspace command usage.
fn print_workspace_usage(output: &mut CoreCommandOutput) {
    let lines = [
        "operit2 workspace default-path <chat-id>",
        "operit2 workspace create-default <chat-id> [project-type]",
        "operit2 workspace bind-default <chat-id> [project-type]",
        "operit2 workspace bind <chat-id> <workspace>",
        "operit2 workspace unbind <chat-id>",
        "operit2 workspace list",
        "operit2 workspace chats <workspace>",
        "operit2 workspace commands <chat-id>",
        "operit2 workspace commands-path <workspace>",
        "operit2 workspace run <chat-id> <command-id>",
        "operit2 workspace run-path <workspace> <command-id>",
    ];
    for line in lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({ "usage": lines }));
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct WorkspaceConfig {
    #[serde(default = "default_project_type")]
    projectType: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    server: ServerConfig,
    #[serde(default)]
    preview: PreviewConfig,
    #[serde(default)]
    commands: Vec<CommandConfig>,
    #[serde(default)]
    export: ExportConfig,
    #[serde(default)]
    watch: WatchConfig,
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct ServerConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_server_port")]
    port: i32,
    #[serde(default)]
    autoStart: bool,
}

impl Default for ServerConfig {
    /// Builds the default server config for workspace files.
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_server_port(),
            autoStart: false,
        }
    }
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct PreviewConfig {
    #[serde(default = "default_preview_type")]
    r#type: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    showPreviewButton: bool,
    #[serde(default)]
    previewButtonLabel: String,
}

impl Default for PreviewConfig {
    /// Builds the default preview config for workspace files.
    fn default() -> Self {
        Self {
            r#type: default_preview_type(),
            url: String::new(),
            showPreviewButton: false,
            previewButtonLabel: String::new(),
        }
    }
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct CommandConfig {
    id: String,
    label: String,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    tool: Option<String>,
    #[serde(default)]
    toolParameters: BTreeMap<String, String>,
    #[serde(default = "default_working_dir")]
    workingDir: String,
    #[serde(default = "default_command_shell")]
    shell: bool,
    #[serde(default)]
    usesDedicatedSession: bool,
    #[serde(default)]
    sessionTitle: Option<String>,
}

impl CommandConfig {
    /// Returns whether this workspace entry runs a terminal command or a tool.
    fn kind(&self) -> &'static str {
        if self.tool.clone().and_then(nonBlankString).is_some() {
            "tool"
        } else {
            "command"
        }
    }
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct ExportConfig {
    #[serde(default = "default_export_enabled")]
    enabled: bool,
}

impl Default for ExportConfig {
    /// Builds the default export config for workspace files.
    fn default() -> Self {
        Self {
            enabled: default_export_enabled(),
        }
    }
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct WatchConfig {
    #[serde(default = "default_watch_enabled")]
    enabled: bool,
    #[serde(default = "default_watch_max_depth")]
    maxDepth: i32,
    #[serde(default = "default_watch_max_changed_files")]
    maxChangedFiles: i32,
    #[serde(default = "default_watch_exclude")]
    exclude: Vec<String>,
}

impl Default for WatchConfig {
    /// Builds the default watch config for workspace files.
    fn default() -> Self {
        Self {
            enabled: default_watch_enabled(),
            maxDepth: default_watch_max_depth(),
            maxChangedFiles: default_watch_max_changed_files(),
            exclude: default_watch_exclude(),
        }
    }
}

struct WorkspaceConfigReader;

impl WorkspaceConfigReader {
    /// Reads and parses a workspace config file.
    #[allow(non_snake_case)]
    fn readConfig(vfs: &VisualFileSystem, workspacePath: &str) -> Result<WorkspaceConfig, String> {
        let configFile = PathMapper::joinVfsPath(workspacePath, ".operit/config.json")?;
        let content = vfs
            .readFile(&configFile)
            .map_err(|error| format!("failed to read {configFile}: {error}"))?;
        serde_json::from_str::<WorkspaceConfig>(&content)
            .map_err(|error| format!("failed to parse {configFile}: {error}"))
    }
}

/// Returns the default workspace project type.
fn default_project_type() -> String {
    "web".to_string()
}

/// Returns the default workspace server port.
fn default_server_port() -> i32 {
    8093
}

/// Returns the default workspace preview type.
fn default_preview_type() -> String {
    "browser".to_string()
}

/// Returns the default command working directory.
fn default_working_dir() -> String {
    ".".to_string()
}

/// Returns the default command shell flag.
fn default_command_shell() -> bool {
    true
}

/// Returns the default export enabled flag.
fn default_export_enabled() -> bool {
    true
}

/// Returns the default watch enabled flag.
fn default_watch_enabled() -> bool {
    true
}

/// Returns the default watch traversal depth.
fn default_watch_max_depth() -> i32 {
    3
}

/// Returns the default watch changed-file limit.
fn default_watch_max_changed_files() -> i32 {
    80
}

/// Returns default workspace watch exclusion entries.
fn default_watch_exclude() -> Vec<String> {
    vec![
        ".git".to_string(),
        ".operit".to_string(),
        ".backup".to_string(),
        "backup".to_string(),
    ]
}
