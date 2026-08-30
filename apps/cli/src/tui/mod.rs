#[path = "core/app.rs"]
mod app;
#[path = "core/approval.rs"]
mod approval;
#[path = "input/commands.rs"]
mod commands;
#[path = "config/mod.rs"]
mod config;
#[path = "transcript/empty_state.rs"]
mod empty_state;
#[path = "core/focus.rs"]
mod focus;
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
#[path = "input/pending_queue.rs"]
mod pending_queue;
#[path = "view/render.rs"]
mod render;
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
    StartupUpdatePrompt,
};
use approval::TuiApprovalBridge;
use i18n::TuiLanguage;
use link_proxy_rs::tui_core;
use operit_access_runtime::{RemoteLinkServer, RemoteLinkServerConfig};
use operit_core_application::CoreApplication;
use operit_providers::chat::enhance::ConversationService::ConversationService;
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::data::preferences::ApiPreferences::ApiPreferences;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_util::GithubReleaseUtil::{FullUpdateStatus, FullUpdateTarget, GithubReleaseUtil};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use crate::{
    create_cli_core_application_configured, initialize_shell_chat, parse_shell_args, ShellArgs,
};

#[derive(Clone, Debug)]
struct TuiLinkServerArgs {
    bindAddress: String,
    token: String,
}

#[derive(Clone, Debug, Default)]
struct TuiLinkStartupArgs {
    server: Option<TuiLinkServerArgs>,
    joinSessions: Vec<String>,
}

/// Runs the local TUI directly against the local Core after completing setup.
pub(crate) async fn run_tui_command(args: &[String]) -> Result<(), String> {
    let (shell_args, link_args) = parse_tui_startup_args(args)?;
    let approval_bridge = TuiApprovalBridge::new();
    let initial_chat_id_cell = Arc::new(StdMutex::new(None::<String>));
    let language_cell = Arc::new(StdMutex::new(None::<TuiLanguage>));
    let shell_args_for_core = shell_args.clone();
    let approval_bridge_for_core = approval_bridge.clone();
    let initial_chat_id_for_core = initial_chat_id_cell.clone();
    let language_for_core = language_cell.clone();
    let core_application = create_cli_core_application_configured("client", move |local_core| {
        let language = {
            let application = local_core.localApplicationMut();
            TuiLanguage::from_context(&application.hostManager)?
        };
        let initial_chat_id =
            initialize_shell_chat(local_core.localApplicationMut(), &shell_args_for_core)?;
        install_local_permission_requester(local_core, approval_bridge_for_core);
        *language_for_core
            .lock()
            .expect("TUI language cell lock must not be poisoned") = Some(language);
        *initial_chat_id_for_core
            .lock()
            .expect("TUI initial chat cell lock must not be poisoned") = Some(initial_chat_id);
        Ok(())
    })
    .await?;
    let link_server_task = start_tui_link_server(&core_application, &link_args).await?;
    join_tui_link_sessions(&core_application, &link_args).await?;
    let language = language_cell
        .lock()
        .expect("TUI language cell lock must not be poisoned")
        .take()
        .expect("TUI language must be initialized by CoreApplication startup");
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
        shell_args,
        initial_chat_id,
        approval_bridge,
        language,
        startup_install_prompt,
        startup_update_prompt,
        startup_workspace_prompt_path,
    )
    .await?;
    let result = tui.run().await;
    drop(tui);
    stop_tui_link_server(link_server_task).await;
    core_application.shutdown().await;
    result
}

/// Splits TUI Link startup arguments from normal shell startup arguments.
fn parse_tui_startup_args(args: &[String]) -> Result<(ShellArgs, TuiLinkStartupArgs), String> {
    let usage = "usage: operit2 tui [--link-server --link-bind <addr:port> --link-token <token>] [--link-join <session>] [--chat <chat-id>] [--resume] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>] [--update-current-version <version>]";
    let mut shell_arg_tokens = Vec::new();
    let mut link_args = TuiLinkStartupArgs::default();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--link-server" => {
                link_args.server = Some(TuiLinkServerArgs {
                    bindAddress: "0.0.0.0:37192".to_string(),
                    token: "operit-link-dev".to_string(),
                });
            }
            "--link-bind" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| usage.to_string())?.clone();
                let server = link_args.server.as_mut().ok_or_else(|| usage.to_string())?;
                server.bindAddress = value;
            }
            "--link-token" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| usage.to_string())?.clone();
                let server = link_args.server.as_mut().ok_or_else(|| usage.to_string())?;
                server.token = value;
            }
            "--link-join" => {
                index += 1;
                link_args
                    .joinSessions
                    .push(args.get(index).ok_or_else(|| usage.to_string())?.clone());
            }
            value => shell_arg_tokens.push(value.to_string()),
        }
        index += 1;
    }
    let shell_args = parse_shell_args(&shell_arg_tokens).map_err(|_| usage.to_string())?;
    Ok((shell_args, link_args))
}

/// Starts a TUI-owned Link server on the current Core tree.
async fn start_tui_link_server(
    core_application: &CoreApplication,
    link_args: &TuiLinkStartupArgs,
) -> Result<Option<tokio::task::JoinHandle<Result<(), String>>>, String> {
    let Some(server) = link_args.server.clone() else {
        return Ok(None);
    };
    let access_identity = core_application.accessIdentity().clone();
    let config = RemoteLinkServerConfig {
        bindAddress: server.bindAddress,
        token: server.token,
        deviceId: access_identity.deviceId,
        deviceInfo: access_identity.deviceInfo,
        webAccess: None,
        printStartupInfo: true,
        accessStore: core_application.accessStore(),
    };
    let node_router = core_application.nodeRouter();
    let mut task = tokio::spawn(async move { RemoteLinkServer::serve(node_router, config).await });
    tokio::select! {
        outcome = &mut task => {
            match outcome {
                Ok(Ok(())) => Ok(None),
                Ok(Err(error)) => Err(error),
                Err(error) => Err(error.to_string()),
            }
        }
        _ = tokio::time::sleep(Duration::from_millis(150)) => Ok(Some(task)),
    }
}

/// Joins configured paired device spaces inside the TUI Core process.
async fn join_tui_link_sessions(
    core_application: &CoreApplication,
    link_args: &TuiLinkStartupArgs,
) -> Result<(), String> {
    let service = core_application.accessServices();
    for session in &link_args.joinSessions {
        let space = service.joinPairedDeviceSpace(session.clone()).await?;
        println!(
            "tui link joined session={} space={} members={}",
            session,
            space.spaceName,
            space.members.len()
        );
    }
    Ok(())
}

/// Stops the TUI-owned Link server task before Core shutdown.
async fn stop_tui_link_server(task: Option<tokio::task::JoinHandle<Result<(), String>>>) {
    if let Some(task) = task {
        task.abort();
        let _ = task.await;
    }
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

fn install_local_permission_requester(
    core: &mut operit_proxy_local::LocalCoreProxy,
    approval_bridge: TuiApprovalBridge,
) {
    let handler = core.localApplicationMut().toolHandler.clone();
    handler
        .getToolPermissionSystem()
        .setAsyncPermissionRequester(move |tool, description| {
            let approval_bridge = approval_bridge.clone();
            async move {
                tokio::task::spawn_blocking(move || approval_bridge.request(&tool, &description))
                    .await
                    .expect("tool approval task failed")
            }
        });
}
