use std::collections::{HashMap, HashSet};
use std::io::{self, Stdout};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use operit_runtime::core::tools::ToolPermissionSystem::{PermissionLevel, PermissionRequestResult};
use operit_runtime::data::model::ActivePrompt::ActivePrompt;
use operit_runtime::data::model::AttachmentInfo::AttachmentInfo;
use operit_runtime::data::model::CharacterCard::CharacterCardChatModelBindingMode;
use operit_runtime::data::model::ChatHistory::ChatHistory;
use operit_runtime::data::model::ChatMessage::ChatMessage;
use operit_runtime::data::model::ChatTurnOptions::ChatTurnOptions;
use operit_runtime::data::model::FunctionType::FunctionType;
use operit_runtime::data::model::InputProcessingState::InputProcessingState;
use operit_runtime::data::model::PromptFunctionType::PromptFunctionType;
use operit_runtime::data::preferences::ModelConfigManager::ModelConfigManager;
use operit_runtime::util::stream::TextStreamRevisionTracker::TextStreamRevisionTracker;
use operit_runtime::util::AppLogger::AppLogger;
use operit_runtime::util::GithubReleaseUtil::{
    FullUpdateProgressEvent, FullUpdateStage, ReleaseInfo,
};
use serde::Deserialize;

use super::approval::TuiApprovalBridge;
use super::helpers::{short_chat_label, split_command_line};
use super::link_proxy_rs::TuiCore;
use super::typewriter::TypewriterState;
use crate::{build_attachment_info, parse_shell_args, ChatSendArgs, ShellArgs};

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
    pub(super) focus: FocusArea,
    pub(super) input: String,
    pub(super) input_cursor: usize,
    pub(super) autocomplete_index: usize,
    pub(super) queued_attachment_paths: Vec<String>,
    pub(super) queued_inline_attachments: Vec<AttachmentInfo>,
    pub(super) queued_attachment_tokens: Vec<QueuedAttachmentToken>,
    pub(super) paste_attachment_counter: usize,
    pub(super) status_message: String,
    pub(super) context_usage_label: String,
    pub(super) transcript_scroll: u16,
    pub(super) transcript_viewport_height: u16,
    pub(super) transcript_max_scroll: u16,
    pub(super) follow_transcript: bool,
    pub(super) show_chat_list: bool,
    pub(super) ctrl_c_pending: bool,
    pub(super) last_current_chat_loading: bool,
    pub(super) awaiting_runtime_loading: bool,
    pub(super) typewriter_state: TypewriterState,
    pub(super) response_stream_subscription_chat_ids: HashSet<String>,
    pub(super) response_stream_text_by_chat_id: HashMap<String, String>,
    pub(super) response_stream_revision_tracker_by_chat_id:
        HashMap<String, TextStreamRevisionTracker>,
    pub(super) approval_bridge: TuiApprovalBridge,
    pub(super) show_help: bool,
    pub(super) startup_update_prompt: Option<StartupUpdatePrompt>,
    pub(super) startup_workspace_prompt: Option<StartupWorkspacePrompt>,
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
    Complete(Result<PathBuf, String>),
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

#[derive(Clone, Debug, Deserialize)]
struct ResponseStreamLinkEvent {
    #[serde(rename = "chatId")]
    chatId: String,
    #[serde(rename = "type")]
    event_type: String,
    value: Option<String>,
    id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FocusArea {
    Chats,
    ModelChooser,
    Input,
}

impl OperitTui {
    pub(super) async fn new(
        mut core: TuiCore,
        initial_shell_args: ShellArgs,
        initial_chat_id: String,
        approval_bridge: TuiApprovalBridge,
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
        let status_message =
            "F3 chats | Enter send | Esc cancel | Ctrl+J newline | Ctrl+N new chat | Ctrl+Q quit | ? help"
                .to_string();
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
            current_chat_id_cache,
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
            focus: FocusArea::Input,
            input: String::new(),
            input_cursor: 0,
            autocomplete_index: 0,
            queued_attachment_paths: Vec::new(),
            queued_inline_attachments: Vec::new(),
            queued_attachment_tokens: Vec::new(),
            paste_attachment_counter: 0,
            status_message,
            context_usage_label: String::new(),
            transcript_scroll: 0,
            transcript_viewport_height: 1,
            transcript_max_scroll: 0,
            follow_transcript: true,
            show_chat_list: false,
            ctrl_c_pending: false,
            last_current_chat_loading: false,
            awaiting_runtime_loading: false,
            typewriter_state: TypewriterState::default(),
            response_stream_subscription_chat_ids: HashSet::new(),
            response_stream_text_by_chat_id: HashMap::new(),
            response_stream_revision_tracker_by_chat_id: HashMap::new(),
            approval_bridge,
            show_help: false,
            startup_update_prompt,
            startup_workspace_prompt: startup_workspace_prompt_path.map(|path| {
                StartupWorkspacePrompt {
                    path,
                    accept_selected: true,
                }
            }),
            should_quit: false,
        })
    }

    pub(super) async fn run(&mut self) -> Result<(), String> {
        let previous_console_logging = AppLogger::enable_console_logging();
        AppLogger::set_enable_console_logging(false);
        if let Err(error) = enable_raw_mode().map_err(|error| error.to_string()) {
            AppLogger::set_enable_console_logging(previous_console_logging);
            return Err(error);
        }
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)
            .map_err(|error| error.to_string())
        {
            let _ = disable_raw_mode();
            AppLogger::set_enable_console_logging(previous_console_logging);
            return Err(error);
        }
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = match Terminal::new(backend).map_err(|error| error.to_string()) {
            Ok(terminal) => terminal,
            Err(error) => {
                let _ = disable_raw_mode();
                AppLogger::set_enable_console_logging(previous_console_logging);
                return Err(error);
            }
        };
        let result = self.run_loop(&mut terminal).await;
        let cleanup_result = disable_raw_mode()
            .map_err(|error| error.to_string())
            .and_then(|_| {
                execute!(
                    terminal.backend_mut(),
                    DisableBracketedPaste,
                    LeaveAlternateScreen
                )
                .map_err(|error| error.to_string())
            })
            .and_then(|_| terminal.show_cursor().map_err(|error| error.to_string()));
        AppLogger::set_enable_console_logging(previous_console_logging);
        result.and(cleanup_result)
    }

    async fn run_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), String> {
        while !self.should_quit {
            self.apply_pushed_events();
            self.sync_response_stream_subscriptions().await;
            self.refresh_runtime_status().await;
            terminal
                .draw(|frame| self.render(frame))
                .map_err(|error| error.to_string())?;

            if event::poll(Duration::from_millis(16)).map_err(|error| error.to_string())? {
                match event::read().map_err(|error| error.to_string())? {
                    Event::Key(key) => self.handle_key_event(key).await?,
                    Event::Paste(text) => self.handle_paste(text).await?,
                    _ => {}
                }
            }
        }
        Ok(())
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<(), String> {
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return Ok(());
        }

        if matches!(key.code, KeyCode::Char('c')) && key.modifiers == KeyModifiers::CONTROL {
            if self.ctrl_c_pending {
                self.should_quit = true;
            } else {
                self.ctrl_c_pending = true;
                self.status_message = "press Ctrl+C again to quit".to_string();
            }
            return Ok(());
        }

        self.ctrl_c_pending = false;

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
                self.status_message = "chat list refreshed".to_string();
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
                    self.close_model_chooser();
                    return Ok(());
                }
                if self.show_chat_list && self.focus == FocusArea::Chats {
                    self.show_chat_list = false;
                    self.focus = FocusArea::Input;
                    self.status_message = "chat list hidden".to_string();
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
                if !self.show_chat_list {
                    self.focus = FocusArea::Input;
                    return Ok(());
                }
                self.focus = match self.focus {
                    FocusArea::Chats => FocusArea::Input,
                    FocusArea::ModelChooser => FocusArea::Input,
                    FocusArea::Input => FocusArea::Chats,
                };
                return Ok(());
            }
            _ => {}
        }

        match self.focus {
            FocusArea::Chats => self.handle_chat_list_key(key).await,
            FocusArea::ModelChooser => self.handle_model_chooser_key(key).await,
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
                if self.selected_model_choice_index > 0 {
                    self.selected_model_choice_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_model_choice_index + 1 < self.model_choices.len() {
                    self.selected_model_choice_index += 1;
                }
            }
            KeyCode::Enter => {
                self.apply_selected_model_choice().await?;
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
                self.status_message = "workspace not bound".to_string();
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
                    self.status_message = "workspace not bound".to_string();
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
                self.status_message = "update skipped".to_string();
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
                    self.status_message = "update skipped".to_string();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn start_full_update_download(&mut self) -> Result<(), String> {
        let Some(prompt) = self.startup_update_prompt.as_mut() else {
            return Ok(());
        };
        let Some(release_info) = prompt.release_info.as_ref() else {
            return Ok(());
        };
        let (tx, rx) = mpsc::channel::<FullUpdateDownloadMessage>();
        let package_url = release_info.downloadUrl.clone();
        let package_file_name = release_info.assetName.clone();
        let work_dir = std::env::temp_dir().join("operit2").join("full_update");
        prompt.progress_rx = Some(rx);
        prompt.download_state = FullUpdateDownloadState::Downloading {
            stage: FullUpdateStage::DownloadingPackage,
            message: "Downloading full update package".to_string(),
            read_bytes: 0,
            total_bytes: 0,
            speed_bytes_per_sec: 0,
        };
        self.status_message = "downloading full update package".to_string();
        tokio::spawn(async move {
            let progress_tx = tx.clone();
            let result = operit_runtime::util::GithubReleaseUtil::GithubReleaseUtil::downloadAndPrepareFullUpdateWithProgress(
                package_url,
                package_file_name,
                work_dir,
                move |event| {
                    let _ = progress_tx.send(FullUpdateDownloadMessage::Progress(event));
                },
            )
            .await;
            let _ = tx.send(FullUpdateDownloadMessage::Complete(result));
        });
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
        self.status_message = format!("workspace bound: {}", prompt.path);
        Ok(())
    }

    async fn cancel_current_request(&mut self) -> Result<(), String> {
        let chat_id = self.current_chat_id()?;
        self.core
            .chat_runtime_holder_main()
            .cancelCurrentMessage()
            .await
            .map_err(|error| error.to_string())?;
        self.last_current_chat_loading = false;
        self.awaiting_runtime_loading = false;
        self.follow_transcript = true;
        self.status_message = format!("request cancelled: {}", short_chat_label(&chat_id));
        Ok(())
    }

    pub(super) async fn submit_input(&mut self) -> Result<(), String> {
        if self.current_chat_is_loading() {
            self.status_message = "request already running".to_string();
            return Ok(());
        }

        let input = self.input.trim_end().to_string();
        let has_queued_attachments =
            !self.queued_attachment_paths.is_empty() || !self.queued_inline_attachments.is_empty();
        if input.trim().is_empty() && !has_queued_attachments {
            return Ok(());
        }
        if input.starts_with('/') {
            self.input.clear();
            self.input_cursor = 0;
            self.handle_local_command(&input).await?;
            return Ok(());
        }

        let chat_id = self.current_chat_id()?;
        let attachment_paths = std::mem::take(&mut self.queued_attachment_paths);
        let inline_attachments = std::mem::take(&mut self.queued_inline_attachments);
        let attachment_tokens = std::mem::take(&mut self.queued_attachment_tokens);
        let message = strip_attachment_tokens(input, &attachment_tokens);
        self.follow_transcript = true;
        self.status_message = "connecting...".to_string();
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
        self.status_message = "streaming".to_string();
        Ok(())
    }

    async fn begin_chat_message(
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
            .updateUserMessage(send_args.message)
            .await
            .map_err(|error| error.to_string())?;
        self.core
            .chat_runtime_holder_main()
            .sendUserMessage(
                PromptFunctionType::CHAT,
                None,
                None,
                None,
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
        let parts = split_command_line(input)?;
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
            "model" => {
                self.handle_model_command(&parts[1..]).await?;
            }
            "approval" => {
                self.handle_approval_command(&parts[1..]).await?;
            }
            "attach" => {
                let path = parts
                    .get(1)
                    .ok_or_else(|| "usage: /attach <path>".to_string())?
                    .clone();
                self.queued_attachment_paths.push(path.clone());
                self.status_message = format!(
                    "queued attachment: {path} ({} total)",
                    self.queued_attachment_paths.len()
                );
            }
            "attachments" => {
                let queued = self.queued_attachment_labels();
                self.status_message = if queued.is_empty() {
                    "attachments=none".to_string()
                } else {
                    format!("attachments={}", queued.join(", "))
                };
            }
            "clear-attachments" => {
                self.clear_queued_attachments();
                self.status_message = "attachments cleared".to_string();
            }
            _ => {
                self.status_message = format!("unknown command: /{command}");
            }
        }
        Ok(())
    }

    fn handle_approval_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('1') | KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.approval_bridge.respond(PermissionRequestResult::ALLOW);
                self.status_message = "tool approved once".to_string();
            }
            KeyCode::Char('2') | KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.approval_bridge.respond(PermissionRequestResult::DENY);
                self.status_message = "tool denied".to_string();
            }
            KeyCode::Char('3') | KeyCode::Char('a') | KeyCode::Char('A') => {
                self.approval_bridge
                    .respond(PermissionRequestResult::ALWAYS_ALLOW);
                self.status_message = "tool approved and remembered".to_string();
            }
            _ => {}
        }
    }

    async fn handle_approval_command(&mut self, args: &[String]) -> Result<(), String> {
        match args.first().map(String::as_str) {
            None | Some("status") => {
                let master = self
                    .core
                    .permissions_tool_permission_system()
                    .getMasterSwitch()
                    .await
                    .map_err(|error| error.to_string())?;
                let overrides = self
                    .core
                    .permissions_tool_permission_system()
                    .getToolPermissionOverrides()
                    .await
                    .map_err(|error| error.to_string())?;
                self.status_message = format!(
                    "approval master={} overrides={}",
                    master.name(),
                    overrides.len()
                );
            }
            Some("allow") | Some("ask") | Some("forbid") => {
                let level = parse_permission_level(args.first().map(String::as_str))?;
                self.core
                    .permissions_tool_permission_system()
                    .saveMasterSwitch(level.clone())
                    .await
                    .map_err(|error| error.to_string())?;
                self.status_message = format!("approval master={}", level.name());
            }
            Some("tool") => {
                let toolName = args.get(1).ok_or_else(|| {
                    "usage: /approval tool <tool-name> <allow|ask|forbid|clear>".to_string()
                })?;
                match args.get(2).map(String::as_str) {
                    Some("clear") => {
                        self.core
                            .permissions_tool_permission_system()
                            .clearToolPermission(toolName)
                            .await
                            .map_err(|error| error.to_string())?;
                        self.status_message = format!("approval cleared: {toolName}");
                    }
                    value @ (Some("allow") | Some("ask") | Some("forbid")) => {
                        let level = parse_permission_level(value)?;
                        self.core
                            .permissions_tool_permission_system()
                            .saveToolPermission(toolName, level.clone())
                            .await
                            .map_err(|error| error.to_string())?;
                        self.status_message = format!("approval {toolName}={}", level.name());
                    }
                    _ => {
                        return Err("usage: /approval tool <tool-name> <allow|ask|forbid|clear>"
                            .to_string());
                    }
                }
            }
            Some("list") => {
                let overrides = self
                    .core
                    .permissions_tool_permission_system()
                    .getToolPermissionOverrides()
                    .await
                    .map_err(|error| error.to_string())?;
                self.status_message = if overrides.is_empty() {
                    "approval overrides=none".to_string()
                } else {
                    overrides
                        .iter()
                        .map(|(tool, level)| format!("{tool}={}", level.name()))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
            }
            Some("help") => {
                self.status_message =
                    "usage: /approval | /approval list|allow|ask|forbid|tool <tool> <allow|ask|forbid|clear>"
                        .to_string();
            }
            Some(other) => {
                self.status_message = format!("unknown /approval command: {other}");
            }
        }
        Ok(())
    }

    async fn handle_model_command(&mut self, args: &[String]) -> Result<(), String> {
        match args.first().map(String::as_str) {
            None | Some("current") => self.show_current_chat_model().await,
            Some("list") => self.list_chat_models().await,
            Some("choose") => self.open_model_chooser().await,
            Some("use") => self.use_chat_model(&args[1..]).await,
            Some("help") => {
                self.status_message =
                    "usage: /model current | /model list | /model choose | /model use <model-id>"
                        .to_string();
                Ok(())
            }
            Some(other) => {
                self.status_message = format!("unknown /model command: {other}");
                Ok(())
            }
        }
    }

    async fn show_current_chat_model(&mut self) -> Result<(), String> {
        let (provider_id, model_id, provider_name) = self.current_chat_model_status_parts().await?;
        self.status_message = format!("CHAT -> {} {} / {}", provider_id, provider_name, model_id);
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
        self.core
            .preferences_model_config_manager()
            .initializeIfNeeded()
            .await
            .map_err(|error| error.to_string())?;
        let entries = self
            .core
            .preferences_model_config_manager()
            .getAllModelSummaries()
            .await
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|summary| {
                format!(
                    "{}:{}={}/{}",
                    summary.providerId, summary.modelId, summary.providerName, summary.modelId
                )
            })
            .collect::<Vec<_>>();
        self.status_message = format!("models: {}", entries.join(" | "));
        Ok(())
    }

    async fn open_model_chooser(&mut self) -> Result<(), String> {
        self.model_choices = self.load_model_choices().await?;
        if self.model_choices.is_empty() {
            self.status_message = "no model configs".to_string();
            return Ok(());
        }
        self.selected_model_choice_index = self
            .model_choices
            .iter()
            .position(|choice| choice.selected)
            .expect("current chat model mapping must be present in model choices");
        self.show_model_chooser = true;
        self.focus = FocusArea::ModelChooser;
        self.status_message = "choose model | Up/Down select | Enter apply | Esc close".to_string();
        Ok(())
    }

    fn close_model_chooser(&mut self) {
        self.show_model_chooser = false;
        self.focus = FocusArea::Input;
        self.status_message = "model chooser closed".to_string();
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
        let choice = self
            .model_choices
            .get(self.selected_model_choice_index)
            .cloned()
            .ok_or_else(|| "no selected model".to_string())?;
        self.apply_chat_model_choice(&choice).await?;
        self.show_model_chooser = false;
        self.focus = FocusArea::Input;
        Ok(())
    }

    async fn use_chat_model(&mut self, args: &[String]) -> Result<(), String> {
        let provider_id = match args.first() {
            Some(value) if !value.trim().is_empty() => value.trim().to_string(),
            _ => {
                self.status_message = "usage: /model use <provider-id> <model-id>".to_string();
                return Ok(());
            }
        };
        let model_id = match args.get(1) {
            Some(value) if !value.trim().is_empty() => value.trim().to_string(),
            _ => {
                self.status_message = "usage: /model use <provider-id> <model-id>".to_string();
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
        self.status_message = format!(
            "CHAT -> {} {} / {}",
            choice.provider_id, choice.provider_name, choice.model_id
        );
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
        self.status_message = format!(
            "context model={} | context={}K",
            model_ref.model_id,
            format_context_length(effective_context_length)
        );
        self.refresh_context_usage_label().await;
        Ok(())
    }

    async fn create_new_chat(&mut self, shell_args: ShellArgs) -> Result<(), String> {
        if self.current_chat_is_loading() {
            self.status_message = "wait for current request to finish".to_string();
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
        self.status_message = "new chat".to_string();
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
            self.status_message =
                "chat list shown | Up/Down select | Enter switch | Esc close".to_string();
        } else {
            self.focus = FocusArea::Input;
            self.status_message = "chat list hidden".to_string();
        }
    }

    async fn resume_previous_chat(&mut self) -> Result<(), String> {
        if self.current_chat_is_loading() {
            self.status_message = "wait for current request to finish".to_string();
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
            self.status_message = "no previous chat to resume".to_string();
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
        self.status_message = format!("resumed chat: {}", target.title);
        Ok(())
    }

    async fn switch_to_chat(&mut self, chat_id: String) -> Result<(), String> {
        if self.current_chat_is_loading() {
            self.status_message = "wait for current request to finish".to_string();
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
        self.status_message = "switched chat".to_string();
        Ok(())
    }

    async fn refresh_chats(&mut self) {
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

    fn select_chat_by_id(&mut self, chat_id: &str) {
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

    fn apply_full_update_download_events(&mut self) {
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
                        let current = match prompt.download_state.clone() {
                            FullUpdateDownloadState::Downloading {
                                read_bytes,
                                total_bytes,
                                speed_bytes_per_sec,
                                ..
                            } => (read_bytes, total_bytes, speed_bytes_per_sec),
                            _ => (0, 0, 0),
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
                                "Downloading full update package".to_string(),
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
                FullUpdateDownloadMessage::Complete(Ok(package_path)) => {
                    prompt.download_state = FullUpdateDownloadState::Complete { package_path };
                    prompt.progress_rx = None;
                    self.status_message = "full update package ready".to_string();
                    break;
                }
                FullUpdateDownloadMessage::Complete(Err(message)) => {
                    prompt.download_state = FullUpdateDownloadState::Error { message };
                    prompt.progress_rx = None;
                    self.status_message = "full update failed".to_string();
                    break;
                }
            }
        }
    }

    async fn sync_response_stream_subscriptions(&mut self) {
        let Some(current_chat_id) = self.current_chat_id_cache.clone() else {
            return;
        };
        if !self
            .active_streaming_chat_ids_cache
            .contains(&current_chat_id)
        {
            return;
        }
        if self
            .response_stream_subscription_chat_ids
            .contains(&current_chat_id)
        {
            return;
        }
        if !self
            .current_messages_cache
            .iter()
            .rev()
            .any(|message| message.sender == "ai")
        {
            return;
        }
        if self
            .core
            .watchMainChatResponseStream(current_chat_id.clone())
            .await
            .is_ok()
        {
            self.response_stream_subscription_chat_ids
                .insert(current_chat_id);
        }
    }

    fn apply_response_stream_event(&mut self, value: serde_json::Value) {
        let Ok(event) = serde_json::from_value::<ResponseStreamLinkEvent>(value) else {
            return;
        };
        match event.event_type.as_str() {
            "chunk" => {
                let Some(chunk) = event.value else {
                    return;
                };
                let tracker = self
                    .response_stream_revision_tracker_by_chat_id
                    .entry(event.chatId.clone())
                    .or_insert_with(|| TextStreamRevisionTracker::new(""));
                let content = tracker.append(&chunk);
                self.response_stream_text_by_chat_id
                    .insert(event.chatId, content);
            }
            "savepoint" => {
                let Some(id) = event.id else {
                    return;
                };
                let tracker = self
                    .response_stream_revision_tracker_by_chat_id
                    .entry(event.chatId)
                    .or_insert_with(|| TextStreamRevisionTracker::new(""));
                tracker.savepoint(&id);
            }
            "rollback" => {
                let Some(id) = event.id else {
                    return;
                };
                if let Some(tracker) = self
                    .response_stream_revision_tracker_by_chat_id
                    .get_mut(&event.chatId)
                {
                    if let Some(content) = tracker.rollback(&id) {
                        self.response_stream_text_by_chat_id
                            .insert(event.chatId, content);
                    }
                }
            }
            "completed" => {}
            _ => {}
        }
    }

    fn retain_active_response_stream_state(&mut self) {
        let active = &self.active_streaming_chat_ids_cache;
        self.response_stream_subscription_chat_ids
            .retain(|chat_id| active.contains(chat_id));
        self.response_stream_text_by_chat_id
            .retain(|chat_id, _| active.contains(chat_id));
        self.response_stream_revision_tracker_by_chat_id
            .retain(|chat_id, _| active.contains(chat_id));
    }

    fn update_current_chat_loading_from_streaming_ids(&mut self) {
        self.current_chat_is_loading_cache = self
            .current_chat_id_cache
            .as_ref()
            .map(|chat_id| self.active_streaming_chat_ids_cache.contains(chat_id))
            .unwrap_or(false);
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
                        "正在连接AI服务...".to_string(),
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
                _ => input_processing_status_text(&state),
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
            return Ok(format!(
                "context {} / {}",
                current_window_size.max(0),
                max_tokens
            ));
        }
        let usage_percent =
            ((current_window_size.max(0) as f64 / max_tokens as f64) * 100.0).round() as i32;
        Ok(format!(
            "context {}% ({}/{})",
            usage_percent, current_window_size, max_tokens
        ))
    }
}

fn input_processing_status_text(state: &InputProcessingState) -> String {
    match state {
        InputProcessingState::Processing { message } => resolve_processing_message(message),
        InputProcessingState::Connecting { message } => resolve_processing_message(message),
        InputProcessingState::Receiving { message } => resolve_processing_message(message),
        InputProcessingState::ExecutingTool { toolName } => {
            format!("正在执行工具: {}", toolName.trim())
        }
        InputProcessingState::ToolProgress { message, .. } => resolve_processing_message(message),
        InputProcessingState::ProcessingToolResult { toolName } => {
            format!("正在处理工具结果: {}", toolName.trim())
        }
        InputProcessingState::Summarizing { message } => resolve_processing_message(message),
        InputProcessingState::ExecutingPlan { message } => resolve_processing_message(message),
        InputProcessingState::Idle | InputProcessingState::Completed => String::new(),
        InputProcessingState::Error { message } => message.clone(),
    }
}

fn resolve_processing_message(message: &str) -> String {
    match message {
        "enhanced_processing_input" => "正在处理输入...".to_string(),
        "enhanced_processing_message" | "message_processing" => "正在处理消息...".to_string(),
        "enhanced_connecting_service" => "正在连接AI服务...".to_string(),
        "enhanced_receiving_response" => "正在接收AI响应...".to_string(),
        "enhanced_receiving_tool_result" => "正在接收工具执行后的AI响应...".to_string(),
        "chat_processing_attachment" => "正在处理附件...".to_string(),
        "chat_processing_shared_files" => "正在处理分享文件...".to_string(),
        "chat_summarizing_memory" => "正在总结记忆...".to_string(),
        "chat_summarizing_generating" => "正在生成总结...".to_string(),
        "compressing history" => "正在压缩历史...".to_string(),
        _ => message.trim().to_string(),
    }
}

fn parse_permission_level(value: Option<&str>) -> Result<PermissionLevel, String> {
    match value {
        Some("allow") | Some("ALLOW") => Ok(PermissionLevel::ALLOW),
        Some("ask") | Some("ASK") => Ok(PermissionLevel::ASK),
        Some("forbid") | Some("FORBID") => Ok(PermissionLevel::FORBID),
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
