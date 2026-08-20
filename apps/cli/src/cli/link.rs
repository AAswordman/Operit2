use super::*;
use crate::{create_cli_link_access_store, create_local_core};

use operit_core_proxy::{
    CoreNodeRouter::CoreNodeRouter, RuntimeRemoteLinkService::RuntimeRemoteLinkService,
};
use operit_link::{CoreCallRequest, CoreLinkClient, CoreObjectPath, CoreWatchRequest};
use operit_link_access::{
    link_token_hash, AcceptedRemoteSessionRecord, LinkAccessStore, PairedRemoteSession,
    PairedRemoteSessionRecord, LinkTransportPreference, RemoteDeviceInfo, RemoteLinkClient,
    RemoteLinkServer, RemoteLinkServerConfig,
};
use operit_providers::chat::enhance::ConversationService::ConversationService;
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::services::RuntimeHostInteractionService::{
    requestOwnerToolPermissionAsync, RuntimeHostInteractionToolPermissionPayload,
    RuntimeHostInteractionToolPermissionTool, RuntimeHostInteractionToolPermissionToolParameter,
};
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::tools::ToolPermissionSystem::PermissionRequestResult;
use operit_tools::ToolExecutionManager::AITool;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const LINK_SESSION_DISCOVERY_TIMEOUT_MS: u64 = 2000;

pub(crate) async fn run_link_command(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("serve") => run_link_serve_command(&args[1..]).await,
        Some("discover") => run_link_discover_command(&args[1..]).await,
        Some("connect") => run_link_connect_command(&args[1..]).await,
        Some("space") => run_link_space_command(&args[1..]).await,
        Some("hello") => run_link_hello_command(&args[1..]).await,
        Some("sessions") => run_link_sessions_command().await,
        Some("transport") => run_link_transport_command(&args[1..]).await,
        Some("session-delete") => run_link_session_delete_command(&args[1..]).await,
        Some("accepted-sessions") => run_link_accepted_sessions_command().await,
        Some("accepted-session-delete") => {
            run_link_accepted_session_delete_command(&args[1..]).await
        }
        Some("ping") => run_link_ping_command(&args[1..]).await,
        Some("refresh") => run_link_refresh_command(&args[1..]).await,
        Some("call") => run_link_call_command(&args[1..]).await,
        Some("watch") => run_link_watch_command(&args[1..]).await,
        Some("tui") => crate::tui::run_link_tui_command(&args[1..]).await,
        Some("run") => run_link_run_command(&args[1..]).await,
        _ => {
            print_link_usage();
            Ok(())
        }
    }
}

async fn run_link_run_command(args: &[String]) -> Result<(), String> {
    let session_name = args
        .get(0)
        .ok_or_else(|| "usage: operit2 cli link run <session> <command>".to_string())?;
    super::run_cli_link_root(session_name, &args[1..]).await
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
                    .ok_or_else(|| {
                        "usage: operit2 cli link serve [--bind <addr:port>] [--token <token>]"
                            .to_string()
                    })?
                    .clone();
            }
            "--token" => {
                index += 1;
                token = args
                    .get(index)
                    .ok_or_else(|| {
                        "usage: operit2 cli link serve [--bind <addr:port>] [--token <token>]"
                            .to_string()
                    })?
                    .clone();
            }
            _ => {
                return Err(
                    "usage: operit2 cli link serve [--bind <addr:port>] [--token <token>]"
                        .to_string(),
                );
            }
        }
        index += 1;
    }
    let mut core = create_local_core();
    core.localApplicationMut().onCreate()?;
    {
        let application = core.localApplicationMut();
        let enhanced_ai_service = EnhancedAIService::new(
            application.toolHandler.clone(),
            application.providerRuntimeContext.clone(),
        );
        let mut holder = application
            .chatRuntimeHolder
            .try_lock()
            .map_err(|_| "Chat runtime holder is busy".to_string())?;
        holder.getCore(ChatRuntimeSlot::MAIN).enhancedAiService = Some(enhanced_ai_service);
    }
    install_link_permission_requester(&mut core);
    let device_info = RemoteDeviceInfo::nativeCli("server")?;
    let access_store = LinkAccessStore::new(core.runtimeStorageHost());
    let identity = access_store.initializeIdentity(device_info.clone())?;
    RuntimeRemoteLinkService::new(core.clone()).startSpaceSync()?;
    RemoteLinkServer::serve(
        CoreNodeRouter::new(Arc::new(core)),
        RemoteLinkServerConfig {
            bindAddress: bind_address,
            token,
            localControlToken: None,
            deviceId: identity.deviceId,
            deviceInfo: identity.deviceInfo,
            webAccess: None,
            printStartupInfo: true,
            accessStore: access_store,
        },
    )
    .await
}

pub(crate) fn install_link_permission_requester(core: &mut operit_core_proxy::LocalCoreProxy) {
    let handler = core.localApplicationMut().toolHandler.clone();
    handler
        .getToolPermissionSystem()
        .setAsyncPermissionRequester(move |tool, description| async move {
            let response = requestOwnerToolPermissionAsync(
                RuntimeHostInteractionToolPermissionPayload {
                    tool: tool_to_permission_payload(&tool),
                    description,
                },
                Duration::from_secs(60),
            )
            .await
            .expect("permission request failed");
            match response.result.as_str() {
                "allow" => PermissionRequestResult::ALLOW,
                "always_allow" => PermissionRequestResult::ALLOW_SESSION,
                "deny" => PermissionRequestResult::DENY,
                other => panic!("unknown permission response result: {other}"),
            }
        });
}

fn tool_to_permission_payload(tool: &AITool) -> RuntimeHostInteractionToolPermissionTool {
    RuntimeHostInteractionToolPermissionTool {
        name: tool.name.clone(),
        parameters: tool
            .parameters
            .iter()
            .map(
                |parameter| RuntimeHostInteractionToolPermissionToolParameter {
                    name: parameter.name.clone(),
                    value: parameter.value.clone(),
                },
            )
            .collect(),
    }
}

async fn run_link_hello_command(args: &[String]) -> Result<(), String> {
    let (url, token) =
        parse_remote_url_token(args, "usage: operit2 cli link hello <url> --token <token>")?;
    let client = RemoteLinkClient::new(url);
    let token_hash = link_token_hash(&token);
    let hello = client.hello(&token_hash).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&hello).map_err(|error| error.to_string())?
    );
    Ok(())
}

/// Discovers nearby Spaces and prints their directly connectable CoreNodes.
async fn run_link_discover_command(args: &[String]) -> Result<(), String> {
    let mut timeout_ms = 2000_u64;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--timeout-ms" => {
                index += 1;
                timeout_ms = args
                    .get(index)
                    .ok_or_else(|| {
                        "usage: operit2 cli link discover [--timeout-ms <ms>]".to_string()
                    })?
                    .parse::<u64>()
                    .map_err(|error| error.to_string())?;
            }
            _ => {
                return Err("usage: operit2 cli link discover [--timeout-ms <ms>]".to_string());
            }
        }
        index += 1;
    }
    let spaces = RuntimeRemoteLinkService::new(create_local_core())
        .discoverSpaces(timeout_ms)
        .await?;
    for space in spaces {
        println!(
            "device space={} id={} devices={}",
            space.spaceName, space.spaceId, space.memberCount
        );
        for device in space.devices {
            println!(
                "  device={} id={} address={}",
                device.displayName, device.deviceId, device.baseUrl
            );
        }
    }
    Ok(())
}

async fn run_link_connect_command(args: &[String]) -> Result<(), String> {
    const USAGE: &str = "usage: operit2 cli link connect <url> --token <token> --save <name> [--transport <http|ws>]";
    let (url, token, save_name, transport) = parse_remote_url_token_save(args, USAGE)?;
    let name = save_name.ok_or_else(|| USAGE.to_string())?;
    let token_hash = link_token_hash(&token);
    let service = RuntimeRemoteLinkService::new(create_local_core());
    let pairing = service
        .startPairedRemote(url, token_hash, RemoteDeviceInfo::nativeCli("client")?)
        .await?;
    println!(
        "device={} id={}",
        pairing.coreDeviceInfo.displayName(),
        pairing.coreDeviceId
    );
    println!("pairing started: {}", pairing.pairingId);
    println!("check the server terminal for pairing code");
    print!("pairing code> ");
    io::stdout().flush().map_err(|error| error.to_string())?;
    let mut code = String::new();
    io::stdin()
        .read_line(&mut code)
        .map_err(|error| error.to_string())?;
    let mut session = service
        .finishPairedRemote(pairing.pairingId, code.trim().to_string(), name.clone())
        .await?;
    if let Some(transport) = transport {
        session.transport = transport;
        create_cli_link_access_store().saveOutboundSession(name.clone(), session.clone())?;
    }
    println!(
        "paired device={} deviceId={} localDeviceId={}",
        session.remoteDeviceInfo.displayName(),
        session.coreDeviceId,
        session.deviceId
    );
    println!("device paired: {name}");
    println!("join its device space with: operit2 cli link space join {name}");
    Ok(())
}

/// Runs user-facing device-space inspection and membership commands.
async fn run_link_space_command(args: &[String]) -> Result<(), String> {
    let core = create_local_core();
    LinkAccessStore::new(core.runtimeStorageHost())
        .initializeIdentity(RemoteDeviceInfo::nativeCli("client")?)?;
    let service = RuntimeRemoteLinkService::new(core);
    match args.first().map(String::as_str) {
        None | Some("show") if args.len() <= 1 => {
            let space = service.deviceSpace()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&space).map_err(|error| error.to_string())?
            );
            Ok(())
        }
        Some("rename") if args.len() == 2 => {
            let space = service.renameDeviceSpace(args[1].clone())?;
            println!("device space renamed: {}", space.spaceName);
            Ok(())
        }
        Some("join") if args.len() == 2 => {
            let space = service.joinPairedDeviceSpace(args[1].clone()).await?;
            println!(
                "joined device space: {} ({} devices)",
                space.spaceName,
                space.members.len()
            );
            Ok(())
        }
        Some("leave") if args.len() == 1 => {
            let space = service.leaveDeviceSpace()?;
            println!("left device space; current space: {}", space.spaceName);
            Ok(())
        }
        _ => Err(
            "usage: operit2 cli link space <show|rename <name>|join <paired-session>|leave>"
                .to_string(),
        ),
    }
}

async fn run_link_sessions_command() -> Result<(), String> {
    let sessions = load_link_sessions()?;
    for (name, session) in sessions {
        println!(
            "{}\t{}\t{}\t{}",
            name,
            session.remoteDeviceInfo.displayName(),
            session.baseUrl,
            session.coreDeviceId
        );
        println!("  transport={}", link_transport_name(&session.transport));
    }
    Ok(())
}

/// Changes the concrete carrier used by one saved paired session.
async fn run_link_transport_command(args: &[String]) -> Result<(), String> {
    if args.len() != 2 {
        return Err("usage: operit2 cli link transport <session> <http|ws>".to_string());
    }
    let name = &args[0];
    let mut record = load_link_session_record(name)?;
    record.transport = parse_link_transport(&args[1])?;
    create_cli_link_access_store().saveOutboundSession(name.clone(), record.clone())?;
    println!("session transport updated: {}", link_transport_name(&record.transport));
    Ok(())
}

async fn run_link_session_delete_command(args: &[String]) -> Result<(), String> {
    let name = args
        .get(0)
        .ok_or_else(|| "usage: operit2 cli link session-delete <name>".to_string())?;
    create_cli_link_access_store().removeOutboundSession(name)?;
    println!("session deleted: {name}");
    Ok(())
}

async fn run_link_accepted_sessions_command() -> Result<(), String> {
    let sessions = load_link_server_sessions()?;
    for (session_id, session) in sessions {
        println!(
            "{}\t{}\t{}",
            session_id,
            session.deviceInfo.displayName(),
            session.deviceId
        );
    }
    Ok(())
}

async fn run_link_accepted_session_delete_command(args: &[String]) -> Result<(), String> {
    let session_id = args.get(0).ok_or_else(|| {
        "usage: operit2 cli link accepted-session-delete <session-id>".to_string()
    })?;
    remove_link_server_session(session_id)?;
    println!("accepted session deleted: {session_id}");
    Ok(())
}

async fn run_link_ping_command(args: &[String]) -> Result<(), String> {
    let name = args
        .get(0)
        .ok_or_else(|| "usage: operit2 cli link ping <name>".to_string())?;
    let session = load_link_session_resolved(name).await?;
    let info = session.sessionInfo().await?;
    println!(
        "session active remote={} core={} client={} transports={}",
        info.coreDeviceInfo.displayName(),
        info.coreDeviceId,
        info.clientDeviceId,
        info.transports.join(",")
    );
    Ok(())
}

/// Refreshes saved paired session URLs from current LAN discovery data.
async fn run_link_refresh_command(args: &[String]) -> Result<(), String> {
    let (target_name, timeout_ms) = parse_link_refresh_args(args)?;
    let devices = crate::mdns::discover_devices(timeout_ms)?;
    let mut sessions = load_link_sessions()?;
    let mut updated_count = 0usize;
    match target_name {
        Some(name) => {
            let record = sessions
                .get(&name)
                .ok_or_else(|| format!("link session not found: {name}"))?
                .clone();
            let (updated, changed) =
                refresh_link_session_record_from_devices(&name, record, &devices).await?;
            if changed {
                updated_count += 1;
            }
            sessions.insert(name, updated);
        }
        None => {
            let names = sessions.keys().cloned().collect::<Vec<_>>();
            for name in names {
                let record = sessions
                    .get(&name)
                    .ok_or_else(|| format!("link session not found while refreshing: {name}"))?
                    .clone();
                let (updated, changed) =
                    refresh_link_session_record_from_devices(&name, record, &devices).await?;
                if changed {
                    updated_count += 1;
                }
                sessions.insert(name, updated);
            }
        }
    }
    write_link_sessions(sessions)?;
    println!("sessions refreshed: updated={updated_count}");
    Ok(())
}

async fn run_link_call_command(args: &[String]) -> Result<(), String> {
    let name = args.get(0).ok_or_else(|| {
        "usage: operit2 cli link call <session> <target-path> <method-name> [args-json]".to_string()
    })?;
    let target_path = args.get(1).ok_or_else(|| {
        "usage: operit2 cli link call <session> <target-path> <method-name> [args-json]".to_string()
    })?;
    let method_name = args.get(2).ok_or_else(|| {
        "usage: operit2 cli link call <session> <target-path> <method-name> [args-json]".to_string()
    })?;
    let args_json = parse_link_args_json(args.get(3))?;
    let session = load_link_session_resolved(name).await?;
    let response = session
        .call(CoreCallRequest::new(
            link_request_id(),
            CoreObjectPath::parse(target_path),
            method_name.clone(),
            operit_link::toCoreValue(args_json).map_err(|error| error.to_string())?,
        ))
        .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&response).map_err(|error| error.to_string())?
    );
    Ok(())
}

async fn run_link_watch_command(args: &[String]) -> Result<(), String> {
    let name = args.get(0).ok_or_else(|| {
        "usage: operit2 cli link watch <session> <target-path> <property-name> [args-json]"
            .to_string()
    })?;
    let target_path = args.get(1).ok_or_else(|| {
        "usage: operit2 cli link watch <session> <target-path> <property-name> [args-json]"
            .to_string()
    })?;
    let property_name = args.get(2).ok_or_else(|| {
        "usage: operit2 cli link watch <session> <target-path> <property-name> [args-json]"
            .to_string()
    })?;
    let args_json = parse_link_args_json(args.get(3))?;
    let mut session = load_link_session_resolved(name).await?;
    let event = operit_link::CoreLinkClient::watchSnapshot(
        &mut session,
        CoreWatchRequest::new(
            link_request_id(),
            CoreObjectPath::parse(target_path),
            property_name.clone(),
            operit_link::toCoreValue(args_json).map_err(|error| error.to_string())?,
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

/// Parses the optional session name and discovery timeout for link refresh.
fn parse_link_refresh_args(args: &[String]) -> Result<(Option<String>, u64), String> {
    let usage = "usage: operit2 cli link refresh [session] [--timeout-ms <ms>]";
    let mut session_name = None::<String>;
    let mut timeout_ms = LINK_SESSION_DISCOVERY_TIMEOUT_MS;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--timeout-ms" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| usage.to_string())?;
                timeout_ms = value.parse::<u64>().map_err(|error| error.to_string())?;
            }
            value => {
                if session_name.is_some() {
                    return Err(usage.to_string());
                }
                session_name = Some(value.to_string());
            }
        }
        index += 1;
    }
    Ok((session_name, timeout_ms))
}

pub(crate) async fn call_application<C>(
    client: &mut C,
    method_name: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, String>
where
    C: CoreLinkClient + Send,
{
    let response = client
        .call(CoreCallRequest::new(
            link_request_id(),
            CoreObjectPath::parse("application"),
            method_name.to_string(),
            operit_link::toCoreValue(args).map_err(|error| error.to_string())?,
        ))
        .await;
    response
        .result
        .map_err(|error| error.to_string())
        .and_then(|value| operit_link::fromCoreValue(value).map_err(|error| error.to_string()))
}

fn parse_remote_url_token(args: &[String], usage: &str) -> Result<(String, String), String> {
    let (url, token, _, _) = parse_remote_url_token_save(args, usage)?;
    Ok((url, token))
}

fn parse_remote_url_token_save(
    args: &[String],
    usage: &str,
) -> Result<(String, String, Option<String>, Option<LinkTransportPreference>), String> {
    let url = args.get(0).ok_or_else(|| usage.to_string())?.clone();
    let mut token = None::<String>;
    let mut save_name = None::<String>;
    let mut transport = None::<LinkTransportPreference>;
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
            "--transport" => {
                index += 1;
                transport = Some(parse_link_transport(
                    args.get(index).ok_or_else(|| usage.to_string())?,
                )?);
            }
            _ => return Err(usage.to_string()),
        }
        index += 1;
    }
    Ok((
        url,
        token.ok_or_else(|| usage.to_string())?,
        save_name,
        transport,
    ))
}

/// Parses the explicit Link carrier selection accepted by the CLI.
fn parse_link_transport(value: &str) -> Result<LinkTransportPreference, String> {
    match value {
        "http" => Ok(LinkTransportPreference::Http),
        "ws" => Ok(LinkTransportPreference::WebSocket),
        _ => Err("Link transport must be http or ws".to_string()),
    }
}

/// Returns the stable CLI spelling for one Link carrier selection.
fn link_transport_name(value: &LinkTransportPreference) -> &'static str {
    match value {
        LinkTransportPreference::Http => "http",
        LinkTransportPreference::WebSocket => "ws",
    }
}

/// Loads all saved paired session records.
fn load_link_sessions() -> Result<BTreeMap<String, PairedRemoteSessionRecord>, String> {
    create_cli_link_access_store().outboundSessions()
}

/// Loads one saved paired session record by name.
fn load_link_session_record(name: &str) -> Result<PairedRemoteSessionRecord, String> {
    let sessions = load_link_sessions()?;
    sessions
        .get(name)
        .ok_or_else(|| format!("link session not found: {name}"))
        .cloned()
}

/// Loads one paired session after applying verified LAN endpoint discovery.
pub(crate) async fn load_link_session_resolved(name: &str) -> Result<PairedRemoteSession, String> {
    let record = load_link_session_record(name)?;
    let devices = crate::mdns::discover_devices(LINK_SESSION_DISCOVERY_TIMEOUT_MS)?;
    let (record, changed) =
        refresh_link_session_record_from_devices(name, record, &devices).await?;
    if changed {
        save_link_session(name, record.clone())?;
    }
    PairedRemoteSession::fromRecord(record)
}

/// Updates one paired session record when discovery advertises the same core device.
async fn refresh_link_session_record_from_devices(
    name: &str,
    record: PairedRemoteSessionRecord,
    devices: &[crate::mdns::DiscoveredDevice],
) -> Result<(PairedRemoteSessionRecord, bool), String> {
    let Some(device) = discovered_device_for_link_record(&record, devices) else {
        return Ok((record, false));
    };
    let updated = record.withBaseUrl(device.base_url.clone());
    if updated.baseUrl == record.baseUrl {
        return Ok((record, false));
    }
    verify_link_session_record(&updated).await?;
    eprintln!("session address updated: {name} {}", updated.baseUrl);
    Ok((updated, true))
}

/// Selects the discovered device whose identity matches a paired session record.
fn discovered_device_for_link_record<'a>(
    record: &PairedRemoteSessionRecord,
    devices: &'a [crate::mdns::DiscoveredDevice],
) -> Option<&'a crate::mdns::DiscoveredDevice> {
    devices
        .iter()
        .find(|device| device.device_id == record.coreDeviceId)
}

/// Verifies a paired session record against its configured endpoint.
async fn verify_link_session_record(record: &PairedRemoteSessionRecord) -> Result<(), String> {
    let session = PairedRemoteSession::fromRecord(record.clone())?;
    let info = session.sessionInfo().await?;
    if info.protocolVersion != 3 {
        return Err(format!(
            "remote Link protocol version is {}, expected 3",
            info.protocolVersion
        ));
    }
    if info.coreDeviceId != record.coreDeviceId {
        return Err("remote runtime identity changed".to_string());
    }
    Ok(())
}

pub(crate) fn parse_link_args_json(value: Option<&String>) -> Result<serde_json::Value, String> {
    match value {
        Some(value) => serde_json::from_str(value).map_err(|error| error.to_string()),
        None => Ok(serde_json::json!({})),
    }
}

pub(crate) fn link_request_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after UNIX_EPOCH")
        .as_millis();
    format!("cli-{millis}")
}

/// Saves one paired session record by name.
fn save_link_session(name: &str, record: PairedRemoteSessionRecord) -> Result<(), String> {
    let mut sessions = load_link_sessions()?;
    sessions.insert(name.to_string(), record);
    write_link_sessions(sessions)
}

/// Writes the complete paired session map to disk.
fn write_link_sessions(
    sessions: BTreeMap<String, PairedRemoteSessionRecord>,
) -> Result<(), String> {
    let store = create_cli_link_access_store();
    for (name, record) in sessions {
        store.saveOutboundSession(name, record)?;
    }
    Ok(())
}

fn load_link_server_sessions() -> Result<BTreeMap<String, AcceptedRemoteSessionRecord>, String> {
    create_cli_link_access_store().inboundSessions()
}

fn save_link_server_session(
    session_id: String,
    record: AcceptedRemoteSessionRecord,
) -> Result<(), String> {
    create_cli_link_access_store().saveInboundSession(session_id, record)
}

fn remove_link_server_session(session_id: &str) -> Result<(), String> {
    if !load_link_server_sessions()?.contains_key(session_id) {
        return Err(format!("accepted link session not found: {session_id}"));
    }
    create_cli_link_access_store().removeInboundSession(session_id)
}

fn print_link_usage() {
    println!("operit2 cli link serve [--bind <addr:port>] [--token <token>]");
    println!("operit2 cli link discover [--timeout-ms <ms>]");
    println!("operit2 cli link hello <url> --token <token>");
    println!("operit2 cli link connect <url> --token <token> --save <name> [--transport <http|ws>]");
    println!("operit2 cli link space <show|rename <name>|join <paired-session>|leave>");
    println!("operit2 cli link sessions");
    println!("operit2 cli link transport <session> <http|ws>");
    println!("operit2 cli link session-delete <name>");
    println!("operit2 cli link accepted-sessions");
    println!("operit2 cli link accepted-session-delete <session-id>");
    println!("operit2 cli link ping <name>");
    println!("operit2 cli link refresh [session] [--timeout-ms <ms>]");
    println!("operit2 cli link call <session> <target-path> <method-name> [args-json]");
    println!("operit2 cli link watch <session> <target-path> <property-name> [args-json]");
    println!("operit2 cli link tui <session> [--chat <chat-id>]");
    println!("operit2 cli link run <session> <version|chat|local-models|stt>");
}
