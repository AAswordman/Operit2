use std::env;
use std::process::ExitCode;

use operit_runtime::data::model::FunctionType::FunctionType;
use operit_runtime::data::model::PromptFunctionType::PromptFunctionType;
use operit_runtime::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use operit_runtime::data::preferences::ModelConfigManager::ModelConfigManager;
use operit_runtime::api::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::api::chat::enhance::ConversationService::ConversationService;
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::data::model::ChatTurnOptions::ChatTurnOptions;
use operit_runtime::data::model::InputProcessingState::InputProcessingState;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "model" => run_model_command(&args[1..]),
        "chat" => run_chat_command(&args[1..]).await,
        _ => {
            print_usage();
            Ok(())
        }
    }
}

fn run_model_command(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_model_usage();
        return Ok(());
    }

    let modelConfigManager = ModelConfigManager::default();
    let functionalConfigManager = FunctionalConfigManager::default();

    match args[0].as_str() {
        "init" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            functionalConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            println!("initialized");
        }
        "list" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            for summary in modelConfigManager
                .getAllConfigSummaries()
                .map_err(|error| error.to_string())?
            {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    summary.id,
                    summary.name,
                    summary.apiProviderType.name(),
                    summary.apiEndpoint,
                    summary.modelName
                );
            }
        }
        "show" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let configId = match args.get(1).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let config = modelConfigManager
                .getModelConfig(configId)
                .map_err(|error| error.to_string())?;
            println!("id={}", config.id);
            println!("name={}", config.name);
            println!("provider={}", config.apiProviderType.name());
            println!("providerTypeId={}", config.apiProviderTypeId);
            println!("endpoint={}", config.apiEndpoint);
            println!("modelName={}", config.modelName);
            println!("apiKeyLength={}", config.apiKey.len());
        }
        "set-key" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let apiKey = args
                .get(1)
                .ok_or_else(|| "usage: operit2 model set-key <api-key> [config-id]".to_string())?
                .clone();
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            modelConfigManager
                .updateApiKey(configId, apiKey)
                .map_err(|error| error.to_string())?;
            println!("api key updated: {configId}");
        }
        "set" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let endpoint = args
                .get(1)
                .ok_or_else(|| "usage: operit2 model set <endpoint> <model-name> [config-id]".to_string())?
                .clone();
            let modelName = args
                .get(2)
                .ok_or_else(|| "usage: operit2 model set <endpoint> <model-name> [config-id]".to_string())?
                .clone();
            let configId = match args.get(3).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let current = modelConfigManager
                .getModelConfig(configId)
                .map_err(|error| error.to_string())?;
            modelConfigManager
                .updateModelConfig(configId, current.apiKey, endpoint, modelName)
                .map_err(|error| error.to_string())?;
            println!("model updated: {configId}");
        }
        "params" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let configId = match args.get(1).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let params = modelConfigManager
                .getModelParametersForConfig(configId)
                .map_err(|error| error.to_string())?;
            for param in params {
                println!(
                    "{}\t{}\t{}\t{}",
                    param.id,
                    param.apiName,
                    param.isEnabled,
                    param.currentValue
                );
            }
        }
        _ => print_model_usage(),
    }

    Ok(())
}

async fn run_chat_command(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_chat_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "new" => create_chat().await,
        "send" => {
            let (chatId, message) = parse_chat_send_args(&args[1..])?;
            send_chat_message(chatId, message).await
        }
        _ => {
            print_chat_usage();
            Ok(())
        }
    }
}

async fn create_chat() -> Result<(), String> {
    let mut application = OperitApplication::new();
    application.onCreate()?;
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    core.createNewChat(None, None, true, true, None);
    let chatId = core
        .currentChatId()
        .clone()
        .ok_or_else(|| "core did not create chat".to_string())?;
    println!("{chatId}");
    Ok(())
}

fn parse_chat_send_args(args: &[String]) -> Result<(Option<String>, String), String> {
    if args.is_empty() {
        return Err("usage: operit2 chat send [--chat <chat-id>] <message>".to_string());
    }
    if args.get(0).map(String::as_str) == Some("--chat") {
        let chatId = args
            .get(1)
            .ok_or_else(|| "usage: operit2 chat send [--chat <chat-id>] <message>".to_string())?
            .clone();
        let message = args
            .get(2)
            .ok_or_else(|| "usage: operit2 chat send [--chat <chat-id>] <message>".to_string())?
            .clone();
        return Ok((Some(chatId), message));
    }
    Ok((None, args[0].clone()))
}

async fn send_chat_message(chatIdOverride: Option<String>, message: String) -> Result<(), String> {
    let modelConfigManager = ModelConfigManager::default();
    let functionalConfigManager = FunctionalConfigManager::default();
    modelConfigManager
        .initializeIfNeeded()
        .map_err(|error| error.to_string())?;
    functionalConfigManager
        .initializeIfNeeded()
        .map_err(|error| error.to_string())?;
    let chatMapping = functionalConfigManager
        .getConfigMappingForFunction(FunctionType::CHAT)
        .map_err(|error| error.to_string())?;
    let turnOptions = ChatTurnOptions::default();
    let mut application = OperitApplication::new();
    application.onCreate()?;
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    core.enhancedAiService = Some(EnhancedAIService::new(ConversationService));
    if let Some(chatId) = chatIdOverride.as_ref() {
        core.switchChat(chatId.clone());
    }
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
        chatIdOverride,
        Some(message),
        None,
        Some(chatMapping.configId),
        Some(chatMapping.modelIndex),
        turnOptions,
    )
    .await;
    let currentChatId = core
        .currentChatId()
        .clone()
        .ok_or_else(|| "core has no active chat after send".to_string())?;
    match core
        .inputProcessingStateByChatId()
        .get(&currentChatId)
        .or_else(|| core.inputProcessingStateByChatId().get("__DEFAULT_CHAT__"))
    {
        Some(InputProcessingState::Error { message }) => return Err(message.clone()),
        _ => {}
    }
    let aiMessage = core
        .chatHistory()
        .iter()
        .rev()
        .find(|message| message.sender == "ai" && message.timestamp > beforeLastAiTimestamp)
        .ok_or_else(|| "core did not produce ai message for current turn".to_string())?;
    print!("{}", aiMessage.content);
    println!();
    eprintln!(
        "provider={} modelName={} inputTokens={} cachedInputTokens={} outputTokens={}",
        aiMessage.provider,
        aiMessage.modelName,
        aiMessage.inputTokens,
        aiMessage.cachedInputTokens,
        aiMessage.outputTokens
    );
    Ok(())
}

fn print_usage() {
    println!("operit2 model <init|list|show|set|set-key|params>");
    println!("operit2 chat new");
    println!("operit2 chat send [--chat <chat-id>] <message>");
}

fn print_model_usage() {
    println!("operit2 model init");
    println!("operit2 model list");
    println!("operit2 model show [config-id]");
    println!("operit2 model set <endpoint> <model-name> [config-id]");
    println!("operit2 model set-key <api-key> [config-id]");
    println!("operit2 model params [config-id]");
}

fn print_chat_usage() {
    println!("operit2 chat new");
    println!("operit2 chat send [--chat <chat-id>] <message>");
}
