use std::collections::BTreeMap;

use crate::output::CoreCommandOutput;
use operit_host_api::HostManager::HostManager;
use operit_host_api::{ManagedRuntimeProgram, RuntimePathAction, RuntimeProcessRequest};
use operit_model::Workspace::{Workspace, WorkspaceFolder};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::services::ChatServiceCore::ChatServiceCore;
use operit_runtime::ui::features::chat::webview::workspace::WorkspaceUtils;
use operit_store::repository::ChatHistoryManager::{
    ChatHistoryManager, ChatHistoryManagerError, ChatHistoryManagerResult,
};
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::tools::ToolResultDataClasses::ToolResultData;
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{AITool, ToolParameter};
use operit_util::FileUtils::FileUtils;
use operit_vfs::VfsPath;
use operit_vfs::VisualFileSystem::VisualFileSystem;
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

/// Runs workspace identity, folder mounting, binding and execution commands.
pub async fn run_workspace_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() || matches!(args, [flag] if flag == "help" || flag == "--help") {
        print_workspace_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "create" => create_workspace(application, &args[1..], output).await,
        "show" => show_workspace(application, &args[1..], output),
        "mount" => mount_workspace(application, &args[1..], output).await,
        "unmount" => unmount_workspace(application, &args[1..], output),
        "primary" => set_primary_folder(application, &args[1..], output),
        "delete" => delete_workspace(application, &args[1..], output),
        "bind" => bind_workspace(application, &args[1..], output),
        "unbind" => unbind_workspace(application, &args[1..], output),
        "list" if args.len() == 1 => list_workspaces(application, output),
        "summary" if args.len() == 1 => {
            let summary = application.workspaceService().workspaceManagementSummary()?;
            output.push_stdout_line(serde_json::to_string_pretty(&summary).map_err(|error| error.to_string())?);
            output.setJsonStdout(json!(summary));
            Ok(())
        }
        "delete-unbound" => {
            let names = match &args[1..] {
                [flag, names @ ..] if flag == "--yes" && !names.is_empty() => names.to_vec(),
                _ => return Err("usage: operit2 workspace delete-unbound --yes <workspace-name> [names...]".into()),
            };
            let deleted = application.workspaceService().deleteUnboundWorkspaces(names.clone())?;
            output.push_stdout_line(format!("Deleted {deleted} unbound workspaces"));
            output.setJsonStdout(json!({"deleted": deleted, "workspaceNames": names}));
            Ok(())
        }
        "chats" => list_workspace_chats(application, &args[1..], output),
        "commands" => list_workspace_commands(application, &args[1..], output),
        "run" => run_workspace_shortcut(application, &args[1..], output).await,
        "exec" => execute_workspace_program(application, &args[1..], output),
        "resolve-path" => resolve_workspace_path(application, &args[1..], output),
        "open-path" => open_workspace_path(application, &args[1..], output),
        _ => Err(format!("Unknown workspace command or invalid arguments: {}. Run operit2 cli workspace for usage.", args[0])),
    }
}

/// Dispatches an explicit external path intent after resolving its VFS locator.
fn open_workspace_path(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let (path, action) = match args {
        [path, action] if action == "reveal" => (path, RuntimePathAction::Reveal),
        [path, action] if action == "terminal" => (path, RuntimePathAction::Terminal),
        [path, action, program, arguments @ ..] if action == "program" && !program.is_empty() =>
            (path, RuntimePathAction::Application { program: program.clone(), args: arguments.to_vec() }),
        _ => return Err("usage: operit2 workspace open-path <vfs-path> <reveal|terminal|program <name> [args...]>".to_string()),
    };
    let vfs = application.hostManager.vfs()?;
    let file = vfs.file(path)?;
    let host = application
        .hostManager
        .managedRuntimeHost
        .as_ref()
        .ok_or("ManagedRuntimeHost is not registered for this runtime")?;
    host.openFilePath(&file, action)
        .map_err(|error| error.message)?;
    output.setJsonStdout(json!({ "path": path, "opened": true }));
    Ok(())
}

/// Resolves a public or native locator through the owning runtime's VFS mapper.
fn resolve_workspace_path(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let path = match args {
        [path] => VfsPath::canonicalizeVfsPath(path)?,
        [flag, path] if flag == "--native" => VfsPath::nativePathToVfs(path)?,
        [flag, base, path] if flag == "--from" => {
            VfsPath::resolveWorkspaceResourcePath(base, path)?
        }
        _ => {
            return Err(
                "usage: operit2 workspace resolve-path [--native | --from <workspace>] <path>"
                    .to_string(),
            )
        }
    };
    let vfs = application.hostManager.vfs()?;
    vfs.validatePath(&path, "path")?;
    output.push_stdout_line(&path);
    output.setJsonStdout(json!({ "vfsPath": path }));
    Ok(())
}

/// Runs an argument-vector command in a VFS-resolved directory without a shell.
fn execute_workspace_program(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let workspace = args
        .first()
        .filter(|value| !value.is_empty())
        .ok_or("usage: operit2 workspace exec <workspace-id> <program> [args...]")?;
    let program = args
        .get(1)
        .filter(|value| !value.is_empty())
        .ok_or("workspace exec requires a program")?;
    let workspaceRecord = get_workspace(application, workspace)?;
    let workspacePath = workspaceRecord.primaryFolder().path.clone();
    let context = &application.hostManager;
    let vfs = context.vfs()?;
    let cwd = vfs.file(&local_workspace_path(&workspacePath)?)?;
    let host = context
        .managedRuntimeHost
        .as_ref()
        .ok_or("ManagedRuntimeHost is not registered for this runtime")?;
    let result = host
        .runRuntimeCommandAt(
            RuntimeProcessRequest {
                program: ManagedRuntimeProgram::Command(program.clone()),
                executablePath: None,
                args: args[2..].to_vec(),
                cwd: None,
                env: BTreeMap::new(),
            },
            &cwd,
        )
        .map_err(|error| error.message)?;
    output.push_stdout(&result.stdout);
    output.push_stderr(&result.stderr);
    output.setJsonStdout(json!({
        "workspaceId": workspace,
        "workspacePath": workspacePath,
        "workingDirectory": workspacePath,
        "exitCode": result.exitCode,
        "stdout": result.stdout,
        "stderr": result.stderr,
    }));
    Ok(())
}

/// Creates an independent workspace with an app-owned folder or one explicit mount.
async fn create_workspace(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let (name, source) =
        match args {
            [name] => (name, None),
            [name, kind, path] if matches!(kind.as_str(), "--native" | "--vfs") => {
                (name, Some((kind.as_str(), path.as_str())))
            }
            _ => return Err(
                "usage: operit2 workspace create <name> [--native <directory> | --vfs <directory>]"
                    .into(),
            ),
        };
    if name.trim().is_empty() {
        return Err("workspace name is required".into());
    }
    let path = match source {
        Some((kind, path)) => {
            let path = folder_source_path(kind, path)?;
            validate_folder(application, &path).await?;
            path
        }
        None => WorkspaceUtils::createNewDefaultWorkspace()?,
    };
    let folder = WorkspaceFolder {
        name: "default".into(),
        path: qualify_folder_path(application, &path)?,
    };
    let workspace = with_main_chat_core(application, |core| {
        core.chatHistoryDelegate
            .chatHistoryManager
            .createWorkspace(name.clone(), vec![folder])
            .map_err(|error| error.to_string())
    })??;
    print_workspace(&workspace, output);
    Ok(())
}

/// Loads one persisted workspace by identity rather than interpreting its id as a path.
fn get_workspace(application: &OperitApplication, workspaceId: &str) -> Result<Workspace, String> {
    with_main_chat_core(application, |core| {
        core.chatHistoryDelegate
            .chatHistoryManager
            .getWorkspace(workspaceId)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("workspace does not exist: {workspaceId}"))
    })?
}

/// Prints a workspace record and its stable id for subsequent CLI operations.
fn print_workspace(workspace: &Workspace, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!("Workspace: {} | {}", workspace.id, workspace.name));
    for (index, folder) in workspace.folders.iter().enumerate() {
        output.push_stdout_line(format!(
            "- {} | {}{}",
            folder.name,
            folder.path,
            if index == 0 { " | primary" } else { "" }
        ));
    }
    output.setJsonStdout(json!({"workspaceId": workspace.id, "workspace": workspace}));
}

/// Shows one workspace including its ordered mounted folders.
fn show_workspace(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let [workspaceId] = args else {
        return Err("usage: operit2 workspace show <workspace-id>".into());
    };
    print_workspace(&get_workspace(application, workspaceId)?, output);
    Ok(())
}

/// Separates explicit native ingress from already virtual, optionally node-qualified paths.
fn folder_source_path(kind: &str, path: &str) -> Result<String, String> {
    match kind {
        "--native" => VfsPath::nativePathToVfs(path),
        "--vfs" => match FileUtils::splitQualifiedVfsPath(path) {
            Some((node, path)) => Ok(FileUtils::qualifyVfsPath(
                node,
                &VfsPath::normalizeWorkspaceVfsPath(path)?,
            )),
            None => VfsPath::normalizeWorkspaceVfsPath(path),
        },
        _ => Err("folder source requires --native or --vfs".into()),
    }
}

/// Gives local mounts the same node ownership persisted by the graphical workspace picker.
fn qualify_folder_path(application: &OperitApplication, path: &str) -> Result<String, String> {
    if FileUtils::splitQualifiedVfsPath(path).is_some() {
        return Ok(path.to_owned());
    }
    let storage = application
        .hostManager
        .runtimeStorageHost
        .clone()
        .ok_or("RuntimeStorageHost is not registered")?;
    let node = operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::new(storage)
        .identity()?
        .nodeId;
    Ok(FileUtils::qualifyVfsPath(&node, path))
}

/// Verifies directory existence through the owning node's file tool before persisting a mount.
async fn validate_folder(application: &OperitApplication, path: &str) -> Result<(), String> {
    let result = application
        .toolHandler
        .clone()
        .executeTool(AITool {
            name: "file_exists".into(),
            parameters: vec![ToolParameter {
                name: "path".into(),
                value: path.into(),
            }],
        })
        .await;
    if !result.success {
        return Err(result
            .error
            .ok_or("folder validation failed without an error message")?);
    }
    match result.result {
        ToolResultData::FileExistsData(info) if info.exists && info.isDirectory => Ok(()),
        ToolResultData::FileExistsData(_) => Err(format!(
            "workspace mount must reference an existing directory: {path}"
        )),
        _ => Err("file_exists returned invalid folder metadata".into()),
    }
}

/// Applies a manager-owned folder edit and refreshes every chat sharing the workspace.
fn edit_workspace(
    application: &OperitApplication,
    edit: impl FnOnce(&ChatHistoryManager) -> ChatHistoryManagerResult<Workspace>,
) -> Result<Workspace, String> {
    with_main_chat_core(application, |core| {
        let workspace = edit(&core.chatHistoryDelegate.chatHistoryManager)
            .map_err(|error| error.to_string())?;
        let chats = core
            .chatHistoryDelegate
            .chatHistoryManager
            .loadChatHistories()
            .map_err(|error| error.to_string())?;
        for chat in chats
            .iter()
            .filter(|chat| chat.workspaceId.as_deref() == Some(workspace.id.as_str()))
        {
            core.chatHistoryDelegate
                .notifyChatWorkspaceChanged(&chat.id);
        }
        Ok(workspace)
    })?
}

/// Mounts a named directory without changing the identity or chat bindings of the workspace.
async fn mount_workspace(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let (workspaceId, name, kind, path, primary) = match args {
        [id, name, kind, path] => (id, name, kind, path, false),
        [id, name, kind, path, flag] if flag == "--primary" => (id, name, kind, path, true),
        _ => return Err("usage: operit2 workspace mount <workspace-id> <folder-name> <--native|--vfs> <directory> [--primary]".into()),
    };
    let path = qualify_folder_path(application, &folder_source_path(kind, path)?)?;
    validate_folder(application, &path).await?;
    let workspace = edit_workspace(application, |manager| {
        let workspace = manager.addWorkspaceFolder(
            workspaceId.clone(),
            WorkspaceFolder {
                name: name.clone(),
                path: path.clone(),
            },
        )?;
        if primary {
            // An already mounted path retains its existing name in the shared mount model.
            let folder = workspace
                .folders
                .iter()
                .find(|folder| folder.path == path)
                .ok_or_else(|| {
                    ChatHistoryManagerError::IllegalArgument(format!(
                        "workspace folder not found: {path}"
                    ))
                })?;
            manager.setPrimaryWorkspaceFolder(workspaceId.clone(), folder.name.clone())
        } else {
            Ok(workspace)
        }
    })?;
    print_workspace(&workspace, output);
    Ok(())
}

/// Unmounts a folder while preserving its files and requiring one remaining working directory.
fn unmount_workspace(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let [workspaceId, name] = args else {
        return Err("usage: operit2 workspace unmount <workspace-id> <folder-name>".into());
    };
    let workspace = edit_workspace(application, |manager| {
        manager.removeWorkspaceFolder(workspaceId.clone(), name.clone())
    })?;
    print_workspace(&workspace, output);
    Ok(())
}

/// Selects the primary working directory without remounting folders or rebinding chats.
fn set_primary_folder(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let [workspaceId, name] = args else {
        return Err("usage: operit2 workspace primary <workspace-id> <folder-name>".into());
    };
    let workspace = edit_workspace(application, |manager| {
        manager.setPrimaryWorkspaceFolder(workspaceId.clone(), name.clone())
    })?;
    print_workspace(&workspace, output);
    Ok(())
}

/// Deletes a workspace through the shared lifecycle while preserving mounted host files.
fn delete_workspace(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let [flag, workspaceId] = args else {
        return Err("usage: operit2 workspace delete --yes <workspace-id>".into());
    };
    if flag != "--yes" {
        return Err("usage: operit2 workspace delete --yes <workspace-id>".into());
    }
    with_main_chat_core(application, |core| {
        core.deleteWorkspace(workspaceId.clone())
    })??;
    output.push_stdout_line(format!("Deleted workspace: {workspaceId}"));
    output.setJsonStdout(json!({"workspaceId": workspaceId, "deleted": true}));
    Ok(())
}

/// Restricts local process execution and config reads to folders owned by this node.
fn local_workspace_path(path: &str) -> Result<String, String> {
    match FileUtils::splitQualifiedVfsPath(path) {
        Some((node, path))
            if Some(node)
                == operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::localNodeId()
                    .as_deref() =>
        {
            Ok(path.to_owned())
        }
        Some(_) => Err("workspace execution requires a folder owned by the current node".into()),
        None => Ok(path.to_owned()),
    }
}

/// Binds one chat to an existing workspace id without creating or mounting folders.
fn bind_workspace(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let [chatId, workspaceId] = args else {
        return Err("usage: operit2 workspace bind <chat-id> <workspace-id>".into());
    };
    with_main_chat_core(application, |core| {
        core.moveChatToWorkspace(chatId.clone(), workspaceId.clone())
    })??;
    output.push_stdout_line(format!("Bound workspace for {chatId}"));
    output.push_stdout_line(format!("Workspace: {workspaceId}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "workspaceId": workspaceId,
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
    let [chatId] = args else {
        return Err("usage: operit2 workspace unbind <chat-id>".into());
    };
    with_main_chat_core(application, |core| {
        core.unbindChatFromWorkspace(chatId.clone())
    })??;
    output.push_stdout_line(format!("Unbound workspace for {chatId}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "unbound": true
    }));
    Ok(())
}

/// Lists persisted workspaces, including those without any bound chat.
fn list_workspaces(
    application: &mut OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let workspaces = application.workspaceService().listWorkspaces()?;
    let chats = with_main_chat_core(application, |core| core.chatHistoriesFlow().value())?;
    let chatCounts = workspaces
        .iter()
        .map(|workspace| {
            (
                workspace.id.clone(),
                chats
                    .iter()
                    .filter(|chat| chat.workspaceId.as_deref() == Some(workspace.id.as_str()))
                    .count(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    output.push_stdout_line(format!("Workspaces: {}", workspaces.len()));
    for workspace in &workspaces {
        let chatCount = chatCounts[&workspace.id];
        output.push_stdout_line(format!(
            "- {} | {} | folders: {} | chats: {chatCount}",
            workspace.id,
            workspace.name,
            workspace.folders.len()
        ));
    }
    output.setJsonStdout(json!({ "workspaces": workspaces, "chatCounts": chatCounts }));
    Ok(())
}

/// Lists chats bound to one existing workspace id.
fn list_workspace_chats(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let [workspaceId] = args else {
        return Err("usage: operit2 workspace chats <workspace-id>".into());
    };
    get_workspace(application, workspaceId)?;
    let chats = with_main_chat_core(application, |core| core.chatHistoriesFlow().value())?
        .into_iter()
        .filter(|chat| chat.workspaceId.as_deref() == Some(workspaceId.as_str()))
        .collect::<Vec<_>>();
    output.push_stdout_line(format!("Chats in workspace: {}", chats.len()));
    output.push_stdout_line(format!("Workspace: {workspaceId}"));
    for chat in &chats {
        output.push_stdout_line(format!("- {} | {}", chat.id, chat.title));
    }
    output.setJsonStdout(json!({
        "workspaceId": workspaceId,
        "chats": chats
    }));
    Ok(())
}

/// Lists commands configured in the primary folder of an existing workspace.
fn list_workspace_commands(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let [workspaceId] = args else {
        return Err("usage: operit2 workspace commands <workspace-id>".into());
    };
    let workspace = get_workspace(application, workspaceId)?;
    let workspacePath = local_workspace_path(&workspace.primaryFolder().path)?;
    list_commands_at_path(
        &application.hostManager,
        workspaceId,
        &workspacePath,
        output,
    )
}

/// Runs a shortcut within the selected workspace's complete folder context.
async fn run_workspace_shortcut(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let [workspaceId, commandId] = args else {
        return Err("usage: operit2 workspace run <workspace-id> <command-id>".into());
    };
    let workspace = get_workspace(application, workspaceId)?;
    let workspacePath = local_workspace_path(&workspace.primaryFolder().path)?;
    use operit_tools::ToolExecutionManager::{
        ToolExecutionManager, ToolExposureMode, ToolRuntimeContext,
    };
    let context = ToolRuntimeContext {
        callerName: None,
        callerChatId: None,
        callerParticipantId: None,
        workspacePath: Some(workspacePath.clone()),
        workspaceFolders: workspace.folderPaths(),
        toolExposureMode: ToolExposureMode::CLI,
        extensionOwner: "core:cli".into(),
        executionData: serde_json::Map::new(),
    };
    ToolExecutionManager::scopeToolRuntimeContext(
        context,
        run_command_at_path(application, workspaceId, &workspacePath, commandId, output),
    )
    .await
}

/// Lists commands from one workspace configuration file.
fn list_commands_at_path(
    context: &HostManager,
    workspaceId: &str,
    workspacePath: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let vfs = context.vfs()?;
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
        "workspaceId": workspaceId,
        "workspacePath": workspacePath,
        "commands": config.commands
    }));
    Ok(())
}

/// Runs one command from a workspace configuration file.
async fn run_command_at_path(
    application: &OperitApplication,
    workspaceId: &str,
    workspacePath: &str,
    commandId: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let context = application.hostManager.clone();
    let vfs = context.vfs()?;
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
            workspaceId,
            workspacePath,
            &toolName,
            output,
        )
        .await;
    }

    let commandText = command
        .command
        .clone()
        .and_then(nonBlankString)
        .ok_or_else(|| "No command/tool configured".to_string())?;
    execute_workspace_shell_command(
        &context,
        workspaceId,
        workspacePath,
        &command,
        &commandText,
        output,
    )
}

/// Executes one workspace command via the tool runtime.
async fn execute_workspace_tool(
    mut handler: AIToolHandler,
    command: &CommandConfig,
    workspaceId: &str,
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

    let result = handler
        .executeTool(AITool {
            name: toolName.to_string(),
            parameters,
        })
        .await;
    print_tool_execution_result(&result, command, workspaceId, workspacePath, output)
}

/// Executes one workspace command via the terminal host.
fn execute_workspace_shell_command(
    context: &HostManager,
    workspaceId: &str,
    workspacePath: &str,
    command: &CommandConfig,
    commandText: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let terminalHost = context
        .terminalHost
        .clone()
        .ok_or_else(|| "TerminalHost is not registered for this runtime.".to_string())?;
    let vfs = context.vfs()?;
    let workingDir = workspace_command_working_dir(workspacePath, &command.workingDir)?;
    let directory = operit_host_api::TerminalWorkingDirectory::resolveSessionDirectory(
        terminalHost.as_ref(),
        None,
        &workingDir,
        &|path| {
            vfs.terminalDirectory(terminalHost.as_ref(), path)
                .map_err(operit_host_api::HostError::new)
        },
    )
    .map_err(|error| format!("failed to resolve workspace directory: {}", error.message))?;
    let sessionName = workspace_command_session_name(workspacePath, command);
    let session = terminalHost
        .createOrGetSession(&sessionName)
        .map_err(|error| {
            format!(
                "failed to create workspace terminal session: {}",
                error.message
            )
        })?;
    let entered = operit_host_api::TerminalWorkingDirectory::enterTerminalDirectory(
        terminalHost.as_ref(),
        &session.sessionId,
        &session.terminalType,
        &directory,
    );
    if let Err(error) = entered {
        if session.isNewSession {
            let _ = terminalHost.closeSession(&session.sessionId);
        }
        return Err(format!(
            "failed to enter workspace directory: {}",
            error.message
        ));
    }
    let commandOutput = terminalHost
        .executeInSession(&session.sessionId, commandText, 1800000)
        .map_err(|error| format!("failed to execute workspace command: {}", error.message))?;
    let commandOutputText = commandOutput.output.clone();
    if !commandOutputText.is_empty() {
        output.push_stdout(commandOutputText.clone());
    }
    output.push_stdout_line(format!("Exit code: {}", commandOutput.exitCode));
    output.setJsonStdout(json!({
        "workspaceId": workspaceId,
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
            "workspace tool parameter `{name}` must use a VFS path; use /virtual/windows/<drive>/... for Windows host paths"
        ));
    }

    if VfsPath::normalizeVfsPath(trimmed).is_ok() {
        return Ok(trimmed.to_string());
    }

    VfsPath::joinVfsPath(workspacePath, trimmed)
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
    workspaceId: &str,
    workspacePath: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    output.push_stdout_line(format!("Tool: {}", result.toolName));
    output.push_stdout_line(format!("Success: {}", result.success));
    let resultText = result.result.toString();
    output.setJsonStdout(json!({
        "workspaceId": workspaceId,
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
fn workspace_command_working_dir(workspacePath: &str, workingDir: &str) -> Result<String, String> {
    let trimmed = workingDir.trim();
    let workingDirPath = if trimmed.is_empty() || trimmed == "." {
        workspacePath.to_string()
    } else if startsWithHostDrivePath(trimmed) {
        return Err(
            "workspace command workingDir must use a VFS path or a path relative to the workspace"
                .to_string(),
        );
    } else if VfsPath::normalizeVfsPath(trimmed).is_ok() {
        trimmed.to_string()
    } else {
        VfsPath::joinVfsPath(workspacePath, trimmed)?
    };
    VfsPath::normalizeVfsPath(&workingDirPath)
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
        "operit2 workspace create <name> [--native <directory> | --vfs <directory>]",
        "operit2 workspace show <workspace-id>",
        "operit2 workspace mount <workspace-id> <folder-name> <--native|--vfs> <directory> [--primary]",
        "operit2 workspace unmount <workspace-id> <folder-name>",
        "operit2 workspace primary <workspace-id> <folder-name>",
        "operit2 workspace bind <chat-id> <workspace-id>",
        "operit2 workspace unbind <chat-id>",
        "operit2 workspace list",
        "operit2 workspace summary",
        "operit2 workspace delete-unbound --yes <workspace-name> [names...]",
        "operit2 workspace delete --yes <workspace-id>",
        "operit2 workspace chats <workspace-id>",
        "operit2 workspace commands <workspace-id>",
        "operit2 workspace run <workspace-id> <command-id>",
        "operit2 workspace exec <workspace-id> <program> [args...]",
        "operit2 workspace resolve-path [--native | --from <workspace>] <path>",
        "operit2 workspace open-path <vfs-path> <reveal|terminal|program <name> [args...]>",
    ];
    for line in lines {
        output.push_stdout_line(line);
    }
    let examples = [
        "operit2 workspace create project --native <absolute-directory>",
        "operit2 chat new --workspace <workspace-id>",
        "operit2 workspace mount <workspace-id> assets --vfs <vfs-directory> --primary",
    ];
    output.push_stdout_line(
        "Create a workspace, mount its folders, then bind chats by workspace ID.",
    );
    output.push_stdout_line("The first folder is the primary working directory. Use --primary when mounting to select it.");
    output.setJsonStdout(json!({ "usage": lines, "examples": examples }));
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
        let configFile = VfsPath::joinVfsPath(workspacePath, ".operit/config.json")?;
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
