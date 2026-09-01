use std::fs;
use std::path::Path;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use crate::output::CoreCommandOutput;
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_model::ChatHistory::ChatHistory;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatTurnOptions::ChatTurnOptions;
use operit_model::FunctionType::FunctionType;
use operit_model::InputProcessingState::InputProcessingState;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_providers::chat::enhance::ConversationService::ConversationService;
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use operit_runtime::services::ChatServiceCore::ChatServiceCore;
use operit_store::repository::ChatHistoryManager::ChatHistoryManager;
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

/// Builds the single-thread runtime used by chat commands that call async Core APIs.
fn build_chat_command_runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())
}

/// Runs chat history, message, branch, stats, binding, and send commands.
pub fn run_chat_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_chat_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "new" => create_chat(application, &args[1..], output),
        "list" => list_chats(application, output),
        "show" => show_chat(application, &args[1..], output),
        "current" => show_current_chat(application, output),
        "switch" => switch_chat_command(application, &args[1..], output),
        "delete" => delete_chat(application, &args[1..], output),
        "delete-message" => delete_chat_message(application, &args[1..], output),
        "clear" => clear_current_chat(application, output),
        "rollback" => rollback_chat(application, &args[1..], output),
        "branch" => create_chat_branch(application, &args[1..], output),
        "branches" => list_chat_branches(application, &args[1..], output),
        "lock" => update_chat_locked(application, &args[1..], output),
        "pin" => update_chat_pinned(application, &args[1..], output),
        "send" => send_chat_message_command(application, &args[1..], output),
        "stats" => show_chat_stats(output),
        "bind-character" => bind_chat_character(application, &args[1..], output),
        "bind-group" => bind_chat_group_card(application, &args[1..], output),
        "set-group" => set_chat_group(&args[1..], output),
        _ => {
            print_chat_usage(output);
            Ok(())
        }
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
            "- {} | {} | messages: {} | tokens: {}/{} | character: {} | group card: {} | locked: {} | pinned: {}",
            chat.id,
            chat.title,
            chat.messages.len(),
            chat.inputTokens,
            chat.outputTokens,
            option_text(chat.characterCardName.as_deref()),
            option_text(chat.characterGroupId.as_deref()),
            chat.locked,
            chat.pinned
        ));
    }
    output.setJsonStdout(serde_json::to_value(&chats).map_err(|error| error.to_string())?);
    Ok(())
}

/// Shows one chat and its messages.
fn show_chat(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat show <chat-id> [--runtime]".to_string())?
        .clone();
    let (chat, messages) = with_main_chat_core(application, |core| {
        core.switchChat(chatId.clone());
        let chat = core
            .chatHistoriesFlow()
            .value()
            .into_iter()
            .find(|chat| chat.id == chatId)
            .ok_or_else(|| format!("chat not found: {chatId}"))?;
        Ok::<_, String>((chat, core.chatHistory().clone()))
    })??;
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
fn delete_chat(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat delete <chat-id>".to_string())?
        .clone();
    let deleted = with_main_chat_core(application, |core| core.deleteChatHistory(chatId.clone()))?;
    output.push_stdout_line(format!("Deleted chat {chatId}: {deleted}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "deleted": deleted
    }));
    Ok(())
}

/// Deletes one message from the current chat by timestamp.
fn delete_chat_message(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let messageTimestamp = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat delete-message <message-timestamp>".to_string())?
        .parse::<i64>()
        .map_err(|error| error.to_string())?;
    let runtime = build_chat_command_runtime()?;
    let chatId = runtime.block_on(async {
        let mut holder = application.chatRuntimeHolder.lock().await;
        let core = holder.getCore(ChatRuntimeSlot::MAIN);
        let chatId = core
            .currentChatIdFlow()
            .value()
            .ok_or_else(|| "core has no active chat".to_string())?;
        core.deleteMessage(chatId.clone(), messageTimestamp).await;
        Ok::<_, String>(chatId)
    })?;
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
fn rollback_chat(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let messageTimestamp = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat rollback <message-timestamp>".to_string())?
        .parse::<i64>()
        .map_err(|error| error.to_string())?;
    let runtime = build_chat_command_runtime()?;
    let (chatId, rolledBack) = runtime.block_on(async {
        let mut holder = application.chatRuntimeHolder.lock().await;
        let core = holder.getCore(ChatRuntimeSlot::MAIN);
        let chatId = core
            .currentChatIdFlow()
            .value()
            .ok_or_else(|| "core has no active chat".to_string())?;
        let rolledBack = core
            .rollbackToMessage(chatId.clone(), messageTimestamp)
            .await;
        Ok::<_, String>((chatId, rolledBack))
    })?;
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
fn create_chat_branch(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let upToMessageTimestamp = parse_branch_args(args)?;
    let chatId = with_main_chat_core(application, |core| {
        core.createBranch(upToMessageTimestamp);
        core.currentChatIdFlow()
            .value()
            .ok_or_else(|| "core did not create branch".to_string())
    })??;
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
            core.currentChatIdFlow()
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
    let chatId = with_main_chat_core(application, |core| core.currentChatIdFlow().value())?;
    output.push_stdout_line(format!("Current chat: {}", option_text(chatId.as_deref())));
    output.setJsonStdout(json!({ "chatId": chatId }));
    Ok(())
}

/// Switches the current chat.
fn switch_chat_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat switch <chat-id>".to_string())?
        .clone();
    with_main_chat_core(application, |core| core.switchChat(chatId.clone()))?;
    output.push_stdout_line(format!("Current chat: {chatId}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "current": true
    }));
    Ok(())
}

/// Shows aggregate chat statistics.
fn show_chat_stats(output: &mut CoreCommandOutput) -> Result<(), String> {
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    let totalChats = manager
        .getTotalChatCount()
        .map_err(|error| error.to_string())?;
    let totalMessages = manager
        .getTotalMessageCount()
        .map_err(|error| error.to_string())?;
    let characterStats = manager
        .characterCardStatsFlow()
        .map_err(|error| error.to_string())?;
    let groupStats = manager
        .characterGroupStatsFlow()
        .map_err(|error| error.to_string())?;
    output.push_stdout_line("Chat statistics");
    output.push_stdout_line(format!("Total chats: {totalChats}"));
    output.push_stdout_line(format!("Total messages: {totalMessages}"));
    output.push_stdout_line(format!("Character cards: {}", characterStats.len()));
    for stats in &characterStats {
        output.push_stdout_line(format!(
            "- {} | chats: {} | messages: {}",
            option_text(stats.characterCardName.as_deref()),
            stats.chatCount,
            stats.messageCount
        ));
    }
    output.push_stdout_line(format!("Character groups: {}", groupStats.len()));
    for stats in &groupStats {
        output.push_stdout_line(format!(
            "- {} | chats: {} | messages: {}",
            option_text(stats.characterGroupId.as_deref()),
            stats.chatCount,
            stats.messageCount
        ));
    }
    output.setJsonStdout(json!({
        "totalChats": totalChats,
        "totalMessages": totalMessages,
        "characterCards": characterStats.iter().map(|stats| {
            json!({
                "characterCardName": stats.characterCardName,
                "chatCount": stats.chatCount,
                "messageCount": stats.messageCount
            })
        }).collect::<Vec<_>>(),
        "characterGroups": groupStats.iter().map(|stats| {
            json!({
                "characterGroupId": stats.characterGroupId,
                "chatCount": stats.chatCount,
                "messageCount": stats.messageCount
            })
        }).collect::<Vec<_>>()
    }));
    Ok(())
}

/// Binds a character card to one chat.
fn bind_chat_character(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| {
            "usage: operit2 chat bind-character <chat-id> <character-card-name>".to_string()
        })?
        .clone();
    let characterCardName = args
        .get(1)
        .cloned()
        .and_then(nonBlankString)
        .ok_or_else(|| {
            "usage: operit2 chat bind-character <chat-id> <character-card-name>".to_string()
        })?;
    with_main_chat_core(application, |core| {
        core.updateChatCharacterCard(chatId.clone(), Some(characterCardName.clone()))
    })?;
    output.push_stdout_line(format!("Updated character binding for {chatId}"));
    output.push_stdout_line(format!("Character card: {characterCardName}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "characterCardName": characterCardName,
        "updated": true
    }));
    Ok(())
}

/// Binds a character group card to one chat.
fn bind_chat_group_card(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat bind-group <chat-id> <character-group-id>".to_string())?
        .clone();
    let characterGroupId = args
        .get(1)
        .cloned()
        .and_then(nonBlankString)
        .ok_or_else(|| {
            "usage: operit2 chat bind-group <chat-id> <character-group-id>".to_string()
        })?;
    with_main_chat_core(application, |core| {
        core.updateChatCharacterGroup(chatId.clone(), Some(characterGroupId.clone()))
    })?;
    output.push_stdout_line(format!("Updated character group binding for {chatId}"));
    output.push_stdout_line(format!("Character group: {characterGroupId}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "characterGroupId": characterGroupId,
        "updated": true
    }));
    Ok(())
}

/// Sets a chat history group label.
fn set_chat_group(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat set-group <chat-id> <group-name>".to_string())?
        .clone();
    let groupName = args
        .get(1)
        .cloned()
        .and_then(nonBlankString)
        .ok_or_else(|| "usage: operit2 chat set-group <chat-id> <group-name>".to_string())?;
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    manager
        .updateChatGroup(chatId.clone(), Some(groupName.clone()))
        .map_err(|error| error.to_string())?;
    output.push_stdout_line(format!("Updated group for {chatId}"));
    output.push_stdout_line(format!("Group: {groupName}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "group": groupName,
        "updated": true
    }));
    Ok(())
}

/// Creates a new chat.
fn create_chat(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let (characterCardName, characterGroupId, group) = parse_chat_new_args(args)?;
    let jsonCharacterCardName = characterCardName.clone();
    let jsonCharacterGroupId = characterGroupId.clone();
    let jsonGroup = group.clone();
    let chatId = with_main_chat_core(application, |core| {
        core.createNewChat(characterCardName, group, true, true, characterGroupId);
        core.currentChatIdFlow()
            .value()
            .ok_or_else(|| "core did not create chat".to_string())
    })??;
    output.push_stdout_line(format!("Created chat {chatId}"));
    output.setJsonStdout(json!({
        "chatId": chatId,
        "characterCardName": jsonCharacterCardName,
        "characterGroupId": jsonCharacterGroupId,
        "group": jsonGroup
    }));
    Ok(())
}

fn parse_chat_new_args(
    args: &[String],
) -> Result<(Option<String>, Option<String>, Option<String>), String> {
    let mut characterCardName = None;
    let mut characterGroupId = None;
    let mut group = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--character" => {
                index += 1;
                characterCardName = args.get(index).cloned().and_then(nonBlankString);
            }
            "--group-card" => {
                index += 1;
                characterGroupId = args.get(index).cloned().and_then(nonBlankString);
            }
            "--group" => {
                index += 1;
                group = args.get(index).cloned().and_then(nonBlankString);
            }
            _ => return Err("usage: operit2 chat new [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]".to_string()),
        }
        index += 1;
    }
    Ok((characterCardName, characterGroupId, group))
}

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

#[derive(Clone, Debug)]
struct ChatSendResult {
    chatId: String,
    aiMessage: ChatMessage,
}

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

/// Sends one user message and waits for the committed AI response.
fn send_chat_message_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let sendArgs = parse_chat_send_args(args)?;
    let runtime = build_chat_command_runtime()?;
    let result = runtime.block_on(send_chat_message_with_application(application, sendArgs))?;
    print_chat_send_result(&result, output);
    Ok(())
}

async fn send_chat_message_with_application(
    application: &mut OperitApplication,
    sendArgs: ChatSendArgs,
) -> Result<ChatSendResult, String> {
    let beforeLastAiTimestamp =
        dispatch_chat_message_with_application(application, sendArgs).await?;
    let (currentChatId, aiMessage) = with_main_chat_core(application, |core| {
        let currentChatId = core
            .currentChatIdFlow()
            .value()
            .ok_or_else(|| "core has no active chat after send".to_string())?;
        let aiMessage = core
            .chatHistory()
            .iter()
            .rev()
            .find(|message| message.sender == "ai" && message.timestamp > beforeLastAiTimestamp)
            .ok_or_else(|| "core did not produce ai message for current turn".to_string())?
            .clone();
        Ok::<_, String>((currentChatId, aiMessage))
    })??;
    let aiMessage = wait_for_committed_ai_message(
        application,
        &currentChatId,
        aiMessage.timestamp,
        Duration::from_secs(30),
    )?;
    Ok(ChatSendResult {
        chatId: currentChatId,
        aiMessage,
    })
}

async fn dispatch_chat_message_with_application(
    application: &mut OperitApplication,
    sendArgs: ChatSendArgs,
) -> Result<i64, String> {
    let functionalConfigManager = FunctionalConfigManager::default();
    let chatBinding = functionalConfigManager
        .getModelBindingForFunction(FunctionType::CHAT)
        .map_err(|error| error.to_string())?;
    let turnOptions = ChatTurnOptions::default();
    let mut holder = application.chatRuntimeHolder.lock().await;
    let core = holder.getCore(ChatRuntimeSlot::MAIN);
    core.enhancedAiService = Some(EnhancedAIService::new(
        application.toolHandler.clone(),
        application.providerRuntimeContext.clone(),
    ));
    if let Some(chatId) = sendArgs.chatId.as_ref() {
        core.switchChat(chatId.clone());
    }
    let attachments = sendArgs
        .attachmentPaths
        .iter()
        .map(|path| build_attachment_info(path))
        .collect::<Result<Vec<_>, _>>()?;
    let replyToMessage = match sendArgs.replyToTimestamp {
        Some(timestamp) => core
            .chatHistory()
            .iter()
            .find(|message| message.timestamp == timestamp)
            .cloned()
            .ok_or_else(|| format!("reply-to message not found: {timestamp}"))?,
        None => ChatMessage::new(String::new()),
    };
    let replyToMessage = if replyToMessage.sender.is_empty() {
        None
    } else {
        Some(replyToMessage)
    };
    let beforeLastAiTimestamp = core
        .chatHistory()
        .iter()
        .filter(|message| message.sender == "ai")
        .map(|message| message.timestamp)
        .max()
        .unwrap_or(0);
    core.sendUserMessage(
        PromptFunctionType::CHAT,
        None,
        None,
        sendArgs.message,
        None,
        Some(chatBinding.providerId),
        Some(chatBinding.modelId),
        attachments,
        replyToMessage,
        turnOptions,
    )
    .await;
    let currentChatId = core
        .currentChatIdFlow()
        .value()
        .ok_or_else(|| "core has no active chat after send".to_string())?;
    let inputProcessingStateByChatId = core.inputProcessingStateByChatIdFlow().value();
    match inputProcessingStateByChatId.get(&currentChatId) {
        Some(InputProcessingState::Error { message }) => return Err(message.clone()),
        _ => {}
    }
    Ok(beforeLastAiTimestamp)
}

/// Waits until the local CLI command observes the committed AI message.
fn wait_for_committed_ai_message(
    application: &mut OperitApplication,
    chatId: &str,
    timestamp: i64,
    timeout: Duration,
) -> Result<ChatMessage, String> {
    enum WaitSignal {
        Ready(ChatMessage),
        Error(String),
    }

    let startedAt = Instant::now();
    let (messageFlow, stateFlow) = with_main_chat_core(application, |core| {
        (
            core.localChatMessagesFlow(chatId.to_string()),
            core.inputProcessingStateByChatIdFlow(),
        )
    })?;

    if let Some(message) = messageFlow.value().into_iter().find(|message| {
        message.sender == "ai" && message.timestamp == timestamp && message.completedAt > 0
    }) {
        return Ok(message);
    }
    if let Some(InputProcessingState::Error { message }) = stateFlow.value().get(chatId) {
        return Err(message.clone());
    }

    let (sender, receiver) = mpsc::channel();
    let messageSender = sender.clone();
    let messageSubscriptionId = messageFlow.subscribe(move |messages| {
        if let Some(message) = messages.into_iter().find(|message| {
            message.sender == "ai" && message.timestamp == timestamp && message.completedAt > 0
        }) {
            let _ = messageSender.send(WaitSignal::Ready(message));
        }
    });
    let stateSender = sender.clone();
    let chatIdForState = chatId.to_string();
    let stateSubscriptionId = stateFlow.subscribe(move |stateByChatId| {
        if let Some(InputProcessingState::Error { message }) = stateByChatId.get(&chatIdForState) {
            let _ = stateSender.send(WaitSignal::Error(message.clone()));
        }
    });
    drop(sender);

    let result = (|| {
        if let Some(message) = messageFlow.value().into_iter().find(|message| {
            message.sender == "ai" && message.timestamp == timestamp && message.completedAt > 0
        }) {
            return Ok(message);
        }
        if let Some(InputProcessingState::Error { message }) = stateFlow.value().get(chatId) {
            return Err(message.clone());
        }
        let elapsed = startedAt.elapsed();
        let remaining = if elapsed >= timeout {
            return Err(format!(
                "timed out waiting for committed ai message: chat={chatId} timestamp={timestamp}"
            ));
        } else {
            timeout - elapsed
        };
        match receiver.recv_timeout(remaining) {
            Ok(WaitSignal::Ready(message)) => Ok(message),
            Ok(WaitSignal::Error(message)) => Err(message),
            Err(RecvTimeoutError::Timeout) => Err(format!(
                "timed out waiting for committed ai message: chat={chatId} timestamp={timestamp}"
            )),
            Err(RecvTimeoutError::Disconnected) => {
                Err("chat message wait channel disconnected".to_string())
            }
        }
    })();
    messageFlow.unsubscribe(messageSubscriptionId);
    stateFlow.unsubscribe(stateSubscriptionId);
    result
}

/// Prints a completed chat send result.
fn print_chat_send_result(result: &ChatSendResult, output: &mut CoreCommandOutput) {
    let text = result.aiMessage.displayText();
    output.push_stdout(&text);
    output.push_stdout_line("");
    output.push_stdout_line(format!(
        "Chat: {} | Provider: {} | Model: {}",
        result.chatId, result.aiMessage.provider, result.aiMessage.modelName
    ));
    output.push_stdout_line(format!(
        "Tokens: input {} | cached input {} | output {}",
        result.aiMessage.inputTokens,
        result.aiMessage.cachedInputTokens,
        result.aiMessage.outputTokens
    ));
    output.setJsonStdout(json!({
        "chatId": &result.chatId,
        "text": &text,
        "message": &result.aiMessage,
        "usage": {
            "inputTokens": result.aiMessage.inputTokens,
            "cachedInputTokens": result.aiMessage.cachedInputTokens,
            "outputTokens": result.aiMessage.outputTokens
        },
        "provider": &result.aiMessage.provider,
        "modelName": &result.aiMessage.modelName
    }));
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
    output.push_stdout_line(format!("Group: {}", chat.group.clone().unwrap_or_default()));
    output.push_stdout_line(format!("Display order: {}", chat.displayOrder));
    output.push_stdout_line(format!(
        "Workspace: {}",
        chat.workspace.clone().unwrap_or_default()
    ));
    output.push_stdout_line(format!(
        "Parent chat: {}",
        chat.parentChatId.clone().unwrap_or_default()
    ));
    output.push_stdout_line(format!(
        "Character: {}",
        chat.characterCardName.clone().unwrap_or_default()
    ));
    output.push_stdout_line(format!(
        "Character group: {}",
        chat.characterGroupId.clone().unwrap_or_default()
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

/// Converts non-empty text to an owned string.
fn nonBlankString(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
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
        "operit2 chat new [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]",
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
        "operit2 chat bind-character <chat-id> <character-card-name>",
        "operit2 chat bind-group <chat-id> <character-group-id>",
        "operit2 chat set-group <chat-id> <group-name>",
        "operit2 chat send [--chat <chat-id>] <message>",
    ];
    for line in lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({ "usage": lines }));
}
