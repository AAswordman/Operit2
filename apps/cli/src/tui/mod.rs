mod approval;
mod app;
mod commands;
mod empty_state;
mod helpers;
mod input;
mod link_proxy_rs;
mod markdown;
mod render;
mod theme;
mod typewriter;

use approval::TuiApprovalBridge;
use app::OperitTui;
use link_proxy_rs::tui_core;
use operit_link::{
    CoreCallRequest, CoreObjectPath, CoreWatchRequest, PairedRemoteSession,
    PairedRemoteSessionRecord, RemoteLinkClient, RemoteLinkServer, RemoteLinkServerConfig,
};
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::api::chat::enhance::ConversationService::ConversationService;
use operit_runtime::api::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::tools::AIToolHandler::AIToolHandler;
use operit_runtime::data::preferences::ApiPreferences::ApiPreferences;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{create_local_core, initialize_shell_chat, parse_shell_args};

pub(crate) async fn run_tui_command(args: &[String]) -> Result<(), String> {
    if args.first().map(String::as_str) == Some("link") {
        return run_tui_link_command(&args[1..]).await;
    }
    if args.first().map(String::as_str) == Some("remote") {
        return run_remote_tui_command(&args[1..]).await;
    }
    let shell_args = parse_shell_args(args)?;
    let mut core = create_local_core();
    core.localApplicationMut().onCreate()?;
    let initial_chat_id = initialize_shell_chat(core.localApplicationMut(), &shell_args)?;
    let approval_bridge = TuiApprovalBridge::new();
    install_local_permission_requester(&mut core, approval_bridge.clone());
    let mut tui = OperitTui::new(tui_core(core), shell_args, initial_chat_id, approval_bridge).await?;
    tui.run().await
}

async fn run_remote_tui_command(args: &[String]) -> Result<(), String> {
    let session_name = args
        .get(0)
        .ok_or_else(|| "usage: operit2 tui remote <session> [--chat <chat-id>]".to_string())?
        .clone();
    let shell_args = parse_shell_args(&args[1..])?;
    let session = load_link_session(&session_name)?;
    let mut core = tui_core(session);
    let initial_chat_id = initialize_remote_chat(&mut core, &shell_args).await?;
    let approval_bridge = TuiApprovalBridge::new();
    let mut tui = OperitTui::new(core, shell_args, initial_chat_id, approval_bridge).await?;
    tui.run().await
}

async fn run_tui_link_command(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("serve") => run_link_serve_command(&args[1..]).await,
        Some("connect") => run_link_connect_command(&args[1..]).await,
        Some("hello") => run_link_hello_command(&args[1..]).await,
        Some("sessions") => run_link_sessions_command().await,
        Some("ping") => run_link_ping_command(&args[1..]).await,
        Some("call") => run_link_call_command(&args[1..]).await,
        Some("watch") => run_link_watch_command(&args[1..]).await,
        _ => {
            print_link_usage();
            Ok(())
        }
    }
}

async fn run_link_serve_command(args: &[String]) -> Result<(), String> {
    let mut bind_address = "0.0.0.0:37192".to_string();
    let mut token = "operit-link-dev".to_string();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--bind" => {
                index += 1;
                bind_address = args
                    .get(index)
                    .ok_or_else(|| "usage: operit2 tui link serve [--bind <addr:port>] [--token <token>]".to_string())?
                    .clone();
            }
            "--token" => {
                index += 1;
                token = args
                    .get(index)
                    .ok_or_else(|| "usage: operit2 tui link serve [--bind <addr:port>] [--token <token>]".to_string())?
                    .clone();
            }
            _ => {
                return Err("usage: operit2 tui link serve [--bind <addr:port>] [--token <token>]".to_string());
            }
        }
        index += 1;
    }
    let mut core = create_local_core();
    core.localApplicationMut().onCreate()?;
    let main_core = core.localApplicationMut().chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    main_core.enhancedAiService = Some(EnhancedAIService::new(ConversationService));
    RemoteLinkServer::serve(
        core,
        RemoteLinkServerConfig {
            bindAddress: bind_address,
            token,
        },
    )
    .await
}

fn install_local_permission_requester(
    core: &mut operit_core_proxy::LocalCoreProxy,
    approval_bridge: TuiApprovalBridge,
) {
    let context = core.localApplicationMut().applicationContext.clone();
    let handler = AIToolHandler::getInstance(context);
    handler
        .getToolPermissionSystem()
        .setPermissionRequester(move |tool, description| approval_bridge.request(tool, description));
}

async fn initialize_remote_chat(
    core: &mut link_proxy_rs::TuiCore,
    shell_args: &crate::ShellArgs,
) -> Result<String, String> {
    core.preferences_model_config_manager()
        .initializeIfNeeded()
        .await
        .map_err(|error| error.to_string())?;
    core.preferences_functional_config_manager()
        .initializeIfNeeded()
        .await
        .map_err(|error| error.to_string())?;
    if let Some(chat_id) = shell_args.chatId.clone() {
        core.chat_runtime_holder_main()
            .switchChat(chat_id.clone())
            .await
            .map_err(|error| error.to_string())?;
        Ok(chat_id)
    } else {
        core.chat_runtime_holder_main()
            .createNewChat(
                shell_args.characterCardName.clone(),
                shell_args.group.clone(),
                true,
                true,
                shell_args.characterGroupId.clone(),
            )
            .await
            .map_err(|error| error.to_string())?;
        core.chat_runtime_holder_main()
            .currentChatIdFlowSnapshot()
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "remote core did not create chat".to_string())
    }
}

async fn run_link_hello_command(args: &[String]) -> Result<(), String> {
    let (url, token) = parse_remote_url_token(args, "usage: operit2 tui link hello <url> --token <token>")?;
    let client = RemoteLinkClient::new(url);
    let hello = client.hello(&token).await?;
    println!("{}", serde_json::to_string_pretty(&hello).map_err(|error| error.to_string())?);
    Ok(())
}

async fn run_link_connect_command(args: &[String]) -> Result<(), String> {
    let (url, token, save_name) =
        parse_remote_url_token_save(args, "usage: operit2 tui link connect <url> --token <token> [--save <name>]")?;
    let client = RemoteLinkClient::new(url);
    let hello = client.hello(&token).await?;
    println!(
        "remote core={} transports={}",
        hello.coreDeviceId,
        hello.transports.join(",")
    );
    let pair_state = client.pairStart(&token).await?;
    println!("pairing started: {}", pair_state.pairingId);
    println!("check the server terminal for pairing code");
    print!("pairing code> ");
    io::stdout().flush().map_err(|error| error.to_string())?;
    let mut code = String::new();
    io::stdin()
        .read_line(&mut code)
        .map_err(|error| error.to_string())?;
    let session = client.pairFinish(&pair_state, &code).await?;
    println!("paired session={}", session.sessionId);
    let info = session.sessionInfo().await?;
    println!(
        "session active core={} client={} transports={}",
        info.coreDeviceId,
        info.clientDeviceId,
        info.transports.join(",")
    );
    if let Some(name) = save_name {
        save_link_session(&name, session.exportRecord())?;
        println!("session saved: {name}");
    }
    Ok(())
}

async fn run_link_sessions_command() -> Result<(), String> {
    let sessions = load_link_sessions()?;
    for (name, session) in sessions {
        println!("{}\t{}\t{}", name, session.baseUrl, session.deviceId);
    }
    Ok(())
}

async fn run_link_ping_command(args: &[String]) -> Result<(), String> {
    let name = args
        .get(0)
        .ok_or_else(|| "usage: operit2 tui link ping <name>".to_string())?;
    let sessions = load_link_sessions()?;
    let record = sessions
        .get(name)
        .ok_or_else(|| format!("link session not found: {name}"))?
        .clone();
    let session = PairedRemoteSession::fromRecord(record)?;
    let info = session.sessionInfo().await?;
    println!(
        "session active core={} client={} transports={}",
        info.coreDeviceId,
        info.clientDeviceId,
        info.transports.join(",")
    );
    Ok(())
}

async fn run_link_call_command(args: &[String]) -> Result<(), String> {
    let name = args
        .get(0)
        .ok_or_else(|| "usage: operit2 tui link call <session> <target-path> <method-name> [args-json]".to_string())?;
    let target_path = args
        .get(1)
        .ok_or_else(|| "usage: operit2 tui link call <session> <target-path> <method-name> [args-json]".to_string())?;
    let method_name = args
        .get(2)
        .ok_or_else(|| "usage: operit2 tui link call <session> <target-path> <method-name> [args-json]".to_string())?;
    let args_json = parse_link_args_json(args.get(3))?;
    let session = load_link_session(name)?;
    let response = session
        .call(CoreCallRequest::new(
            link_request_id(),
            CoreObjectPath::parse(target_path),
            method_name.clone(),
            args_json,
        ))
        .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&response).map_err(|error| error.to_string())?
    );
    Ok(())
}

async fn run_link_watch_command(args: &[String]) -> Result<(), String> {
    let name = args
        .get(0)
        .ok_or_else(|| "usage: operit2 tui link watch <session> <target-path> <property-name> [args-json]".to_string())?;
    let target_path = args
        .get(1)
        .ok_or_else(|| "usage: operit2 tui link watch <session> <target-path> <property-name> [args-json]".to_string())?;
    let property_name = args
        .get(2)
        .ok_or_else(|| "usage: operit2 tui link watch <session> <target-path> <property-name> [args-json]".to_string())?;
    let args_json = parse_link_args_json(args.get(3))?;
    let mut session = load_link_session(name)?;
    let event = operit_link::CoreLinkClient::watchSnapshot(
        &mut session,
        CoreWatchRequest::new(
            link_request_id(),
            CoreObjectPath::parse(target_path),
            property_name.clone(),
            args_json,
        ),
    )
    .await
    .map_err(|error| serde_json::to_string(&error).expect("CoreLinkError must serialize"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&event).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn parse_remote_url_token(args: &[String], usage: &str) -> Result<(String, String), String> {
    let (url, token, _) = parse_remote_url_token_save(args, usage)?;
    Ok((url, token))
}

fn parse_remote_url_token_save(
    args: &[String],
    usage: &str,
) -> Result<(String, String, Option<String>), String> {
    let url = args.get(0).ok_or_else(|| usage.to_string())?.clone();
    let mut token = None::<String>;
    let mut save_name = None::<String>;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--token" => {
                index += 1;
                token = Some(args.get(index).ok_or_else(|| usage.to_string())?.clone());
            }
            "--save" => {
                index += 1;
                save_name = Some(args.get(index).ok_or_else(|| usage.to_string())?.clone());
            }
            _ => return Err(usage.to_string()),
        }
        index += 1;
    }
    Ok((url, token.ok_or_else(|| usage.to_string())?, save_name))
}

fn link_sessions_path() -> PathBuf {
    ApiPreferences::data_dir().join("link_sessions.json")
}

fn load_link_sessions() -> Result<BTreeMap<String, PairedRemoteSessionRecord>, String> {
    let path = link_sessions_path();
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let content = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

fn load_link_session(name: &str) -> Result<PairedRemoteSession, String> {
    let sessions = load_link_sessions()?;
    let record = sessions
        .get(name)
        .ok_or_else(|| format!("link session not found: {name}"))?
        .clone();
    PairedRemoteSession::fromRecord(record)
}

fn parse_link_args_json(value: Option<&String>) -> Result<serde_json::Value, String> {
    match value {
        Some(value) => serde_json::from_str(value).map_err(|error| error.to_string()),
        None => Ok(serde_json::json!({})),
    }
}

pub(super) fn link_request_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after UNIX_EPOCH")
        .as_millis();
    format!("cli-{millis}")
}

fn save_link_session(name: &str, record: PairedRemoteSessionRecord) -> Result<(), String> {
    let path = link_sessions_path();
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid link session path: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let mut sessions = load_link_sessions()?;
    sessions.insert(name.to_string(), record);
    let content = serde_json::to_string_pretty(&sessions).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

fn print_link_usage() {
    println!("operit2 tui link serve [--bind <addr:port>] [--token <token>]");
    println!("operit2 tui link hello <url> --token <token>");
    println!("operit2 tui link connect <url> --token <token> [--save <name>]");
    println!("operit2 tui link sessions");
    println!("operit2 tui link ping <name>");
    println!("operit2 tui link call <session> <target-path> <method-name> [args-json]");
    println!("operit2 tui link watch <session> <target-path> <property-name> [args-json]");
}
