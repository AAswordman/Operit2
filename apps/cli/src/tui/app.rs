use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use operit_link::LocalCoreProxy;
use operit_runtime::core::tools::ToolPermissionSystem::{
    PermissionLevel, PermissionRequestResult,
};
use operit_runtime::data::model::ActivePrompt::ActivePrompt;
use operit_runtime::data::model::ChatHistory::ChatHistory;
use operit_runtime::data::model::CharacterCard::CharacterCardChatModelBindingMode;
use operit_runtime::data::model::ChatMessage::ChatMessage;
use operit_runtime::data::model::FunctionType::FunctionType;
use operit_runtime::data::model::InputProcessingState::InputProcessingState;
use operit_runtime::data::model::ModelConfigData::{
    getModelByIndex, getModelList, getValidModelIndex,
};
use operit_runtime::util::AppLogger::AppLogger;

use super::approval::TuiApprovalBridge;
use super::helpers::{short_chat_label, split_command_line};
use super::link_proxy_rs::TuiLocalCoreBorrowExt;
use super::typewriter::TypewriterState;
use crate::{parse_shell_args, ChatSendArgs, ShellArgs};

pub(super) struct OperitTui {
    pub(super) core: LocalCoreProxy,
    pub(super) initial_shell_args: ShellArgs,
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
    pub(super) approval_bridge: TuiApprovalBridge,
    pub(super) show_help: bool,
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
    pub(super) config_id: String,
    pub(super) config_name: String,
    pub(super) model_index: i32,
    pub(super) provider_name: &'static str,
    pub(super) model_name: String,
    pub(super) selected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FocusArea {
    Chats,
    ModelChooser,
    Input,
}

impl OperitTui {
    pub(super) fn new(
        mut core: LocalCoreProxy,
        initial_shell_args: ShellArgs,
        initial_chat_id: String,
    ) -> Result<Self, String> {
        let chats = {
            core.withMainChatCore(load_chat_list_from_core)
        };
        let selected_chat_index = chats
            .iter()
            .position(|item| item.id == initial_chat_id)
            .unwrap_or(0);
        let status_message =
            "F3 chats | Enter send | Esc cancel | Ctrl+J newline | Ctrl+N new chat | Ctrl+Q quit | ? help"
                .to_string();
        let _ = core
            .withMainChatCore(|core| core.currentChatIdFlow().value())
            .ok_or_else(|| "no active chat in tui".to_string())?;
        let approval_bridge = TuiApprovalBridge::new();
        {
            let bridge = approval_bridge.clone();
            core.withToolHandler(|handler| {
                handler
                .getToolPermissionSystem()
                .setPermissionRequester(move |tool, description| bridge.request(tool, description));
            });
        }
        Ok(Self {
            core,
            initial_shell_args,
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
            approval_bridge,
            show_help: false,
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
        if let Err(error) = execute!(stdout, EnterAlternateScreen).map_err(|error| error.to_string()) {
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
                execute!(terminal.backend_mut(), LeaveAlternateScreen)
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
            self.refresh_runtime_status();
            terminal
                .draw(|frame| self.render(frame))
                .map_err(|error| error.to_string())?;

            if event::poll(Duration::from_millis(16)).map_err(|error| error.to_string())? {
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
                if self.current_chat_is_loading() {
                    self.cancel_current_request()?;
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
            FocusArea::Chats => self.handle_chat_list_key(key),
            FocusArea::ModelChooser => self.handle_model_chooser_key(key),
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

    fn handle_model_chooser_key(&mut self, key: KeyEvent) -> Result<(), String> {
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
                self.apply_selected_model_choice()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn cancel_current_request(&mut self) -> Result<(), String> {
        let chat_id = self.current_chat_id()?;
        self.core
            .withMainChatCore(|core| core.cancelCurrentMessage());
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
        let result = self.core.beginChatMessage(send_args).await?;
        let active_chat_id = result.chatId;
        self.refresh_chats();
        self.select_chat_by_id(&active_chat_id);
        self.last_current_chat_loading = true;
        self.awaiting_runtime_loading = true;
        self.status_message = "streaming".to_string();
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
            "resume" => {
                self.resume_previous_chat()?;
            }
            "max" => {
                self.toggle_max_context_mode()?;
            }
            "model" => {
                self.handle_model_command(&parts[1..])?;
            }
            "approval" => {
                self.handle_approval_command(&parts[1..])?;
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

    fn handle_approval_command(&mut self, args: &[String]) -> Result<(), String> {
        let permissionSystem = self
            .core
            .withToolHandler(|handler| handler.getToolPermissionSystem());
        match args.first().map(String::as_str) {
            None | Some("status") => {
                let master = permissionSystem
                    .getMasterSwitch()
                    .map_err(|error| error.to_string())?;
                let overrides = permissionSystem
                    .getToolPermissionOverrides()
                    .map_err(|error| error.to_string())?;
                self.status_message = format!(
                    "approval master={} overrides={}",
                    master.name(),
                    overrides.len()
                );
            }
            Some("allow") | Some("ask") | Some("forbid") => {
                let level = parse_permission_level(args.first().map(String::as_str))?;
                permissionSystem
                    .saveMasterSwitch(level.clone())
                    .map_err(|error| error.to_string())?;
                self.status_message = format!("approval master={}", level.name());
            }
            Some("tool") => {
                let toolName = args
                    .get(1)
                    .ok_or_else(|| "usage: /approval tool <tool-name> <allow|ask|forbid|clear>".to_string())?;
                match args.get(2).map(String::as_str) {
                    Some("clear") => {
                        permissionSystem
                            .clearToolPermission(toolName)
                            .map_err(|error| error.to_string())?;
                        self.status_message = format!("approval cleared: {toolName}");
                    }
                    value @ (Some("allow") | Some("ask") | Some("forbid")) => {
                        let level = parse_permission_level(value)?;
                        permissionSystem
                            .saveToolPermission(toolName, level.clone())
                            .map_err(|error| error.to_string())?;
                        self.status_message = format!("approval {toolName}={}", level.name());
                    }
                    _ => {
                        return Err("usage: /approval tool <tool-name> <allow|ask|forbid|clear>".to_string());
                    }
                }
            }
            Some("list") => {
                let overrides = permissionSystem
                    .getToolPermissionOverrides()
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

    fn handle_model_command(&mut self, args: &[String]) -> Result<(), String> {
        match args.first().map(String::as_str) {
            None | Some("current") => self.show_current_chat_model(),
            Some("list") => self.list_chat_models(),
            Some("choose") => self.open_model_chooser(),
            Some("use") => self.use_chat_model(&args[1..]),
            Some("help") => {
                self.status_message =
                    "usage: /model current | /model list | /model choose | /model use <config-id> [model-index]"
                        .to_string();
                Ok(())
            }
            Some(other) => {
                self.status_message = format!("unknown /model command: {other}");
                Ok(())
            }
        }
    }

    fn show_current_chat_model(&mut self) -> Result<(), String> {
        let (config_id, actual_index, provider_name, selected_model_name) =
            self.current_chat_model_status_parts()?;
        self.status_message = format!(
            "CHAT -> {}[{}] {} / {}",
            config_id,
            actual_index,
            provider_name,
            selected_model_name
        );
        self.refresh_context_usage_label();
        Ok(())
    }

    fn current_chat_model_status_parts(&mut self) -> Result<(String, i32, &'static str, String), String> {
        let model_config_manager = self.core.withModelConfigManager(|manager| manager);
        let functional_config_manager = self.core.withFunctionalConfigManager(|manager| manager);
        model_config_manager
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;
        functional_config_manager
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;

        let mapping = functional_config_manager
            .getConfigMappingForFunction(FunctionType::CHAT)
            .map_err(|error| error.to_string())?;
        let config = model_config_manager
            .getModelConfig(&mapping.configId)
            .map_err(|error| error.to_string())?;
        let actual_index = getValidModelIndex(&config.modelName, mapping.modelIndex);
        let selected_model_name = getModelByIndex(&config.modelName, actual_index);
        Ok((
            mapping.configId,
            actual_index,
            config.apiProviderType.name(),
            selected_model_name,
        ))
    }

    fn current_chat_model_status_label(&mut self) -> Result<String, String> {
        let (_, _, _, selected_model_name) = self.current_chat_model_status_parts()?;
        Ok(selected_model_name)
    }

    fn list_chat_models(&mut self) -> Result<(), String> {
        let model_config_manager = self.core.withModelConfigManager(|manager| manager);
        model_config_manager
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;
        let mut entries = Vec::new();
        for config_id in model_config_manager
            .getConfigIds()
            .map_err(|error| error.to_string())?
        {
            let config = model_config_manager
                .getModelConfig(&config_id)
                .map_err(|error| error.to_string())?;
            let model_names = getModelList(&config.modelName);
            for (index, model_name) in model_names.into_iter().enumerate() {
                entries.push(format!(
                    "{}[{}]={}/{}",
                    config.id,
                    index,
                    config.apiProviderType.name(),
                    model_name
                ));
            }
        }
        self.status_message = format!("models: {}", entries.join(" | "));
        Ok(())
    }

    fn open_model_chooser(&mut self) -> Result<(), String> {
        self.model_choices = self.load_model_choices()?;
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

    fn load_model_choices(&mut self) -> Result<Vec<ModelChoiceItem>, String> {
        let model_config_manager = self.core.withModelConfigManager(|manager| manager);
        let functional_config_manager = self.core.withFunctionalConfigManager(|manager| manager);
        model_config_manager
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;
        functional_config_manager
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;

        let mapping = functional_config_manager
            .getConfigMappingForFunction(FunctionType::CHAT)
            .map_err(|error| error.to_string())?;
        let mut choices = Vec::new();
        for config_id in model_config_manager
            .getConfigIds()
            .map_err(|error| error.to_string())?
        {
            let config = model_config_manager
                .getModelConfig(&config_id)
                .map_err(|error| error.to_string())?;
            let active_model_index = if mapping.configId == config.id {
                getValidModelIndex(&config.modelName, mapping.modelIndex)
            } else {
                -1
            };
            for (index, model_name) in getModelList(&config.modelName).into_iter().enumerate() {
                let model_index = index as i32;
                choices.push(ModelChoiceItem {
                    config_id: config.id.clone(),
                    config_name: config.name.clone(),
                    model_index,
                    provider_name: config.apiProviderType.name(),
                    model_name,
                    selected: mapping.configId == config.id && active_model_index == model_index,
                });
            }
        }
        Ok(choices)
    }

    fn apply_selected_model_choice(&mut self) -> Result<(), String> {
        let choice = self
            .model_choices
            .get(self.selected_model_choice_index)
            .cloned()
            .ok_or_else(|| "no selected model".to_string())?;
        self.apply_chat_model_choice(&choice)?;
        self.show_model_chooser = false;
        self.focus = FocusArea::Input;
        Ok(())
    }

    fn use_chat_model(&mut self, args: &[String]) -> Result<(), String> {
        let config_id = match args.first() {
            Some(value) if !value.trim().is_empty() => value.trim().to_string(),
            _ => {
                self.status_message = "usage: /model use <config-id> [model-index]".to_string();
                return Ok(());
            }
        };
        let requested_model_index = parse_optional_model_index(args.get(1))?;

        let model_config_manager = self.core.withModelConfigManager(|manager| manager);
        let functional_config_manager = self.core.withFunctionalConfigManager(|manager| manager);
        model_config_manager
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;
        functional_config_manager
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;
        let config = model_config_manager
            .getModelConfig(&config_id)
            .map_err(|error| error.to_string())?;
        let model_names = getModelList(&config.modelName);
        if model_names.is_empty() {
            self.status_message = format!("model config has no modelName: {config_id}");
            return Ok(());
        }
        if requested_model_index < 0 || requested_model_index as usize >= model_names.len() {
            self.status_message = format!(
                "model index out of range: {} (available 0..{})",
                requested_model_index,
                model_names.len().saturating_sub(1)
            );
            return Ok(());
        }

        let choice = ModelChoiceItem {
            config_id,
            config_name: config.name,
            model_index: requested_model_index,
            provider_name: config.apiProviderType.name(),
            model_name: model_names[requested_model_index as usize].clone(),
            selected: true,
        };
        self.apply_chat_model_choice(&choice)?;
        Ok(())
    }

    fn apply_chat_model_choice(&mut self, choice: &ModelChoiceItem) -> Result<(), String> {
        let functional_config_manager = self.core.withFunctionalConfigManager(|manager| manager);
        functional_config_manager
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;
        functional_config_manager
            .setConfigForFunctionWithIndex(
                FunctionType::CHAT,
                choice.config_id.clone(),
                choice.model_index,
            )
            .map_err(|error| error.to_string())?;
        {
            self.core.withMainChatCore(|core| {
                if let Some(service) = core.enhancedAiService.as_mut() {
                    service.refreshServiceForFunction(FunctionType::CHAT);
                }
            });
        }
        self.status_message = format!(
            "CHAT -> {}[{}] {} / {}",
            choice.config_id,
            choice.model_index,
            choice.provider_name,
            choice.model_name
        );
        self.refresh_context_usage_label();
        Ok(())
    }

    fn toggle_max_context_mode(&mut self) -> Result<(), String> {
        let config_id = self.resolve_editable_chat_config_id()?;
        let model_config_manager = self.core.withModelConfigManager(|manager| manager);
        model_config_manager
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;
        let current = model_config_manager
            .getModelConfig(&config_id)
            .map_err(|error| error.to_string())?;
        let new_value = !current.enableMaxContextMode;
        let updated = model_config_manager
            .updateContextSettings(
                &config_id,
                current.contextLength,
                current.maxContextLength,
                new_value,
            )
            .map_err(|error| error.to_string())?;
        let effective_context_length = if updated.enableMaxContextMode {
            updated.maxContextLength
        } else {
            updated.contextLength
        };
        self.status_message = format!(
            "context config={} | context={}K",
            config_id,
            format_context_length(effective_context_length)
        );
        self.refresh_context_usage_label();
        Ok(())
    }

    fn resolve_editable_chat_config_id(&mut self) -> Result<String, String> {
        let active_prompt_manager = self.core.withActivePromptManager(|manager| manager);
        if let ActivePrompt::CharacterCard { id } = active_prompt_manager
            .getActivePrompt()
            .map_err(|error| error.to_string())?
        {
            let character_card_manager = self.core.withCharacterCardManager(|manager| manager);
            let card = character_card_manager
                .getCharacterCard(&id)
                .map_err(|error| error.to_string())?;
            let binding_mode =
                CharacterCardChatModelBindingMode::normalize(Some(card.chatModelBindingMode.as_str()));
            if binding_mode == CharacterCardChatModelBindingMode::FIXED_CONFIG {
                if let Some(config_id) = card
                    .chatModelConfigId
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                {
                    return Ok(config_id);
                }
            }
        }

        let functional_config_manager = self.core.withFunctionalConfigManager(|manager| manager);
        functional_config_manager
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;
        functional_config_manager
            .getConfigIdForFunction(FunctionType::CHAT)
            .map_err(|error| error.to_string())
    }

    fn create_new_chat(&mut self, shell_args: ShellArgs) -> Result<(), String> {
        if self.current_chat_is_loading() {
            self.status_message = "wait for current request to finish".to_string();
            return Ok(());
        }

        let chat_id = self.core.withMainChatCore(|core| {
            core.createNewChat(
                shell_args.characterCardName,
                shell_args.group,
                true,
                true,
                shell_args.characterGroupId,
            );
            core.currentChatIdFlow().value()
        })
        .ok_or_else(|| "core did not create chat".to_string())?;
        self.follow_transcript = true;
        self.refresh_chats();
        self.select_chat_by_id(&chat_id);
        self.status_message = "new chat".to_string();
        Ok(())
    }

    fn toggle_chat_list(&mut self) {
        self.show_chat_list = !self.show_chat_list;
        if self.show_chat_list {
            self.focus = FocusArea::Chats;
            self.refresh_chats();
            if let Ok(chat_id) = self.current_chat_id() {
                self.select_chat_by_id(&chat_id);
            }
            self.status_message = "chat list shown | Up/Down select | Enter switch | Esc close".to_string();
        } else {
            self.focus = FocusArea::Input;
            self.status_message = "chat list hidden".to_string();
        }
    }

    fn resume_previous_chat(&mut self) -> Result<(), String> {
        if self.current_chat_is_loading() {
            self.status_message = "wait for current request to finish".to_string();
            return Ok(());
        }

        self.refresh_chats();
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
            .withMainChatCore(|core| core.switchChat(target.id.clone()));
        self.follow_transcript = true;
        self.select_chat_by_id(&target.id);
        self.status_message = format!("resumed chat: {}", target.title);
        Ok(())
    }

    fn switch_to_chat(&mut self, chat_id: String) -> Result<(), String> {
        if self.current_chat_is_loading() {
            self.status_message = "wait for current request to finish".to_string();
            return Ok(());
        }

        let exists = self.core.withMainChatCore(|core| {
            core.chatHistoriesFlow()
                .value()
                .iter()
                .any(|chat| chat.id == chat_id)
        });
        if !exists {
            return Err(format!("chat not found: {chat_id}"));
        }
        self.core
            .withMainChatCore(|core| core.switchChat(chat_id.clone()));
        self.follow_transcript = true;
        self.select_chat_by_id(&chat_id);
        self.status_message = "switched chat".to_string();
        Ok(())
    }

    fn refresh_chats(&mut self) {
        let current_chat_id = self.current_chat_id().ok();
        self.chats = {
            self.core.withMainChatCore(load_chat_list_from_core)
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
        self.core
            .withMainChatCore(|core| core.currentChatIdFlow().value())
            .ok_or_else(|| "no active chat in tui".to_string())
    }

    pub(super) fn current_messages(&mut self) -> Vec<ChatMessage> {
        self.core
            .withMainChatCore(|core| core.chatHistoryFlow().value())
    }

    pub(super) fn current_chat_is_loading(&mut self) -> bool {
        self.last_current_chat_loading || self.raw_current_chat_is_loading()
    }

    fn raw_current_chat_is_loading(&mut self) -> bool {
        self.core.withMainChatCore(|core| core.currentChatIsLoading())
    }

    pub(super) fn current_chat_input_processing_state(&mut self) -> InputProcessingState {
        self.core
            .withMainChatCore(|core| core.currentChatInputProcessingState())
    }

    fn refresh_runtime_status(&mut self) {
        self.refresh_context_usage_label();
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
                InputProcessingState::Idle => match self.current_chat_model_status_label() {
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
            self.refresh_chats();
            match self.current_chat_model_status_label() {
                Ok(label) => self.set_status_message(label),
                Err(error) => self.set_status_message(error),
            }
        } else if matches!(state, InputProcessingState::Idle | InputProcessingState::Completed) {
            match self.current_chat_model_status_label() {
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

    fn refresh_context_usage_label(&mut self) {
        match self.current_context_usage_label() {
            Ok(label) => {
                self.context_usage_label = label;
            }
            Err(_) => {
                self.context_usage_label.clear();
            }
        }
    }

    fn current_context_usage_label(&mut self) -> Result<String, String> {
        let config_id = self.resolve_editable_chat_config_id()?;
        let model_config_manager = self.core.withModelConfigManager(|manager| manager);
        model_config_manager
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;
        let config = model_config_manager
            .getModelConfig(&config_id)
            .map_err(|error| error.to_string())?;
        let effective_context_length = if config.enableMaxContextMode {
            config.maxContextLength
        } else {
            config.contextLength
        };
        let max_tokens = (effective_context_length * 1024.0) as i32;
        let current_window_size = {
            self.core
                .withMainChatCore(|core| core.currentWindowSizeFlow().value())
        };
        if max_tokens <= 0 {
            return Ok(format!("context {} / {}", current_window_size.max(0), max_tokens));
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
                updated_at: chat
                    .updatedAt
                    .parse::<i64>()
                    .expect("chat.updatedAt must be epoch millis"),
                display_order: chat.displayOrder,
            }
        })
        .collect()
}

fn format_context_length(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i32)
    } else {
        format!("{value:.1}")
    }
}

fn parse_optional_model_index(value: Option<&String>) -> Result<i32, String> {
    match value {
        Some(value) => value.parse::<i32>().map_err(|error| error.to_string()),
        None => Ok(0),
    }
}
