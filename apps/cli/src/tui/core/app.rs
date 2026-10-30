use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Stdout};
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

use operit_link::protocol::LinkDeviceInfo;
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_model::ChatHistory::ChatHistory;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatTurnOptions::ChatTurnOptions;
use operit_model::FunctionType::FunctionType;
use operit_model::InputProcessingState::InputProcessingState;
use operit_model::MessagePart::MessagePart;
use operit_model::MessagePartCodec::AssistantMarkupStreamState;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_node_runtime::NodeServices::{
    DiscoveredPeer, PairedPeer, PairingPrompt, PeerTransport, PendingPairing,
};
use operit_node_runtime::PeerStateStore::{PeerHostConfig, PeerHostPortMode};
use operit_node_runtime::RuntimeRemoteLinkService::{
    RuntimeDeviceSpaceDevice, RuntimeDeviceSpaceTopology, RuntimePairedDevice,
    RuntimePairedDeviceStatus, RuntimeRemoteLinkService, SpaceJoinRequest, SpaceJoinStatus,
};
use operit_node_runtime::SpaceDoctor::{diagnose as space_diagnose, SpaceHealthReport};
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
use super::i18n::{TuiLanguage, TuiText, TuiTextKey};
use super::input::InputHistory;
use super::link_proxy_rs::{TuiContentStreamEventInfo, TuiCore};
pub(super) use super::outgoing_joins::space_join_is_active;
use super::peer_discovery::spawn_peer_discovery;
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
    network_capabilities, network_capability_short, network_device_id, network_device_label_by_id,
    network_role_id, network_role_summary, new_network_control_id,
};
use crate::cli::CliInstallProgress;
use crate::tui::NetworkUiEvent;
use crate::{build_attachment_info, parse_shell_args, ChatSendArgs, ShellArgs};
use operit_store::NetworkControlStore::{NetworkControlIdentityAssignment, NetworkControlRole};

const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(16);
const RUNTIME_STATUS_REFRESH_INTERVAL: Duration = Duration::from_millis(250);
/// Space membership is a local read; a slower cadence than the chat status is
/// enough to notice a member joining, leaving, or being removed.
const SPACE_MEMBERSHIP_REFRESH_INTERVAL: Duration = Duration::from_secs(2);
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
    /// Submitted inputs recalled with the arrow keys in the input box.
    pub(super) input_history: InputHistory,
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
    /// Sender for background network results that must not block the loop.
    network_event_sender: mpsc::Sender<NetworkUiEvent>,
    seen_pairing_prompt_ids: BTreeSet<String>,
    seen_join_request_ids: BTreeSet<String>,
    /// Pairings this session started with `/network pair` and has not yet
    /// confirmed or cancelled; lets the follow-up commands resolve the id.
    pub(super) pending_pairings: Vec<PendingPairing>,
    /// The list popup is currently the Y/N confirm for `/network leave`; any
    /// other popup open clears it so a stale confirm cannot fire elsewhere.
    leave_confirm_pending: bool,
    /// The list popup is currently the Y/N confirm for hub unpair; holds the
    /// (device id, label) the confirm will act on.
    unpair_confirm: Option<(String, String)>,
    /// The network hub panel opened by bare `/network`.
    pub(super) network_hub: Option<NetworkHubModal>,
    /// The pairing wizard; opened from the hub, discovery, or `/network pair`.
    pub(super) pair_wizard: Option<PairWizardModal>,
    /// Listener settings opened from the hub (`c`) or `/network listen`.
    pub(super) listener_settings: Option<ListenerSettingsModal>,
    /// Direct-address editor for one paired device.
    pub(super) endpoint_edit: Option<EndpointEditModal>,
    /// Discovery candidates behind the current list popup; Enter on the
    /// popup feeds the selected candidate into the pairing wizard.
    pub(super) discovered_peers: Vec<DiscoveredPeer>,
    discover_select_pending: bool,
    /// A LAN scan is running; the candidate list opens from its result event.
    discover_in_flight: bool,
    /// Last observed local Space identity (id, name) and members; drives the
    /// membership change notice. `None` until the first observation seeds it.
    space_identity: Option<(String, String)>,
    space_members: BTreeSet<String>,
    last_space_membership_check_at: Option<Instant>,
    /// Seeds the seen-id snapshots from the first fetch so a TUI start does
    /// not replay requests that predate the session.
    network_snapshots_seeded: bool,
    pub(super) context_usage_label: String,
    pub(super) transcript_scroll: u16,
    pub(super) transcript_viewport_height: u16,
    pub(super) transcript_max_scroll: u16,
    /// Pins the rendered transcript to its newest line. Only explicit user
    /// intent sets this (sending a turn, switching chats, scrolling back to
    /// the bottom); growing content and periodic status refreshes never do, so
    /// a response can stream in while the reader browses history.
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
    pub(super) diff_viewer: Option<super::diff::DiffViewerModal>,
    pub(super) control_panel: Option<ControlPanelModal>,
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
    Pair,
    UpdateEndpoint,
    Admit,
    Disconnect,
    AssignIdentity,
    ClearIdentity,
    Unpair,
    Remove,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DeviceManagerMode {
    Browsing,
    ActionMenu,
    ConfirmRemove,
    ConfirmUnpair,
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
    /// Devices this node already holds a pairing (edge) for.
    pub(super) paired: BTreeSet<String>,
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
    ///
    /// Pairing leads the menu because it is the only link-layer action: an
    /// admitted member without a local pairing can gain a direct edge without
    /// any Space capability and without joining again.
    pub(super) fn menu_actions(&self, device_id: &str) -> Vec<DeviceManagerAction> {
        let is_self = device_id == self.topology.currentDeviceId;
        if is_self {
            return Vec::new();
        }
        let mut actions = Vec::new();
        if !self.blocked.contains(device_id) && !self.paired.contains(device_id) {
            actions.push(DeviceManagerAction::Pair);
        }
        // Rewriting the dial address only makes sense for a device this node
        // already holds a pairing record for; the runtime rejects it otherwise.
        if self.paired.contains(device_id) {
            actions.push(DeviceManagerAction::UpdateEndpoint);
        }
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
        actions.push(DeviceManagerAction::Unpair);
        actions.push(DeviceManagerAction::Remove);
        actions
    }
}

/// Tabs of the headless control panel opened by `/status`, `/log` and the
/// extension-management slash commands. One panel keeps browse, detail,
/// confirm and text-input modes in a single key-routing surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ControlPanelTab {
    Status,
    Packages,
    Plugins,
    Skills,
    Mcp,
    Logs,
    Identities,
}

impl ControlPanelTab {
    pub(super) const ALL: [Self; 7] = [
        Self::Status,
        Self::Packages,
        Self::Plugins,
        Self::Skills,
        Self::Mcp,
        Self::Logs,
        Self::Identities,
    ];

    pub(super) fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|tab| *tab == self)
            .expect("control panel tab must be listed")
    }
}

/// One actionable row in the control panel.
#[derive(Clone, Debug, Default)]
pub(super) struct ControlPanelRow {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) detail: String,
    pub(super) enabled: Option<bool>,
    pub(super) loadable: bool,
    pub(super) deletable: bool,
    pub(super) toggleable: bool,
    pub(super) showable: bool,
    pub(super) startable: bool,
    pub(super) killable: bool,
    pub(super) editable: bool,
    pub(super) issue: bool,
    pub(super) package_kind: Option<String>,
    pub(super) package_name: Option<String>,
    pub(super) code: Option<String>,
    pub(super) source_path: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ControlPanelMode {
    Browse,
    Detail,
    Confirm,
    ImportInput,
    IdentityNameInput,
    IdentityCapsInput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ControlPanelPending {
    None,
    DeleteRow,
    ClearLog,
    DeleteIdentity,
}

/// State for the headless control panel. Data is loaded when a slash command
/// opens the panel and refreshed after every action, so the panel never
/// mutates stale rows.
pub(super) struct ControlPanelModal {
    pub(super) tab: ControlPanelTab,
    pub(super) rows: Vec<ControlPanelRow>,
    pub(super) selected: usize,
    pub(super) mode: ControlPanelMode,
    pub(super) input_label: String,
    pub(super) input: String,
    pub(super) confirm_label: String,
    pub(super) pending: ControlPanelPending,
    pub(super) pending_row: Option<ControlPanelRow>,
    pub(super) identity_role_id: Option<String>,
    pub(super) identity_name: String,
    pub(super) identity_caps: String,
    pub(super) detail_title: String,
    pub(super) detail_lines: Vec<String>,
    pub(super) detail_scroll: u16,
    pub(super) status: Option<String>,
    pub(super) log_path: String,
    pub(super) log_lines: Vec<String>,
}

impl ControlPanelModal {
    pub(super) fn new(tab: ControlPanelTab) -> Self {
        Self {
            tab,
            rows: Vec::new(),
            selected: 0,
            mode: ControlPanelMode::Browse,
            input_label: String::new(),
            input: String::new(),
            confirm_label: String::new(),
            pending: ControlPanelPending::None,
            pending_row: None,
            identity_role_id: None,
            identity_name: String::new(),
            identity_caps: String::new(),
            detail_title: String::new(),
            detail_lines: Vec::new(),
            detail_scroll: 0,
            status: None,
            log_path: String::new(),
            log_lines: Vec::new(),
        }
    }

    pub(super) fn selected_row(&self) -> Option<&ControlPanelRow> {
        self.rows.get(self.selected)
    }
}

/// Loaded data for one control-panel tab. Status and logs use the detail
/// fields; extension tabs use rows and may still fill a detail title.
struct ControlPanelData {
    rows: Vec<ControlPanelRow>,
    detail_title: String,
    detail_lines: Vec<String>,
    log_path: String,
    log_lines: Vec<String>,
}

impl ControlPanelData {
    fn empty() -> Self {
        Self {
            rows: Vec::new(),
            detail_title: String::new(),
            detail_lines: Vec::new(),
            log_path: String::new(),
            log_lines: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub(super) struct StartupInstallPrompt {
    pub(super) install_selected: bool,
    pub(super) state: StartupInstallState,
    pub(super) progress_rx: Option<mpsc::Receiver<StartupInstallMessage>>,
}

/// One selectable row in the network hub device area: a Space member from
/// the live topology, or a paired device that has not joined this Space.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum NetworkHubRow {
    Member(RuntimeDeviceSpaceDevice),
    Peer {
        deviceId: String,
        label: String,
    },
}

impl NetworkHubRow {
    pub(super) fn id(&self) -> &str {
        match self {
            NetworkHubRow::Member(device) => &device.deviceId,
            NetworkHubRow::Peer { deviceId, .. } => deviceId,
        }
    }
}

/// The `/network` hub panel: a persistent answer to "who am I, what is
/// waiting for me, what do I manage", with flows one key away. Data is a
/// snapshot refreshed on open and on every peer-change signal.
pub(super) struct NetworkHubModal {
    pub(super) topology: RuntimeDeviceSpaceTopology,
    pub(super) spaceName: String,
    pub(super) initialized: bool,
    /// Listener summary ("bind · transports") or None when not listening.
    pub(super) listening: Option<String>,
    pub(super) paired: BTreeMap<String, RuntimePairedDevice>,
    pub(super) prompts: Vec<PairingPrompt>,
    pub(super) outgoingJoins: Vec<SpaceJoinRequest>,
    /// Devices the Space policy restricts (`disconnectedNodeIds`). The hub must
    /// show these as restricted instead of merely offline: they will not come
    /// back on their own, and no other panel exposes the restriction.
    pub(super) blocked: BTreeSet<String>,
    pub(super) selected: usize,
}

/// Focusable row in the listener settings dialog. Text rows edit in place;
/// the remaining rows toggle with Left/Right, Space, or Enter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ListenerField {
    PortMode,
    Host,
    Port,
    Discovery,
    Transports,
    Save,
}

impl ListenerField {
    const ORDER: [ListenerField; 6] = [
        ListenerField::PortMode,
        ListenerField::Host,
        ListenerField::Port,
        ListenerField::Discovery,
        ListenerField::Transports,
        ListenerField::Save,
    ];

    /// Moves focus through the form in render order and wraps around.
    pub(super) fn step(self, forward: bool) -> Self {
        let index = Self::ORDER
            .iter()
            .position(|field| *field == self)
            .unwrap_or(0);
        let next = if forward {
            (index + 1) % Self::ORDER.len()
        } else {
            (index + Self::ORDER.len() - 1) % Self::ORDER.len()
        };
        Self::ORDER[next]
    }
}

/// Node-local listener settings, the TUI counterpart of the desktop
/// connection dialog: transports, LAN discovery, bind address and port mode
/// in one form. Saving restarts the listeners with the new configuration.
pub(super) struct ListenerSettingsModal {
    pub(super) bindHost: String,
    pub(super) bindPort: String,
    pub(super) portMode: PeerHostPortMode,
    pub(super) transports: BTreeSet<PeerTransport>,
    pub(super) supportedTransports: Vec<PeerTransport>,
    pub(super) discoveryEnabled: bool,
    pub(super) discoverySupported: bool,
    pub(super) transportCursor: usize,
    pub(super) field: ListenerField,
    pub(super) error: Option<String>,
    /// Opened from the hub, so closing returns there instead of the chat.
    pub(super) returnToHub: bool,
}

/// Direct-address editor for one paired device: the TUI counterpart of the
/// desktop "update direct address" action offered when a direct link fails.
pub(super) struct EndpointEditModal {
    pub(super) deviceId: String,
    pub(super) label: String,
    pub(super) input: String,
    pub(super) error: Option<String>,
    pub(super) returnToHub: bool,
}

/// Splits a persisted bind address into independently editable host and port
/// fields, keeping bracketed IPv6 hosts verbatim. Bare IPv6 without brackets
/// cannot be split unambiguously and is kept in the host field.
pub(super) fn split_bind_address(address: &str) -> (String, String) {
    let value = address.trim();
    if let Some(rest) = value.strip_prefix('[') {
        if let Some((host, tail)) = rest.split_once(']') {
            if let Some(port) = tail.strip_prefix(':') {
                return (host.to_string(), port.to_string());
            }
        }
    }
    match value.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => (host.to_string(), port.to_string()),
        _ => (value.trim_matches(|ch| ch == '[' || ch == ']').to_string(), String::new()),
    }
}

/// Reassembles the Host-compatible socket address after form validation.
pub(super) fn compose_bind_address(host: &str, port: &str) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

/// Validated listener settings ready to persist and apply.
pub(super) struct ListenerPlan {
    pub(super) bindAddress: String,
    pub(super) transports: Vec<PeerTransport>,
    pub(super) discoveryEnabled: bool,
    pub(super) portMode: PeerHostPortMode,
}

/// Validates the listener form with the same rules as the desktop dialog:
/// fixed mode needs a nonzero port, TCP cannot share the bind port with
/// HTTP/WebSocket, discovery needs at least one transport, and transports
/// this Host does not support are dropped instead of failing the save.
pub(super) fn plan_listener_settings(
    text: TuiText,
    host: &str,
    portText: &str,
    portMode: PeerHostPortMode,
    discoveryEnabled: bool,
    discoverySupported: bool,
    selected: &BTreeSet<PeerTransport>,
    supported: &[PeerTransport],
) -> Result<ListenerPlan, String> {
    let host = host.trim().trim_matches(|ch| ch == '[' || ch == ']');
    let portText = portText.trim();
    // Automatic mode treats an empty field as "let the host choose", which is
    // what the field shows before the first save.
    let portText = if portMode == PeerHostPortMode::Automatic && portText.is_empty() {
        "0"
    } else {
        portText
    };
    let transports = selected
        .iter()
        .copied()
        .filter(|transport| supported.contains(transport))
        .collect::<Vec<_>>();
    let portValid = portText
        .parse::<u16>()
        .map(|port| portMode != PeerHostPortMode::Fixed || port > 0)
        .unwrap_or(false);
    if host.is_empty() {
        return Err(text.network_listener_address_required().to_string());
    }
    if !portValid {
        return Err(text.network_listener_port_required().to_string());
    }
    if transports.contains(&PeerTransport::Tcp)
        && (transports.contains(&PeerTransport::Http)
            || transports.contains(&PeerTransport::WebSocket))
    {
        return Err(text.network_listener_port_conflict().to_string());
    }
    if transports.is_empty() && discoveryEnabled && discoverySupported {
        return Err(text.network_listener_discovery_needs_transport().to_string());
    }
    Ok(ListenerPlan {
        bindAddress: compose_bind_address(host, portText),
        transports,
        discoveryEnabled,
        portMode,
    })
}

/// Stages of the pairing wizard. `Code` and `JoinOffer` are reached
/// automatically: starting a pairing (command or wizard) jumps straight to
/// code entry, and a successful confirmation advances to the join offer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PairStage {
    Address,
    Code,
    JoinOffer,
}

/// Focused input field on the wizard address form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PairField {
    Address,
    Transport,
}

pub(super) struct PairWizardModal {
    pub(super) stage: PairStage,
    pub(super) field: PairField,
    pub(super) address: String,
    pub(super) transportIndex: usize,
    /// Node id the handshake must match, when the wizard targets a known
    /// device; empty means "accept whatever node answers the address".
    pub(super) expectedNode: Option<String>,
    /// Display label of the expected device, shown while dialing.
    pub(super) expectedLabel: Option<String>,
    pub(super) pairing: Option<PendingPairing>,
    pub(super) code: String,
    /// The peer a finished pairing produced; drives the join offer.
    pub(super) peer: Option<PairedPeer>,
    pub(super) error: Option<String>,
}

/// Transports the wizard cycles through, in listener-spelling order.
pub(super) const PAIR_WIZARD_TRANSPORTS: [PeerTransport; 5] = [
    PeerTransport::Http,
    PeerTransport::WebSocket,
    PeerTransport::Tcp,
    PeerTransport::Serial,
    PeerTransport::Bluetooth,
];

/// Listener-spelling label for one transport, shared by the wizard and hub.
pub(super) fn peer_transport_label(transport: &PeerTransport) -> &'static str {
    match transport {
        PeerTransport::Http => "http",
        PeerTransport::WebSocket => "ws",
        PeerTransport::Tcp => "tcp",
        PeerTransport::Serial => "serial",
        PeerTransport::Bluetooth => "bluetooth",
    }
}

/// Comma-separated listener transports in saved order, for status lines.
pub(super) fn listener_transport_summary(transports: &[PeerTransport]) -> String {
    transports
        .iter()
        .map(|transport| peer_transport_label(transport))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Pushes one character into the confirmation-code buffer: digits only,
/// six digits at most, so `Enter` can mean "code is complete".
pub(super) fn pair_code_push(code: &str, ch: char) -> String {
    if !ch.is_ascii_digit() || code.len() >= 6 {
        return code.to_string();
    }
    format!("{code}{ch}")
}

/// Derives the hub device rows: every Space member (self first, matching
/// the topology order), then every paired device that is not a member yet -
/// those are the natural join targets after a fresh pairing.
pub(super) fn network_hub_rows(
    topology: &RuntimeDeviceSpaceTopology,
    paired: &BTreeMap<String, RuntimePairedDevice>,
) -> Vec<NetworkHubRow> {
    let mut rows = topology
        .devices
        .iter()
        .cloned()
        .map(NetworkHubRow::Member)
        .collect::<Vec<_>>();
    let members = topology
        .devices
        .iter()
        .map(|device| device.deviceId.as_str())
        .collect::<BTreeSet<_>>();
    rows.extend(
        paired
            .iter()
            .filter(|(device_id, _)| !members.contains(device_id.as_str()))
            .map(|(device_id, peer)| NetworkHubRow::Peer {
                deviceId: device_id.clone(),
                label: paired_device_label(device_id, peer),
            }),
    );
    rows
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
        network_event_sender: mpsc::Sender<NetworkUiEvent>,
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
            input_history: InputHistory::default(),
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
            network_event_sender,
            seen_pairing_prompt_ids: BTreeSet::new(),
            seen_join_request_ids: BTreeSet::new(),
            pending_pairings: Vec::new(),
            leave_confirm_pending: false,
            unpair_confirm: None,
            network_hub: None,
            pair_wizard: None,
            listener_settings: None,
            endpoint_edit: None,
            discovered_peers: Vec::new(),
            discover_select_pending: false,
            discover_in_flight: false,
            space_identity: None,
            space_members: BTreeSet::new(),
            last_space_membership_check_at: None,
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
            diff_viewer: None,
            control_panel: None,
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
            self.apply_network_events();
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
            // Crossterm polls synchronously and local proxy futures can be
            // immediately ready. Explicitly let the local network tasks run
            // even when an idle frame contains no other yielding await.
            tokio::task::yield_now().await;
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

    /// Applies background network results and opens popups when ids beyond the
    /// previous snapshot appear. The first fetch only seeds the snapshots so
    /// a TUI start does not replay requests that predate the session.
    fn apply_network_events(&mut self) {
        while let Ok(event) = self.network_event_receiver.try_recv() {
            match event {
                NetworkUiEvent::OutgoingJoinSettled(updated) => {
                    let message = match updated.status {
                        SpaceJoinStatus::Joined => format!(
                            "network join approved: joined {} with {}",
                            updated.spaceName, updated.targetDeviceId
                        ),
                        SpaceJoinStatus::Rejected => {
                            format!("network join rejected by {}", updated.targetDeviceId)
                        }
                        _ => format!(
                            "network join {}: {} ({})",
                            join_status_label(self.text(), &updated.status),
                            updated.targetDeviceId,
                            updated.spaceName
                        ),
                    };
                    self.set_transient_status_message(message);
                }
                NetworkUiEvent::Snapshot { prompts, requests } => {
                    self.apply_network_snapshots(prompts, requests);
                }
                NetworkUiEvent::DiscoveryFinished(result) => {
                    self.apply_discovery_result(result);
                }
            }
        }
    }

    /// Applies a finished LAN scan: opens the candidate list, or reports why
    /// there is none. Runs on the terminal loop, so it only touches state.
    fn apply_discovery_result(&mut self, result: Result<Vec<DiscoveredPeer>, String>) {
        self.discover_in_flight = false;
        match result {
            Err(error) => self.status_message = status_error_line(&error),
            Ok(candidates) if candidates.is_empty() => {
                self.set_transient_status_message(self.text().network_discover_none().to_string());
            }
            Ok(candidates) => {
                let text = self.text();
                let items = discovery_popup_items(text, &candidates);
                self.discovered_peers = candidates;
                self.open_list_popup(text.network_discover_title().to_string(), items);
                self.discover_select_pending = true;
            }
        }
    }

    async fn refresh_network_snapshots(&mut self) {
        let prompts = self.networkControl.pairingPrompts().unwrap_or_default();
        let requests = self.networkControl.incomingDeviceSpaceJoins().await.ok();
        self.apply_network_snapshots(prompts, requests);
    }

    /// Applies already-fetched data without waiting for network I/O.
    fn apply_network_snapshots(
        &mut self,
        prompts: Vec<PairingPrompt>,
        requests: Option<Vec<SpaceJoinRequest>>,
    ) {
        let has_new_prompts = prompts
            .iter()
            .any(|prompt| !self.seen_pairing_prompt_ids.contains(&prompt.pairingId));
        // The hub's waiting area shows inbound pairing codes live, so the
        // popup would only fight it for screen space.
        if self.network_snapshots_seeded && has_new_prompts && self.network_hub.is_none() {
            self.open_pairing_prompts_popup(&prompts);
        }
        self.seen_pairing_prompt_ids = prompts
            .iter()
            .map(|prompt| prompt.pairingId.clone())
            .collect();

        // Hub snapshots are local reads, so refresh them even when fetching
        // incoming join requests failed, without blocking the terminal loop.
        if self.network_hub.is_some() {
            self.refresh_network_hub();
        }
        let Some(requests) = requests else {
            self.network_snapshots_seeded = true;
            return;
        };
        let has_new_requests = requests
            .iter()
            .any(|request| !self.seen_join_request_ids.contains(&request.requestId));
        if self.network_snapshots_seeded && has_new_requests {
            self.open_join_decision_modal(requests.clone());
        }
        // Keep an already open queue aligned with this fetch: reassignments
        // bump the assignment version, and a decision on a cached version
        // would be rejected forever.
        self.refresh_join_decision_modal(&requests);
        self.seen_join_request_ids = requests
            .iter()
            .map(|request| request.requestId.clone())
            .collect();
        self.network_snapshots_seeded = true;
        if self.device_manager.is_some() {
            self.apply_device_manager_snapshot(requests);
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

    /// Keeps an open decision queue aligned with the freshest decidable set:
    /// entries are replaced in place (a reviewer reassignment bumps the
    /// assignment version) and entries this device no longer reviews are
    /// dropped, so a retry can never loop against a stale assignment. The
    /// popup closes when nothing is left to decide.
    fn refresh_join_decision_modal(&mut self, fresh: &[SpaceJoinRequest]) {
        let Some(modal) = self.join_decision.as_ref() else {
            return;
        };
        let (requests, selected) = refreshed_decision_queue(&modal.requests, fresh, modal.selected);
        if requests.is_empty() {
            self.join_decision = None;
            return;
        }
        if let Some(modal) = self.join_decision.as_mut() {
            modal.requests = requests;
            modal.selected = selected;
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
            || self.device_manager.is_some()
            || self.listener_settings.is_some()
            || self.endpoint_edit.is_some();
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

        if self.diff_viewer.is_some() {
            self.handle_diff_viewer_key(key)?;
            return Ok(());
        }

        if self.control_panel.is_some() {
            self.handle_control_panel_key(key).await?;
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

        if self.endpoint_edit.is_some() {
            self.handle_endpoint_edit_key(key).await?;
            return Ok(());
        }

        if self.listener_settings.is_some() {
            self.handle_listener_settings_key(key).await?;
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

        if self.pair_wizard.is_some() {
            self.handle_pair_wizard_key(key).await?;
            return Ok(());
        }

        // List popups opened from the hub render above it, so they must also
        // receive keys first; otherwise a hub key would fire under the popup
        // (the hub's discover, audit, policy, token and leave shortcuts).
        if self.show_list_popup && self.discover_select_pending && key.code == KeyCode::Enter {
            self.start_pair_wizard_from_discovery().await;
            return Ok(());
        }

        if self.show_list_popup && self.leave_confirm_pending {
            self.handle_leave_confirm_key(key).await;
            return Ok(());
        }

        if self.show_list_popup && self.unpair_confirm.is_some() {
            self.handle_unpair_confirm_key(key).await;
            return Ok(());
        }

        if self.show_list_popup {
            return self.handle_list_popup_key(key);
        }

        if self.network_hub.is_some() {
            self.handle_network_hub_key(key).await?;
            return Ok(());
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
            (KeyCode::Up, KeyModifiers::CONTROL) => {
                self.scroll_transcript_up(self.terminal_wheel_step());
                return Ok(());
            }
            (KeyCode::Down, KeyModifiers::CONTROL) => {
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
            self.input_history.record(&input);
            self.input.clear();
            self.input_cursor = 0;
            // Slash-command failures (bad arguments, rejected core calls) are
            // status-line material, never reasons to tear down the TUI loop.
            if let Err(error) = self.handle_local_command(&input).await {
                self.status_message = status_error_line(&error);
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
        self.input_history.record(&message);
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
            "diff" => {
                self.open_diff_viewer().await?;
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
            "status" => {
                self.open_control_panel(ControlPanelTab::Status).await;
            }
            "log" => {
                self.open_control_panel(ControlPanelTab::Logs).await;
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

    /// `/new` takes generic `source` and opaque `input` keyword/value pairs;
    /// translate them into the shared shell flags so
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
        const USAGE: &str = "network opens the hub panel; subcommands: <show|bootstrap|audit|doctor|devices|identities|identity|name|listen|endpoint|admit|remove|disconnect|policy|prompts|requests|approve|reject|discover|pair|pair-confirm|pair-cancel|peers|unpair|join|joins|join-cancel|leave>";
        match args.first().map(String::as_str) {
            None if args.is_empty() => {
                self.open_network_hub().await;
            }
            Some("show") if args.len() == 1 => {
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
                self.network_audit_popup().await?;
            }
            Some("doctor") if args.len() == 1 => {
                self.network_doctor_popup()?;
            }
            Some("devices") if args.len() == 1 => {
                self.open_device_manager().await;
            }
            Some("identities") if args.len() == 1 => {
                self.open_control_panel(ControlPanelTab::Identities).await;
            }
            Some("name") if args.len() == 2 => {
                self.rename_local_device(&args[1])?;
                self.status_message = format!("device renamed: {}", args[1]);
            }
            Some("listen") if args.len() == 1 => {
                self.open_listener_settings(false).await;
            }
            Some("listen") if args.len() == 2 && args[1] == "show" => {
                self.network_listener_popup()?;
            }
            Some("endpoint") if (2..=3).contains(&args.len()) => {
                let (device_id, label) = self.resolve_paired_device_argument(&args[1]).await?;
                if args.len() == 3 {
                    self.networkControl
                        .updatePairedDeviceEndpoint(device_id, args[2].clone(), None)
                        .await?;
                    self.status_message = self.text().network_endpoint_saved(&label);
                } else {
                    self.open_endpoint_edit(device_id, label, false);
                }
            }
            Some("identity") if args.len() >= 2 => {
                if args[1] == "list" && args.len() == 2 {
                    self.open_control_panel(ControlPanelTab::Identities).await;
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
                if args[1] == "update" && args.len() >= 3 {
                    let state = self.networkControl.deviceSpaceControl()?;
                    let role_id = network_role_id(&state, &args[2])?;
                    let display_name = args.get(3).cloned().ok_or_else(|| USAGE.to_string())?;
                    self.networkControl
                        .updateDeviceSpaceRole(NetworkControlRole {
                            roleId: role_id,
                            displayName: display_name,
                            capabilities: network_capabilities(args.get(4..).unwrap_or(&[]))?,
                        })?;
                    self.status_message = format!("network identity updated: {}", args[2]);
                    return Ok(());
                }
                if args[1] == "delete" && args.len() == 3 {
                    let state = self.networkControl.deviceSpaceControl()?;
                    let role_id = network_role_id(&state, &args[2])?;
                    self.networkControl.removeDeviceSpaceRole(role_id)?;
                    self.status_message = format!("network identity deleted: {}", args[2]);
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
                self.network_policy_popup()?;
            }
            Some("policy") if args.len() == 4 && args[1] == "set" => {
                self.networkControl
                    .updateDeviceSpacePolicy(args[2].clone(), args[3].clone())?;
                self.status_message = format!("network policy updated: {}", args[2]);
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
                    if args[0] == "approve" {
                        "approved"
                    } else {
                        "rejected"
                    },
                    request.status,
                );
            }
            Some("discover") if args.len() == 1 => {
                self.start_peer_discovery();
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
            Some("leave") if args.len() == 1 => {
                self.network_leave().await?;
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
        self.networkControl
            .removeDeviceSpaceMember(device_id)
            .await?;
        Ok(format!("network member removed: {device_label}"))
    }

    async fn network_disconnect_device(&mut self, device: &str) -> Result<String, String> {
        let topology = self.networkControl.deviceSpaceTopology()?;
        let device_id = network_device_id(&topology, device)?;
        let device_label = network_device_label_by_id(&topology, &device_id)?;
        self.networkControl
            .disconnectDeviceSpaceNode(device_id)
            .await?;
        Ok(self.text().network_devices_disconnected(device_label))
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
        self.networkControl
            .setDeviceSpaceIdentity(NetworkControlIdentityAssignment {
                nodeId: device_id,
                roleId: identity_id,
            })?;
        Ok(format!(
            "network identity set: {device_label} · {identity_label}"
        ))
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
    /// Starts an outbound pairing toward an explicit address. The pairing id
    /// must stay on screen for the confirmation step, so it goes to the popup
    /// and is remembered for id-less `/network pair-confirm <code>`.
    /// `/network pair` funnels into the wizard and dials immediately: the
    /// command keeps working for muscle memory, but its UX is the wizard's
    /// code-entry screen instead of an id to copy around.
    async fn network_pair(&mut self, args: &[String]) -> Result<(), String> {
        let (address, transport) = parse_pair_arguments(args)?;
        let transport_index = PAIR_WIZARD_TRANSPORTS
            .iter()
            .position(|candidate| {
                peer_transport_label(candidate) == peer_transport_label(&transport)
            })
            .unwrap_or(2);
        self.open_pair_wizard(address, transport_index, true, None);
        self.pair_wizard_start().await;
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
        // An admitted member needs only the direct link: the join submission
        // would be rejected as a duplicate membership, so pairing ends here.
        if self.network_peer_is_member(&peer.nodeId) {
            return Ok(self
                .text()
                .network_direct_link_established(&paired_peer_label(&peer)));
        }
        // A finished pairing naturally continues into the join offer: the
        // dialing side holds the wizard, so it is the one asked whether to
        // join the paired device's Space. Joining works from either side.
        self.pair_wizard = Some(PairWizardModal {
            stage: PairStage::JoinOffer,
            field: PairField::Address,
            address: String::new(),
            transportIndex: 2,
            expectedNode: None,
            expectedLabel: None,
            pairing: None,
            code: String::new(),
            peer: Some(peer.clone()),
            error: None,
        });
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
        let mut items = Vec::new();
        for (device_id, peer) in peers {
            let status = self
                .networkControl
                .pairedDeviceStatus(device_id.clone())
                .await?;
            items.push(format!(
                "{} · {} · {} · {}",
                paired_device_label(&device_id, &peer),
                device_id,
                paired_device_direction(peer.inbound, peer.outbound),
                paired_status_label(text, &status),
            ));
        }
        self.open_list_popup(text.network_peers_title().to_string(), items);
        Ok(())
    }

    /// Opens the listener settings form, seeded from the persisted config and
    /// the Host capability declaration so unsupported transports cannot be
    /// selected and the current bind address can be edited in place.
    async fn open_listener_settings(&mut self, return_to_hub: bool) {
        match self.build_listener_settings(return_to_hub) {
            Ok(()) => self.network_hub = None,
            Err(error) => self.status_message = error,
        }
    }

    fn build_listener_settings(&mut self, return_to_hub: bool) -> Result<(), String> {
        let config = self
            .networkControl
            .localHostConfig()?
            .ok_or_else(|| self.text().network_listener_unavailable().to_string())?;
        let capabilities = self.networkControl.listenerCapabilities()?;
        let (bindHost, bindPort) = split_bind_address(&config.bindAddress);
        self.listener_settings = Some(ListenerSettingsModal {
            bindHost,
            bindPort: if bindPort.is_empty() {
                "0".to_string()
            } else {
                bindPort
            },
            portMode: config.portMode,
            transports: config.transports.into_iter().collect(),
            supportedTransports: capabilities.transports,
            discoveryEnabled: config.discoveryEnabled,
            discoverySupported: capabilities.discoveryAdvertisement,
            transportCursor: 0,
            field: ListenerField::PortMode,
            error: None,
            returnToHub: return_to_hub,
        });
        Ok(())
    }

    /// Lists the persisted listener configuration without opening the form;
    /// useful from a scripted session where the form is not wanted.
    fn network_listener_popup(&mut self) -> Result<(), String> {
        let text = self.text();
        let Some(config) = self.networkControl.localHostConfig()? else {
            self.set_transient_status_message(text.network_listener_show_none().to_string());
            return Ok(());
        };
        let transports = if config.transports.is_empty() {
            text.network_listener_show_none_transports().to_string()
        } else {
            listener_transport_summary(&config.transports)
        };
        let mode = match config.portMode {
            PeerHostPortMode::Automatic => text.network_listener_mode_auto(),
            PeerHostPortMode::Fixed => text.network_listener_mode_fixed(),
        };
        let discovery = if config.discoveryEnabled {
            text.config_enabled()
        } else {
            text.config_disabled()
        };
        let items = vec![
            text.network_listener_show_bind(&config.bindAddress),
            text.network_listener_show_mode(mode),
            text.network_listener_show_discovery(discovery),
            text.network_listener_show_transports(&transports),
        ];
        self.open_list_popup(text.network_listener_show_title().to_string(), items);
        Ok(())
    }

    async fn handle_listener_settings_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let Some(field) = self
            .listener_settings
            .as_ref()
            .map(|modal| modal.field)
        else {
            return Ok(());
        };
        match key.code {
            KeyCode::Esc => {
                let return_to_hub = self
                    .listener_settings
                    .take()
                    .is_some_and(|modal| modal.returnToHub);
                if return_to_hub {
                    self.open_network_hub().await;
                }
            }
            KeyCode::Tab | KeyCode::Down => self.step_listener_field(true),
            KeyCode::BackTab | KeyCode::Up => self.step_listener_field(false),
            KeyCode::Left => self.adjust_listener_field(false),
            KeyCode::Right => self.adjust_listener_field(true),
            KeyCode::Char(' ') => match field {
                ListenerField::Host | ListenerField::Port => {}
                ListenerField::Save => self.apply_listener_settings().await,
                _ => self.activate_listener_field(),
            },
            // The fields are prefilled, so clearing one keystroke at a time is
            // unusable; Ctrl+U resets the focused text field.
            KeyCode::Char('u') | KeyCode::Char('U')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                if let Some(modal) = self.listener_settings.as_mut() {
                    match modal.field {
                        ListenerField::Host => modal.bindHost.clear(),
                        ListenerField::Port => modal.bindPort.clear(),
                        _ => {}
                    }
                    modal.error = None;
                }
            }
            KeyCode::Backspace => {
                if let Some(modal) = self.listener_settings.as_mut() {
                    match modal.field {
                        ListenerField::Host => {
                            modal.bindHost.pop();
                        }
                        ListenerField::Port => {
                            modal.bindPort.pop();
                        }
                        _ => {}
                    }
                    modal.error = None;
                }
            }
            KeyCode::Char(ch)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                if let Some(modal) = self.listener_settings.as_mut() {
                    match modal.field {
                        // Hostnames and addresses never contain whitespace;
                        // swallowing it keeps the form value valid.
                        ListenerField::Host if !ch.is_whitespace() => {
                            modal.bindHost.push(ch);
                            modal.error = None;
                        }
                        ListenerField::Port if ch.is_ascii_digit() => {
                            modal.bindPort.push(ch);
                            modal.error = None;
                        }
                        _ => {}
                    }
                }
            }
            KeyCode::Enter => match field {
                ListenerField::Host | ListenerField::Port | ListenerField::Save => {
                    self.apply_listener_settings().await;
                }
                _ => self.activate_listener_field(),
            },
            _ => {}
        }
        Ok(())
    }

    fn step_listener_field(&mut self, forward: bool) {
        if let Some(modal) = self.listener_settings.as_mut() {
            modal.field = modal.field.step(forward);
            modal.error = None;
        }
    }

    /// Left/Right on a choice row: the port mode flips, the transport cursor
    /// moves. Text rows ignore the arrows, matching the pairing wizard.
    fn adjust_listener_field(&mut self, forward: bool) {
        let Some(modal) = self.listener_settings.as_mut() else {
            return;
        };
        match modal.field {
            ListenerField::PortMode => {
                modal.portMode = match modal.portMode {
                    PeerHostPortMode::Automatic => PeerHostPortMode::Fixed,
                    PeerHostPortMode::Fixed => PeerHostPortMode::Automatic,
                };
                if modal.portMode == PeerHostPortMode::Automatic {
                    modal.bindPort = "0".to_string();
                }
                modal.error = None;
            }
            ListenerField::Discovery => {
                if modal.discoverySupported {
                    modal.discoveryEnabled = !modal.discoveryEnabled;
                    modal.error = None;
                }
            }
            ListenerField::Transports => {
                let len = PAIR_WIZARD_TRANSPORTS.len();
                modal.transportCursor = if forward {
                    (modal.transportCursor + 1) % len
                } else {
                    (modal.transportCursor + len - 1) % len
                };
                modal.error = None;
            }
            _ => {}
        }
    }

    /// Space/Enter on a choice row. Transports respect the Host capability
    /// declaration: an unsupported chip cannot be turned on.
    fn activate_listener_field(&mut self) {
        let text = self.text();
        let Some(modal) = self.listener_settings.as_mut() else {
            return;
        };
        match modal.field {
            ListenerField::PortMode => {
                modal.portMode = match modal.portMode {
                    PeerHostPortMode::Automatic => PeerHostPortMode::Fixed,
                    PeerHostPortMode::Fixed => PeerHostPortMode::Automatic,
                };
                if modal.portMode == PeerHostPortMode::Automatic {
                    modal.bindPort = "0".to_string();
                }
                modal.error = None;
            }
            ListenerField::Discovery => {
                if modal.discoverySupported {
                    modal.discoveryEnabled = !modal.discoveryEnabled;
                    modal.error = None;
                }
            }
            ListenerField::Transports => {
                let transport = PAIR_WIZARD_TRANSPORTS[modal.transportCursor];
                if modal.supportedTransports.contains(&transport) {
                    if !modal.transports.remove(&transport) {
                        modal.transports.insert(transport);
                    }
                    modal.error = None;
                } else {
                    modal.error = Some(
                        text.network_listener_unsupported_transport(peer_transport_label(
                            &transport,
                        )),
                    );
                }
            }
            _ => {}
        }
    }

    fn set_listener_error(&mut self, error: String) {
        if let Some(modal) = self.listener_settings.as_mut() {
            modal.error = Some(error);
        } else {
            self.status_message = error;
        }
    }

    /// Persists the form and restarts the listeners, mirroring desktop save:
    /// stop first so the old bind is released, save, then reopen the selected
    /// transports. A failed start leaves the saved config in place and keeps
    /// the form open so the address can be corrected.
    async fn apply_listener_settings(&mut self) {
        let text = self.text();
        let Some(modal) = self.listener_settings.as_ref() else {
            return;
        };
        let plan = match plan_listener_settings(
            text,
            &modal.bindHost,
            &modal.bindPort,
            modal.portMode,
            modal.discoveryEnabled,
            modal.discoverySupported,
            &modal.transports,
            &modal.supportedTransports,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                self.set_listener_error(error);
                return;
            }
        };
        let return_to_hub = modal.returnToHub;
        let transports = plan.transports.clone();
        let config = PeerHostConfig {
            bindAddress: plan.bindAddress.clone(),
            transports: transports.clone(),
            discoveryEnabled: plan.discoveryEnabled,
            portMode: plan.portMode,
            updatedAt: operit_host_api::TimeUtils::currentTimeMillis(),
        };
        if let Err(error) = self.networkControl.stopListening().await {
            self.set_listener_error(error);
            return;
        }
        if let Err(error) = self.networkControl.saveLocalHostConfig(config.clone()) {
            self.set_listener_error(error);
            return;
        }
        if !transports.is_empty() {
            if let Err(error) = self.networkControl.startListening(transports.clone()).await {
                self.set_listener_error(error);
                return;
            }
        }
        // Automatic mode may replace the requested port with the bound one.
        let address = self
            .networkControl
            .localHostConfig()
            .ok()
            .flatten()
            .map(|config| config.bindAddress)
            .unwrap_or_else(|| config.bindAddress.clone());
        self.listener_settings = None;
        self.status_message = if transports.is_empty() {
            text.network_listener_stopped().to_string()
        } else {
            text.network_listener_saved(&address, listener_transport_summary(&transports))
        };
        if return_to_hub {
            self.open_network_hub().await;
        }
    }

    /// Opens the direct-address editor for one paired device. The address is
    /// deliberately empty: the runtime stores the dial record but does not
    /// expose it, so the operator types the replacement (same as desktop).
    fn open_endpoint_edit(&mut self, device_id: String, label: String, return_to_hub: bool) {
        self.network_hub = None;
        self.device_manager = None;
        self.endpoint_edit = Some(EndpointEditModal {
            deviceId: device_id,
            label,
            input: String::new(),
            error: None,
            returnToHub: return_to_hub,
        });
    }

    async fn handle_endpoint_edit_key(&mut self, key: KeyEvent) -> Result<(), String> {
        match key.code {
            KeyCode::Esc => {
                let return_to_hub = self
                    .endpoint_edit
                    .take()
                    .is_some_and(|modal| modal.returnToHub);
                if return_to_hub {
                    self.open_network_hub().await;
                }
            }
            KeyCode::Char('u') | KeyCode::Char('U')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                if let Some(modal) = self.endpoint_edit.as_mut() {
                    modal.input.clear();
                    modal.error = None;
                }
            }
            KeyCode::Backspace => {
                if let Some(modal) = self.endpoint_edit.as_mut() {
                    modal.input.pop();
                    modal.error = None;
                }
            }
            KeyCode::Char(ch)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                if let Some(modal) = self.endpoint_edit.as_mut() {
                    if !ch.is_whitespace() {
                        modal.input.push(ch);
                        modal.error = None;
                    }
                }
            }
            KeyCode::Enter => self.submit_endpoint_edit().await,
            _ => {}
        }
        Ok(())
    }

    async fn submit_endpoint_edit(&mut self) {
        let text = self.text();
        let Some((device_id, label, endpoint, return_to_hub)) =
            self.endpoint_edit.as_ref().map(|modal| {
                (
                    modal.deviceId.clone(),
                    modal.label.clone(),
                    modal.input.trim().to_string(),
                    modal.returnToHub,
                )
            })
        else {
            return;
        };
        if endpoint.is_empty() {
            if let Some(modal) = self.endpoint_edit.as_mut() {
                modal.error = Some(text.network_endpoint_address_required().to_string());
            }
            return;
        }
        match self
            .networkControl
            .updatePairedDeviceEndpoint(device_id, endpoint, None)
            .await
        {
            Ok(()) => {
                self.endpoint_edit = None;
                self.status_message = text.network_endpoint_saved(&label);
                if return_to_hub {
                    self.open_network_hub().await;
                }
            }
            Err(error) => {
                if let Some(modal) = self.endpoint_edit.as_mut() {
                    modal.error = Some(error);
                }
            }
        }
    }

    /// Removes one pairing credential without touching Space membership;
    /// `/network remove` is the full forget for Space members.
    async fn network_unpair(&mut self, device: &str) -> Result<String, String> {
        let (device_id, label) = self.resolve_paired_device_argument(device).await?;
        self.networkControl.removePairedDevice(device_id).await?;
        Ok(format!("network pairing removed: {label}"))
    }

    /// Requests to join the paired device's Space. Approval happens on the
    /// target device; this session pulls the decision in the background.
    async fn network_join(&mut self, device: &str) -> Result<String, String> {
        let (device_id, label) = self.resolve_paired_device_argument(device).await?;
        self.network_join_target(&device_id, &label).await
    }

    /// Join entry for callers that already hold the node id (hub rows,
    /// pairing wizard): requires a current pairing, then submits. The pairing
    /// direction does not gate joining; approval happens on the target device.
    async fn network_join_target(
        &mut self,
        device_id: &str,
        label: &str,
    ) -> Result<String, String> {
        let peers = self.networkControl.pairedDevicesSnapshot()?;
        if !peers.contains_key(device_id) {
            return Err(format!("no paired device matches {device_id}"));
        }
        let request = self
            .networkControl
            .requestDeviceSpaceJoin(device_id.to_string())
            .await?;
        if request.status == SpaceJoinStatus::Joined {
            return Ok(format!(
                "network join completed: now a member of {}",
                request.spaceName
            ));
        }
        Ok(format!(
            "network join request submitted: {label} must approve it; \
             this TUI refreshes the status automatically"
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
                    join_status_label(text, &request.status),
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
                _ => {
                    return Err(format!(
                    "no outgoing join request matches \"{value}\"; list them with /network joins"
                ))
                }
            }
        };
        let request = self
            .networkControl
            .cancelDeviceSpaceJoin(request_id.clone())
            .await?;
        Ok(format!(
            "network join request cancelled: {} ({})",
            join_status_label(self.text(), &request.status),
            request_id
        ))
    }

    /// Opens the Y/N confirm popup for leaving the current device space, the
    /// quick way out of tangled connection state. The leave itself only runs
    /// on confirmation, and the popup quotes the space being exited.
    async fn network_leave(&mut self) -> Result<(), String> {
        let space = self.networkControl.deviceSpace()?;
        let text = self.text();
        let items = vec![
            format!("{}: {}", text.network_leave_space_label(), space.spaceName),
            format!(
                "{}: {}",
                text.network_leave_members_label(),
                space.members.len()
            ),
            text.network_leave_warning().to_string(),
        ];
        self.open_list_popup(text.network_leave_title().to_string(), items);
        self.leave_confirm_pending = true;
        Ok(())
    }

    /// Resolves a `/network unpair|join` argument against paired devices:
    /// exact node id, or a unique display-name match.
    async fn resolve_paired_device_argument(
        &self,
        value: &str,
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
            .filter(|request| request.applicantDeviceId == value || request.applicantName == value)
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
        let request_id = request.requestId.clone();
        let decision = self
            .networkControl
            .decideDeviceSpaceJoin(request_id.clone(), request.assignmentVersion, approve)
            .await;
        let modal = self
            .join_decision
            .as_mut()
            .expect("join modal checked above");
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
                self.status_message = status_error_line(&error);
                // A rejected decision usually means the assignment moved under
                // the cached entry; re-pull so the retry targets the current
                // version instead of failing forever.
                if let Ok(fresh) = self.networkControl.incomingDeviceSpaceJoins().await {
                    self.refresh_join_decision_modal(&fresh);
                    let still_pending = self.join_decision.as_ref().is_some_and(|modal| {
                        modal
                            .requests
                            .iter()
                            .any(|request| request.requestId == request_id)
                    });
                    if !still_pending {
                        self.status_message =
                            self.text().network_join_decision_refreshed().to_string();
                    }
                }
            }
        }
    }

    /// Opens the device management window over a fresh control snapshot.
    /// The window is keyboard-driven like the join decision modal; the
    /// mouse overlay flag only keeps clicks from leaking into the
    /// transcript underneath.
    /// Lists accepted and rejected Space authorization commands.
    async fn network_audit_popup(&mut self) -> Result<(), String> {
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
        Ok(())
    }

    /// Lists the current Space policy settings.
    fn network_policy_popup(&mut self) -> Result<(), String> {
        let items = self
            .networkControl
            .deviceSpaceControl()?
            .policies
            .into_iter()
            .map(|(policyId, value)| format!("{policyId}={value}"))
            .collect::<Vec<_>>();
        self.open_list_popup("Network policies".to_string(), items);
        Ok(())
    }

    /// Runs the read-only shared-Space doctor and lists its report, including
    /// the reconciliation counters. Repair stays a CLI action
    /// (`operit2 cli link doctor --repair`); the TUI only observes health.
    fn network_doctor_popup(&mut self) -> Result<(), String> {
        let report = space_diagnose(&self.networkControl, false)
            .map_err(|error| format!("space diagnosis failed: {error}"))?;
        let items = space_health_report_lines(self.text(), &report);
        let title = self.text().network_doctor_title().to_string();
        self.open_list_popup(title, items);
        Ok(())
    }

    /// Opens the network hub: the persistent `/network` panel answering
    /// who-this-node-is, what-is-waiting, and what-is-managed in one place.
    async fn open_network_hub(&mut self) {
        self.network_hub = Some(NetworkHubModal {
            topology: RuntimeDeviceSpaceTopology {
                currentDeviceId: String::new(),
                devices: Vec::new(),
                connections: Vec::new(),
            },
            spaceName: String::new(),
            initialized: false,
            listening: None,
            paired: BTreeMap::new(),
            prompts: Vec::new(),
            outgoingJoins: Vec::new(),
            blocked: BTreeSet::new(),
            selected: 0,
        });
        self.refresh_network_hub();
    }

    /// Re-pulls every hub snapshot, keeping the selection on the same row.
    fn refresh_network_hub(&mut self) {
        let selected_id = self
            .network_hub
            .as_ref()
            .map(|hub| {
                network_hub_rows(&hub.topology, &hub.paired)
                    .get(hub.selected)
                    .map(|row| row.id().to_string())
            })
            .flatten();
        let Ok(state) = self.networkControl.deviceSpaceControl() else {
            return;
        };
        let Ok(topology) = self.networkControl.deviceSpaceTopology() else {
            return;
        };
        let space_name = self
            .networkControl
            .deviceSpace()
            .map(|space| space.spaceName);
        let listening = self
            .networkControl
            .localHostConfig()
            .ok()
            .flatten()
            // An empty transport list means the operator stopped listening;
            // the saved bind address alone would read as a live listener.
            .filter(|config| !config.transports.is_empty())
            .map(|config| {
                format!(
                    "{} · {}",
                    config.bindAddress,
                    config
                        .transports
                        .iter()
                        .map(|transport| peer_transport_label(transport))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            });
        let Ok(paired) = self.networkControl.pairedDevicesSnapshot() else {
            return;
        };
        let prompts = self.networkControl.pairingPrompts().unwrap_or_default();
        let outgoing_joins = self
            .networkControl
            .outgoingDeviceSpaceJoins()
            .unwrap_or_default();
        let Some(hub) = self.network_hub.as_mut() else {
            return;
        };
        hub.initialized = state.initialized;
        hub.blocked = state.disconnectedNodeIds;
        hub.topology = topology;
        if let Ok(name) = space_name {
            hub.spaceName = name;
        }
        hub.listening = listening;
        hub.paired = paired;
        hub.prompts = prompts;
        hub.outgoingJoins = outgoing_joins;
        let rows = network_hub_rows(&hub.topology, &hub.paired);
        if let Some(selected_id) = selected_id {
            if let Some(position) = rows.iter().position(|row| row.id() == selected_id) {
                hub.selected = position;
            }
        }
        hub.selected = hub.selected.min(rows.len().saturating_sub(1));
    }

    /// Routes keys inside the network hub: rows select with Up/Down, Enter
    /// manages members and joins paired peers, single letters open flows.
    async fn handle_network_hub_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let row_count = self
            .network_hub
            .as_ref()
            .map(|hub| network_hub_rows(&hub.topology, &hub.paired).len())
            .unwrap_or(0);
        let selected_row = self.network_hub.as_ref().and_then(|hub| {
            network_hub_rows(&hub.topology, &hub.paired)
                .into_iter()
                .nth(hub.selected)
        });
        match key.code {
            KeyCode::Up => {
                if let Some(hub) = self.network_hub.as_mut() {
                    hub.selected = hub.selected.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if let Some(hub) = self.network_hub.as_mut() {
                    if hub.selected + 1 < row_count {
                        hub.selected += 1;
                    }
                }
            }
            KeyCode::Enter | KeyCode::Char('j') | KeyCode::Char('J') => {
                match selected_row {
                    Some(NetworkHubRow::Member(device)) if key.code == KeyCode::Enter => {
                        // The local device never offers management actions, so
                        // opening the manager on it would be a dead window.
                        let is_self = self
                            .network_hub
                            .as_ref()
                            .is_some_and(|hub| hub.topology.currentDeviceId == device.deviceId);
                        if is_self {
                            self.set_transient_status_message(
                                self.text().network_hub_self_no_actions().to_string(),
                            );
                        } else {
                            self.open_device_manager_for(&device.deviceId).await;
                        }
                    }
                    Some(NetworkHubRow::Peer {
                        deviceId, label, ..
                    }) => {
                        // A rejected join is ordinary peer feedback; it must
                        // never tear down the TUI loop.
                        match self.network_join_target(&deviceId, &label).await {
                            Ok(message) => self.status_message = message,
                            Err(error) => self.status_message = status_error_line(&error),
                        }
                    }
                    Some(NetworkHubRow::Member(device)) => {
                        self.status_message = format!(
                            "{} ({}) is already a member of this space",
                            device.deviceName, device.deviceId
                        );
                    }
                    None => {}
                }
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                if let Some(NetworkHubRow::Peer {
                    deviceId, label, ..
                }) = selected_row
                {
                    self.network_unpair_confirm(&deviceId, &label);
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.open_listener_settings(true).await;
            }
            KeyCode::Char('e') | KeyCode::Char('E') => match selected_row {
                Some(NetworkHubRow::Peer {
                    deviceId, label, ..
                }) => self.open_endpoint_edit(deviceId, label, true),
                Some(NetworkHubRow::Member(device)) => {
                    let paired = self
                        .network_hub
                        .as_ref()
                        .is_some_and(|hub| hub.paired.contains_key(&device.deviceId));
                    if paired {
                        self.open_endpoint_edit(
                            device.deviceId.clone(),
                            device.deviceName.clone(),
                            true,
                        );
                    } else {
                        self.status_message =
                            self.text().network_endpoint_no_pairing(&device.deviceName);
                    }
                }
                None => {}
            },
            KeyCode::Char('p') | KeyCode::Char('P') if key.modifiers.is_empty() => {
                self.open_pair_wizard(String::new(), 0, false, None);
            }
            KeyCode::Char('P') => {
                if let Err(error) = self.network_policy_popup() {
                    self.status_message = error;
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.start_peer_discovery();
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                if let Err(error) = self.network_leave().await {
                    self.status_message = status_error_line(&error);
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                if let Err(error) = self.network_audit_popup().await {
                    self.status_message = error;
                }
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                if let Err(error) = self.network_doctor_popup() {
                    self.status_message = error;
                }
            }
            KeyCode::Esc => {
                self.network_hub = None;
            }
            _ => {}
        }
        Ok(())
    }

    /// Opens the pairing wizard. `autostart` immediately dials the address -
    /// the wizard then lands on code entry, which is the only step left.
    /// `expected` binds the handshake to a known node id and shows it while
    /// dialing, which is how the device window pairs one specific member.
    fn open_pair_wizard(
        &mut self,
        address: String,
        transport_index: usize,
        autostart: bool,
        expected: Option<(String, String)>,
    ) {
        let (expectedNode, expectedLabel) = match expected {
            Some((node_id, label)) => (Some(node_id), Some(label)),
            None => (None, None),
        };
        self.pair_wizard = Some(PairWizardModal {
            stage: if autostart {
                PairStage::Code
            } else {
                PairStage::Address
            },
            field: PairField::Address,
            address,
            transportIndex: transport_index.min(PAIR_WIZARD_TRANSPORTS.len() - 1),
            expectedNode,
            expectedLabel,
            pairing: None,
            code: String::new(),
            peer: None,
            error: None,
        });
        self.network_hub = None;
    }

    /// Dials the wizard's address form and advances to code entry. The
    /// pending pairing is remembered app-level so Esc never loses it.
    async fn pair_wizard_start(&mut self) {
        let Some(wizard) = self.pair_wizard.as_ref() else {
            return;
        };
        if wizard.address.trim().is_empty() {
            let message = self
                .text()
                .network_pair_wizard_address_required()
                .to_string();
            if let Some(wizard) = self.pair_wizard.as_mut() {
                wizard.error = Some(message);
            }
            return;
        }
        let address = wizard.address.trim().to_string();
        let transport = PAIR_WIZARD_TRANSPORTS[wizard.transportIndex];
        let expectedNode = wizard.expectedNode.clone().unwrap_or_default();
        match self
            .networkControl
            .startPairing(expectedNode, address, Some(transport))
            .await
        {
            Ok(pending) => {
                if let Some(wizard) = self.pair_wizard.as_mut() {
                    wizard.stage = PairStage::Code;
                    wizard.pairing = Some(pending.clone());
                    wizard.code.clear();
                    wizard.error = None;
                }
                self.pending_pairings.push(pending);
            }
            Err(error) => {
                if let Some(wizard) = self.pair_wizard.as_mut() {
                    wizard.stage = PairStage::Address;
                    wizard.error = Some(error);
                }
            }
        }
    }

    /// Submits the six-digit code for the wizard's pending pairing. Failure
    /// keeps the code stage open for a retry, exactly like the command path.
    async fn pair_wizard_confirm(&mut self) {
        let (pairing_id, code) = match self.pair_wizard.as_ref() {
            Some(wizard) => match &wizard.pairing {
                Some(pending) => (pending.pairingId.clone(), wizard.code.clone()),
                None => return,
            },
            None => return,
        };
        if code.len() != 6 {
            let message = self.text().network_pair_wizard_code_invalid().to_string();
            if let Some(wizard) = self.pair_wizard.as_mut() {
                wizard.error = Some(message);
            }
            return;
        }
        match self
            .networkControl
            .finishPairing(pairing_id.clone(), code)
            .await
        {
            Ok(peer) => {
                self.pending_pairings
                    .retain(|pending| pending.pairingId != pairing_id);
                // An admitted member needs only the direct link: the join
                // submission would be rejected as a duplicate membership.
                if self.network_peer_is_member(&peer.nodeId) {
                    self.pair_wizard = None;
                    self.set_transient_status_message(
                        self.text()
                            .network_direct_link_established(&paired_peer_label(&peer)),
                    );
                    return;
                }
                if let Some(wizard) = self.pair_wizard.as_mut() {
                    wizard.stage = PairStage::JoinOffer;
                    wizard.peer = Some(peer);
                    wizard.error = None;
                }
            }
            Err(error) => {
                if let Some(wizard) = self.pair_wizard.as_mut() {
                    wizard.error = Some(error);
                }
            }
        }
    }

    /// Sends the join request for the freshly paired peer and closes the
    /// wizard; the background refresher reports the reviewer's decision.
    async fn pair_wizard_join(&mut self) {
        let Some(peer) = self
            .pair_wizard
            .as_ref()
            .and_then(|wizard| wizard.peer.clone())
        else {
            return;
        };
        let label = paired_peer_label(&peer);
        let result = self.network_join_target(&peer.nodeId, &label).await;
        self.pair_wizard = None;
        match result {
            Ok(message) => self.set_transient_status_message(message),
            Err(error) => self.status_message = error,
        }
    }

    /// Routes keys inside the pairing wizard by stage.
    async fn handle_pair_wizard_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let stage = self
            .pair_wizard
            .as_ref()
            .map(|wizard| wizard.stage)
            .unwrap_or(PairStage::Address);
        match stage {
            PairStage::Address => {
                let field = self
                    .pair_wizard
                    .as_ref()
                    .map(|wizard| wizard.field)
                    .unwrap_or(PairField::Address);
                match (key.code, field) {
                    (KeyCode::Esc, _) => self.pair_wizard = None,
                    (KeyCode::Tab, _) | (KeyCode::BackTab, _) => {
                        if let Some(wizard) = self.pair_wizard.as_mut() {
                            wizard.field = match wizard.field {
                                PairField::Address => PairField::Transport,
                                PairField::Transport => PairField::Address,
                            };
                            wizard.error = None;
                        }
                    }
                    (KeyCode::Left, PairField::Transport) => self.cycle_pair_transport(false),
                    (KeyCode::Right, PairField::Transport) => self.cycle_pair_transport(true),
                    (KeyCode::Enter, _) => self.pair_wizard_start().await,
                    (KeyCode::Backspace, PairField::Address) => {
                        if let Some(wizard) = self.pair_wizard.as_mut() {
                            match wizard.field {
                                PairField::Address => {
                                    wizard.address.pop();
                                }
                                PairField::Transport => {}
                            }
                        }
                    }
                    (KeyCode::Char(ch), PairField::Address)
                        if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
                    {
                        if let Some(wizard) = self.pair_wizard.as_mut() {
                            match wizard.field {
                                PairField::Address => wizard.address.push(ch),
                                PairField::Transport => {}
                            }
                            wizard.error = None;
                        }
                    }
                    _ => {}
                }
            }
            PairStage::Code => match key.code {
                KeyCode::Esc => self.pair_wizard = None,
                KeyCode::Backspace => {
                    if let Some(wizard) = self.pair_wizard.as_mut() {
                        wizard.code.pop();
                    }
                }
                KeyCode::Char(ch) => {
                    if let Some(wizard) = self.pair_wizard.as_mut() {
                        wizard.code = pair_code_push(&wizard.code, ch);
                        wizard.error = None;
                    }
                }
                KeyCode::Enter => self.pair_wizard_confirm().await,
                _ => {}
            },
            PairStage::JoinOffer => match key.code {
                KeyCode::Enter => self.pair_wizard_join().await,
                KeyCode::Esc => self.pair_wizard = None,
                _ => {}
            },
        }
        Ok(())
    }

    /// Cycles the wizard transport selector; Tab walks fields while the
    /// arrows walk transports, so one hand never leaves the home row.
    fn cycle_pair_transport(&mut self, forward: bool) {
        let Some(wizard) = self.pair_wizard.as_mut() else {
            return;
        };
        let len = PAIR_WIZARD_TRANSPORTS.len();
        wizard.transportIndex = if forward {
            (wizard.transportIndex + 1) % len
        } else {
            (wizard.transportIndex + len - 1) % len
        };
    }

    /// Starts a background LAN scan; the selectable candidate list opens from
    /// its result event (Enter pairs with the selection). A scan blocks for its
    /// whole timeout, so it never runs on the terminal loop; pressing `d` again
    /// while one is in flight only re-states the status.
    fn start_peer_discovery(&mut self) {
        if !self.discover_in_flight {
            self.discover_in_flight = true;
            spawn_peer_discovery(
                self.networkControl.clone(),
                self.network_event_sender.clone(),
            );
        }
        self.set_transient_status_message(self.text().network_discover_running().to_string());
    }

    /// Feeds the discovery candidate under the popup cursor into the wizard
    /// and dials it right away - selecting a device is already consent.
    async fn start_pair_wizard_from_discovery(&mut self) {
        let selected = self
            .list_popup_filtered_indices
            .get(self.list_popup_selected_index)
            .copied();
        let candidate = selected.and_then(|index| self.discovered_peers.get(index).cloned());
        self.close_list_popup();
        self.discover_select_pending = false;
        let Some(candidate) = candidate else {
            return;
        };
        let transport_index = PAIR_WIZARD_TRANSPORTS
            .iter()
            .position(|transport| matches!(transport, PeerTransport::Tcp))
            .unwrap_or(2);
        // The candidate carries its node id, so the handshake binds to it.
        self.open_pair_wizard(
            candidate.address.clone(),
            transport_index,
            false,
            Some((candidate.nodeId.clone(), candidate.displayName.clone())),
        );
        self.pair_wizard_start().await;
    }

    /// Opens the device manager focused on one device and jumps straight
    /// into its action menu: the hub row already was the selection step.
    async fn open_device_manager_for(&mut self, device_id: &str) {
        self.open_device_manager().await;
        let Some(modal) = self.device_manager.as_mut() else {
            return;
        };
        let Some(position) = modal.rows().iter().position(|row| row.id() == device_id) else {
            return;
        };
        modal.selected = position;
        // Mirror the browse-Enter rule: no applicable action (the local
        // device with no roles defined) means no menu to open.
        if !modal.menu_actions(device_id).is_empty() {
            modal.mode = DeviceManagerMode::ActionMenu;
            modal.menu_device_id = Some(device_id.to_string());
            modal.menu_index = 0;
        }
    }

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
            paired: self.paired_device_ids(),
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

    /// Node ids this device currently holds a pairing (edge) for.
    fn paired_device_ids(&self) -> BTreeSet<String> {
        self.networkControl
            .pairedDevicesSnapshot()
            .map(|snapshot| snapshot.into_keys().collect())
            .unwrap_or_default()
    }

    /// Persists a human-friendly device model so headless servers appear as a
    /// recognizable name in Space instead of the CLI transport label.
    fn rename_local_device(&self, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("device name must not be empty".to_string());
        }
        let platform = self
            .networkControl
            .deviceSpaceTopology()
            .ok()
            .and_then(|topology| {
                topology
                    .devices
                    .iter()
                    .find(|device| device.deviceId == topology.currentDeviceId)
                    .map(|device| device.platform.clone())
            })
            .filter(|platform| !platform.trim().is_empty())
            .unwrap_or_else(|| std::env::consts::OS.to_string());
        self.networkControl.updateDeviceInfo(LinkDeviceInfo {
            platform,
            model: name.to_string(),
            pairingIncarnation: None,
        })?;
        Ok(())
    }

    /// Reports whether one device is already a member of the local Space.
    ///
    /// Membership is the join contract's own criterion: a request from an
    /// admitted device is rejected as a duplicate, so the pairing flows must
    /// not offer that join step.
    fn network_peer_is_member(&self, device_id: &str) -> bool {
        self.networkControl
            .deviceSpace()
            .map(|space| space.members.iter().any(|member| member == device_id))
            .unwrap_or(false)
    }

    /// Opens the pairing wizard for one device picked in the device window.
    ///
    /// Pairing leads the device menu because it is the only link-layer action:
    /// the expected node id binds the handshake, and an admitted device needs
    /// no join afterwards.
    async fn network_pair_device(&mut self, device: &str) -> Result<String, String> {
        let topology = self.networkControl.deviceSpaceTopology()?;
        let device_id = network_device_id(&topology, device)?;
        let device_label = network_device_label_by_id(&topology, &device_id)?;
        self.open_pair_wizard(
            String::new(),
            0,
            false,
            Some((device_id, device_label.clone())),
        );
        Ok(self.text().network_pair_device_hint(&device_label))
    }

    /// Re-pulls the control snapshot into an open device window, keeping
    /// the selection on the same row identity. A device whose action menu
    /// is open disappears from the topology, the window drops back to
    /// browsing instead of managing a ghost row.
    async fn refresh_device_manager_snapshot(&mut self) {
        let requests = self
            .networkControl
            .incomingDeviceSpaceJoins()
            .await
            .unwrap_or_default();
        self.apply_device_manager_snapshot(requests);
    }

    fn apply_device_manager_snapshot(&mut self, requests: Vec<SpaceJoinRequest>) {
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
        let paired = self.paired_device_ids();
        let Some(modal) = self.device_manager.as_mut() else {
            return;
        };
        modal.initialized = state.initialized;
        modal.blocked = state.disconnectedNodeIds;
        modal.paired = paired;
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
            if let Some(position) = modal.rows().iter().position(|row| row.id() == selected_id) {
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
            DeviceManagerMode::ConfirmUnpair => {
                self.handle_device_manager_confirm_unpair_key(key).await
            }
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
                // Unpairing severs the link the same way the hub's `u` does, so
                // both paths ask for the same explicit Y.
                if action == DeviceManagerAction::Unpair {
                    if let Some(modal) = self.device_manager.as_mut() {
                        modal.mode = DeviceManagerMode::ConfirmUnpair;
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

    /// Y/N confirm for unpairing from the device window: the same explicit Y
    /// the hub's `u` shortcut requires, so neither path severs a pairing on a
    /// stray key.
    async fn handle_device_manager_confirm_unpair_key(
        &mut self,
        key: KeyEvent,
    ) -> Result<(), String> {
        let Some(device_id) = self
            .device_manager
            .as_ref()
            .and_then(|modal| modal.menu_device_id.clone())
        else {
            return Ok(());
        };
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Char('1') => {
                self.run_device_manager_action(&device_id, DeviceManagerAction::Unpair)
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
        // The address editor takes over the screen, so close the window it
        // was launched from instead of stacking two keyboard surfaces.
        if action == DeviceManagerAction::UpdateEndpoint {
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
            self.device_manager = None;
            self.open_endpoint_edit(device_id.to_string(), label, false);
            return;
        }
        let result = match action {
            DeviceManagerAction::Pair => self.network_pair_device(device_id).await,
            DeviceManagerAction::Admit => self.network_admit_device(device_id).await,
            DeviceManagerAction::Disconnect => self.network_disconnect_device(device_id).await,
            DeviceManagerAction::Unpair => self.network_unpair(device_id).await,
            DeviceManagerAction::Remove => self.network_remove_device(device_id).await,
            DeviceManagerAction::ClearIdentity => {
                self.network_clear_device_identity(device_id).await
            }
            DeviceManagerAction::UpdateEndpoint => return,
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

    /// The frontend edits the application model binding; plugin-specific overrides belong to the plugin.
    async fn editable_chat_model_ref(&mut self) -> Result<ModelRef, String> {
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
        self.leave_confirm_pending = false;
        self.unpair_confirm = None;
        self.discover_select_pending = false;
        self.focus = FocusArea::Input;
    }

    fn close_list_popup(&mut self) {
        self.show_list_popup = false;
        self.list_popup_title.clear();
        self.list_popup_items.clear();
        self.list_popup_search.clear();
        self.list_popup_filtered_indices.clear();
        self.list_popup_selected_index = 0;
        self.leave_confirm_pending = false;
        self.unpair_confirm = None;
        self.discover_select_pending = false;
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

    /// Y/N confirm for `/network leave`: Y leaves the current device space,
    /// N/Esc cancels; every other key is consumed so the warning cannot be
    /// searched or Enter-closed into an accidental state.
    async fn handle_leave_confirm_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.leave_confirm_pending = false;
                self.close_list_popup();
                match self.networkControl.leaveDeviceSpace() {
                    Ok(space) => {
                        self.set_transient_status_message(
                            self.text().network_leave_done(&space.spaceName),
                        );
                        // Re-seed the network snapshots for the new singleton
                        // space so ids from the old one cannot replay.
                        self.refresh_network_snapshots().await;
                    }
                    Err(error) => self.status_message = error,
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.leave_confirm_pending = false;
                self.close_list_popup();
            }
            _ => {}
        }
    }

    /// Opens the Y/N confirm for unpairing a peer from the hub: one stray key
    /// must never sever a pairing, so removal waits for an explicit Y.
    fn network_unpair_confirm(&mut self, device_id: &str, label: &str) {
        let text = self.text();
        let items = vec![
            format!("{}: {}", text.network_unpair_device_label(), label),
            text.network_unpair_warning(label),
        ];
        self.open_list_popup(text.network_unpair_title().to_string(), items);
        self.unpair_confirm = Some((device_id.to_string(), label.to_string()));
    }

    /// Y/N confirm for the hub's `u`: Y removes the pairing, N/Esc cancels;
    /// every other key is consumed so it cannot reach the hub underneath.
    async fn handle_unpair_confirm_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let Some((device_id, label)) = self.unpair_confirm.take() else {
                    return;
                };
                self.close_list_popup();
                match self.networkControl.removePairedDevice(device_id).await {
                    Ok(()) => {
                        self.set_transient_status_message(self.text().network_unpair_done(&label))
                    }
                    Err(error) => self.status_message = status_error_line(&error),
                }
                self.refresh_network_hub();
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.unpair_confirm = None;
                self.close_list_popup();
            }
            _ => {}
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
            .runCoreCommand(&shell_args.new_chat_command_args())
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
                    if let Some(value) =
                        self.core.decodeStateFlowEvent::<Vec<ChatHistory>>(&event)?
                    {
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
            std::hash::Hash::hash(
                &serde_json::to_vec(&markdown).map_err(|error| error.to_string())?,
                &mut hash,
            );
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
            AppLogger::trace(
                "TuiSyncMeasure",
                &format!(
                    "event.applied streamId={} sequence={} fingerprint={:016x} eventType={}",
                    streamId, eventSequence, fingerprint, markdown.eventType
                ),
            );
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

    async fn open_diff_viewer(&mut self) -> Result<(), String> {
        let mut files = Vec::new();
        for message in self.current_messages() {
            for part in &message.parts {
                files.extend(super::diff::parse_file_diffs_from_text(&part.content));
            }
        }
        if files.is_empty() {
            self.status_message = self.text().diff_no_changes().to_string();
            return Ok(());
        }
        self.diff_viewer = Some(super::diff::DiffViewerModal::new(files));
        Ok(())
    }

    fn handle_diff_viewer_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let Some(modal) = &mut self.diff_viewer else {
            return Ok(());
        };
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.diff_viewer = None,
            KeyCode::Char('j') | KeyCode::Down => modal.scroll_down(1),
            KeyCode::Char('k') | KeyCode::Up => modal.scroll_up(1),
            KeyCode::PageDown => modal.scroll_down(10),
            KeyCode::PageUp => modal.scroll_up(10),
            KeyCode::Char('[') => modal.prev_file(),
            KeyCode::Char(']') => modal.next_file(),
            KeyCode::Tab => modal.toggle_view_mode(),
            KeyCode::Char('e') => {
                if let Err(error) = modal.open_external() {
                    self.status_message = error;
                }
            }
            _ => {}
        }
        Ok(())
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
        if self
            .last_space_membership_check_at
            .map(|last| now.saturating_duration_since(last) >= SPACE_MEMBERSHIP_REFRESH_INTERVAL)
            .unwrap_or(true)
        {
            self.last_space_membership_check_at = Some(now);
            self.check_space_membership_changes();
        }
    }

    /// Notices local Space membership changes (a device joined, left, or was
    /// removed) on the status line. A Space identity change re-seeds silently:
    /// leaving and joining already report themselves, and the member set
    /// changes wholesale with them.
    fn check_space_membership_changes(&mut self) {
        let Ok(space) = self.networkControl.deviceSpace() else {
            return;
        };
        let members = space.members.iter().cloned().collect::<BTreeSet<_>>();
        let identity_changed = self
            .space_identity
            .as_ref()
            .is_some_and(|(space_id, _)| space_id != &space.spaceId);
        let previous = std::mem::replace(&mut self.space_members, members.clone());
        let seeded = self
            .space_identity
            .replace((space.spaceId.clone(), space.spaceName.clone()));
        if seeded.is_none() || identity_changed {
            return;
        }
        let name_for = self
            .networkControl
            .deviceSpaceTopology()
            .map(|topology| {
                topology
                    .devices
                    .into_iter()
                    .map(|device| (device.deviceId, device.deviceName))
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default();
        if let Some(message) = space_membership_notice(self.text(), &previous, &members, &name_for)
        {
            self.set_transient_status_message(message);
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
        // Loading transitions only refresh the status line. They must never
        // re-engage transcript following: this runs every 250ms while a
        // response streams, and a reader who scrolled into history keeps
        // their place until they return to the newest line themselves.
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
                    self.refresh_chats().await;
                    // Model binding now lives in the persistent right footer
                    // segment; reset the left status for command feedback.
                    match self.current_chat_model_status_label().await {
                        Ok(_) => self.status_message.clear(),
                        Err(error) => self.set_status_message(error),
                    }
                }
                _ => {
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
        let max_tokens = i64::from(config.context.maxContextTokens());
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

    /// Opens the headless control panel on one tab and loads its snapshot.
    async fn open_control_panel(&mut self, tab: ControlPanelTab) {
        self.show_list_popup = false;
        self.control_panel = Some(ControlPanelModal::new(tab));
        self.refresh_control_panel().await;
    }

    /// Reloads the active panel without losing the selected row identity.
    async fn refresh_control_panel(&mut self) {
        let Some(tab) = self.control_panel.as_ref().map(|panel| panel.tab) else {
            return;
        };
        let selected_id = self
            .control_panel
            .as_ref()
            .and_then(|panel| panel.selected_row().map(|row| row.id.clone()));
        match self.load_control_panel_data(tab).await {
            Ok(data) => {
                let Some(panel) = self.control_panel.as_mut() else {
                    return;
                };
                panel.rows = data.rows;
                panel.detail_title = data.detail_title;
                panel.detail_lines = data.detail_lines;
                panel.log_path = data.log_path;
                panel.log_lines = data.log_lines;
                panel.detail_scroll = 0;
                if let Some(selected_id) = selected_id {
                    panel.selected = panel
                        .rows
                        .iter()
                        .position(|row| row.id == selected_id)
                        .unwrap_or(0);
                }
                let row_count = panel.rows.len();
                panel.selected = panel.selected.min(row_count.saturating_sub(1));
                if panel.mode == ControlPanelMode::Detail {
                    panel.mode = ControlPanelMode::Browse;
                }
                panel.status = None;
            }
            Err(error) => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.status = Some(error);
                }
            }
        }
    }

    async fn load_control_panel_data(
        &mut self,
        tab: ControlPanelTab,
    ) -> Result<ControlPanelData, String> {
        match tab {
            ControlPanelTab::Status => self.load_status_panel().await,
            ControlPanelTab::Logs => Ok(self.load_log_panel()),
            ControlPanelTab::Packages => self.load_package_panel().await,
            ControlPanelTab::Plugins => self.load_plugin_panel().await,
            ControlPanelTab::Skills => self.load_skill_panel().await,
            ControlPanelTab::Mcp => self.load_mcp_panel().await,
            ControlPanelTab::Identities => self.load_identity_panel(),
        }
    }

    async fn load_status_panel(&mut self) -> Result<ControlPanelData, String> {
        let text = self.text();
        let mut lines = Vec::new();
        let (identities, active_id) = crate::bootstrap::cli_identities();
        if let Some(active) = identities.iter().find(|identity| identity.id == active_id) {
            lines.push(text.control_status_identity(&active.id, &active.name));
        }
        lines.push(
            text.control_status_version(env!("CARGO_PKG_VERSION"), operit_runtime::CORE_VERSION),
        );
        if let Ok(topology) = self.networkControl.deviceSpaceTopology() {
            lines.push(text.control_status_node(&topology.currentDeviceId));
            if let Some(device) = topology
                .devices
                .iter()
                .find(|device| device.deviceId == topology.currentDeviceId)
            {
                lines.push(text.control_status_device(
                    &device.model,
                    &device.deviceName,
                    &device.platform,
                ));
            }
        }
        if let Ok(space) = self.networkControl.deviceSpace() {
            lines.push(text.control_status_space(space.members.len(), &space.spaceName));
        }
        let listener = self
            .networkControl
            .localHostConfig()
            .ok()
            .flatten()
            .map(|config| {
                format!(
                    "{} · {}",
                    config.bindAddress,
                    config
                        .transports
                        .iter()
                        .map(|transport| peer_transport_label(transport))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .unwrap_or_else(|| "off".to_string());
        lines.push(text.control_status_listener(&listener));
        let model = self
            .current_chat_model_status_label()
            .await
            .unwrap_or_else(|error| error);
        lines.push(text.control_status_model(&model));
        let mode = self
            .core
            .permissions_tool_permission_system()
            .getAiPermissionMode()
            .await
            .map(|mode| mode.name().to_string())
            .unwrap_or_else(|error| error.to_string());
        lines.push(text.control_status_approval(&mode));
        let prompts = self
            .networkControl
            .pairingPrompts()
            .unwrap_or_default()
            .len();
        let joins = self
            .networkControl
            .incomingDeviceSpaceJoins()
            .await
            .map(|requests| requests.len())
            .unwrap_or(0);
        lines.push(text.control_status_prompts(joins, prompts));
        let issues = self
            .run_control_core_json(&["package", "issues"])
            .await
            .ok()
            .and_then(|value| value.as_array().map(Vec::len))
            .unwrap_or(0);
        lines.push(text.control_status_issues(issues));
        let log_path = AppLogger::get_log_file_path().unwrap_or_default();
        lines.push(text.control_status_log(&log_path));
        Ok(ControlPanelData {
            detail_title: text.control_status_title().to_string(),
            detail_lines: lines,
            ..ControlPanelData::empty()
        })
    }

    fn load_log_panel(&self) -> ControlPanelData {
        let text = self.text();
        let path = AppLogger::get_log_file_path().unwrap_or_default();
        let lines = read_log_tail(&path, 500);
        ControlPanelData {
            detail_title: text.control_log_path(&path),
            detail_lines: lines.clone(),
            log_path: path,
            log_lines: lines,
            ..ControlPanelData::empty()
        }
    }

    async fn load_package_panel(&mut self) -> Result<ControlPanelData, String> {
        let mut data = ControlPanelData::empty();
        data.rows = self.load_extension_rows("package").await?;
        Ok(data)
    }

    async fn load_plugin_panel(&mut self) -> Result<ControlPanelData, String> {
        let mut data = ControlPanelData::empty();
        data.rows = self.load_extension_rows("plugin").await?;
        Ok(data)
    }

    async fn load_extension_rows(&mut self, kind: &str) -> Result<Vec<ControlPanelRow>, String> {
        let mut rows = Vec::new();
        let loaded = self
            .run_control_core_json(&[kind, "list"])
            .await
            .unwrap_or(serde_json::Value::Array(Vec::new()));
        for item in loaded.as_array().into_iter().flatten() {
            let Some(name) = item.get("name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let display = item
                .get("displayName")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.is_empty())
                .unwrap_or(name);
            let description = item
                .get("description")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let enabled = item.get("enabled").and_then(serde_json::Value::as_bool);
            let builtin = item
                .get("isBuiltIn")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
                || item
                    .get("sourceType")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|source| source == "Asset");
            let count = match kind {
                "plugin" => format!(
                    "{} subpackages",
                    item.get("subpackages")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0)
                ),
                _ => format!(
                    "{} tools",
                    item.get("tools")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0)
                ),
            };
            let detail = if description.is_empty() {
                count
            } else {
                format!("{description} · {count}")
            };
            rows.push(ControlPanelRow {
                id: name.to_string(),
                label: display.to_string(),
                detail,
                enabled,
                toggleable: true,
                deletable: !builtin,
                showable: true,
                ..ControlPanelRow::default()
            });
        }
        let available = self
            .run_control_core_json(&[kind, "more"])
            .await
            .unwrap_or(serde_json::Value::Array(Vec::new()));
        for item in available.as_array().into_iter().flatten() {
            let Some(name) = item.get("name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            // `plugin more` and `package more` share one bundled candidate
            // pool. Keep each panel to the candidates its own manager can
            // list afterwards, otherwise a loaded row disappears from the
            // panel it was loaded from.
            let package_kind = item
                .get("type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if !extension_candidate_belongs_to(kind, package_kind) {
                continue;
            }
            let description = item
                .get("description")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let scope = item
                .get("requiredInstallationScope")
                .and_then(serde_json::Value::as_str)
                .map(|scope| format!(" · scope: {scope}"))
                .unwrap_or_default();
            rows.push(ControlPanelRow {
                id: name.to_string(),
                label: name.to_string(),
                detail: format!("{description}{scope}"),
                loadable: true,
                showable: true,
                ..ControlPanelRow::default()
            });
        }
        let issues = self
            .run_control_core_json(&["package", "issues"])
            .await
            .unwrap_or(serde_json::Value::Array(Vec::new()));
        for (index, issue) in issues.as_array().into_iter().flatten().enumerate() {
            let package_kind = issue
                .get("packageKind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let source_path = issue
                .get("sourcePath")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if !extension_issue_belongs_to(kind, package_kind, source_path) {
                continue;
            }
            let display = issue
                .get("displayName")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("package");
            let code = issue
                .get("code")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let message = issue
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            rows.push(ControlPanelRow {
                id: format!("issue-{index}"),
                label: display.to_string(),
                detail: format!("{} · {} · {message}", code, package_kind),
                issue: true,
                deletable: true,
                showable: true,
                package_kind: Some(package_kind.to_string()),
                package_name: issue
                    .get("packageName")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                code: Some(code.to_string()),
                source_path: issue
                    .get("sourcePath")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                ..ControlPanelRow::default()
            });
        }
        Ok(rows)
    }

    async fn load_skill_panel(&mut self) -> Result<ControlPanelData, String> {
        let mut rows = Vec::new();
        let listed = self
            .run_control_core_json(&["skill", "list"])
            .await
            .unwrap_or(serde_json::Value::Null);
        for item in listed
            .get("skills")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(name) = item.get("name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let description = item
                .get("description")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            rows.push(ControlPanelRow {
                id: name.to_string(),
                label: name.to_string(),
                detail: description.to_string(),
                deletable: true,
                showable: true,
                ..ControlPanelRow::default()
            });
        }
        for error in listed
            .get("loadErrors")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let label = error
                .get("name")
                .and_then(serde_json::Value::as_str)
                .or_else(|| error.as_str())
                .unwrap_or("skill error");
            let detail = error
                .get("message")
                .and_then(serde_json::Value::as_str)
                .or_else(|| error.as_str())
                .unwrap_or_default();
            rows.push(ControlPanelRow {
                id: format!("error-{}", rows.len()),
                label: label.to_string(),
                detail: detail.to_string(),
                showable: true,
                ..ControlPanelRow::default()
            });
        }
        let available = self
            .run_control_core_json(&["skill", "more"])
            .await
            .unwrap_or(serde_json::Value::Array(Vec::new()));
        for item in available.as_array().into_iter().flatten() {
            let Some(name) = item.get("name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            rows.push(ControlPanelRow {
                id: name.to_string(),
                label: name.to_string(),
                detail: item
                    .get("description")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                loadable: true,
                showable: true,
                ..ControlPanelRow::default()
            });
        }
        Ok(ControlPanelData {
            rows,
            ..ControlPanelData::empty()
        })
    }

    async fn load_mcp_panel(&mut self) -> Result<ControlPanelData, String> {
        let mut rows = Vec::new();
        let listed = self
            .run_control_core_json(&["mcp", "list"])
            .await
            .unwrap_or(serde_json::Value::Array(Vec::new()));
        for item in listed.as_array().into_iter().flatten() {
            let Some(id) = item.get("id").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let name = item
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(id);
            let command = item
                .get("command")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let url = item
                .get("url")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let endpoint = if !command.is_empty() {
                command.to_string()
            } else if !url.is_empty() {
                url.to_string()
            } else {
                "remote".to_string()
            };
            let detail = item
                .get("tools")
                .and_then(serde_json::Value::as_u64)
                .map(|count| format!("{endpoint} · {count} tools"))
                .unwrap_or(endpoint);
            rows.push(ControlPanelRow {
                id: id.to_string(),
                label: name.to_string(),
                detail,
                enabled: item.get("enabled").and_then(serde_json::Value::as_bool),
                toggleable: true,
                deletable: true,
                showable: true,
                startable: true,
                killable: true,
                ..ControlPanelRow::default()
            });
        }
        Ok(ControlPanelData {
            rows,
            ..ControlPanelData::empty()
        })
    }

    fn load_identity_panel(&self) -> Result<ControlPanelData, String> {
        let state = self.networkControl.deviceSpaceControl()?;
        let mut roles = state.roles.values().collect::<Vec<_>>();
        roles.sort_by(|left, right| left.displayName.cmp(&right.displayName));
        let mut rows = Vec::new();
        for role in roles {
            let builtin = matches!(role.roleId.as_str(), "admin" | "user" | "relay" | "edge");
            rows.push(ControlPanelRow {
                id: role.roleId.clone(),
                label: role.displayName.clone(),
                detail: format!(
                    "{}{}",
                    network_role_summary(role),
                    if builtin { " · built-in" } else { "" }
                ),
                enabled: None,
                deletable: !builtin,
                editable: !builtin,
                showable: true,
                ..ControlPanelRow::default()
            });
        }
        Ok(ControlPanelData {
            rows,
            ..ControlPanelData::empty()
        })
    }

    async fn run_control_core_json(&mut self, args: &[&str]) -> Result<serde_json::Value, String> {
        let mut command = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        command.push("--json".to_string());
        let output = self
            .core
            .runCoreCommand(&command)
            .await
            .map_err(|error| error.to_string())?;
        let text = output.stdout.trim();
        if text.is_empty() {
            return Err(if output.stderr.trim().is_empty() {
                format!("{} returned no output", args.join(" "))
            } else {
                output.stderr.trim().to_string()
            });
        }
        serde_json::from_str(text)
            .map_err(|error| format!("invalid JSON from {}: {error}", args.join(" ")))
    }

    async fn run_control_core_text(&mut self, args: &[&str]) -> Result<Vec<String>, String> {
        let command = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        let output = self
            .core
            .runCoreCommand(&command)
            .await
            .map_err(|error| error.to_string())?;
        let mut lines = output
            .stdout
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !output.stderr.trim().is_empty() {
            lines.push(output.stderr.trim_end().to_string());
        }
        Ok(lines)
    }

    async fn handle_control_panel_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let Some(mode) = self.control_panel.as_ref().map(|panel| panel.mode) else {
            return Ok(());
        };
        match mode {
            ControlPanelMode::Confirm => self.handle_control_panel_confirm_key(key).await,
            ControlPanelMode::ImportInput
            | ControlPanelMode::IdentityNameInput
            | ControlPanelMode::IdentityCapsInput => {
                self.handle_control_panel_input_key(key).await;
                Ok(())
            }
            ControlPanelMode::Detail => {
                self.handle_control_panel_detail_key(key);
                Ok(())
            }
            ControlPanelMode::Browse => self.handle_control_panel_browse_key(key).await,
        }
    }

    async fn handle_control_panel_browse_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let tab = self
            .control_panel
            .as_ref()
            .map(|panel| panel.tab)
            .unwrap_or(ControlPanelTab::Status);
        let row_count = self
            .control_panel
            .as_ref()
            .map(|panel| panel.rows.len())
            .unwrap_or(0);
        match key.code {
            KeyCode::Tab | KeyCode::BackTab => {
                let forward = key.code == KeyCode::Tab;
                let current = tab.index();
                let next = if forward {
                    (current + 1) % ControlPanelTab::ALL.len()
                } else {
                    (current + ControlPanelTab::ALL.len() - 1) % ControlPanelTab::ALL.len()
                };
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.tab = ControlPanelTab::ALL[next];
                    panel.selected = 0;
                    panel.mode = ControlPanelMode::Browse;
                    panel.detail_scroll = 0;
                }
                self.refresh_control_panel().await;
            }
            KeyCode::Up => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.selected = panel.selected.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if let Some(panel) = self.control_panel.as_mut() {
                    if panel.selected + 1 < row_count {
                        panel.selected += 1;
                    }
                }
            }
            KeyCode::PageUp => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.selected = panel.selected.saturating_sub(10);
                }
            }
            KeyCode::PageDown => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.selected = (panel.selected + 10).min(row_count.saturating_sub(1));
                }
            }
            KeyCode::Home => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.selected = 0;
                }
            }
            KeyCode::End => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.selected = row_count.saturating_sub(1);
                }
            }
            KeyCode::Enter => match tab {
                ControlPanelTab::Status | ControlPanelTab::Logs => {
                    if let Some(panel) = self.control_panel.as_mut() {
                        panel.mode = ControlPanelMode::Detail;
                        panel.detail_scroll = 0;
                    }
                }
                _ => {
                    let row = self
                        .control_panel
                        .as_ref()
                        .and_then(|panel| panel.selected_row().cloned());
                    if let Some(row) = row {
                        if row.loadable {
                            self.load_control_row(&row).await;
                        } else if row.showable {
                            self.show_control_row(&row).await;
                        }
                    }
                }
            },
            KeyCode::Char('e') | KeyCode::Char('E') => {
                let row = self
                    .control_panel
                    .as_ref()
                    .and_then(|panel| panel.selected_row().cloned());
                if let Some(row) = row {
                    if tab == ControlPanelTab::Identities && row.editable {
                        self.begin_control_identity_edit(&row);
                    } else if row.toggleable {
                        self.toggle_control_row(&row).await;
                    }
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                let row = self
                    .control_panel
                    .as_ref()
                    .and_then(|panel| panel.selected_row().cloned());
                if let Some(row) = row {
                    if row.deletable {
                        let label = if tab == ControlPanelTab::Identities {
                            self.text().control_delete_identity().to_string()
                        } else {
                            self.text().control_delete_row().to_string()
                        };
                        if let Some(panel) = self.control_panel.as_mut() {
                            panel.pending = if tab == ControlPanelTab::Identities {
                                ControlPanelPending::DeleteIdentity
                            } else {
                                ControlPanelPending::DeleteRow
                            };
                            panel.pending_row = Some(row);
                            panel.confirm_label = label;
                            panel.mode = ControlPanelMode::Confirm;
                        }
                    }
                }
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                if tab == ControlPanelTab::Packages || tab == ControlPanelTab::Plugins {
                    let label = self.text().control_import_label().to_string();
                    if let Some(panel) = self.control_panel.as_mut() {
                        panel.input_label = label;
                        panel.input.clear();
                        panel.mode = ControlPanelMode::ImportInput;
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if tab == ControlPanelTab::Identities {
                    self.begin_control_identity_create();
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                let row = self
                    .control_panel
                    .as_ref()
                    .and_then(|panel| panel.selected_row().cloned());
                if let Some(row) = row.filter(|row| row.startable) {
                    self.run_control_action(&["mcp", "start", &row.id]).await;
                }
            }
            KeyCode::Char('k') | KeyCode::Char('K') => {
                let row = self
                    .control_panel
                    .as_ref()
                    .and_then(|panel| panel.selected_row().cloned());
                if let Some(row) = row.filter(|row| row.killable) {
                    self.run_control_action(&["mcp", "kill", &row.id]).await;
                }
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                if tab == ControlPanelTab::Logs {
                    let label = self.text().control_clear_log().to_string();
                    if let Some(panel) = self.control_panel.as_mut() {
                        panel.pending = ControlPanelPending::ClearLog;
                        panel.confirm_label = label;
                        panel.mode = ControlPanelMode::Confirm;
                    }
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.refresh_control_panel().await;
            }
            KeyCode::Esc => {
                self.control_panel = None;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_control_panel_detail_key(&mut self, key: KeyEvent) {
        let height = self
            .control_panel
            .as_ref()
            .map(|panel| panel.detail_lines.len().min(u16::MAX as usize) as u16)
            .unwrap_or(0);
        match key.code {
            KeyCode::Up => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.detail_scroll = panel.detail_scroll.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.detail_scroll = (panel.detail_scroll + 1).min(height.saturating_sub(1));
                }
            }
            KeyCode::PageUp => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.detail_scroll = panel.detail_scroll.saturating_sub(20);
                }
            }
            KeyCode::PageDown => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.detail_scroll = (panel.detail_scroll + 20).min(height.saturating_sub(1));
                }
            }
            KeyCode::Home => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.detail_scroll = 0;
                }
            }
            KeyCode::End => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.detail_scroll = height.saturating_sub(1);
                }
            }
            KeyCode::Esc | KeyCode::Enter => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.mode = ControlPanelMode::Browse;
                }
            }
            _ => {}
        }
    }

    async fn handle_control_panel_confirm_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let Some((pending, row)) = self
            .control_panel
            .as_ref()
            .map(|panel| (panel.pending, panel.pending_row.clone()))
        else {
            return Ok(());
        };
        let confirmed = matches!(
            key.code,
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Char('1')
        );
        let cancelled = matches!(
            key.code,
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('2') | KeyCode::Esc
        );
        if cancelled {
            if let Some(panel) = self.control_panel.as_mut() {
                panel.mode = ControlPanelMode::Browse;
                panel.pending = ControlPanelPending::None;
                panel.pending_row = None;
            }
            return Ok(());
        }
        if !confirmed {
            return Ok(());
        }
        if let Some(panel) = self.control_panel.as_mut() {
            panel.mode = ControlPanelMode::Browse;
            panel.pending = ControlPanelPending::None;
            panel.pending_row = None;
        }
        match pending {
            ControlPanelPending::DeleteRow => {
                if let Some(row) = row {
                    self.delete_control_row(&row).await;
                }
            }
            ControlPanelPending::ClearLog => {
                AppLogger::reset_log_file();
                self.refresh_control_panel().await;
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.status = Some("log cleared".to_string());
                }
            }
            ControlPanelPending::DeleteIdentity => {
                if let Some(row) = row {
                    match self.networkControl.removeDeviceSpaceRole(row.id.clone()) {
                        Ok(()) => {
                            self.refresh_control_panel().await;
                            if let Some(panel) = self.control_panel.as_mut() {
                                panel.status = Some("identity removed".to_string());
                            }
                        }
                        Err(error) => {
                            if let Some(panel) = self.control_panel.as_mut() {
                                panel.status = Some(error);
                            }
                        }
                    }
                }
            }
            ControlPanelPending::None => {}
        }
        Ok(())
    }

    async fn handle_control_panel_input_key(&mut self, key: KeyEvent) {
        let Some(mode) = self.control_panel.as_ref().map(|panel| panel.mode) else {
            return;
        };
        match key.code {
            KeyCode::Esc => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.mode = ControlPanelMode::Browse;
                    panel.input.clear();
                }
            }
            KeyCode::Backspace => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.input.pop();
                }
            }
            KeyCode::Char(ch)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.input.push(ch);
                }
            }
            KeyCode::Enter => match mode {
                ControlPanelMode::ImportInput => {
                    let Some(path) = self
                        .control_panel
                        .as_ref()
                        .map(|panel| panel.input.trim().to_string())
                        .filter(|value| !value.is_empty())
                    else {
                        return;
                    };
                    let kind = self
                        .control_panel
                        .as_ref()
                        .map(|panel| panel.tab)
                        .unwrap_or(ControlPanelTab::Packages);
                    let command = if kind == ControlPanelTab::Plugins {
                        "plugin"
                    } else {
                        "package"
                    };
                    if let Some(panel) = self.control_panel.as_mut() {
                        panel.mode = ControlPanelMode::Browse;
                        panel.input.clear();
                    }
                    self.run_control_action(&[command, "import", &path]).await;
                }
                ControlPanelMode::IdentityNameInput => {
                    let Some(name) = self
                        .control_panel
                        .as_ref()
                        .map(|panel| panel.input.trim().to_string())
                        .filter(|value| !value.is_empty())
                    else {
                        return;
                    };
                    let label = self.text().control_identity_caps_label().to_string();
                    if let Some(panel) = self.control_panel.as_mut() {
                        panel.identity_name = name;
                        panel.input_label = label;
                        panel.input = panel.identity_caps.clone();
                        panel.mode = ControlPanelMode::IdentityCapsInput;
                    }
                }
                ControlPanelMode::IdentityCapsInput => {
                    self.save_control_identity().await;
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn begin_control_identity_edit(&mut self, row: &ControlPanelRow) {
        let Ok(state) = self.networkControl.deviceSpaceControl() else {
            return;
        };
        let Some(role) = state.roles.get(&row.id) else {
            return;
        };
        let capabilities = role
            .capabilities
            .iter()
            .map(|capability| network_capability_short(capability))
            .collect::<Vec<_>>()
            .join(", ");
        let label = self.text().control_identity_name_label().to_string();
        if let Some(panel) = self.control_panel.as_mut() {
            panel.identity_role_id = Some(role.roleId.clone());
            panel.identity_name = role.displayName.clone();
            panel.identity_caps = capabilities.clone();
            panel.input_label = label;
            panel.input = role.displayName.clone();
            panel.mode = ControlPanelMode::IdentityNameInput;
        }
    }

    fn begin_control_identity_create(&mut self) {
        let label = self.text().control_identity_name_label().to_string();
        if let Some(panel) = self.control_panel.as_mut() {
            panel.identity_role_id = None;
            panel.identity_name.clear();
            panel.identity_caps.clear();
            panel.input_label = label;
            panel.input.clear();
            panel.mode = ControlPanelMode::IdentityNameInput;
        }
    }

    async fn save_control_identity(&mut self) {
        let Some((role_id, name, caps)) = self.control_panel.as_ref().map(|panel| {
            (
                panel.identity_role_id.clone(),
                panel.identity_name.trim().to_string(),
                panel.input.trim().to_string(),
            )
        }) else {
            return;
        };
        if name.is_empty() {
            if let Some(panel) = self.control_panel.as_mut() {
                panel.status = Some("identity name must not be empty".to_string());
            }
            return;
        }
        let values = caps
            .split(|character: char| character == ',' || character.is_whitespace())
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        let capabilities = match network_capabilities(&values) {
            Ok(capabilities) => capabilities,
            Err(error) => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.status = Some(error);
                }
                return;
            }
        };
        let role = NetworkControlRole {
            roleId: role_id
                .clone()
                .unwrap_or_else(|| new_network_control_id("identity")),
            displayName: name,
            capabilities,
        };
        let result = match role_id {
            Some(_) => self.networkControl.updateDeviceSpaceRole(role),
            None => self.networkControl.defineDeviceSpaceRole(role),
        };
        match result {
            Ok(()) => {
                self.refresh_control_panel().await;
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.mode = ControlPanelMode::Browse;
                    panel.pending_row = None;
                    panel.status = Some("identity saved".to_string());
                }
            }
            Err(error) => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.status = Some(error);
                }
            }
        }
    }

    async fn toggle_control_row(&mut self, row: &ControlPanelRow) {
        let tab = self
            .control_panel
            .as_ref()
            .map(|panel| panel.tab)
            .unwrap_or(ControlPanelTab::Packages);
        let command = match tab {
            ControlPanelTab::Packages => "package",
            ControlPanelTab::Plugins => "plugin",
            ControlPanelTab::Mcp => "mcp",
            _ => return,
        };
        let action = if row.enabled == Some(true) {
            "disable"
        } else {
            "enable"
        };
        self.run_control_action(&[command, action, &row.id]).await;
    }

    async fn load_control_row(&mut self, row: &ControlPanelRow) {
        let tab = self
            .control_panel
            .as_ref()
            .map(|panel| panel.tab)
            .unwrap_or(ControlPanelTab::Packages);
        let command = match tab {
            ControlPanelTab::Packages => "package",
            ControlPanelTab::Plugins => "plugin",
            ControlPanelTab::Skills => "skill",
            _ => return,
        };
        self.run_control_action(&[command, "load", &row.id]).await;
    }

    async fn delete_control_row(&mut self, row: &ControlPanelRow) {
        if row.issue {
            let Some(source_path) = row.source_path.clone() else {
                return;
            };
            let package_kind = row.package_kind.clone().unwrap_or_default();
            let code = row.code.clone().unwrap_or_default();
            let mut args = vec![
                "package".to_string(),
                "delete-issue".to_string(),
                source_path,
                package_kind,
                code,
            ];
            if let Some(package_name) = &row.package_name {
                args.push(package_name.clone());
            }
            let args_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
            self.run_control_action(&args_refs).await;
            return;
        }
        let tab = self
            .control_panel
            .as_ref()
            .map(|panel| panel.tab)
            .unwrap_or(ControlPanelTab::Packages);
        let (command, action) = match tab {
            ControlPanelTab::Packages => ("package", "delete"),
            ControlPanelTab::Plugins => ("plugin", "delete"),
            ControlPanelTab::Skills => ("skill", "delete"),
            ControlPanelTab::Mcp => ("mcp", "remove"),
            _ => return,
        };
        self.run_control_action(&[command, action, &row.id]).await;
    }

    async fn show_control_row(&mut self, row: &ControlPanelRow) {
        let tab = self
            .control_panel
            .as_ref()
            .map(|panel| panel.tab)
            .unwrap_or(ControlPanelTab::Packages);
        let mut title = row.label.clone();
        let mut lines = Vec::new();
        if tab == ControlPanelTab::Identities {
            if let Ok(state) = self.networkControl.deviceSpaceControl() {
                if let Some(role) = state.roles.get(&row.id) {
                    title = role.displayName.clone();
                    lines.push(format!("role: {}", role.roleId));
                    for capability in &role.capabilities {
                        lines.push(format!("- {capability}"));
                    }
                    lines.push(format!("summary: {}", network_role_summary(role)));
                }
            }
        } else if row.issue {
            lines.push(format!(
                "source: {}",
                row.source_path.as_deref().unwrap_or("")
            ));
            lines.push(format!(
                "kind: {}",
                row.package_kind.as_deref().unwrap_or("")
            ));
            lines.push(format!("code: {}", row.code.as_deref().unwrap_or("")));
            lines.push(format!(
                "package: {}",
                row.package_name.as_deref().unwrap_or("")
            ));
            lines.push(row.detail.clone());
        } else if row.id.starts_with("error-") {
            lines.push(row.detail.clone());
        } else {
            let command = match tab {
                ControlPanelTab::Packages => Some("package"),
                ControlPanelTab::Plugins => Some("plugin"),
                ControlPanelTab::Skills => Some("skill"),
                ControlPanelTab::Mcp => Some("mcp"),
                _ => None,
            };
            if let Some(command) = command {
                match self
                    .run_control_core_text(&[command, "show", &row.id])
                    .await
                {
                    Ok(loaded) => {
                        if !loaded.is_empty() {
                            lines = loaded;
                        }
                    }
                    Err(error) => lines.push(error),
                }
            }
        }
        if lines.is_empty() {
            lines.push(row.detail.clone());
        }
        if let Some(panel) = self.control_panel.as_mut() {
            panel.detail_title = title;
            panel.detail_lines = lines;
            panel.detail_scroll = 0;
            panel.mode = ControlPanelMode::Detail;
        }
    }

    async fn run_control_action(&mut self, args: &[&str]) {
        match self.run_control_core_text(args).await {
            Ok(lines) => {
                self.refresh_control_panel().await;
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.status = Some(if lines.is_empty() {
                        format!("{} completed", args.join(" "))
                    } else {
                        lines.join(" · ")
                    });
                }
            }
            Err(error) => {
                if let Some(panel) = self.control_panel.as_mut() {
                    panel.status = Some(error);
                }
            }
        }
    }

    async fn handle_character_command(&mut self, args: &[String]) -> Result<(), String> {
        let default_args = vec!["list".to_string()];
        let args = if args.is_empty() || args == ["choose"] {
            &default_args
        } else {
            args
        };
        self.handle_plugin_core_command("character", args).await;
        Ok(())
    }

    async fn handle_group_command(&mut self, args: &[String]) -> Result<(), String> {
        let default_args = vec!["list".to_string()];
        let args = if args.is_empty() || args == ["choose"] {
            &default_args
        } else {
            args
        };
        self.handle_plugin_core_command("group", args).await;
        Ok(())
    }

    /// Runs one direct extension command and shows its output in a popup.
    /// Bare commands open the control panel; this path keeps scripted
    /// subcommands such as `import` and `show` usable from the TUI.
    async fn run_extension_cli_command(&mut self, family: &str, args: &[String]) {
        let mut command = vec![family.to_string()];
        command.extend_from_slice(args);
        match self.core.runCoreCommand(&command).await {
            Ok(output) => {
                let lines = output
                    .stdout
                    .lines()
                    .chain(output.stderr.lines())
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                if lines.is_empty() {
                    self.status_message = format!("{family} completed");
                } else if lines.len() > 1 {
                    self.open_list_popup(format!("/{family} {}", args.join(" ")), lines);
                } else {
                    self.status_message = lines.join("");
                }
            }
            Err(error) => self.status_message = error.to_string(),
        }
    }

    /// Toggles one extension by querying its current enabled state first.
    async fn toggle_extension_by_name(&mut self, family: &str, name: &str) -> Result<(), String> {
        let list = self.run_control_core_json(&[family, "list"]).await?;
        let enabled = list
            .as_array()
            .and_then(|items| {
                items.iter().find(|item| {
                    item.get("name")
                        .or_else(|| item.get("id"))
                        .and_then(serde_json::Value::as_str)
                        == Some(name)
                })
            })
            .and_then(|item| item.get("enabled").and_then(serde_json::Value::as_bool))
            .unwrap_or(false);
        let action = if enabled { "disable" } else { "enable" };
        let lines = self.run_control_core_text(&[family, action, name]).await?;
        self.status_message = if lines.is_empty() {
            format!("{family} {name}: {action}")
        } else {
            lines.join(" · ")
        };
        Ok(())
    }

    async fn handle_skill_command(&mut self, args: &[String]) -> Result<(), String> {
        if args.is_empty() || args == ["panel"] {
            self.open_control_panel(ControlPanelTab::Skills).await;
            return Ok(());
        }
        if args.first().map(String::as_str) == Some("toggle") {
            self.status_message =
                "skills have no enable state; use /skill to load or delete them".to_string();
            return Ok(());
        }
        self.run_extension_cli_command("skill", args).await;
        Ok(())
    }

    async fn handle_package_command(&mut self, args: &[String]) -> Result<(), String> {
        if args.is_empty() || args == ["panel"] {
            self.open_control_panel(ControlPanelTab::Packages).await;
            return Ok(());
        }
        if args.first().map(String::as_str) == Some("toggle") {
            let name = args
                .get(1)
                .ok_or_else(|| self.text().usage_package_toggle().to_string())?;
            return self.toggle_extension_by_name("package", name).await;
        }
        self.run_extension_cli_command("package", args).await;
        Ok(())
    }

    async fn handle_plugin_command(&mut self, args: &[String]) -> Result<(), String> {
        if args.is_empty() || args == ["panel"] {
            self.open_control_panel(ControlPanelTab::Plugins).await;
            return Ok(());
        }
        if args.first().map(String::as_str) == Some("toggle") {
            let name = args
                .get(1)
                .ok_or_else(|| self.text().usage_plugin_toggle().to_string())?;
            return self.toggle_extension_by_name("plugin", name).await;
        }
        self.run_extension_cli_command("plugin", args).await;
        Ok(())
    }

    async fn handle_mcp_command(&mut self, args: &[String]) -> Result<(), String> {
        if args.is_empty() || args == ["panel"] {
            self.open_control_panel(ControlPanelTab::Mcp).await;
            return Ok(());
        }
        if args.first().map(String::as_str) == Some("toggle") {
            let name = args
                .get(1)
                .ok_or_else(|| self.text().usage_mcp_toggle().to_string())?;
            return self.toggle_extension_by_name("mcp", name).await;
        }
        self.run_extension_cli_command("mcp", args).await;
        Ok(())
    }

    async fn handle_tag_command(&mut self, args: &[String]) -> Result<(), String> {
        let default_args = vec!["list".to_string()];
        let args = if args.is_empty() || args == ["choose"] {
            &default_args
        } else {
            args
        };
        self.handle_plugin_core_command("tag", args).await;
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
            let secondary = short_chat_label(&chat.id);
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

/// Parses the address and transport for code-confirmed manual pairing.
fn parse_pair_arguments(args: &[String]) -> Result<(String, PeerTransport), String> {
    const USAGE: &str = "usage: network pair <address> <http|ws|tcp|serial|bluetooth>";
    let [address, transport] = args else {
        return Err(USAGE.to_string());
    };
    Ok((address.clone(), parse_peer_transport(transport)?))
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

/// Localized label for a join request's lifecycle state; the hub, the joins
/// list and the decision popup all render the same word for the same state.
pub(super) fn join_status_label(text: TuiText, status: &SpaceJoinStatus) -> &'static str {
    match status {
        SpaceJoinStatus::Pending => text.network_join_status_pending(),
        SpaceJoinStatus::Approving => text.network_join_status_approving(),
        SpaceJoinStatus::Approved => text.network_join_status_approved(),
        SpaceJoinStatus::Rejected => text.network_join_status_rejected(),
        SpaceJoinStatus::Cancelled => text.network_join_status_cancelled(),
        SpaceJoinStatus::Expired => text.network_join_status_expired(),
        SpaceJoinStatus::Joined => text.network_join_status_joined(),
    }
}

/// Selectable lines for the discovery popup: one entry per candidate plus the
/// trailing select hint. Pure so the popup content is testable without a scan.
pub(super) fn discovery_popup_items(text: TuiText, candidates: &[DiscoveredPeer]) -> Vec<String> {
    let mut items = candidates
        .iter()
        .map(|peer| format!("{} · {} · {}", peer.displayName, peer.nodeId, peer.address))
        .collect::<Vec<_>>();
    items.push(text.network_discover_select_hint().to_string());
    items
}

/// Rebuilds an open join decision queue from the freshest decidable set:
/// matching requests are replaced in place (a reviewer reassignment bumps the
/// assignment version), entries the fetch no longer reports are dropped, and
/// the selected row follows its request id. Without this a retry would keep
/// presenting a stale version that the runtime rejects forever.
pub(super) fn refreshed_decision_queue(
    current: &[SpaceJoinRequest],
    fresh: &[SpaceJoinRequest],
    selected: usize,
) -> (Vec<SpaceJoinRequest>, usize) {
    let selected_id = current
        .get(selected)
        .map(|request| request.requestId.clone());
    let requests = fresh
        .iter()
        .filter(|request| request.canApprove)
        .cloned()
        .collect::<Vec<_>>();
    let selected = selected_id
        .and_then(|id| requests.iter().position(|request| request.requestId == id))
        .unwrap_or_else(|| selected.min(requests.len().saturating_sub(1)));
    (requests, selected)
}

/// Status suffix for one hub member row. A policy restriction outranks
/// reachability: a restricted device will not come back on its own, so
/// "offline; waiting for it to come back" would be a lie there.
pub(super) fn hub_member_status_label(
    text: TuiText,
    blocked: bool,
    online: bool,
) -> Option<&'static str> {
    if blocked {
        Some(text.network_hub_member_restricted())
    } else if !online {
        Some(text.network_devices_offline())
    } else {
        None
    }
}

/// Localized label for one paired device's status as listed by `/network peers`;
/// `RemovedFromSpace` is the only way a user learns a peer is paired but no
/// longer shares this Space.
pub(super) fn paired_status_label(
    text: TuiText,
    status: &RuntimePairedDeviceStatus,
) -> &'static str {
    match status {
        RuntimePairedDeviceStatus::Online => text.network_peer_status_online(),
        RuntimePairedDeviceStatus::Offline => text.network_peer_status_offline(),
        RuntimePairedDeviceStatus::Invalid => text.network_peer_status_invalid(),
        RuntimePairedDeviceStatus::RemovedFromSpace => text.network_peer_status_removed(),
    }
}

/// One-line notice for a local Space membership change: who left (or was
/// removed) and who joined. `None` when the member set is unchanged. Device
/// names fall back to node ids for members the topology no longer lists.
pub(super) fn space_membership_notice(
    text: TuiText,
    previous: &BTreeSet<String>,
    current: &BTreeSet<String>,
    name_for: &BTreeMap<String, String>,
) -> Option<String> {
    let render = |ids: &[String]| {
        ids.iter()
            .map(|id| name_for.get(id).cloned().unwrap_or_else(|| id.clone()))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let left = previous.difference(current).cloned().collect::<Vec<_>>();
    let joined = current.difference(previous).cloned().collect::<Vec<_>>();
    let mut notices = Vec::new();
    if !left.is_empty() {
        notices.push(text.network_space_members_removed(render(&left)));
    }
    if !joined.is_empty() {
        notices.push(text.network_space_members_added(render(&joined)));
    }
    if notices.is_empty() {
        None
    } else {
        Some(notices.join(" · "))
    }
}

/// Renders the doctor report as popup lines. Multi-placeholder lines go
/// through named `render` args: the generated per-key methods take their
/// placeholders in alphabetical order, which must never be confused with
/// reading order.
pub(super) fn space_health_report_lines(text: TuiText, report: &SpaceHealthReport) -> Vec<String> {
    let flag = |value: bool| {
        if value {
            text.network_doctor_yes().to_string()
        } else {
            text.network_doctor_no().to_string()
        }
    };
    let policy_state = if report.policyReadable {
        text.network_doctor_policy_readable()
    } else {
        text.network_doctor_policy_unreadable()
    };
    let mut items = vec![
        text.render(
            TuiTextKey::NetworkDoctorSpace,
            &[
                ("space_name", report.spaceName.clone()),
                ("space_id", report.spaceId.clone()),
                ("revision", report.spaceRevision.to_string()),
            ],
        ),
        text.render(
            TuiTextKey::NetworkDoctorPolicy,
            &[
                ("state", policy_state.to_string()),
                ("records", report.recordMembers.len().to_string()),
                ("replay", report.replayMembers.len().to_string()),
            ],
        ),
    ];
    for journal in &report.journals {
        items.push(text.render(
            TuiTextKey::NetworkDoctorJournal,
            &[
                ("device", journal.deviceId.clone()),
                ("lines", journal.totalLines.to_string()),
                ("decoded", journal.decodedLines.to_string()),
                ("findings", journal.findings.len().to_string()),
            ],
        ));
    }
    items.push(
        text.render(
            TuiTextKey::NetworkDoctorProjection,
            &[
                ("entries", report.projection.projectionEntries.to_string()),
                (
                    "journal_only",
                    report.projection.journalOnlyOperationIds.len().to_string(),
                ),
                (
                    "projection_only",
                    report
                        .projection
                        .projectionOnlyOperationIds
                        .len()
                        .to_string(),
                ),
            ],
        ),
    );
    for pair in &report.pairs {
        items.push(text.render(
            TuiTextKey::NetworkDoctorPeer,
            &[
                ("device", pair.deviceId.clone()),
                ("member", flag(pair.member)),
                ("online", flag(pair.online)),
                ("inbound", flag(pair.pairedInbound)),
                ("outbound", flag(pair.pairedOutbound)),
            ],
        ));
    }
    // D3 join review state: committed/recoverable/reassignable are the
    // Approving-exit signals, and only recoverable records are not findings.
    for join in &report.joins {
        items.push(
            text.render(
                TuiTextKey::NetworkDoctorJoin,
                &[
                    ("request", join.requestId.clone()),
                    ("status", join_status_label(text, &join.status).to_string()),
                    (
                        "reviewer",
                        join.reviewerDeviceId
                            .clone()
                            .unwrap_or_else(|| "-".to_string()),
                    ),
                    ("version", join.assignmentVersion.to_string()),
                    ("committed", flag(join.committed)),
                    ("recoverable", flag(join.recoverable)),
                    ("reassignable", flag(join.reassignable)),
                ],
            ),
        );
    }
    if report.findings.is_empty() {
        items.push(text.network_doctor_findings_none().to_string());
    } else {
        for finding in &report.findings {
            items.push(text.render(
                TuiTextKey::NetworkDoctorFinding,
                &[
                    ("severity", finding.severity.clone()),
                    ("check", finding.check.clone()),
                    ("summary", finding.summary.clone()),
                ],
            ));
        }
    }
    let counters = &report.counters;
    items.push(text.render(
        TuiTextKey::NetworkDoctorCounters,
        &[
            ("divergences", counters.divergencesDetected.to_string()),
            ("quarantined", counters.linesQuarantined.to_string()),
            ("repairs", counters.repairsRun.to_string()),
            ("failed", counters.repairsFailed.to_string()),
            ("reconciles", counters.reconcileExchanges.to_string()),
            ("rounds", counters.reconcileRounds.to_string()),
            ("incomplete", counters.reconcileIncomplete.to_string()),
            ("oversize", counters.oversizeOffersRefused.to_string()),
        ],
    ));
    items.push(text.network_doctor_repair_hint().to_string());
    items
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

/// One-line error text for the status line: peer failures arrive as
/// CoreLinkError text whose extra lines are a location and backtrace dump.
fn status_error_line(error: &str) -> String {
    error.lines().next().unwrap_or_default().to_string()
}

/// Display label of a finished pairing, falling back to its node id.
fn paired_peer_label(peer: &PairedPeer) -> String {
    if peer.displayName.is_empty() {
        peer.nodeId.clone()
    } else {
        peer.displayName.clone()
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

/// Returns whether one bundled candidate belongs to the extension manager
/// being browsed. Script packages are listed by `package list` and ToolPkg
/// containers by `plugin list`; mixing both in one panel would make a freshly
/// loaded row leave that panel instead of turning into a loaded entry.
fn extension_candidate_belongs_to(kind: &str, package_kind: &str) -> bool {
    let is_toolpkg = package_kind == "toolpkg";
    if kind == "plugin" {
        is_toolpkg
    } else {
        !is_toolpkg
    }
}

/// Returns whether a load failure belongs to the plugin panel.
///
/// ToolPkg parse failures stay with plugins even when an older record labeled
/// them as ordinary packages. Script and metadata failures stay with packages.
fn extension_issue_belongs_to(kind: &str, package_kind: &str, source_path: &str) -> bool {
    let is_plugin = matches!(
        package_kind,
        "toolpkg" | "market_toolpkg" | "bundled_toolpkg"
    ) || source_path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(source_path)
        .rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("toolpkg"));
    if kind == "plugin" {
        is_plugin
    } else {
        !is_plugin
    }
}

/// Reads a bounded tail from the runtime log. A headless node can run for
/// weeks, so the panel never loads the whole file just to show recent lines.
fn read_log_tail(path: &str, max_lines: usize) -> Vec<String> {
    const WINDOW_BYTES: u64 = 256 * 1024;
    if path.is_empty() {
        return Vec::new();
    }
    let Ok(mut file) = File::open(path) else {
        return Vec::new();
    };
    let length = file.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    let start = length.saturating_sub(WINDOW_BYTES);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return Vec::new();
    }
    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&bytes);
    let mut lines = text.lines().map(str::to_string).collect::<Vec<_>>();
    if start > 0 && !lines.is_empty() {
        lines.remove(0);
    }
    if lines.len() > max_lines {
        lines.drain(0..lines.len() - max_lines);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_model::MessagePart::MessagePartKind;
    use operit_model::MessagePartCodec::MessagePartCodec;
    use operit_node_runtime::RuntimeRemoteLinkService::{
        RuntimeDeviceSpaceIdentity, SpaceJoinStatus,
    };
    use operit_node_runtime::SpaceDoctor::JoinHealth;
    use std::collections::BTreeSet;

    /// Builds a device window fixture: self plus two foreign devices, one
    /// restricted, one carrying an identity, and one pending join request.
    fn device_manager_fixture() -> DeviceManagerModal {
        let device =
            |device_id: &str, online: bool, identity: Option<&str>| RuntimeDeviceSpaceDevice {
                deviceId: device_id.to_string(),
                userName: "alice".to_string(),
                deviceName: device_id.to_string(),
                platform: "linux".to_string(),
                model: String::new(),
                coreVersion: None,
                online,
                relayHops: None,
                relayPath: None,
                currentIdentity: identity.map(|display_name| RuntimeDeviceSpaceIdentity {
                    displayName: display_name.to_string(),
                    capabilities: Vec::new(),
                }),
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
            paired: BTreeSet::new(),
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
        assert!(rows[1..]
            .iter()
            .all(|row| matches!(row, DeviceManagerRow::Device(_))));
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

    /// Direct-link pairing is link-level: it leads the menu for a member
    /// without a local pairing, and disappears once the edge exists or the
    /// device is restricted from connections.
    #[test]
    fn device_manager_menu_offers_pairing_only_for_unpaired_members() {
        let modal = device_manager_fixture();
        let actions = modal.menu_actions("phone");
        assert_eq!(actions.first(), Some(&DeviceManagerAction::Pair));
        assert!(!modal
            .menu_actions("tablet")
            .contains(&DeviceManagerAction::Pair));

        let mut paired = device_manager_fixture();
        paired.paired = ["phone".to_string()].into_iter().collect();
        assert!(!paired
            .menu_actions("phone")
            .contains(&DeviceManagerAction::Pair));
    }

    /// Identity actions appear only when they can apply, and only for
    /// foreign devices: assignment needs a defined role, clearing needs an
    /// assigned identity.
    #[test]
    fn device_manager_menu_offers_identity_actions_only_when_applicable() {
        let mut modal = device_manager_fixture();
        assert!(!modal
            .menu_actions("phone")
            .contains(&DeviceManagerAction::AssignIdentity));
        assert!(modal
            .menu_actions("phone")
            .contains(&DeviceManagerAction::ClearIdentity));
        assert!(!modal
            .menu_actions("tablet")
            .contains(&DeviceManagerAction::ClearIdentity));
        modal.roles.insert(
            "role-1".to_string(),
            NetworkControlRole {
                roleId: "role-1".to_string(),
                displayName: "user".to_string(),
                capabilities: BTreeSet::new(),
            },
        );
        assert!(modal
            .menu_actions("tablet")
            .contains(&DeviceManagerAction::AssignIdentity));
        // The local row stays read-only even with roles defined.
        assert!(modal.menu_actions("self").is_empty());
    }

    /// Verifies generic creation input is preserved without frontend domain interpretation.
    #[test]
    fn new_chat_args_translate_keywords_into_shell_flags() {
        let args = OperitTui::parse_new_chat_args(&[
            "source".to_string(),
            "chat-parent".to_string(),
            "input".to_string(),
            r#"{"plugin.example":{"selection":"opaque"}}"#.to_string(),
        ])
        .expect("generic keyword options must translate");
        assert_eq!(args.sourceChatId.as_deref(), Some("chat-parent"));
        assert_eq!(args.input.unwrap()["plugin.example"]["selection"], "opaque");
    }

    #[test]
    fn new_chat_args_accept_flags_and_reject_retired_role_options() {
        assert!(OperitTui::parse_new_chat_args(&["--source".into(), "parent".into()]).is_ok());
        assert!(OperitTui::parse_new_chat_args(&["character".into(), "Alice".into()]).is_err());
        assert!(OperitTui::parse_new_chat_args(&["chat".into(), "x".into()]).is_err());
        assert!(OperitTui::parse_new_chat_args(&["input".into()]).is_err());
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

    /// Verifies manual pairing accepts only its address and transport.
    #[test]
    fn parse_pair_arguments_reads_address_and_transport() {
        let args = |values: &[&str]| {
            values
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
        };
        let (address, transport) =
            parse_pair_arguments(&args(&["192.168.1.8:4835", "ws"])).unwrap();
        assert_eq!(address, "192.168.1.8:4835");
        assert_eq!(transport, PeerTransport::WebSocket);
        assert!(parse_pair_arguments(&args(&["192.168.1.8:4835"])).is_err());
        assert!(
            parse_pair_arguments(&args(&["192.168.1.8:4835", "ws", "--token", "secret"])).is_err()
        );
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
                pairingIncarnation: None,
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

    #[test]
    fn status_error_line_keeps_only_the_message_from_diagnostics() {
        assert_eq!(
            status_error_line(
                "INTERNAL_ERROR: An active join request already exists\nRust error location: core/src/ops/function.rs:250:5\nRust backtrace:\n   0: 0x1 - <unknown>"
            ),
            "INTERNAL_ERROR: An active join request already exists"
        );
        assert_eq!(status_error_line("plain failure"), "plain failure");
        assert_eq!(status_error_line(""), "");
    }

    fn hub_topology_fixture() -> RuntimeDeviceSpaceTopology {
        let device = |device_id: &str, online: bool| RuntimeDeviceSpaceDevice {
            deviceId: device_id.to_string(),
            userName: "alice".to_string(),
            deviceName: device_id.to_string(),
            platform: "linux".to_string(),
            model: String::new(),
            coreVersion: None,
            online,
            relayHops: None,
            relayPath: None,
            currentIdentity: None,
        };
        RuntimeDeviceSpaceTopology {
            currentDeviceId: "self".to_string(),
            devices: vec![device("self", true), device("phone", true)],
            connections: Vec::new(),
        }
    }

    fn hub_paired_fixture() -> BTreeMap<String, RuntimePairedDevice> {
        let peer = |device_id: &str, model: &str, outbound: bool| {
            (
                device_id.to_string(),
                RuntimePairedDevice {
                    deviceId: device_id.to_string(),
                    deviceInfo: operit_link::protocol::LinkDeviceInfo {
                        platform: String::new(),
                        model: model.to_string(),
                        pairingIncarnation: None,
                    },
                    inbound: true,
                    outbound,
                },
            )
        };
        BTreeMap::from([
            peer("phone", "phone", true),
            peer("server", "server", true),
            peer("watch", "watch", false),
        ])
    }

    #[test]
    fn network_hub_rows_list_members_then_paired_non_members() {
        let topology = hub_topology_fixture();
        let paired = hub_paired_fixture();
        let rows = network_hub_rows(&topology, &paired);

        let ids = rows
            .iter()
            .map(|row| row.id().to_string())
            .collect::<Vec<_>>();
        // Members come first in topology order; paired non-members follow in
        // stable id order; an existing member never appears twice. The watch
        // holds only the inbound credential: it still gets a joinable row,
        // because the pairing direction never gates the hub.
        assert_eq!(ids, vec!["self", "phone", "server", "watch"]);
        assert!(matches!(&rows[0], NetworkHubRow::Member(_)));
        assert!(matches!(&rows[2], NetworkHubRow::Peer { .. }));
        assert!(matches!(&rows[3], NetworkHubRow::Peer { .. }));
        if let NetworkHubRow::Peer { label, .. } = &rows[2] {
            assert_eq!(label, "server");
        } else {
            panic!("third hub row must be a paired non-member");
        }
    }

    #[test]
    fn pair_code_push_accepts_digits_only_up_to_six() {
        assert_eq!(pair_code_push("", '4'), "4");
        assert_eq!(pair_code_push("123", '4'), "1234");
        assert_eq!(pair_code_push("123456", '7'), "123456");
        assert_eq!(pair_code_push("123", 'a'), "123");
        assert_eq!(pair_code_push("123", '*'), "123");
    }

    #[test]
    fn peer_transport_label_matches_listener_spellings() {
        let round_trip = |label: &str, expected: PeerTransport| {
            let parsed = parse_peer_transport(label).expect("listener spelling must parse back");
            assert!(
                std::mem::discriminant(&parsed) == std::mem::discriminant(&expected),
                "{label} must round-trip"
            );
        };
        round_trip("http", PeerTransport::Http);
        round_trip("ws", PeerTransport::WebSocket);
        round_trip("tcp", PeerTransport::Tcp);
        round_trip("serial", PeerTransport::Serial);
        round_trip("bluetooth", PeerTransport::Bluetooth);
        assert_eq!(peer_transport_label(&PeerTransport::Tcp), "tcp");
    }

    /// Builds one join request fixture for decision-queue refresh tests.
    fn join_request_fixture(id: &str, version: u64, can_approve: bool) -> SpaceJoinRequest {
        SpaceJoinRequest {
            requestId: id.to_string(),
            targetDeviceId: "self".to_string(),
            applicantDeviceId: format!("applicant-{id}"),
            applicantName: format!("name-{id}"),
            spaceName: "Space".to_string(),
            status: SpaceJoinStatus::Pending,
            createdAt: 0,
            expiresAt: i64::MAX,
            canApprove: can_approve,
            reviewerDeviceId: Some("self".to_string()),
            reviewerName: None,
            reviewerHops: None,
            assignmentVersion: version,
            decisionApprove: None,
        }
    }

    /// A reviewer reassignment bumps the assignment version; the refreshed
    /// queue must present the new version, because the runtime rejects a
    /// decision on the cached one forever.
    #[test]
    fn decision_queue_refresh_replaces_stale_versions() {
        let current = vec![
            join_request_fixture("req", 1, true),
            join_request_fixture("gone", 1, true),
        ];
        let fresh = vec![join_request_fixture("req", 2, true)];
        let (requests, selected) = refreshed_decision_queue(&current, &fresh, 0);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].requestId, "req");
        assert_eq!(requests[0].assignmentVersion, 2);
        assert_eq!(selected, 0);
    }

    /// The selected row follows its request id across the refresh, withdrawn
    /// or no-longer-decidable requests disappear, and an empty result lets
    /// the popup close instead of holding a ghost entry.
    #[test]
    fn decision_queue_refresh_keeps_selection_and_drops_withdrawn() {
        let current = vec![
            join_request_fixture("a", 1, true),
            join_request_fixture("b", 1, true),
        ];
        let fresh = vec![
            join_request_fixture("b", 3, true),
            join_request_fixture("c", 1, true),
            join_request_fixture("not-mine", 1, false),
        ];
        let (requests, selected) = refreshed_decision_queue(&current, &fresh, 1);
        let ids = requests
            .iter()
            .map(|request| request.requestId.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, ["b", "c"]);
        assert_eq!(selected, 0);

        let (empty, selected) = refreshed_decision_queue(&current, &[], 1);
        assert!(empty.is_empty());
        assert_eq!(selected, 0);
    }

    /// The discovery popup lists every candidate and keeps the select hint
    /// as the trailing (unselectable by count) line.
    #[test]
    fn discovery_items_end_with_the_select_hint() {
        let text = TuiLanguage::English.text();
        let candidates = vec![
            DiscoveredPeer {
                nodeId: "n1".to_string(),
                address: "10.0.0.2:8080".to_string(),
                displayName: "phone".to_string(),
            },
            DiscoveredPeer {
                nodeId: "n2".to_string(),
                address: "10.0.0.3:8080".to_string(),
                displayName: "tablet".to_string(),
            },
        ];
        let items = discovery_popup_items(text, &candidates);
        assert_eq!(items.len(), 3);
        assert!(items[0].contains("phone") && items[0].contains("n1"));
        assert!(items[1].contains("tablet") && items[1].contains("n2"));
        assert_eq!(items[2], text.network_discover_select_hint());
    }

    /// The health report renders the D3 signals that carry the Approving
    /// exit - committed/recoverable/reassignable - plus the reconciliation
    /// counters, so the TUI can observe the new core behavior.
    #[test]
    fn doctor_lines_report_join_health_and_counters() {
        let text = TuiLanguage::English.text();
        let report = SpaceHealthReport {
            localNodeId: "self".to_string(),
            spaceId: "space-1".to_string(),
            spaceName: "Home".to_string(),
            spaceRevision: 7,
            policyReadable: true,
            policyError: None,
            recordMembers: vec!["self".to_string()],
            replayMembers: vec!["self".to_string(), "phone".to_string()],
            pairs: Vec::new(),
            joins: vec![JoinHealth {
                requestId: "req-1".to_string(),
                applicantDeviceId: "phone".to_string(),
                status: SpaceJoinStatus::Approving,
                reviewerDeviceId: Some("tab".to_string()),
                assignmentVersion: 2,
                approvedDecision: Some(true),
                committed: true,
                recoverable: true,
                reassignable: false,
            }],
            journals: Vec::new(),
            projection: Default::default(),
            clockAnomalies: Vec::new(),
            findings: Vec::new(),
            counters: Default::default(),
            repair: None,
        };
        let lines = space_health_report_lines(text, &report);
        let joined = lines.join("\n");
        assert!(
            joined.contains("Home"),
            "space identity must render: {joined}"
        );
        assert!(
            joined.contains("readable"),
            "policy state must render: {joined}"
        );
        assert!(
            joined.contains("req-1") && joined.contains("recoverable=yes"),
            "join health must render: {joined}"
        );
        assert!(
            joined.contains("reconciles=0"),
            "counters must render: {joined}"
        );
        assert!(
            joined.contains("Findings: none"),
            "findings must render: {joined}"
        );
        assert!(
            joined.contains("--repair"),
            "the CLI repair hint must render: {joined}"
        );
    }

    /// The hub status tag must call out a policy restriction instead of
    /// pretending the device is merely offline: it will not return by itself.
    #[test]
    fn hub_status_prefers_restricted_over_offline() {
        let text = TuiLanguage::English.text();
        assert_eq!(
            hub_member_status_label(text, true, true),
            Some(text.network_hub_member_restricted())
        );
        assert_eq!(
            hub_member_status_label(text, false, false),
            Some(text.network_devices_offline())
        );
        assert_eq!(hub_member_status_label(text, false, true), None);
    }

    /// `/network peers` names every status, including the pairing-only case
    /// where the peer is no longer in this Space.
    #[test]
    fn paired_status_labels_cover_every_state() {
        let text = TuiLanguage::English.text();
        assert_eq!(
            paired_status_label(text, &RuntimePairedDeviceStatus::Online),
            text.network_peer_status_online()
        );
        assert_eq!(
            paired_status_label(text, &RuntimePairedDeviceStatus::Offline),
            text.network_peer_status_offline()
        );
        assert_eq!(
            paired_status_label(text, &RuntimePairedDeviceStatus::Invalid),
            text.network_peer_status_invalid()
        );
        assert_eq!(
            paired_status_label(text, &RuntimePairedDeviceStatus::RemovedFromSpace),
            text.network_peer_status_removed()
        );
    }

    /// A membership change names the devices that left and joined; a departed
    /// device keeps its node id because the topology no longer lists it.
    #[test]
    fn runtime_log_tail_is_bounded_to_recent_lines() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!(
            "operit-log-tail-{}-{unique}.log",
            std::process::id()
        ));
        let content = (0..200)
            .map(|index| format!("line-{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, content).expect("write log fixture");
        let tail = read_log_tail(path.to_string_lossy().as_ref(), 5);
        assert_eq!(
            tail,
            vec![
                "line-195".to_string(),
                "line-196".to_string(),
                "line-197".to_string(),
                "line-198".to_string(),
                "line-199".to_string(),
            ]
        );
        assert!(read_log_tail("", 5).is_empty());
        let _ = std::fs::remove_file(path);
    }

    /// Regression: `openai_draw` is a script candidate, so loading it from the
    /// Plugins tab used to remove the row without adding a loaded plugin entry.
    #[test]
    fn extension_candidates_are_split_by_manager_kind() {
        assert!(extension_candidate_belongs_to("plugin", "toolpkg"));
        assert!(!extension_candidate_belongs_to("plugin", "script"));
        assert!(extension_candidate_belongs_to("package", "script"));
        assert!(!extension_candidate_belongs_to("package", "toolpkg"));
        // Unknown kinds stay reachable through the package manager.
        assert!(extension_candidate_belongs_to("package", ""));
        assert!(!extension_candidate_belongs_to("plugin", ""));
    }

    /// Plugin archives and their parse failures stay out of the package panel.
    #[test]
    fn extension_issues_stay_with_their_manager() {
        assert!(extension_issue_belongs_to(
            "plugin",
            "toolpkg",
            "demo.toolpkg"
        ));
        assert!(extension_issue_belongs_to(
            "plugin",
            "market_toolpkg",
            "market.toolpkg"
        ));
        assert!(extension_issue_belongs_to(
            "plugin",
            "bundled_toolpkg",
            "bundled.toolpkg"
        ));
        assert!(extension_issue_belongs_to(
            "plugin",
            "package",
            r"C:\packages\Broken.TOOLPKG"
        ));
        assert!(!extension_issue_belongs_to(
            "package",
            "toolpkg",
            "demo.toolpkg"
        ));
        assert!(!extension_issue_belongs_to(
            "package",
            "package",
            "/packages/broken.toolpkg"
        ));
        assert!(extension_issue_belongs_to(
            "package",
            "javascript",
            "time.js"
        ));
        assert!(extension_issue_belongs_to(
            "package",
            "metadata",
            "tools.hjson"
        ));
        assert!(extension_issue_belongs_to(
            "package",
            "package_manager",
            "runtime://package-manager"
        ));
        assert!(!extension_issue_belongs_to(
            "plugin",
            "javascript",
            "time.js"
        ));
    }

    /// The bind address editor must round-trip IPv4 and bracketed IPv6
    /// without corrupting the socket address the listener expects.
    #[test]
    fn bind_address_round_trips_ipv4_and_ipv6() {
        assert_eq!(
            split_bind_address("0.0.0.0:37195"),
            ("0.0.0.0".to_string(), "37195".to_string())
        );
        assert_eq!(
            split_bind_address("[::]:37195"),
            ("::".to_string(), "37195".to_string())
        );
        assert_eq!(
            split_bind_address("[fe80::1]:0"),
            ("fe80::1".to_string(), "0".to_string())
        );
        assert_eq!(compose_bind_address("::", "37195"), "[::]:37195");
        assert_eq!(compose_bind_address("127.0.0.1", "0"), "127.0.0.1:0");
    }

    /// Listener save applies the desktop rules before touching the runtime:
    /// automatic mode owns port 0, TCP cannot share a port with HTTP/WS,
    /// unsupported transports are dropped, and discovery needs a transport.
    #[test]
    fn listener_plan_enforces_desktop_rules() {
        let text = TuiLanguage::English.text();
        let supported = PAIR_WIZARD_TRANSPORTS.to_vec();
        let selected = |transports: &[PeerTransport]| {
            transports.iter().copied().collect::<BTreeSet<_>>()
        };
        let plan = |mode, discovery, transports: &[PeerTransport], supported: &[PeerTransport]| {
            plan_listener_settings(
                text,
                "0.0.0.0",
                if mode == PeerHostPortMode::Automatic {
                    "0"
                } else {
                    "37195"
                },
                mode,
                discovery,
                true,
                &selected(transports),
                supported,
            )
        };
        assert!(plan(
            PeerHostPortMode::Automatic,
            false,
            &[PeerTransport::Http],
            &supported
        )
        .is_ok());
        assert!(plan(
            PeerHostPortMode::Fixed,
            false,
            &[PeerTransport::Http],
            &supported
        )
        .is_ok());
        assert!(plan(
            PeerHostPortMode::Fixed,
            false,
            &[PeerTransport::Tcp, PeerTransport::Http],
            &supported
        )
        .is_err());
        let dropped = plan(
            PeerHostPortMode::Fixed,
            false,
            &[PeerTransport::Bluetooth],
            &[PeerTransport::Http],
        )
        .expect("an unsupported transport must be dropped, not rejected");
        assert!(dropped.transports.is_empty());
        assert!(plan(
            PeerHostPortMode::Fixed,
            true,
            &[PeerTransport::Bluetooth],
            &[PeerTransport::Http],
        )
        .is_err());
    }

    /// Fixed mode must reject port zero before the runtime reports it.
    #[test]
    fn listener_plan_rejects_zero_in_fixed_mode() {
        let text = TuiLanguage::English.text();
        let selected = BTreeSet::from([PeerTransport::Http]);
        let error = plan_listener_settings(
            text,
            "0.0.0.0",
            "0",
            PeerHostPortMode::Fixed,
            false,
            true,
            &selected,
            &[PeerTransport::Http],
        )
        .err()
        .expect("fixed mode must reject port zero");
        assert!(error.contains("1-65535"), "unexpected error: {error}");
    }

    /// Updating a dial address is only possible for a device this node holds
    /// a pairing for, and never for the local device.
    #[test]
    fn endpoint_action_requires_a_pairing_record() {
        let mut modal = device_manager_fixture();
        assert!(!modal
            .menu_actions("phone")
            .contains(&DeviceManagerAction::UpdateEndpoint));
        modal.paired = ["phone".to_string()].into_iter().collect();
        assert!(modal
            .menu_actions("phone")
            .contains(&DeviceManagerAction::UpdateEndpoint));
        modal.paired.insert("self".to_string());
        assert!(modal.menu_actions("self").is_empty());
    }

    /// Focus order matches the rendered form so Tab never skips or repeats a
    /// row; wrapping keeps Enter save reachable from the first keypress.
    #[test]
    fn listener_field_cycles_in_render_order() {
        assert_eq!(ListenerField::PortMode.step(true), ListenerField::Host);
        assert_eq!(ListenerField::Save.step(true), ListenerField::PortMode);
        assert_eq!(ListenerField::PortMode.step(false), ListenerField::Save);
    }

    #[test]
    fn space_membership_notice_names_changed_members() {
        let text = TuiLanguage::English.text();
        let previous = BTreeSet::from(["self".to_string(), "old".to_string()]);
        let current = BTreeSet::from(["self".to_string(), "new".to_string()]);
        let names = BTreeMap::from([("new".to_string(), "phone".to_string())]);
        assert_eq!(
            space_membership_notice(text, &previous, &previous, &names),
            None
        );
        let notice = space_membership_notice(text, &previous, &current, &names)
            .expect("a changed member set must produce a notice");
        assert!(
            notice.contains("old"),
            "departed id must be named: {notice}"
        );
        assert!(
            notice.contains("phone"),
            "joined device must use its name: {notice}"
        );
        assert!(
            notice.contains(" · "),
            "both sides must share one notice: {notice}"
        );
    }
}
