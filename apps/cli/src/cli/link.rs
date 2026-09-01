use super::*;
use crate::{
    create_cli_core_application, create_cli_core_application_configured,
    create_cli_core_application_without_space_sync,
};

use operit_access_runtime::{
    link_token_hash, AcceptedRemoteSessionRecord, LinkAccessStore, LinkTransportPreference,
    PairedRemoteSession, PairedRemoteSessionRecord, RemoteDeviceInfo, RemoteLinkClient,
};
use operit_core_application::CoreRemoteLinkServerConfig;
use operit_link::{
    CoreEvent, CoreEventKind, CoreEventStream, CoreLinkSharedClient, CoreStreamDescriptor,
    CoreValue, CoreWatchRequest, CORE_STREAM_POOL_OBJECT_ID,
};
use operit_model::PromptTurn::PromptTurn;
use operit_providers::chat::enhance::ConversationService::ConversationService;
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::services::RuntimeHostInteractionService::{
    requestOwnerToolPermissionAsync, RuntimeHostInteractionToolPermissionPayload,
    RuntimeHostInteractionToolPermissionTool, RuntimeHostInteractionToolPermissionToolParameter,
};
use operit_store::CoreNodeBindingStore::CoreNodeBindingStore;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::tools::ToolPermissionSystem::PermissionRequestResult;
use operit_tools::ToolExecutionManager::AITool;
use operit_util::MarkdownRenderStream::MarkdownStreamEvent;
use std::io::{self, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;

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
        Some("stream-probe") => run_link_stream_probe_command(&args[1..]).await,
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
    let coreApplication =
        create_cli_core_application_configured("server", configure_link_server_core).await?;
    coreApplication
        .serveRemoteLink(CoreRemoteLinkServerConfig::new(bind_address, token))
        .await
}

/// Configures the local client before the CLI Link server shares it through the Core tree.
fn configure_link_server_core(core: &mut operit_proxy_local::LocalCoreProxy) -> Result<(), String> {
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
    install_link_permission_requester(core);
    Ok(())
}

/// Installs the owner permission requester used by Link server tool calls.
pub(crate) fn install_link_permission_requester(core: &mut operit_proxy_local::LocalCoreProxy) {
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
    let _coreApplication = create_cli_core_application_without_space_sync("client").await?;
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
    let coreApplication = create_cli_core_application_without_space_sync("client").await?;
    let spaces = coreApplication
        .accessServices()
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
    let coreApplication = create_cli_core_application("client").await?;
    let service = coreApplication.accessServices();
    let pairing = service
        .startPairedRemote(url, token_hash, RemoteDeviceInfo::nativeCli("client"))
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
        session = service.setPairedRemoteTransport(name.clone(), transport)?;
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
    let ownsSpaceMutation = matches!(
        args,
        [command, ..] if matches!(command.as_str(), "rename" | "disconnect" | "remove" | "join" | "leave")
    );
    let coreApplication = if ownsSpaceMutation {
        create_cli_core_application("client").await?
    } else {
        create_cli_core_application_without_space_sync("client").await?
    };
    let service = coreApplication.accessServices();
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
        Some("status") if args.len() == 2 => {
            let status = service.pairedDeviceStatus(args[1].clone()).await?;
            println!("status={status:?}");
            Ok(())
        }
        Some("disconnect") if args.len() == 2 => {
            service.disconnectDeviceSpaceConnection(args[1].clone())?;
            println!("device space connection disconnected: {}", args[1]);
            Ok(())
        }
        Some("remove") if args.len() == 2 => {
            service.removePairedDevice(args[1].clone())?;
            println!("paired device removed: {}", args[1]);
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
        _ => Err("usage: operit2 cli link space <show|status <device-id>|rename <name>|join <paired-session>|disconnect <device-id>|remove <device-id>|leave>".to_string()),
    }
}

async fn run_link_sessions_command() -> Result<(), String> {
    let coreApplication = create_cli_core_application_without_space_sync("client").await?;
    let sessions = load_link_sessions(&coreApplication.accessStore())?;
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
    let coreApplication = create_cli_core_application_without_space_sync("client").await?;
    let accessStore = coreApplication.accessStore();
    let mut record = load_link_session_record(&accessStore, name)?;
    record.transport = parse_link_transport(&args[1])?;
    accessStore.saveOutboundSession(name.clone(), record.clone())?;
    println!(
        "session transport updated: {}",
        link_transport_name(&record.transport)
    );
    Ok(())
}

async fn run_link_session_delete_command(args: &[String]) -> Result<(), String> {
    let name = args
        .get(0)
        .ok_or_else(|| "usage: operit2 cli link session-delete <name>".to_string())?;
    let coreApplication = create_cli_core_application_without_space_sync("client").await?;
    coreApplication.accessStore().removeOutboundSession(name)?;
    println!("session deleted: {name}");
    Ok(())
}

async fn run_link_accepted_sessions_command() -> Result<(), String> {
    let coreApplication = create_cli_core_application_without_space_sync("server").await?;
    let sessions = load_link_server_sessions(&coreApplication.accessStore())?;
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
    let coreApplication = create_cli_core_application_without_space_sync("server").await?;
    remove_link_server_session(&coreApplication.accessStore(), session_id)?;
    println!("accepted session deleted: {session_id}");
    Ok(())
}

async fn run_link_ping_command(args: &[String]) -> Result<(), String> {
    let name = args
        .get(0)
        .ok_or_else(|| "usage: operit2 cli link ping <name>".to_string())?;
    let coreApplication = create_cli_core_application_without_space_sync("client").await?;
    let session = load_link_session_resolved(&coreApplication.accessStore(), name).await?;
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

/// Proves routed StateFlow values and embedded response streams over one real paired CLI session.
async fn run_link_stream_probe_command(args: &[String]) -> Result<(), String> {
    let name = args
        .get(0)
        .ok_or_else(|| "usage: operit2 cli link stream-probe <session>".to_string())?;
    let coreApplication = create_cli_core_application_without_space_sync("client").await?;
    let accessStore = coreApplication.accessStore();
    let record = load_link_session_record(&accessStore, name)?;
    let service = coreApplication.accessServices();
    let space = service.joinPairedDeviceSpace(name.clone()).await?;
    println!(
        "probe.space_joined name={} remoteNode={} members={}",
        name,
        record.coreDeviceId,
        space.members.len()
    );

    let chatId = format!("route-probe-{}", link_probe_unix_millis());
    CoreNodeBindingStore::new(coreApplication.nodeRuntime().runtimeStorageHost())?
        .create(&chatId, &record.coreDeviceId)?;
    println!(
        "probe.binding_created chatId={} target={}",
        chatId, record.coreDeviceId
    );

    let targetObjectId =
        operit_proxy_local::LocalCoreProxy::generatedObjectIdForSchema("chatRuntimeHolderMain")
            .ok_or_else(|| "generated object id missing: chatRuntimeHolderMain".to_string())?;
    let flowArgs = CoreValue::Map(BTreeMap::from([
        ("chatId".to_string(), CoreValue::String(chatId.clone())),
        (
            "streamText".to_string(),
            CoreValue::String("rslink-route-probe".to_string()),
        ),
    ]));
    let mut flowStream = coreApplication
        .localClient()
        .watch(CoreWatchRequest::new(
            format!("route-probe-flow-{chatId}"),
            targetObjectId,
            "routeProbeChatMessagesFlow",
            flowArgs,
        ))
        .await
        .map_err(|error| error.to_string())?;
    let flowEvent = recv_link_probe_event(&mut flowStream, "route probe flow").await?;
    let messages: Vec<ChatMessage> =
        operit_link::fromCoreValue(flowEvent.value.clone()).map_err(|error| error.to_string())?;
    let contentStreamCount = messages
        .iter()
        .filter(|message| message.contentStream.is_some())
        .count();
    println!(
        "probe.flow_event kind={:?} messages={} contentStreams={}",
        flowEvent.kind,
        messages.len(),
        contentStreamCount
    );
    if messages.is_empty() || contentStreamCount == 0 {
        return Err("probe flow did not expose a ChatMessage.contentStream".to_string());
    }

    let descriptor = find_core_stream_descriptor(&flowEvent.value)
        .ok_or_else(|| "probe flow did not contain a $coreStream descriptor".to_string())?;
    println!(
        "probe.stream_descriptor streamId={} target={} property={}",
        descriptor.streamId, descriptor.targetObjectId, descriptor.propertyName
    );
    if descriptor.targetObjectId != CORE_STREAM_POOL_OBJECT_ID
        || descriptor.propertyName != "openCoreStream"
    {
        return Err("probe stream descriptor does not target the Core stream pool".to_string());
    }

    let mut embeddedStream = coreApplication
        .localClient()
        .watch(CoreWatchRequest::new(
            format!("route-probe-embedded-{chatId}"),
            CORE_STREAM_POOL_OBJECT_ID,
            "openCoreStream",
            descriptor.args.clone(),
        ))
        .await
        .map_err(|error| error.to_string())?;
    let mut changedCount = 0usize;
    let mut completedCount = 0usize;
    let mut chunkText = String::new();
    loop {
        let event =
            recv_link_probe_event(&mut embeddedStream, "route probe embedded stream").await?;
        let markdown: MarkdownStreamEvent =
            operit_link::fromCoreValue(event.value.clone()).map_err(|error| error.to_string())?;
        println!(
            "probe.stream_event kind={:?} markdownType={} value={}",
            event.kind,
            markdown.eventType,
            markdown.value.clone().unwrap_or_default()
        );
        match event.kind {
            CoreEventKind::Changed => {
                changedCount += 1;
                if markdown.eventType == "chunk" {
                    if let Some(value) = markdown.value {
                        chunkText.push_str(&value);
                    }
                }
            }
            CoreEventKind::Completed => {
                completedCount += 1;
                break;
            }
            CoreEventKind::Snapshot | CoreEventKind::Delta => {}
        }
    }
    if changedCount == 0 || completedCount != 1 {
        return Err(format!(
            "probe embedded stream events invalid: changed={} completed={}",
            changedCount, completedCount
        ));
    }
    if chunkText != "rslink-route-probe / chunk-one / chunk-two" {
        return Err(format!("probe embedded stream chunks invalid: {chunkText}"));
    }
    println!(
        "probe.ok changed={} completed={} text={}",
        changedCount, completedCount, chunkText
    );
    Ok(())
}

/// Refreshes saved paired session URLs from current LAN discovery data.
async fn run_link_refresh_command(args: &[String]) -> Result<(), String> {
    let (target_name, timeout_ms) = parse_link_refresh_args(args)?;
    let devices = crate::mdns::discover_devices(timeout_ms)?;
    let coreApplication = create_cli_core_application_without_space_sync("client").await?;
    let accessStore = coreApplication.accessStore();
    let mut sessions = load_link_sessions(&accessStore)?;
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
    write_link_sessions(&accessStore, sessions)?;
    println!("sessions refreshed: updated={updated_count}");
    Ok(())
}

/// Receives one probe event with a short diagnostic deadline.
async fn recv_link_probe_event(
    stream: &mut CoreEventStream,
    label: &str,
) -> Result<CoreEvent, String> {
    match timeout(Duration::from_secs(3), stream.recv()).await {
        Ok(Some(event)) => Ok(event),
        Ok(None) => Err(format!("{label} closed before producing an event")),
        Err(_) => Err(format!("{label} did not produce an event in time")),
    }
}

/// Returns the current Unix epoch in milliseconds for unique probe keys.
fn link_probe_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after UNIX_EPOCH")
        .as_millis() as i64
}

/// Finds the first embedded Core stream descriptor in a structured Link value.
fn find_core_stream_descriptor(value: &CoreValue) -> Option<CoreStreamDescriptor> {
    match value {
        CoreValue::List(values) => values.iter().find_map(find_core_stream_descriptor),
        CoreValue::Map(values) => {
            if let Some(CoreValue::Map(descriptor)) = values.get("$coreStream") {
                return operit_link::fromCoreValue(CoreValue::Map(descriptor.clone())).ok();
            }
            values.values().find_map(find_core_stream_descriptor)
        }
        _ => None,
    }
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

fn parse_remote_url_token(args: &[String], usage: &str) -> Result<(String, String), String> {
    let (url, token, _, _) = parse_remote_url_token_save(args, usage)?;
    Ok((url, token))
}

fn parse_remote_url_token_save(
    args: &[String],
    usage: &str,
) -> Result<
    (
        String,
        String,
        Option<String>,
        Option<LinkTransportPreference>,
    ),
    String,
> {
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
fn load_link_sessions(
    accessStore: &LinkAccessStore,
) -> Result<BTreeMap<String, PairedRemoteSessionRecord>, String> {
    accessStore.outboundSessions()
}

/// Loads one saved paired session record by name.
fn load_link_session_record(
    accessStore: &LinkAccessStore,
    name: &str,
) -> Result<PairedRemoteSessionRecord, String> {
    let sessions = load_link_sessions(accessStore)?;
    sessions
        .get(name)
        .ok_or_else(|| format!("link session not found: {name}"))
        .cloned()
}

/// Loads one paired session after applying verified LAN endpoint discovery.
pub(crate) async fn load_link_session_resolved(
    accessStore: &LinkAccessStore,
    name: &str,
) -> Result<PairedRemoteSession, String> {
    let record = load_link_session_record(accessStore, name)?;
    let devices = crate::mdns::discover_devices(LINK_SESSION_DISCOVERY_TIMEOUT_MS)?;
    let (record, changed) =
        refresh_link_session_record_from_devices(name, record, &devices).await?;
    if changed {
        save_link_session(accessStore, name, record.clone())?;
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

/// Saves one paired session record by name.
fn save_link_session(
    accessStore: &LinkAccessStore,
    name: &str,
    record: PairedRemoteSessionRecord,
) -> Result<(), String> {
    let mut sessions = load_link_sessions(accessStore)?;
    sessions.insert(name.to_string(), record);
    write_link_sessions(accessStore, sessions)
}

/// Writes the complete paired session map to disk.
fn write_link_sessions(
    accessStore: &LinkAccessStore,
    sessions: BTreeMap<String, PairedRemoteSessionRecord>,
) -> Result<(), String> {
    for (name, record) in sessions {
        accessStore.saveOutboundSession(name, record)?;
    }
    Ok(())
}

/// Loads every accepted remote session from the application-owned access store.
fn load_link_server_sessions(
    accessStore: &LinkAccessStore,
) -> Result<BTreeMap<String, AcceptedRemoteSessionRecord>, String> {
    accessStore.inboundSessions()
}

/// Removes one accepted remote session from the application-owned access store.
fn remove_link_server_session(
    accessStore: &LinkAccessStore,
    session_id: &str,
) -> Result<(), String> {
    if !load_link_server_sessions(accessStore)?.contains_key(session_id) {
        return Err(format!("accepted link session not found: {session_id}"));
    }
    accessStore.removeInboundSession(session_id)
}

fn print_link_usage() {
    println!("operit2 cli link serve [--bind <addr:port>] [--token <token>]");
    println!("operit2 cli link discover [--timeout-ms <ms>]");
    println!("operit2 cli link hello <url> --token <token>");
    println!(
        "operit2 cli link connect <url> --token <token> --save <name> [--transport <http|ws>]"
    );
    println!("operit2 cli link space <show|status <device-id>|rename <name>|join <paired-session>|disconnect <device-id>|remove <device-id>|leave>");
    println!("operit2 cli link sessions");
    println!("operit2 cli link transport <session> <http|ws>");
    println!("operit2 cli link session-delete <name>");
    println!("operit2 cli link accepted-sessions");
    println!("operit2 cli link accepted-session-delete <session-id>");
    println!("operit2 cli link ping <name>");
    println!("operit2 cli link refresh [session] [--timeout-ms <ms>]");
    println!("operit2 cli link stream-probe <session>");
}
