#[path = "core/app.rs"]
mod app;
#[path = "input/commands.rs"]
mod commands;
#[path = "compose/mod.rs"]
mod compose;
#[path = "config/mod.rs"]
mod config;
#[path = "transcript/empty_state.rs"]
mod empty_state;
#[path = "core/focus.rs"]
mod focus;
#[path = "transcript/fold.rs"]
mod fold;
#[path = "transcript/helpers.rs"]
mod helpers;
#[path = "i18n.rs"]
mod i18n;
#[path = "input/input.rs"]
mod input;
#[path = "core/link_proxy_rs.rs"]
mod link_proxy_rs;
#[path = "transcript/markdown.rs"]
mod markdown;
#[path = "core/outgoing_joins.rs"]
mod outgoing_joins;
#[path = "input/pending_queue.rs"]
mod pending_queue;
#[path = "core/peer_discovery.rs"]
mod peer_discovery;
#[path = "view/render.rs"]
mod render;
#[path = "view/scrollbar.rs"]
mod scrollbar;
#[path = "transcript/selection.rs"]
mod selection;
#[path = "view/theme.rs"]
mod theme;
#[path = "transcript/transcript.rs"]
mod transcript;
#[path = "transcript/typewriter.rs"]
mod typewriter;

use app::{
    FullUpdateDownloadState, OperitTui, StartupInstallPrompt, StartupInstallState,
    StartupUpdatePrompt, parse_peer_transport,
};
use i18n::TuiLanguage;
use link_proxy_rs::tui_core;
use operit_node_runtime::NodeServices::{DiscoveredPeer, PairingPrompt, PeerTransport};
use operit_node_runtime::RuntimeRemoteLinkService::{RuntimeRemoteLinkService, SpaceJoinRequest};
use operit_node_runtime::RuntimePeerService::RuntimePeerService;
use operit_core_application::CoreApplication;
use operit_providers::chat::enhance::ConversationService::ConversationService;
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::data::preferences::ApiPreferences::ApiPreferences;
use operit_runtime::services::RuntimeHostInteractionService::{
    requestChatToolPermissionAsync, RuntimeHostInteractionToolPermissionTool,
    RuntimeHostInteractionToolPermissionToolParameter,
};
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::tools::ToolPermissionSystem::PermissionRequestResult;
use operit_tools::ToolExecutionManager::AITool;
use operit_util::AppLogger::AppLogger;
use operit_util::GithubReleaseUtil::{FullUpdateStatus, FullUpdateTarget, GithubReleaseUtil};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex as StdMutex};
use std::time::Duration;

use crate::{
    create_cli_core_application_configured_with_toast_host, initialize_shell_chat,
    parse_shell_args, ShellArgs,
};

#[derive(Clone, Debug, Default)]
struct TuiLinkStartupArgs {
    listen: Option<PeerTransport>,
    joinNodes: Vec<String>,
}

/// Runs the local TUI directly against the local Core after completing setup.
pub(crate) async fn run_tui_command(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{}", tui_usage_text());
        return Ok(());
    }
    let (shell_args, link_args) = parse_tui_startup_args(args)?;
    let initial_chat_id_cell = Arc::new(StdMutex::new(None::<String>));
    let language_cell = Arc::new(StdMutex::new(None::<TuiLanguage>));
    let (toast_sender, toast_receiver) = mpsc::channel::<String>();
    let toast_host = tui_toast_host(toast_sender);
    let (network_event_sender, network_event_receiver) = mpsc::channel::<NetworkUiEvent>();
    let shell_args_for_core = shell_args.clone();
    let initial_chat_id_for_core = initial_chat_id_cell.clone();
    let language_for_core = language_cell.clone();
    let core_application = create_cli_core_application_configured_with_toast_host(
        "client",
        toast_host,
        move |local_core| {
            let language = {
                let application = local_core.localApplicationMut();
                TuiLanguage::from_context(&application.hostManager)?
            };
            let initial_chat_id =
                initialize_shell_chat(local_core.localApplicationMut(), &shell_args_for_core)?;
            install_local_permission_requester(local_core);
            *language_for_core
                .lock()
                .expect("TUI language cell lock must not be poisoned") = Some(language);
            *initial_chat_id_for_core
                .lock()
                .expect("TUI initial chat cell lock must not be poisoned") = Some(initial_chat_id);
            Ok(())
        },
    )
    .await?;
    if let Some(transport) = link_args.listen {
        core_application.accessServices().startListening(vec![transport]).await
            .map_err(|error| error.to_string())?;
    }
    join_tui_paired_nodes(&core_application, &link_args).await?;
    let language = language_cell
        .lock()
        .expect("TUI language cell lock must not be poisoned")
        .take()
        .expect("TUI language must be initialized by CoreApplication startup");
    let network_peers = core_application.nodeServices()?.peers();
    let initial_chat_id = initial_chat_id_cell
        .lock()
        .expect("TUI initial chat cell lock must not be poisoned")
        .take()
        .expect("TUI initial chat must be initialized by CoreApplication startup");
    let startup_install_prompt = build_startup_install_prompt()?;
    let startup_update_prompt =
        build_startup_update_prompt(shell_args.updateCurrentVersion.as_deref()).await?;
    let startup_workspace_prompt_path = if shell_args.chatId.is_none() && !shell_args.resume {
        Some(
            std::env::current_dir()
                .map_err(|error| error.to_string())?
                .to_string_lossy()
                .replace('\\', "/"),
        )
    } else {
        None
    };
    let mut tui = OperitTui::new(
        tui_core(core_application.localClient()),
        core_application.accessServices(),
        shell_args,
        initial_chat_id,
        language,
        startup_install_prompt,
        startup_update_prompt,
        startup_workspace_prompt_path,
        toast_receiver,
        network_event_receiver,
        network_event_sender.clone(),
    )
    .await?;
    // Runtime peer/proxy futures are not Send. Keep the monitor on the local
    // executor, independently of the terminal loop, and drop its in-flight
    // operations before shutting down the Core.
    let result = tokio::task::LocalSet::new()
        .run_until(async {
            let network_event_task = spawn_network_ui_events(
                network_peers,
                core_application.accessServices(),
                network_event_sender.clone(),
            );
            let outgoing_join_task = outgoing_joins::spawn_outgoing_join_monitor(
                core_application.accessServices(),
                network_event_sender,
            );
            let result = tui.run().await;
            outgoing_join_task.abort();
            network_event_task.abort();
            let _ = outgoing_join_task.await;
            let _ = network_event_task.await;
            result
        })
        .await;
    drop(tui);
    core_application.shutdown().await;
    result
}

/// Structured network events the watcher pushes to the TUI event loop.
pub(crate) enum NetworkUiEvent {
    /// A background refresh reached a terminal join status. Applying this
    /// event must not perform another remote call on the terminal loop.
    OutgoingJoinSettled(SpaceJoinRequest),
    /// Snapshots fetched in the background after peer changes or a settled join.
    Snapshot {
        prompts: Vec<PairingPrompt>,
        requests: Option<Vec<SpaceJoinRequest>>,
    },
    /// A background LAN scan finished; the candidate list opens on the loop.
    DiscoveryFinished(Result<Vec<DiscoveredPeer>, String>),
}

/// Fetches change-triggered snapshots off the terminal loop. Peer futures are
/// not Send, so this watcher shares the local executor with the join monitor.
fn spawn_network_ui_events(
    peers: Arc<dyn RuntimePeerService>,
    network: RuntimeRemoteLinkService,
    events: mpsc::Sender<NetworkUiEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_local(async move {
        let mut changes = peers.subscribePeerChanges();
        loop {
            match changes.recv().await {
                Ok(()) => {}
                // A lagged receiver recovers on the next recv; only a stopped
                // peer service ends the watcher.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
            if events
                .send(outgoing_joins::network_snapshot(&network).await)
                .is_err()
            {
                break;
            }
        }
    })
}

/// Creates the toast host that feeds the active TUI event loop.
fn tui_toast_host(sender: mpsc::Sender<String>) -> Arc<dyn operit_host_api::ToastHost> {
    Arc::new(move |message: &str| {
        sender.send(message.to_string()).map_err(|error| {
            operit_host_api::HostError::new(format!("TUI toast delivery failed: {error}"))
        })
    })
}

/// Returns the TUI startup usage text, shared by `tui --help` and argument
/// parsing errors.
fn tui_usage_text() -> &'static str {
    "usage: operit2 tui [--link-listen <http|ws|tcp|serial|bluetooth>] [--link-join <node-id>] [--chat <chat-id>] [--resume] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>] [--update-current-version <version>]"
}

/// Splits TUI Link startup arguments from normal shell startup arguments.
fn parse_tui_startup_args(args: &[String]) -> Result<(ShellArgs, TuiLinkStartupArgs), String> {
    let usage = tui_usage_text();
    let mut shell_arg_tokens = Vec::new();
    let mut link_args = TuiLinkStartupArgs::default();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--link-listen" => {
                index += 1;
                link_args.listen = Some(
                    parse_peer_transport(args.get(index).map(String::as_str).unwrap_or_default())
                        .map_err(|_| usage.to_string())?,
                );
            }
            "--link-join" => {
                index += 1;
                link_args
                    .joinNodes
                    .push(args.get(index).ok_or_else(|| usage.to_string())?.clone());
            }
            value => shell_arg_tokens.push(value.to_string()),
        }
        index += 1;
    }
    let shell_args = parse_shell_args(&shell_arg_tokens).map_err(|_| usage.to_string())?;
    Ok((shell_args, link_args))
}

/// Joins configured paired device spaces inside the TUI Core process.
async fn join_tui_paired_nodes(
    core_application: &CoreApplication,
    link_args: &TuiLinkStartupArgs,
) -> Result<(), String> {
    let service = core_application.accessServices();
    for nodeId in &link_args.joinNodes {
        let space = service.joinPairedDeviceSpace(nodeId.clone()).await?;
        println!(
            "tui link joined node={} space={} members={}",
            nodeId,
            space.spaceName,
            space.members.len()
        );
    }
    Ok(())
}

fn build_startup_install_prompt() -> Result<Option<StartupInstallPrompt>, String> {
    if crate::cli::cli_is_installed()? {
        return Ok(None);
    }
    if startup_install_prompt_declined()? {
        return Ok(None);
    }
    Ok(Some(StartupInstallPrompt {
        install_selected: true,
        state: StartupInstallState::Ready,
        progress_rx: None,
    }))
}

fn startup_install_prompt_declined_path() -> PathBuf {
    crate::client_paths::client_root_dir().join("startup_install_prompt_declined")
}

fn startup_install_prompt_declined() -> Result<bool, String> {
    match fs::metadata(startup_install_prompt_declined_path()) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.to_string()),
    }
}

pub(crate) fn mark_startup_install_prompt_declined() -> Result<(), String> {
    let path = startup_install_prompt_declined_path();
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid path: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    fs::write(path, b"declined\n").map_err(|error| error.to_string())
}

async fn build_startup_update_prompt(
    current_version_override: Option<&str>,
) -> Result<Option<StartupUpdatePrompt>, String> {
    let target = FullUpdateTarget::cliForCurrentHost()?;
    let current_version = current_version_override.unwrap_or(env!("CARGO_PKG_VERSION"));
    let status = match GithubReleaseUtil::checkForFullUpdate(current_version, target).await {
        Ok(status) => status,
        Err(_) => return Ok(None),
    };
    match status {
        FullUpdateStatus::Available(release_info) => Ok(Some(StartupUpdatePrompt {
            release_info: Some(release_info),
            download_selected: true,
            download_state: FullUpdateDownloadState::Ready,
            progress_rx: None,
        })),
        FullUpdateStatus::UpToDate => Ok(None),
    }
}

/// Installs the chat-scoped permission requester so tool approvals become
/// chat state that every reader of the chat can see and answer, matching the
/// Flutter bridge and the headless CLI.
fn install_local_permission_requester(core: &mut operit_proxy_local::LocalCoreProxy) {
    let handler = core.localApplicationMut().toolHandler.clone();
    handler
        .getToolPermissionSystem()
        .setAsyncPermissionRequester(move |tool, description, chatId| async move {
            let Some(chatId) = chatId else {
                return PermissionRequestResult::DENY;
            };
            let response = match requestChatToolPermissionAsync(
                chatId,
                tool_to_permission_payload(&tool),
                description,
                std::time::Duration::from_secs(60),
            )
            .await
            {
                Ok(response) => response,
                Err(error) => {
                    AppLogger::e(
                        "TuiApproval",
                        &format!("permission request failed: {error}"),
                    );
                    return PermissionRequestResult::DENY;
                }
            };
            match response.as_str() {
                "allow" => PermissionRequestResult::ALLOW,
                "allow_session" => PermissionRequestResult::ALLOW_SESSION,
                "deny" => PermissionRequestResult::DENY,
                other => {
                    AppLogger::e("TuiApproval", &format!("unknown permission result: {other}"));
                    PermissionRequestResult::DENY
                }
            }
        });
}

/// Converts a typed tool into the chat permission request schema.
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
