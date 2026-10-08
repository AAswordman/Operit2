use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::{self, Stdout};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;
use serde::Deserialize;

use operit_model::ActivePrompt::ActivePrompt;
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_model::CharacterCard::CharacterCardChatModelBindingMode;
use operit_model::ChatHistory::ChatHistory;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatTurnOptions::ChatTurnOptions;
use operit_model::FunctionType::FunctionType;
use operit_model::InputProcessingState::InputProcessingState;
use operit_model::MessagePart::MessagePart;
use operit_model::MessagePartCodec::AssistantMarkupStreamState;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_node_runtime::NodeServices::{PairingPrompt, PeerTransport, PendingPairing};
use operit_node_runtime::RuntimeRemoteLinkService::{
    RuntimeDeviceSpaceDevice, RuntimeDeviceSpaceTopology, RuntimePairedDevice,
    RuntimeRemoteLinkService, SpaceJoinRequest, SpaceJoinStatus,
};
use operit_runtime::data::preferences::ModelConfigManager::ModelConfigManager;
use operit_runtime::services::ChatServiceCore::ChatState;
use operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionToolPermissionRequest;
use operit_tools::tools::ToolPermissionSystem::AiPermissionMode;
use operit_util::stream::TextStreamRevisionTracker::TextStreamRevisionTracker;
use operit_util::AppLogger::AppLogger;
use operit_util::GithubReleaseUtil::{
    FullUpdateProgressEvent, FullUpdateStage, FullUpdateTarget, ReleaseInfo,
};
use operit_util::MarkdownRenderStream::MarkdownStreamEvent;

use super::commands::{expand_plugin_command, keyword_options_for, TuiPluginCommandSpec};
use super::config;
use super::config::ConfigUi;
use super::helpers::{short_chat_label, split_command_line};
use super::i18n::{TuiLanguage, TuiText};
use super::link_proxy_rs::{TuiContentStreamEventInfo, TuiCore};
use super::pending_queue::PendingQueueMessage;
use super::scrollbar::{
    pointer_hits_scrollbar, scroll_position_for_pointer, scrollbar_hit_part, ScrollbarHit,
};
use super::selection::{
    apply_popup_selection_highlight, mouse_drag_transcript_position, mouse_popup_drag_position,
    mouse_popup_position, mouse_transcript_position, popup_rows_from_buffer, popup_selected_text,
    TranscriptCopyLine, TranscriptSelectionState,
};
use super::transcript::TranscriptRenderCache;
use super::typewriter::TypewriterState;
use crate::cli::network_control_ui::{
    network_capabilities, network_device_id, network_device_label_by_id, network_role_id,
    network_role_summary, new_network_control_id,
};
use crate::cli::CliInstallProgress;
use crate::tui::NetworkUiEvent;
use crate::{build_attachment_info, parse_shell_args, ChatSendArgs, ShellArgs};
use operit_store::NetworkControlStore::{NetworkControlIdentityAssignment, NetworkControlRole};

const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(16);
const RUNTIME_STATUS_REFRESH_INTERVAL: Duration = Duration::from_millis(250);
const OUTGOING_JOIN_REFRESH_INTERVAL: Duration = Duration::from_secs(5);
const TRANSIENT_STATUS_DURATION: Duration = Duration::from_secs(3);
const MAX_PENDING_TERMINAL_EVENTS_PER_FRAME: usize = 64;

pub(super) struct OperitTui {
    pub(super) compose: super::compose::ComposeHost,
    pub(super) core: TuiCore,
    networkControl: RuntimeRemoteLinkService,
    pub(super) initial_shell_args: ShellArgs,
    pub(super) current_chat_id_cache: Option<String>,
    pub(super) current_messages_cache: Vec<ChatMessage>,
    content_stream_states: BTreeMap<String, TuiMessageContentStreamState>,
    pub(super) current_chat_is_loading_cache: bool,
    pub(super) current_chat_input_processing_state_cache: InputProcessingState,
    pub(super) current_window_size_cache: i64,
    pub(super) chats: Vec<ChatListItem>,
    pub(super) selected_chat_index: usize,
    pub(super) model_choices: Vec<ModelChoiceItem>,
    pub(super) selected_model_choice_index: usize,
    pub(super) show_model_chooser: bool,
    pub(super) model_chooser_search: String,
    pub(super) model_chooser_filtered_indices: Vec<usize>,
    pub(super) model_list_mode: bool,
    pub(super) show_list_popup: bool,
    pub(super) list_popup_title: String,
    pub(super) list_popup_items: Vec<String>,
    pub(super) list_popup_search: String,
    pub(super) list_popup_filtered_indices: Vec<usize>,
    pub(super) list_popup_selected_index: usize,
    pub(super) focus: FocusArea,
    pub(super) input: String,
    pub(super) input_cursor: usize,
    pub(super) autocomplete_index: usize,
    pub(super) plugin_commands: Vec<TuiPluginCommandSpec>,
    pub(super) queued_attachment_paths: Vec<String>,
    pub(super) queued_inline_attachments: Vec<AttachmentInfo>,
    pub(super) queued_attachment_tokens: Vec<QueuedAttachmentToken>,
    pub(super) pending_queue_chat_id: Option<String>,
    pub(super) pending_queue_messages: VecDeque<PendingQueueMessage>,
    pub(super) selected_pending_queue_index: usize,
    pub(super) next_pending_queue_id: u64,
    pub(super) was_pending_queue_blocked: bool,
    pub(super) suppress_next_pending_queue_auto_send: bool,
    pub(super) pending_queue_auto_send_at: Option<Instant>,
    pub(super) pending_queue_manual_send: Option<PendingQueueMessage>,
    pub(super) paste_attachment_counter: usize,
    pub(super) status_message: String,
    route_permission_error: Option<String>,
    pub(super) status_message_expires_at: Option<Instant>,
    pub(super) transient_status_message: Option<String>,
    toast_receiver: mpsc::Receiver<String>,
    network_event_receiver: mpsc::Receiver<NetworkUiEvent>,
    seen_pairing_prompt_ids: BTreeSet<String>,
    seen_join_request_ids: BTreeSet<String>,
    /// Pairings this session started with `/network pair` and has not yet
    /// confirmed or cancelled; lets the follow-up commands resolve the id.
    pending_pairings: Vec<PendingPairing>,
    last_outgoing_join_refresh_at: Option<Instant>,
    /// Seeds the seen-id snapshots from the first fetch so a TUI start does
    /// not replay requests that predate the session.
    network_snapshots_seeded: bool,
    pub(super) context_usage_label: String,
    pub(super) transcript_scroll: u16,
    pub(super) transcript_viewport_height: u16,
    pub(super) transcript_max_scroll: u16,
    pub(super) follow_transcript: bool,
    pub(super) transcript_render_cache: TranscriptRenderCache,
    pub(super) transcript_area: Rect,
    pub(super) transcript_copy_lines: Vec<TranscriptCopyLine>,
    pub(super) transcript_selection: TranscriptSelectionState,
    pub(super) popup_selection: TranscriptSelectionState,
    /// Content rect of the topmost modal popup, recorded during render.
    pub(super) popup_selection_rect: Option<Rect>,
    pub(super) popup_copy_rows: Vec<Vec<String>>,
    /// Long-lived clipboard owner. On Linux the clipboard is served by this
    /// process, so a short-lived instance loses the contents when dropped.
    clipboard: Option<arboard::Clipboard>,
    pub(super) scrollbar_hovered: bool,
    pub(super) scrollbar_pressed: bool,
    pub(super) scrollbar_dragging: bool,
    pub(super) show_chat_list: bool,
    pub(super) ctrl_c_pending: bool,
    pub(super) last_current_chat_loading: bool,
    pub(super) awaiting_runtime_loading: bool,
    pub(super) last_runtime_status_refresh_at: Option<Instant>,
    pub(super) typewriter_state: TypewriterState,
    /// Pending chat-scoped tool permission requests from the current chat
    /// state; answers go back through the owning chat route.
    pub(super) current_tool_permission_requests: Vec<RuntimeHostInteractionToolPermissionRequest>,
    pub(super) language: TuiLanguage,
    pub(super) show_help: bool,
    pub(super) startup_install_prompt: Option<StartupInstallPrompt>,
    pub(super) startup_update_prompt: Option<StartupUpdatePrompt>,
    pub(super) startup_workspace_prompt: Option<StartupWorkspacePrompt>,
    pub(super) join_decision: Option<JoinDecisionModal>,
    pub(super) device_manager: Option<DeviceManagerModal>,
    pub(super) show_config_popup: bool,
    pub(super) config_ui: ConfigUi,
    pub(super) should_quit: bool,
}

struct TuiMessageContentStreamState {
    eventCount: u64,
    revisionTracker: TextStreamRevisionTracker,
    partStream: AssistantMarkupStreamState,
}

#[derive(Deserialize)]
struct PluginCommandInfoPayload {
    name: String,
    usage: String,
    description: String,
}

/// Loads enabled ToolPkg slash-command metadata from the Core command registry.
async fn load_plugin_command_specs(
    core: &mut TuiCore,
) -> Result<Vec<TuiPluginCommandSpec>, String> {
    let args = vec![
        "plugin".to_string(),
        "commands".to_string(),
        "--json".to_string(),
    ];
    let output = core
        .runCoreCommand(&args)
        .await
        .map_err(|error| error.to_string())?;
    if !output.stderr.trim().is_empty() {
        return Err(output.stderr.trim().to_string());
    }
    let commands = serde_json::from_str::<Vec<PluginCommandInfoPayload>>(&output.stdout)
        .map_err(|error| format!("plugin command metadata is invalid: {error}"))?;
    Ok(commands
        .into_iter()
        .flat_map(|command| expand_plugin_command(command.name, command.usage, command.description))
        .collect())
}

impl TuiMessageContentStreamState {
    /// Creates an empty semantic projection for one embedded AI content stream.
    fn new() -> Self {
        Self {
            eventCount: 0,
            revisionTracker: TextStreamRevisionTracker::new(""),
            partStream: AssistantMarkupStreamState::new(),
        }
    }

    /// Starts a self-contained stream snapshot.
    fn reset(&mut self) {
        self.revisionTracker.replace("");
        self.partStream = AssistantMarkupStreamState::new();
    }

    /// Appends one raw provider chunk and updates semantic message parts.
    fn pushChunk(&mut self, chunk: &str) -> Result<Vec<MessagePart>, String> {
        self.revisionTracker.append(chunk);
        self.partStream.push(chunk)?;
        Ok(self.partStream.parts().to_vec())
    }

    /// Records a named text revision point.
    fn savepoint(&mut self, id: &str) {
        self.revisionTracker.savepoint(id);
    }

    /// Restores a named revision point and rebuilds semantic message parts.
    fn rollback(&mut self, id: &str) -> Result<Vec<MessagePart>, String> {
        let content = self
            .revisionTracker
            .rollback(id)
            .ok_or_else(|| format!("TUI content stream rollback point is missing: {id}"))?
            .to_string();
        self.partStream.resetToSnapshot(&content)?;
        Ok(self.partStream.parts().to_vec())
    }

    /// Finalizes semantic message parts after the embedded stream completes.
    fn finish(&mut self) -> Result<Vec<MessagePart>, String> {
        self.partStream.finish()
    }

    /// Returns the current semantic message parts without mutating stream state.
    fn currentParts(&self) -> Vec<MessagePart> {
        self.partStream.parts().to_vec()
    }
}

#[derive(Clone, Debug)]
pub(super) struct ChatListItem {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) secondary: String,
    pub(super) updated_at: i64,
    pub(super) display_order: i64,
}

#[derive(Clone, Debug)]
pub(super) struct ModelChoiceItem {
    pub(super) provider_id: String,
    pub(super) model_id: String,
    pub(super) provider_name: String,
    pub(super) provider_type_id: String,
    pub(super) selected: bool,
}

#[derive(Clone, Debug)]
pub(super) struct ModelRef {
    pub(super) provider_id: String,
    pub(super) model_id: String,
}

#[derive(Clone, Debug)]
pub(super) struct StartupWorkspacePrompt {
    pub(super) path: String,
    pub(super) accept_selected: bool,
}

/// Interactive decision popup for incoming Space join requests. Requests are
/// decided in place with Y/N; Esc defers them and the command path stays
/// available as the fallback.
pub(super) struct JoinDecisionModal {
    pub(super) requests: Vec<SpaceJoinRequest>,
    pub(super) selected: usize,
}

/// One selectable row in the device management window: a pending join
/// request or a known device. Row identities stay stable across snapshot
/// merges so the selection never jumps.
pub(super) enum DeviceManagerRow {
    Request(SpaceJoinRequest),
    Device(RuntimeDeviceSpaceDevice),
}

impl DeviceManagerRow {
    pub(super) fn id(&self) -> &str {
        match self {
            Self::Request(request) => &request.requestId,
            Self::Device(device) => &device.deviceId,
        }
    }
}

/// Actions offered by the device management window. The connection axis is
/// a single slot derived from the restriction flag: admit and disconnect
/// are policy-level conjugates over `disconnectedNodeIds`, never two
/// parallel menu entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DeviceManagerAction {
    Admit,
    Disconnect,
    AssignIdentity,
    ClearIdentity,
    Remove,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DeviceManagerMode {
    Browsing,
    ActionMenu,
    ConfirmRemove,
    AssignIdentity,
}

/// Interactive device management window opened by `/network devices`.
/// Reachability (`online` in the topology) and policy restriction
/// (`blocked`) are independent flags: admit only lifts a restriction and
/// never dials a connection, so an offline member offers no connection
/// action at all.
pub(super) struct DeviceManagerModal {
    pub(super) topology: RuntimeDeviceSpaceTopology,
    pub(super) blocked: BTreeSet<String>,
    pub(super) requests: Vec<SpaceJoinRequest>,
    pub(super) roles: BTreeMap<String, NetworkControlRole>,
    pub(super) initialized: bool,
    pub(super) selected: usize,
    pub(super) mode: DeviceManagerMode,
    /// Device the action menu / confirm / identity picker is targeting.
    pub(super) menu_device_id: Option<String>,
    /// Cursor inside the current menu or identity picker.
    pub(super) menu_index: usize,
}

impl DeviceManagerModal {
    /// Pending join requests first, then devices; render and selection both
    /// derive from this ordering.
    pub(super) fn rows(&self) -> Vec<DeviceManagerRow> {
        self.requests
            .iter()
            .cloned()
            .map(DeviceManagerRow::Request)
            .chain(
                self.topology
                    .devices
                    .iter()
                    .cloned()
                    .map(DeviceManagerRow::Device),
            )
            .collect()
    }

    pub(super) fn selected_row(&self) -> Option<DeviceManagerRow> {
        self.rows().into_iter().nth(self.selected)
    }

    /// Identities ordered by display name so the picker is stable across
    /// BTreeMap reorderings.
    pub(super) fn sorted_roles(&self) -> Vec<&NetworkControlRole> {
        let mut roles = self.roles.values().collect::<Vec<_>>();
        roles.sort_by(|left, right| left.displayName.cmp(&right.displayName));
        roles
    }

    /// Derives the action menu for one device from its current policy
    /// state: a restricted device can only be restored, an unrestricted
    /// foreign device can be disconnected, identities appear only when
    /// they can apply, and the local device never offers actions at all -
    /// identity changes would drop the capabilities the local UI itself
    /// depends on, and leaving or demoting this device is managed from
    /// another administrator device.
    pub(super) fn menu_actions(&self, device_id: &str) -> Vec<DeviceManagerAction> {
        let is_self = device_id == self.topology.currentDeviceId;
        if is_self {
            return Vec::new();
        }
        let mut actions = Vec::new();
        if self.blocked.contains(device_id) {
            actions.push(DeviceManagerAction::Admit);
        } else {
            actions.push(DeviceManagerAction::Disconnect);
        }
        if !self.roles.is_empty() {
            actions.push(DeviceManagerAction::AssignIdentity);
        }
        if self
            .topology
            .devices
            .iter()
            .find(|device| device.deviceId == device_id)
            .is_some_and(|device| device.currentIdentity.is_some())
        {
            actions.push(DeviceManagerAction::ClearIdentity);
        }
        actions.push(DeviceManagerAction::Remove);
        actions
    }
}

#[derive(Debug)]
pub(super) struct StartupInstallPrompt {
    pub(super) install_selected: bool,
    pub(super) state: StartupInstallState,
    pub(super) progress_rx: Option<mpsc::Receiver<StartupInstallMessage>>,
}

#[derive(Debug, Clone)]
pub(super) enum StartupInstallState {
    Ready,
    Installing { message: String },
    Complete,
    Error { message: String },
}

#[derive(Debug, Clone)]
pub(super) enum StartupInstallMessage {
    Progress(CliInstallProgress),
    Complete(Result<(), String>),
}

#[derive(Debug)]
pub(super) struct StartupUpdatePrompt {
    pub(super) release_info: Option<ReleaseInfo>,
    pub(super) download_selected: bool,
    pub(super) download_state: FullUpdateDownloadState,
    pub(super) progress_rx: Option<mpsc::Receiver<FullUpdateDownloadMessage>>,
}

#[derive(Debug, Clone)]
pub(super) enum FullUpdateDownloadState {
    Ready,
    Downloading {
        stage: FullUpdateStage,
        message: String,
        read_bytes: u64,
        total_bytes: u64,
        speed_bytes_per_sec: u64,
    },
    Complete {
        package_path: PathBuf,
        install_status: Option<crate::cli::DownloadedUpdateInstallStatus>,
    },
    Error {
        message: String,
    },
    CheckError {
        message: String,
    },
}

#[derive(Debug, Clone)]
pub(super) enum FullUpdateDownloadMessage {
    Progress(FullUpdateProgressEvent),
    Complete(Result<(PathBuf, Option<crate::cli::DownloadedUpdateInstallStatus>), String>),
}

#[derive(Clone, Debug)]
pub(super) enum QueuedAttachmentTokenKind {
    Path { path: String },
    Inline { file_path: String },
}

#[derive(Clone, Debug)]
pub(super) struct QueuedAttachmentToken {
    pub(super) token: String,
    pub(super) kind: QueuedAttachmentTokenKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FocusArea {
    Chats,
    ModelChooser,
    Queue,
    Input,
}

impl OperitTui {
    pub(super) async fn new(
        mut core: TuiCore,
        networkControl: RuntimeRemoteLinkService,
        initial_shell_args: ShellArgs,
        initial_chat_id: String,
        language: TuiLanguage,
        startup_install_prompt: Option<StartupInstallPrompt>,
        startup_update_prompt: Option<StartupUpdatePrompt>,
        startup_workspace_prompt_path: Option<String>,
        toast_receiver: mpsc::Receiver<String>,
        network_event_receiver: mpsc::Receiver<NetworkUiEvent>,
    ) -> Result<Self, String> {
        let chat_histories = core
            .chat_runtime_holder_main()
            .chatHistoriesFlowSnapshot()
            .await
            .map_err(|error| error.to_string())?;
        let chats = chat_histories_to_list(chat_histories);
        let selected_chat_index = chats
            .iter()
            .position(|item| item.id == initial_chat_id)
            .unwrap_or(0);
        let status_message = language.text().initial_status().to_string();
        let current_chat_id_cache = core
            .chat_runtime_holder_main()
            .currentChatIdFlowSnapshot()
            .await
            .map_err(|error| error.to_string())?;
        let active_chat_id = current_chat_id_cache
            .clone()
            .ok_or_else(|| "no active chat in tui".to_string())?;
        let mut route_permission_error = None;
        let current_messages_cache = match core
            .chat_runtime_holder_main()
            .chatMessagesFlowSnapshot(active_chat_id.clone())
            .await
        {
            Ok(messages) => messages,
            Err(error) if error.isRoutePermissionDenied() => {
                route_permission_error = Some(error.to_string());
                Vec::new()
            }
            Err(error) => return Err(error.to_string()),
        };
        let current_chat_state_cache = match core
            .chat_runtime_holder_main()
            .chatStateFlowSnapshot(active_chat_id.clone())
            .await
        {
            Ok(state) => state,
            Err(error) if error.isRoutePermissionDenied() => {
                route_permission_error = Some(error.to_string());
                ChatState {
                    currentChatId: active_chat_id.clone(),
                    currentChatTitle: String::new(),
                    currentCharacterCardName: None,
                    currentCharacterCardAvatarUri: None,
                    currentWorkspacePath: None,
                    isLoading: false,
                    inputProcessingState: InputProcessingState::Error {
                        message: route_permission_error
                            .clone()
                            .expect("route permission error must be recorded"),
                    },
                    hasOlderDisplayHistory: false,
                    hasNewerDisplayHistory: false,
                    isLoadingDisplayWindow: false,
                    pendingQueueMessages: Vec::new(),
                    isPendingQueueExpanded: false,
                    toolPermissionRequests: Vec::new(),
                }
            }
            Err(error) => return Err(error.to_string()),
        };
        let current_chat_is_loading_cache = current_chat_state_cache.isLoading;
        let current_chat_input_processing_state_cache =
            current_chat_state_cache.inputProcessingState.clone();
        let current_tool_permission_requests = current_chat_state_cache.toolPermissionRequests;
        let current_window_size_cache = core
            .chat_runtime_holder_main()
            .currentWindowSizeFlowSnapshot()
            .await
            .map_err(|error| error.to_string())?;
        if let Err(error) = core.watchMainChatGeneratedStateFlows().await {
            if error.isRoutePermissionDenied() {
                route_permission_error = Some(error.to_string());
            } else {
                return Err(error.to_string());
            }
        }
        if let Err(error) = core.watchMainChatStateFlow(active_chat_id.clone()).await {
            if error.isRoutePermissionDenied() {
                route_permission_error = Some(error.to_string());
            } else {
                return Err(error.to_string());
            }
        }
        if let Err(error) = core.watchMainChatMessagesFlow(active_chat_id).await {
            if error.isRoutePermissionDenied() {
                route_permission_error = Some(error.to_string());
            } else {
                return Err(error.to_string());
            }
        }
        core.syncMainChatContentStreams(&current_messages_cache)
            .await
            .map_err(|error| error.to_string())?;
        let plugin_commands = load_plugin_command_specs(&mut core).await?;
        Ok(Self {
            compose: super::compose::ComposeHost::default(),
            core,
            networkControl,
            initial_shell_args,
            current_chat_id_cache: current_chat_id_cache.clone(),
            current_messages_cache,
            content_stream_states: BTreeMap::new(),
            current_chat_is_loading_cache,
            current_chat_input_processing_state_cache,
            current_window_size_cache,
            chats,
            selected_chat_index,
            model_choices: Vec::new(),
            selected_model_choice_index: 0,
            show_model_chooser: false,
            model_chooser_search: String::new(),
            model_chooser_filtered_indices: Vec::new(),
            model_list_mode: false,
            show_list_popup: false,
            list_popup_title: String::new(),
            list_popup_items: Vec::new(),
            list_popup_search: String::new(),
            list_popup_filtered_indices: Vec::new(),
            list_popup_selected_index: 0,
            focus: FocusArea::Input,
            input: String::new(),
            input_cursor: 0,
            autocomplete_index: 0,
            plugin_commands,
            queued_attachment_paths: Vec::new(),
            queued_inline_attachments: Vec::new(),
            queued_attachment_tokens: Vec::new(),
            pending_queue_chat_id: current_chat_id_cache.clone(),
            pending_queue_messages: VecDeque::new(),
            selected_pending_queue_index: 0,
            next_pending_queue_id: 1,
            was_pending_queue_blocked: false,
            suppress_next_pending_queue_auto_send: false,
            pending_queue_auto_send_at: None,
            pending_queue_manual_send: None,
            paste_attachment_counter: 0,
            status_message: route_permission_error.clone().unwrap_or(status_message),
            route_permission_error,
            status_message_expires_at: None,
            transient_status_message: None,
            toast_receiver,
            network_event_receiver,
            seen_pairing_prompt_ids: BTreeSet::new(),
            seen_join_request_ids: BTreeSet::new(),
            pending_pairings: Vec::new(),
            last_outgoing_join_refresh_at: None,
            network_snapshots_seeded: false,
            context_usage_label: String::new(),
            transcript_scroll: 0,
            transcript_viewport_height: 1,
            transcript_max_scroll: 0,
            follow_transcript: true,
            transcript_render_cache: TranscriptRenderCache::default(),
            transcript_area: Rect::default(),
            transcript_copy_lines: Vec::new(),
            transcript_selection: TranscriptSelectionState::default(),
            popup_selection: TranscriptSelectionState::default(),
            popup_selection_rect: None,
            popup_copy_rows: Vec::new(),
            clipboard: None,
            scrollbar_hovered: false,
            scrollbar_pressed: false,
            scrollbar_dragging: false,
            show_chat_list: false,
            ctrl_c_pending: false,
            last_current_chat_loading: false,
            awaiting_runtime_loading: false,
            last_runtime_status_refresh_at: None,
            typewriter_state: TypewriterState::default(),
            current_tool_permission_requests,
            language,
            show_help: false,
            startup_install_prompt,
            startup_update_prompt,
            startup_workspace_prompt: startup_workspace_prompt_path.map(|path| {
                StartupWorkspacePrompt {
                    path,
                    accept_selected: true,
                }
            }),
            show_config_popup: false,
            config_ui: ConfigUi::new(),
            join_decision: None,
            device_manager: None,
            should_quit: false,
        })
    }

    pub(super) fn text(&self) -> TuiText {
        self.language.text()
    }

    pub(super) async fn run(&mut self) -> Result<(), String> {
        let previous_console_logging = AppLogger::enable_console_logging();
        AppLogger::set_enable_console_logging(false);
        if let Err(error) = enable_raw_mode().map_err(|error| error.to_string()) {
            AppLogger::set_enable_console_logging(previous_console_logging);
            return Err(error);
        }
        let mut stdout = io::stdout();
        if let Err(error) = execute!(
            stdout,
            EnterAlternateScreen,
            EnableBracketedPaste,
            EnableMouseCapture
        )
        .map_err(|error| error.to_string())
        {
            let _ = execute!(
                io::stdout(),
                DisableMouseCapture,
                DisableBracketedPaste,
                LeaveAlternateScreen
            );
            let _ = disable_raw_mode();
            AppLogger::set_enable_console_logging(previous_console_logging);
            return Err(error);
        }
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = match Terminal::new(backend).map_err(|error| error.to_string()) {
            Ok(terminal) => terminal,
            Err(error) => {
                let _ = execute!(
                    io::stdout(),
                    DisableMouseCapture,
                    DisableBracketedPaste,
                    LeaveAlternateScreen
                );
                let _ = disable_raw_mode();
                AppLogger::set_enable_console_logging(previous_console_logging);
                return Err(error);
            }
        };
        let result = self.run_loop(&mut terminal).await;
        let compose_cleanup = self.compose.clear(&mut self.core).await;
        let screen_result = execute!(
            terminal.backend_mut(),
            DisableMouseCapture,
            DisableBracketedPaste,
            LeaveAlternateScreen
        )
        .map_err(|error| error.to_string());
        let raw_mode_result = disable_raw_mode().map_err(|error| error.to_string());
        let cursor_result = terminal.show_cursor().map_err(|error| error.to_string());
        let cleanup_result = screen_result.and(raw_mode_result).and(cursor_result);
        AppLogger::set_enable_console_logging(previous_console_logging);
        result.and(cleanup_result).and(compose_cleanup)
    }

    /// Applies a route permission failure to the visible TUI state.
    fn apply_route_permission_error(&mut self, error: String) {
        self.current_chat_is_loading_cache = false;
        self.last_current_chat_loading = false;
        self.awaiting_runtime_loading = false;
        self.current_chat_input_processing_state_cache = InputProcessingState::Error {
            message: error.clone(),
        };
        self.route_permission_error = Some(error.clone());
        self.set_status_message(error);
    }

    /// Returns whether a rendered error is a route permission failure.
    fn is_route_permission_error_message(error: &str) -> bool {
        error.strip_prefix("ROUTE_PERMISSION_DENIED:").is_some()
    }

    async fn run_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), String> {
        while !self.should_quit {
            self.ensure_pending_queue_chat_id();
            if let Err(error) = self.apply_pushed_events().await {
                if Self::is_route_permission_error_message(&error) {
                    self.apply_route_permission_error(error);
                } else {
                    return Err(error);
                }
            }
            self.apply_toast_messages();
            self.apply_network_events().await;
            self.refresh_outgoing_joins_if_due().await;
            if let Err(error) = self.sync_compose_surfaces().await {
                if Self::is_route_permission_error_message(&error) {
                    self.apply_route_permission_error(error);
                } else {
                    self.status_message = error;
                }
            }
            self.ensure_pending_queue_chat_id();
            self.refresh_runtime_status_if_due().await;
            if let Err(error) = self.advance_pending_message_queue().await {
                if Self::is_route_permission_error_message(&error) {
                    self.apply_route_permission_error(error);
                } else {
                    return Err(error);
                }
            }
            self.clear_expired_status_message();
            terminal
                .draw(|frame| self.render(frame))
                .map_err(|error| error.to_string())?;

            if let Err(error) = self.handle_terminal_events(EVENT_POLL_INTERVAL).await {
                if Self::is_route_permission_error_message(&error) {
                    self.apply_route_permission_error(error);
                } else {
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    fn set_transient_status_message(&mut self, message: String) {
        self.status_message = message.clone();
        self.transient_status_message = Some(message);
        self.status_message_expires_at = Some(Instant::now() + TRANSIENT_STATUS_DURATION);
    }

    /// Applies queued host toasts to the TUI transient status line.
    fn apply_toast_messages(&mut self) {
        while let Ok(message) = self.toast_receiver.try_recv() {
            self.set_transient_status_message(message);
        }
    }

    /// Applies peer-change watcher signals: fetches fresh pairing prompt and
    /// join request snapshots and opens the popups when ids beyond the
    /// previous snapshot appear. The first fetch only seeds the snapshots so
    /// a TUI start does not replay requests that predate the session.
    async fn apply_network_events(&mut self) {
        while self
            .network_event_receiver
            .try_recv()
            .map(|event| matches!(event, NetworkUiEvent::PeerChanges))
            .unwrap_or(false)
        {
            self.refresh_network_snapshots().await;
        }
    }

    /// Pulls the reviewer decision for every active outgoing join request,
    /// mirroring the Flutter peer-dialog monitor: the target never pushes the
    /// approval, so the applicant side has to poll. Failures (offline targets)
    /// leave the request pending for the next tick; only a terminal status is
    /// reported and it refreshes the network snapshots once.
    async fn refresh_outgoing_joins_if_due(&mut self) {
        let now = Instant::now();
        let due = self
            .last_outgoing_join_refresh_at
            .map(|last| now.saturating_duration_since(last) >= OUTGOING_JOIN_REFRESH_INTERVAL)
            .unwrap_or(true);
        if !due {
            return;
        }
        self.last_outgoing_join_refresh_at = Some(now);
        let Ok(requests) = self.networkControl.outgoingDeviceSpaceJoins() else {
            return;
        };
        for request in requests
            .iter()
            .filter(|request| space_join_is_active(&request.status))
        {
            let Ok(updated) = self
                .networkControl
                .refreshDeviceSpaceJoin(request.requestId.clone())
                .await
            else {
                continue;
            };
            if !space_join_is_active(&updated.status) {
                let status = updated.status;
                let message = match status {
                    SpaceJoinStatus::Joined => format!(
                        "network join approved: joined {} with {}",
                        updated.spaceName, updated.targetDeviceId
                    ),
                    SpaceJoinStatus::Rejected => {
                        format!("network join rejected by {}", updated.targetDeviceId)
                    }
                    _ => format!(
                        "network join {}: {} ({})",
                        join_status_label(&status),
                        updated.targetDeviceId,
                        updated.spaceName
                    ),
                };
                self.set_transient_status_message(message);
                self.refresh_network_snapshots().await;
            }
        }
    }

    async fn refresh_network_snapshots(&mut self) {
        let prompts = self.networkControl.pairingPrompts().unwrap_or_default();
        let has_new_prompts = prompts
            .iter()
            .any(|prompt| !self.seen_pairing_prompt_ids.contains(&prompt.pairingId));
        if self.network_snapshots_seeded && has_new_prompts {
            self.open_pairing_prompts_popup(&prompts);
        }
        self.seen_pairing_prompt_ids = prompts
            .iter()
            .map(|prompt| prompt.pairingId.clone())
            .collect();

        let Ok(requests) = self.networkControl.incomingDeviceSpaceJoins().await else {
            self.network_snapshots_seeded = true;
            return;
        };
        let has_new_requests = requests
            .iter()
            .any(|request| !self.seen_join_request_ids.contains(&request.requestId));
        if self.network_snapshots_seeded && has_new_requests {
            self.open_join_decision_modal(requests.clone());
        }
        self.seen_join_request_ids = requests
            .iter()
            .map(|request| request.requestId.clone())
            .collect();
        self.network_snapshots_seeded = true;
        if self.device_manager.is_some() {
            self.refresh_device_manager_snapshot().await;
        }
    }

    /// Opens the pairing popup listing every pending prompt, one block per
    /// prompt with the confirmation code on its own line, plus a trailing
    /// hint line. Esc or Enter closes it.
    fn open_pairing_prompts_popup(&mut self, prompts: &[PairingPrompt]) {
        let text = self.text();
        let mut items = Vec::new();
        for prompt in prompts {
            items.push(prompt.displayName.clone());
            items.push(format!(
                "{}: {}",
                text.network_pairing_popup_code(),
                prompt.confirmationCode
            ));
        }
        items.push(text.network_pairing_popup_hint().to_string());
        self.open_list_popup(text.network_pairing_popup_title().to_string(), items);
    }

    /// Opens the Space join decision popup for the given pending requests.
    /// Requests this node cannot approve are filtered out. When the popup is
    /// already open, fresh requests merge into the queue and the current
    /// selection is preserved.
    fn open_join_decision_modal(&mut self, requests: Vec<SpaceJoinRequest>) {
        let decidable: Vec<SpaceJoinRequest> = requests
            .into_iter()
            .filter(|request| request.canApprove)
            .collect();
        if decidable.is_empty() {
            return;
        }
        match self.join_decision.as_mut() {
            Some(modal) => {
                for request in decidable {
                    if !modal
                        .requests
                        .iter()
                        .any(|existing| existing.requestId == request.requestId)
                    {
                        modal.requests.push(request);
                    }
                }
            }
            None => {
                self.join_decision = Some(JoinDecisionModal {
                    requests: decidable,
                    selected: 0,
                });
            }
        }
    }

    fn clear_expired_status_message(&mut self) {
        if self
            .status_message_expires_at
            .is_some_and(|expires_at| Instant::now() >= expires_at)
        {
            if self
                .transient_status_message
                .as_ref()
                .is_some_and(|message| message == &self.status_message)
            {
                self.status_message.clear();
            }
            self.status_message_expires_at = None;
            self.transient_status_message = None;
        }
    }

    async fn handle_terminal_events(&mut self, initial_poll: Duration) -> Result<(), String> {
        if !event::poll(initial_poll).map_err(|error| error.to_string())? {
            return Ok(());
        }
        for _ in 0..MAX_PENDING_TERMINAL_EVENTS_PER_FRAME {
            let terminal_event = event::read().map_err(|error| error.to_string())?;
            self.handle_terminal_event(terminal_event).await?;
            if self.should_quit
                || !event::poll(Duration::from_millis(0)).map_err(|error| error.to_string())?
            {
                break;
            }
        }
        Ok(())
    }

    async fn handle_terminal_event(&mut self, terminal_event: Event) -> Result<(), String> {
        match terminal_event {
            Event::Key(key) => self.handle_key_event(key).await,
            Event::Mouse(mouse) => self.handle_mouse_event(mouse).await,
            Event::Paste(text) => {
                if self.show_config_popup {
                    self.config_ui.handle_paste(&text);
                    return Ok(());
                }
                if let Some(editor) = &mut self.compose.editor {
                    editor.value.push_str(&text);
                    return Ok(());
                }
                if self.show_list_popup {
                    self.list_popup_search.push_str(&text);
                    self.update_list_popup_filter();
                    return Ok(());
                }
                if self.show_model_chooser {
                    self.status_message.clear();
                    self.status_message_expires_at = None;
                    self.transient_status_message = None;
                    self.model_chooser_search.push_str(&text);
                    self.update_model_chooser_filter();
                    return Ok(());
                }
                self.handle_paste(text).await
            }
            _ => Ok(()),
        }
    }

    /// Routes plugin control clicks before transcript selection and scrolling.
    async fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<(), String> {
        let overlay = self.show_help
            || self.show_config_popup
            || self.show_list_popup
            || self.show_model_chooser
            || self.startup_install_prompt.is_some()
            || self.startup_update_prompt.is_some()
            || self.startup_workspace_prompt.is_some()
            || !self.current_tool_permission_requests.is_empty()
            || self.join_decision.is_some()
            || self.device_manager.is_some();
        if self.compose.editor.is_some() {
            return Ok(());
        }
        if let Some(area) = self.popup_selection_rect {
            let inside = mouse.column >= area.x
                && mouse.column < area.x.saturating_add(area.width)
                && mouse.row >= area.y
                && mouse.row < area.y.saturating_add(area.height);
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) if inside => {
                    self.scrollbar_pressed = false;
                    self.scrollbar_dragging = false;
                    self.transcript_selection.clear();
                    if let Some(position) =
                        mouse_popup_position(mouse.column, mouse.row, area, &self.popup_copy_rows)
                    {
                        self.popup_selection.begin(position);
                    }
                    return Ok(());
                }
                MouseEventKind::Drag(MouseButton::Left) if self.popup_selection.is_dragging() => {
                    if let Some(position) = mouse_popup_drag_position(
                        mouse.column,
                        mouse.row,
                        area,
                        &self.popup_copy_rows,
                    ) {
                        self.popup_selection.drag_to(position);
                    }
                    return Ok(());
                }
                MouseEventKind::Up(MouseButton::Left) if self.popup_selection.is_dragging() => {
                    match mouse_popup_drag_position(
                        mouse.column,
                        mouse.row,
                        area,
                        &self.popup_copy_rows,
                    ) {
                        Some(position) => self.popup_selection.end(position),
                        None => self.popup_selection.clear(),
                    }
                    return Ok(());
                }
                MouseEventKind::Down(MouseButton::Right) if inside => {
                    self.popup_selection.clear();
                    return Ok(());
                }
                // Presses outside the popup discard its selection and keep the
                // scrollbar/transcript behavior below.
                MouseEventKind::Down(MouseButton::Left) => {
                    self.popup_selection.clear();
                }
                _ => {}
            }
        }
        if !overlay {
            let area = super::scrollbar::split_transcript_inner(self.transcript_area).content;
            let inside = mouse.column >= area.x
                && mouse.column < area.x + area.width
                && mouse.row >= area.y
                && mouse.row < area.y + area.height;
            let row = if inside {
                usize::from(mouse.row - area.y + self.transcript_scroll)
            } else {
                usize::MAX
            };
            let col = if inside {
                usize::from(mouse.column - area.x)
            } else {
                usize::MAX
            };
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) if self.compose.press(row, col) => {
                    self.transcript_selection.clear();
                    return Ok(());
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    match self.compose.release_click(&mut self.core, row, col).await {
                        Ok(true) => return Ok(()),
                        Ok(false) => {}
                        Err(error) => {
                            self.status_message = error;
                            return Ok(());
                        }
                    }
                }
                _ => {}
            }
        }
        match mouse.kind {
            MouseEventKind::ScrollUp => self.scroll_transcript_up(self.terminal_wheel_step()),
            MouseEventKind::ScrollDown => self.scroll_transcript_down(self.terminal_wheel_step()),
            MouseEventKind::Down(MouseButton::Left) => {
                if self.handle_scrollbar_press(mouse) {
                    return Ok(());
                }
                if let Some(position) = mouse_transcript_position(
                    mouse,
                    self.transcript_area,
                    self.transcript_scroll,
                    &self.transcript_copy_lines,
                ) {
                    self.transcript_selection.begin(position);
                } else {
                    self.transcript_selection.clear();
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.scrollbar_dragging {
                    self.drag_scrollbar_to(mouse.row);
                    return Ok(());
                }
                if self.scrollbar_pressed {
                    return Ok(());
                }
                if let Some(position) = mouse_drag_transcript_position(
                    mouse,
                    self.transcript_area,
                    self.transcript_scroll,
                    &self.transcript_copy_lines,
                ) {
                    self.transcript_selection.drag_to(position);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if self.scrollbar_pressed {
                    self.scrollbar_pressed = false;
                    self.scrollbar_dragging = false;
                    return Ok(());
                }
                if let Some(position) = mouse_drag_transcript_position(
                    mouse,
                    self.transcript_area,
                    self.transcript_scroll,
                    &self.transcript_copy_lines,
                ) {
                    if self.transcript_selection.is_click()
                        && self
                            .transcript_render_cache
                            .toggle_fold_at_line(position.line)
                    {
                        self.transcript_selection.clear();
                        return Ok(());
                    }
                    self.transcript_selection.end(position);
                }
            }
            MouseEventKind::Moved => self.update_scrollbar_hover(mouse.column, mouse.row),
            MouseEventKind::Down(MouseButton::Right) => {
                self.scrollbar_pressed = false;
                self.scrollbar_dragging = false;
                self.transcript_selection.clear();
            }
            _ => {}
        }
        Ok(())
    }

    /// Starts scrollbar interaction when the pointer is on the visible bar.
    fn handle_scrollbar_press(&mut self, mouse: MouseEvent) -> bool {
        if self.transcript_max_scroll == 0
            || !pointer_hits_scrollbar(mouse.column, mouse.row, self.transcript_area)
        {
            self.scrollbar_pressed = false;
            self.scrollbar_dragging = false;
            self.scrollbar_hovered = false;
            return false;
        }
        self.transcript_selection.clear();
        self.scrollbar_hovered = true;
        self.scrollbar_pressed = true;
        match scrollbar_hit_part(mouse.row, self.transcript_area) {
            ScrollbarHit::Begin => self.scroll_transcript_up(self.transcript_page_step()),
            ScrollbarHit::End => self.scroll_transcript_down(self.transcript_page_step()),
            ScrollbarHit::Track => {
                self.scrollbar_dragging = true;
                self.drag_scrollbar_to(mouse.row);
            }
        }
        true
    }

    /// Moves transcript scroll to the track position under `row`.
    fn drag_scrollbar_to(&mut self, row: u16) {
        let position =
            scroll_position_for_pointer(row, self.transcript_area, self.transcript_max_scroll);
        self.transcript_scroll = position;
        self.follow_transcript = position >= self.transcript_max_scroll;
        self.scrollbar_hovered = true;
    }

    /// Tracks whether the pointer is resting on the visible scrollbar.
    fn update_scrollbar_hover(&mut self, column: u16, row: u16) {
        self.scrollbar_hovered = self.transcript_max_scroll > 0
            && pointer_hits_scrollbar(column, row, self.transcript_area);
    }

    fn copy_transcript_selection(&mut self) -> bool {
        let Some(text) = self
            .transcript_selection
            .selected_text(&self.transcript_copy_lines)
        else {
            return false;
        };
        if text.is_empty() {
            return false;
        }
        self.copy_text_to_clipboard(text)
    }

    /// Copies the current modal popup selection, if any.
    fn copy_popup_selection(&mut self) -> bool {
        let Some(text) = popup_selected_text(&self.popup_copy_rows, &self.popup_selection) else {
            return false;
        };
        if text.is_empty() {
            return false;
        }
        self.copy_text_to_clipboard(text)
    }

    fn copy_text_to_clipboard(&mut self, text: String) -> bool {
        if self.clipboard.is_none() {
            match arboard::Clipboard::new() {
                Ok(clipboard) => self.clipboard = Some(clipboard),
                Err(error) => {
                    self.status_message = self.text().copy_failed(&error.to_string());
                    return true;
                }
            }
        }
        let Some(clipboard) = self.clipboard.as_mut() else {
            return false;
        };
        match clipboard.set_text(text) {
            Ok(()) => {
                self.status_message = self.text().selection_copied().to_string();
                true
            }
            Err(error) => {
                self.status_message = self.text().copy_failed(&error.to_string());
                true
            }
        }
    }

    /// Snapshots the topmost popup's rendered cells and paints the live selection
    /// highlight; called once per frame after all overlays have rendered.
    pub(super) fn finish_popup_selection(&mut self, buffer: &mut Buffer) {
        let Some(area) = self
            .popup_selection_rect
            .filter(|area| area.width > 0 && area.height > 0)
        else {
            self.popup_selection_rect = None;
            if !self.popup_copy_rows.is_empty() {
                self.popup_copy_rows.clear();
            }
            self.popup_selection.clear();
            return;
        };
        let rows = popup_rows_from_buffer(buffer, area);
        // Content changed under the selection (refilter, list move, …) — drop it.
        if rows != self.popup_copy_rows {
            self.popup_selection.clear();
        }
        self.popup_copy_rows = rows;
        apply_popup_selection_highlight(buffer, area, &self.popup_selection);
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<(), String> {
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return Ok(());
        }

        // Popup selection copies win over popup key routing.
        if matches!(key.code, KeyCode::Char('c'))
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && self.copy_popup_selection()
        {
            return Ok(());
        }

        // Config popup has its own key handling — intercept early
        if self.show_config_popup {
            self.handle_config_key(key).await?;
            return Ok(());
        }

        if matches!(key.code, KeyCode::Char('c'))
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && self.copy_transcript_selection()
        {
            return Ok(());
        }

        if matches!(key.code, KeyCode::Char('c')) && key.modifiers == KeyModifiers::CONTROL {
            if self.ctrl_c_pending {
                self.should_quit = true;
            } else {
                self.ctrl_c_pending = true;
                self.status_message = self.text().ctrl_c_again_to_quit().to_string();
            }
            return Ok(());
        }

        self.ctrl_c_pending = false;

        if self.compose.editor.is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.compose.editor = None;
                }
                KeyCode::Enter => {
                    if let Err(error) = self.compose.commit_editor(&mut self.core).await {
                        self.status_message = error;
                    }
                }
                KeyCode::Backspace => {
                    self.compose.editor.as_mut().unwrap().value.pop();
                }
                KeyCode::Char(ch)
                    if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
                {
                    self.compose.editor.as_mut().unwrap().value.push(ch);
                }
                _ => {}
            }
            return Ok(());
        }

        if self.startup_install_prompt.is_some() {
            self.handle_startup_install_prompt_key(key).await?;
            return Ok(());
        }

        if self.startup_update_prompt.is_some() {
            self.handle_startup_update_prompt_key(key).await?;
            return Ok(());
        }

        if self.startup_workspace_prompt.is_some() {
            self.handle_startup_workspace_prompt_key(key).await?;
            return Ok(());
        }

        if !self.current_tool_permission_requests.is_empty() {
            self.handle_approval_key(key).await;
            return Ok(());
        }

        if self.join_decision.is_some() {
            self.handle_join_decision_key(key).await?;
            return Ok(());
        }

        if self.device_manager.is_some() {
            self.handle_device_manager_key(key).await?;
            return Ok(());
        }

        if self.show_list_popup {
            return self.handle_list_popup_key(key);
        }

        if self.show_help {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::F(1) => {
                    self.show_help = false;
                }
                _ => {}
            }
            return Ok(());
        }

        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return Ok(());
            }
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.create_new_chat(self.initial_shell_args.clone())
                    .await?;
                return Ok(());
            }
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                self.refresh_chats().await;
                self.status_message = self.text().chat_list_refreshed().to_string();
                return Ok(());
            }
            (KeyCode::F(3), _) => {
                self.toggle_chat_list().await;
                return Ok(());
            }
            (KeyCode::PageUp, _) => {
                self.scroll_transcript_page_up();
                return Ok(());
            }
            (KeyCode::PageDown, _) => {
                self.scroll_transcript_page_down();
                return Ok(());
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.scroll_transcript_half_page_up();
                return Ok(());
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.scroll_transcript_half_page_down();
                return Ok(());
            }
            (KeyCode::Up, KeyModifiers::NONE) if self.should_arrow_scroll_transcript() => {
                self.scroll_transcript_up(self.terminal_wheel_step());
                return Ok(());
            }
            (KeyCode::Down, KeyModifiers::NONE) if self.should_arrow_scroll_transcript() => {
                self.scroll_transcript_down(self.terminal_wheel_step());
                return Ok(());
            }
            (KeyCode::Home, KeyModifiers::CONTROL) => {
                self.scroll_transcript_to_top();
                return Ok(());
            }
            (KeyCode::End, KeyModifiers::CONTROL) => {
                self.scroll_transcript_to_bottom();
                return Ok(());
            }
            (KeyCode::Esc, _) => {
                if self.current_chat_is_loading() {
                    self.cancel_current_request().await?;
                    return Ok(());
                }
                if self.show_model_chooser {
                    if !self.model_chooser_search.is_empty() {
                        self.model_chooser_search.clear();
                        self.update_model_chooser_filter();
                    } else {
                        self.close_model_chooser();
                    }
                    return Ok(());
                }
                if self.show_chat_list && self.focus == FocusArea::Chats {
                    self.show_chat_list = false;
                    self.focus = FocusArea::Input;
                    self.status_message = self.text().chat_list_hidden().to_string();
                    return Ok(());
                }
                self.status_message.clear();
                self.focus = FocusArea::Input;
                return Ok(());
            }
            (KeyCode::Char('?'), _) | (KeyCode::F(1), _) => {
                self.show_help = true;
                return Ok(());
            }
            (KeyCode::Tab, _)
                if self.focus == FocusArea::Input && !self.command_suggestions().is_empty() => {}
            (KeyCode::Tab, _) => {
                self.focus_next_area();
                return Ok(());
            }
            _ => {}
        }

        match self.focus {
            FocusArea::Chats => self.handle_chat_list_key(key).await,
            FocusArea::ModelChooser => self.handle_model_chooser_key(key).await,
            FocusArea::Queue => self.handle_pending_queue_key(key).await,
            FocusArea::Input => self.handle_input_key(key).await,
        }
    }

    fn scroll_transcript_page_up(&mut self) {
        self.scroll_transcript_up(self.transcript_page_step());
    }

    fn scroll_transcript_page_down(&mut self) {
        self.scroll_transcript_down(self.transcript_page_step());
    }

    fn scroll_transcript_half_page_up(&mut self) {
        self.scroll_transcript_up(self.transcript_half_page_step());
    }

    fn scroll_transcript_half_page_down(&mut self) {
        self.scroll_transcript_down(self.transcript_half_page_step());
    }

    fn scroll_transcript_to_top(&mut self) {
        self.follow_transcript = false;
        self.transcript_scroll = 0;
    }

    fn scroll_transcript_to_bottom(&mut self) {
        self.follow_transcript = true;
        self.transcript_scroll = self.transcript_max_scroll;
    }

    fn scroll_transcript_up(&mut self, amount: u16) {
        self.follow_transcript = false;
        self.transcript_scroll = self.transcript_scroll.saturating_sub(amount);
    }

    fn scroll_transcript_down(&mut self, amount: u16) {
        let next_scroll = self
            .transcript_scroll
            .saturating_add(amount)
            .min(self.transcript_max_scroll);
        self.transcript_scroll = next_scroll;
        self.follow_transcript = next_scroll >= self.transcript_max_scroll;
    }

    fn transcript_page_step(&self) -> u16 {
        self.transcript_viewport_height.max(1)
    }

    fn transcript_half_page_step(&self) -> u16 {
        (self.transcript_viewport_height / 2).max(1)
    }

    fn terminal_wheel_step(&self) -> u16 {
        (self.transcript_viewport_height / 6).max(3)
    }

    fn should_arrow_scroll_transcript(&self) -> bool {
        self.focus == FocusArea::Input && self.command_suggestions().is_empty()
    }

    async fn handle_chat_list_key(&mut self, key: KeyEvent) -> Result<(), String> {
        match key.code {
            KeyCode::Up => {
                if self.selected_chat_index > 0 {
                    self.selected_chat_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_chat_index + 1 < self.chats.len() {
                    self.selected_chat_index += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(item) = self.chats.get(self.selected_chat_index) {
                    self.switch_to_chat(item.id.clone()).await?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_model_chooser_key(&mut self, key: KeyEvent) -> Result<(), String> {
        match key.code {
            KeyCode::Up => {
                self.status_message.clear();
                self.status_message_expires_at = None;
                self.transient_status_message = None;
                if self.selected_model_choice_index > 0 {
                    self.selected_model_choice_index -= 1;
                }
            }
            KeyCode::Down => {
                self.status_message.clear();
                self.status_message_expires_at = None;
                self.transient_status_message = None;
                if self.selected_model_choice_index + 1 < self.model_chooser_filtered_indices.len()
                {
                    self.selected_model_choice_index += 1;
                }
            }
            KeyCode::Enter => {
                if self.model_list_mode {
                    self.close_model_chooser();
                } else {
                    self.apply_selected_model_choice().await?;
                }
            }
            KeyCode::Char(c) => {
                self.status_message.clear();
                self.status_message_expires_at = None;
                self.transient_status_message = None;
                self.model_chooser_search.push(c);
                self.update_model_chooser_filter();
            }
            KeyCode::Backspace => {
                self.status_message.clear();
                self.status_message_expires_at = None;
                self.transient_status_message = None;
                self.model_chooser_search.pop();
                self.update_model_chooser_filter();
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_startup_install_prompt_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let state = self
            .startup_install_prompt
            .as_ref()
            .map(|prompt| prompt.state.clone());
        match state {
            Some(StartupInstallState::Installing { .. }) => return Ok(()),
            Some(StartupInstallState::Complete) | Some(StartupInstallState::Error { .. }) => {
                match key.code {
                    KeyCode::Enter | KeyCode::Esc | KeyCode::Char('1') => {
                        self.startup_install_prompt = None;
                    }
                    _ => {}
                }
                return Ok(());
            }
            Some(StartupInstallState::Ready) => {}
            None => return Ok(()),
        }

        match key.code {
            KeyCode::Left | KeyCode::Up => {
                if let Some(prompt) = self.startup_install_prompt.as_mut() {
                    prompt.install_selected = true;
                }
            }
            KeyCode::Right | KeyCode::Down | KeyCode::Tab => {
                if let Some(prompt) = self.startup_install_prompt.as_mut() {
                    prompt.install_selected = false;
                }
            }
            KeyCode::Char('1') | KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.accept_startup_install_prompt().await?;
            }
            KeyCode::Char('2') | KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.decline_startup_install_prompt().await?;
            }
            KeyCode::Enter => {
                let Some(install_selected) = self
                    .startup_install_prompt
                    .as_ref()
                    .map(|prompt| prompt.install_selected)
                else {
                    return Ok(());
                };
                if install_selected {
                    self.accept_startup_install_prompt().await?;
                } else {
                    self.decline_startup_install_prompt().await?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_startup_workspace_prompt_key(&mut self, key: KeyEvent) -> Result<(), String> {
        match key.code {
            KeyCode::Left | KeyCode::Up => {
                if let Some(prompt) = self.startup_workspace_prompt.as_mut() {
                    prompt.accept_selected = true;
                }
            }
            KeyCode::Right | KeyCode::Down | KeyCode::Tab => {
                if let Some(prompt) = self.startup_workspace_prompt.as_mut() {
                    prompt.accept_selected = false;
                }
            }
            KeyCode::Char('1') | KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.accept_startup_workspace_prompt().await?;
            }
            KeyCode::Char('2') | KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.startup_workspace_prompt = None;
                self.status_message = self.text().workspace_not_bound().to_string();
            }
            KeyCode::Enter => {
                let Some(accept_selected) = self
                    .startup_workspace_prompt
                    .as_ref()
                    .map(|prompt| prompt.accept_selected)
                else {
                    return Ok(());
                };
                if accept_selected {
                    self.accept_startup_workspace_prompt().await?;
                } else {
                    self.startup_workspace_prompt = None;
                    self.status_message = self.text().workspace_not_bound().to_string();
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_startup_update_prompt_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let state = self
            .startup_update_prompt
            .as_ref()
            .map(|prompt| prompt.download_state.clone());
        match state {
            Some(FullUpdateDownloadState::Downloading { .. }) => return Ok(()),
            Some(FullUpdateDownloadState::Complete { .. })
            | Some(FullUpdateDownloadState::Error { .. })
            | Some(FullUpdateDownloadState::CheckError { .. }) => {
                match key.code {
                    KeyCode::Enter | KeyCode::Esc | KeyCode::Char('1') => {
                        self.startup_update_prompt = None;
                    }
                    _ => {}
                }
                return Ok(());
            }
            Some(FullUpdateDownloadState::Ready) => {}
            None => return Ok(()),
        }

        match key.code {
            KeyCode::Left | KeyCode::Up => {
                if let Some(prompt) = self.startup_update_prompt.as_mut() {
                    prompt.download_selected = true;
                }
            }
            KeyCode::Right | KeyCode::Down | KeyCode::Tab => {
                if let Some(prompt) = self.startup_update_prompt.as_mut() {
                    prompt.download_selected = false;
                }
            }
            KeyCode::Char('1') | KeyCode::Char('d') | KeyCode::Char('D') => {
                self.start_full_update_download()?;
            }
            KeyCode::Char('2') | KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Esc => {
                self.startup_update_prompt = None;
                self.status_message = self.text().update_skipped().to_string();
            }
            KeyCode::Enter => {
                let Some(download_selected) = self
                    .startup_update_prompt
                    .as_ref()
                    .map(|prompt| prompt.download_selected)
                else {
                    return Ok(());
                };
                if download_selected {
                    self.start_full_update_download()?;
                } else {
                    self.startup_update_prompt = None;
                    self.status_message = self.text().update_skipped().to_string();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn start_full_update_download(&mut self) -> Result<(), String> {
        let text = self.text();
        let Some(prompt) = self.startup_update_prompt.as_mut() else {
            return Ok(());
        };
        let Some(release_info) = prompt.release_info.as_ref() else {
            return Ok(());
        };
        let (tx, rx) = mpsc::channel::<FullUpdateDownloadMessage>();
        let package_url = release_info.downloadUrl.clone();
        let package_file_name = release_info.assetName.clone();
        let update_target = FullUpdateTarget::cliForCurrentHost()?;
        let work_dir = std::env::temp_dir().join("operit2").join("full_update");
        prompt.progress_rx = Some(rx);
        prompt.download_state = FullUpdateDownloadState::Downloading {
            stage: FullUpdateStage::DownloadingPackage,
            message: text.preparing_download().to_string(),
            read_bytes: 0,
            total_bytes: 0,
            speed_bytes_per_sec: 0,
        };
        self.status_message = text.downloading_full_update_package().to_string();
        tokio::spawn(async move {
            let progress_tx = tx.clone();
            let result = operit_util::GithubReleaseUtil::GithubReleaseUtil::downloadAndPrepareFullUpdateWithProgress(
                package_url,
                package_file_name,
                work_dir,
                move |event| {
                    let _ = progress_tx.send(FullUpdateDownloadMessage::Progress(event));
                },
            )
            .await
            .and_then(|package_path| {
                crate::cli::install_downloaded_cli_update(
                    &update_target,
                    &package_path,
                    crate::cli::InstallOutput::Silent,
                )
                .map(|status| (package_path, Some(status)))
            });
            let _ = tx.send(FullUpdateDownloadMessage::Complete(result));
        });
        Ok(())
    }

    async fn accept_startup_install_prompt(&mut self) -> Result<(), String> {
        let copying_message = self.text().install_command_copying_operit().to_string();
        let installing_message = self.text().install_command_installing().to_string();
        let Some(prompt) = self.startup_install_prompt.as_mut() else {
            return Ok(());
        };
        let (tx, rx) = mpsc::channel::<StartupInstallMessage>();
        prompt.progress_rx = Some(rx);
        prompt.state = StartupInstallState::Installing {
            message: copying_message,
        };
        self.status_message = installing_message;
        std::thread::spawn(move || {
            let progress_tx = tx.clone();
            let result = crate::cli::install_current_cli_with_progress(
                crate::cli::InstallOutput::Silent,
                move |progress| {
                    let _ = progress_tx.send(StartupInstallMessage::Progress(progress));
                },
            );
            let _ = tx.send(StartupInstallMessage::Complete(result));
        });
        Ok(())
    }

    async fn decline_startup_install_prompt(&mut self) -> Result<(), String> {
        self.startup_install_prompt = None;
        crate::tui::mark_startup_install_prompt_declined()?;
        self.status_message = self.text().install_command_skipped().to_string();
        Ok(())
    }

    async fn accept_startup_workspace_prompt(&mut self) -> Result<(), String> {
        let Some(prompt) = self.startup_workspace_prompt.clone() else {
            return Ok(());
        };
        let chat_id = self.current_chat_id()?;
        let bind_result = self
            .core
            .chat_runtime_holder_main()
            .bindChatToWorkspace(chat_id.clone(), prompt.path.clone())
            .await
            .map_err(|error| error.to_string());
        if let Err(error) = bind_result {
            self.set_status_message(error);
            return Ok(());
        }
        self.startup_workspace_prompt = None;
        self.refresh_core_snapshot().await?;
        self.refresh_chats().await;
        self.select_chat_by_id(&chat_id);
        self.status_message = self.text().workspace_bound(&prompt.path);
        Ok(())
    }

    pub(super) async fn cancel_current_request(&mut self) -> Result<(), String> {
        let chat_id = self.current_chat_id()?;
        self.core
            .chat_runtime_holder_main()
            .cancelMessage(chat_id.clone())
            .await
            .map_err(|error| error.to_string())?;
        self.last_current_chat_loading = false;
        self.awaiting_runtime_loading = false;
        self.follow_transcript = true;
        self.status_message = self.text().request_cancelled(&short_chat_label(&chat_id));
        Ok(())
    }

    pub(super) async fn submit_input(&mut self) -> Result<(), String> {
        let input = self.input.trim_end().to_string();
        if input.starts_with('/') {
            self.input.clear();
            self.input_cursor = 0;
            // Slash-command failures (bad arguments, rejected core calls) are
            // status-line material, never reasons to tear down the TUI loop.
            if let Err(error) = self.handle_local_command(&input).await {
                self.status_message = error;
            }
            return Ok(());
        }

        if self.current_chat_is_loading() {
            if self.enqueue_pending_message_from_input() {
                return Ok(());
            }
            self.status_message = self.text().request_already_running().to_string();
            return Ok(());
        }

        let has_queued_attachments =
            !self.queued_attachment_paths.is_empty() || !self.queued_inline_attachments.is_empty();
        if input.trim().is_empty() && !has_queued_attachments {
            return Ok(());
        }

        let chat_id = self.current_chat_id()?;
        let attachment_paths = std::mem::take(&mut self.queued_attachment_paths);
        let inline_attachments = std::mem::take(&mut self.queued_inline_attachments);
        let attachment_tokens = std::mem::take(&mut self.queued_attachment_tokens);
        let message = strip_attachment_tokens(input, &attachment_tokens);
        self.follow_transcript = true;
        self.status_message = self.text().connecting().to_string();
        self.input.clear();
        self.input_cursor = 0;

        let send_args = ChatSendArgs {
            chatId: Some(chat_id),
            message,
            attachmentPaths: attachment_paths,
            replyToTimestamp: None,
        };
        let active_chat_id = self
            .begin_chat_message(send_args, inline_attachments)
            .await?;
        self.refresh_chats().await;
        self.select_chat_by_id(&active_chat_id);
        self.last_current_chat_loading = true;
        self.awaiting_runtime_loading = true;
        self.status_message = self.text().streaming().to_string();
        Ok(())
    }

    pub(super) async fn begin_chat_message(
        &mut self,
        send_args: ChatSendArgs,
        inline_attachments: Vec<AttachmentInfo>,
    ) -> Result<String, String> {
        let chat_binding = self
            .core
            .preferences_functional_config_manager()
            .getModelBindingForFunction(FunctionType::CHAT)
            .await
            .map_err(|error| error.to_string())?;
        if let Some(chat_id) = send_args.chatId.as_ref() {
            self.core
                .chat_runtime_holder_main()
                .switchChat(chat_id.clone())
                .await
                .map_err(|error| error.to_string())?;
        }
        let chat_id = self.current_chat_id()?;
        let mut attachments = build_attachments(&send_args.attachmentPaths)?;
        attachments.extend(inline_attachments);
        let reply_to_message = match send_args.replyToTimestamp {
            Some(timestamp) => Some(
                self.current_messages_cache
                    .iter()
                    .find(|message| message.timestamp == timestamp)
                    .cloned()
                    .ok_or_else(|| format!("reply-to message not found: {timestamp}"))?,
            ),
            None => None,
        };
        self.core
            .chat_runtime_holder_main()
            .sendUserMessage(
                PromptFunctionType::CHAT,
                None,
                Some(chat_id),
                send_args.message,
                None,
                Some(chat_binding.providerId),
                Some(chat_binding.modelId),
                attachments,
                reply_to_message,
                ChatTurnOptions::default(),
            )
            .await
            .map_err(|error| error.to_string())?;
        self.refresh_core_snapshot().await?;
        self.current_chat_id()
    }

    async fn handle_local_command(&mut self, input: &str) -> Result<(), String> {
        let parts =
            split_command_line(input).map_err(|_| self.text().unterminated_quote().to_string())?;
        if parts.is_empty() {
            return Ok(());
        }
        let command = parts[0].trim_start_matches('/');
        // Builtin dispatch is case-insensitive to match the completion popup;
        // plugin commands still receive the original spelling.
        match command.to_ascii_lowercase().as_str() {
            "help" => {
                self.show_help = true;
            }
            "quit" | "exit" => {
                self.should_quit = true;
            }
            "new" => {
                let shell_args = Self::parse_new_chat_args(&parts[1..])
                    .map_err(|()| self.text().usage_new().to_string())?;
                self.create_new_chat(shell_args).await?;
            }
            "switch" => {
                self.toggle_chat_list().await;
            }
            "resume" => {
                self.resume_previous_chat().await?;
            }
            "language" => {
                self.handle_language_command(&parts[1..])?;
            }
            "network" => {
                if let Err(error) = self.handle_network_command(&parts[1..]).await {
                    self.status_message = error;
                }
            }
            "model" => {
                self.handle_model_command(&parts[1..]).await?;
            }
            "approval" => {
                self.handle_approval_command(&parts[1..]).await?;
            }
            "attach" => {
                let path = parts
                    .get(1)
                    .ok_or_else(|| self.text().usage_attach().to_string())?
                    .clone();
                self.queued_attachment_paths.push(path.clone());
                self.status_message = self
                    .text()
                    .queued_attachment(&path, self.queued_attachment_paths.len());
            }
            "attachments" => {
                let queued = self.queued_attachment_labels();
                self.status_message = if queued.is_empty() {
                    self.text().attachments_none().to_string()
                } else {
                    self.text().attachments_list(&queued.join(", "))
                };
            }
            "clear-attachments" => {
                self.clear_queued_attachments();
                self.status_message = self.text().attachments_cleared().to_string();
            }
            "queue" => {
                self.handle_pending_queue_command(&parts[1..]).await?;
            }
            "character" => {
                self.handle_character_command(&parts[1..]).await?;
            }
            "group" => {
                self.handle_group_command(&parts[1..]).await?;
            }
            "skill" => {
                self.handle_skill_command(&parts[1..]).await?;
            }
            "package" => {
                self.handle_package_command(&parts[1..]).await?;
            }
            "plugin" => {
                self.handle_plugin_command(&parts[1..]).await?;
            }
            "mcp" => {
                self.handle_mcp_command(&parts[1..]).await?;
            }
            "tag" => {
                self.handle_tag_command(&parts[1..]).await?;
            }
            "update" => {
                self.handle_update_command().await?;
            }
            _ => {
                self.handle_plugin_core_command(command, &parts[1..]).await;
            }
        }
        Ok(())
    }

    /// `/new` takes repeatable `keyword value` pairs (`character`,
    /// `group-card`, `group`); translate them into the shared shell flags so
    /// `parse_shell_args` stays the single option parser. Legacy `--keyword`
    /// spellings are still accepted.
    fn parse_new_chat_args(args: &[String]) -> Result<ShellArgs, ()> {
        let options = keyword_options_for("new").ok_or(())?;
        let mut translated = Vec::with_capacity(args.len());
        let mut index = 0;
        while index < args.len() {
            let keyword = args[index].trim_start_matches("--").to_ascii_lowercase();
            if !options.contains(&keyword.as_str()) {
                return Err(());
            }
            let Some(value) = args.get(index + 1) else {
                return Err(());
            };
            translated.push(format!("--{keyword}"));
            translated.push(value.clone());
            index += 2;
        }
        parse_shell_args(&translated).map_err(|_| ())
    }

    /// Executes a ToolPkg slash command through the shared Core command dispatcher.
    async fn handle_plugin_core_command(&mut self, command: &str, args: &[String]) {
        let mut command_args = vec![
            "plugin".to_string(),
            "exec".to_string(),
            command.to_string(),
        ];
        command_args.extend_from_slice(args);
        match self.core.runCoreCommand(&command_args).await {
            Ok(output) => {
                let lines = output
                    .stdout
                    .lines()
                    .chain(output.stderr.lines())
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                if lines.len() > 1 {
                    self.open_list_popup(format!("/{command}"), lines);
                } else {
                    self.status_message = lines.join("");
                }
            }
            Err(error) => {
                self.status_message = error.to_string();
            }
        }
    }

    /// Executes a Space control command through the runtime-owned authorization service.
    async fn handle_network_command(&mut self, args: &[String]) -> Result<(), String> {
        const USAGE: &str = "network <show|bootstrap|audit|devices|identities|identity|admit|remove|disconnect|policy|token|prompts|requests|approve|reject|discover|pair|pair-confirm|pair-cancel|peers|unpair|join|joins|join-cancel>";
        match args.first().map(String::as_str) {
            None | Some("show") if args.len() <= 1 => {
                let state = self.networkControl.deviceSpaceControl()?;
                let topology = self.networkControl.deviceSpaceTopology()?;
                // Read-only views share the list popup so output persists
                // until Esc instead of competing for the status line.
                let items = vec![
                    format!(
                        "state: {}",
                        if state.initialized {
                            "ready"
                        } else {
                            "not initialized"
                        }
                    ),
                    format!("devices: {}", topology.devices.len()),
                    format!("identities: {}", state.roles.len()),
                    format!("policies: {}", state.policies.len()),
                ];
                self.open_list_popup("Network".to_string(), items);
            }
            Some("bootstrap") if args.len() == 1 => {
                self.networkControl.bootstrapDeviceSpaceControl()?;
                self.status_message = "network control initialized".to_string();
            }
            Some("audit") if args.len() == 1 => {
                let audit = self.networkControl.deviceSpaceControlAudit()?;
                let items = audit
                    .into_iter()
                    .map(|record| {
                        format!(
                            "{} {}",
                            if record.accepted {
                                "accepted"
                            } else {
                                "rejected"
                            },
                            record.summary,
                        )
                    })
                    .collect::<Vec<_>>();
                self.open_list_popup("Network audit".to_string(), items);
            }
            Some("devices") if args.len() == 1 => {
                self.open_device_manager().await;
            }
            Some("identities") if args.len() == 1 => {
                let state = self.networkControl.deviceSpaceControl()?;
                let items = state
                    .roles
                    .values()
                    .map(network_role_summary)
                    .collect::<Vec<_>>();
                self.open_list_popup("Network identities".to_string(), items);
            }
            Some("identity") if args.len() >= 2 => {
                if args[1] == "list" && args.len() == 2 {
                    let state = self.networkControl.deviceSpaceControl()?;
                    let items = state
                        .roles
                        .values()
                        .map(network_role_summary)
                        .collect::<Vec<_>>();
                    self.open_list_popup("Network identities".to_string(), items);
                    return Ok(());
                }
                if args[1] == "define" && args.len() >= 4 {
                    let role_id = new_network_control_id("identity");
                    self.networkControl
                        .defineDeviceSpaceRole(NetworkControlRole {
                            roleId: role_id,
                            displayName: args[2].clone(),
                            capabilities: network_capabilities(&args[3..])?,
                        })?;
                    self.status_message = format!("network identity defined: {}", args[2]);
                    return Ok(());
                }
                if args[1] == "set" && args.len() == 4 {
                    self.status_message = self.network_assign_identity(&args[2], &args[3]).await?;
                    return Ok(());
                }
                if args[1] == "clear" && args.len() == 3 {
                    self.status_message = self.network_clear_device_identity(&args[2]).await?;
                    return Ok(());
                }
                Err(USAGE.to_string())?
            }
            Some("admit") if args.len() == 2 => {
                self.status_message = self.network_admit_device(&args[1]).await?;
            }
            Some("remove") if args.len() == 2 => {
                self.status_message = self.network_remove_device(&args[1]).await?;
            }
            Some("disconnect") if args.len() == 2 => {
                self.status_message = self.network_disconnect_device(&args[1]).await?;
            }
            Some("policy") if args.len() == 2 && args[1] == "list" => {
                let items = self
                    .networkControl
                    .deviceSpaceControl()?
                    .policies
                    .into_iter()
                    .map(|(policyId, value)| format!("{policyId}={value}"))
                    .collect::<Vec<_>>();
                self.open_list_popup("Network policies".to_string(), items);
            }
            Some("policy") if args.len() == 4 && args[1] == "set" => {
                self.networkControl
                    .updateDeviceSpacePolicy(args[2].clone(), args[3].clone())?;
                self.status_message = format!("network policy updated: {}", args[2]);
            }
            Some("token") if args.len() == 1 => {
                // The pairing token must stay on screen for transfer to the
                // other device, so it goes to the popup instead of the status line.
                let token = self.networkControl.localPairingToken()?;
                self.open_list_popup("Network token".to_string(), vec![token]);
            }
            Some("prompts") if args.len() == 1 => {
                let prompts = self.networkControl.pairingPrompts()?;
                if prompts.is_empty() {
                    self.status_message = self.text().network_prompts_none().to_string();
                } else {
                    self.open_pairing_prompts_popup(&prompts);
                }
            }
            Some("requests") if args.len() == 1 => {
                let requests = self.networkControl.incomingDeviceSpaceJoins().await?;
                if requests.is_empty() {
                    self.status_message = self.text().network_requests_none().to_string();
                } else {
                    self.open_join_decision_modal(requests);
                    if self.join_decision.is_none() {
                        self.status_message = self.text().network_requests_none().to_string();
                    }
                }
            }
            Some("approve" | "reject") if args.len() == 3 => {
                let assignment_version = args[2]
                    .parse::<u64>()
                    .map_err(|_| "assignment-version must be a non-negative integer".to_string())?;
                let request_id = self.resolve_join_request_argument(&args[1]).await?;
                let request = self
                    .networkControl
                    .decideDeviceSpaceJoin(request_id, assignment_version, args[0] == "approve")
                    .await?;
                self.status_message = format!(
                    "network join request {} {}: {:?}",
                    args[1],
                    if args[0] == "approve" { "approved" } else { "rejected" },
                    request.status,
                );
            }
            Some("discover") if args.len() <= 2 => {
                self.network_discover(&args[1..]).await?;
            }
            Some("pair") if args.len() >= 3 => {
                self.network_pair(&args[1..]).await?;
            }
            Some("pair-confirm") if (2..=3).contains(&args.len()) => {
                self.status_message = self.network_pair_confirm(&args[1..]).await?;
            }
            Some("pair-cancel") if args.len() <= 2 => {
                self.status_message = self.network_pair_cancel(&args[1..]).await?;
            }
            Some("peers") if args.len() == 1 => {
                self.network_peers().await?;
            }
            Some("unpair") if args.len() == 2 => {
                self.status_message = self.network_unpair(&args[1]).await?;
            }
            Some("join") if args.len() == 2 => {
                self.status_message = self.network_join(&args[1]).await?;
            }
            Some("joins") if args.len() == 1 => {
                self.network_joins().await?;
            }
            Some("join-cancel") if args.len() == 2 => {
                self.status_message = self.network_join_cancel(&args[1]).await?;
            }
            _ => {
                self.status_message = USAGE.to_string();
            }
        }
        Ok(())
    }

    /// Shared device-action layer for the `/network` slash commands and the
    /// device management window: each helper resolves the human-facing
    /// device reference, performs exactly one control operation, and returns
    /// the status text so both entry points cannot drift apart.
    async fn network_admit_device(&mut self, device: &str) -> Result<String, String> {
        let topology = self.networkControl.deviceSpaceTopology()?;
        let device_id = network_device_id(&topology, device)?;
        let device_label = network_device_label_by_id(&topology, &device_id)?;
        self.networkControl.admitDeviceSpaceMember(device_id)?;
        Ok(format!("network member admitted: {device_label}"))
    }

    async fn network_remove_device(&mut self, device: &str) -> Result<String, String> {
        let topology = self.networkControl.deviceSpaceTopology()?;
        let device_id = network_device_id(&topology, device)?;
        let device_label = network_device_label_by_id(&topology, &device_id)?;
        self.networkControl.removeDeviceSpaceMember(device_id).await?;
        Ok(format!("network member removed: {device_label}"))
    }

    async fn network_disconnect_device(&mut self, device: &str) -> Result<String, String> {
        let topology = self.networkControl.deviceSpaceTopology()?;
        let device_id = network_device_id(&topology, device)?;
        let device_label = network_device_label_by_id(&topology, &device_id)?;
        self.networkControl
            .disconnectDeviceSpaceNode(device_id)
            .await?;
        Ok(format!("network node disconnected: {device_label}"))
    }

    async fn network_assign_identity(
        &mut self,
        device: &str,
        identity: &str,
    ) -> Result<String, String> {
        let state = self.networkControl.deviceSpaceControl()?;
        let topology = self.networkControl.deviceSpaceTopology()?;
        let device_id = network_device_id(&topology, device)?;
        let device_label = network_device_label_by_id(&topology, &device_id)?;
        let identity_id = network_role_id(&state, identity)?;
        let identity_label = state
            .roles
            .get(&identity_id)
            .map(|role| role.displayName.clone())
            .ok_or_else(|| format!("network identity does not exist: {identity}"))?;
        self.networkControl.setDeviceSpaceIdentity(
            NetworkControlIdentityAssignment {
                nodeId: device_id,
                roleId: identity_id,
            },
        )?;
        Ok(format!("network identity set: {device_label} · {identity_label}"))
    }

    async fn network_clear_device_identity(&mut self, device: &str) -> Result<String, String> {
        let topology = self.networkControl.deviceSpaceTopology()?;
        let device_id = network_device_id(&topology, device)?;
        self.networkControl.clearDeviceSpaceIdentity(device_id)?;
        Ok(format!("network identity reset to default: {device}"))
    }

    /// Lists unpaired LAN discovery candidates so `/network pair` has an
    /// address to dial. The result stays in the popup because the address is
    /// typed into the next command.
    async fn network_discover(&mut self, args: &[String]) -> Result<(), String> {
        const USAGE: &str = "usage: network discover [timeout-ms]";
        let timeout_ms = match args {
            [] => 3000,
            [timeout_ms] => timeout_ms.parse::<u64>().map_err(|_| USAGE.to_string())?,
            _ => return Err(USAGE.to_string()),
        };
        let candidates = self.networkControl.discoverPeers(timeout_ms).await?;
        if candidates.is_empty() {
            self.set_transient_status_message(self.text().network_discover_none().to_string());
            return Ok(());
        }
        let text = self.text();
        let mut items = candidates
            .iter()
            .map(|peer| format!("{} · {} · {}", peer.displayName, peer.nodeId, peer.address))
            .collect::<Vec<_>>();
        items.push(text.network_discover_hint().to_string());
        self.open_list_popup(text.network_discover_title().to_string(), items);
        Ok(())
    }

    /// Starts an outbound pairing toward an explicit address. The pairing id
    /// must stay on screen for the confirmation step, so it goes to the popup
    /// and is remembered for id-less `/network pair-confirm <code>`.
    async fn network_pair(&mut self, args: &[String]) -> Result<(), String> {
        let (address, transport, token) = parse_pair_arguments(args)?;
        let pending = self
            .networkControl
            .startPairing(String::new(), address, transport, token)
            .await?;
        let text = self.text();
        let mut items = vec![format!(
            "{}: {}",
            text.network_pair_id_label(),
            pending.pairingId
        )];
        if !pending.displayName.is_empty() {
            items.push(pending.displayName.clone());
        }
        items.push(text.network_pair_hint(&pending.pairingId));
        self.pending_pairings.push(pending);
        self.open_list_popup(text.network_pair_title().to_string(), items);
        Ok(())
    }

    /// Confirms an outbound pairing with the code shown on the other device.
    /// With exactly one pending pairing the id can be omitted. A failed
    /// confirmation keeps the pairing pending so the code can be retried.
    async fn network_pair_confirm(&mut self, args: &[String]) -> Result<String, String> {
        const USAGE: &str = "usage: network pair-confirm [pairing-id] <code>";
        let (pairing_id, code) = match args {
            [code] => (
                resolve_pending_pairing(&self.pending_pairings, None)?
                    .pairingId
                    .clone(),
                code.clone(),
            ),
            [pairing_id, code] => (pairing_id.clone(), code.clone()),
            _ => return Err(USAGE.to_string()),
        };
        let peer = self
            .networkControl
            .finishPairing(pairing_id.clone(), code)
            .await?;
        self.pending_pairings
            .retain(|pending| pending.pairingId != pairing_id);
        Ok(format!(
            "network pairing confirmed: {} ({})",
            peer.displayName, peer.nodeId
        ))
    }

    /// Cancels a pending pairing this session started (id omitted) or any
    /// pairing by id. Cancelling never removes an established pairing.
    async fn network_pair_cancel(&mut self, args: &[String]) -> Result<String, String> {
        const USAGE: &str = "usage: network pair-cancel [pairing-id]";
        let pairing_id = match args {
            [] => resolve_pending_pairing(&self.pending_pairings, None)?
                .pairingId
                .clone(),
            [pairing_id] => pairing_id.clone(),
            _ => return Err(USAGE.to_string()),
        };
        self.networkControl
            .cancelPairing(pairing_id.clone())
            .await?;
        self.pending_pairings
            .retain(|pending| pending.pairingId != pairing_id);
        Ok(format!("network pairing cancelled: {pairing_id}"))
    }

    /// Lists paired devices with their authorization directions. The node id
    /// is required input for `/network unpair` and `/network join`.
    async fn network_peers(&mut self) -> Result<(), String> {
        let peers = self.networkControl.pairedDevicesSnapshot()?;
        if peers.is_empty() {
            self.set_transient_status_message(self.text().network_peers_none().to_string());
            return Ok(());
        }
        let text = self.text();
        let items = peers
            .into_iter()
            .map(|(device_id, peer)| {
                format!(
                    "{} · {} · {}",
                    paired_device_label(&device_id, &peer),
                    device_id,
                    paired_device_direction(peer.inbound, peer.outbound),
                )
            })
            .collect::<Vec<_>>();
        self.open_list_popup(text.network_peers_title().to_string(), items);
        Ok(())
    }

    /// Removes one pairing credential without touching Space membership;
    /// `/network remove` is the full forget for Space members.
    async fn network_unpair(&mut self, device: &str) -> Result<String, String> {
        let (device_id, label) = self.resolve_paired_device_argument(device, false).await?;
        self.networkControl.removePairedDevice(device_id).await?;
        Ok(format!("network pairing removed: {label}"))
    }

    /// Requests to join the paired device's Space. Approval happens on the
    /// target device; this session pulls the decision in the background.
    async fn network_join(&mut self, device: &str) -> Result<String, String> {
        let (device_id, label) = self.resolve_paired_device_argument(device, true).await?;
        let request = self
            .networkControl
            .requestDeviceSpaceJoin(device_id)
            .await?;
        if request.status == SpaceJoinStatus::Joined {
            return Ok(format!(
                "network join completed: now a member of {}",
                request.spaceName
            ));
        }
        Ok(format!(
            "network join request submitted: {} must approve it; this TUI refreshes the status automatically",
            label
        ))
    }

    /// Lists this device's outgoing join requests with their live status.
    async fn network_joins(&mut self) -> Result<(), String> {
        let requests = self.networkControl.outgoingDeviceSpaceJoins()?;
        if requests.is_empty() {
            self.set_transient_status_message(self.text().network_joins_none().to_string());
            return Ok(());
        }
        let text = self.text();
        let items = requests
            .iter()
            .map(|request| {
                format!(
                    "{} · {} · {} · {}",
                    request.targetDeviceId,
                    request.spaceName,
                    join_status_label(&request.status),
                    request.requestId,
                )
            })
            .collect::<Vec<_>>();
        self.open_list_popup(text.network_joins_title().to_string(), items);
        Ok(())
    }

    /// Withdraws an outgoing join request by id or by its target device.
    async fn network_join_cancel(&mut self, value: &str) -> Result<String, String> {
        let requests = self.networkControl.outgoingDeviceSpaceJoins()?;
        let request_id = if requests.iter().any(|request| request.requestId == value) {
            value.to_string()
        } else {
            let matches = requests
                .iter()
                .filter(|request| {
                    request.targetDeviceId == value && space_join_is_active(&request.status)
                })
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [request] => request.requestId.clone(),
                _ => return Err(format!(
                    "no outgoing join request matches \"{value}\"; list them with /network joins"
                )),
            }
        };
        let request = self
            .networkControl
            .cancelDeviceSpaceJoin(request_id.clone())
            .await?;
        Ok(format!(
            "network join request cancelled: {} ({})",
            join_status_label(&request.status),
            request_id
        ))
    }

    /// Resolves a `/network unpair|join` argument against paired devices:
    /// exact node id, or a unique display-name match. Join additionally
    /// requires the outbound authorization direction.
    async fn resolve_paired_device_argument(
        &self,
        value: &str,
        require_outbound: bool,
    ) -> Result<(String, String), String> {
        let peers = self.networkControl.pairedDevicesSnapshot()?;
        let (device_id, peer) = match peers.get(value) {
            Some(peer) => (value.to_string(), peer.clone()),
            None => {
                let matches = peers
                    .iter()
                    .filter(|(_, peer)| peer.deviceInfo.model == value)
                    .collect::<Vec<_>>();
                match matches.as_slice() {
                    [(device_id, peer)] => ((*device_id).clone(), (*peer).clone()),
                    [] => {
                        return Err(format!(
                            "no paired device matches \"{value}\"; list them with /network peers"
                        ))
                    }
                    _ => {
                        return Err(format!(
                            "paired device name is ambiguous: {value}; use the node id from /network peers"
                        ))
                    }
                }
            }
        };
        let label = paired_device_label(&device_id, &peer);
        if require_outbound && !peer.outbound {
            return Err(format!(
                "network join requires an outbound pairing with {label}"
            ));
        }
        Ok((device_id, label))
    }

    /// Resolves a `/network approve|reject` argument into the pending join
    /// request id. Accepts the exact request id, device id, or applicant name;
    /// ambiguous names are rejected so a decision never hits the wrong request.
    async fn resolve_join_request_argument(&self, value: &str) -> Result<String, String> {
        let requests = self.networkControl.incomingDeviceSpaceJoins().await?;
        if requests.iter().any(|request| request.requestId == value) {
            return Ok(value.to_string());
        }
        let matches = requests
            .iter()
            .filter(|request| {
                request.applicantDeviceId == value || request.applicantName == value
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [request] => Ok(request.requestId.clone()),
            [] => Err(format!(
                "no pending join request matches \"{value}\"; list them with /network requests"
            )),
            _ => Err(format!(
                "join request name is ambiguous: {value}; use the request id from /network requests"
            )),
        }
    }

    fn handle_language_command(&mut self, args: &[String]) -> Result<(), String> {
        match args.first().map(String::as_str) {
            None | Some("status") => {
                self.status_message = self.text().language_status(self.language);
                Ok(())
            }
            Some("help") => {
                self.status_message = self.text().language_usage().to_string();
                Ok(())
            }
            Some(value) => {
                let language = TuiLanguage::from_language_code(value)
                    .map_err(|_| self.text().unsupported_language(value))?;
                language.save()?;
                self.language = language;
                self.transcript_render_cache.clear();
                self.status_message = self.text().language_updated(language);
                Ok(())
            }
        }
    }

    async fn handle_approval_key(&mut self, key: KeyEvent) {
        let result = match key.code {
            KeyCode::Char('1') | KeyCode::Char('y') | KeyCode::Char('Y') => "allow",
            KeyCode::Char('2') | KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => "deny",
            KeyCode::Char('3') | KeyCode::Char('a') | KeyCode::Char('A') => "allow_session",
            _ => return,
        };
        let Some(request) = self.current_tool_permission_requests.first().cloned() else {
            return;
        };
        if let Err(error) = self
            .core
            .respondToolPermission(
                request.chatId.clone(),
                request.requestId.clone(),
                result.to_string(),
            )
            .await
        {
            self.status_message = error;
            return;
        }
        // The chat state watch confirms the removal; drop it locally so a
        // second keypress cannot answer the same request twice.
        self.current_tool_permission_requests.remove(0);
        self.status_message = match result {
            "allow" => self.text().tool_approved_once().to_string(),
            "allow_session" => self.text().tool_approved_remembered().to_string(),
            _ => self.text().tool_denied().to_string(),
        };
    }

    /// Handles keys for the Space join decision popup. Y/N decide the
    /// selected request in place, Up/Down move between queued requests, and
    /// Esc defers everything - rejection has side effects for the applicant,
    /// so only an explicit N rejects.
    async fn handle_join_decision_key(&mut self, key: KeyEvent) -> Result<(), String> {
        match key.code {
            KeyCode::Up => {
                if let Some(modal) = self.join_decision.as_mut() {
                    modal.selected = modal.selected.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if let Some(modal) = self.join_decision.as_mut() {
                    if modal.selected + 1 < modal.requests.len() {
                        modal.selected += 1;
                    }
                }
            }
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Char('1') => {
                self.decide_selected_join(true).await;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('2') => {
                self.decide_selected_join(false).await;
            }
            KeyCode::Esc => {
                self.join_decision = None;
                self.status_message = self.text().network_join_decision_deferred().to_string();
            }
            _ => {}
        }
        Ok(())
    }

    /// Decides the selected join request through the runtime link service.
    /// Decision failures keep the popup open so the request can be retried;
    /// success drops the request from the queue and closes the popup when
    /// nothing remains.
    async fn decide_selected_join(&mut self, approve: bool) {
        let Some(modal) = self.join_decision.as_ref() else {
            return;
        };
        let Some(request) = modal.requests.get(modal.selected) else {
            self.join_decision = None;
            return;
        };
        let decision = self
            .networkControl
            .decideDeviceSpaceJoin(
                request.requestId.clone(),
                request.assignmentVersion,
                approve,
            )
            .await;
        let modal = self.join_decision.as_mut().expect("join modal checked above");
        match decision {
            Ok(updated) => {
                self.status_message = format!(
                    "network join request {}: {:?}",
                    updated.applicantName, updated.status
                );
                modal.requests.remove(modal.selected);
                if modal.selected >= modal.requests.len() {
                    modal.selected = modal.requests.len().saturating_sub(1);
                }
                if modal.requests.is_empty() {
                    self.join_decision = None;
                }
            }
            Err(error) => {
                self.status_message = error;
            }
        }
    }

    /// Opens the device management window over a fresh control snapshot.
    /// The window is keyboard-driven like the join decision modal; the
    /// mouse overlay flag only keeps clicks from leaking into the
    /// transcript underneath.
    async fn open_device_manager(&mut self) {
        let state = match self.networkControl.deviceSpaceControl() {
            Ok(state) => state,
            Err(error) => {
                self.status_message = error;
                return;
            }
        };
        let topology = match self.networkControl.deviceSpaceTopology() {
            Ok(topology) => topology,
            Err(error) => {
                self.status_message = error;
                return;
            }
        };
        let requests = self
            .networkControl
            .incomingDeviceSpaceJoins()
            .await
            .unwrap_or_default();
        self.device_manager = Some(DeviceManagerModal {
            initialized: state.initialized,
            topology,
            blocked: state.disconnectedNodeIds,
            requests: requests
                .into_iter()
                .filter(|request| request.canApprove)
                .collect(),
            roles: state.roles,
            selected: 0,
            mode: DeviceManagerMode::Browsing,
            menu_device_id: None,
            menu_index: 0,
        });
    }

    /// Re-pulls the control snapshot into an open device window, keeping
    /// the selection on the same row identity. A device whose action menu
    /// is open disappears from the topology, the window drops back to
    /// browsing instead of managing a ghost row.
    async fn refresh_device_manager_snapshot(&mut self) {
        let selected_id = self
            .device_manager
            .as_ref()
            .and_then(|modal| modal.selected_row())
            .map(|row| row.id().to_string());
        let Ok(state) = self.networkControl.deviceSpaceControl() else {
            return;
        };
        let Ok(topology) = self.networkControl.deviceSpaceTopology() else {
            return;
        };
        let requests = self
            .networkControl
            .incomingDeviceSpaceJoins()
            .await
            .unwrap_or_default();
        let Some(modal) = self.device_manager.as_mut() else {
            return;
        };
        modal.initialized = state.initialized;
        modal.blocked = state.disconnectedNodeIds;
        modal.roles = state.roles;
        modal.topology = topology;
        modal.requests = requests
            .into_iter()
            .filter(|request| request.canApprove)
            .collect();
        if let Some(menu_device_id) = modal.menu_device_id.clone() {
            if !modal
                .topology
                .devices
                .iter()
                .any(|device| device.deviceId == menu_device_id)
            {
                modal.mode = DeviceManagerMode::Browsing;
                modal.menu_device_id = None;
                modal.menu_index = 0;
            }
        }
        if let Some(selected_id) = selected_id {
            if let Some(position) = modal
                .rows()
                .iter()
                .position(|row| row.id() == selected_id)
            {
                modal.selected = position;
            }
        }
        modal.selected = modal.selected.min(modal.rows().len().saturating_sub(1));
    }

    /// Routes keys inside the device management window by mode.
    async fn handle_device_manager_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let mode = self
            .device_manager
            .as_ref()
            .expect("device window checked above")
            .mode;
        match mode {
            DeviceManagerMode::Browsing => self.handle_device_manager_browse_key(key).await,
            DeviceManagerMode::ActionMenu => self.handle_device_manager_menu_key(key).await,
            DeviceManagerMode::ConfirmRemove => self.handle_device_manager_confirm_key(key).await,
            DeviceManagerMode::AssignIdentity => self.handle_device_manager_assign_key(key).await,
        }
    }

    async fn handle_device_manager_browse_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let row_count = self
            .device_manager
            .as_ref()
            .map(|modal| modal.rows().len())
            .unwrap_or(0);
        match key.code {
            KeyCode::Up => {
                if let Some(modal) = self.device_manager.as_mut() {
                    modal.selected = modal.selected.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if let Some(modal) = self.device_manager.as_mut() {
                    if modal.selected + 1 < row_count {
                        modal.selected += 1;
                    }
                }
            }
            // Only offered while the Space control is uninitialized; boot-
            // strapping an initialized Space again is the slash command's job.
            KeyCode::Char('b') | KeyCode::Char('B') => {
                let initialized = self
                    .device_manager
                    .as_ref()
                    .is_some_and(|modal| modal.initialized);
                if !initialized {
                    match self.networkControl.bootstrapDeviceSpaceControl() {
                        Ok(_) => self.refresh_device_manager_snapshot().await,
                        Err(error) => self.status_message = error,
                    }
                }
            }
            KeyCode::Enter => {
                let Some(row) = self
                    .device_manager
                    .as_ref()
                    .and_then(|modal| modal.selected_row())
                else {
                    return Ok(());
                };
                match row {
                    DeviceManagerRow::Request(request) => {
                        self.open_join_decision_modal(vec![request]);
                        if self.join_decision.is_none() {
                            self.status_message = self.text().network_requests_none().to_string();
                        }
                    }
                    DeviceManagerRow::Device(device) => {
                        // A device without any applicable action (the local
                        // device with no roles defined) must not open an
                        // empty, unhighlightable menu.
                        if self
                            .device_manager
                            .as_ref()
                            .is_some_and(|modal| !modal.menu_actions(&device.deviceId).is_empty())
                        {
                            if let Some(modal) = self.device_manager.as_mut() {
                                modal.mode = DeviceManagerMode::ActionMenu;
                                modal.menu_device_id = Some(device.deviceId.clone());
                                modal.menu_index = 0;
                            }
                        }
                    }
                }
            }
            KeyCode::Esc => {
                self.device_manager = None;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_device_manager_menu_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let Some((device_id, actions)) = self.device_manager.as_ref().and_then(|modal| {
            let device_id = modal.menu_device_id.clone()?;
            let actions = modal.menu_actions(&device_id);
            Some((device_id, actions))
        }) else {
            return Ok(());
        };
        let menu_index = self
            .device_manager
            .as_ref()
            .map(|modal| modal.menu_index)
            .unwrap_or(0);
        match key.code {
            KeyCode::Up => {
                if let Some(modal) = self.device_manager.as_mut() {
                    modal.menu_index = modal.menu_index.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if let Some(modal) = self.device_manager.as_mut() {
                    if modal.menu_index + 1 < actions.len() {
                        modal.menu_index += 1;
                    }
                }
            }
            KeyCode::Enter => {
                let Some(action) = actions.get(menu_index).copied() else {
                    return Ok(());
                };
                if action == DeviceManagerAction::AssignIdentity {
                    if let Some(modal) = self.device_manager.as_mut() {
                        modal.mode = DeviceManagerMode::AssignIdentity;
                        modal.menu_index = 0;
                    }
                    return Ok(());
                }
                // Removal is destructive and irreversible; everything else
                // runs immediately.
                if action == DeviceManagerAction::Remove {
                    if let Some(modal) = self.device_manager.as_mut() {
                        modal.mode = DeviceManagerMode::ConfirmRemove;
                        modal.menu_index = 0;
                    }
                    return Ok(());
                }
                self.run_device_manager_action(&device_id, action).await;
            }
            KeyCode::Esc => {
                if let Some(modal) = self.device_manager.as_mut() {
                    modal.mode = DeviceManagerMode::Browsing;
                    modal.menu_device_id = None;
                    modal.menu_index = 0;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Removal is the only destructive action, so it keeps the join
    /// decision modal's "no side effects on a stray key" rule: only an
    /// explicit Y removes.
    async fn handle_device_manager_confirm_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let Some(device_id) = self
            .device_manager
            .as_ref()
            .and_then(|modal| modal.menu_device_id.clone())
        else {
            return Ok(());
        };
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Char('1') => {
                self.run_device_manager_action(&device_id, DeviceManagerAction::Remove)
                    .await;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('2') | KeyCode::Esc => {
                if let Some(modal) = self.device_manager.as_mut() {
                    modal.mode = DeviceManagerMode::Browsing;
                    modal.menu_device_id = None;
                    modal.menu_index = 0;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_device_manager_assign_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let role_count = self
            .device_manager
            .as_ref()
            .map(|modal| modal.sorted_roles().len())
            .unwrap_or(0);
        match key.code {
            KeyCode::Up => {
                if let Some(modal) = self.device_manager.as_mut() {
                    modal.menu_index = modal.menu_index.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if let Some(modal) = self.device_manager.as_mut() {
                    if modal.menu_index + 1 < role_count {
                        modal.menu_index += 1;
                    }
                }
            }
            KeyCode::Enter => {
                let Some((device_id, role_id)) = self.device_manager.as_ref().and_then(|modal| {
                    let device_id = modal.menu_device_id.clone()?;
                    modal
                        .sorted_roles()
                        .get(modal.menu_index)
                        .map(|role| (device_id, role.roleId.clone()))
                }) else {
                    return Ok(());
                };
                self.run_device_manager_assign(&device_id, &role_id).await;
            }
            KeyCode::Esc => {
                if let Some(modal) = self.device_manager.as_mut() {
                    modal.mode = DeviceManagerMode::Browsing;
                    modal.menu_device_id = None;
                    modal.menu_index = 0;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Runs one confirmed device action through the shared action layer.
    /// Failures keep the window open so the action can be retried. The
    /// admit feedback deliberately avoids promising a reconnection: admit
    /// only lifts the policy restriction, the link returns on its own.
    async fn run_device_manager_action(&mut self, device_id: &str, action: DeviceManagerAction) {
        let result = match action {
            DeviceManagerAction::Admit => self.network_admit_device(device_id).await,
            DeviceManagerAction::Disconnect => self.network_disconnect_device(device_id).await,
            DeviceManagerAction::Remove => self.network_remove_device(device_id).await,
            DeviceManagerAction::ClearIdentity => {
                self.network_clear_device_identity(device_id).await
            }
            DeviceManagerAction::AssignIdentity => return,
        };
        if let Some(modal) = self.device_manager.as_mut() {
            modal.mode = DeviceManagerMode::Browsing;
            modal.menu_device_id = None;
            modal.menu_index = 0;
        }
        match result {
            Ok(message) => {
                self.status_message = if action == DeviceManagerAction::Admit {
                    let label = self
                        .device_manager
                        .as_ref()
                        .and_then(|modal| {
                            modal
                                .topology
                                .devices
                                .iter()
                                .find(|device| device.deviceId == device_id)
                        })
                        .map(|device| device.deviceName.clone())
                        .unwrap_or_else(|| device_id.to_string());
                    self.text().network_devices_admitted(&label)
                } else {
                    message
                };
            }
            Err(error) => self.status_message = error,
        }
        self.refresh_device_manager_snapshot().await;
    }

    async fn run_device_manager_assign(&mut self, device_id: &str, role_id: &str) {
        let result = self.network_assign_identity(device_id, role_id).await;
        if let Some(modal) = self.device_manager.as_mut() {
            modal.mode = DeviceManagerMode::Browsing;
            modal.menu_device_id = None;
            modal.menu_index = 0;
        }
        match result {
            Ok(message) => self.status_message = message,
            Err(error) => self.status_message = error,
        }
        self.refresh_device_manager_snapshot().await;
    }

    async fn handle_approval_command(&mut self, args: &[String]) -> Result<(), String> {
        match args.first().map(String::as_str) {
            None | Some("status") => {
                let mode = self
                    .core
                    .permissions_tool_permission_system()
                    .getAiPermissionMode()
                    .await
                    .map_err(|error| error.to_string())?;
                self.status_message = self.text().approval_status(mode.name(), 0);
            }
            Some("allow") | Some("ask") | Some("forbid") => {
                let level = parse_permission_level(args.first().map(String::as_str))?;
                self.core
                    .permissions_tool_permission_system()
                    .saveAiPermissionMode(level.clone())
                    .await
                    .map_err(|error| error.to_string())?;
                self.status_message = self.text().approval_master(level.name());
            }
            Some("tool") => {
                return Err(
                    "per-tool permission overrides are not supported by AiPermissionMode"
                        .to_string(),
                );
            }
            Some("list") => {
                self.status_message = self.text().approval_overrides_none().to_string();
            }
            Some("help") => {
                self.status_message = self.text().approval_help().to_string();
            }
            Some(other) => {
                self.status_message = self.text().unknown_approval_command(other);
            }
        }
        Ok(())
    }

    async fn handle_model_command(&mut self, args: &[String]) -> Result<(), String> {
        match args.first().map(String::as_str) {
            None | Some("current") => self.show_current_chat_model().await,
            Some("list") => self.list_chat_models().await,
            Some("choose") => self.open_model_chooser().await,
            Some("config") => self.open_config_popup().await,
            Some("use") => self.use_chat_model(&args[1..]).await,
            Some("help") => {
                self.status_message = self.text().model_help().to_string();
                Ok(())
            }
            Some(other) => {
                self.status_message = self.text().unknown_model_command(other);
                Ok(())
            }
        }
    }

    async fn show_current_chat_model(&mut self) -> Result<(), String> {
        let (provider_id, model_id, provider_name) = self.current_chat_model_status_parts().await?;
        self.status_message =
            self.text()
                .chat_model_status(&provider_id, &provider_name, &model_id);
        self.refresh_context_usage_label().await;
        Ok(())
    }

    async fn current_chat_model_status_parts(
        &mut self,
    ) -> Result<(String, String, String), String> {
        let binding = self
            .core
            .preferences_functional_config_manager()
            .getModelBindingForFunction(FunctionType::CHAT)
            .await
            .map_err(|error| error.to_string())?;
        let config = self
            .core
            .preferences_model_config_manager()
            .getResolvedModelConfig(&binding.providerId, &binding.modelId)
            .await
            .map_err(|error| error.to_string())?;
        Ok((binding.providerId, binding.modelId, config.providerName))
    }

    async fn current_chat_model_status_label(&mut self) -> Result<String, String> {
        let (_, model_id, provider_name) = self.current_chat_model_status_parts().await?;
        Ok(format!("{provider_name} / {model_id}"))
    }

    async fn current_chat_model_ref(&mut self) -> Result<ModelRef, String> {
        let binding = self
            .core
            .preferences_functional_config_manager()
            .getModelBindingForFunction(FunctionType::CHAT)
            .await
            .map_err(|error| error.to_string())?;
        Ok(ModelRef {
            provider_id: binding.providerId,
            model_id: binding.modelId,
        })
    }

    async fn editable_chat_model_ref(&mut self) -> Result<ModelRef, String> {
        if let ActivePrompt::CharacterCard { id } = self
            .core
            .preferences_active_prompt_manager()
            .getActivePrompt()
            .await
            .map_err(|error| error.to_string())?
        {
            let card = self
                .core
                .preferences_character_card_manager()
                .getCharacterCard(&id)
                .await
                .map_err(|error| error.to_string())?;
            let binding_mode = CharacterCardChatModelBindingMode::normalize(Some(
                card.chatModelBindingMode.as_str(),
            ));
            if binding_mode == CharacterCardChatModelBindingMode::FIXED_MODEL {
                let model_id = card
                    .chatModelId
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| format!("character card fixed model is empty: {id}"))?;
                return Ok(ModelRef {
                    provider_id: ModelConfigManager::DEFAULT_PROVIDER_ID.to_string(),
                    model_id,
                });
            }
        }
        self.current_chat_model_ref().await
    }

    async fn list_chat_models(&mut self) -> Result<(), String> {
        self.model_list_mode = true;
        self.open_model_chooser().await
    }

    async fn open_model_chooser(&mut self) -> Result<(), String> {
        self.model_choices = self.load_model_choices().await?;
        if self.model_choices.is_empty() {
            self.status_message = self.text().no_model_configs().to_string();
            return Ok(());
        }
        self.selected_model_choice_index = self
            .model_choices
            .iter()
            .position(|choice| choice.selected)
            .expect("current chat model mapping must be present in model choices");
        self.show_model_chooser = true;
        self.focus = FocusArea::ModelChooser;
        self.model_chooser_search.clear();
        self.update_model_chooser_filter();
        if !self.model_list_mode {
            self.status_message = self.text().choose_model_status().to_string();
        }
        Ok(())
    }

    fn close_model_chooser(&mut self) {
        self.show_model_chooser = false;
        self.focus = FocusArea::Input;
        self.model_list_mode = false;
        self.model_chooser_search.clear();
        self.model_chooser_filtered_indices.clear();
        self.status_message = self.text().model_chooser_closed().to_string();
    }

    fn open_list_popup(&mut self, title: String, items: Vec<String>) {
        self.list_popup_title = title;
        self.list_popup_items = items;
        self.list_popup_search.clear();
        self.list_popup_selected_index = 0;
        self.update_list_popup_filter();
        self.show_list_popup = true;
        self.focus = FocusArea::Input;
    }

    fn close_list_popup(&mut self) {
        self.show_list_popup = false;
        self.list_popup_title.clear();
        self.list_popup_items.clear();
        self.list_popup_search.clear();
        self.list_popup_filtered_indices.clear();
        self.list_popup_selected_index = 0;
    }

    fn update_list_popup_filter(&mut self) {
        let search = self.list_popup_search.to_ascii_lowercase();
        if search.is_empty() {
            self.list_popup_filtered_indices = (0..self.list_popup_items.len()).collect();
        } else {
            self.list_popup_filtered_indices = self
                .list_popup_items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.to_ascii_lowercase().contains(&search))
                .map(|(index, _)| index)
                .collect();
        }
        if self.list_popup_selected_index >= self.list_popup_filtered_indices.len() {
            self.list_popup_selected_index =
                self.list_popup_filtered_indices.len().saturating_sub(1);
        }
    }

    fn handle_list_popup_key(&mut self, key: KeyEvent) -> Result<(), String> {
        match key.code {
            KeyCode::Up => {
                if self.list_popup_selected_index > 0 {
                    self.list_popup_selected_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.list_popup_selected_index + 1 < self.list_popup_filtered_indices.len() {
                    self.list_popup_selected_index += 1;
                }
            }
            KeyCode::Esc => {
                self.close_list_popup();
            }
            KeyCode::Enter => {
                self.close_list_popup();
            }
            KeyCode::Char(c) => {
                self.list_popup_search.push(c);
                self.update_list_popup_filter();
            }
            KeyCode::Backspace => {
                self.list_popup_search.pop();
                self.update_list_popup_filter();
            }
            _ => {}
        }
        Ok(())
    }

    fn update_model_chooser_filter(&mut self) {
        let search = self.model_chooser_search.to_ascii_lowercase();
        if search.is_empty() {
            self.model_chooser_filtered_indices = (0..self.model_choices.len()).collect();
        } else {
            self.model_chooser_filtered_indices = self
                .model_choices
                .iter()
                .enumerate()
                .filter(|(_, choice)| {
                    choice.model_id.to_ascii_lowercase().contains(&search)
                        || choice.provider_name.to_ascii_lowercase().contains(&search)
                        || choice.provider_id.to_ascii_lowercase().contains(&search)
                        || choice
                            .provider_type_id
                            .to_ascii_lowercase()
                            .contains(&search)
                })
                .map(|(index, _)| index)
                .collect();
        }
        if self.selected_model_choice_index >= self.model_chooser_filtered_indices.len() {
            self.selected_model_choice_index =
                self.model_chooser_filtered_indices.len().saturating_sub(1);
        }
    }

    async fn load_model_choices(&mut self) -> Result<Vec<ModelChoiceItem>, String> {
        let binding = self
            .core
            .preferences_functional_config_manager()
            .getModelBindingForFunction(FunctionType::CHAT)
            .await
            .map_err(|error| error.to_string())?;
        let choices = self
            .core
            .preferences_model_config_manager()
            .getAllModelSummaries()
            .await
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|summary| ModelChoiceItem {
                selected: binding.providerId == summary.providerId
                    && binding.modelId == summary.modelId,
                provider_id: summary.providerId,
                model_id: summary.modelId,
                provider_name: summary.providerName,
                provider_type_id: summary.providerTypeId,
            })
            .collect::<Vec<_>>();
        Ok(choices)
    }

    async fn apply_selected_model_choice(&mut self) -> Result<(), String> {
        if self.model_chooser_filtered_indices.is_empty() {
            return Ok(());
        }
        let original_index = self.model_chooser_filtered_indices[self.selected_model_choice_index];
        let choice = self
            .model_choices
            .get(original_index)
            .cloned()
            .ok_or_else(|| "no selected model".to_string())?;
        self.apply_chat_model_choice(&choice).await?;
        self.show_model_chooser = false;
        self.model_list_mode = false;
        self.model_chooser_search.clear();
        self.model_chooser_filtered_indices.clear();
        self.focus = FocusArea::Input;
        Ok(())
    }

    async fn use_chat_model(&mut self, args: &[String]) -> Result<(), String> {
        let provider_id = match args.first() {
            Some(value) if !value.trim().is_empty() => value.trim().to_string(),
            _ => {
                self.status_message = self.text().model_use_usage().to_string();
                return Ok(());
            }
        };
        let model_id = match args.get(1) {
            Some(value) if !value.trim().is_empty() => value.trim().to_string(),
            _ => {
                self.status_message = self.text().model_use_usage().to_string();
                return Ok(());
            }
        };
        let config = self
            .core
            .preferences_model_config_manager()
            .getResolvedModelConfig(&provider_id, &model_id)
            .await
            .map_err(|error| error.to_string())?;
        let choice = ModelChoiceItem {
            provider_id,
            model_id,
            provider_name: config.providerName,
            provider_type_id: config.apiProviderTypeId,
            selected: true,
        };
        self.apply_chat_model_choice(&choice).await?;
        Ok(())
    }

    async fn apply_chat_model_choice(&mut self, choice: &ModelChoiceItem) -> Result<(), String> {
        self.core
            .preferences_functional_config_manager()
            .setModelForFunction(
                FunctionType::CHAT,
                choice.provider_id.clone(),
                choice.model_id.clone(),
            )
            .await
            .map_err(|error| error.to_string())?;
        self.set_transient_status_message(self.text().chat_model_status(
            &choice.provider_id,
            &choice.provider_name,
            &choice.model_id,
        ));
        self.refresh_context_usage_label().await;
        Ok(())
    }

    async fn create_new_chat(&mut self, shell_args: ShellArgs) -> Result<(), String> {
        if self.current_chat_is_loading() {
            self.status_message = self.text().wait_for_current_request().to_string();
            return Ok(());
        }

        self.core
            .chat_runtime_holder_main()
            .createNewChat(
                shell_args.characterCardName,
                shell_args.group,
                true,
                true,
                shell_args.characterGroupId,
            )
            .await
            .map_err(|error| error.to_string())?;
        self.refresh_core_snapshot().await?;
        let chat_id = self.current_chat_id()?;
        self.follow_transcript = true;
        self.refresh_chats().await;
        self.select_chat_by_id(&chat_id);
        self.status_message = self.text().new_chat().to_string();
        Ok(())
    }

    async fn toggle_chat_list(&mut self) {
        self.show_chat_list = !self.show_chat_list;
        if self.show_chat_list {
            self.focus = FocusArea::Chats;
            self.refresh_chats().await;
            if let Ok(chat_id) = self.current_chat_id() {
                self.select_chat_by_id(&chat_id);
            }
            self.status_message = self.text().chat_list_shown().to_string();
        } else {
            self.focus = FocusArea::Input;
            self.status_message = self.text().chat_list_hidden().to_string();
        }
    }

    async fn resume_previous_chat(&mut self) -> Result<(), String> {
        if self.current_chat_is_loading() {
            self.status_message = self.text().wait_for_current_request().to_string();
            return Ok(());
        }

        self.refresh_chats().await;
        let current_chat_id = self.current_chat_id()?;
        let target = self
            .chats
            .iter()
            .filter(|chat| chat.id != current_chat_id)
            .max_by(|left, right| {
                left.updated_at
                    .cmp(&right.updated_at)
                    .then_with(|| right.display_order.cmp(&left.display_order))
            })
            .cloned();
        let Some(target) = target else {
            self.status_message = self.text().no_previous_chat().to_string();
            return Ok(());
        };

        self.core
            .chat_runtime_holder_main()
            .switchChat(target.id.clone())
            .await
            .map_err(|error| error.to_string())?;
        self.refresh_core_snapshot().await?;
        self.follow_transcript = true;
        self.select_chat_by_id(&target.id);
        self.status_message = self.text().resumed_chat(&target.title);
        Ok(())
    }

    async fn switch_to_chat(&mut self, chat_id: String) -> Result<(), String> {
        if self.current_chat_is_loading() {
            self.status_message = self.text().wait_for_current_request().to_string();
            return Ok(());
        }

        self.refresh_chats().await;
        let exists = self.chats.iter().any(|chat| chat.id == chat_id);
        if !exists {
            return Err(format!("chat not found: {chat_id}"));
        }
        self.core
            .chat_runtime_holder_main()
            .switchChat(chat_id.clone())
            .await
            .map_err(|error| error.to_string())?;
        self.refresh_core_snapshot().await?;
        self.follow_transcript = true;
        self.select_chat_by_id(&chat_id);
        self.status_message = self.text().switched_chat().to_string();
        Ok(())
    }

    pub(super) async fn refresh_chats(&mut self) {
        let current_chat_id = self.current_chat_id().ok();
        if let Ok(chat_histories) = self
            .core
            .chat_runtime_holder_main()
            .chatHistoriesFlowSnapshot()
            .await
        {
            self.chats = chat_histories_to_list(chat_histories);
        }
        if let Some(chat_id) = current_chat_id {
            self.select_chat_by_id(&chat_id);
        } else if self.selected_chat_index >= self.chats.len() {
            self.selected_chat_index = self.chats.len().saturating_sub(1);
        }
    }

    pub(super) fn select_chat_by_id(&mut self, chat_id: &str) {
        if let Some(index) = self.chats.iter().position(|item| item.id == chat_id) {
            self.selected_chat_index = index;
        }
    }

    async fn refresh_core_snapshot(&mut self) -> Result<(), String> {
        self.current_chat_id_cache = self
            .core
            .chat_runtime_holder_main()
            .currentChatIdFlowSnapshot()
            .await
            .map_err(|error| error.to_string())?;
        self.refresh_current_messages_for_cached_chat().await?;
        self.refresh_current_chat_state_for_cached_chat().await?;
        self.current_window_size_cache = self
            .core
            .chat_runtime_holder_main()
            .currentWindowSizeFlowSnapshot()
            .await
            .map_err(|error| error.to_string())?;
        self.route_permission_error = None;
        Ok(())
    }

    /// Refreshes the current chat state cache and its chat-scoped watch.
    async fn refresh_current_chat_state_for_cached_chat(&mut self) -> Result<(), String> {
        let Some(chat_id) = self.current_chat_id_cache.clone() else {
            self.core.clearMainChatStateWatch();
            self.current_chat_is_loading_cache = false;
            self.current_chat_input_processing_state_cache = InputProcessingState::Idle;
            return Ok(());
        };
        self.core
            .watchMainChatStateFlow(chat_id.clone())
            .await
            .map_err(|error| error.to_string())?;
        let state = self
            .core
            .chat_runtime_holder_main()
            .chatStateFlowSnapshot(chat_id)
            .await
            .map_err(|error| error.to_string())?;
        self.apply_chat_state(state);
        Ok(())
    }

    /// Refreshes the current message cache and its chat-scoped watch.
    async fn refresh_current_messages_for_cached_chat(&mut self) -> Result<(), String> {
        let Some(chat_id) = self.current_chat_id_cache.clone() else {
            self.core.clearMainChatMessagesWatch();
            self.current_messages_cache.clear();
            self.content_stream_states.clear();
            return Ok(());
        };
        self.core
            .watchMainChatMessagesFlow(chat_id.clone())
            .await
            .map_err(|error| error.to_string())?;
        self.current_messages_cache = self
            .core
            .chat_runtime_holder_main()
            .chatMessagesFlowSnapshot(chat_id)
            .await
            .map_err(|error| error.to_string())?;
        self.apply_existing_content_stream_parts_to_cache();
        self.sync_content_stream_watches().await?;
        Ok(())
    }

    /// Applies queued Core events and refreshes chat-scoped message state when selection changes.
    async fn apply_pushed_events(&mut self) -> Result<(), String> {
        self.apply_startup_install_events();
        self.apply_full_update_download_events();
        let mut should_refresh_messages = false;
        let mut should_sync_content_streams = false;
        let drainedEvents = self.core.drainEvents();
        for event in drainedEvents {
            match event.propertyName.as_str() {
                "currentChatIdFlow" => {
                    if let Some(value) = self.core.decodeStateFlowEvent::<Option<String>>(&event)? {
                        if self.current_chat_id_cache != value {
                            self.current_chat_id_cache = value;
                            should_refresh_messages = true;
                        }
                    }
                }
                "chatMessagesFlow" => {
                    if self.core.isActiveMainChatMessagesEvent(&event) {
                        if let Some(value) =
                            self.core.decodeStateFlowEvent::<Vec<ChatMessage>>(&event)?
                        {
                            let streamIds = value
                                .iter()
                                .filter_map(|message| {
                                    message
                                        .contentStream
                                        .as_ref()
                                        .map(|stream| stream.descriptor.streamId.clone())
                                })
                                .collect::<Vec<_>>();
                            let last = value.last().map(|message| {
                                format!(
                                    "ts={} sender={} chars={} stream={}",
                                    message.timestamp,
                                    message.sender,
                                    message.displayText().chars().count(),
                                    message
                                        .contentStream
                                        .as_ref()
                                        .map(|stream| stream.descriptor.streamId.as_str())
                                        .unwrap_or("none")
                                )
                            });
                            AppLogger::trace(
                                "TuiStreamTrace",
                                &format!(
                                    "messages_watch.applied requestId={} eventKind={:?} messages={} streams={} last={}",
                                    event.requestId.as_ref().map(|id| id.0.as_str()).unwrap_or("none"),
                                    event.kind,
                                    value.len(),
                                    streamIds.join(","),
                                    last.as_deref().unwrap_or("none")
                                ),
                            );
                            self.current_messages_cache = value;
                            self.apply_existing_content_stream_parts_to_cache();
                            should_sync_content_streams = true;
                        }
                    }
                }
                "chatStateFlow" => {
                    if self.core.isActiveMainChatStateEvent(&event) {
                        if let Some(value) = self.core.decodeStateFlowEvent::<ChatState>(&event)? {
                            self.apply_chat_state(value);
                        }
                    }
                }
                "chatHistoriesFlow" => {
                    if let Some(value) = self.core.decodeStateFlowEvent::<Vec<ChatHistory>>(&event)? {
                        self.chats = chat_histories_to_list(value);
                        if let Some(chat_id) = self.current_chat_id_cache.clone() {
                            self.select_chat_by_id(&chat_id);
                        }
                    }
                }
                "currentWindowSizeFlow" => {
                    if let Some(value) = self.core.decodeStateFlowEvent::<i64>(&event)? {
                        self.current_window_size_cache = value;
                    }
                }
                _ => {
                    if let Some(info) = self.core.mainChatContentStreamEventInfo(&event) {
                        self.apply_content_stream_event(info, event)?;
                    }
                }
            }
        }
        if should_refresh_messages {
            self.refresh_current_messages_for_cached_chat().await?;
            should_sync_content_streams = false;
        }
        if should_sync_content_streams {
            self.sync_content_stream_watches().await?;
        }
        Ok(())
    }

    /// Synchronizes active embedded content streams with the current TUI message cache.
    async fn sync_content_stream_watches(&mut self) -> Result<(), String> {
        self.core
            .syncMainChatContentStreams(&self.current_messages_cache)
            .await
            .map_err(|error| error.to_string())?;
        let activeStreamIds = self.core.activeMainChatContentStreamIds();
        self.content_stream_states
            .retain(|streamId, _| activeStreamIds.contains(streamId));
        self.apply_existing_content_stream_parts_to_cache();
        Ok(())
    }

    /// Applies one embedded Markdown stream event to the matching cached chat message.
    fn apply_content_stream_event(
        &mut self,
        info: TuiContentStreamEventInfo,
        event: operit_link::CoreEvent,
    ) -> Result<(), String> {
        let markdown: MarkdownStreamEvent =
            operit_link::fromCoreValue(event.value).map_err(|error| error.to_string())?;
        let eventKind = event.kind.clone();
        let streamId = info.streamId.clone();
        let messageTimestamp = info.messageTimestamp;
        let traceFingerprint = if tui_sync_trace_enabled() {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            std::hash::Hash::hash(&serde_json::to_vec(&markdown).map_err(|error| error.to_string())?, &mut hash);
            Some(std::hash::Hasher::finish(&hash))
        } else {
            None
        };
        let state = self
            .content_stream_states
            .entry(streamId.clone())
            .or_insert_with(TuiMessageContentStreamState::new);
        state.eventCount += 1;
        let eventSequence = state.eventCount;
        let parts = match markdown.eventType.as_str() {
            "reset" => {
                state.reset();
                None
            }
            "chunk" => {
                let chunk = markdown
                    .value
                    .ok_or_else(|| "TUI content stream chunk event is missing value".to_string())?;
                Some(state.pushChunk(&chunk)?)
            }
            "savepoint" => {
                let id = markdown.id.ok_or_else(|| {
                    "TUI content stream savepoint event is missing id".to_string()
                })?;
                state.savepoint(&id);
                None
            }
            "rollback" => {
                let id = markdown
                    .id
                    .ok_or_else(|| "TUI content stream rollback event is missing id".to_string())?;
                Some(state.rollback(&id)?)
            }
            "completed" => Some(state.finish()?),
            _ => None,
        };
        let partCount = parts.as_ref().map(Vec::len);
        if let Some(parts) = parts {
            self.update_cached_content_stream_message(messageTimestamp, parts);
        }
        if let Some(fingerprint) = traceFingerprint {
            AppLogger::trace("TuiSyncMeasure", &format!(
                "event.applied streamId={} sequence={} fingerprint={:016x} eventType={}",
                streamId, eventSequence, fingerprint, markdown.eventType
            ));
        }
        if is_content_stream_boundary_event(&markdown.eventType) {
            AppLogger::trace(
                "TuiStreamTrace",
                &format!(
                    "content_event.applied streamId={} messageTimestamp={} kind={:?} eventType={} resultingParts={} cachedMessages={}",
                    streamId,
                    messageTimestamp,
                    eventKind,
                    markdown.eventType,
                    partCount.map(|count| count.to_string()).unwrap_or_else(|| "unchanged".to_string()),
                    self.current_messages_cache.len()
                ),
            );
        }
        Ok(())
    }

    /// Restores active embedded stream projections after message Flow snapshots arrive.
    fn apply_existing_content_stream_parts_to_cache(&mut self) {
        for message in &mut self.current_messages_cache {
            let Some(stream) = message.contentStream.as_ref() else {
                continue;
            };
            let Some(state) = self.content_stream_states.get(&stream.descriptor.streamId) else {
                continue;
            };
            message.parts = state.currentParts();
        }
    }

    /// Replaces the semantic parts for one cached message updated by an embedded stream.
    fn update_cached_content_stream_message(
        &mut self,
        messageTimestamp: i64,
        parts: Vec<MessagePart>,
    ) {
        if let Some(message) = self
            .current_messages_cache
            .iter_mut()
            .find(|message| message.timestamp == messageTimestamp)
        {
            message.parts = parts;
        }
    }

    /// Applies routed runtime state for the selected chat.
    fn apply_chat_state(&mut self, state: ChatState) {
        self.current_chat_is_loading_cache = state.isLoading;
        self.current_chat_input_processing_state_cache = state.inputProcessingState;
        self.current_tool_permission_requests = state.toolPermissionRequests;
    }

    fn apply_startup_install_events(&mut self) {
        let text = self.text();
        let Some(prompt) = self.startup_install_prompt.as_mut() else {
            return;
        };
        let Some(rx) = prompt.progress_rx.as_ref() else {
            return;
        };
        while let Ok(message) = rx.try_recv() {
            match message {
                StartupInstallMessage::Progress(progress) => {
                    let message = match progress {
                        CliInstallProgress::CopyOperit => text.install_command_copying_operit(),
                        CliInstallProgress::CopyOperit2 => text.install_command_copying_operit2(),
                        CliInstallProgress::UpdatePath => text.install_command_updating_path(),
                        CliInstallProgress::Complete => text.install_command_installed(),
                    }
                    .to_string();
                    prompt.state = StartupInstallState::Installing {
                        message: message.clone(),
                    };
                    self.status_message = message;
                }
                StartupInstallMessage::Complete(Ok(())) => {
                    prompt.state = StartupInstallState::Complete;
                    prompt.progress_rx = None;
                    self.status_message = text.install_command_installed().to_string();
                    break;
                }
                StartupInstallMessage::Complete(Err(message)) => {
                    prompt.state = StartupInstallState::Error { message };
                    prompt.progress_rx = None;
                    self.status_message = text.install_command_failed().to_string();
                    break;
                }
            }
        }
    }

    fn apply_full_update_download_events(&mut self) {
        let text = self.text();
        let Some(prompt) = self.startup_update_prompt.as_mut() else {
            return;
        };
        let Some(rx) = prompt.progress_rx.as_ref() else {
            return;
        };
        while let Ok(message) = rx.try_recv() {
            match message {
                FullUpdateDownloadMessage::Progress(event) => match event {
                    FullUpdateProgressEvent::StageChanged { stage, message } => {
                        if stage == FullUpdateStage::Ready {
                            continue;
                        }
                        let current = match prompt.download_state.clone() {
                            FullUpdateDownloadState::Downloading {
                                read_bytes,
                                total_bytes,
                                speed_bytes_per_sec,
                                ..
                            } => (read_bytes, total_bytes, speed_bytes_per_sec),
                            _ => (0, 0, 0),
                        };
                        let message = if current.1 == 0 {
                            text.preparing_download().to_string()
                        } else {
                            message
                        };
                        prompt.download_state = FullUpdateDownloadState::Downloading {
                            stage,
                            message,
                            read_bytes: current.0,
                            total_bytes: current.1,
                            speed_bytes_per_sec: current.2,
                        };
                    }
                    FullUpdateProgressEvent::DownloadProgress {
                        readBytes,
                        totalBytes,
                        speedBytesPerSec,
                    } => {
                        let current = match prompt.download_state.clone() {
                            FullUpdateDownloadState::Downloading { stage, message, .. } => {
                                (stage, message)
                            }
                            _ => (
                                FullUpdateStage::DownloadingPackage,
                                text.full_update_download_message().to_string(),
                            ),
                        };
                        prompt.download_state = FullUpdateDownloadState::Downloading {
                            stage: current.0,
                            message: current.1,
                            read_bytes: readBytes,
                            total_bytes: totalBytes,
                            speed_bytes_per_sec: speedBytesPerSec,
                        };
                    }
                },
                FullUpdateDownloadMessage::Complete(Ok((package_path, install_status))) => {
                    prompt.download_state = FullUpdateDownloadState::Complete {
                        package_path,
                        install_status,
                    };
                    prompt.progress_rx = None;
                    self.status_message = text.full_update_ready().to_string();
                    if install_status == Some(crate::cli::DownloadedUpdateInstallStatus::Scheduled)
                    {
                        self.should_quit = true;
                    }
                    break;
                }
                FullUpdateDownloadMessage::Complete(Err(message)) => {
                    prompt.download_state = FullUpdateDownloadState::Error { message };
                    prompt.progress_rx = None;
                    self.status_message = text.full_update_failed().to_string();
                    break;
                }
            }
        }
    }

    pub(super) fn current_chat_id(&mut self) -> Result<String, String> {
        self.current_chat_id_cache
            .clone()
            .ok_or_else(|| "no active chat in tui".to_string())
    }

    pub(super) fn current_messages(&mut self) -> Vec<ChatMessage> {
        self.current_messages_cache.clone()
    }

    pub(super) fn current_chat_is_loading(&mut self) -> bool {
        self.last_current_chat_loading || self.raw_current_chat_is_loading()
    }

    fn raw_current_chat_is_loading(&mut self) -> bool {
        self.current_chat_is_loading_cache
    }

    pub(super) fn current_chat_input_processing_state(&mut self) -> InputProcessingState {
        self.current_chat_input_processing_state_cache.clone()
    }

    async fn refresh_runtime_status_if_due(&mut self) {
        let now = Instant::now();
        let transition_pending = self.awaiting_runtime_loading
            || self.last_current_chat_loading != self.raw_current_chat_is_loading();
        let refresh_due = self
            .last_runtime_status_refresh_at
            .map(|last| now.saturating_duration_since(last) >= RUNTIME_STATUS_REFRESH_INTERVAL)
            .unwrap_or(true);
        if transition_pending || refresh_due {
            self.last_runtime_status_refresh_at = Some(now);
            self.refresh_runtime_status().await;
        }
    }

    async fn refresh_runtime_status(&mut self) {
        if let Some(error) = self.route_permission_error.clone() {
            self.set_status_message(error);
            return;
        }
        self.refresh_context_usage_label().await;
        let is_loading = self.raw_current_chat_is_loading();
        let state = self.current_chat_input_processing_state();
        if self.awaiting_runtime_loading && !is_loading {
            match &state {
                InputProcessingState::Error { message } => {
                    self.awaiting_runtime_loading = false;
                    self.last_current_chat_loading = false;
                    self.set_runtime_status_message(message.clone(), &state, is_loading);
                }
                InputProcessingState::Idle | InputProcessingState::Completed => {
                    self.awaiting_runtime_loading = false;
                    self.last_current_chat_loading = false;
                    self.follow_transcript = true;
                    self.refresh_chats().await;
                    // Model binding now lives in the persistent right footer
                    // segment; reset the left status for command feedback.
                    match self.current_chat_model_status_label().await {
                        Ok(_) => self.status_message.clear(),
                        Err(error) => self.set_status_message(error),
                    }
                }
                _ => {
                    self.follow_transcript = true;
                    self.set_runtime_status_message(
                        self.text().connecting_ai_service().to_string(),
                        &state,
                        is_loading,
                    );
                }
            }
            return;
        }
        if is_loading {
            self.awaiting_runtime_loading = false;
            self.follow_transcript = true;
            let status = match &state {
                InputProcessingState::Idle => match self.current_chat_model_status_label().await {
                    Ok(label) => label,
                    Err(error) => error,
                },
                InputProcessingState::Error { message } => message.clone(),
                _ => self.input_processing_status_text(&state),
            };
            self.set_runtime_status_message(status, &state, is_loading);
        } else if self.last_current_chat_loading {
            self.awaiting_runtime_loading = false;
            self.follow_transcript = true;
            self.refresh_chats().await;
            // Loading just finished; reset the status line for command
            // feedback instead of rewriting the model binding label.
            match self.current_chat_model_status_label().await {
                Ok(_) => self.status_message.clear(),
                Err(error) => self.set_status_message(error),
            }
        }
        self.last_current_chat_loading = is_loading;
    }

    fn set_status_message(&mut self, message: String) {
        self.status_message = message;
    }

    fn set_runtime_status_message(
        &mut self,
        message: String,
        _state: &InputProcessingState,
        _is_loading: bool,
    ) {
        self.status_message = message;
    }

    async fn refresh_context_usage_label(&mut self) {
        match self.current_context_usage_label().await {
            Ok(label) => {
                self.context_usage_label = label;
            }
            Err(_) => {
                self.context_usage_label.clear();
            }
        }
    }

    /// Persistent right footer segment: chat model binding plus context
    /// usage. The model label used to live in the left status segment,
    /// which the runtime refresher rewrote periodically and clobbered
    /// command feedback.
    async fn current_context_usage_label(&mut self) -> Result<String, String> {
        let model_label = self.current_chat_model_status_label().await?;
        let usage = self.current_context_usage_text().await?;
        Ok(format!("{model_label} | {usage}"))
    }

    async fn current_context_usage_text(&mut self) -> Result<String, String> {
        let model_ref = self.editable_chat_model_ref().await?;
        let config = self
            .core
            .preferences_model_config_manager()
            .getResolvedModelConfig(&model_ref.provider_id, &model_ref.model_id)
            .await
            .map_err(|error| error.to_string())?;
        let effective_context_length = config.context.maxContextLength;
        let max_tokens = (effective_context_length * 1024.0) as i64;
        let current_window_size = self.current_window_size_cache;
        if max_tokens <= 0 {
            return Ok(self
                .text()
                .context_usage_raw(current_window_size, max_tokens));
        }
        let usage_percent =
            ((current_window_size.max(0) as f64 / max_tokens as f64) * 100.0).round() as i32;
        Ok(self
            .text()
            .context_usage(usage_percent, current_window_size, max_tokens))
    }

    async fn handle_character_command(&mut self, args: &[String]) -> Result<(), String> {
        match args.first().map(String::as_str) {
            None | Some("choose") => {
                let cards = self
                    .core
                    .preferences_character_card_manager()
                    .getAllCharacterCards()
                    .await
                    .map_err(|error| error.to_string())?;
                if cards.is_empty() {
                    self.status_message = self.text().character_none().to_string();
                } else {
                    let items = cards.into_iter().map(|c| c.name).collect::<Vec<_>>();
                    self.open_list_popup(self.text().character_title().to_string(), items);
                }
            }
            Some(other) => {
                self.status_message = self.text().unknown_command(&format!("character {other}"));
            }
        }
        Ok(())
    }

    async fn handle_group_command(&mut self, args: &[String]) -> Result<(), String> {
        match args.first().map(String::as_str) {
            None | Some("choose") => {
                let groups = self
                    .core
                    .preferences_character_group_card_manager()
                    .getAllCharacterGroupCards()
                    .await
                    .map_err(|error| error.to_string())?;
                if groups.is_empty() {
                    self.status_message = self.text().group_none().to_string();
                } else {
                    let items = groups.into_iter().map(|g| g.name).collect::<Vec<_>>();
                    self.open_list_popup(self.text().group_title().to_string(), items);
                }
            }
            Some(other) => {
                self.status_message = self.text().unknown_command(&format!("group {other}"));
            }
        }
        Ok(())
    }

    /// Handles installed-skill commands through the application skill repository.
    async fn handle_skill_command(&mut self, args: &[String]) -> Result<(), String> {
        if args.first().map(String::as_str) == Some("toggle") {
            let name = args
                .get(1)
                .ok_or_else(|| self.text().usage_skill_toggle().to_string())?;
            self.status_message = self.text().skill_toggled(name, "toggled");
            return Ok(());
        }
        if args.first().is_some() {
            self.status_message = self.text().unknown_command(&format!("skill {}", args[0]));
            return Ok(());
        }
        let skills = self
            .core
            .application()
            .skillRepository()
            .getAvailableSkillPackages()
            .await
            .map_err(|error| error.to_string())?;
        if skills.is_empty() {
            self.status_message = self.text().skill_none().to_string();
        } else {
            let items = skills.into_keys().collect::<Vec<_>>();
            self.open_list_popup(self.text().skill_title().to_string(), items);
        }
        Ok(())
    }

    async fn handle_package_command(&mut self, args: &[String]) -> Result<(), String> {
        if args.first().map(String::as_str) == Some("toggle") {
            let name = args
                .get(1)
                .ok_or_else(|| self.text().usage_package_toggle().to_string())?;
            self.status_message = self.text().package_toggled(name, "toggled");
            return Ok(());
        }
        if args.first().is_some() {
            self.status_message = self.text().unknown_command(&format!("package {}", args[0]));
            return Ok(());
        }
        let names = self
            .core
            .application()
            .active_package_names()
            .await
            .map_err(|error| error.to_string())?;
        if names.is_empty() {
            self.status_message = self.text().package_none().to_string();
        } else {
            self.open_list_popup(self.text().package_title().to_string(), names);
        }
        Ok(())
    }

    async fn handle_plugin_command(&mut self, args: &[String]) -> Result<(), String> {
        if args.first().map(String::as_str) == Some("toggle") {
            let name = args
                .get(1)
                .ok_or_else(|| self.text().usage_plugin_toggle().to_string())?;
            self.status_message = self.text().plugin_toggled(name, "toggled");
            return Ok(());
        }
        if args.first().is_some() {
            self.status_message = self.text().unknown_command(&format!("plugin {}", args[0]));
            return Ok(());
        }
        let plugins = self
            .core
            .permissions_mcp_runtime_mcp_local_server()
            .getAllPluginMetadata()
            .await
            .map_err(|error| error.to_string())?;
        if plugins.is_empty() {
            self.status_message = self.text().plugin_none().to_string();
        } else {
            let items = plugins.into_keys().collect::<Vec<_>>();
            self.open_list_popup(self.text().plugin_title().to_string(), items);
        }
        Ok(())
    }

    async fn handle_mcp_command(&mut self, args: &[String]) -> Result<(), String> {
        if args.first().map(String::as_str) == Some("toggle") {
            let name = args
                .get(1)
                .ok_or_else(|| self.text().usage_mcp_toggle().to_string())?;
            self.status_message = self.text().mcp_toggled(name, "toggled");
            return Ok(());
        }
        if args.first().is_some() {
            self.status_message = self.text().unknown_command(&format!("mcp {}", args[0]));
            return Ok(());
        }
        let servers = self
            .core
            .permissions_mcp_runtime_mcp_local_server()
            .getAllMCPServers()
            .await
            .map_err(|error| error.to_string())?;
        if servers.is_empty() {
            self.status_message = self.text().mcp_none().to_string();
        } else {
            let items = servers.into_keys().collect::<Vec<_>>();
            self.open_list_popup(self.text().mcp_title().to_string(), items);
        }
        Ok(())
    }

    async fn handle_tag_command(&mut self, args: &[String]) -> Result<(), String> {
        if args.first().is_some() {
            self.status_message = self.text().unknown_command(&format!("tag {}", args[0]));
            return Ok(());
        }
        let tags = self
            .core
            .preferences_prompt_tag_manager()
            .getAllTags()
            .await
            .map_err(|error| error.to_string())?;
        if tags.is_empty() {
            self.status_message = self.text().tag_none().to_string();
        } else {
            let items = tags.into_iter().map(|t| t.name).collect::<Vec<_>>();
            self.open_list_popup(self.text().tag_title().to_string(), items);
        }
        Ok(())
    }

    async fn handle_update_command(&mut self) -> Result<(), String> {
        let version = self
            .core
            .application()
            .coreVersion()
            .await
            .map_err(|error| error.to_string())?;
        self.status_message = self.text().update_version(&version);
        Ok(())
    }

    async fn open_config_popup(&mut self) -> Result<(), String> {
        self.show_config_popup = true;
        self.config_ui.state = config::ConfigState::ProviderList;
        self.config_ui.refresh_providers(&mut self.core).await;
        self.config_ui.search.clear();
        self.config_ui.selected_index = 0;
        self.config_ui.update_filter();

        // Fetch current chat binding
        if let Ok(binding) = self
            .core
            .preferences_functional_config_manager()
            .getModelBindingForFunction(FunctionType::CHAT)
            .await
        {
            self.config_ui.chat_provider_id = binding.providerId;
            self.config_ui.chat_model_id = binding.modelId;
        }

        Ok(())
    }

    async fn handle_config_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let text = self.text();
        let closed = self.config_ui.handle_key(key, &mut self.core, text).await?;
        if closed {
            self.show_config_popup = false;
        }
        Ok(())
    }

    fn input_processing_status_text(&self, state: &InputProcessingState) -> String {
        match state {
            InputProcessingState::Processing { message } => self.text().processing_message(message),
            InputProcessingState::Connecting { message } => self.text().processing_message(message),
            InputProcessingState::Receiving { message } => self.text().processing_message(message),
            InputProcessingState::ExecutingTool { toolName } => {
                self.text().executing_tool(toolName.trim())
            }
            InputProcessingState::ToolProgress { message, .. } => {
                self.text().processing_message(message)
            }
            InputProcessingState::ProcessingToolResult { toolName } => {
                self.text().processing_tool_result(toolName.trim())
            }
            InputProcessingState::Summarizing { message } => {
                self.text().processing_message(message)
            }
            InputProcessingState::ExecutingPlan { message } => {
                self.text().processing_message(message)
            }
            InputProcessingState::Idle | InputProcessingState::Completed => String::new(),
            InputProcessingState::Error { message } => message.clone(),
        }
    }
}

/// Returns whether a Markdown stream event should be logged as a lifecycle boundary.
fn is_content_stream_boundary_event(eventType: &str) -> bool {
    matches!(eventType, "reset" | "savepoint" | "rollback" | "completed")
}

/// Enables explicit per-event synchronization measurements without logging message content.
fn tui_sync_trace_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("OPERIT_TUI_SYNC_TRACE").as_deref() == Ok("1"))
}

fn parse_permission_level(value: Option<&str>) -> Result<AiPermissionMode, String> {
    match value {
        Some("allow") | Some("ALLOW") => Ok(AiPermissionMode::Full),
        Some("ask") | Some("ASK") => Ok(AiPermissionMode::WorkspaceWrite),
        Some("forbid") | Some("FORBID") => Ok(AiPermissionMode::ReadOnly),
        _ => Err("expected allow, ask, or forbid".to_string()),
    }
}

fn chat_histories_to_list(chat_histories: Vec<ChatHistory>) -> Vec<ChatListItem> {
    chat_histories
        .into_iter()
        .map(|chat| {
            let title = if chat.title.trim().is_empty() {
                chat.id.clone()
            } else {
                chat.title.clone()
            };
            let mut secondary = short_chat_label(&chat.id);
            let character_card_name = chat.characterCardName.clone().unwrap_or_default();
            if !character_card_name.is_empty() {
                secondary.push_str(" | ");
                secondary.push_str(&character_card_name);
            }
            if let Some(group_id) = chat.characterGroupId.clone() {
                if !group_id.trim().is_empty() {
                    secondary.push_str(" | group=");
                    secondary.push_str(&group_id);
                }
            }
            ChatListItem {
                id: chat.id,
                title,
                secondary,
                updated_at: chat
                    .updatedAt
                    .parse::<i64>()
                    .expect("chat.updatedAt must be epoch millis"),
                display_order: chat.displayOrder,
            }
        })
        .collect()
}

fn build_attachments(paths: &[String]) -> Result<Vec<AttachmentInfo>, String> {
    paths
        .iter()
        .map(|path| build_attachment_info(path))
        .collect()
}

fn strip_attachment_tokens(
    mut message: String,
    attachment_tokens: &[QueuedAttachmentToken],
) -> String {
    for attachment_token in attachment_tokens {
        message = message.replace(&attachment_token.token, " ");
    }
    message.trim().to_string()
}

fn format_context_length(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i32)
    } else {
        format!("{value:.1}")
    }
}

/// Parses the transport spelling shared by TUI startup (`--link-listen`) and
/// `/network pair`.
pub(super) fn parse_peer_transport(value: &str) -> Result<PeerTransport, String> {
    match value {
        "http" => Ok(PeerTransport::Http),
        "ws" => Ok(PeerTransport::WebSocket),
        "tcp" => Ok(PeerTransport::Tcp),
        "serial" => Ok(PeerTransport::Serial),
        "bluetooth" => Ok(PeerTransport::Bluetooth),
        _ => Err(format!(
            "unknown transport: {value}; expected http, ws, tcp, serial or bluetooth"
        )),
    }
}

/// Parses `/network pair <address> <transport> [--token <token>]`; the token
/// flag matches the headless CLI spelling. The node id is left empty like the
/// Flutter manual pairing dialog: the handshake learns the real identity.
fn parse_pair_arguments(
    args: &[String],
) -> Result<(String, PeerTransport, Option<String>), String> {
    const USAGE: &str =
        "usage: network pair <address> <http|ws|tcp|serial|bluetooth> [--token <token>]";
    let [address, transport, tail @ ..] = args else {
        return Err(USAGE.to_string());
    };
    let token = match tail {
        [] => None,
        [flag, token] if flag == "--token" => Some(token.clone()),
        _ => return Err(USAGE.to_string()),
    };
    Ok((address.clone(), parse_peer_transport(transport)?, token))
}

/// Resolves the pairing a `/network pair-confirm|pair-cancel` refers to: an
/// exact pairing id when given, otherwise the only pairing this session
/// started. Several pending pairings require an explicit id.
fn resolve_pending_pairing<'a>(
    pending: &'a [PendingPairing],
    pairing_id: Option<&str>,
) -> Result<&'a PendingPairing, String> {
    match (pairing_id, pending) {
        (Some(pairing_id), _) => pending
            .iter()
            .find(|pending| pending.pairingId == pairing_id)
            .ok_or_else(|| {
                format!("no pending pairing \"{pairing_id}\"; start one with /network pair")
            }),
        (None, [only]) => Ok(only),
        (None, []) => Err("no pending pairing; start one with /network pair".to_string()),
        (None, _) => {
            Err("several pending pairings; name the pairing id from /network pair".to_string())
        }
    }
}

/// Join requests in these states still resolve to a decision; mirrors the
/// runtime's own active-status rule for outgoing refreshes.
fn space_join_is_active(status: &SpaceJoinStatus) -> bool {
    matches!(
        status,
        SpaceJoinStatus::Pending | SpaceJoinStatus::Approving | SpaceJoinStatus::Approved
    )
}

fn join_status_label(status: &SpaceJoinStatus) -> &'static str {
    match status {
        SpaceJoinStatus::Pending => "pending",
        SpaceJoinStatus::Approving => "approving",
        SpaceJoinStatus::Approved => "approved",
        SpaceJoinStatus::Rejected => "rejected",
        SpaceJoinStatus::Cancelled => "cancelled",
        SpaceJoinStatus::Expired => "expired",
        SpaceJoinStatus::Joined => "joined",
    }
}

/// Display label for a paired device; the projection carries the human name
/// in the device-info model slot and may leave it empty.
fn paired_device_label(device_id: &str, peer: &RuntimePairedDevice) -> String {
    if peer.deviceInfo.model.is_empty() {
        device_id.to_string()
    } else {
        peer.deviceInfo.model.clone()
    }
}

fn paired_device_direction(inbound: bool, outbound: bool) -> &'static str {
    match (inbound, outbound) {
        (true, true) => "inbound+outbound",
        (true, false) => "inbound",
        (false, true) => "outbound",
        (false, false) => "no authorization direction",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_model::MessagePart::MessagePartKind;
    use operit_model::MessagePartCodec::MessagePartCodec;
    use operit_node_runtime::RuntimeRemoteLinkService::{
        RuntimeDeviceSpaceIdentity, SpaceJoinStatus,
    };
    use std::collections::BTreeSet;

    /// Builds a device window fixture: self plus two foreign devices, one
    /// restricted, one carrying an identity, and one pending join request.
    fn device_manager_fixture() -> DeviceManagerModal {
        let device = |device_id: &str, online: bool, identity: Option<&str>| {
            RuntimeDeviceSpaceDevice {
                deviceId: device_id.to_string(),
                userName: "alice".to_string(),
                deviceName: device_id.to_string(),
                platform: "linux".to_string(),
                model: String::new(),
                coreVersion: None,
                online,
                currentIdentity: identity.map(|display_name| RuntimeDeviceSpaceIdentity {
                    displayName: display_name.to_string(),
                    capabilities: Vec::new(),
                }),
            }
        };
        DeviceManagerModal {
            topology: RuntimeDeviceSpaceTopology {
                currentDeviceId: "self".to_string(),
                devices: vec![
                    device("self", true, Some("admin")),
                    device("phone", true, Some("user")),
                    device("tablet", false, None),
                ],
                connections: Vec::new(),
            },
            blocked: ["tablet".to_string()].into_iter().collect(),
            requests: vec![SpaceJoinRequest {
                requestId: "req-1".to_string(),
                targetDeviceId: "self".to_string(),
                applicantDeviceId: "applicant".to_string(),
                applicantName: "carol".to_string(),
                spaceName: "Space".to_string(),
                status: SpaceJoinStatus::Pending,
                createdAt: 0,
                expiresAt: 0,
                canApprove: true,
                reviewerDeviceId: None,
                reviewerName: None,
                reviewerHops: None,
                assignmentVersion: 0,
                decisionApprove: None,
            }],
            roles: BTreeMap::new(),
            initialized: true,
            selected: 0,
            mode: DeviceManagerMode::Browsing,
            menu_device_id: None,
            menu_index: 0,
        }
    }

    /// Verifies pending join requests list before devices in the window.
    #[test]
    fn device_manager_rows_list_requests_before_devices() {
        let rows = device_manager_fixture().rows();
        assert_eq!(rows.len(), 4);
        assert!(matches!(rows[0], DeviceManagerRow::Request(_)));
        assert!(rows[1..].iter().all(|row| matches!(row, DeviceManagerRow::Device(_))));
    }

    /// The connection axis is a single state-derived slot: restricted
    /// devices get admit, unrestricted foreign devices get disconnect, and
    /// the local device never offers actions at all - identity changes on
    /// self would drop the capabilities the local UI depends on.
    #[test]
    fn device_manager_menu_derives_connection_action_from_restriction() {
        let modal = device_manager_fixture();
        let actions = modal.menu_actions("tablet");
        assert!(actions.contains(&DeviceManagerAction::Admit));
        assert!(!actions.contains(&DeviceManagerAction::Disconnect));

        let actions = modal.menu_actions("phone");
        assert!(actions.contains(&DeviceManagerAction::Disconnect));
        assert!(!actions.contains(&DeviceManagerAction::Admit));

        assert!(modal.menu_actions("self").is_empty());
    }

    /// Identity actions appear only when they can apply, and only for
    /// foreign devices: assignment needs a defined role, clearing needs an
    /// assigned identity.
    #[test]
    fn device_manager_menu_offers_identity_actions_only_when_applicable() {
        let mut modal = device_manager_fixture();
        assert!(
            !modal
                .menu_actions("phone")
                .contains(&DeviceManagerAction::AssignIdentity)
        );
        assert!(
            modal
                .menu_actions("phone")
                .contains(&DeviceManagerAction::ClearIdentity)
        );
        assert!(
            !modal
                .menu_actions("tablet")
                .contains(&DeviceManagerAction::ClearIdentity)
        );
        modal.roles.insert(
            "role-1".to_string(),
            NetworkControlRole {
                roleId: "role-1".to_string(),
                displayName: "user".to_string(),
                capabilities: BTreeSet::new(),
            },
        );
        assert!(
            modal
                .menu_actions("tablet")
                .contains(&DeviceManagerAction::AssignIdentity)
        );
        // The local row stays read-only even with roles defined.
        assert!(modal.menu_actions("self").is_empty());
    }

    /// Verifies `/new` keyword options translate into the shared shell flags.
    #[test]
    fn new_chat_args_translate_keywords_into_shell_flags() {
        let args = OperitTui::parse_new_chat_args(&[
            "character".to_string(),
            "Alice".to_string(),
            "group-card".to_string(),
            "42".to_string(),
            "group".to_string(),
            "Heroes".to_string(),
        ])
        .expect("keyword options must translate");
        assert_eq!(args.characterCardName.as_deref(), Some("Alice"));
        assert_eq!(args.characterGroupId.as_deref(), Some("42"));
        assert_eq!(args.group.as_deref(), Some("Heroes"));
    }

    /// Verifies legacy `--` spellings still parse and bad input is rejected.
    #[test]
    fn new_chat_args_accept_legacy_flags_and_reject_unknowns() {
        let args = OperitTui::parse_new_chat_args(&["--group".to_string(), "Heroes".to_string()])
            .expect("legacy flag spelling must translate");
        assert_eq!(args.group.as_deref(), Some("Heroes"));

        assert!(OperitTui::parse_new_chat_args(&["chat".to_string(), "x".to_string()]).is_err());
        assert!(OperitTui::parse_new_chat_args(&["character".to_string()]).is_err());
    }

    /// Verifies TUI embedded stream projection preserves split tool markup.
    #[test]
    fn tui_content_stream_state_projects_split_tool_markup() {
        let mut state = TuiMessageContentStreamState::new();

        state
            .pushChunk("before <tool name=\"switch_core\" call_id=\"part-1\"><param name=\"node_id\">core-a")
            .expect("first embedded stream chunk must project");
        let parts = state
            .pushChunk("</param></tool> after")
            .expect("second embedded stream chunk must project");

        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].kind, MessagePartKind::Markdown);
        assert_eq!(parts[0].content, "before ");
        assert_eq!(parts[1].kind, MessagePartKind::ToolCall);
        assert_eq!(parts[1].toolName.as_deref(), Some("switch_core"));
        assert_eq!(
            parts[1].attributes.get("node_id").map(String::as_str),
            Some("core-a")
        );
        assert_eq!(parts[2].kind, MessagePartKind::Markdown);
        assert_eq!(parts[2].content, " after");
    }

    /// Verifies TUI embedded stream projection can restore a provider revision.
    #[test]
    fn tui_content_stream_state_projects_revision_rollback() {
        let mut state = TuiMessageContentStreamState::new();

        state
            .pushChunk("stable ")
            .expect("initial embedded stream chunk must project");
        state.savepoint("retry");
        state
            .pushChunk("discarded")
            .expect("discarded embedded stream chunk must project");
        let parts = state
            .rollback("retry")
            .expect("embedded stream rollback must project");
        assert_eq!(MessagePartCodec::visibleText(&parts), "stable ");

        let parts = state
            .pushChunk("final")
            .expect("final embedded stream chunk must project");
        assert_eq!(MessagePartCodec::visibleText(&parts), "stable final");
    }

    /// Verifies TUI embedded stream projection keeps thinking content out of visible text.
    #[test]
    fn tui_content_stream_state_hides_thinking_markup() {
        let mut state = TuiMessageContentStreamState::new();

        state
            .pushChunk("<think>internal")
            .expect("thinking opening chunk must project");
        let parts = state
            .pushChunk("</think>visible")
            .expect("thinking closing chunk must project");

        assert!(parts
            .iter()
            .any(|part| part.kind == MessagePartKind::Thinking));
        assert_eq!(MessagePartCodec::visibleText(&parts), "visible");
    }

    fn pending_pairing(pairing_id: &str, display_name: &str) -> PendingPairing {
        PendingPairing {
            pairingId: pairing_id.to_string(),
            peerNodeId: format!("node-{pairing_id}"),
            displayName: display_name.to_string(),
        }
    }

    #[test]
    fn parse_peer_transport_accepts_listener_transports() {
        assert!(matches!(
            parse_peer_transport("http"),
            Ok(PeerTransport::Http)
        ));
        assert!(matches!(
            parse_peer_transport("ws"),
            Ok(PeerTransport::WebSocket)
        ));
        assert!(matches!(
            parse_peer_transport("tcp"),
            Ok(PeerTransport::Tcp)
        ));
        assert!(matches!(
            parse_peer_transport("serial"),
            Ok(PeerTransport::Serial)
        ));
        assert!(matches!(
            parse_peer_transport("bluetooth"),
            Ok(PeerTransport::Bluetooth)
        ));
        assert!(parse_peer_transport("quic").is_err());
    }

    #[test]
    fn parse_pair_arguments_reads_address_transport_and_optional_token() {
        let args = |values: &[&str]| values.iter().map(ToString::to_string).collect::<Vec<_>>();

        let (address, transport, token) = parse_pair_arguments(&args(&["192.168.1.8:4835", "ws"]))
            .expect("pair arguments must parse");
        assert_eq!(address, "192.168.1.8:4835");
        assert!(matches!(transport, PeerTransport::WebSocket));
        assert_eq!(token, None);

        let (address, transport, token) =
            parse_pair_arguments(&args(&["192.168.1.8:4835", "tcp", "--token", "secret"]))
                .expect("pair arguments with token must parse");
        assert_eq!(address, "192.168.1.8:4835");
        assert!(matches!(transport, PeerTransport::Tcp));
        assert_eq!(token.as_deref(), Some("secret"));

        assert!(parse_pair_arguments(&args(&["192.168.1.8:4835"])).is_err());
        assert!(parse_pair_arguments(&args(&["192.168.1.8:4835", "quic"])).is_err());
        assert!(parse_pair_arguments(&args(&["192.168.1.8:4835", "ws", "--token"])).is_err());
    }

    #[test]
    fn resolve_pending_pairing_auto_picks_a_single_session_pairing() {
        let pending = vec![pending_pairing("pair-1", "phone")];

        let resolved = resolve_pending_pairing(&pending, None)
            .expect("a single pending pairing resolves without id");
        assert_eq!(resolved.pairingId, "pair-1");

        let resolved = resolve_pending_pairing(&pending, Some("pair-1"))
            .expect("an exact pairing id resolves");
        assert_eq!(resolved.displayName, "phone");

        assert!(resolve_pending_pairing(&pending, Some("pair-2")).is_err());
        assert!(resolve_pending_pairing(&[], None).is_err());

        let several = vec![
            pending_pairing("pair-1", "phone"),
            pending_pairing("pair-2", "server"),
        ];
        assert!(resolve_pending_pairing(&several, None).is_err());
        let resolved = resolve_pending_pairing(&several, Some("pair-2"))
            .expect("an explicit id resolves among several pending pairings");
        assert_eq!(resolved.displayName, "server");
    }

    #[test]
    fn space_join_active_states_cover_every_decision_pending_status() {
        assert!(space_join_is_active(&SpaceJoinStatus::Pending));
        assert!(space_join_is_active(&SpaceJoinStatus::Approving));
        assert!(space_join_is_active(&SpaceJoinStatus::Approved));
        assert!(!space_join_is_active(&SpaceJoinStatus::Joined));
        assert!(!space_join_is_active(&SpaceJoinStatus::Rejected));
        assert!(!space_join_is_active(&SpaceJoinStatus::Cancelled));
        assert!(!space_join_is_active(&SpaceJoinStatus::Expired));
    }

    #[test]
    fn paired_device_label_falls_back_to_node_id_when_the_name_is_missing() {
        let mut peer = RuntimePairedDevice {
            deviceId: "node-1".to_string(),
            deviceInfo: operit_link::protocol::LinkDeviceInfo {
                platform: String::new(),
                model: "phone".to_string(),
            },
            inbound: true,
            outbound: false,
        };
        assert_eq!(paired_device_label("node-1", &peer), "phone");

        peer.deviceInfo.model = String::new();
        assert_eq!(paired_device_label("node-1", &peer), "node-1");
        assert_eq!(
            paired_device_direction(peer.inbound, peer.outbound),
            "inbound"
        );
    }
}
