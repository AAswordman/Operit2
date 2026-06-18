use std::collections::BTreeMap;

use crate::output::CoreCommandOutput;
use operit_runtime::api::chat::enhance::ConversationMarkupManager::ToolResult;
use operit_runtime::api::chat::enhance::ToolExecutionManager::{AITool, ToolParameter};
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::core::files::PathMapper::PathMapper;
use operit_runtime::core::files::VisualFileSystem::VisualFileSystem;
use operit_runtime::core::tools::AIToolHandler::AIToolHandler;
use operit_runtime::ui::features::chat::webview::workspace::WorkspaceUtils;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use serde::Deserialize;

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

fn default_workspace_path(
    _application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 workspace default-path <chat-id>".to_string())?;
    let path = PathMapper::workspacePath(chatId)?;
    output.push_stdout_line(path);
    Ok(())
}

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
    let workspacePath = WorkspaceUtils::createAndGetDefaultWorkspace(chatId, projectType)?;
    output.push_stdout_line(workspacePath);
    Ok(())
}

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
    application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .bindChatToWorkspace(chatId.clone(), workspacePath.clone());
    output.push_stdout_line(format!("workspace bound: {chatId}\t{workspacePath}"));
    Ok(())
}

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
    application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .bindChatToWorkspace(chatId.clone(), workspace.clone());
    output.push_stdout_line(format!("workspace bound: {chatId}\t{workspace}"));
    Ok(())
}

fn unbind_workspace(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 workspace unbind <chat-id>".to_string())?
        .clone();
    application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .unbindChatFromWorkspace(chatId.clone());
    output.push_stdout_line(format!("workspace unbound: {chatId}"));
    Ok(())
}

fn list_workspaces(
    application: &mut OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let mut workspaces = BTreeMap::<String, usize>::new();
    for chat in application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .chatHistoriesFlow()
        .value()
    {
        let Some(workspace) = chat.workspace else {
            continue;
        };
        let entry = workspaces.entry(workspace).or_insert(0);
        *entry += 1;
    }
    for (workspace, chatCount) in workspaces {
        output.push_stdout_line(format!("{workspace}\t{chatCount}"));
    }
    Ok(())
}

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
    for chat in application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .chatHistoriesFlow()
        .value()
        .into_iter()
        .filter(|chat| chat.workspace.as_deref() == Some(workspace.as_str()))
    {
        output.push_stdout_line(format!("{}\t{}", chat.id, chat.title));
    }
    Ok(())
}

fn list_workspace_commands(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 workspace commands <chat-id>".to_string())?;
    let workspacePath = workspace_path_for_chat(application, chatId)?;
    list_commands_at_path(&application.applicationContext, &workspacePath, output)
}

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
    list_commands_at_path(&application.applicationContext, &workspacePath, output)
}

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
    run_command_at_path(
        application.applicationContext.clone(),
        &workspacePath,
        commandId,
        output,
    )
}

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
    run_command_at_path(
        application.applicationContext.clone(),
        &workspacePath,
        commandId,
        output,
    )
}

fn workspace_path_for_chat(
    application: &mut OperitApplication,
    chatId: &str,
) -> Result<String, String> {
    let chat = application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .chatHistoriesFlow()
        .value()
        .into_iter()
        .find(|chat| chat.id == chatId)
        .ok_or_else(|| format!("chat not found: {chatId}"))?;
    chat.workspace
        .and_then(nonBlankString)
        .ok_or_else(|| format!("chat has no workspace: {chatId}"))
}

#[allow(non_snake_case)]
fn vfsForWorkspace(context: &OperitApplicationContext) -> Result<VisualFileSystem, String> {
    let runtimeStoreRoot = context
        .runtimeStorageHost
        .as_ref()
        .and_then(|host| host.rootDir())
        .ok_or_else(|| {
            "RuntimeStorageHost root is not configured for workspace commands".to_string()
        })?;
    let runtimeStorePaths = RuntimeStorePaths::new(runtimeStoreRoot.clone());
    Ok(VisualFileSystem::new(
        context
            .fileSystemHost
            .clone()
            .ok_or_else(|| "FileSystemHost is not registered for workspace commands".to_string())?,
        PathMapper::new(
            runtimeStoreRoot,
            context.appFilesRoot.clone(),
            runtimeStorePaths.workspace_dir(),
        ),
    ))
}

fn list_commands_at_path(
    context: &OperitApplicationContext,
    workspacePath: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let vfs = vfsForWorkspace(context)?;
    let config = WorkspaceConfigReader::readConfig(&vfs, workspacePath)?;
    for command in config.commands {
        output.push_stdout_line(format!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            command.id,
            command.label,
            command.kind(),
            command.workingDir,
            command.shell,
            command.usesDedicatedSession
        ));
    }
    Ok(())
}

fn run_command_at_path(
    context: OperitApplicationContext,
    workspacePath: &str,
    commandId: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let vfs = vfsForWorkspace(&context)?;
    let config = WorkspaceConfigReader::readConfig(&vfs, workspacePath)?;
    let command = config
        .commands
        .into_iter()
        .find(|command| command.id == commandId)
        .ok_or_else(|| format!("workspace command not found: {commandId}"))?;

    let toolName = command.tool.clone().and_then(nonBlankString);
    if let Some(toolName) = toolName {
        return execute_workspace_tool(context, &command, workspacePath, &toolName, output);
    }

    let commandText = command
        .command
        .clone()
        .and_then(nonBlankString)
        .ok_or_else(|| "No command/tool configured".to_string())?;
    execute_workspace_shell_command(&context, workspacePath, &command, &commandText, output)
}

fn execute_workspace_tool(
    context: OperitApplicationContext,
    command: &CommandConfig,
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

    let mut handler = AIToolHandler::getInstance(context);
    let result = handler.executeTool(AITool {
        name: toolName.to_string(),
        parameters,
    });
    print_tool_execution_result(&result, output)
}

fn execute_workspace_shell_command(
    context: &OperitApplicationContext,
    workspacePath: &str,
    command: &CommandConfig,
    commandText: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let terminalHost = context
        .terminalHost
        .clone()
        .ok_or_else(|| "TerminalHost is not registered for this runtime.".to_string())?;
    let terminalInfo = terminalHost
        .terminalInfo()
        .map_err(|error| format!("failed to read terminal info: {}", error.message))?;
    let vfs = vfsForWorkspace(context)?;
    let workingDir = workspace_command_working_dir(&vfs, workspacePath, &command.workingDir)?;
    let sessionName = workspace_command_session_name(workspacePath, command);
    let session = terminalHost
        .createOrGetSession(&sessionName, &terminalInfo.defaultType)
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
    if !commandOutput.output.is_empty() {
        output.push_stdout(commandOutput.output);
    }
    output.push_stdout_line(format!("exitCode={}", commandOutput.exitCode));
    if commandOutput.exitCode == 0 || commandOutput.timedOut {
        Ok(())
    } else {
        Err(format!(
            "workspace command failed with exitCode={}",
            commandOutput.exitCode
        ))
    }
}

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

fn is_path_like_tool_parameter(name: &str) -> bool {
    name.split(['_', '-'])
        .map(|part| part.to_ascii_lowercase())
        .any(|part| matches!(part.as_str(), "path" | "file" | "dir" | "directory"))
}

#[allow(non_snake_case)]
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
fn startsWithHostDrivePath(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn print_tool_execution_result(
    result: &ToolResult,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    output.push_stdout_line(format!("toolName={}", result.toolName));
    output.push_stdout_line(format!("success={}", result.success));
    let resultText = result.result.toString();
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

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn parse_default_workspace_args(
    args: &[String],
    usage: &str,
) -> Result<(String, Option<String>), String> {
    let chatId = args.get(0).cloned().ok_or_else(|| usage.to_string())?;
    let projectType = args.get(1).cloned().and_then(nonBlankString);
    Ok((chatId, projectType))
}

fn nonBlankString(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn print_workspace_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 workspace default-path <chat-id>");
    output.push_stdout_line("operit2 workspace create-default <chat-id> [project-type]");
    output.push_stdout_line("operit2 workspace bind-default <chat-id> [project-type]");
    output.push_stdout_line("operit2 workspace bind <chat-id> <workspace>");
    output.push_stdout_line("operit2 workspace unbind <chat-id>");
    output.push_stdout_line("operit2 workspace list");
    output.push_stdout_line("operit2 workspace chats <workspace>");
    output.push_stdout_line("operit2 workspace commands <chat-id>");
    output.push_stdout_line("operit2 workspace commands-path <workspace>");
    output.push_stdout_line("operit2 workspace run <chat-id> <command-id>");
    output.push_stdout_line("operit2 workspace run-path <workspace> <command-id>");
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
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
#[derive(Clone, Debug, Deserialize)]
struct ServerConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_server_port")]
    port: i32,
    #[serde(default)]
    autoStart: bool,
}

impl Default for ServerConfig {
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
#[derive(Clone, Debug, Deserialize)]
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
#[derive(Clone, Debug, Deserialize)]
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
#[derive(Clone, Debug, Deserialize)]
struct ExportConfig {
    #[serde(default = "default_export_enabled")]
    enabled: bool,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            enabled: default_export_enabled(),
        }
    }
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
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

fn default_project_type() -> String {
    "web".to_string()
}

fn default_server_port() -> i32 {
    8093
}

fn default_preview_type() -> String {
    "browser".to_string()
}

fn default_working_dir() -> String {
    ".".to_string()
}

fn default_command_shell() -> bool {
    true
}

fn default_export_enabled() -> bool {
    true
}

fn default_watch_enabled() -> bool {
    true
}

fn default_watch_max_depth() -> i32 {
    3
}

fn default_watch_max_changed_files() -> i32 {
    80
}

fn default_watch_exclude() -> Vec<String> {
    vec![
        ".git".to_string(),
        ".operit".to_string(),
        ".backup".to_string(),
        "backup".to_string(),
    ]
}
