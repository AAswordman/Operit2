use std::fs;
use std::path::Path;

use crate::output::CoreCommandOutput;
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_model::ChatHistory::ChatHistory;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatTurnOptions::ChatTurnOptions;
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::services::ChatServiceCore::ChatServiceCore;
use operit_store::repository::ChatHistoryManager::ChatHistoryManager;
use operit_tools::runtime_support::{RuntimeChatSendRequest, RuntimeChatSlot};
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

/// Runs only host-owned chat history, messages, branches, neutral statistics and sends.
pub async fn run_chat_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_chat_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "new" => create_chat(application, &args[1..], output).await,
        "list" => list_chats(application, output),
        "show" => show_chat(application, &args[1..], output).await,
        "current" => show_current_chat(application, output),
        "switch" => switch_chat_command(application, &args[1..], output).await,
        "delete" => delete_chat(application, &args[1..], output).await,
        "delete-message" => delete_chat_message(application, &args[1..], output).await,
        "clear" => clear_current_chat(application, output),
        "rollback" => rollback_chat(application, &args[1..], output).await,
        "branch" => create_chat_branch(application, &args[1..], output).await,
        "branches" => list_chat_branches(application, &args[1..], output),
        "lock" => update_chat_locked(application, &args[1..], output),
        "pin" => update_chat_pinned(application, &args[1..], output),
        "send" => send_chat_message_command(application, &args[1..], output).await,
        "stats" => show_chat_stats(output),
        _ => Err(format!("unknown chat command: {}", args[0])),
    }
}

/// Lists all chats with compact metadata.
fn list_chats(
    application: &mut OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chats = with_main_chat_core(application, |core| core.chatHistoriesFlow().value())?;
    output.push_stdout_line(format!("Chats: {}", chats.len()));
    for chat in &chats {
        output.push_stdout_line(format!(
            "- {} | {} | messages: {} | tokens: {}/{} | locked: {} | pinned: {}",
            chat.id,
            chat.title,
            chat.messages.len(),
            chat.inputTokens,
            chat.outputTokens,
            chat.locked,
            chat.pinned
        ));
    }
    output.setJsonStdout(serde_json::to_value(&chats).map_err(|error| error.to_string())?);
    Ok(())
}

/// Shows one chat and its messages.
async fn show_chat(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat show <chat-id> [--runtime]".to_string())?
        .clone();
    let (chat, messages) = async {
        let mut holder = application.chatRuntimeHolder.lock().await;
        let core = holder.getCore(ChatRuntimeSlot::MAIN);
        core.switchChat(chatId.clone()).await?;
        let chat = core
            .chatHistoriesFlow()
            .value()
            .into_iter()
            .find(|chat| chat.id == chatId)
            .ok_or_else(|| format!("chat not found: {chatId}"))?;
        Ok::<_, String>((chat, core.chatHistory()))
    }
    .await?;
    print_chat_history_header(&chat, output);
    for message in &messages {
        print_chat_message(&message, output);
    }
    output.setJsonStdout(json!({
        "chat": chat,
        "messages": messages
    }));
    Ok(())
}

/// Deletes one chat.
async fn delete_chat(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat delete <chat-id>".to_string())?
        .clone();
    let deleted = async {
        let mut holder = application.chatRuntimeHolder.lock().await;
        holder
            .getCore(ChatRuntimeSlot::MAIN)
            .deleteChatHistory(chatId.clone())
            .await
    }
    .await?;
    output.push_stdout_line(format!("Deleted chat {chatId}: {deleted}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "deleted": deleted
    }));
    Ok(())
}

/// Deletes one message from the current chat by timestamp.
async fn delete_chat_message(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let messageTimestamp = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat delete-message <message-timestamp>".to_string())?
        .parse::<i64>()
        .map_err(|error| error.to_string())?;
    let chatId = async {
        let mut holder = application.chatRuntimeHolder.lock().await;
        let core = holder.getCore(ChatRuntimeSlot::MAIN);
        let chatId = core
            .chatHistoryDelegate
            .currentChatIdFlow()
            .value()
            .ok_or_else(|| "core has no active chat".to_string())?;
        core.deleteMessage(chatId.clone(), messageTimestamp).await;
        Ok::<_, String>(chatId)
    }
    .await?;
    output.push_stdout_line(format!("Deleted message {messageTimestamp} from {chatId}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "messageTimestamp": messageTimestamp,
        "deleted": true
    }));
    Ok(())
}

/// Clears the current chat.
fn clear_current_chat(
    application: &mut OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    with_main_chat_core(application, |core| core.clearCurrentChat())?;
    output.push_stdout_line("Cleared current chat");
    output.setJsonStdout(json!({ "clearedCurrentChat": true }));
    Ok(())
}

/// Rolls the current chat back to one user message timestamp.
async fn rollback_chat(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let messageTimestamp = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat rollback <message-timestamp>".to_string())?
        .parse::<i64>()
        .map_err(|error| error.to_string())?;
    let (chatId, rolledBack) = async {
        let mut holder = application.chatRuntimeHolder.lock().await;
        let core = holder.getCore(ChatRuntimeSlot::MAIN);
        let chatId = core
            .chatHistoryDelegate
            .currentChatIdFlow()
            .value()
            .ok_or_else(|| "core has no active chat".to_string())?;
        let rolledBack = core
            .rollbackToMessage(chatId.clone(), messageTimestamp)
            .await;
        Ok::<_, String>((chatId, rolledBack))
    }
    .await?;
    let rolledBackMessage = rolledBack.clone();
    if rolledBack.is_some() {
        output.push_stdout_line(format!(
            "Rolled back {chatId} to message {messageTimestamp}"
        ));
    } else {
        output.push_stdout_line("Rollback not applied: message must exist and be a user message");
    }
    output.setJsonStdout(json!({
        "chatId": chatId,
        "messageTimestamp": messageTimestamp,
        "rolledBack": rolledBackMessage.is_some(),
        "message": rolledBackMessage
    }));
    Ok(())
}

/// Creates a branch from the current chat.
async fn create_chat_branch(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let upToMessageTimestamp = parse_branch_args(args)?;
    let service = EnhancedAIService::new(
        application.toolHandler.clone(),
        application.providerRuntimeContext.clone(),
    );
    let chatId = async {
        let mut holder = application.chatRuntimeHolder.lock().await;
        let core = holder.getCore(ChatRuntimeSlot::MAIN);
        core.enhancedAiService = Some(service);
        core.createBranch(upToMessageTimestamp).await
    }
    .await?;
    output.push_stdout_line(format!("Created chat branch {chatId}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "upToMessageTimestamp": upToMessageTimestamp
    }));
    Ok(())
}

/// Lists branches for a parent chat.
fn list_chat_branches(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let (parentChatId, branches) = with_main_chat_core(application, |core| {
        let parentChatId = args.get(0).cloned().map(Ok).unwrap_or_else(|| {
            core.chatHistoryDelegate
                .currentChatIdFlow()
                .value()
                .ok_or_else(|| "usage: operit2 chat branches [parent-chat-id]".to_string())
        })?;
        let branches = core.getBranches(parentChatId.clone());
        Ok::<_, String>((parentChatId, branches))
    })??;
    output.push_stdout_line(format!("Branches for {parentChatId}: {}", branches.len()));
    for chat in &branches {
        output.push_stdout_line(format!(
            "- {} | {} | created: {} | updated: {} | locked: {} | pinned: {}",
            chat.id, chat.title, chat.createdAt, chat.updatedAt, chat.locked, chat.pinned
        ));
    }
    output.setJsonStdout(json!({
        "parentChatId": parentChatId,
        "branches": branches
    }));
    Ok(())
}

/// Updates the locked flag for one chat.
fn update_chat_locked(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let (chatId, locked) = parse_chat_bool_update_args(args, "lock")?;
    with_main_chat_core(application, |core| {
        core.updateChatLocked(chatId.clone(), locked)
    })?;
    output.push_stdout_line(format!("Chat {chatId} locked: {locked}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "locked": locked,
        "updated": true
    }));
    Ok(())
}

/// Updates the pinned flag for one chat.
fn update_chat_pinned(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let (chatId, pinned) = parse_chat_bool_update_args(args, "pin")?;
    with_main_chat_core(application, |core| {
        core.updateChatPinned(chatId.clone(), pinned)
    })?;
    output.push_stdout_line(format!("Chat {chatId} pinned: {pinned}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "pinned": pinned,
        "updated": true
    }));
    Ok(())
}

/// Shows the current chat id.
fn show_current_chat(
    application: &mut OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = with_main_chat_core(application, |core| {
        core.chatHistoryDelegate.currentChatIdFlow().value()
    })?;
    output.push_stdout_line(format!("Current chat: {}", option_text(chatId.as_deref())));
    output.setJsonStdout(json!({ "chatId": chatId }));
    Ok(())
}

/// Switches the current chat.
async fn switch_chat_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat switch <chat-id>".to_string())?
        .clone();
    async {
        let mut holder = application.chatRuntimeHolder.lock().await;
        holder
            .getCore(ChatRuntimeSlot::MAIN)
            .switchChat(chatId.clone())
            .await
    }
    .await?;
    output.push_stdout_line(format!("Current chat: {chatId}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "current": true
    }));
    Ok(())
}

/// Shows only neutral aggregate statistics from canonical host records.
fn show_chat_stats(output: &mut CoreCommandOutput) -> Result<(), String> {
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    let totalChats = manager
        .getTotalChatCount()
        .map_err(|error| error.to_string())?;
    let totalMessages = manager
        .getTotalMessageCount()
        .map_err(|error| error.to_string())?;
    output.push_stdout_line("Chat statistics");
    output.push_stdout_line(format!("Total chats: {totalChats}"));
    output.push_stdout_line(format!("Total messages: {totalMessages}"));
    output.setJsonStdout(json!({ "totalChats": totalChats, "totalMessages": totalMessages }));
    Ok(())
}

/// Holds explicit generic creation options without interpreting plugin input.
#[derive(Debug)]
struct ChatNewArgs {
    setAsCurrentChat: bool,
    sourceChatId: Option<String>,
    input: Option<serde_json::Value>,
}

/// Creates a chat through the awaited canonical lifecycle and uses its returned identity.
async fn create_chat(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let options = parse_chat_new_args(args)?;
    let service = EnhancedAIService::new(
        application.toolHandler.clone(),
        application.providerRuntimeContext.clone(),
    );
    let chatId = async {
        let mut holder = application.chatRuntimeHolder.lock().await;
        let core = holder.getCore(ChatRuntimeSlot::MAIN);
        core.enhancedAiService = Some(service);
        core.createNewChat(
            options.setAsCurrentChat,
            options.sourceChatId.clone(),
            options.input.clone(),
        )
        .await
    }
    .await?;
    output.push_stdout_line(format!("Created chat {chatId}"));
    output.setJsonStdout(json!({ "chatId": chatId, "setAsCurrentChat": options.setAsCurrentChat, "sourceChatId": options.sourceChatId, "input": options.input }));
    Ok(())
}

/// Parses only the generic source, opaque JSON input and current-selection switches.
fn parse_chat_new_args(args: &[String]) -> Result<ChatNewArgs, String> {
    let usage = "usage: operit2 chat new [--set-current <true|false>] [--source <chat-id>] [--input <json-object>]";
    let mut options = ChatNewArgs {
        setAsCurrentChat: true,
        sourceChatId: None,
        input: None,
    };
    let mut hasSetCurrent = false;
    let mut index = 0;
    while index < args.len() {
        let option = args[index].as_str();
        index += 1;
        let value = args.get(index).ok_or_else(|| usage.to_string())?;
        match option {
            "--set-current" => {
                if hasSetCurrent {
                    return Err("Duplicate --set-current option".to_string());
                }
                options.setAsCurrentChat = parse_bool_arg(value)?;
                hasSetCurrent = true;
            }
            "--source" => {
                if options.sourceChatId.is_some() {
                    return Err("Duplicate --source option".to_string());
                }
                if value.trim().is_empty() || value.trim() != value {
                    return Err("Source chat id must be nonblank canonical text".to_string());
                }
                options.sourceChatId = Some(value.clone());
            }
            "--input" => {
                if options.input.is_some() {
                    return Err("Duplicate --input option".to_string());
                }
                let input: serde_json::Value =
                    serde_json::from_str(value).map_err(|error| error.to_string())?;
                if !input.is_object() {
                    return Err("Chat creation input must be a JSON object".to_string());
                }
                options.input = Some(input);
            }
            _ => return Err(usage.to_string()),
        }
        index += 1;
    }
    Ok(options)
}

/// Parses the explicit source cutoff for a branch without changing its identity.
fn parse_branch_args(args: &[String]) -> Result<Option<i64>, String> {
    let usage = "usage: operit2 chat branch [--up-to <message-timestamp>]";
    let mut upToMessageTimestamp = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--up-to" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| usage.to_string())?;
                upToMessageTimestamp =
                    Some(value.parse::<i64>().map_err(|error| error.to_string())?);
            }
            _ => return Err(usage.to_string()),
        }
        index += 1;
    }
    Ok(upToMessageTimestamp)
}

/// Parses the exact host metadata flag and its target conversation.
fn parse_chat_bool_update_args(args: &[String], command: &str) -> Result<(String, bool), String> {
    let usage = format!("usage: operit2 chat {command} <chat-id> <true|false>");
    let chatId = args.get(0).ok_or_else(|| usage.clone())?.clone();
    let value = args.get(1).ok_or_else(|| usage.clone())?;
    let parsed = parse_bool_arg(value).map_err(|_| usage)?;
    Ok((chatId, parsed))
}

/// Parses the exact boolean values accepted by chat update commands.
fn parse_bool_arg(value: &str) -> Result<bool, String> {
    match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("invalid bool: {other}; expected true | false")),
    }
}

#[derive(Clone, Debug)]
struct ChatSendArgs {
    chatId: Option<String>,
    message: String,
    attachmentPaths: Vec<String>,
    replyToTimestamp: Option<i64>,
}

/// Parses a neutral chat send with explicit attachments and reply identity.
fn parse_chat_send_args(args: &[String]) -> Result<ChatSendArgs, String> {
    if args.is_empty() {
        return Err("usage: operit2 chat send [--chat <chat-id>] [--attachment <path>] [--reply-to <timestamp>] <message>".to_string());
    }
    let usage = "usage: operit2 chat send [--chat <chat-id>] [--attachment <path>] [--reply-to <timestamp>] <message>";
    let mut chatId = None;
    let mut attachmentPaths = Vec::new();
    let mut replyToTimestamp = None;
    let mut messageParts = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--chat" => {
                index += 1;
                chatId = Some(args.get(index).ok_or_else(|| usage.to_string())?.clone());
            }
            "--attachment" | "--attach" => {
                index += 1;
                attachmentPaths.push(args.get(index).ok_or_else(|| usage.to_string())?.clone());
            }
            "--reply-to" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| usage.to_string())?;
                replyToTimestamp = Some(
                    value
                        .parse::<i64>()
                        .map_err(|_| "reply-to must be a message timestamp".to_string())?,
                );
            }
            value => messageParts.push(value.to_string()),
        }
        index += 1;
    }
    if messageParts.is_empty() {
        return Err(usage.to_string());
    }
    Ok(ChatSendArgs {
        chatId,
        message: messageParts.join(" "),
        attachmentPaths,
        replyToTimestamp,
    })
}

/// Sends one user message and awaits its native originating-turn outcome.
async fn send_chat_message_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let sendArgs = parse_chat_send_args(args)?;
    let result = send_chat_message_with_application(application, sendArgs).await?;
    print_chat_send_result(&result, output)?;
    Ok(())
}

/// Awaits the generic originating-turn receipt without inferring an assistant from global history.
async fn send_chat_message_with_application(
    application: &mut OperitApplication,
    sendArgs: ChatSendArgs,
) -> Result<serde_json::Value, String> {
    let service = EnhancedAIService::new(
        application.toolHandler.clone(),
        application.providerRuntimeContext.clone(),
    );
    {
        let mut holder = application.chatRuntimeHolder.lock().await;
        let core = holder.getCore(ChatRuntimeSlot::MAIN);
        if core.enhancedAiService.is_none() {
            core.enhancedAiService = Some(service);
        }
    }
    let attachments = sendArgs
        .attachmentPaths
        .iter()
        .map(|path| build_attachment_info(path))
        .collect::<Result<Vec<_>, _>>()?;
    let request = RuntimeChatSendRequest {
        slot: RuntimeChatSlot::MAIN,
        participantId: None,
        chatId: sendArgs.chatId,
        message: sendArgs.message,
        proxySenderName: None,
        attachments,
        replyToMessageTimestamp: sendArgs.replyToTimestamp,
        turnOptions: ChatTurnOptions::default(),
    };
    let receipt = application
        .toolHandler
        .runtimeSupport()
        .sendChatMessage(request)
        .await?;
    serde_json::to_value(&receipt).map_err(|error| error.to_string())
}

/// Preserves the complete native completion, cancellation, blocked or consumed outcome in text and JSON.
fn print_chat_send_result(
    result: &serde_json::Value,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    output
        .push_stdout_line(serde_json::to_string_pretty(result).map_err(|error| error.to_string())?);
    output.setJsonStdout(result.clone());
    Ok(())
}

/// Builds attachment metadata for one path supplied to a chat send command.
fn build_attachment_info(path: &str) -> Result<AttachmentInfo, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("attachment metadata failed: {path}: {error}"))?;
    let fileName = Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("attachment file name invalid: {path}"))?
        .to_string();
    let mimeType = guess_mime_type(path).to_string();
    let content = if mimeType == "text/plain" {
        fs::read_to_string(path)
            .map_err(|error| format!("attachment read failed: {path}: {error}"))?
    } else {
        String::new()
    };
    Ok(AttachmentInfo {
        nodeId: operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::localNodeId(),
        filePath: path.to_string(),
        fileName,
        mimeType,
        fileSize: metadata.len() as i64,
        content,
    })
}

/// Detects a MIME type from a supported attachment extension.
fn guess_mime_type(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("txt") | Some("md") | Some("rs") | Some("kt") | Some("json") | Some("toml") => {
            "text/plain"
        }
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("mp4") => "video/mp4",
        _ => "application/octet-stream",
    }
}

/// Prints one chat header for a human reader.
fn print_chat_history_header(chat: &ChatHistory, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!("Chat {}", chat.id));
    output.push_stdout_line(format!("Title: {}", chat.title));
    output.push_stdout_line(format!("Created: {}", chat.createdAt));
    output.push_stdout_line(format!("Updated: {}", chat.updatedAt));
    output.push_stdout_line(format!("Input tokens: {}", chat.inputTokens));
    output.push_stdout_line(format!("Output tokens: {}", chat.outputTokens));
    output.push_stdout_line(format!("Context window: {}", chat.currentWindowSize));
    output.push_stdout_line(format!("Display order: {}", chat.displayOrder));
    output.push_stdout_line(format!(
        "Workspace: {}",
        chat.workspacePrimaryPath.clone().unwrap_or_default()
    ));
    output.push_stdout_line(format!(
        "Parent chat: {}",
        chat.parentChatId.clone().unwrap_or_default()
    ));
    output.push_stdout_line(format!("Locked: {}", chat.locked));
    output.push_stdout_line(format!("Pinned: {}", chat.pinned));
}

/// Prints one chat message for a human reader.
fn print_chat_message(message: &ChatMessage, output: &mut CoreCommandOutput) {
    output.push_stdout_line("--- message ---");
    output.push_stdout_line(format!("Sender: {}", message.sender));
    output.push_stdout_line(format!("Timestamp: {}", message.timestamp));
    output.push_stdout_line(format!("Role: {}", message.roleName));
    output.push_stdout_line(format!(
        "Selected variant: {}",
        message.selectedVariantIndex
    ));
    output.push_stdout_line(format!("Variants: {}", message.variantCount));
    output.push_stdout_line(format!("Provider: {}", message.provider));
    output.push_stdout_line(format!("Model: {}", message.modelName));
    output.push_stdout_line(format!("Input tokens: {}", message.inputTokens));
    output.push_stdout_line(format!(
        "Cached input tokens: {}",
        message.cachedInputTokens
    ));
    output.push_stdout_line(format!("Output tokens: {}", message.outputTokens));
    output.push_stdout_line(format!("Sent at: {}", message.sentAt));
    output.push_stdout_line(format!("Wait duration: {} ms", message.waitDurationMs));
    output.push_stdout_line(format!("Output duration: {} ms", message.outputDurationMs));
    output.push_stdout_line(format!("Completed at: {}", message.completedAt));
    output.push_stdout_line(format!("Display mode: {:?}", message.displayMode));
    output.push_stdout_line(format!("Favorite: {}", message.isFavorite));
    output.push_stdout_line(format!("Content: {}", message.displayText()));
}

/// Formats optional text for readable command output.
fn option_text(value: Option<&str>) -> &str {
    match value {
        Some(text) => text,
        None => "-",
    }
}

/// Prints chat command usage.
fn print_chat_usage(output: &mut CoreCommandOutput) {
    let lines = [
        "operit2 chat new [--set-current <true|false>] [--source <chat-id>] [--input <json-object>]",
        "operit2 chat list",
        "operit2 chat show <chat-id> [--runtime]",
        "operit2 chat current",
        "operit2 chat switch <chat-id>",
        "operit2 chat delete <chat-id>",
        "operit2 chat delete-message <message-timestamp>",
        "operit2 chat clear",
        "operit2 chat rollback <message-timestamp>",
        "operit2 chat branch [--up-to <message-timestamp>]",
        "operit2 chat branches [parent-chat-id]",
        "operit2 chat lock <chat-id> <true|false>",
        "operit2 chat pin <chat-id> <true|false>",
        "operit2 chat stats",
        "operit2 chat send [--chat <chat-id>] <message>",
    ];
    for line in lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({ "usage": lines }));
}
