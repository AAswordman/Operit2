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

use crate::{create_cli_core_application_configured, initialize_shell_chat, parse_shell_args};

/// Runs the local TUI directly against the local Core after completing setup.
pub(crate) async fn run_tui_command(args: &[String]) -> Result<(), String> {
    let shell_args = parse_shell_args(args)?;
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
    core_application.shutdown().await;
    result
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
