use std::collections::{HashMap, HashSet, VecDeque};
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
use ratatui::layout::Rect;
use ratatui::Terminal;

use operit_model::ActivePrompt::ActivePrompt;
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_model::CharacterCard::CharacterCardChatModelBindingMode;
use operit_model::ChatHistory::ChatHistory;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatTurnOptions::ChatTurnOptions;
use operit_model::FunctionType::FunctionType;
use operit_model::InputProcessingState::InputProcessingState;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_runtime::data::preferences::ModelConfigManager::ModelConfigManager;
use operit_tools::tools::ToolPermissionSystem::{AiPermissionMode, PermissionRequestResult};
use operit_util::stream::TextStreamRevisionTracker::TextStreamRevisionTracker;
use operit_util::AppLogger::AppLogger;
use operit_util::GithubReleaseUtil::{
    FullUpdateProgressEvent, FullUpdateStage, FullUpdateTarget, ReleaseInfo,
};

use super::approval::TuiApprovalBridge;
use super::config;
use super::config::ConfigUi;
use super::helpers::{short_chat_label, split_command_line};
use super::i18n::{TuiLanguage, TuiText};
use super::link_proxy_rs::TuiCore;
use super::pending_queue::PendingQueueMessage;
use super::selection::{
    mouse_drag_transcript_position, mouse_transcript_position, TranscriptCopyLine,
    TranscriptSelectionState,
};
use super::stream_markdown::TuiMarkdownStreamState;
use super::transcript::TranscriptRenderCache;
use super::typewriter::TypewriterState;
use crate::cli::CliInstallProgress;
use crate::{build_attachment_info, parse_shell_args, ChatSendArgs, ShellArgs};

const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(16);
const RUNTIME_STATUS_REFRESH_INTERVAL: Duration = Duration::from_millis(250);
const TRANSIENT_STATUS_DURATION: Duration = Duration::from_secs(3);
const MAX_PENDING_TERMINAL_EVENTS_PER_FRAME: usize = 64;

pub(super) struct OperitTui {
    pub(super) core: TuiCore,
    pub(super) initial_shell_args: ShellArgs,
    pub(super) current_chat_id_cache: Option<String>,
    pub(super) current_messages_cache: Vec<ChatMessage>,
    pub(super) current_chat_is_loading_cache: bool,
    pub(super) current_chat_input_processing_state_cache: InputProcessingState,
    pub(super) active_streaming_chat_ids_cache: HashSet<String>,
    pub(super) current_window_size_cache: i32,
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
    pub(super) status_message_expires_at: Option<Instant>,
    pub(super) transient_status_message: Option<String>,
    pub(super) context_usage_label: String,
    pub(super) transcript_scroll: u16,
    pub(super) transcript_viewport_height: u16,
    pub(super) transcript_max_scroll: u16,
    pub(super) follow_transcript: bool,
    pub(super) transcript_render_cache: TranscriptRenderCache,
    pub(super) transcript_area: Rect,
    pub(super) transcript_copy_lines: Vec<TranscriptCopyLine>,
    pub(super) transcript_selection: TranscriptSelectionState,
    pub(super) show_chat_list: bool,
    pub(super) ctrl_c_pending: bool,
    pub(super) last_current_chat_loading: bool,
    pub(super) awaiting_runtime_loading: bool,
    pub(super) last_runtime_status_refresh_at: Option<Instant>,
    pub(super) typewriter_state: TypewriterState,
    pub(super) response_stream_subscription_chat_ids: HashSet<String>,
    pub(super) response_stream_text_by_chat_id: HashMap<String, String>,
    pub(super) response_stream_markdown_by_chat_id: HashMap<String, TuiMarkdownStreamState>,
    pub(super) response_stream_revision_tracker_by_chat_id:
        HashMap<String, TextStreamRevisionTracker>,
    pub(super) approval_bridge: TuiApprovalBridge,
    pub(super) language: TuiLanguage,
    pub(super) show_help: bool,
    pub(super) startup_install_prompt: Option<StartupInstallPrompt>,
    pub(super) startup_update_prompt: Option<StartupUpdatePrompt>,
    pub(super) startup_workspace_prompt: Option<StartupWorkspacePrompt>,
    pub(super) show_config_popup: bool,
    pub(super) config_ui: ConfigUi,
    pub(super) should_quit: bool,
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
        initial_shell_args: ShellArgs,
        initial_chat_id: String,
        approval_bridge: TuiApprovalBridge,
        language: TuiLanguage,
        startup_install_prompt: Option<StartupInstallPrompt>,
        startup_update_prompt: Option<StartupUpdatePrompt>,
        startup_workspace_prompt_path: Option<String>,
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
        let current_messages_cache = core
            .chat_runtime_holder_main()
            .chatHistoryFlowSnapshot()
            .await
            .map_err(|error| error.to_string())?;
        let current_chat_is_loading_cache = core
            .chat_runtime_holder_main()
            .currentChatIsLoading()
            .await
            .map_err(|error| error.to_string())?;
        let current_chat_input_processing_state_cache = core
            .chat_runtime_holder_main()
            .currentChatInputProcessingState()
            .await
            .map_err(|error| error.to_string())?;
        let active_streaming_chat_ids_cache = core
            .chat_runtime_holder_main()
            .activeStreamingChatIdsFlowSnapshot()
            .await
            .map_err(|error| error.to_string())?;
        let current_window_size_cache = core
            .chat_runtime_holder_main()
            .currentWindowSizeFlowSnapshot()
            .await
            .map_err(|error| error.to_string())?;
        current_chat_id_cache
            .as_ref()
            .ok_or_else(|| "no active chat in tui".to_string())?;
        core.watchMainChatGeneratedStateFlows()
            .await
            .map_err(|error| error.to_string())?;
        Ok(Self {
            core,
            initial_shell_args,
            current_chat_id_cache: current_chat_id_cache.clone(),
            current_messages_cache,
            current_chat_is_loading_cache,
            current_chat_input_processing_state_cache,
            active_streaming_chat_ids_cache,
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
            status_message,
            status_message_expires_at: None,
            transient_status_message: None,
            context_usage_label: String::new(),
            transcript_scroll: 0,
            transcript_viewport_height: 1,
            transcript_max_scroll: 0,
            follow_transcript: true,
            transcript_render_cache: TranscriptRenderCache::default(),
            transcript_area: Rect::default(),
            transcript_copy_lines: Vec::new(),
            transcript_selection: TranscriptSelectionState::default(),
            show_chat_list: false,
            ctrl_c_pending: false,
            last_current_chat_loading: false,
            awaiting_runtime_loading: false,
            last_runtime_status_refresh_at: None,
            typewriter_state: TypewriterState::default(),
            response_stream_subscription_chat_ids: HashSet::new(),
            response_stream_text_by_chat_id: HashMap::new(),
            response_stream_markdown_by_chat_id: HashMap::new(),
            response_stream_revision_tracker_by_chat_id: HashMap::new(),
            approval_bridge,
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
        result.and(cleanup_result)
    }

    async fn run_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), String> {
        while !self.should_quit {
            self.ensure_pending_queue_chat_id();
            self.apply_pushed_events();
            self.ensure_pending_queue_chat_id();
            self.sync_response_stream_subscriptions().await;
            self.refresh_runtime_status_if_due().await;
            self.advance_pending_message_queue().await?;
            self.clear_expired_status_message();
            terminal
                .draw(|frame| self.render(frame))
                .map_err(|error| error.to_string())?;

            self.handle_terminal_events(EVENT_POLL_INTERVAL).await?;
        }
        Ok(())
    }

    fn set_transient_status_message(&mut self, message: String) {
        self.status_message = message.clone();
        self.transient_status_message = Some(message);
        self.status_message_expires_at = Some(Instant::now() + TRANSIENT_STATUS_DURATION);
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
            Event::Mouse(mouse) => self.handle_mouse_event(mouse),
            Event::Paste(text) => self.handle_paste(text).await,
            _ => Ok(()),
        }
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<(), String> {
        match mouse.kind {
            MouseEventKind::ScrollUp => self.scroll_transcript_up(self.terminal_wheel_step()),
            MouseEventKind::ScrollDown => self.scroll_transcript_down(self.terminal_wheel_step()),
            MouseEventKind::Down(MouseButton::Left) => {
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
                if let Some(position) = mouse_drag_transcript_position(
                    mouse,
                    self.transcript_area,
                    self.transcript_scroll,
                    &self.transcript_copy_lines,
                ) {
                    self.transcript_selection.end(position);
                }
            }
            MouseEventKind::Down(MouseButton::Right) => self.transcript_selection.clear(),
            _ => {}
        }
        Ok(())
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
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => match clipboard.set_text(text) {
                Ok(()) => {
                    self.status_message = self.text().selection_copied().to_string();
                    true
                }
                Err(error) => {
                    self.status_message = self.text().copy_failed(&error.to_string());
                    true
                }
            },
            Err(error) => {
                self.status_message = self.text().copy_failed(&error.to_string());
                true
            }
        }
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<(), String> {
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
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

        if self.approval_bridge.current().is_some() {
            self.handle_approval_key(key);
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
        let Some(prompt) = self.startup_workspace_prompt.take() else {
            return Ok(());
        };
        let chat_id = self.current_chat_id()?;
        self.core
            .chat_runtime_holder_main()
            .bindChatToWorkspace(chat_id.clone(), prompt.path.clone())
            .await
            .map_err(|error| error.to_string())?;
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
            .cancelCurrentMessage()
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
            self.handle_local_command(&input).await?;
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
        self.core
            .preferences_model_config_manager()
            .initializeIfNeeded()
            .await
            .map_err(|error| error.to_string())?;
        self.core
            .preferences_functional_config_manager()
            .initializeIfNeeded()
            .await
            .map_err(|error| error.to_string())?;
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
                None,
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
        match command {
            "help" => {
                self.show_help = true;
            }
            "quit" | "exit" => {
                self.should_quit = true;
            }
            "new" => {
                let shell_args = parse_shell_args(&parts[1..])?;
                self.create_new_chat(shell_args).await?;
            }
            "switch" => {
                self.toggle_chat_list().await;
            }
            "resume" => {
                self.resume_previous_chat().await?;
            }
            "max" => {
                self.toggle_max_context_mode().await?;
            }
            "language" => {
                self.handle_language_command(&parts[1..])?;
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
                self.status_message = self.text().unknown_command(command);
            }
        }
        Ok(())
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

    fn handle_approval_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('1') | KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.approval_bridge.respond(PermissionRequestResult::ALLOW);
                self.status_message = self.text().tool_approved_once().to_string();
            }
            KeyCode::Char('2') | KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.approval_bridge.respond(PermissionRequestResult::DENY);
                self.status_message = self.text().tool_denied().to_string();
            }
            KeyCode::Char('3') | KeyCode::Char('a') | KeyCode::Char('A') => {
                self.approval_bridge
                    .respond(PermissionRequestResult::ALLOW_SESSION);
                self.status_message = self.text().tool_approved_remembered().to_string();
            }
            _ => {}
        }
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
        self.core
            .preferences_model_config_manager()
            .initializeIfNeeded()
            .await
            .map_err(|error| error.to_string())?;
        self.core
            .preferences_functional_config_manager()
            .initializeIfNeeded()
            .await
            .map_err(|error| error.to_string())?;

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
        self.core
            .preferences_functional_config_manager()
            .initializeIfNeeded()
            .await
            .map_err(|error| error.to_string())?;
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
        self.core
            .preferences_model_config_manager()
            .initializeIfNeeded()
            .await
            .map_err(|error| error.to_string())?;
        self.core
            .preferences_functional_config_manager()
            .initializeIfNeeded()
            .await
            .map_err(|error| error.to_string())?;

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
        self.core
            .preferences_model_config_manager()
            .initializeIfNeeded()
            .await
            .map_err(|error| error.to_string())?;
        self.core
            .preferences_functional_config_manager()
            .initializeIfNeeded()
            .await
            .map_err(|error| error.to_string())?;
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
            .initializeIfNeeded()
            .await
            .map_err(|error| error.to_string())?;
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

    async fn toggle_max_context_mode(&mut self) -> Result<(), String> {
        let model_ref = self.editable_chat_model_ref().await?;
        self.core
            .preferences_model_config_manager()
            .initializeIfNeeded()
            .await
            .map_err(|error| error.to_string())?;
        let current = self
            .core
            .preferences_model_config_manager()
            .getResolvedModelConfig(&model_ref.provider_id, &model_ref.model_id)
            .await
            .map_err(|error| error.to_string())?;
        let mut context = current.context;
        context.enableMaxContextMode = !context.enableMaxContextMode;
        let updated = self
            .core
            .preferences_model_config_manager()
            .updateContextForModel(&model_ref.provider_id, &model_ref.model_id, context.clone())
            .await
            .map_err(|error| error.to_string())?;
        let updated_context = updated
            .contextOverride
            .ok_or_else(|| format!("model context not saved: {}", model_ref.model_id))?;
        let effective_context_length = if updated_context.enableMaxContextMode {
            updated_context.maxContextLength
        } else {
            updated_context.maxContextLength * 0.4
        };
        self.status_message = self.text().context_model_status(
            &model_ref.model_id,
            &format_context_length(effective_context_length),
        );
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
        self.current_messages_cache = self
            .core
            .chat_runtime_holder_main()
            .chatHistoryFlowSnapshot()
            .await
            .map_err(|error| error.to_string())?;
        self.current_chat_is_loading_cache = self
            .core
            .chat_runtime_holder_main()
            .currentChatIsLoading()
            .await
            .map_err(|error| error.to_string())?;
        self.current_chat_input_processing_state_cache = self
            .core
            .chat_runtime_holder_main()
            .currentChatInputProcessingState()
            .await
            .map_err(|error| error.to_string())?;
        self.active_streaming_chat_ids_cache = self
            .core
            .chat_runtime_holder_main()
            .activeStreamingChatIdsFlowSnapshot()
            .await
            .map_err(|error| error.to_string())?;
        self.current_window_size_cache = self
            .core
            .chat_runtime_holder_main()
            .currentWindowSizeFlowSnapshot()
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn apply_pushed_events(&mut self) {
        self.apply_startup_install_events();
        self.apply_full_update_download_events();
        for event in self.core.drainEvents() {
            match event.propertyName.as_str() {
                "currentChatIdFlow" => {
                    if let Ok(value) = serde_json::from_value::<Option<String>>(event.value) {
                        self.current_chat_id_cache = value;
                    }
                }
                "chatHistoryFlow" => {
                    if let Ok(value) = serde_json::from_value::<Vec<ChatMessage>>(event.value) {
                        self.current_messages_cache = value;
                    }
                }
                "chatHistoriesFlow" => {
                    if let Ok(value) = serde_json::from_value::<Vec<ChatHistory>>(event.value) {
                        self.chats = chat_histories_to_list(value);
                        if let Some(chat_id) = self.current_chat_id_cache.clone() {
                            self.select_chat_by_id(&chat_id);
                        }
                    }
                }
                "activeStreamingChatIdsFlow" => {
                    if let Ok(value) = serde_json::from_value::<HashSet<String>>(event.value) {
                        self.active_streaming_chat_ids_cache = value;
                        self.update_current_chat_loading_from_streaming_ids();
                        self.retain_active_response_stream_state();
                    }
                }
                "getResponseStream" => {
                    self.apply_response_stream_event(event.value);
                }
                "inputProcessingStateByChatIdFlow" => {
                    if let Ok(value) =
                        serde_json::from_value::<HashMap<String, InputProcessingState>>(event.value)
                    {
                        self.current_chat_input_processing_state_cache =
                            current_input_processing_state_from_map(
                                &value,
                                self.current_chat_id_cache.as_ref(),
                            );
                    }
                }
                "currentWindowSizeFlow" => {
                    if let Ok(value) = serde_json::from_value::<i32>(event.value) {
                        self.current_window_size_cache = value;
                    }
                }
                _ => {}
            }
        }
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
        let mut messages = self.current_messages_cache.clone();
        let Some(chat_id) = self.current_chat_id_cache.as_ref() else {
            return messages;
        };
        let Some(content) = self.response_stream_text_by_chat_id.get(chat_id) else {
            return messages;
        };
        if content.is_empty() {
            return messages;
        }
        if let Some(message) = messages
            .iter_mut()
            .rev()
            .find(|message| message.sender == "ai")
        {
            message.content = content.clone();
        }
        messages
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
            match self.current_chat_model_status_label().await {
                Ok(label) => self.set_status_message(label),
                Err(error) => self.set_status_message(error),
            }
        } else if matches!(
            state,
            InputProcessingState::Idle | InputProcessingState::Completed
        ) {
            match self.current_chat_model_status_label().await {
                Ok(label) => self.set_status_message(label),
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

    async fn current_context_usage_label(&mut self) -> Result<String, String> {
        let model_ref = self.editable_chat_model_ref().await?;
        self.core
            .preferences_model_config_manager()
            .initializeIfNeeded()
            .await
            .map_err(|error| error.to_string())?;
        let config = self
            .core
            .preferences_model_config_manager()
            .getResolvedModelConfig(&model_ref.provider_id, &model_ref.model_id)
            .await
            .map_err(|error| error.to_string())?;
        let effective_context_length = if config.context.enableMaxContextMode {
            config.context.maxContextLength
        } else {
            config.context.maxContextLength * 0.4
        };
        let max_tokens = (effective_context_length * 1024.0) as i32;
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

    async fn handle_skill_command(&mut self, args: &[String]) -> Result<(), String> {
        if args.first().map(String::as_str) == Some("toggle") {
            let name = args
                .get(1)
                .ok_or_else(|| self.text().usage_approval_tool().to_string())?;
            self.status_message = self.text().skill_toggled(name, "toggled");
            return Ok(());
        }
        if args.first().is_some() {
            self.status_message = self.text().unknown_command(&format!("skill {}", args[0]));
            return Ok(());
        }
        let skills = self
            .core
            .permissions_skill_manager()
            .getAvailableSkills()
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
                .ok_or_else(|| self.text().usage_approval_tool().to_string())?;
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
                .ok_or_else(|| self.text().usage_approval_tool().to_string())?;
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
                .ok_or_else(|| self.text().usage_approval_tool().to_string())?;
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

fn current_input_processing_state_from_map(
    value: &HashMap<String, InputProcessingState>,
    chat_id: Option<&String>,
) -> InputProcessingState {
    chat_id
        .and_then(|chat_id| value.get(chat_id))
        .or_else(|| value.get("__DEFAULT_CHAT__"))
        .cloned()
        .unwrap_or(InputProcessingState::Idle)
}

fn format_context_length(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i32)
    } else {
        format!("{value:.1}")
    }
}
