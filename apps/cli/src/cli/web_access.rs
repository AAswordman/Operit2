use super::*;
use crate::create_local_core;

use crate::access::{
    link_token_hash, AcceptedRemoteSessionLoader, AcceptedRemoteSessionRecord,
    AcceptedRemoteSessionStore, RemoteDeviceInfo, RemoteHostInteractionBroker, RemoteLinkServer,
    RemoteLinkServerConfig, RemotePairingCodeRecord, RemotePairingCodeSink, RemoteWebAccessConfig,
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::mdns::MdnsRegistration;
use operit_runtime::api::chat::enhance::ConversationService::ConversationService;
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::api::chat::EnhancedAIService::EnhancedAIService;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliLinkHostConfig {
    pub web_access_enabled: bool,
    pub discovery_enabled: bool,
    pub port_mode: CliLinkHostPortMode,
    pub bind_address: String,
    pub token: String,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CliLinkHostPortMode {
    Automatic,
    Fixed,
}

impl CliLinkHostPortMode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::Fixed => "fixed",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CliLinkHostState {
    device_id: String,
    bind_address: String,
    base_url: String,
    web_access_enabled: bool,
    discovery_enabled: bool,
    web_root: String,
    shutdown_token: String,
    process_id: u32,
    started_at: i64,
}

const WEB_ACCESS_AUTOMATIC_PORTS: [u16; 10] = [
    37194, 37195, 37196, 37197, 37198, 37199, 37200, 37201, 37202, 37203,
];

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
    let mut discoverable = false;
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
            "--discoverable" => {
                discoverable = true;
            }
            _ => {
                return Err(
                    "usage: operit2 cli web open [--bind <addr:port>] [--token <token>] [--link <session>] [--web-root <path>] [--discoverable]"
                        .to_string(),
                );
            }
        }
        index += 1;
    }

    let stored_config = read_link_host_config()?;
    let port_mode = if bind_address.is_some() {
        CliLinkHostPortMode::Fixed
    } else {
        CliLinkHostPortMode::Automatic
    };
    let configured_bind_address = bind_address
        .as_ref()
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|| default_web_access_bind_address(discoverable));
    let (resolved_bind_address, listener_address, listener) =
        bind_web_access_listener(bind_address, discoverable).await?;
    let config = CliLinkHostConfig {
        web_access_enabled: true,
        discovery_enabled: discoverable,
        port_mode,
        bind_address: configured_bind_address,
        token: token.unwrap_or_else(|| {
            stored_config
                .as_ref()
                .map(|config| config.token.clone())
                .unwrap_or_else(generate_token)
        }),
        updated_at: unix_millis(),
    };

    let web_root = resolve_web_root(web_root)?;
    let shutdown_token = generate_token();
    let device_info = RemoteDeviceInfo::nativeCli("server")?;
    let device_id = super::link::load_link_host_device_id()?;
    let state = CliLinkHostState {
        device_id: device_id.clone(),
        bind_address: resolved_bind_address.clone(),
        base_url: base_url_for_bind_address(&resolved_bind_address)?,
        web_access_enabled: true,
        discovery_enabled: discoverable,
        web_root: web_root.to_string_lossy().to_string(),
        shutdown_token: shutdown_token.clone(),
        process_id: process::id(),
        started_at: unix_millis(),
    };

    let _mdns_registration = if discoverable {
        let device_display_name = device_info.displayName();
        let port = listener_address.port();
        let mut props = HashMap::new();
        props.insert("deviceId".to_string(), device_id.clone());
        props.insert("displayName".to_string(), device_display_name.clone());
        props.insert("platform".to_string(), device_info.platform.clone());
        props.insert("model".to_string(), device_info.model.clone());
        props.insert("tokenHash".to_string(), link_token_hash(&config.token));
        props.insert("version".to_string(), "1".to_string());
        let registration = MdnsRegistration::register(port, props)?;
        eprintln!("mDNS: this device is discoverable");
        Some(registration)
    } else {
        None
    };

    write_link_host_config(&config)?;
    write_link_host_state(&state)?;

    println!("webAccessUrl={}", state.base_url);
    println!("webAccessToken={}", config.token);
    println!(
        "webAccessStatePath={}",
        crate::client_paths::link_host_state_path().display()
    );
    println!("webRoot={}", web_root.display());

    let accepted_sessions_path = crate::client_paths::link_server_sessions_path();
    let acceptedSessions: BTreeMap<String, AcceptedRemoteSessionRecord> =
        load_accepted_sessions_from_file(&accepted_sessions_path)?;
    let loader_path = accepted_sessions_path.clone();
    let acceptedSessionLoader: AcceptedRemoteSessionLoader =
        Arc::new(move || load_accepted_sessions_from_file(&loader_path));
    let store_path = accepted_sessions_path.clone();
    let acceptedSessionStore: AcceptedRemoteSessionStore = Arc::new(move |sessionId, record| {
        save_accepted_session_to_file(&store_path, sessionId, record)
    });
    let pairingCodeSink: RemotePairingCodeSink = Arc::new(|record: RemotePairingCodeRecord| {
        eprintln!(
            "配对请求来自 {}: 配对码={}",
            record.clientDeviceInfo.displayName(),
            record.pairingCode
        );
        eprintln!(
            "Pairing request from {}: code={}",
            record.clientDeviceInfo.displayName(),
            record.pairingCode
        );
        Ok(())
    });

    if let Some(session_name) = link_session_name {
        let remote = super::link::load_link_session(&session_name)?;
        println!("runtimeMode=remote");
        println!("runtimeSession={session_name}");
        let result = RemoteLinkServer::serveWithListener(
            remote,
            RemoteLinkServerConfig {
                bindAddress: resolved_bind_address,
                token: config.token.clone(),
                deviceId: device_id,
                deviceInfo: device_info.clone(),
                hostInteractionBroker: None,
                webAccess: Some(RemoteWebAccessConfig {
                    token: config.token,
                    shutdownToken: shutdown_token,
                    webRoot: web_root,
                }),
                printStartupInfo: false,
                acceptedSessions,
                acceptedSessionLoader: Some(acceptedSessionLoader),
                acceptedSessionStore: Some(acceptedSessionStore),
                pairingCodeSink: Some(pairingCodeSink),
            },
            listener,
            listener_address,
        )
        .await;
        remove_link_host_state()?;
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
    let result = RemoteLinkServer::serveWithListener(
        core,
        RemoteLinkServerConfig {
            bindAddress: resolved_bind_address,
            token: config.token.clone(),
            deviceId: device_id,
            deviceInfo: device_info,
            hostInteractionBroker: Some(host_interaction_broker),
            webAccess: Some(RemoteWebAccessConfig {
                token: config.token,
                shutdownToken: shutdown_token,
                webRoot: web_root,
            }),
            printStartupInfo: false,
            acceptedSessions,
            acceptedSessionLoader: Some(acceptedSessionLoader),
            acceptedSessionStore: Some(acceptedSessionStore),
            pairingCodeSink: Some(pairingCodeSink),
        },
        listener,
        listener_address,
    )
    .await;
    remove_link_host_state()?;
    result
}

async fn run_web_access_close_command() -> Result<(), String> {
    let mut config = link_host_config_for_write()?;
    config.web_access_enabled = false;
    config.discovery_enabled = false;
    config.updated_at = unix_millis();
    write_link_host_config(&config)?;

    let Some(state) = read_link_host_state_optional()? else {
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
    let config_path = crate::client_paths::link_host_config_path();
    println!("configPath={}", config_path.display());
    match read_link_host_config()? {
        Some(config) => {
            println!("configured=true");
            println!("webAccessEnabled={}", config.web_access_enabled);
            println!("discoveryEnabled={}", config.discovery_enabled);
            println!("portMode={}", config.port_mode.as_str());
            println!("bindAddress={}", config.bind_address);
            println!("token={}", config.token);
            println!("updatedAt={}", config.updated_at);
        }
        None => {
            println!("configured=false");
        }
    }
    let state_path = crate::client_paths::link_host_state_path();
    println!("statePath={}", state_path.display());
    match read_link_host_state_optional()? {
        Some(state) => {
            println!("runningState=true");
            println!("deviceId={}", state.device_id);
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
            let mut config = link_host_config_for_write()?;
            config.token = generate_token();
            config.updated_at = unix_millis();
            write_link_host_config(&config)?;
            println!("webAccessToken={}", config.token);
            Ok(())
        }
        Some("set") if args.len() == 2 => {
            let mut config = link_host_config_for_write()?;
            config.token = args[1].clone();
            config.updated_at = unix_millis();
            write_link_host_config(&config)?;
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

async fn bind_web_access_listener(
    bind_address: Option<String>,
    discoverable: bool,
) -> Result<(String, SocketAddr, tokio::net::TcpListener), String> {
    if let Some(bind_address) = bind_address {
        return bind_exact_web_access_address(bind_address).await;
    }
    let host = if discoverable { "0.0.0.0" } else { "127.0.0.1" };
    let mut last_error = None::<String>;
    for port in WEB_ACCESS_AUTOMATIC_PORTS {
        let bind_address = format!("{host}:{port}");
        match bind_exact_web_access_address(bind_address).await {
            Ok(value) => return Ok(value),
            Err(error) => last_error = Some(error),
        }
    }
    let ports = WEB_ACCESS_AUTOMATIC_PORTS
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let detail = last_error.unwrap_or_else(|| "no port was tried".to_string());
    Err(format!(
        "web access ports are unavailable: {ports}; last error: {detail}"
    ))
}

async fn bind_exact_web_access_address(
    bind_address: String,
) -> Result<(String, SocketAddr, tokio::net::TcpListener), String> {
    let address: SocketAddr = bind_address
        .trim()
        .parse()
        .map_err(|error| format!("invalid bind address: {error}"))?;
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|error| error.to_string())?;
    let listener_address = listener.local_addr().map_err(|error| error.to_string())?;
    let resolved_bind_address = listener_address.to_string();
    Ok((resolved_bind_address, listener_address, listener))
}

fn default_web_access_bind_address(discoverable: bool) -> String {
    let host = if discoverable { "0.0.0.0" } else { "127.0.0.1" };
    format!("{host}:{}", WEB_ACCESS_AUTOMATIC_PORTS[0])
}

fn link_host_config_for_write() -> Result<CliLinkHostConfig, String> {
    Ok(
        read_link_host_config()?.unwrap_or_else(|| CliLinkHostConfig {
            web_access_enabled: false,
            discovery_enabled: false,
            port_mode: CliLinkHostPortMode::Automatic,
            bind_address: default_web_access_bind_address(false),
            token: generate_token(),
            updated_at: unix_millis(),
        }),
    )
}

fn read_link_host_config() -> Result<Option<CliLinkHostConfig>, String> {
    let path = crate::client_paths::link_host_config_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn write_link_host_config(config: &CliLinkHostConfig) -> Result<(), String> {
    let path = crate::client_paths::link_host_config_path();
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid web access config path: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let content = serde_json::to_string_pretty(config).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

fn read_link_host_state_optional() -> Result<Option<CliLinkHostState>, String> {
    let path = crate::client_paths::link_host_state_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn write_link_host_state(state: &CliLinkHostState) -> Result<(), String> {
    let path = crate::client_paths::link_host_state_path();
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid web access state path: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let content = serde_json::to_string_pretty(state).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

fn remove_link_host_state() -> Result<(), String> {
    let path = crate::client_paths::link_host_state_path();
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

fn load_accepted_sessions_from_file(
    path: &PathBuf,
) -> Result<BTreeMap<String, AcceptedRemoteSessionRecord>, String> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let content = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

fn save_accepted_session_to_file(
    path: &PathBuf,
    sessionId: String,
    record: AcceptedRemoteSessionRecord,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid accepted session path: {}", path.display()))?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let mut sessions = load_accepted_sessions_from_file(path)?;
    sessions.insert(sessionId, record);
    let content = serde_json::to_string_pretty(&sessions).map_err(|error| error.to_string())?;
    std::fs::write(path, content).map_err(|error| error.to_string())
}

fn print_web_access_usage() {
    println!("operit2 cli web open [--bind <addr:port>] [--token <token>] [--link <session>] [--web-root <path>] [--discoverable]");
    println!("operit2 cli web close");
    println!("operit2 cli web status");
    println!("operit2 cli web token rotate");
    println!("operit2 cli web token set <token>");
}
