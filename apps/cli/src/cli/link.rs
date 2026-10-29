use super::network_control_ui::{
    network_capabilities, network_device_id, network_device_label_by_id, network_device_labels,
    network_role_id, network_role_summary, new_network_control_id,
};
use super::*;
use crate::{
    create_cli_core_application, create_cli_core_application_configured,
    create_cli_core_application_without_space_sync,
};

use operit_link::{
    CoreCallRequest, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkSharedClient,
    CoreStreamDescriptor, CoreValue, CoreWatchRequest, CORE_STREAM_TARGET,
};
use operit_model::PromptTurn::PromptTurn;
use operit_providers::chat::enhance::ConversationService::ConversationService;
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::services::RuntimeHostInteractionService::{
    requestChatToolPermissionAsync, RuntimeHostInteractionToolPermissionTool,
    RuntimeHostInteractionToolPermissionToolParameter,
};
use operit_store::CoreNodeBindingStore::CoreNodeBindingStore;
use operit_store::NetworkControlStore::{NetworkControlIdentityAssignment, NetworkControlRole};
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::tools::ToolPermissionSystem::PermissionRequestResult;
use operit_tools::ToolExecutionManager::AITool;
use operit_util::MarkdownRenderStream::MarkdownStreamEvent;
use std::io::{self, IsTerminal, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;

pub(crate) async fn run_link_command(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("unpair" | "peers" | "prompts" | "listen") => {
            run_node_pairing_command(args).await
        }
        Some("pair") => run_link_pair(&args[1..]).await,
        Some("qr") => run_link_qr(&args[1..]).await,
        Some("session") => run_link_session(&args[1..]).await,
        Some("discover") => run_link_discover_command(&args[1..]).await,
        Some("space") => run_link_space_command(&args[1..]).await,
        Some("doctor") => run_link_doctor_command(&args[1..]).await,
        Some("control") => run_link_control_command(&args[1..]).await,
        Some("stream-probe") => run_link_stream_probe_command(&args[1..]).await,
        Some("edge-call") => run_link_edge_call_command(&args[1..]).await,
        _ => {
            print_link_usage();
            Ok(())
        }
    }
}

/// Resolves a visible device/request name or a one-based list position.
fn select_link_choice(choices: &[&str], selector: Option<&str>, kind: &str) -> Result<usize, String> {
    if let Some(selector) = selector {
        let matches: Vec<_> = choices.iter().enumerate()
            .filter(|(_, name)| **name == selector).map(|(index, _)| index).collect();
        match matches.as_slice() {
            [index] => return Ok(*index),
            [] => {
                if let Ok(number) = selector.parse::<usize>() {
                    if number > 0 && number <= choices.len() { return Ok(number - 1); }
                }
            }
            _ => return Err(format!("Ambiguous {kind} name: {selector}. Choose a number:\n{}", link_choices(choices))),
        }
    } else if choices.len() == 1 {
        return Ok(0);
    }
    if choices.is_empty() { return Err(format!("No {kind} available")); }
    let reason = if selector.is_some() { "No matching" } else { "Choose a" };
    Err(format!("{reason} {kind}; pass its name or number:\n{}", link_choices(choices)))
}

fn link_choices(choices: &[&str]) -> String {
    choices.iter().enumerate().map(|(index, name)| format!("  {}. {name}", index + 1))
        .collect::<Vec<_>>().join("\n")
}

/// Both interactive pairing and live sessions use the same public coordinates.
async fn start_link_pair(application: &operit_core_application::CoreApplication, selector: Option<&str>, interactive: bool)
    -> Result<operit_node_runtime::NodeServices::PendingPairing, String> {
    if let Some(code) = selector.filter(|value| value.trim().starts_with("operit://")) {
        return application.accessServices().startPairingFromCode(code.to_owned()).await;
    }
    let peers = application.accessServices().discoverPeers(2000).await?;
    let choices: Vec<_> = peers.iter().map(|peer| peer.displayName.as_str()).collect();
    if choices.is_empty() { return Err("No devices discovered. Run link qr on the receiving device, then use pair <connection-code>.".into()); }
    let input;
    let selector = if interactive && selector.is_none() && choices.len() > 1 {
        eprintln!("Devices:\n{}", link_choices(&choices));
        input = read_link_input("Device number or name: ").await?;
        Some(input.as_str())
    } else { selector };
    let index = select_link_choice(&choices, selector, "device")?;
    let peer = &peers[index];
    application.nodeServices()?.peers().startPairing(
        operit_node_runtime::NodeServices::PeerEndpoint { nodeId: peer.nodeId.clone(), address: peer.address.clone() }, None,
    ).await.map_err(|error| error.to_string())
}

/// Reads on a blocking worker so the pairing handshake can keep serving network traffic.
async fn read_link_input(prompt: &str) -> Result<String, String> {
    eprint!("{prompt}");
    io::stderr().flush().map_err(|error| error.to_string())?;
    tokio::task::spawn_blocking(|| {
        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|error| error.to_string())?;
        if input.trim().is_empty() { return Err("Pairing cancelled".into()); }
        Ok(input.trim().to_owned())
    }).await.map_err(|error| error.to_string())?
}

/// One terminal flow from device selection to six-digit confirmation.
async fn run_link_pair(args: &[String]) -> Result<(), String> {
    if args.len() > 1 { return Err("usage: operit2 cli link pair [device-name|connection-code]".into()); }
    if cli_json_mode() || !io::stdin().is_terminal() {
        return Err("link pair needs an interactive terminal; scripts can use link session with pair and confirm".into());
    }
    let application = create_cli_core_application_without_space_sync("client").await?;
    let result = async {
        let service = application.accessServices();
        let pending = start_link_pair(&application, args.first().map(String::as_str), true).await?;
        eprintln!("Pairing with {}. Read the six-digit code on the receiving device (CLI: link prompts).", pending.displayName);
        let confirmation = async {
            let code = read_link_input("Pairing code (empty to cancel): ").await?;
            service.finishPairing(pending.pairingId.clone(), code).await
        }.await;
        match confirmation {
            Ok(peer) => {
                println!("Paired with {}", peer.displayName);
                println!("To request Space membership: operit2 cli link space join \"{}\"", peer.displayName);
                Ok(())
            }
            Err(error) => {
                let _ = application.nodeServices()?.peers().cancelPairing(&pending.pairingId).await;
                Err(error)
            }
        }
    }.await;
    application.shutdown().await;
    result
}

/// Reuses the current owner, or keeps an explicitly created listener alive until Ctrl-C.
async fn run_link_qr(args: &[String]) -> Result<(), String> {
    if let Some(code) = crate::daemon::owner_peer_connection_code().await {
        if !args.is_empty() {
            return Err("A Core is already running. Use 'link qr' without listener options to display its live code.".into());
        }
        return print_connection_qr(&code?);
    }
    let application = create_cli_core_application_configured("server", configure_link_core).await?;
    let result = async {
        let mut listenerArgs = args.to_vec();
        if listenerArgs.first().map_or(true, |arg| arg.starts_with("--")) {
            listenerArgs.insert(0, "http".into());
        }
        let duration = start_node_listener(&application, &listenerArgs).await?;
        print_connection_qr(&application.accessServices().peerConnectionCode().await?)?;
        if !cli_json_mode() {
            eprintln!("Keep this command running while connecting. Press Ctrl-C to stop.");
            eprintln!("The pairing code will appear here when the other device connects.");
        }
        let deadline = async {
            if let Some(ms) = duration { tokio::time::sleep(Duration::from_millis(ms)).await; }
            else { std::future::pending::<()>().await; }
        };
        tokio::pin!(deadline);
        let mut tick = tokio::time::interval(Duration::from_millis(500));
        let mut shown = std::collections::BTreeSet::new();
        loop {
            tokio::select! {
                signal = tokio::signal::ctrl_c() => { signal.map_err(|error| error.to_string())?; break; }
                _ = &mut deadline => break,
                _ = tick.tick() => {
                    for prompt in application.accessServices().pairingPrompts()? {
                        if shown.insert(prompt.pairingId.clone()) {
                            if cli_json_mode() { emit_cli_json(serde_json::json!({"pairingPrompt": prompt})); }
                            else { println!("Pairing: {} ({})   {}", prompt.displayName, prompt.peerNodeId, prompt.confirmationCode); }
                        }
                    }
                }
            }
        }
        Ok(())
    }.await;
    application.shutdown().await;
    result
}

fn print_connection_qr(code: &str) -> Result<(), String> {
    let info = operit_node_runtime::PeerConnectionCode::decode(code, "").map_err(|error| error.to_string())?;
    if cli_json_mode() {
        emit_cli_json(serde_json::json!({"connectionCode": code, "device": info}));
        return Ok(());
    }
    let rendered = render_connection_qr(code)?;
    println!("{} ({})", info.displayName, info.nodeId);
    if let Ok((width, _)) = crossterm::terminal::size() {
        let needed = qrcode::QrCode::with_error_correction_level(code.as_bytes(), qrcode::EcLevel::L)
            .map_err(|error| error.to_string())?.width() + 8;
        if usize::from(width) < needed { eprintln!("Widen the terminal to at least {needed} columns to scan this QR code."); }
    }
    print!("{rendered}");
    println!("{code}");
    println!("Scan with Operit → Add device → Scan code. Pairing and Space approval still apply.");
    io::stdout().flush().map_err(|error| error.to_string())
}

/// Two modules per terminal row. Explicit black/white colors and a four-module quiet zone
/// keep the QR readable in both light and dark terminals.
fn render_connection_qr(code: &str) -> Result<String, String> {
    use qrcode::{types::Color, EcLevel, QrCode};
    let qr = QrCode::with_error_correction_level(code.as_bytes(), EcLevel::L)
        .map_err(|error| error.to_string())?;
    let side = qr.width() + 8;
    let dark = |x: usize, y: usize| x >= 4 && y >= 4 && x < side - 4 && y < side - 4
        && qr[(x - 4, y - 4)] == Color::Dark;
    let mut output = String::new();
    for y in (0..side).step_by(2) {
        output.push_str("\x1b[30;107m");
        for x in 0..side {
            output.push(match (dark(x, y), dark(x, y + 1)) {
                (false, false) => ' ', (true, true) => '█', (true, false) => '▀', (false, true) => '▄',
            });
        }
        output.push_str("\x1b[0m\n");
    }
    Ok(output)
}

/// CLI 只解析输入和展示结果；配对直接调用应用注入的 RuntimePeerService。
async fn run_node_pairing_command(args: &[String]) -> Result<(), String> {
    let application = if args.first().map(String::as_str) == Some("listen") {
        create_cli_core_application_configured("server", configure_link_core).await?
    } else {
        create_cli_core_application_without_space_sync("client").await?
    };
    if args.first().map(String::as_str) == Some("listen") {
        return run_node_listen(&application, &args[1..]).await;
    }
    let result = execute_node_pairing_command(application.nodeServices()?.peers().as_ref(), args).await;
    application.shutdown().await;
    result
}

/// 管理配置仍保存原路径；监听由统一 RuntimePeerService 拥有。
async fn start_node_listener(application: &operit_core_application::CoreApplication, args: &[String]) -> Result<Option<u64>, String> {
    use operit_node_runtime::{NodeServices::PeerTransport, PeerStateStore::PeerHostPortMode};
    let mut transports = Vec::new();
    let mut config = application.accessServices().localHostConfig()?.ok_or("Node listener preferences are not initialized")?;
    let mut duration = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--bind" | "--duration-ms" => {
                let flag = &args[index]; index += 1;
                let value = args.get(index).ok_or_else(|| format!("Missing value for {flag}"))?;
                match flag.as_str() {
                    "--bind" => config.bindAddress = value.clone(),
                    _ => duration = Some(value.parse::<u64>().map_err(|e| e.to_string())?),
                }
            },
            "--no-discovery" => config.discoveryEnabled = false,
            "--fixed-port" => config.portMode = PeerHostPortMode::Fixed,
            "--automatic-port" => config.portMode = PeerHostPortMode::Automatic,
            value => for mode in value.split(',') {
                let transport = match mode {
                    "http" => PeerTransport::Http, "ws" => PeerTransport::WebSocket, "tcp" => PeerTransport::Tcp,
                    "serial" => PeerTransport::Serial, "bluetooth" => PeerTransport::Bluetooth,
                    _ => return Err(format!("Unknown transport: {mode}")),
                };
                if !transports.contains(&transport) { transports.push(transport); }
            },
        }
        index += 1;
    }
    if transports.is_empty() { transports = config.transports.clone(); }
    if transports.is_empty() { return Err("Choose at least one listener transport".into()); }
    config.transports = transports.clone();
    config.updatedAt = operit_host_api::TimeUtils::currentTimeMillis();
    application.accessServices().saveLocalHostConfig(config.clone())?;
    application.accessServices().startListening(transports).await?;
    let config = application.accessServices().localHostConfig()?.ok_or("Listener config missing after startup")?;
    let ready = serde_json::json!({"listening": true, "nodeId": application.localNodeId(), "bindAddress": config.bindAddress, "transports": config.transports});
    if cli_json_mode() { emit_cli_json(ready); } else { println!("{ready}"); }
    io::stdout().flush().map_err(|e| e.to_string())?;
    Ok(duration)
}

async fn run_node_listen(application: &operit_core_application::CoreApplication, args: &[String]) -> Result<(), String> {
    let duration = start_node_listener(application, args).await?;
    // 本机验证码通过普通管理命令 `link prompts` 查看，不放入匿名协议响应。
    match duration {
        Some(ms) => application.localClient().hostManager().hostRuntimeTaskSchedulerHost.as_ref()
            .ok_or("Host task scheduler missing")?.waitForHostRuntimeDelay(ms).await.map_err(|e| e.to_string())?,
        None => tokio::signal::ctrl_c().await.map_err(|e| e.to_string())?,
    }
    application.accessServices().stopListening().await?;
    Ok(())
}

/// One live Core application for interactive network management. No per-command
/// restart, hidden approval, or private test storage path/protocol.
async fn run_link_session(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--duration-ms") {
        return Err("link session does not accept --duration-ms; use quit or Ctrl-C".into());
    }
    let application = create_cli_core_application_configured("session", configure_link_core).await?;
    let result = run_link_session_inner(&application, args).await;
    application.shutdown().await;
    result
}

fn parse_session_command(line: &str) -> Result<Vec<String>, String> {
    if line.trim_start().starts_with('[') {
        serde_json::from_str(line).map_err(|e| format!("Expected a JSON array of command arguments: {e}"))
    } else {
        Ok(line.split_whitespace().map(str::to_owned).collect())
    }
}

async fn run_link_session_inner(application: &operit_core_application::CoreApplication, args: &[String]) -> Result<(), String> {
    start_node_listener(application, args).await?;
    if !cli_json_mode() {
        println!("Commands: qr/pair/confirm/pair-cancel/prompts/peers/unpair, space ..., control ..., core ..., quit");
        println!("For arguments containing spaces, enter a JSON array, e.g. [\"space\",\"rename\",\"My Space\"]");
    }
    let (sender, mut receiver) = tokio::sync::mpsc::channel(16);
    // stdin must not block the executor that owns live peer sessions and sync.
    std::thread::spawn(move || {
        use std::io::BufRead;
        for line in io::stdin().lock().lines() {
            let failed = line.is_err();
            if sender.blocking_send(line.map_err(|e| e.to_string())).is_err() || failed { break; }
        }
    });
    let mut pendingPairing = None;
    loop {
        let line = tokio::select! {
            line = receiver.recv() => match line { Some(line) => line?, None => break },
            signal = tokio::signal::ctrl_c() => { signal.map_err(|e| e.to_string())?; break; }
        };
        let result = match parse_session_command(&line) {
            Ok(command) if command.is_empty() => continue,
            Ok(command) if command.as_slice() == ["quit"] || command.as_slice() == ["exit"] => break,
            Ok(command) => execute_link_session_command(application, &command, &mut pendingPairing).await,
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            if cli_json_mode() { emit_cli_json(serde_json::json!({"error": error})); }
            else { eprintln!("{error}"); }
        }
        io::stdout().flush().map_err(|e| e.to_string())?;
    }
    if let Some(pending) = pendingPairing {
        application.accessServices().cancelPairing(pending.pairingId).await?;
    }
    application.accessServices().stopListening().await?;
    if cli_json_mode() { emit_cli_json(serde_json::json!({"stopped": true})); }
    Ok(())
}

async fn execute_link_session_command(application: &operit_core_application::CoreApplication, args: &[String],
    pendingPairing: &mut Option<operit_node_runtime::NodeServices::PendingPairing>) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("qr") if args.len() == 1 => {
            print_connection_qr(&application.accessServices().peerConnectionCode().await?)
        }
        Some("pair") if args.len() <= 2 => {
            if pendingPairing.is_some() { return Err("A pairing is waiting. Use confirm <six-digit-code> or pair-cancel first.".into()); }
            let pending = start_link_pair(application, args.get(1).map(String::as_str), false).await?;
            if cli_json_mode() { emit_cli_json(serde_json::json!({"deviceName": pending.displayName, "awaitingConfirmation": true})); }
            else { println!("Pairing with {}. Read the receiving device's code, then enter: confirm <six-digit-code>", pending.displayName); }
            *pendingPairing = Some(pending);
            Ok(())
        }
        Some("confirm") if args.len() == 2 => {
            let pending = pendingPairing.as_ref().ok_or("No pairing waiting. Use pair first.")?;
            let peer = application.accessServices().finishPairing(pending.pairingId.clone(), args[1].clone()).await?;
            *pendingPairing = None;
            if cli_json_mode() { emit_cli_json(serde_json::to_value(peer).map_err(|error| error.to_string())?); }
            else { println!("Paired with {}", peer.displayName); }
            Ok(())
        }
        Some("pair-cancel") if args.len() == 1 => {
            let pending = pendingPairing.as_ref().ok_or("No pairing waiting")?;
            application.accessServices().cancelPairing(pending.pairingId.clone()).await?;
            *pendingPairing = None;
            if cli_json_mode() { emit_cli_json(serde_json::json!({"cancelled": true})); }
            else { println!("Pairing cancelled"); }
            Ok(())
        }
        Some("space") => execute_link_space_command(application, &args[1..]).await,
        Some("control") => execute_link_control_command(application, &args[1..]).await,
        Some("edge-call") => execute_link_edge_call_command(application, &args[1..]).await,
        Some("unpair" | "peers" | "prompts") =>
            execute_node_pairing_command(application.nodeServices()?.peers().as_ref(), args).await,
        Some("chat-watch") if args.len() == 2 => {
            let target = operit_proxy_local::LocalCoreProxy::generatedTargetForSchema("chatRuntimeHolderMain")
                .ok_or("Chat schema missing")?;
            let mut snapshots = serde_json::Map::new();
            for property in ["chatMessagesFlow", "chatStateFlow"] {
                let event = tokio::time::timeout(std::time::Duration::from_secs(10), async {
                    let mut stream = application.localClient().watch(CoreWatchRequest::new(
                        format!("session-{property}-{}", link_probe_unix_millis()), target, property,
                        operit_link::toCoreValue(serde_json::json!({"chatId": args[1]})).map_err(|e| e.to_string())?,
                    )).await.map_err(|e| e.to_string())?;
                    stream.recv().await.ok_or_else(|| "Chat watch ended before snapshot".to_string())
                }).await.map_err(|_| format!("Chat watch timed out: {property}"))??;
                let value: serde_json::Value = operit_link::fromCoreValue(event.value).map_err(|e| e.to_string())?;
                snapshots.insert(property.into(), value);
            }
            snapshots.insert("route".into(), serde_json::to_value(
                application.nodeRouter().bindingRouteStatus(args[1].clone())?
            ).map_err(|e| e.to_string())?);
            emit_cli_json(serde_json::Value::Object(snapshots));
            Ok(())
        }
        Some("call") if args.len() == 4 => {
            execute_link_local_call_command(application, &args[1..]).await
        }
        Some("core") if args.len() > 1 => {
            let mut proxy = operit_proxy_local::GeneratedCoreProxy::new(crate::core_proxy::SharedLocalCore(application.localClient()));
            let mut command = args[1..].to_vec();
            if cli_json_mode() { command.push("--json".into()); }
            let output = proxy.runCoreCommand(&command).await.map_err(core_command_error_message)?;
            if !output.stdout.is_empty() { ::std::println!("{}", output.stdout); }
            if !output.stderr.is_empty() { eprint!("{}", output.stderr); }
            Ok(())
        }
        _ => Err("session commands: qr|pair [device-name|connection-code]|confirm <code>|pair-cancel|unpair [device-name|number]|prompts|peers|space|control|core|call <schema> <method> <JSON>|chat-watch <chat-id>|quit".into()),
    }
}

/// Calls one generated local Core schema through the same routed client used by interactive commands.
async fn execute_link_local_call_command(
    application: &operit_core_application::CoreApplication,
    args: &[String],
) -> Result<(), String> {
    let target = operit_proxy_local::LocalCoreProxy::generatedTargetForSchema(&args[0])
        .ok_or_else(|| format!("Unknown Core schema: {}", args[0]))?;
    let arguments: serde_json::Value = serde_json::from_str(&args[2])
        .map_err(|error| error.to_string())?;
    let response = application.localClient().call(CoreCallRequest::new(
        operit_link::nextCoreRouteRequestId("cliCall"), target, &args[1],
        operit_link::toCoreValue(arguments).map_err(|error| error.to_string())?,
    )).await;
    let value: serde_json::Value = operit_link::fromCoreValue(
        response.result.map_err(|error| error.to_string())?,
    ).map_err(|error| error.to_string())?;
    emit_cli_json(value);
    Ok(())
}

/// Executes pairing and listener commands through the shared node service.
async fn execute_node_pairing_command(
    services: &dyn operit_node_runtime::RuntimePeerService::RuntimePeerService,
    args: &[String],
) -> Result<(), String> {
    let value = match args {
        [command, ..] if command == "unpair" && args.len() <= 2 => {
            let peers = services.pairedPeers().map_err(|error| error.to_string())?;
            let choices: Vec<_> = peers.iter().map(|peer| peer.displayName.as_str()).collect();
            let index = select_link_choice(&choices, args.get(1).map(String::as_str), "paired device")?;
            services.removePairedPeer(&peers[index].nodeId).await.map_err(|error| error.to_string())?;
            serde_json::json!({"removed": peers[index].displayName})
        }
        [command] if command == "prompts" => serde_json::to_value(services.pairingPrompts().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?,
        [command] if command == "peers" => serde_json::to_value(services.pairedPeers().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?,
        _ => return Err("link unpair [device-name|number] | peers | prompts".into()),
    };
    if cli_json_mode() {
        emit_cli_json(value);
    } else if let Some(entries) = value.as_array() {
        if entries.is_empty() { println!("No devices"); }
        for (index, entry) in entries.iter().enumerate() {
            let name = entry["displayName"].as_str().unwrap_or("Unnamed device");
            if let Some(code) = entry["confirmationCode"].as_str() { println!("{}. {name} · code: {code}", index + 1); }
            else { println!("{}. {name}", index + 1); }
        }
    } else {
        println!("Removed {}", value["removed"].as_str().unwrap_or("device"));
    }
    Ok(())
}

async fn run_link_edge_call_command(args: &[String]) -> Result<(), String> {
    let application = create_cli_core_application_without_space_sync("client").await?;
    let result = execute_link_edge_call_command(&application, args).await;
    application.shutdown().await;
    result
}

async fn execute_link_edge_call_command(coreApplication: &operit_core_application::CoreApplication, args: &[String]) -> Result<(), String> {
    const USAGE: &str = "usage: operit2 cli link edge-call <device> <edge-target> <method> [json-args]";
    if !(3..=4).contains(&args.len()) || !args[1].starts_with("edge.") || args[1] == "edge.plugins"
        || args[1].chars().any(char::is_whitespace) || args[2].is_empty() || args[2].chars().any(char::is_whitespace) {
        return Err(USAGE.into());
    }
    let device = &args[0];
    let topology = coreApplication.accessServices().deviceSpaceTopology()?;
    let deviceId = network_device_id(&topology, device)?;
    // Restored credentials are not yet an authenticated online route. Give the
    // runtime's existing availability worker time to establish it; do not bypass
    // routing/Space policy or depend on the business persistence sync worker.
    let router = coreApplication.nodeRouter();
    tokio::time::timeout(Duration::from_secs(8), async {
        let peers = coreApplication.nodeServices()?.peers();
        let mut changes = peers.subscribePeerChanges();
        loop {
            if router.nodeIsReachable(&deviceId)? { return Ok::<_, String>(()); }
            changes.recv().await.map_err(|error| error.to_string())?;
        }
    }).await.map_err(|_| format!("Device did not become reachable: {deviceId}"))??;
    let client = operit_node_runtime::NodeClient::NodeClient::new(router, deviceId);
    let value: serde_json::Value = serde_json::from_str(args.get(3).map(String::as_str).unwrap_or("{}"))
        .map_err(|error| error.to_string())?;
    let request = operit_link::CoreCallRequest::new(
        operit_link::nextCoreRouteRequestId("edgeCall"), &args[1], &args[2],
        operit_link::toCoreValue(value).map_err(|error| error.to_string())?,
    );
    let response = operit_link::CoreLinkSharedClient::call(&client, request).await;
    let json: serde_json::Value = operit_link::fromCoreValue(response.result.map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    if cli_json_mode() {
        emit_cli_json(json);
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&json).map_err(|error| error.to_string())?
        );
    }
    Ok(())
}

/// Configures the local client before the CLI Link server shares it through the Core tree.
fn configure_link_core(core: &mut operit_proxy_local::LocalCoreProxy) -> Result<(), String> {
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
        .setAsyncPermissionRequester(move |tool, description, chatId| async move {
            let Some(chatId) = chatId else {
                return PermissionRequestResult::DENY;
            };
            let response = requestChatToolPermissionAsync(
                chatId,
                tool_to_permission_payload(&tool),
                description,
                Duration::from_secs(60),
            )
            .await
            .expect("permission request failed");
            match response.as_str() {
                "allow" => PermissionRequestResult::ALLOW,
                "allow_session" => PermissionRequestResult::ALLOW_SESSION,
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

/// Discovers devices through the shared node service for code-confirmed pairing.
/// Runs the shared-Space doctor: read-only by default, `--repair` quarantines
/// damaged journal lines after a backup and a shadow-replay selfcheck.
async fn run_link_doctor_command(args: &[String]) -> Result<(), String> {
    let repair = args.iter().any(|arg| arg == "--repair");
    if args.iter().any(|arg| arg != "--repair") || args.len() > 1 {
        return Err("usage: operit2 cli link doctor [--repair]".into());
    }
    let coreApplication = if repair {
        create_cli_core_application("client").await?
    } else {
        create_cli_core_application_without_space_sync("client").await?
    };
    let service = coreApplication.accessServices();
    let diagnosis = operit_node_runtime::SpaceDoctor::diagnose(&service, repair);
    coreApplication.shutdown().await;
    let report = diagnosis.map_err(|error| format!("space diagnosis failed: {error}"))?;
    if cli_json_mode() {
        emit_cli_json(serde_json::to_value(&report).map_err(|error| error.to_string())?);
        return Ok(());
    }
    println!("Space: {} ({}) revision {}", report.spaceName, report.spaceId, report.spaceRevision);
    println!(
        "Policy replay: {} · members records={} replay={}",
        if report.policyReadable { "readable" } else { "UNREADABLE" },
        report.recordMembers.len(),
        report.replayMembers.len()
    );
    for journal in &report.journals {
        println!(
            "Journal {}: {} lines, {} decoded, {} findings",
            journal.deviceId, journal.totalLines, journal.decodedLines, journal.findings.len()
        );
    }
    println!(
        "Projection: {} entries, {} journal-only, {} projection-only",
        report.projection.projectionEntries,
        report.projection.journalOnlyOperationIds.len(),
        report.projection.projectionOnlyOperationIds.len()
    );
    for pair in &report.pairs {
        println!(
            "Peer {}: member={} online={} inbound={} outbound={}",
            pair.deviceId, pair.member, pair.online, pair.pairedInbound, pair.pairedOutbound
        );
    }
    if report.findings.is_empty() {
        println!("Findings: none");
    } else {
        for finding in &report.findings {
            println!("[{}] {}: {}", finding.severity, finding.check, finding.summary);
        }
    }
    println!(
        "Counters: divergences={} quarantined={} repairs={} failed={} reconciles={} rounds={} incomplete={}",
        report.counters.divergencesDetected, report.counters.linesQuarantined,
        report.counters.repairsRun, report.counters.repairsFailed,
        report.counters.reconcileExchanges, report.counters.reconcileRounds,
        report.counters.reconcileIncomplete
    );
    if let Some(repair) = &report.repair {
        println!(
            "Repair: selfcheck={} quarantined={} restoredFromBackup={} ({})",
            repair.selfcheckPassed, repair.linesQuarantined, repair.restoredFromBackup, repair.detail
        );
        for backup in &repair.backupPaths {
            println!("  backup: {backup}");
        }
    }
    Ok(())
}

async fn run_link_discover_command(args: &[String]) -> Result<(), String> {    let mut timeout_ms = 2000_u64;
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
    let peers = coreApplication.accessServices().discoverPeers(timeout_ms).await
        .map_err(|error| error.to_string())?;
    if cli_json_mode() {
        emit_cli_json(serde_json::json!({ "peers": peers }));
    } else {
        for peer in peers { println!("{} — {} ({})", peer.displayName, peer.address, peer.nodeId); }
    }
    Ok(())
}

/// Runs user-facing device-space inspection and membership commands.
async fn run_link_space_command(args: &[String]) -> Result<(), String> {
    let ownsSpaceMutation = matches!(
        args,
        [command, ..] if matches!(command.as_str(), "rename" | "disconnect" | "admit" | "remove" | "join" | "leave" | "approve" | "reject" | "refresh" | "cancel" | "sync" | "preference-set" | "preference-delete")
    );
    let coreApplication = if ownsSpaceMutation {
        create_cli_core_application("client").await?
    } else {
        create_cli_core_application_without_space_sync("client").await?
    };
    let result = execute_link_space_command(&coreApplication, args).await;
    coreApplication.shutdown().await;
    result
}

async fn execute_link_space_command(coreApplication: &operit_core_application::CoreApplication, args: &[String]) -> Result<(), String> {
    let service = coreApplication.accessServices();
    match args.first().map(String::as_str) {
        None | Some("show") if args.len() <= 1 => {
            let space = service.deviceSpace()?;
            if cli_json_mode() {
                emit_cli_json(serde_json::to_value(&space).map_err(|error| error.to_string())?);
            } else {
                println!("Device space: {}", space.spaceName);
                println!("Devices: {}", space.members.len());
            }
            Ok(())
        }
        Some("topology") if args.len() == 1 => {
            let topology = service.deviceSpaceTopology()?;
            emit_cli_json(serde_json::to_value(topology).map_err(|error| error.to_string())?);
            Ok(())
        }
        Some("rename") if args.len() == 2 => {
            let space = service.renameDeviceSpace(args[1].clone())?;
            if cli_json_mode() { emit_cli_json(serde_json::json!(space)); }
            else { println!("Device space renamed to {}", space.spaceName); }
            Ok(())
        }
        Some("status") if args.len() == 2 => {
            // Paired devices in independent Spaces are not in the current
            // Space topology yet, but their connection status is still inspectable.
            let (device_id, device_label) = if service.pairedDevicesSnapshot()?.contains_key(&args[1]) {
                (args[1].clone(), args[1].clone())
            } else {
                let topology = service.deviceSpaceTopology()?;
                let id = network_device_id(&topology, &args[1])?;
                let label = network_device_label_by_id(&topology, &id)?;
                (id, label)
            };
            let status = service.pairedDeviceStatus(device_id).await?;
            if cli_json_mode() { emit_cli_json(serde_json::json!({ "status": format!("{status:?}") })); }
            else { println!("{device_label}: {status:?}"); }
            Ok(())
        }
        // Same policy operation as `control device disconnect`: one word, one
        // meaning across the CLI, the TUI and Flutter's control panel.
        Some("disconnect") if args.len() == 2 => {
            let topology = service.deviceSpaceTopology()?;
            let device_id = network_device_id(&topology, &args[1])?;
            let device_label = network_device_label_by_id(&topology, &device_id)?;
            service.disconnectDeviceSpaceNode(device_id).await?;
            if cli_json_mode() { emit_cli_json(serde_json::json!({ "disconnected": true, "restricted": true })); }
            else { println!("Disconnected device {device_label}; direct links and route transit stay blocked until `space admit`"); }
            Ok(())
        }
        Some("admit") if args.len() == 2 => {
            let topology = service.deviceSpaceTopology()?;
            let device_id = network_device_id(&topology, &args[1])?;
            let device_label = network_device_label_by_id(&topology, &device_id)?;
            service.admitDeviceSpaceMember(device_id)?;
            if cli_json_mode() { emit_cli_json(serde_json::json!({ "admitted": true })); }
            else { println!("Admitted device {device_label}; it may connect again"); }
            Ok(())
        }
        Some("remove") if args.len() == 2 => {
            let topology = service.deviceSpaceTopology()?;
            let device_id = network_device_id(&topology, &args[1])?;
            let device_label = network_device_label_by_id(&topology, &device_id)?;
            service.removeDeviceSpaceMember(device_id).await?;
            if cli_json_mode() { emit_cli_json(serde_json::json!({ "removed": true })); }
            else { println!("Removed device {device_label} from the Space and forgot its pairing"); }
            Ok(())
        }
        Some("join") if args.len() <= 2 => {
            let peers = coreApplication.nodeServices()?.peers().pairedPeers().map_err(|error| error.to_string())?;
            let choices: Vec<_> = peers.iter().map(|peer| peer.displayName.as_str()).collect();
            let index = select_link_choice(&choices, args.get(1).map(String::as_str), "paired device")?;
            let request = service.requestDeviceSpaceJoin(peers[index].nodeId.clone()).await?;
            print_space_join_request(&request)?;
            Ok(())
        }
        Some("requests") if args.len() == 2 => {
            let requests = match args[1].as_str() {
                "incoming" => service.incomingDeviceSpaceJoins().await?,
                "outgoing" => service.outgoingDeviceSpaceJoins()?,
                _ => return Err("usage: link space requests <incoming|outgoing>".into()),
            };
            if cli_json_mode() { emit_cli_json(serde_json::json!(requests)); }
            else if requests.is_empty() { println!("No join requests"); }
            else { for request in &requests { print_space_join_request(request)?; } }
            Ok(())
        }
        Some("refresh" | "cancel") if args.len() <= 2 => {
            let history = service.outgoingDeviceSpaceJoins()?;
            // A fresh application replaces the previous receipt for this target.
            let mut latest = std::collections::BTreeMap::new();
            for request in &history { latest.insert(&request.targetDeviceId, request); }
            let mut requests: Vec<_> = latest.into_values().collect();
            requests.sort_by_key(|request| request.createdAt);
            let choices: Vec<_> = requests.iter().map(|request| request.spaceName.as_str()).collect();
            let index = select_link_choice(&choices, args.get(1).map(String::as_str), "outgoing join request")?;
            let id = requests[index].requestId.clone();
            let updated = if args[0] == "refresh" { service.refreshDeviceSpaceJoin(id).await? }
                else { service.cancelDeviceSpaceJoin(id).await? };
            print_space_join_request(&updated)
        }
        Some("approve" | "reject") if args.len() <= 2 => {
            let requests = service.incomingDeviceSpaceJoins().await?;
            let requests: Vec<_> = requests.iter().filter(|request| request.canApprove).collect();
            let choices: Vec<_> = requests.iter().map(|request| request.applicantName.as_str()).collect();
            let index = select_link_choice(&choices, args.get(1).map(String::as_str), "join request awaiting your decision")?;
            let request = requests[index];
            print_space_join_request(&service.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, args[0] == "approve").await?)
        }
        Some("preferences") if args.len() == 2 => {
            let preferences = service.devicePreferences(args[1].clone())?;
            if cli_json_mode() { emit_cli_json(serde_json::json!(preferences)); }
            else { println!("{}", serde_json::to_string_pretty(&preferences).map_err(|e| e.to_string())?); }
            Ok(())
        }
        Some("preference-set") if args.len() == 4 => {
            let value = serde_json::from_str(&args[3]).map_err(|e| format!("Invalid preference JSON: {e}"))?;
            service.setDevicePreference(args[1].clone(), args[2].clone(), Some(value))?;
            if cli_json_mode() { emit_cli_json(serde_json::json!({"updated": true})); }
            else { println!("Device preference updated"); }
            Ok(())
        }
        Some("preference-delete") if args.len() == 3 => {
            service.setDevicePreference(args[1].clone(), args[2].clone(), None)?;
            if cli_json_mode() { emit_cli_json(serde_json::json!({"updated": true})); }
            else { println!("Device preference deleted"); }
            Ok(())
        }
        Some("sync") if args.len() == 1 => {
            operit_node_runtime::SpacePersistenceSyncService::SpacePersistenceSyncService::new(
                Arc::new(coreApplication.nodeRuntime()), coreApplication.nodeRouter(),
                operit_store::CoreSpaceStore::CoreSpaceStore::new(coreApplication.nodeRuntime().runtimeStorageHost()),
            ).synchronizeOnce().await?;
            if cli_json_mode() { emit_cli_json(serde_json::json!({"synchronized": true})); }
            else { println!("Space synchronization completed"); }
            Ok(())
        }
        Some("leave") if args.len() == 1 => {
            let space = service.leaveDeviceSpace()?;
            if cli_json_mode() { emit_cli_json(serde_json::json!(space)); }
            else { println!("Left device space; current space: {}", space.spaceName); }
            Ok(())
        }
        _ => Err("usage: operit2 cli link space <show|topology|status <device-name>|rename <name>|join [device-name]|requests <incoming|outgoing>|refresh [space-name|number]|approve [device-name|number]|reject [device-name|number]|cancel [space-name|number]|preferences <node-id>|preference-set <node-id> <key> <JSON>|preference-delete <node-id> <key>|sync|disconnect <device-name>|admit <device-name>|remove <device-name>|leave>".to_string()),
    }
}

fn print_space_join_request(request: &operit_node_runtime::RuntimeRemoteLinkService::SpaceJoinRequest) -> Result<(), String> {
    if cli_json_mode() {
        emit_cli_json(serde_json::to_value(request).map_err(|e| e.to_string())?);
    } else {
        println!("{}: {:?} · Space: {} · reviewer: {}",
            request.applicantName, request.status, request.spaceName,
            request.reviewerName.as_deref().unwrap_or("not assigned"));
    }
    Ok(())
}

/// Runs authoritative Space control policy commands through the shared runtime service.
async fn run_link_control_command(args: &[String]) -> Result<(), String> {
    let mutatesPolicy = matches!(
        args.first().map(String::as_str),
        Some("bootstrap") | Some("identity") | Some("device") | Some("policy")
    );
    let coreApplication = if mutatesPolicy {
        create_cli_core_application("client").await?
    } else {
        create_cli_core_application_without_space_sync("client").await?
    };
    let result = execute_link_control_command(&coreApplication, args).await;
    coreApplication.shutdown().await;
    result
}

async fn execute_link_control_command(coreApplication: &operit_core_application::CoreApplication, args: &[String]) -> Result<(), String> {
    const USAGE: &str = "usage: operit2 cli link control <show|bootstrap|audit|identity|device|policy>";
    let service = coreApplication.accessServices();
    match args.first().map(String::as_str) {
        Some("show") if args.len() == 1 => {
            let state = service.deviceSpaceControl()?;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!(state));
            } else {
                println!(
                    "Network control: {} · {} identities · {} devices",
                    if state.initialized {
                        "ready"
                    } else {
                        "not initialized"
                    },
                    state.roles.len(),
                    service.deviceSpaceTopology()?.devices.len(),
                );
            }
            Ok(())
        }
        Some("bootstrap") if args.len() == 1 => {
            let state = service.bootstrapDeviceSpaceControl()?;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!(state));
            } else {
                println!("Network control initialized");
            }
            Ok(())
        }
        Some("audit") if args.len() == 1 => {
            let audit = service.deviceSpaceControlAudit()?;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!(audit));
            } else {
                for record in audit {
                    println!(
                        "{} · {}",
                        if record.accepted {
                            "accepted"
                        } else {
                            "rejected"
                        },
                        record.summary,
                    );
                }
            }
            Ok(())
        }
        Some("identity") => {
            run_link_control_identity_command(&service, coreApplication, &args[1..])
        }
        Some("device") => run_link_control_device_command(&service, &args[1..]).await,
        Some("policy") => run_link_control_policy_command(&service, &args[1..]),
        _ => Err(USAGE.to_string()),
    }
}

/// Runs identity definition, assignment, and revocation commands.
fn run_link_control_identity_command(
    service: &operit_node_runtime::RuntimeRemoteLinkService::RuntimeRemoteLinkService,
    coreApplication: &operit_core_application::CoreApplication,
    args: &[String],
) -> Result<(), String> {
    const USAGE: &str = "usage: operit2 cli link control identity <list|define <name> <all|audit|relay|storage|execute|view|manage-identities|assign-identity|approve|join>...|set <device-name> <identity-name>|clear <device-name>>";
    match args {
        [command] if command == "list" => {
            let state = service.deviceSpaceControl()?;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!(state.roles));
            } else {
                for role in state.roles.values() {
                    println!("{}", network_role_summary(role));
                }
            }
            Ok(())
        }
        [command, displayName, capabilities @ ..]
            if command == "define" && !capabilities.is_empty() =>
        {
            let roleId = new_network_control_id("role");
            let capabilities = network_capabilities(capabilities)?;
            service.defineDeviceSpaceRole(NetworkControlRole {
                roleId: roleId.clone(),
                displayName: displayName.clone(),
                capabilities,
            })?;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!({
                    "roleId": roleId,
                    "name": displayName,
                    "defined": true
                }));
            } else {
                println!("Identity created: {displayName}");
            }
            Ok(())
        }
        [command, deviceName, roleName] if command == "set" => {
            let state = service.deviceSpaceControl()?;
            let topology = service.deviceSpaceTopology()?;
            let deviceId = network_device_id(&topology, deviceName)?;
            let roleId = network_role_id(&state, roleName)?;
            let roleLabel = state
                .roles
                .get(&roleId)
                .map(|role| role.displayName.clone())
                .ok_or_else(|| format!("network role does not exist: {roleName}"))?;
            service.setDeviceSpaceIdentity(NetworkControlIdentityAssignment {
                nodeId: deviceId.clone(),
                roleId: roleId.clone(),
            })?;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!({
                    "deviceId": deviceId,
                    "identityId": roleId,
                    "set": true
                }));
            } else {
                println!("Set identity \"{roleLabel}\" on \"{deviceName}\"");
            }
            Ok(())
        }
        [command, deviceName] if command == "clear" => {
            let topology = service.deviceSpaceTopology()?;
            let deviceId = network_device_id(&topology, deviceName)?;
            service.clearDeviceSpaceIdentity(deviceId.clone())?;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!({ "deviceId": deviceId, "cleared": true }));
            } else {
                println!("Reset identity of \"{deviceName}\" to the default user");
            }
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    }
}

/// Runs member removal and connection prohibition commands.
async fn run_link_control_device_command(
    service: &operit_node_runtime::RuntimeRemoteLinkService::RuntimeRemoteLinkService,
    args: &[String],
) -> Result<(), String> {
    const USAGE: &str = "usage: operit2 cli link control device <list|admit <device-name>|remove <device-name>|disconnect <device-name>>";
    match args {
        [command] if command == "list" => {
            let topology = service.deviceSpaceTopology()?;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!(topology.devices));
            } else {
                for label in network_device_labels(&topology).values() {
                    println!("{label}");
                }
            }
            Ok(())
        }
        [command, deviceName] if matches!(command.as_str(), "admit" | "remove" | "disconnect") => {
            let topology = service.deviceSpaceTopology()?;
            let deviceId = network_device_id(&topology, deviceName)?;
            let deviceLabel = network_device_label_by_id(&topology, &deviceId)?;
            if command == "admit" {
                service.admitDeviceSpaceMember(deviceId)?;
            } else if command == "remove" {
                service.removeDeviceSpaceMember(deviceId).await?;
            } else {
                service.disconnectDeviceSpaceNode(deviceId).await?;
            }
            if cli_json_mode() {
                emit_cli_json(serde_json::json!({ command: true }));
            } else {
                println!("{} device \"{}\"", command, deviceLabel);
            }
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    }
}

/// Updates one explicitly named policy value.
fn run_link_control_policy_command(
    service: &operit_node_runtime::RuntimeRemoteLinkService::RuntimeRemoteLinkService,
    args: &[String],
) -> Result<(), String> {
    const USAGE: &str = "usage: operit2 cli link control policy <list|set <policy-name> <value>>";
    match args {
        [command] if command == "list" => {
            let policies = service.deviceSpaceControl()?.policies;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!(policies));
            } else {
                for (name, value) in policies {
                    println!("{name}: {value}");
                }
            }
            Ok(())
        }
        [command, policyId, value] if command == "set" => {
            service.updateDeviceSpacePolicy(policyId.clone(), value.clone())?;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!({ "policy": policyId, "updated": true }));
            } else {
                println!("Policy updated: {policyId}");
            }
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    }
}

/// 通过节点身份验证实际路由的 StateFlow 和嵌入流，不读取底层 session。
async fn run_link_stream_probe_command(args: &[String]) -> Result<(), String> {
    let json_mode = cli_json_mode();
    let name = args
        .get(0)
        .ok_or_else(|| "usage: operit2 cli link stream-probe <node-id>".to_string())?;
    let coreApplication = create_cli_core_application_without_space_sync("client").await?;
    let peerNodeId = name.clone();
    let service = coreApplication.accessServices();
    let space = service.joinPairedDeviceSpace(name.clone()).await?;
    if !json_mode {
        println!(
            "Probe joined space {name} on {} ({} members)",
            peerNodeId,
            space.members.len()
        );
    }

    let chatId = format!("route-probe-{}", link_probe_unix_millis());
    CoreNodeBindingStore::new(coreApplication.nodeRuntime().runtimeStorageHost())?
        .create(&chatId, &peerNodeId)?;
    if !json_mode {
        println!(
            "Probe binding created for chat {chatId} -> {}",
            peerNodeId
        );
    }

    let target =
        operit_proxy_local::LocalCoreProxy::generatedTargetForSchema("chatRuntimeHolderMain")
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
            target,
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
    if !json_mode {
        println!(
            "Probe flow event {:?}: {} messages, {} content streams",
            flowEvent.kind,
            messages.len(),
            contentStreamCount
        );
    }
    if messages.is_empty() || contentStreamCount == 0 {
        return Err("probe flow did not expose a ChatMessage.contentStream".to_string());
    }

    let descriptor = find_core_stream_descriptor(&flowEvent.value)
        .ok_or_else(|| "probe flow did not contain a $coreStream descriptor".to_string())?;
    if !json_mode {
        println!(
            "Probe stream descriptor {} -> {}.{}",
            descriptor.streamId, descriptor.target, descriptor.propertyName
        );
    }
    if descriptor.target != CORE_STREAM_TARGET || descriptor.propertyName != "openCoreStream" {
        return Err("probe stream descriptor does not target the Core stream pool".to_string());
    }

    let mut embeddedStream = coreApplication
        .localClient()
        .watch(CoreWatchRequest::new(
            format!("route-probe-embedded-{chatId}"),
            CORE_STREAM_TARGET,
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
        if !json_mode {
            println!(
                "Probe stream event {:?}: {} {}",
                event.kind,
                markdown.eventType,
                markdown.value.clone().unwrap_or_default()
            );
        }
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
    if json_mode {
        emit_cli_json(serde_json::json!({
            "ok": true,
            "space": name,
            "remoteNode": peerNodeId,
            "chatId": chatId,
            "messages": messages.len(),
            "contentStreams": contentStreamCount,
            "changed": changedCount,
            "completed": completedCount,
            "text": chunkText,
        }));
    } else {
        println!("Probe succeeded: {changedCount} changes, {completedCount} completion, text: {chunkText}");
    }
    Ok(())
}

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

/// Prints Link command usage in the selected output format.
pub(crate) fn print_link_usage() {
    if cli_json_mode() {
        emit_cli_json(
            serde_json::json!({ "usage": "operit2 cli link <discover|qr|pair|unpair|peers|prompts|listen|session|space|doctor|control|stream-probe|edge-call>" }),
        );
        return;
    }
    println!("operit2 cli link pair [device-name|connection-code]  # discover, choose device, enter six-digit code");
    println!("operit2 cli link discover [--timeout-ms <ms>]");
    println!("operit2 cli link qr [http|ws|tcp] [--bind <host:port>] [--no-discovery]  # terminal QR; reuses daemon/TUI when running");
    println!("  In a live link session: qr | pair [device-name|connection-code] | confirm <six-digit-code> | pair-cancel");
    println!("operit2 cli link doctor [--repair]  # shared-Space health diagnosis and repair");
    println!("operit2 cli link unpair [device-name|number] | peers | prompts");
    println!("operit2 cli link listen <http|ws|tcp|serial|bluetooth> [--automatic-port|--fixed-port] [--bind <host:port>]");
    println!("operit2 cli link space <show|topology|status <device-name>|rename <name>|join [device-name|number]|disconnect <device-name>|admit <device-name>|remove <device-name>|leave>");
    println!("operit2 cli link space requests <incoming|outgoing> | refresh [space-name|number] | cancel [space-name|number]");
    println!("operit2 cli link space approve|reject [device-name|number] | sync  # a single pending request is selected automatically");
    println!("operit2 cli link space preferences <node-id> | preference-set <node-id> <key> <JSON> | preference-delete <node-id> <key>");
    println!("operit2 cli link session <http|ws|tcp> [--automatic-port|--fixed-port] [--bind <host:port>] [--no-discovery]  # live interactive management");
    println!("operit2 cli link control <show|bootstrap|audit|identity|device|policy>");
    println!("  identity list|define <name> <all|audit|relay|storage|execute|view|manage-identities|assign-identity|approve|join>...|set <device-name> <identity-name>|clear <device-name>");
    println!("  device list|admit <device-name>|remove <device-name>|disconnect <device-name>");
    println!("operit2 cli link stream-probe <node-id>");
    println!("operit2 cli link edge-call <device> <edge-target> <method> [json-args]");
}

#[cfg(test)]
mod session_command_tests {
    use super::{parse_session_command, select_link_choice};

    #[test]
    fn single_candidate_needs_no_selector_but_multiple_candidates_do() {
        assert_eq!(select_link_choice(&["Phone"], None, "device").unwrap(), 0);
        let error = select_link_choice(&["Phone", "Server"], None, "device").unwrap_err();
        assert!(error.contains("1. Phone") && error.contains("2. Server"));
        assert!(select_link_choice(&[], None, "device").is_err());
    }

    #[test]
    fn duplicate_and_numeric_names_cannot_silently_select_another_device() {
        assert!(select_link_choice(&["Phone", "Phone"], Some("Phone"), "device").is_err());
        assert_eq!(select_link_choice(&["Phone", "Phone"], Some("2"), "device").unwrap(), 1);
        assert_eq!(select_link_choice(&["Server", "1"], Some("1"), "device").unwrap(), 1);
        for selector in ["0", "3", "unknown", "-1"] {
            assert!(select_link_choice(&["Phone", "Server"], Some(selector), "device").is_err());
        }
    }

    #[test]
    fn accepts_simple_commands_and_json_arguments_with_spaces() {
        assert_eq!(parse_session_command("space requests incoming").unwrap(), vec!["space", "requests", "incoming"]);
        assert_eq!(parse_session_command(r#"["space", "rename", "My Space"]"#).unwrap(), vec!["space", "rename", "My Space"]);
        assert!(parse_session_command("  ").unwrap().is_empty());
    }

    #[test]
    fn invalid_json_or_non_string_arguments_are_rejected() {
        assert!(parse_session_command("[broken").is_err());
        assert!(parse_session_command(r#"["space", 1]"#).is_err());
    }
}
