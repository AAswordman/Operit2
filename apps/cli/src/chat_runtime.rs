use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::bootstrap::create_cli_application;
use crate::core_proxy::{local_cli_core, CliCore};
use operit_runtime::api::chat::enhance::ConversationService::ConversationService;
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::api::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::data::model::AttachmentInfo::AttachmentInfo;
use operit_runtime::data::model::ChatMessage::ChatMessage;
use operit_runtime::data::model::ChatTurnOptions::ChatTurnOptions;
use operit_runtime::data::model::FunctionType::FunctionType;
use operit_runtime::data::model::InputProcessingState::InputProcessingState;
use operit_runtime::data::model::PromptFunctionType::PromptFunctionType;
use operit_runtime::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use operit_runtime::data::preferences::ModelConfigManager::ModelConfigManager;
use operit_runtime::data::repository::ChatHistoryManager::ChatHistoryManager;
use operit_runtime::services::core::MessageCoordinationDelegate::MessageCoordinationDelegate;
use operit_runtime::util::stream::Stream::Stream;

pub(super) async fn run_chat_shell_command_with_core(
    core: &mut CliCore,
    args: &[String],
) -> Result<(), String> {
    run_shell_command_with_core(core, args).await
}

async fn list_chats_with_core(core: &mut CliCore) -> Result<(), String> {
    for chat in core
        .chat_runtime_holder_main()
        .chatHistoriesFlowSnapshot()
        .await
        .map_err(|error| error.to_string())?
    {
        println!(
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
        );
    }
    Ok(())
}

async fn show_chat_with_core(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat show <chat-id> [--runtime]".to_string())?;
    core.chat_runtime_holder_main()
        .switchChat(chatId.clone())
        .await
        .map_err(|error| error.to_string())?;
    let chat = core
        .chat_runtime_holder_main()
        .chatHistoriesFlowSnapshot()
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|chat| chat.id == *chatId)
        .ok_or_else(|| format!("chat not found: {chatId}"))?;
    print_chat_history_header(&chat);
    for message in core
        .chat_runtime_holder_main()
        .chatHistoryFlowSnapshot()
        .await
        .map_err(|error| error.to_string())?
    {
        print_chat_message(&message);
    }
    Ok(())
}

async fn delete_chat_with_core(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat delete <chat-id>".to_string())?
        .clone();
    core.chat_runtime_holder_main()
        .deleteChatHistory(chatId.clone())
        .await
        .map_err(|error| error.to_string())?;
    println!("chat deleted: {chatId}");
    Ok(())
}

async fn delete_chat_message_with_core(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    let index = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat delete-message <index>".to_string())?
        .parse::<usize>()
        .map_err(|error| error.to_string())?;
    core.chat_runtime_holder_main()
        .deleteMessage(index)
        .await
        .map_err(|error| error.to_string())?;
    println!("message deleted: {index}");
    Ok(())
}

async fn clear_current_chat_with_core(core: &mut CliCore) -> Result<(), String> {
    core.chat_runtime_holder_main()
        .clearCurrentChat()
        .await
        .map_err(|error| error.to_string())?;
    println!("current chat cleared");
    Ok(())
}

async fn rollback_chat_with_core(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    let index = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat rollback <message-index>".to_string())?
        .parse::<usize>()
        .map_err(|error| error.to_string())?;
    let rolledBack = core
        .chat_runtime_holder_main()
        .rollbackToMessage(index)
        .await
        .map_err(|error| error.to_string())?;
    if rolledBack {
        println!("rolled back to message: {index}");
    } else {
        println!("rollback skipped: message must exist and be a user message");
    }
    Ok(())
}

async fn create_chat_branch_with_core(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    let upToMessageTimestamp = parse_branch_args(args)?;
    core.chat_runtime_holder_main()
        .createBranch(upToMessageTimestamp)
        .await
        .map_err(|error| error.to_string())?;
    let chatId = core
        .chat_runtime_holder_main()
        .currentChatIdFlowSnapshot()
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "core did not create branch".to_string())?;
    println!("{chatId}");
    Ok(())
}

async fn list_chat_branches_with_core(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    let parentChatId = match args.get(0) {
        Some(chatId) => chatId.clone(),
        None => core
            .chat_runtime_holder_main()
            .currentChatIdFlowSnapshot()
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "usage: operit2 chat branches [parent-chat-id]".to_string())?,
    };
    for chat in core
        .chat_runtime_holder_main()
        .getBranches(parentChatId)
        .await
        .map_err(|error| error.to_string())?
    {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            chat.id, chat.title, chat.createdAt, chat.updatedAt, chat.locked, chat.pinned
        );
    }
    Ok(())
}

async fn update_chat_locked_with_core(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    let (chatId, locked) = parse_chat_bool_update_args(args, "lock")?;
    core.chat_runtime_holder_main()
        .updateChatLocked(chatId.clone(), locked)
        .await
        .map_err(|error| error.to_string())?;
    println!("chat locked={locked}: {chatId}");
    Ok(())
}

async fn update_chat_pinned_with_core(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    let (chatId, pinned) = parse_chat_bool_update_args(args, "pin")?;
    core.chat_runtime_holder_main()
        .updateChatPinned(chatId.clone(), pinned)
        .await
        .map_err(|error| error.to_string())?;
    println!("chat pinned={pinned}: {chatId}");
    Ok(())
}

async fn show_current_chat_with_core(core: &mut CliCore) -> Result<(), String> {
    match core
        .chat_runtime_holder_main()
        .currentChatIdFlowSnapshot()
        .await
        .map_err(|error| error.to_string())?
    {
        Some(chatId) => println!("{chatId}"),
        None => println!(),
    }
    Ok(())
}

async fn switch_chat_command_with_core(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat switch <chat-id>".to_string())?
        .clone();
    core.chat_runtime_holder_main()
        .switchChat(chatId.clone())
        .await
        .map_err(|error| error.to_string())?;
    println!("current chat: {chatId}");
    Ok(())
}

fn show_chat_stats() -> Result<(), String> {
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    println!(
        "totalChats={}",
        manager
            .getTotalChatCount()
            .map_err(|error| error.to_string())?
    );
    println!(
        "totalMessages={}",
        manager
            .getTotalMessageCount()
            .map_err(|error| error.to_string())?
    );
    for stats in manager
        .characterCardStatsFlow()
        .map_err(|error| error.to_string())?
    {
        println!(
            "characterCard\t{}\t{}\t{}",
            stats.characterCardName.clone().unwrap_or_default(),
            stats.chatCount,
            stats.messageCount
        );
    }
    for stats in manager
        .characterGroupStatsFlow()
        .map_err(|error| error.to_string())?
    {
        println!(
            "characterGroup\t{}\t{}\t{}",
            stats.characterGroupId.clone().unwrap_or_default(),
            stats.chatCount,
            stats.messageCount
        );
    }
    Ok(())
}

fn bind_chat_character(args: &[String]) -> Result<(), String> {
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
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    manager
        .updateChatCharacterBinding(chatId.clone(), Some(characterCardName), None)
        .map_err(|error| error.to_string())?;
    println!("chat character binding updated: {chatId}");
    Ok(())
}

fn bind_chat_group_card(args: &[String]) -> Result<(), String> {
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
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    manager
        .updateChatCharacterBinding(chatId.clone(), None, Some(characterGroupId))
        .map_err(|error| error.to_string())?;
    println!("chat group binding updated: {chatId}");
    Ok(())
}

fn set_chat_group(args: &[String]) -> Result<(), String> {
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
    println!("chat group updated: {chatId}");
    Ok(())
}

async fn create_chat_with_core(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    let (characterCardName, characterGroupId, group) = parse_chat_new_args(args)?;
    core.chat_runtime_holder_main()
        .createNewChat(characterCardName, group, true, true, characterGroupId)
        .await
        .map_err(|error| error.to_string())?;
    let chatId = core
        .chat_runtime_holder_main()
        .currentChatIdFlowSnapshot()
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "core did not create chat".to_string())?;
    println!("{chatId}");
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ChatSendArgs {
    pub(crate) chatId: Option<String>,
    pub(crate) message: String,
    pub(crate) attachmentPaths: Vec<String>,
    pub(crate) replyToTimestamp: Option<i64>,
}

#[derive(Clone, Debug)]
pub(crate) struct ShellArgs {
    pub(crate) chatId: Option<String>,
    pub(crate) resume: bool,
    pub(crate) characterCardName: Option<String>,
    pub(crate) characterGroupId: Option<String>,
    pub(crate) group: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ChatSendResult {
    chatId: String,
    aiMessage: ChatMessage,
}

pub(crate) enum ShellLoopControl {
    Continue,
    Exit,
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

pub(crate) fn parse_shell_args(args: &[String]) -> Result<ShellArgs, String> {
    let usage = "usage: operit2 [--chat <chat-id>] [--resume] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]";
    let mut shellArgs = ShellArgs {
        chatId: None,
        resume: false,
        characterCardName: None,
        characterGroupId: None,
        group: None,
    };
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--chat" => {
                index += 1;
                shellArgs.chatId = Some(args.get(index).ok_or_else(|| usage.to_string())?.clone());
            }
            "--resume" => {
                shellArgs.resume = true;
            }
            "--character" => {
                index += 1;
                shellArgs.characterCardName = args.get(index).cloned().and_then(nonBlankString);
            }
            "--group-card" => {
                index += 1;
                shellArgs.characterGroupId = args.get(index).cloned().and_then(nonBlankString);
            }
            "--group" => {
                index += 1;
                shellArgs.group = args.get(index).cloned().and_then(nonBlankString);
            }
            _ => return Err(usage.to_string()),
        }
        index += 1;
    }
    if shellArgs.chatId.is_some()
        && (shellArgs.resume
            || shellArgs.characterCardName.is_some()
            || shellArgs.characterGroupId.is_some()
            || shellArgs.group.is_some())
    {
        return Err(usage.to_string());
    }
    if shellArgs.resume
        && (shellArgs.characterCardName.is_some()
            || shellArgs.characterGroupId.is_some()
            || shellArgs.group.is_some())
    {
        return Err(usage.to_string());
    }
    Ok(shellArgs)
}

pub(crate) async fn run_shell_command(args: &[String]) -> Result<(), String> {
    let shellArgs = parse_shell_args(args)?;
    let mut application = create_cli_application();
    application.onCreate()?;
    let _externalRuntimeEventRegistration =
        operit_runtime::core::application::ExternalRuntimeEventSupport::startExternalRuntimeEventSupport(
            application.applicationContext.clone(),
            "cli-shell",
        )?;
    let mut queuedAttachmentPaths = Vec::<String>::new();
    let initialChatId = initialize_shell_chat(&mut application, &shellArgs)?;
    println!("interactive shell ready");
    println!("chat={initialChatId}");
    println!("type /help for commands");
    loop {
        let currentChatId = current_shell_chat_id(&mut application)?;
        print!("operit2[{}]> ", short_chat_label(&currentChatId));
        io::stdout().flush().map_err(|error| error.to_string())?;
        let mut line = String::new();
        let readBytes = io::stdin()
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if readBytes == 0 {
            println!();
            break;
        }
        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input.starts_with('/') {
            match handle_shell_command(input, &mut application, &mut queuedAttachmentPaths).await? {
                ShellLoopControl::Continue => continue,
                ShellLoopControl::Exit => break,
            }
        } else {
            let sendArgs = ChatSendArgs {
                chatId: Some(currentChatId),
                message: input.to_string(),
                attachmentPaths: queuedAttachmentPaths.clone(),
                replyToTimestamp: None,
            };
            match send_chat_message_with_application(&mut application, sendArgs).await {
                Ok(result) => {
                    print_chat_send_result(&result);
                    queuedAttachmentPaths.clear();
                }
                Err(error) => eprintln!("{error}"),
            }
        }
    }
    Ok(())
}

async fn run_shell_command_with_core(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    let shellArgs = parse_shell_args(args)?;
    let mut queuedAttachmentPaths = Vec::<String>::new();
    let initialChatId = initialize_shell_chat_with_core(core, &shellArgs).await?;
    println!("interactive shell ready");
    println!("chat={initialChatId}");
    println!("type /help for commands");
    loop {
        let currentChatId = current_shell_chat_id_with_core(core).await?;
        print!("operit2[{}]> ", short_chat_label(&currentChatId));
        io::stdout().flush().map_err(|error| error.to_string())?;
        let mut line = String::new();
        let readBytes = io::stdin()
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if readBytes == 0 {
            println!();
            break;
        }
        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input.starts_with('/') {
            match handle_shell_command_with_core(input, core, &mut queuedAttachmentPaths).await? {
                ShellLoopControl::Continue => continue,
                ShellLoopControl::Exit => break,
            }
        } else {
            let sendArgs = ChatSendArgs {
                chatId: Some(currentChatId),
                message: input.to_string(),
                attachmentPaths: queuedAttachmentPaths.clone(),
                replyToTimestamp: None,
            };
            match send_chat_message_with_core_result(core, sendArgs).await {
                Ok(result) => {
                    print_chat_send_result(&result);
                    queuedAttachmentPaths.clear();
                }
                Err(error) => eprintln!("{error}"),
            }
        }
    }
    Ok(())
}

async fn initialize_shell_chat_with_core(
    core: &mut CliCore,
    shellArgs: &ShellArgs,
) -> Result<String, String> {
    core.preferences_model_config_manager()
        .initializeIfNeeded()
        .await
        .map_err(|error| error.to_string())?;
    core.preferences_functional_config_manager()
        .initializeIfNeeded()
        .await
        .map_err(|error| error.to_string())?;
    if let Some(chatId) = shellArgs.chatId.clone() {
        core.chat_runtime_holder_main()
            .switchChat(chatId.clone())
            .await
            .map_err(|error| error.to_string())?;
        Ok(chatId)
    } else if shellArgs.resume {
        let chatId = latest_chat_id_with_core(core).await?;
        core.chat_runtime_holder_main()
            .switchChat(chatId.clone())
            .await
            .map_err(|error| error.to_string())?;
        Ok(chatId)
    } else {
        core.chat_runtime_holder_main()
            .createNewChat(
                shellArgs.characterCardName.clone(),
                shellArgs.group.clone(),
                true,
                true,
                shellArgs.characterGroupId.clone(),
            )
            .await
            .map_err(|error| error.to_string())?;
        core.chat_runtime_holder_main()
            .currentChatIdFlowSnapshot()
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "core did not create chat".to_string())
    }
}

pub(crate) fn initialize_shell_chat(
    application: &mut OperitApplication,
    shellArgs: &ShellArgs,
) -> Result<String, String> {
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    core.enhancedAiService = Some(EnhancedAIService::new(ConversationService));
    if let Some(chatId) = shellArgs.chatId.clone() {
        ensure_chat_exists(&chatId)?;
        core.switchChat(chatId.clone());
        Ok(chatId)
    } else if shellArgs.resume {
        let chatId = latest_chat_id()?;
        core.switchChat(chatId.clone());
        Ok(chatId)
    } else {
        core.createNewChat(
            shellArgs.characterCardName.clone(),
            shellArgs.group.clone(),
            true,
            true,
            shellArgs.characterGroupId.clone(),
        );
        core.currentChatIdFlow()
            .value()
            .ok_or_else(|| "core did not create chat".to_string())
    }
}

fn latest_chat_id() -> Result<String, String> {
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    manager
        .chatHistoriesFlow()
        .map_err(|error| error.to_string())?
        .into_iter()
        .max_by(|left, right| {
            let leftUpdated = left
                .updatedAt
                .parse::<i64>()
                .expect("chat.updatedAt must be epoch millis");
            let rightUpdated = right
                .updatedAt
                .parse::<i64>()
                .expect("chat.updatedAt must be epoch millis");
            leftUpdated
                .cmp(&rightUpdated)
                .then_with(|| right.displayOrder.cmp(&left.displayOrder))
        })
        .map(|chat| chat.id)
        .ok_or_else(|| "no previous chat to resume".to_string())
}

async fn latest_chat_id_with_core(core: &mut CliCore) -> Result<String, String> {
    core.chat_runtime_holder_main()
        .chatHistoriesFlowSnapshot()
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .max_by(|left, right| {
            let leftUpdated = left
                .updatedAt
                .parse::<i64>()
                .expect("chat.updatedAt must be epoch millis");
            let rightUpdated = right
                .updatedAt
                .parse::<i64>()
                .expect("chat.updatedAt must be epoch millis");
            leftUpdated
                .cmp(&rightUpdated)
                .then_with(|| right.displayOrder.cmp(&left.displayOrder))
        })
        .map(|chat| chat.id)
        .ok_or_else(|| "no previous chat to resume".to_string())
}

pub(crate) fn ensure_chat_exists(chatId: &str) -> Result<(), String> {
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    let exists = manager
        .chatHistoriesFlow()
        .map_err(|error| error.to_string())?
        .iter()
        .any(|chat| chat.id == chatId);
    if exists {
        Ok(())
    } else {
        Err(format!("chat not found: {chatId}"))
    }
}

pub(crate) fn current_shell_chat_id(application: &mut OperitApplication) -> Result<String, String> {
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    core.currentChatIdFlow()
        .value()
        .ok_or_else(|| "no active chat in shell".to_string())
}

async fn current_shell_chat_id_with_core(core: &mut CliCore) -> Result<String, String> {
    core.chat_runtime_holder_main()
        .currentChatIdFlowSnapshot()
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "no active chat in shell".to_string())
}

async fn handle_shell_command(
    input: &str,
    application: &mut OperitApplication,
    queuedAttachmentPaths: &mut Vec<String>,
) -> Result<ShellLoopControl, String> {
    let parts = split_shell_command_line(input)?;
    if parts.is_empty() {
        return Ok(ShellLoopControl::Continue);
    }
    let command = parts[0].trim_start_matches('/');
    let args = &parts[1..];
    match command {
        "help" => {
            print_shell_usage();
        }
        "exit" | "quit" => {
            return Ok(ShellLoopControl::Exit);
        }
        "chat" | "current" => {
            println!("{}", current_shell_chat_id(application)?);
        }
        "new" => {
            let shellArgs = parse_shell_args(args)?;
            if shellArgs.chatId.is_some() {
                return Err("shell /new does not accept --chat".to_string());
            }
            let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
            core.createNewChat(
                shellArgs.characterCardName,
                shellArgs.group,
                true,
                true,
                shellArgs.characterGroupId,
            );
            let chatId = core
                .currentChatIdFlow()
                .value()
                .ok_or_else(|| "core did not create chat".to_string())?;
            println!("chat={chatId}");
        }
        "switch" => {
            let chatId = args
                .get(0)
                .ok_or_else(|| "usage: /switch <chat-id>".to_string())?
                .clone();
            ensure_chat_exists(&chatId)?;
            let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
            core.switchChat(chatId.clone());
            println!("chat={chatId}");
        }
        "resume" => {
            let currentChatId = current_shell_chat_id(application)?;
            let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
            let target = manager
                .chatHistoriesFlow()
                .map_err(|error| error.to_string())?
                .into_iter()
                .filter(|chat| chat.id != currentChatId)
                .max_by(|left, right| {
                    left.updatedAt
                        .parse::<i64>()
                        .expect("chat.updatedAt must be epoch millis")
                        .cmp(
                            &right
                                .updatedAt
                                .parse::<i64>()
                                .expect("chat.updatedAt must be epoch millis"),
                        )
                        .then_with(|| right.displayOrder.cmp(&left.displayOrder))
                });
            let Some(target) = target else {
                println!("no previous chat to resume");
                return Ok(ShellLoopControl::Continue);
            };
            let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
            core.switchChat(target.id.clone());
            println!("chat={}", target.id);
        }
        "show" => {
            let chatId = current_shell_chat_id(application)?;
            let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
            let chat = manager
                .chatHistoriesFlow()
                .map_err(|error| error.to_string())?
                .into_iter()
                .find(|chat| chat.id == chatId)
                .ok_or_else(|| format!("chat not found: {chatId}"))?;
            print_chat_history_header(&chat);
            let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
            for message in core.chatHistoryFlow().value() {
                print_chat_message(&message);
            }
        }
        "attach" => {
            let path = args
                .get(0)
                .ok_or_else(|| "usage: /attach <path>".to_string())?
                .clone();
            queuedAttachmentPaths.push(path.clone());
            println!("queued attachment: {path}");
        }
        "attachments" => {
            if queuedAttachmentPaths.is_empty() {
                println!("attachments=none");
            } else {
                for path in queuedAttachmentPaths.iter() {
                    println!("{path}");
                }
            }
        }
        "clear-attachments" => {
            queuedAttachmentPaths.clear();
            println!("attachments cleared");
        }
        "send" => {
            let message = args.join(" ");
            if message.trim().is_empty() {
                return Err("usage: /send <message>".to_string());
            }
            let chatId = current_shell_chat_id(application)?;
            let sendArgs = ChatSendArgs {
                chatId: Some(chatId),
                message,
                attachmentPaths: queuedAttachmentPaths.clone(),
                replyToTimestamp: None,
            };
            match send_chat_message_with_application(application, sendArgs).await {
                Ok(result) => {
                    print_chat_send_result(&result);
                    queuedAttachmentPaths.clear();
                }
                Err(error) => eprintln!("{error}"),
            }
        }
        _ => {
            return Err(format!("unknown shell command: /{command}"));
        }
    }
    Ok(ShellLoopControl::Continue)
}

async fn handle_shell_command_with_core(
    input: &str,
    core: &mut CliCore,
    queuedAttachmentPaths: &mut Vec<String>,
) -> Result<ShellLoopControl, String> {
    let parts = split_shell_command_line(input)?;
    if parts.is_empty() {
        return Ok(ShellLoopControl::Continue);
    }
    let command = parts[0].trim_start_matches('/');
    let args = &parts[1..];
    match command {
        "help" => {
            print_shell_usage();
        }
        "exit" | "quit" => {
            return Ok(ShellLoopControl::Exit);
        }
        "chat" | "current" => {
            println!("{}", current_shell_chat_id_with_core(core).await?);
        }
        "new" => {
            let shellArgs = parse_shell_args(args)?;
            if shellArgs.chatId.is_some() {
                return Err("shell /new does not accept --chat".to_string());
            }
            core.chat_runtime_holder_main()
                .createNewChat(
                    shellArgs.characterCardName,
                    shellArgs.group,
                    true,
                    true,
                    shellArgs.characterGroupId,
                )
                .await
                .map_err(|error| error.to_string())?;
            let chatId = current_shell_chat_id_with_core(core).await?;
            println!("chat={chatId}");
        }
        "switch" => {
            let chatId = args
                .get(0)
                .ok_or_else(|| "usage: /switch <chat-id>".to_string())?
                .clone();
            core.chat_runtime_holder_main()
                .switchChat(chatId.clone())
                .await
                .map_err(|error| error.to_string())?;
            println!("chat={chatId}");
        }
        "resume" => {
            let currentChatId = current_shell_chat_id_with_core(core).await?;
            let target = core
                .chat_runtime_holder_main()
                .chatHistoriesFlowSnapshot()
                .await
                .map_err(|error| error.to_string())?
                .into_iter()
                .filter(|chat| chat.id != currentChatId)
                .max_by(|left, right| {
                    left.updatedAt
                        .parse::<i64>()
                        .expect("chat.updatedAt must be epoch millis")
                        .cmp(
                            &right
                                .updatedAt
                                .parse::<i64>()
                                .expect("chat.updatedAt must be epoch millis"),
                        )
                        .then_with(|| right.displayOrder.cmp(&left.displayOrder))
                });
            let Some(target) = target else {
                println!("no previous chat to resume");
                return Ok(ShellLoopControl::Continue);
            };
            core.chat_runtime_holder_main()
                .switchChat(target.id.clone())
                .await
                .map_err(|error| error.to_string())?;
            println!("chat={}", target.id);
        }
        "show" => {
            let chatId = current_shell_chat_id_with_core(core).await?;
            show_chat_with_core(core, &[chatId]).await?;
        }
        "attach" => {
            let path = args
                .get(0)
                .ok_or_else(|| "usage: /attach <path>".to_string())?
                .clone();
            queuedAttachmentPaths.push(path.clone());
            println!("queued attachment: {path}");
        }
        "attachments" => {
            if queuedAttachmentPaths.is_empty() {
                println!("attachments=none");
            } else {
                for path in queuedAttachmentPaths.iter() {
                    println!("{path}");
                }
            }
        }
        "clear-attachments" => {
            queuedAttachmentPaths.clear();
            println!("attachments cleared");
        }
        "send" => {
            let message = args.join(" ");
            if message.trim().is_empty() {
                return Err("usage: /send <message>".to_string());
            }
            let chatId = current_shell_chat_id_with_core(core).await?;
            let sendArgs = ChatSendArgs {
                chatId: Some(chatId),
                message,
                attachmentPaths: queuedAttachmentPaths.clone(),
                replyToTimestamp: None,
            };
            match send_chat_message_with_core_result(core, sendArgs).await {
                Ok(result) => {
                    print_chat_send_result(&result);
                    queuedAttachmentPaths.clear();
                }
                Err(error) => eprintln!("{error}"),
            }
        }
        _ => {
            return Err(format!("unknown shell command: /{command}"));
        }
    }
    Ok(ShellLoopControl::Continue)
}

fn split_shell_command_line(input: &str) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote = None::<char>;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match quote {
            Some(activeQuote) => {
                if ch == activeQuote {
                    quote = None;
                } else if ch == '\\' && activeQuote == '"' {
                    match chars.next() {
                        Some(next) => current.push(next),
                        None => current.push('\\'),
                    }
                } else {
                    current.push(ch);
                }
            }
            None => match ch {
                '"' | '\'' => quote = Some(ch),
                '\\' => match chars.next() {
                    Some(next) => current.push(next),
                    None => current.push('\\'),
                },
                ch if ch.is_whitespace() => {
                    if !current.is_empty() {
                        parts.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(ch),
            },
        }
    }
    if quote.is_some() {
        return Err("unterminated quote".to_string());
    }
    if !current.is_empty() {
        parts.push(current);
    }
    Ok(parts)
}

fn short_chat_label(chatId: &str) -> String {
    chatId.chars().take(8).collect()
}

fn print_shell_usage() {
    println!("/help");
    println!("/exit");
    println!("/quit");
    println!("/chat");
    println!("/new [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("/switch <chat-id>");
    println!("/resume");
    println!("/show");
    println!("/attach <path>");
    println!("/attachments");
    println!("/clear-attachments");
    println!("/send <message>");
}

pub(crate) async fn begin_chat_message_with_application(
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
    let aiMessage = core
        .chatHistoryFlow()
        .value()
        .iter()
        .rev()
        .find(|message| message.sender == "ai" && message.timestamp > beforeLastAiTimestamp)
        .ok_or_else(|| "core did not produce ai message for current turn".to_string())?
        .clone();
    Ok(ChatSendResult {
        chatId: currentChatId,
        aiMessage,
    })
}

pub(crate) async fn dispatch_chat_message_with_application(
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
    match inputProcessingStateByChatId
        .get(&currentChatId)
        .or_else(|| inputProcessingStateByChatId.get("__DEFAULT_CHAT__"))
    {
        Some(InputProcessingState::Error { message }) => return Err(message.clone()),
        _ => {}
    }
    Ok(beforeLastAiTimestamp)
}

pub(crate) fn launch_chat_message_with_application(
    application: &mut OperitApplication,
    sendArgs: ChatSendArgs,
) -> Result<String, String> {
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
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    core.enhancedAiService = Some(EnhancedAIService::new(ConversationService));
    if let Some(chatId) = sendArgs.chatId.as_ref() {
        core.switchChat(chatId.clone());
    }
    let chatId = core
        .currentChatIdFlow()
        .value()
        .ok_or_else(|| "core has no active chat before send".to_string())?;
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

    let mut service = core
        .enhancedAiService
        .clone()
        .ok_or_else(|| "ai service is not initialized".to_string())?;
    let chatHistoryDelegate = core.chatHistoryDelegate.clone_for_core();
    let messageProcessingDelegate = core.messageProcessingDelegate.clone_for_core();
    let mut delegate =
        MessageCoordinationDelegate::new(chatHistoryDelegate, messageProcessingDelegate);
    if let Some(coreDelegate) = core.messageCoordinationDelegate.as_mut() {
        coreDelegate
            .tokenStatisticsDelegate
            .setActiveChatId(Some(chatId.clone()));
        coreDelegate
            .tokenStatisticsDelegate
            .bindChatService(Some(chatId.clone()), &service);
        delegate.tokenStatisticsDelegate = coreDelegate.tokenStatisticsDelegate.clone();
    }
    let threadChatId = chatId.clone();
    std::thread::spawn(move || {
        let runtimeResult = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let runtime = match runtimeResult {
            Ok(runtime) => runtime,
            Err(error) => {
                delegate
                    .messageProcessingDelegate
                    .setInputProcessingStateForChat(
                        threadChatId,
                        InputProcessingState::Error {
                            message: error.to_string(),
                        },
                    );
                return;
            }
        };
        runtime.block_on(async move {
            delegate
                .sendUserMessage(
                    &mut service,
                    PromptFunctionType::CHAT,
                    None,
                    Some(threadChatId),
                    None,
                    None,
                    Some(chatBinding.providerId),
                    Some(chatBinding.modelId),
                    attachments,
                    replyToMessage,
                    ChatTurnOptions::default(),
                )
                .await;
        });
    });
    Ok(chatId)
}

pub(crate) async fn send_chat_message_with_application(
    application: &mut OperitApplication,
    sendArgs: ChatSendArgs,
) -> Result<ChatSendResult, String> {
    let mut result = begin_chat_message_with_application(application, sendArgs).await?;
    if let Some(mut stream) = result.aiMessage.contentStream.clone() {
        let mut content = String::new();
        stream.collect(&mut |chunk| {
            content.push_str(&chunk);
        });
        result.aiMessage.content = content;
        result.aiMessage.contentStream = None;
    }
    result.aiMessage = wait_for_committed_ai_message(
        application,
        &result.chatId,
        result.aiMessage.timestamp,
        Duration::from_secs(30),
    )?;
    Ok(result)
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

fn print_chat_send_result(result: &ChatSendResult) {
    print!("{}", result.aiMessage.content);
    println!();
    eprintln!(
        "chat={} provider={} modelName={} inputTokens={} cachedInputTokens={} outputTokens={}",
        result.chatId,
        result.aiMessage.provider,
        result.aiMessage.modelName,
        result.aiMessage.inputTokens,
        result.aiMessage.cachedInputTokens,
        result.aiMessage.outputTokens
    );
}

async fn send_chat_message(sendArgs: ChatSendArgs) -> Result<(), String> {
    let mut application = create_cli_application();
    application.onCreate()?;
    let result = send_chat_message_with_application(&mut application, sendArgs).await?;
    print_chat_send_result(&result);
    Ok(())
}

async fn send_chat_message_with_core(
    core: &mut CliCore,
    sendArgs: ChatSendArgs,
) -> Result<(), String> {
    let result = send_chat_message_with_core_result(core, sendArgs).await?;
    print_chat_send_result(&result);
    Ok(())
}

async fn send_chat_message_with_core_result(
    core: &mut CliCore,
    sendArgs: ChatSendArgs,
) -> Result<ChatSendResult, String> {
    core.preferences_model_config_manager()
        .initializeIfNeeded()
        .await
        .map_err(|error| error.to_string())?;
    core.preferences_functional_config_manager()
        .initializeIfNeeded()
        .await
        .map_err(|error| error.to_string())?;
    if let Some(chatId) = sendArgs.chatId.clone() {
        core.chat_runtime_holder_main()
            .switchChat(chatId)
            .await
            .map_err(|error| error.to_string())?;
    }
    let beforeMessages = core
        .chat_runtime_holder_main()
        .chatHistoryFlowSnapshot()
        .await
        .map_err(|error| error.to_string())?;
    let beforeLastAiTimestamp = beforeMessages
        .iter()
        .filter(|message| message.sender == "ai")
        .map(|message| message.timestamp)
        .max()
        .unwrap_or(0);
    let attachments = sendArgs
        .attachmentPaths
        .iter()
        .map(|path| build_attachment_info(path))
        .collect::<Result<Vec<_>, _>>()?;
    let replyToMessage = match sendArgs.replyToTimestamp {
        Some(timestamp) => beforeMessages
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
    core.chat_runtime_holder_main()
        .sendUserMessage(
            PromptFunctionType::CHAT,
            None,
            None,
            Some(sendArgs.message),
            None,
            None,
            None,
            attachments,
            replyToMessage,
            ChatTurnOptions::default(),
        )
        .await
        .map_err(|error| error.to_string())?;
    let chatId = core
        .chat_runtime_holder_main()
        .currentChatIdFlowSnapshot()
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "core has no active chat after send".to_string())?;
    let messages = core
        .chat_runtime_holder_main()
        .chatHistoryFlowSnapshot()
        .await
        .map_err(|error| error.to_string())?;
    let aiMessage = messages
        .into_iter()
        .rev()
        .find(|message| message.sender == "ai" && message.timestamp > beforeLastAiTimestamp)
        .ok_or_else(|| "core did not produce ai message for current turn".to_string())?;
    Ok(ChatSendResult { chatId, aiMessage })
}

pub(crate) fn build_attachment_info(path: &str) -> Result<AttachmentInfo, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("attachment metadata failed: {path}: {error}"))?;
    let fileName = Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("attachment file name invalid: {path}"))?
        .to_string();
    let content = fs::read_to_string(path).unwrap_or_default();
    Ok(AttachmentInfo {
        filePath: path.to_string(),
        fileName,
        mimeType: guess_mime_type(path).to_string(),
        fileSize: metadata.len() as i64,
        content,
    })
}

pub(crate) fn guess_mime_type(path: &str) -> &'static str {
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

fn print_chat_history_header(chat: &operit_runtime::data::model::ChatHistory::ChatHistory) {
    println!("id={}", chat.id);
    println!("title={}", chat.title);
    println!("createdAt={}", chat.createdAt);
    println!("updatedAt={}", chat.updatedAt);
    println!("inputTokens={}", chat.inputTokens);
    println!("outputTokens={}", chat.outputTokens);
    println!("currentWindowSize={}", chat.currentWindowSize);
    println!("group={}", chat.group.clone().unwrap_or_default());
    println!("displayOrder={}", chat.displayOrder);
    println!("workspace={}", chat.workspace.clone().unwrap_or_default());
    println!(
        "parentChatId={}",
        chat.parentChatId.clone().unwrap_or_default()
    );
    println!(
        "characterCardName={}",
        chat.characterCardName.clone().unwrap_or_default()
    );
    println!(
        "characterGroupId={}",
        chat.characterGroupId.clone().unwrap_or_default()
    );
    println!("locked={}", chat.locked);
    println!("pinned={}", chat.pinned);
}

fn print_chat_message(message: &operit_runtime::data::model::ChatMessage::ChatMessage) {
    println!("--- message ---");
    println!("sender={}", message.sender);
    println!("timestamp={}", message.timestamp);
    println!("roleName={}", message.roleName);
    println!("selectedVariantIndex={}", message.selectedVariantIndex);
    println!("variantCount={}", message.variantCount);
    println!("provider={}", message.provider);
    println!("modelName={}", message.modelName);
    println!("inputTokens={}", message.inputTokens);
    println!("cachedInputTokens={}", message.cachedInputTokens);
    println!("outputTokens={}", message.outputTokens);
    println!("sentAt={}", message.sentAt);
    println!("waitDurationMs={}", message.waitDurationMs);
    println!("outputDurationMs={}", message.outputDurationMs);
    println!("completedAt={}", message.completedAt);
    println!("displayMode={:?}", message.displayMode);
    println!("isFavorite={}", message.isFavorite);
    println!("content={}", message.content);
}

fn nonBlankString(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
