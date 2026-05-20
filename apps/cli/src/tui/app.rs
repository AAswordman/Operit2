use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::data::model::ChatMessage::ChatMessage;
use operit_runtime::data::model::InputProcessingState::InputProcessingState;

use super::helpers::{short_chat_label, split_command_line};
use crate::{
    current_shell_chat_id, ensure_chat_exists, launch_chat_message_with_application,
    parse_shell_args, ChatSendArgs, ShellArgs,
};

pub(super) struct OperitTui {
    pub(super) application: OperitApplication,
    pub(super) initial_shell_args: ShellArgs,
    pub(super) chats: Vec<ChatListItem>,
    pub(super) selected_chat_index: usize,
    pub(super) focus: FocusArea,
    pub(super) input: String,
    pub(super) input_cursor: usize,
    pub(super) autocomplete_index: usize,
    pub(super) queued_attachment_paths: Vec<String>,
    pub(super) status_message: String,
    pub(super) transcript_scroll: u16,
    pub(super) transcript_viewport_height: u16,
    pub(super) transcript_max_scroll: u16,
    pub(super) follow_transcript: bool,
    pub(super) show_chat_list: bool,
    pub(super) ctrl_c_pending: bool,
    pub(super) last_current_chat_loading: bool,
    pub(super) awaiting_runtime_loading: bool,
    pub(super) show_help: bool,
    pub(super) should_quit: bool,
}

#[derive(Clone, Debug)]
pub(super) struct ChatListItem {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) secondary: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FocusArea {
    Chats,
    Input,
}

impl OperitTui {
    pub(super) fn new(
        mut application: OperitApplication,
        initial_shell_args: ShellArgs,
        initial_chat_id: String,
    ) -> Result<Self, String> {
        let chats = {
            let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
            load_chat_list_from_core(core)
        };
        let selected_chat_index = chats
            .iter()
            .position(|item| item.id == initial_chat_id)
            .unwrap_or(0);
        let status_message = format!(
            "chat={} | F3 chats | Enter send | Ctrl+J newline | Ctrl+N new chat | Ctrl+Q quit | ? help",
            short_chat_label(&initial_chat_id)
        );
        let _ = current_shell_chat_id(&mut application)?;
        Ok(Self {
            application,
            initial_shell_args,
            chats,
            selected_chat_index,
            focus: FocusArea::Input,
            input: String::new(),
            input_cursor: 0,
            autocomplete_index: 0,
            queued_attachment_paths: Vec::new(),
            status_message,
            transcript_scroll: 0,
            transcript_viewport_height: 1,
            transcript_max_scroll: 0,
            follow_transcript: true,
            show_chat_list: false,
            ctrl_c_pending: false,
            last_current_chat_loading: false,
            awaiting_runtime_loading: false,
            show_help: false,
            should_quit: false,
        })
    }

    pub(super) async fn run(&mut self) -> Result<(), String> {
        enable_raw_mode().map_err(|error| error.to_string())?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(|error| error.to_string())?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).map_err(|error| error.to_string())?;
        let result = self.run_loop(&mut terminal).await;
        disable_raw_mode().map_err(|error| error.to_string())?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(|error| error.to_string())?;
        terminal.show_cursor().map_err(|error| error.to_string())?;
        result
    }

    async fn run_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), String> {
        while !self.should_quit {
            self.refresh_runtime_status();
            terminal
                .draw(|frame| self.render(frame))
                .map_err(|error| error.to_string())?;

            if event::poll(Duration::from_millis(120)).map_err(|error| error.to_string())? {
                if let Event::Key(key) = event::read().map_err(|error| error.to_string())? {
                    self.handle_key_event(key).await?;
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
                self.create_new_chat(self.initial_shell_args.clone())?;
                return Ok(());
            }
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                self.refresh_chats();
                self.status_message = "chat list refreshed".to_string();
                return Ok(());
            }
            (KeyCode::F(3), _) => {
                self.toggle_chat_list();
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
                    FocusArea::Input => FocusArea::Chats,
                };
                return Ok(());
            }
            _ => {}
        }

        match self.focus {
            FocusArea::Chats => self.handle_chat_list_key(key),
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

    fn handle_chat_list_key(&mut self, key: KeyEvent) -> Result<(), String> {
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
                    self.switch_to_chat(item.id.clone())?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) async fn submit_input(&mut self) -> Result<(), String> {
        if self.current_chat_is_loading() {
            self.status_message = "request already running".to_string();
            return Ok(());
        }

        let input = self.input.trim_end().to_string();
        if input.trim().is_empty() {
            return Ok(());
        }
        if input.starts_with('/') {
            self.input.clear();
            self.input_cursor = 0;
            self.handle_local_command(&input).await?;
            return Ok(());
        }

        let chat_id = self.current_chat_id()?;
        let attachment_paths = self.queued_attachment_paths.clone();
        self.follow_transcript = true;
        self.status_message = "connecting...".to_string();
        self.queued_attachment_paths.clear();
        self.input.clear();
        self.input_cursor = 0;

        let send_args = ChatSendArgs {
            chatId: Some(chat_id),
            message: input,
            attachmentPaths: attachment_paths,
            replyToTimestamp: None,
        };
        let active_chat_id = launch_chat_message_with_application(&mut self.application, send_args)?;
        self.refresh_chats();
        self.select_chat_by_id(&active_chat_id);
        self.last_current_chat_loading = true;
        self.awaiting_runtime_loading = true;
        self.status_message = format!(
            "streaming | chat={}",
            short_chat_label(&active_chat_id)
        );
        Ok(())
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
                self.create_new_chat(shell_args)?;
            }
            "switch" => {
                self.toggle_chat_list();
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
                self.status_message = if self.queued_attachment_paths.is_empty() {
                    "attachments=none".to_string()
                } else {
                    format!("attachments={}", self.queued_attachment_paths.join(", "))
                };
            }
            "clear-attachments" => {
                self.queued_attachment_paths.clear();
                self.status_message = "attachments cleared".to_string();
            }
            _ => {
                self.status_message = format!("unknown command: /{command}");
            }
        }
        Ok(())
    }

    fn create_new_chat(&mut self, shell_args: ShellArgs) -> Result<(), String> {
        if self.current_chat_is_loading() {
            self.status_message = "wait for current request to finish".to_string();
            return Ok(());
        }

        let core = self.application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
        core.createNewChat(
            shell_args.characterCardName,
            shell_args.group,
            true,
            true,
            shell_args.characterGroupId,
        );
        let chat_id = current_shell_chat_id(&mut self.application)?;
        self.follow_transcript = true;
        self.refresh_chats();
        self.select_chat_by_id(&chat_id);
        self.status_message = format!("new chat={chat_id}");
        Ok(())
    }

    fn toggle_chat_list(&mut self) {
        self.show_chat_list = !self.show_chat_list;
        if self.show_chat_list {
            self.focus = FocusArea::Chats;
            self.status_message = "chat list shown | Up/Down select | Enter switch | Esc close".to_string();
        } else {
            self.focus = FocusArea::Input;
            self.status_message = "chat list hidden".to_string();
        }
    }

    fn switch_to_chat(&mut self, chat_id: String) -> Result<(), String> {
        if self.current_chat_is_loading() {
            self.status_message = "wait for current request to finish".to_string();
            return Ok(());
        }

        ensure_chat_exists(&chat_id)?;
        let core = self.application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
        core.switchChat(chat_id.clone());
        self.follow_transcript = true;
        self.select_chat_by_id(&chat_id);
        self.status_message = format!("switched chat={chat_id}");
        Ok(())
    }

    fn refresh_chats(&mut self) {
        let current_chat_id = self.current_chat_id().ok();
        self.chats = {
            let core = self.application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
            load_chat_list_from_core(core)
        };
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

    pub(super) fn current_chat_id(&mut self) -> Result<String, String> {
        let core = self.application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
        core.currentChatIdFlow()
            .value()
            .ok_or_else(|| "no active chat in tui".to_string())
    }

    pub(super) fn current_messages(&mut self) -> Vec<ChatMessage> {
        let core = self.application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
        core.chatHistoryFlow().value()
    }

    fn current_chat_is_loading(&mut self) -> bool {
        self.last_current_chat_loading || self.raw_current_chat_is_loading()
    }

    fn raw_current_chat_is_loading(&mut self) -> bool {
        let core = self.application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
        core.currentChatIsLoading()
    }

    fn current_chat_input_processing_state(&mut self) -> InputProcessingState {
        let core = self.application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
        core.currentChatInputProcessingState()
    }

    fn refresh_runtime_status(&mut self) {
        let chat_id = self.current_chat_id().unwrap_or_default();
        let is_loading = self.raw_current_chat_is_loading();
        let state = self.current_chat_input_processing_state();
        if self.awaiting_runtime_loading && !is_loading {
            match state {
                InputProcessingState::Error { message } => {
                    self.awaiting_runtime_loading = false;
                    self.last_current_chat_loading = false;
                    self.status_message = message;
                }
                _ => {
                    self.follow_transcript = true;
                    self.status_message = format!("connecting | chat={}", short_chat_label(&chat_id));
                }
            }
            return;
        }
        if is_loading {
            self.awaiting_runtime_loading = false;
            self.follow_transcript = true;
            self.status_message = match state {
                InputProcessingState::Processing { message } => {
                    format!("processing | chat={} | {}", short_chat_label(&chat_id), message)
                }
                InputProcessingState::Connecting { message } => {
                    format!("connecting | chat={} | {}", short_chat_label(&chat_id), message)
                }
                InputProcessingState::Receiving { message } => {
                    format!("receiving | chat={} | {}", short_chat_label(&chat_id), message)
                }
                InputProcessingState::ExecutingTool { toolName } => {
                    format!("tool | chat={} | {}", short_chat_label(&chat_id), toolName)
                }
                InputProcessingState::ToolProgress { toolName, message, .. } => {
                    format!("tool | chat={} | {} {}", short_chat_label(&chat_id), toolName, message)
                }
                InputProcessingState::ProcessingToolResult { toolName } => {
                    format!("tool result | chat={} | {}", short_chat_label(&chat_id), toolName)
                }
                InputProcessingState::Summarizing { message } => {
                    format!("summarizing | chat={} | {}", short_chat_label(&chat_id), message)
                }
                InputProcessingState::ExecutingPlan { message } => {
                    format!("plan | chat={} | {}", short_chat_label(&chat_id), message)
                }
                InputProcessingState::Error { message } => message,
                InputProcessingState::Completed | InputProcessingState::Idle => {
                    format!("streaming | chat={}", short_chat_label(&chat_id))
                }
            };
        } else if self.last_current_chat_loading {
            self.awaiting_runtime_loading = false;
            self.follow_transcript = true;
            self.refresh_chats();
            self.status_message = format!("reply ready | chat={}", short_chat_label(&chat_id));
        }
        self.last_current_chat_loading = is_loading;
    }
}

fn load_chat_list_from_core(
    core: &mut operit_runtime::services::ChatServiceCore::ChatServiceCore,
) -> Vec<ChatListItem> {
    core.chatHistoriesFlow()
        .value()
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
            }
        })
        .collect()
}
