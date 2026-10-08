use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::core_proxy::{local_cli_core, CliCore};
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_model::ChatMessage::ChatMessage;
use operit_store::repository::ChatHistoryManager::ChatHistoryManager;

/// Runs the chat shell through an existing CoreNode-aware CLI proxy.
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
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            chat.id,
            chat.title,
            chat.createdAt,
            chat.updatedAt,
            chat.inputTokens,
            chat.outputTokens,
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
        .chatMessagesFlowSnapshot(chatId.clone())
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

/// Deletes one message from the active chat by its stable timestamp.
async fn delete_chat_message_with_core(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    let messageTimestamp = parse_message_timestamp_arg(args, "delete-message")?;
    let chatId = current_chat_id_with_core(core).await?;
    core.chat_runtime_holder_main()
        .deleteMessage(chatId, messageTimestamp)
        .await
        .map_err(|error| error.to_string())?;
    println!("message deleted: {messageTimestamp}");
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

/// Rolls the active chat back to one user message by its stable timestamp.
async fn rollback_chat_with_core(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    let messageTimestamp = parse_message_timestamp_arg(args, "rollback")?;
    let chatId = current_chat_id_with_core(core).await?;
    let rolledBack = core
        .chat_runtime_holder_main()
        .rollbackToMessage(chatId, messageTimestamp)
        .await
        .map_err(|error| error.to_string())?;
    match rolledBack {
        Some(message) => println!("rolled back to message: {messageTimestamp}\n{message}"),
        None => println!("rollback skipped: message must exist and be a user message"),
    }
    Ok(())
}

/// Reads the active chat id used by timestamp-scoped message commands.
async fn current_chat_id_with_core(core: &mut CliCore) -> Result<String, String> {
    core.chat_runtime_holder_main()
        .currentChatIdFlowSnapshot()
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "no current chat selected".to_string())
}

/// Parses the message timestamp used by a CLI message command.
fn parse_message_timestamp_arg(args: &[String], command: &str) -> Result<i64, String> {
    let usage = format!("usage: operit2 chat {command} <message-timestamp>");
    args.get(0)
        .ok_or_else(|| usage.clone())?
        .parse::<i64>()
        .map_err(|_| usage)
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
    println!("Chat {chatId} locked: {locked}");
    Ok(())
}

async fn update_chat_pinned_with_core(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    let (chatId, pinned) = parse_chat_bool_update_args(args, "pin")?;
    core.chat_runtime_holder_main()
        .updateChatPinned(chatId.clone(), pinned)
        .await
        .map_err(|error| error.to_string())?;
    println!("Chat {chatId} pinned: {pinned}");
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
    pub(crate) sourceChatId: Option<String>,
    pub(crate) input: Option<serde_json::Value>,
    pub(crate) updateCurrentVersion: Option<String>,
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

/// Parses only generic chat creation input; plugin identity remains opaque to the frontend.
pub(crate) fn parse_shell_args(args: &[String]) -> Result<ShellArgs, String> {
    let usage = "usage: operit2 [--chat <chat-id> | --resume | --source <chat-id> --input <json-object>] [--update-current-version <version>]";
    let mut shellArgs = ShellArgs {
        chatId: None,
        resume: false,
        sourceChatId: None,
        input: None,
        updateCurrentVersion: None,
    };
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        if flag == "--resume" {
            if shellArgs.resume {
                return Err(usage.to_string());
            }
            shellArgs.resume = true;
        } else {
            index += 1;
            let value = args
                .get(index)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| usage.to_string())?;
            match flag {
                "--chat" if shellArgs.chatId.is_none() => shellArgs.chatId = Some(value.clone()),
                "--source" if shellArgs.sourceChatId.is_none() => {
                    shellArgs.sourceChatId = Some(value.clone())
                }
                "--input" if shellArgs.input.is_none() => {
                    let input: serde_json::Value = serde_json::from_str(value)
                        .map_err(|error| format!("invalid --input JSON: {error}"))?;
                    if !input.is_object() {
                        return Err("--input must be a JSON object".to_string());
                    }
                    shellArgs.input = Some(input);
                }
                "--update-current-version" if shellArgs.updateCurrentVersion.is_none() => {
                    shellArgs.updateCurrentVersion = Some(value.clone())
                }
                _ => return Err(usage.to_string()),
            }
        }
        index += 1;
    }
    if (shellArgs.chatId.is_some() && shellArgs.resume)
        || ((shellArgs.chatId.is_some() || shellArgs.resume)
            && (shellArgs.sourceChatId.is_some() || shellArgs.input.is_some()))
    {
        return Err(usage.to_string());
    }
    Ok(shellArgs)
}

impl ShellArgs {
    /// Builds the canonical new-chat command without interpreting any plugin payload.
    pub(crate) fn new_chat_command_args(&self) -> Vec<String> {
        let mut args = vec!["chat".to_string(), "new".to_string(), "--json".to_string()];
        if let Some(source) = &self.sourceChatId {
            args.extend(["--source".to_string(), source.clone()]);
        }
        if let Some(input) = &self.input {
            args.extend(["--input".to_string(), input.to_string()]);
        }
        args
    }
}

pub(crate) async fn run_shell_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    run_shell_command_with_core(core, args).await
}

/// Runs one interactive shell session through a CoreNode-aware CLI proxy.
async fn run_shell_command_with_core(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    let shellArgs = parse_shell_args(args)?;
    let mut queuedAttachmentPaths = Vec::<String>::new();
    let initialChatId = initialize_shell_chat_with_core(core, &shellArgs).await?;
    println!("interactive shell ready");
    println!("Chat: {initialChatId}");
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
        let output = core
            .runCoreCommand(&shellArgs.new_chat_command_args())
            .await
            .map_err(|error| error.to_string())?;
        let result: serde_json::Value =
            serde_json::from_str(&output.stdout).map_err(|error| error.to_string())?;
        result
            .get("chatId")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| "chat creation did not return a chatId".to_string())
    }
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
        .loadChatHistories()
        .map_err(|error| error.to_string())?
        .iter()
        .any(|chat| chat.id == chatId);
    if exists {
        Ok(())
    } else {
        Err(format!("chat not found: {chatId}"))
    }
}

async fn current_shell_chat_id_with_core(core: &mut CliCore) -> Result<String, String> {
    core.chat_runtime_holder_main()
        .currentChatIdFlowSnapshot()
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "no active chat in shell".to_string())
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
            core.runCoreCommand(&shellArgs.new_chat_command_args())
                .await
                .map_err(|error| error.to_string())?;
            let chatId = current_shell_chat_id_with_core(core).await?;
            println!("Chat: {chatId}");
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
            println!("Chat: {chatId}");
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
            println!("Chat: {}", target.id);
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
                println!("Attachments: none");
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
    println!("/new [--source <chat-id>] [--input <json-object>]");
    println!("/switch <chat-id>");
    println!("/resume");
    println!("/show");
    println!("/attach <path>");
    println!("/attachments");
    println!("/clear-attachments");
    println!("/send <message>");
}

/// Prints the native originating-turn outcome, including cancellation and hook consumption.
fn print_chat_send_result(result: &serde_json::Value) {
    if crate::cli::cli_json_mode() {
        crate::cli::emit_cli_json(result.clone());
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(result).expect("chat receipt must serialize")
        );
    }
}

/// Uses the same receipt-based Core command as non-interactive CLI sends.
async fn send_chat_message_with_core_result(
    core: &mut CliCore,
    sendArgs: ChatSendArgs,
) -> Result<serde_json::Value, String> {
    let mut args = vec!["chat".to_string(), "send".to_string(), "--json".to_string()];
    if let Some(chatId) = sendArgs.chatId {
        args.extend(["--chat".to_string(), chatId]);
    }
    for path in sendArgs.attachmentPaths {
        args.extend(["--attachment".to_string(), path]);
    }
    if let Some(timestamp) = sendArgs.replyToTimestamp {
        args.extend(["--reply-to".to_string(), timestamp.to_string()]);
    }
    args.push(sendArgs.message);
    let output = core
        .runCoreCommand(&args)
        .await
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&output.stdout)
        .map_err(|error| format!("invalid Core chat receipt: {error}"))
}

/// Builds one attachment descriptor from a local filesystem path.
pub(crate) fn build_attachment_info(path: &str) -> Result<AttachmentInfo, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("attachment metadata failed: {path}: {error}"))?;
    let fileName = Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("attachment file name invalid: {path}"))?
        .to_string();
    let mimeType = guess_mime_type(path).to_string();
    let content = attachment_text_content(path, &mimeType)?;
    Ok(AttachmentInfo {
        nodeId: operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::localNodeId(),
        filePath: path.to_string(),
        fileName,
        mimeType,
        fileSize: metadata.len() as i64,
        content,
    })
}

/// Reads text attachment content for MIME types carried inline by the CLI.
fn attachment_text_content(path: &str, mimeType: &str) -> Result<String, String> {
    match mimeType {
        "text/plain" => fs::read_to_string(path)
            .map_err(|error| format!("attachment text read failed: {path}: {error}")),
        _ => Ok(String::new()),
    }
}

/// Returns the MIME type associated with a local attachment path.
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

fn print_chat_history_header(chat: &operit_model::ChatHistory::ChatHistory) {
    println!("Chat {}", chat.id);
    println!("Title: {}", chat.title);
    println!("Created: {}", chat.createdAt);
    println!("Updated: {}", chat.updatedAt);
    println!("Input tokens: {}", chat.inputTokens);
    println!("Output tokens: {}", chat.outputTokens);
    println!("Context window: {}", chat.currentWindowSize);
    println!("Display order: {}", chat.displayOrder);
    println!(
        "Workspace: {}",
        chat.workspacePrimaryPath.clone().unwrap_or_default()
    );
    println!(
        "parentChatId={}",
        chat.parentChatId.clone().unwrap_or_default()
    );
    println!("Locked: {}", chat.locked);
    println!("Pinned: {}", chat.pinned);
}

fn print_chat_message(message: &operit_model::ChatMessage::ChatMessage) {
    println!("--- message ---");
    println!("Sender: {}", message.sender);
    println!("Timestamp: {}", message.timestamp);
    println!("Role: {}", message.roleName);
    println!("Selected variant: {}", message.selectedVariantIndex);
    println!("Variants: {}", message.variantCount);
    println!("Provider: {}", message.provider);
    println!("Model: {}", message.modelName);
    println!("Input tokens: {}", message.inputTokens);
    println!("Cached input tokens: {}", message.cachedInputTokens);
    println!("Output tokens: {}", message.outputTokens);
    println!("Sent at: {}", message.sentAt);
    println!("Wait duration: {} ms", message.waitDurationMs);
    println!("Output duration: {} ms", message.outputDurationMs);
    println!("Completed at: {}", message.completedAt);
    println!("Display mode: {:?}", message.displayMode);
    println!("Favorite: {}", message.isFavorite);
    println!("Content: {}", message.displayText());
}

fn nonBlankString(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod shell_argument_tests {
    use super::*;

    #[test]
    fn new_chat_payload_remains_opaque_and_source_is_forwarded() {
        let input =
            serde_json::json!({"arbitrary.plugin": {"selection": "opaque value", "version": 7}});
        let args = parse_shell_args(&[
            "--source".to_string(),
            "source-chat".to_string(),
            "--input".to_string(),
            input.to_string(),
        ])
        .unwrap();
        assert_eq!(args.input, Some(input.clone()));
        assert_eq!(
            args.new_chat_command_args(),
            vec![
                "chat".to_string(),
                "new".to_string(),
                "--json".to_string(),
                "--source".to_string(),
                "source-chat".to_string(),
                "--input".to_string(),
                input.to_string()
            ]
        );
    }

    #[test]
    fn rejects_retired_role_flags_and_invalid_creation_options() {
        for args in [
            vec!["--character", "old-role"],
            vec!["--group-card", "old-group"],
            vec!["--input", "[]"],
            vec!["--input", "not-json"],
            vec!["--source"],
            vec!["--source", "one", "--source", "two"],
            vec!["--chat", "existing", "--input", "{}"],
            vec!["--resume", "--source", "source"],
            vec!["--resume", "--resume"],
            vec!["--chat", "existing", "--resume"],
        ] {
            assert!(
                parse_shell_args(&args.into_iter().map(str::to_string).collect::<Vec<_>>())
                    .is_err()
            );
        }
    }
}
