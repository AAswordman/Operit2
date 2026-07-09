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
#[path = "transcript/response_stream.rs"]
mod response_stream;
#[path = "transcript/selection.rs"]
mod selection;
#[path = "transcript/stream_markdown.rs"]
mod stream_markdown;
#[path = "view/theme.rs"]
mod theme;
#[path = "transcript/transcript.rs"]
mod transcript;
#[path = "transcript/typewriter.rs"]
mod typewriter;

use crate::access::{
    PairedRemoteSession, PairedRemoteSessionRecord, RemoteLinkClient, RemoteLinkServer,
    RemoteLinkServerConfig,
};
use app::{
    FullUpdateDownloadState, OperitTui, StartupInstallPrompt, StartupInstallState,
    StartupUpdatePrompt,
};
use approval::TuiApprovalBridge;
use i18n::TuiLanguage;
use link_proxy_rs::tui_core;
use operit_core_proxy::GeneratedCoreProxy;
use operit_link::{CoreCallRequest, CoreLinkClient, CoreObjectPath, CoreWatchRequest};
use operit_providers::chat::enhance::ConversationService::ConversationService;
use operit_tools::ToolExecutionManager::{AITool, ToolParameter};
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::tools::ToolPermissionSystem::PermissionRequestResult;
use operit_runtime::data::preferences::ApiPreferences::ApiPreferences;
use operit_runtime::services::RuntimeHostInteractionService::{
    RuntimeHostInteractionKind, RuntimeHostInteractionRequest, RuntimeHostInteractionResponse,
    RuntimeHostInteractionToolPermissionResponse, RuntimeHostInteractionToolPermissionTool,
};
use operit_util::GithubReleaseUtil::{
    FullUpdateStatus, FullUpdateTarget, GithubReleaseUtil,
};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cli::link::load_link_session;
use crate::{create_local_core, initialize_shell_chat, parse_shell_args};

pub(crate) async fn run_tui_command(args: &[String]) -> Result<(), String> {
    let shell_args = parse_shell_args(args)?;
    let mut core = create_local_core();
    core.localApplicationMut().onCreate()?;
    let language = TuiLanguage::from_context(&core.localApplicationMut().hostManager)?;
    let initial_chat_id = initialize_shell_chat(core.localApplicationMut(), &shell_args)?;
    let approval_bridge = TuiApprovalBridge::new();
    install_local_permission_requester(&mut core, approval_bridge.clone());
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
        tui_core(core),
        shell_args,
        initial_chat_id,
        approval_bridge,
        language,
        startup_install_prompt,
        startup_update_prompt,
        startup_workspace_prompt_path,
    )
    .await?;
    tui.run().await
}

pub(crate) async fn run_link_tui_command(args: &[String]) -> Result<(), String> {
    let session_name = args
        .get(0)
        .ok_or_else(|| {
            "usage: operit2 cli link tui <session> [--chat <chat-id>] [--resume]".to_string()
        })?
        .clone();
    let shell_args = parse_shell_args(&args[1..])?;
    let session = load_link_session(&session_name)?;
    let local_application = crate::bootstrap::create_cli_application();
    let language = TuiLanguage::from_context(&local_application.hostManager)?;
    let host_interaction_session = session.clone();
    let mut core = tui_core(session);
    let initial_chat_id = initialize_remote_chat(&mut core, &shell_args).await?;
    let approval_bridge = TuiApprovalBridge::new();
    start_remote_host_interaction_loop(host_interaction_session, approval_bridge.clone());
    let mut tui = OperitTui::new(
        core,
        shell_args,
        initial_chat_id,
        approval_bridge,
        language,
        None,
        None,
        None,
    )
    .await?;
    tui.run().await
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
    core: &mut operit_core_proxy::LocalCoreProxy,
    approval_bridge: TuiApprovalBridge,
) {
    let context = core.localApplicationMut().hostManager.clone();
    let handler = AIToolHandler::getInstance(context);
    handler
        .getToolPermissionSystem()
        .setPermissionRequester(move |tool, description| {
            approval_bridge.request(tool, description)
        });
}

fn start_remote_host_interaction_loop(
    session: PairedRemoteSession,
    approval_bridge: TuiApprovalBridge,
) {
    tokio::spawn(async move {
        let mut proxy =
            GeneratedCoreProxy::new(Box::new(session) as Box<dyn CoreLinkClient + Send>);
        let mut stream = proxy
            .services_runtime_host_interaction_service()
            .ownerHostInteractionEvents(vec![RuntimeHostInteractionKind::ToolPermission])
            .await
            .expect("remote host interaction stream must open");
        while let Some(event) = stream.recv().await {
            let request: RuntimeHostInteractionRequest =
                serde_json::from_value(event.value).expect("host interaction event must be typed");
            handle_remote_tool_permission_interaction(&mut proxy, approval_bridge.clone(), request)
                .await;
        }
    });
}

async fn handle_remote_tool_permission_interaction(
    proxy: &mut GeneratedCoreProxy<Box<dyn CoreLinkClient + Send>>,
    approval_bridge: TuiApprovalBridge,
    request: RuntimeHostInteractionRequest,
) {
    let payload = request
        .toolPermission
        .expect("tool permission payload must be present");
    let tool = tool_from_permission_payload(&payload.tool);
    let description = payload.description;
    let result =
        match tokio::task::spawn_blocking(move || approval_bridge.request(&tool, &description))
            .await
        {
            Ok(result) => result,
            Err(error) => panic!("tool approval task failed: {error}"),
        };
    let result = match result {
        PermissionRequestResult::ALLOW => "allow",
        PermissionRequestResult::DENY => "deny",
        PermissionRequestResult::ALLOW_SESSION => "always_allow",
    };
    proxy
        .services_runtime_host_interaction_service()
        .respondOwnerHostInteraction(
            request.requestId,
            RuntimeHostInteractionResponse {
                browserAutomation: None,
                webVisit: None,
                composeWebViewController: None,
                systemCaptureScreenshot: None,
                systemRecognizeText: None,
                audioPlay: None,
                musicPlayback: None,
                bluetooth: None,
                ttsSynthesis: None,
                ttsPlayback: None,
                toolPermission: Some(RuntimeHostInteractionToolPermissionResponse {
                    result: result.to_string(),
                }),
            },
        )
        .await
        .expect("tool permission response must be accepted");
}

fn tool_from_permission_payload(tool: &RuntimeHostInteractionToolPermissionTool) -> AITool {
    AITool {
        name: tool.name.clone(),
        parameters: tool
            .parameters
            .iter()
            .map(|parameter| ToolParameter {
                name: parameter.name.clone(),
                value: parameter.value.clone(),
            })
            .collect(),
    }
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
    } else if shell_args.resume {
        let chat_histories = core
            .chat_runtime_holder_main()
            .chatHistoriesFlowSnapshot()
            .await
            .map_err(|error| error.to_string())?;
        let chat_id = chat_histories
            .into_iter()
            .max_by(|left, right| {
                let left_updated = left
                    .updatedAt
                    .parse::<i64>()
                    .expect("chat.updatedAt must be epoch millis");
                let right_updated = right
                    .updatedAt
                    .parse::<i64>()
                    .expect("chat.updatedAt must be epoch millis");
                left_updated
                    .cmp(&right_updated)
                    .then_with(|| right.displayOrder.cmp(&left.displayOrder))
            })
            .map(|chat| chat.id)
            .ok_or_else(|| "no previous chat to resume".to_string())?;
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
