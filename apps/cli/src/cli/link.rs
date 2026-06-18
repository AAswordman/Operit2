use super::*;
use crate::create_local_core;

use crate::access::{
    link_token_hash, AcceptedRemoteSessionLoader, AcceptedRemoteSessionRecord,
    AcceptedRemoteSessionStore, PairedRemoteSession, PairedRemoteSessionRecord, RemoteDeviceInfo,
    RemoteHostInteractionBroker, RemoteLinkClient, RemoteLinkServer, RemoteLinkServerConfig,
};
use operit_link::{CoreCallRequest, CoreLinkClient, CoreObjectPath, CoreWatchRequest};
use operit_runtime::api::chat::enhance::ConversationService::ConversationService;
use operit_runtime::api::chat::enhance::ToolExecutionManager::AITool;
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::api::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::tools::AIToolHandler::AIToolHandler;
use operit_runtime::core::tools::ToolPermissionSystem::PermissionRequestResult;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub(crate) async fn run_link_command(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("serve") => run_link_serve_command(&args[1..]).await,
        Some("discover") => run_link_discover_command(&args[1..]).await,
        Some("connect") => run_link_connect_command(&args[1..]).await,
        Some("hello") => run_link_hello_command(&args[1..]).await,
        Some("sessions") => run_link_sessions_command().await,
        Some("session-delete") => run_link_session_delete_command(&args[1..]).await,
        Some("accepted-sessions") => run_link_accepted_sessions_command().await,
        Some("accepted-session-delete") => {
            run_link_accepted_session_delete_command(&args[1..]).await
        }
        Some("ping") => run_link_ping_command(&args[1..]).await,
        Some("sync") => run_link_sync_command(&args[1..]).await,
        Some("sync-status") => run_link_sync_status_command(&args[1..]).await,
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
    let _externalRuntimeEventRegistration =
        operit_runtime::core::application::ExternalRuntimeEventSupport::startExternalRuntimeEventSupport(
            core.localApplicationMut().applicationContext.clone(),
            "cli-link-serve",
        )?;
    let main_core = core
        .localApplicationMut()
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN);
    main_core.enhancedAiService = Some(EnhancedAIService::new(ConversationService));
    let host_interaction_broker = RemoteHostInteractionBroker::new();
    install_remote_host_permission_requester(&mut core, host_interaction_broker.clone());
    let accepted_sessions = load_link_server_sessions()?;
    let accepted_session_loader: AcceptedRemoteSessionLoader = Arc::new(load_link_server_sessions);
    let accepted_session_store: AcceptedRemoteSessionStore = Arc::new(save_link_server_session);
    let device_info = RemoteDeviceInfo::nativeCli("server")?;
    let device_id = load_link_host_device_id()?;
    RemoteLinkServer::serve(
        core,
        RemoteLinkServerConfig {
            bindAddress: bind_address,
            token,
            deviceId: device_id,
            deviceInfo: device_info,
            hostInteractionBroker: Some(host_interaction_broker),
            webAccess: None,
            printStartupInfo: true,
            acceptedSessions: accepted_sessions,
            acceptedSessionLoader: Some(accepted_session_loader),
            acceptedSessionStore: Some(accepted_session_store),
            pairingCodeSink: None,
        },
    )
    .await
}

pub(crate) fn load_link_host_device_id() -> Result<String, String> {
    let path = crate::client_paths::link_host_device_id_path();
    if path.exists() {
        let content = fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let device_id = content.trim().to_string();
        if device_id.is_empty() {
            return Err(format!("empty link host device id: {}", path.display()));
        }
        return Ok(device_id);
    }
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid link host device id path: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let device_id = format!("core-{}", Uuid::new_v4());
    fs::write(&path, device_id.as_bytes()).map_err(|error| error.to_string())?;
    Ok(device_id)
}

pub(crate) fn install_remote_host_permission_requester(
    core: &mut operit_core_proxy::LocalCoreProxy,
    broker: RemoteHostInteractionBroker,
) {
    let context = core.localApplicationMut().applicationContext.clone();
    let handler = AIToolHandler::getInstance(context);
    handler
        .getToolPermissionSystem()
        .setPermissionRequester(move |tool, description| {
            let response = broker.requestInteraction(
                "tool_permission",
                serde_json::json!({
                    "tool": tool_to_json(tool),
                    "description": description,
                }),
                std::time::Duration::from_secs(60),
            );
            match response
                .as_ref()
                .and_then(|value| value.get("result"))
                .and_then(|value| value.as_str())
            {
                Some("allow") => PermissionRequestResult::ALLOW,
                Some("always_allow") => PermissionRequestResult::ALWAYS_ALLOW,
                _ => PermissionRequestResult::DENY,
            }
        });
}

fn tool_to_json(tool: &AITool) -> serde_json::Value {
    serde_json::json!({
        "name": &tool.name,
        "parameters": tool.parameters.iter().map(|parameter| {
            serde_json::json!({
                "name": &parameter.name,
                "value": &parameter.value,
            })
        }).collect::<Vec<_>>(),
    })
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
    let devices = crate::mdns::discover_devices(timeout_ms)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&devices).map_err(|error| error.to_string())?
    );
    Ok(())
}

async fn run_link_connect_command(args: &[String]) -> Result<(), String> {
    let (url, token, save_name) = parse_remote_url_token_save(
        args,
        "usage: operit2 cli link connect <url> --token <token> [--save <name>]",
    )?;
    let client = RemoteLinkClient::new(url);
    let token_hash = link_token_hash(&token);
    let hello = client.hello(&token_hash).await?;
    println!(
        "remote device={} core={} transports={}",
        hello.coreDeviceInfo.displayName(),
        hello.coreDeviceId,
        hello.transports.join(",")
    );
    let pair_state = client
        .pairStart(&token_hash, RemoteDeviceInfo::nativeCli("client")?)
        .await?;
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
        "session active remote={} core={} client={} transports={}",
        info.coreDeviceInfo.displayName(),
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
        println!(
            "{}\t{}\t{}\t{}",
            name,
            session.remoteDeviceInfo.displayName(),
            session.baseUrl,
            session.coreDeviceId
        );
    }
    Ok(())
}

async fn run_link_session_delete_command(args: &[String]) -> Result<(), String> {
    let name = args
        .get(0)
        .ok_or_else(|| "usage: operit2 cli link session-delete <name>".to_string())?;
    remove_link_session(name)?;
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
    let sessions = load_link_sessions()?;
    let record = sessions
        .get(name)
        .ok_or_else(|| format!("link session not found: {name}"))?
        .clone();
    let session = PairedRemoteSession::fromRecord(record)?;
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

async fn run_link_sync_command(args: &[String]) -> Result<(), String> {
    let (session_name, limit) = parse_link_sync_args(args)?;
    let mut local = create_local_core();
    let mut remote = load_link_session(&session_name)?;
    assert_sync_core_versions_match(&mut local, &mut remote).await?;
    let mut rounds = 0usize;
    let mut localApplied = 0usize;
    let mut remoteApplied = 0usize;
    loop {
        rounds += 1;
        let localClock = call_application(&mut local, "syncClock", serde_json::json!({})).await?;
        let remoteClock = call_application(&mut remote, "syncClock", serde_json::json!({})).await?;
        let localOperations = call_application(
            &mut local,
            "syncOperationsSince",
            serde_json::json!({
                "clock": remoteClock,
                "domains": ["preferences", "chat", "objectbox"],
                "limit": limit,
            }),
        )
        .await?;
        let remoteOperations = call_application(
            &mut remote,
            "syncOperationsSince",
            serde_json::json!({
                "clock": localClock,
                "domains": ["preferences", "chat", "objectbox"],
                "limit": limit,
            }),
        )
        .await?;
        let mergedOperations = merge_sync_operations(localOperations, remoteOperations)?;
        let count = sync_operation_count(&mergedOperations)?;
        if count == 0 {
            break;
        }
        let remoteResult = call_application(
            &mut remote,
            "syncApplyOperations",
            serde_json::json!({
                "operations": mergedOperations.clone(),
            }),
        )
        .await?;
        let localResult = call_application(
            &mut local,
            "syncApplyOperations",
            serde_json::json!({
                "operations": mergedOperations,
            }),
        )
        .await?;
        remoteApplied += applied_count(&remoteResult)?;
        localApplied += applied_count(&localResult)?;
        if count < limit {
            break;
        }
    }
    println!(
        "sync completed: rounds={rounds}, local_applied={localApplied}, remote_applied={remoteApplied}"
    );
    Ok(())
}

async fn run_link_sync_status_command(args: &[String]) -> Result<(), String> {
    let (session_name, limit) = parse_link_sync_status_args(args)?;
    let mut local = create_local_core();
    let mut remote = load_link_session(&session_name)?;
    let local_version = call_application_core_version(&mut local).await?;
    let remote_version = call_application_core_version(&mut remote).await?;
    println!("localVersion={local_version}");
    println!("remoteVersion={remote_version}");
    println!("versionsMatch={}", local_version == remote_version);

    let localClock = call_application(&mut local, "syncClock", serde_json::json!({})).await?;
    let remoteClock = call_application(&mut remote, "syncClock", serde_json::json!({})).await?;
    let localOperations = call_application(
        &mut local,
        "syncOperationsSince",
        serde_json::json!({
            "clock": remoteClock,
            "domains": ["preferences", "chat", "objectbox"],
            "limit": limit,
        }),
    )
    .await?;
    let remoteOperations = call_application(
        &mut remote,
        "syncOperationsSince",
        serde_json::json!({
            "clock": localClock,
            "domains": ["preferences", "chat", "objectbox"],
            "limit": limit,
        }),
    )
    .await?;
    println!("localPending={}", sync_operation_count(&localOperations)?);
    println!("remotePending={}", sync_operation_count(&remoteOperations)?);
    println!(
        "mergedPending={}",
        sync_operation_count(&merge_sync_operations(localOperations, remoteOperations)?)?
    );
    Ok(())
}

async fn assert_sync_core_versions_match<L, R>(local: &mut L, remote: &mut R) -> Result<(), String>
where
    L: CoreLinkClient + Send,
    R: CoreLinkClient + Send,
{
    let local_version = call_application_core_version(local).await?;
    let remote_version = call_application_core_version(remote).await?;
    if local_version != remote_version {
        return Err(format!(
            "core version mismatch: local={local_version}, remote={remote_version}. sync blocked"
        ));
    }
    Ok(())
}

async fn call_application_core_version<C>(client: &mut C) -> Result<String, String>
where
    C: CoreLinkClient + Send,
{
    let value = call_application(client, "coreVersion", serde_json::json!({})).await?;
    value
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| "coreVersion response must be a string".to_string())
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

fn parse_link_sync_args(args: &[String]) -> Result<(String, usize), String> {
    let session = args
        .get(0)
        .ok_or_else(|| "usage: operit2 cli link sync <session> [--limit <n>]".to_string())?
        .clone();
    let mut limit = 512usize;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--limit" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    "usage: operit2 cli link sync <session> [--limit <n>]".to_string()
                })?;
                limit = value.parse::<usize>().map_err(|error| error.to_string())?;
            }
            _ => return Err("usage: operit2 cli link sync <session> [--limit <n>]".to_string()),
        }
        index += 1;
    }
    if limit == 0 {
        return Err("sync limit must be greater than 0".to_string());
    }
    Ok((session, limit))
}

fn parse_link_sync_status_args(args: &[String]) -> Result<(String, usize), String> {
    let usage = "usage: operit2 cli link sync-status <session> [--limit <n>]";
    let session = args.get(0).ok_or_else(|| usage.to_string())?.clone();
    let mut limit = 512usize;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--limit" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| usage.to_string())?;
                limit = value.parse::<usize>().map_err(|error| error.to_string())?;
            }
            _ => return Err(usage.to_string()),
        }
        index += 1;
    }
    if limit == 0 {
        return Err("sync status limit must be greater than 0".to_string());
    }
    Ok((session, limit))
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
            args,
        ))
        .await;
    response.result.map_err(|error| error.to_string())
}

fn merge_sync_operations(
    left: serde_json::Value,
    right: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut byId = BTreeMap::<String, serde_json::Value>::new();
    for value in sync_operation_array(left)?
        .into_iter()
        .chain(sync_operation_array(right)?)
    {
        let opId = value
            .get("opId")
            .and_then(|value| value.as_str())
            .ok_or_else(|| "sync operation missing opId".to_string())?
            .to_string();
        byId.insert(opId, value);
    }
    let mut operations = byId
        .into_values()
        .map(|value| sync_sort_key(&value).map(|key| (key, value)))
        .collect::<Result<Vec<_>, _>>()?;
    operations.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(serde_json::Value::Array(
        operations.into_iter().map(|(_, value)| value).collect(),
    ))
}

fn sync_operation_array(value: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    match value {
        serde_json::Value::Array(values) => Ok(values),
        _ => Err("sync operations response must be an array".to_string()),
    }
}

fn sync_sort_key(value: &serde_json::Value) -> Result<(i64, String, i64, String), String> {
    let createdAt = value
        .get("createdAt")
        .and_then(|value| value.as_i64())
        .ok_or_else(|| "sync operation missing createdAt".to_string())?;
    let originDeviceId = value
        .get("originDeviceId")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "sync operation missing originDeviceId".to_string())?
        .to_string();
    let sequence = value
        .get("sequence")
        .and_then(|value| value.as_i64())
        .ok_or_else(|| "sync operation missing sequence".to_string())?;
    let opId = value
        .get("opId")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "sync operation missing opId".to_string())?
        .to_string();
    Ok((createdAt, originDeviceId, sequence, opId))
}

fn sync_operation_count(value: &serde_json::Value) -> Result<usize, String> {
    value
        .as_array()
        .map(Vec::len)
        .ok_or_else(|| "sync operations must be an array".to_string())
}

fn applied_count(value: &serde_json::Value) -> Result<usize, String> {
    value
        .get("applied")
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .ok_or_else(|| "sync apply response missing applied".to_string())
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

fn load_link_sessions() -> Result<BTreeMap<String, PairedRemoteSessionRecord>, String> {
    let path = crate::client_paths::link_sessions_path();
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let content = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

pub(crate) fn load_link_session(name: &str) -> Result<PairedRemoteSession, String> {
    let sessions = load_link_sessions()?;
    let record = sessions
        .get(name)
        .ok_or_else(|| format!("link session not found: {name}"))?
        .clone();
    PairedRemoteSession::fromRecord(record)
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

fn save_link_session(name: &str, record: PairedRemoteSessionRecord) -> Result<(), String> {
    let path = crate::client_paths::link_sessions_path();
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid link session path: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let mut sessions = load_link_sessions()?;
    sessions.insert(name.to_string(), record);
    let content = serde_json::to_string_pretty(&sessions).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

fn remove_link_session(name: &str) -> Result<(), String> {
    let path = crate::client_paths::link_sessions_path();
    let mut sessions = load_link_sessions()?;
    if sessions.remove(name).is_none() {
        return Err(format!("link session not found: {name}"));
    }
    let content = serde_json::to_string_pretty(&sessions).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

fn load_link_server_sessions() -> Result<BTreeMap<String, AcceptedRemoteSessionRecord>, String> {
    let path = crate::client_paths::link_server_sessions_path();
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let content = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

fn save_link_server_session(
    session_id: String,
    record: AcceptedRemoteSessionRecord,
) -> Result<(), String> {
    let path = crate::client_paths::link_server_sessions_path();
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid link server session path: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let mut sessions = load_link_server_sessions()?;
    sessions.insert(session_id, record);
    let content = serde_json::to_string_pretty(&sessions).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

fn remove_link_server_session(session_id: &str) -> Result<(), String> {
    let path = crate::client_paths::link_server_sessions_path();
    let mut sessions = load_link_server_sessions()?;
    if sessions.remove(session_id).is_none() {
        return Err(format!("accepted link session not found: {session_id}"));
    }
    let content = serde_json::to_string_pretty(&sessions).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

fn print_link_usage() {
    println!("operit2 cli link serve [--bind <addr:port>] [--token <token>]");
    println!("operit2 cli link discover [--timeout-ms <ms>]");
    println!("operit2 cli link hello <url> --token <token>");
    println!("operit2 cli link connect <url> --token <token> [--save <name>]");
    println!("operit2 cli link sessions");
    println!("operit2 cli link session-delete <name>");
    println!("operit2 cli link accepted-sessions");
    println!("operit2 cli link accepted-session-delete <session-id>");
    println!("operit2 cli link ping <name>");
    println!("operit2 cli link sync <session> [--limit <n>]");
    println!("operit2 cli link sync-status <session> [--limit <n>]");
    println!("operit2 cli link call <session> <target-path> <method-name> [args-json]");
    println!("operit2 cli link watch <session> <target-path> <property-name> [args-json]");
    println!("operit2 cli link tui <session> [--chat <chat-id>]");
    println!("operit2 cli link run <session> <version|chat>");
}
