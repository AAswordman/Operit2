mod app;
mod approval;
mod commands;
mod empty_state;
mod helpers;
mod input;
mod link_proxy_rs;
mod markdown;
mod render;
mod theme;
mod typewriter;

use app::{FullUpdateDownloadState, OperitTui, StartupUpdatePrompt};
use approval::TuiApprovalBridge;
use link_proxy_rs::tui_core;
use crate::access::{
    PairedRemoteSession, PairedRemoteSessionRecord, RemoteHostInteractionBroker,
    RemoteHostInteractionRequest, RemoteLinkClient, RemoteLinkServer, RemoteLinkServerConfig,
};
use operit_link::{CoreCallRequest, CoreLinkClient, CoreObjectPath, CoreWatchRequest};
use operit_runtime::api::chat::enhance::ConversationService::ConversationService;
use operit_runtime::api::chat::enhance::ToolExecutionManager::{AITool, ToolParameter};
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::api::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::tools::AIToolHandler::AIToolHandler;
use operit_runtime::core::tools::ToolPermissionSystem::PermissionRequestResult;
use operit_runtime::data::preferences::ApiPreferences::ApiPreferences;
use operit_runtime::util::GithubReleaseUtil::{
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
    let _externalRuntimeEventRegistration =
        operit_runtime::core::application::ExternalRuntimeEventSupport::startExternalRuntimeEventSupport(
            core.localApplicationMut().applicationContext.clone(),
            "cli-tui",
        )?;
    let initial_chat_id = initialize_shell_chat(core.localApplicationMut(), &shell_args)?;
    let approval_bridge = TuiApprovalBridge::new();
    install_local_permission_requester(&mut core, approval_bridge.clone());
    let startup_update_prompt = build_startup_update_prompt().await?;
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
        None,
        None,
    )
    .await?;
    tui.run().await
}

async fn build_startup_update_prompt() -> Result<Option<StartupUpdatePrompt>, String> {
    let target = FullUpdateTarget::cliForCurrentHost()?;
    let status =
        match GithubReleaseUtil::checkForFullUpdate(env!("CARGO_PKG_VERSION"), target).await {
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
    let context = core.localApplicationMut().applicationContext.clone();
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
        loop {
            let request = match session.pollHostInteraction(500).await {
                Ok(Some(request)) => request,
                Ok(None) => continue,
                Err(_) => break,
            };
            if request.kind == "tool_permission" {
                handle_remote_tool_permission_interaction(
                    session.clone(),
                    approval_bridge.clone(),
                    request,
                )
                .await;
            }
        }
    });
}

async fn handle_remote_tool_permission_interaction(
    session: PairedRemoteSession,
    approval_bridge: TuiApprovalBridge,
    request: RemoteHostInteractionRequest,
) {
    let Some(tool) = tool_from_interaction_payload(&request.payload) else {
        let _ = session
            .respondHostInteraction(&request.requestId, serde_json::json!({"result": "deny"}))
            .await;
        return;
    };
    let Some(description) = request
        .payload
        .get("description")
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
    else {
        let _ = session
            .respondHostInteraction(&request.requestId, serde_json::json!({"result": "deny"}))
            .await;
        return;
    };
    let result =
        match tokio::task::spawn_blocking(move || approval_bridge.request(&tool, &description))
            .await
        {
            Ok(result) => result,
            Err(_) => PermissionRequestResult::DENY,
        };
    let result = match result {
        PermissionRequestResult::ALLOW => "allow",
        PermissionRequestResult::DENY => "deny",
        PermissionRequestResult::ALWAYS_ALLOW => "always_allow",
    };
    let _ = session
        .respondHostInteraction(&request.requestId, serde_json::json!({"result": result}))
        .await;
}

fn tool_from_interaction_payload(value: &serde_json::Value) -> Option<AITool> {
    let tool = value.get("tool")?;
    let name = tool.get("name")?.as_str()?.to_string();
    let parameters = tool
        .get("parameters")?
        .as_array()?
        .iter()
        .map(|parameter| {
            Some(ToolParameter {
                name: parameter.get("name")?.as_str()?.to_string(),
                value: parameter.get("value")?.as_str()?.to_string(),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(AITool { name, parameters })
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
