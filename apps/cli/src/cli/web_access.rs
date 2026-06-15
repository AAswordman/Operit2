use super::*;
use crate::create_local_core;

use operit_link::{
    RemoteDeviceInfo, RemoteHostInteractionBroker, RemoteLinkServer, RemoteLinkServerConfig,
};
use operit_runtime::api::chat::enhance::ConversationService::ConversationService;
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::api::chat::EnhancedAIService::EnhancedAIService;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliWebAccessConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub token: String,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CliWebAccessState {
    bind_address: String,
    base_url: String,
    web_root: String,
    shutdown_token: String,
    process_id: u32,
    started_at: i64,
}

pub(crate) async fn run_web_access_command(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("open") => run_web_access_open_command(&args[1..]).await,
        Some("close") => run_web_access_close_command().await,
        Some("status") => run_web_access_status_command().await,
        Some("token") => run_web_access_token_command(&args[1..]).await,
        _ => {
            print_web_access_usage();
            Ok(())
        }
    }
}

async fn run_web_access_open_command(args: &[String]) -> Result<(), String> {
    let mut bind_address = None::<String>;
    let mut token = None::<String>;
    let mut link_session_name = None::<String>;
    let mut web_root = None::<PathBuf>;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--bind" => {
                index += 1;
                bind_address = Some(
                    args.get(index)
                        .ok_or_else(|| {
                            "usage: operit2 cli web open [--bind <addr:port>] [--token <token>] [--link <session>] [--web-root <path>]"
                                .to_string()
                        })?
                        .clone(),
                );
            }
            "--token" => {
                index += 1;
                token = Some(
                    args.get(index)
                        .ok_or_else(|| {
                            "usage: operit2 cli web open [--bind <addr:port>] [--token <token>] [--link <session>] [--web-root <path>]"
                                .to_string()
                        })?
                        .clone(),
                );
            }
            "--web-root" => {
                index += 1;
                web_root = Some(PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| {
                            "usage: operit2 cli web open [--bind <addr:port>] [--token <token>] [--link <session>] [--web-root <path>]"
                                .to_string()
                        })?
                        .clone(),
                ));
            }
            "--link" => {
                index += 1;
                link_session_name = Some(
                    args.get(index)
                        .ok_or_else(|| {
                            "usage: operit2 cli web open [--bind <addr:port>] [--token <token>] [--link <session>] [--web-root <path>]"
                                .to_string()
                        })?
                        .clone(),
                );
            }
            _ => {
                return Err(
                    "usage: operit2 cli web open [--bind <addr:port>] [--token <token>] [--link <session>] [--web-root <path>]"
                        .to_string(),
                );
            }
        }
        index += 1;
    }

    let stored_config = read_web_access_config()?;
    let config = CliWebAccessConfig {
        enabled: true,
        bind_address: bind_address.unwrap_or_else(|| {
            stored_config
                .as_ref()
                .map(|config| config.bind_address.clone())
                .unwrap_or_else(|| "127.0.0.1:37193".to_string())
        }),
        token: token.unwrap_or_else(|| {
            stored_config
                .as_ref()
                .map(|config| config.token.clone())
                .unwrap_or_else(generate_token)
        }),
        updated_at: unix_millis(),
    };
    write_web_access_config(&config)?;

    let web_root = resolve_web_root(web_root)?;
    let shutdown_token = generate_token();
    let state = CliWebAccessState {
        bind_address: config.bind_address.clone(),
        base_url: base_url_for_bind_address(&config.bind_address)?,
        web_root: web_root.to_string_lossy().to_string(),
        shutdown_token: shutdown_token.clone(),
        process_id: process::id(),
        started_at: unix_millis(),
    };
    write_web_access_state(&state)?;

    println!("webAccessUrl={}", state.base_url);
    println!("webAccessToken={}", config.token);
    println!(
        "webAccessStatePath={}",
        crate::client_paths::web_access_state_path().display()
    );
    println!("webRoot={}", web_root.display());

    if let Some(session_name) = link_session_name {
        let remote = super::link::load_link_session(&session_name)?;
        println!("runtimeMode=remote");
        println!("runtimeSession={session_name}");
        let result = RemoteLinkServer::serve(
            remote,
            RemoteLinkServerConfig {
                bindAddress: config.bind_address,
                token: config.token.clone(),
                deviceInfo: RemoteDeviceInfo::native(),
                hostInteractionBroker: None,
                webAccess: Some(operit_link::RemoteWebAccessConfig {
                    token: config.token,
                    shutdownToken: shutdown_token,
                    webRoot: web_root,
                }),
                printStartupInfo: false,
                acceptedSessions: BTreeMap::new(),
                acceptedSessionLoader: None,
                acceptedSessionStore: None,
                pairingCodeSink: None,
            },
        )
        .await;
        remove_web_access_state()?;
        return result;
    }

    let mut core = create_local_core();
    core.localApplicationMut().onCreate()?;
    let _external_runtime_event_registration =
        operit_runtime::core::application::ExternalRuntimeEventSupport::startExternalRuntimeEventSupport(
            core.localApplicationMut().applicationContext.clone(),
            "cli-web-access",
        )?;
    let main_core = core
        .localApplicationMut()
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN);
    main_core.enhancedAiService = Some(EnhancedAIService::new(ConversationService));
    let host_interaction_broker = RemoteHostInteractionBroker::new();
    super::link::install_remote_host_permission_requester(
        &mut core,
        host_interaction_broker.clone(),
    );

    println!("runtimeMode=local");
    let result = RemoteLinkServer::serve(
        core,
        RemoteLinkServerConfig {
            bindAddress: config.bind_address,
            token: config.token.clone(),
            deviceInfo: RemoteDeviceInfo::native(),
            hostInteractionBroker: Some(host_interaction_broker),
            webAccess: Some(operit_link::RemoteWebAccessConfig {
                token: config.token,
                shutdownToken: shutdown_token,
                webRoot: web_root,
            }),
            printStartupInfo: false,
            acceptedSessions: BTreeMap::new(),
            acceptedSessionLoader: None,
            acceptedSessionStore: None,
            pairingCodeSink: None,
        },
    )
    .await;
    remove_web_access_state()?;
    result
}

async fn run_web_access_close_command() -> Result<(), String> {
    let mut config = web_access_config_for_write()?;
    config.enabled = false;
    config.updated_at = unix_millis();
    write_web_access_config(&config)?;

    let Some(state) = read_web_access_state_optional()? else {
        println!("webAccessClosed=true");
        println!("runningState=false");
        return Ok(());
    };
    let client = reqwest::Client::new();
    client
        .post(format!("{}/client/web-access/close", state.base_url))
        .header("x-operit-web-access-shutdown-token", state.shutdown_token)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;
    println!("webAccessClosed=true");
    Ok(())
}

async fn run_web_access_status_command() -> Result<(), String> {
    let config_path = crate::client_paths::web_access_config_path();
    println!("configPath={}", config_path.display());
    match read_web_access_config()? {
        Some(config) => {
            println!("configured=true");
            println!("enabled={}", config.enabled);
            println!("bindAddress={}", config.bind_address);
            println!("token={}", config.token);
            println!("updatedAt={}", config.updated_at);
        }
        None => {
            println!("configured=false");
        }
    }
    let state_path = crate::client_paths::web_access_state_path();
    println!("statePath={}", state_path.display());
    match read_web_access_state_optional()? {
        Some(state) => {
            println!("runningState=true");
            println!("baseUrl={}", state.base_url);
            println!("processId={}", state.process_id);
            println!("startedAt={}", state.started_at);
        }
        None => {
            println!("runningState=false");
        }
    }
    Ok(())
}

async fn run_web_access_token_command(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("rotate") if args.len() == 1 => {
            let mut config = web_access_config_for_write()?;
            config.token = generate_token();
            config.updated_at = unix_millis();
            write_web_access_config(&config)?;
            println!("webAccessToken={}", config.token);
            Ok(())
        }
        Some("set") if args.len() == 2 => {
            let mut config = web_access_config_for_write()?;
            config.token = args[1].clone();
            config.updated_at = unix_millis();
            write_web_access_config(&config)?;
            println!("webAccessToken={}", config.token);
            Ok(())
        }
        _ => {
            println!("operit2 cli web token rotate");
            println!("operit2 cli web token set <token>");
            Ok(())
        }
    }
}

fn web_access_config_for_write() -> Result<CliWebAccessConfig, String> {
    Ok(
        read_web_access_config()?.unwrap_or_else(|| CliWebAccessConfig {
            enabled: false,
            bind_address: "127.0.0.1:37193".to_string(),
            token: generate_token(),
            updated_at: unix_millis(),
        }),
    )
}

fn read_web_access_config() -> Result<Option<CliWebAccessConfig>, String> {
    let path = crate::client_paths::web_access_config_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn write_web_access_config(config: &CliWebAccessConfig) -> Result<(), String> {
    let path = crate::client_paths::web_access_config_path();
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid web access config path: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let content = serde_json::to_string_pretty(config).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

fn read_web_access_state_optional() -> Result<Option<CliWebAccessState>, String> {
    let path = crate::client_paths::web_access_state_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn write_web_access_state(state: &CliWebAccessState) -> Result<(), String> {
    let path = crate::client_paths::web_access_state_path();
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid web access state path: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let content = serde_json::to_string_pretty(state).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

fn remove_web_access_state() -> Result<(), String> {
    let path = crate::client_paths::web_access_state_path();
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn base_url_for_bind_address(bind_address: &str) -> Result<String, String> {
    let (host, port) = bind_address
        .rsplit_once(':')
        .ok_or_else(|| format!("invalid bind address: {bind_address}"))?;
    let host = match host {
        "0.0.0.0" | "::" => "127.0.0.1",
        value => value,
    };
    Ok(format!("http://{host}:{port}"))
}

fn resolve_web_root(value: Option<PathBuf>) -> Result<PathBuf, String> {
    let web_root = match value {
        Some(path) => path,
        None => crate::web_access_assets::materialize_web_access_bundle()?,
    };
    let index = web_root.join("index.html");
    if !index.is_file() {
        return Err(format!(
            "Flutter Web bundle not found: {}. Rebuild operit2 after building Flutter Web or pass --web-root <path>.",
            web_root.display()
        ));
    }
    Ok(web_root)
}

fn generate_token() -> String {
    format!("ow-{}", Uuid::new_v4().simple())
}

fn unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after UNIX_EPOCH")
        .as_millis() as i64
}

fn print_web_access_usage() {
    println!("operit2 cli web open [--bind <addr:port>] [--token <token>] [--link <session>] [--web-root <path>]");
    println!("operit2 cli web close");
    println!("operit2 cli web status");
    println!("operit2 cli web token rotate");
    println!("operit2 cli web token set <token>");
}
