use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::output::CoreCommandOutput;
use operit_runtime::api::chat::enhance::ConversationService::ConversationService;
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::api::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::data::model::AttachmentInfo::AttachmentInfo;
use operit_runtime::data::model::ChatHistory::ChatHistory;
use operit_runtime::data::model::ChatMessage::ChatMessage;
use operit_runtime::data::model::ChatTurnOptions::ChatTurnOptions;
use operit_runtime::data::model::FunctionType::FunctionType;
use operit_runtime::data::model::InputProcessingState::InputProcessingState;
use operit_runtime::data::model::PromptFunctionType::PromptFunctionType;
use operit_runtime::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use operit_runtime::data::preferences::ModelConfigManager::ModelConfigManager;
use operit_runtime::data::repository::ChatHistoryManager::ChatHistoryManager;
use operit_runtime::util::stream::Stream::Stream;

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

fn list_chats(
    application: &mut OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    for chat in application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .chatHistoriesFlow()
        .value()
    {
        output.push_stdout_line(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            chat.id,
            chat.title,
            chat.createdAt,
            chat.updatedAt,
            "",
            chat.inputTokens,
            chat.outputTokens,
            chat.characterCardName.clone().unwrap_or_default(),
            chat.characterGroupId.clone().unwrap_or_default(),
            chat.locked,
            chat.pinned
        ));
    }
    Ok(())
}

fn show_chat(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat show <chat-id> [--runtime]".to_string())?
        .clone();
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    core.switchChat(chatId.clone());
    let chat = core
        .chatHistoriesFlow()
        .value()
        .into_iter()
        .find(|chat| chat.id == chatId)
        .ok_or_else(|| format!("chat not found: {chatId}"))?;
    print_chat_history_header(&chat, output);
    for message in core.chatHistoryFlow().value() {
        print_chat_message(&message, output);
    }
    Ok(())
}

fn delete_chat(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat delete <chat-id>".to_string())?
        .clone();
    application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .deleteChatHistory(chatId.clone());
    output.push_stdout_line(format!("chat deleted: {chatId}"));
    Ok(())
}

fn delete_chat_message(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let index = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat delete-message <index>".to_string())?
        .parse::<usize>()
        .map_err(|error| error.to_string())?;
    application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .deleteMessage(index);
    output.push_stdout_line(format!("message deleted: {index}"));
    Ok(())
}

fn clear_current_chat(
    application: &mut OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .clearCurrentChat();
    output.push_stdout_line("current chat cleared");
    Ok(())
}

fn rollback_chat(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let index = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat rollback <message-index>".to_string())?
        .parse::<usize>()
        .map_err(|error| error.to_string())?;
    let rolledBack = application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .rollbackToMessage(index);
    if rolledBack {
        output.push_stdout_line(format!("rolled back to message: {index}"));
    } else {
        output.push_stdout_line("rollback skipped: message must exist and be a user message");
    }
    Ok(())
}

fn create_chat_branch(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let upToMessageTimestamp = parse_branch_args(args)?;
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    core.createBranch(upToMessageTimestamp);
    let chatId = core
        .currentChatIdFlow()
        .value()
        .ok_or_else(|| "core did not create branch".to_string())?;
    output.push_stdout_line(chatId);
    Ok(())
}

fn list_chat_branches(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    let parentChatId = match args.get(0) {
        Some(chatId) => chatId.clone(),
        None => core
            .currentChatIdFlow()
            .value()
            .ok_or_else(|| "usage: operit2 chat branches [parent-chat-id]".to_string())?,
    };
    for chat in core.getBranches(parentChatId) {
        output.push_stdout_line(format!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            chat.id, chat.title, chat.createdAt, chat.updatedAt, chat.locked, chat.pinned
        ));
    }
    Ok(())
}

fn update_chat_locked(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let (chatId, locked) = parse_chat_bool_update_args(args, "lock")?;
    application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .updateChatLocked(chatId.clone(), locked);
    output.push_stdout_line(format!("chat locked={locked}: {chatId}"));
    Ok(())
}

fn update_chat_pinned(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let (chatId, pinned) = parse_chat_bool_update_args(args, "pin")?;
    application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .updateChatPinned(chatId.clone(), pinned);
    output.push_stdout_line(format!("chat pinned={pinned}: {chatId}"));
    Ok(())
}

fn show_current_chat(
    application: &mut OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    match application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .currentChatIdFlow()
        .value()
    {
        Some(chatId) => output.push_stdout_line(chatId),
        None => output.push_stdout_line(""),
    }
    Ok(())
}

fn switch_chat_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat switch <chat-id>".to_string())?
        .clone();
    application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .switchChat(chatId.clone());
    output.push_stdout_line(format!("current chat: {chatId}"));
    Ok(())
}

fn show_chat_stats(output: &mut CoreCommandOutput) -> Result<(), String> {
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    output.push_stdout_line(format!(
        "totalChats={}",
        manager
            .getTotalChatCount()
            .map_err(|error| error.to_string())?
    ));
    output.push_stdout_line(format!(
        "totalMessages={}",
        manager
            .getTotalMessageCount()
            .map_err(|error| error.to_string())?
    ));
    for stats in manager
        .characterCardStatsFlow()
        .map_err(|error| error.to_string())?
    {
        output.push_stdout_line(format!(
            "characterCard\t{}\t{}\t{}",
            stats.characterCardName.clone().unwrap_or_default(),
            stats.chatCount,
            stats.messageCount
        ));
    }
    for stats in manager
        .characterGroupStatsFlow()
        .map_err(|error| error.to_string())?
    {
        output.push_stdout_line(format!(
            "characterGroup\t{}\t{}\t{}",
            stats.characterGroupId.clone().unwrap_or_default(),
            stats.chatCount,
            stats.messageCount
        ));
    }
    Ok(())
}

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
    application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .updateChatCharacterCard(chatId.clone(), Some(characterCardName));
    output.push_stdout_line(format!("chat character binding updated: {chatId}"));
    Ok(())
}

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
    application
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN)
        .updateChatCharacterGroup(chatId.clone(), Some(characterGroupId));
    output.push_stdout_line(format!("chat group binding updated: {chatId}"));
    Ok(())
}

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
        .updateChatGroup(chatId.clone(), Some(groupName))
        .map_err(|error| error.to_string())?;
    output.push_stdout_line(format!("chat group updated: {chatId}"));
    Ok(())
}

fn create_chat(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let (characterCardName, characterGroupId, group) = parse_chat_new_args(args)?;
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    core.createNewChat(characterCardName, group, true, true, characterGroupId);
    let chatId = core
        .currentChatIdFlow()
        .value()
        .ok_or_else(|| "core did not create chat".to_string())?;
    output.push_stdout_line(chatId);
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
    let parsed = parse_bool_arg(value).ok_or(usage)?;
    Ok((chatId, parsed))
}

fn parse_bool_arg(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" | "lock" | "locked" | "pin" | "pinned" => Some(true),
        "false" | "0" | "no" | "off" | "unlock" | "unlocked" | "unpin" | "unpinned" => Some(false),
        _ => None,
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

fn send_chat_message_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let sendArgs = parse_chat_send_args(args)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
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
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    let currentChatId = core
        .currentChatIdFlow()
        .value()
        .ok_or_else(|| "core has no active chat after send".to_string())?;
    let mut aiMessage = core
        .chatHistoryFlow()
        .value()
        .iter()
        .rev()
        .find(|message| message.sender == "ai" && message.timestamp > beforeLastAiTimestamp)
        .ok_or_else(|| "core did not produce ai message for current turn".to_string())?
        .clone();
    if let Some(mut stream) = aiMessage.contentStream.clone() {
        let mut content = String::new();
        stream.collect(&mut |chunk| {
            content.push_str(&chunk);
        });
        aiMessage.content = content;
        aiMessage.contentStream = None;
    }
    aiMessage = wait_for_committed_ai_message(
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
    let modelConfigManager = ModelConfigManager::default();
    let functionalConfigManager = FunctionalConfigManager::default();
    modelConfigManager
        .initializeIfNeeded()
        .map_err(|error| error.to_string())?;
    functionalConfigManager
        .initializeIfNeeded()
        .map_err(|error| error.to_string())?;
    let chatBinding = functionalConfigManager
        .getModelBindingForFunction(FunctionType::CHAT)
        .map_err(|error| error.to_string())?;
    let turnOptions = ChatTurnOptions::default();
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    core.enhancedAiService = Some(EnhancedAIService::new(ConversationService));
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
            .chatHistoryFlow()
            .value()
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
    core.updateUserMessage(sendArgs.message);
    let beforeLastAiTimestamp = core
        .chatHistoryFlow()
        .value()
        .iter()
        .filter(|message| message.sender == "ai")
        .map(|message| message.timestamp)
        .max()
        .unwrap_or(0);
    core.sendUserMessage(
        PromptFunctionType::CHAT,
        None,
        None,
        None,
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

fn wait_for_committed_ai_message(
    application: &mut OperitApplication,
    chatId: &str,
    timestamp: i64,
    timeout: Duration,
) -> Result<ChatMessage, String> {
    let startedAt = Instant::now();
    loop {
        let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
        if let Some(message) = core.chatHistoryFlow().value().into_iter().find(|message| {
            message.sender == "ai"
                && message.timestamp == timestamp
                && message.contentStream.is_none()
                && message.completedAt > 0
        }) {
            return Ok(message);
        }
        let stateByChatId = core.inputProcessingStateByChatIdFlow().value();
        if let Some(InputProcessingState::Error { message }) = stateByChatId.get(chatId) {
            return Err(message.clone());
        }
        if startedAt.elapsed() >= timeout {
            return Err(format!(
                "timed out waiting for committed ai message: chat={chatId} timestamp={timestamp}"
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn print_chat_send_result(result: &ChatSendResult, output: &mut CoreCommandOutput) {
    output.push_stdout(&result.aiMessage.content);
    output.push_stdout_line("");
    output.push_stderr_line(format!(
        "chat={} provider={} modelName={} inputTokens={} cachedInputTokens={} outputTokens={}",
        result.chatId,
        result.aiMessage.provider,
        result.aiMessage.modelName,
        result.aiMessage.inputTokens,
        result.aiMessage.cachedInputTokens,
        result.aiMessage.outputTokens
    ));
}

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

fn print_chat_history_header(chat: &ChatHistory, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!("id={}", chat.id));
    output.push_stdout_line(format!("title={}", chat.title));
    output.push_stdout_line(format!("createdAt={}", chat.createdAt));
    output.push_stdout_line(format!("updatedAt={}", chat.updatedAt));
    output.push_stdout_line(format!("inputTokens={}", chat.inputTokens));
    output.push_stdout_line(format!("outputTokens={}", chat.outputTokens));
    output.push_stdout_line(format!("currentWindowSize={}", chat.currentWindowSize));
    output.push_stdout_line(format!("group={}", chat.group.clone().unwrap_or_default()));
    output.push_stdout_line(format!("displayOrder={}", chat.displayOrder));
    output.push_stdout_line(format!(
        "workspace={}",
        chat.workspace.clone().unwrap_or_default()
    ));
    output.push_stdout_line(format!(
        "parentChatId={}",
        chat.parentChatId.clone().unwrap_or_default()
    ));
    output.push_stdout_line(format!(
        "characterCardName={}",
        chat.characterCardName.clone().unwrap_or_default()
    ));
    output.push_stdout_line(format!(
        "characterGroupId={}",
        chat.characterGroupId.clone().unwrap_or_default()
    ));
    output.push_stdout_line(format!("locked={}", chat.locked));
    output.push_stdout_line(format!("pinned={}", chat.pinned));
}

fn print_chat_message(message: &ChatMessage, output: &mut CoreCommandOutput) {
    output.push_stdout_line("--- message ---");
    output.push_stdout_line(format!("sender={}", message.sender));
    output.push_stdout_line(format!("timestamp={}", message.timestamp));
    output.push_stdout_line(format!("roleName={}", message.roleName));
    output.push_stdout_line(format!(
        "selectedVariantIndex={}",
        message.selectedVariantIndex
    ));
    output.push_stdout_line(format!("variantCount={}", message.variantCount));
    output.push_stdout_line(format!("provider={}", message.provider));
    output.push_stdout_line(format!("modelName={}", message.modelName));
    output.push_stdout_line(format!("inputTokens={}", message.inputTokens));
    output.push_stdout_line(format!("cachedInputTokens={}", message.cachedInputTokens));
    output.push_stdout_line(format!("outputTokens={}", message.outputTokens));
    output.push_stdout_line(format!("sentAt={}", message.sentAt));
    output.push_stdout_line(format!("waitDurationMs={}", message.waitDurationMs));
    output.push_stdout_line(format!("outputDurationMs={}", message.outputDurationMs));
    output.push_stdout_line(format!("completedAt={}", message.completedAt));
    output.push_stdout_line(format!("displayMode={:?}", message.displayMode));
    output.push_stdout_line(format!("isFavorite={}", message.isFavorite));
    output.push_stdout_line(format!("content={}", message.content));
}

fn nonBlankString(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn print_chat_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 chat new [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    output.push_stdout_line("operit2 chat list");
    output.push_stdout_line("operit2 chat show <chat-id> [--runtime]");
    output.push_stdout_line("operit2 chat current");
    output.push_stdout_line("operit2 chat switch <chat-id>");
    output.push_stdout_line("operit2 chat delete <chat-id>");
    output.push_stdout_line("operit2 chat delete-message <index>");
    output.push_stdout_line("operit2 chat clear");
    output.push_stdout_line("operit2 chat rollback <message-index>");
    output.push_stdout_line("operit2 chat branch [--up-to <message-timestamp>]");
    output.push_stdout_line("operit2 chat branches [parent-chat-id]");
    output.push_stdout_line("operit2 chat lock <chat-id> <true|false>");
    output.push_stdout_line("operit2 chat pin <chat-id> <true|false>");
    output.push_stdout_line("operit2 chat stats");
    output.push_stdout_line("operit2 chat bind-character <chat-id> <character-card-name>");
    output.push_stdout_line("operit2 chat bind-group <chat-id> <character-group-id>");
    output.push_stdout_line("operit2 chat set-group <chat-id> <group-name>");
    output.push_stdout_line("operit2 chat send [--chat <chat-id>] <message>");
}
