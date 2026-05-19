use std::io::{self, Stdout};
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use tokio::runtime::Builder;

use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::data::model::ChatMessage::ChatMessage;
use operit_runtime::data::repository::ChatHistoryManager::ChatHistoryManager;

use super::helpers::{
    centered_rect, char_to_byte_index, render_message_lines, short_chat_label,
    split_command_line, transcript_max_scroll, wrap_approx_lines,
};
use crate::{
    create_cli_application, current_shell_chat_id, ensure_chat_exists, parse_shell_args,
    send_chat_message_with_application, ChatSendArgs, ChatSendResult, ShellArgs,
};

pub(super) struct OperitTui {
    application: OperitApplication,
    initial_shell_args: ShellArgs,
    chats: Vec<ChatListItem>,
    selected_chat_index: usize,
    focus: FocusArea,
    input: String,
    input_cursor: usize,
    queued_attachment_paths: Vec<String>,
    status_message: String,
    transcript_scroll: u16,
    follow_transcript: bool,
    ctrl_c_pending: bool,
    pending_send: Option<thread::JoinHandle<Result<ChatSendResult, String>>>,
    pending_preview: Option<PendingSendPreview>,
    show_help: bool,
    should_quit: bool,
}

#[derive(Clone, Debug)]
struct ChatListItem {
    id: String,
    title: String,
    secondary: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FocusArea {
    Chats,
    Input,
}

struct PendingSendPreview {
    chat_id: String,
    messages: Vec<ChatMessage>,
}

impl OperitTui {
    pub(super) fn new(
        mut application: OperitApplication,
        initial_shell_args: ShellArgs,
        initial_chat_id: String,
    ) -> Result<Self, String> {
        let chats = load_chat_list()?;
        let selected_chat_index = chats
            .iter()
            .position(|item| item.id == initial_chat_id)
            .unwrap_or(0);
        let status_message = format!(
            "chat={} | Tab switch focus | Enter send | Ctrl+J newline | Ctrl+N new chat | Ctrl+Q quit | ? help",
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
            queued_attachment_paths: Vec::new(),
            status_message,
            transcript_scroll: 0,
            follow_transcript: true,
            ctrl_c_pending: false,
            pending_send: None,
            pending_preview: None,
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
            self.poll_pending_send().await?;
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

    async fn poll_pending_send(&mut self) -> Result<(), String> {
        let finished = self
            .pending_send
            .as_ref()
            .map(|handle| handle.is_finished())
            .unwrap_or(false);
        if !finished {
            return Ok(());
        }

        let handle = self.pending_send.take().expect("pending send vanished");
        let result = handle
            .join()
            .map_err(|_| "background send panicked".to_string())?;
        let preview_chat_id = self.pending_preview.as_ref().map(|preview| preview.chat_id.clone());
        self.pending_preview = None;

        match result {
            Ok(result) => {
                let core = self.application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
                core.switchChat(result.chatId.clone());
                self.follow_transcript = true;
                self.refresh_chats()?;
                self.select_chat_by_id(&result.chatId);
                self.status_message = format!(
                    "reply ready | chat={} | provider={} | model={} | out={}",
                    short_chat_label(&result.chatId),
                    result.aiMessage.provider,
                    result.aiMessage.modelName,
                    result.aiMessage.outputTokens,
                );
            }
            Err(error) => {
                if let Some(chat_id) = preview_chat_id {
                    let core = self.application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
                    core.switchChat(chat_id);
                }
                self.follow_transcript = true;
                self.status_message = error;
            }
        }

        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.area());

        self.render_header(frame, root[0]);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(28), Constraint::Min(0)])
            .split(root[1]);

        self.render_chat_list(frame, body[0]);

        let main = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(7)])
            .split(body[1]);

        self.render_transcript(frame, main[0]);
        self.render_input(frame, main[1]);
        self.render_footer(frame, root[2]);

        if self.show_help {
            self.render_help_modal(frame);
        }
    }

    fn render_header(&mut self, frame: &mut Frame, area: Rect) {
        let current_chat_id = self.current_chat_id().unwrap_or_default();
        let focus_label = match self.focus {
            FocusArea::Chats => "chats",
            FocusArea::Input => "input",
        };
        let attachment_count = self.queued_attachment_paths.len();
        let spans = Line::from(vec![
            Span::styled(" Operit2 ", Style::default().fg(Color::Black).bg(Color::Cyan)),
            Span::raw(" "),
            Span::styled(
                format!("chat={} ", short_chat_label(&current_chat_id)),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("focus={} ", focus_label)),
            Span::raw(format!("attachments={} ", attachment_count)),
        ]);
        frame.render_widget(Paragraph::new(spans), area);
    }

    fn render_chat_list(&self, frame: &mut Frame, area: Rect) {
        let items = if self.chats.is_empty() {
            vec![ListItem::new(Line::from("no chats"))]
        } else {
            self.chats
                .iter()
                .map(|item| {
                    ListItem::new(vec![
                        Line::from(Span::styled(
                            item.title.clone(),
                            Style::default().add_modifier(Modifier::BOLD),
                        )),
                        Line::from(Span::styled(
                            item.secondary.clone(),
                            Style::default().fg(Color::DarkGray),
                        )),
                    ])
                })
                .collect::<Vec<_>>()
        };

        let border_style = if self.focus == FocusArea::Chats {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };
        let list = List::new(items)
            .block(Block::default().title("Chats").borders(Borders::ALL).border_style(border_style))
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");
        let mut state = ListState::default();
        if !self.chats.is_empty() {
            state.select(Some(self.selected_chat_index.min(self.chats.len().saturating_sub(1))));
        }
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn render_transcript(&mut self, frame: &mut Frame, area: Rect) {
        let messages = self.current_messages();
        let transcript_lines = render_message_lines(&messages);
        let max_scroll = transcript_max_scroll(&transcript_lines, area);
        if self.follow_transcript {
            self.transcript_scroll = max_scroll;
        } else if self.transcript_scroll > max_scroll {
            self.transcript_scroll = max_scroll;
        }

        let paragraph = Paragraph::new(Text::from(transcript_lines))
            .block(Block::default().title("Conversation").borders(Borders::ALL))
            .wrap(Wrap { trim: false })
            .scroll((self.transcript_scroll, 0));
        frame.render_widget(paragraph, area);
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focus == FocusArea::Input {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };
        let input_block = Block::default()
            .title("Input")
            .borders(Borders::ALL)
            .border_style(border_style);
        let inner = input_block.inner(area);
        let visible_text = self.input_view_text(inner.width.saturating_sub(1) as usize, inner.height as usize);
        let input = Paragraph::new(visible_text)
            .block(input_block)
            .wrap(Wrap { trim: false });
        frame.render_widget(input, area);

        if self.focus == FocusArea::Input && !self.show_help {
            let (cursor_x, cursor_y) = self.cursor_position(inner.width.saturating_sub(1) as usize, inner.height as usize);
            frame.set_cursor_position((inner.x + cursor_x as u16, inner.y + cursor_y as u16));
        }
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let text = if self.status_message.is_empty() {
            "Ready".to_string()
        } else {
            self.status_message.clone()
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {text}"),
                Style::default().fg(Color::DarkGray),
            ))),
            area,
        );
    }

    fn render_help_modal(&self, frame: &mut Frame) {
        let popup = centered_rect(72, 60, frame.area());
        frame.render_widget(Clear, popup);
        let lines = vec![
            Line::from(Span::styled(
                "Operit2 TUI",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("Tab: switch focus between chat list and input"),
            Line::from("Enter: send message / activate selected chat"),
            Line::from("Ctrl+J: insert newline in input"),
            Line::from("Ctrl+N: create new chat"),
            Line::from("Ctrl+C: press twice to quit"),
            Line::from("Ctrl+Q: quit"),
            Line::from("PageUp/PageDown: scroll conversation"),
            Line::from("Esc: close help / clear status"),
            Line::from(""),
            Line::from("Local commands:"),
            Line::from("/help"),
            Line::from("/new [--character <name>] [--group-card <id>] [--group <name>]"),
            Line::from("/switch <chat-id>"),
            Line::from("/attach <path>"),
            Line::from("/attachments"),
            Line::from("/clear-attachments"),
            Line::from("/quit"),
        ];
        let help = Paragraph::new(Text::from(lines))
            .block(Block::default().title("Help").borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        frame.render_widget(help, popup);
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
                self.refresh_chats()?;
                self.status_message = "chat list refreshed".to_string();
                return Ok(());
            }
            (KeyCode::PageUp, _) => {
                self.follow_transcript = false;
                self.transcript_scroll = self.transcript_scroll.saturating_sub(8);
                return Ok(());
            }
            (KeyCode::PageDown, _) => {
                self.follow_transcript = false;
                self.transcript_scroll = self.transcript_scroll.saturating_add(8);
                return Ok(());
            }
            (KeyCode::Esc, _) => {
                self.status_message.clear();
                self.focus = FocusArea::Input;
                return Ok(());
            }
            (KeyCode::Char('?'), _) | (KeyCode::F(1), _) => {
                self.show_help = true;
                return Ok(());
            }
            (KeyCode::Tab, _) => {
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

    async fn handle_input_key(&mut self, key: KeyEvent) -> Result<(), String> {
        match (key.code, key.modifiers) {
            (KeyCode::Enter, KeyModifiers::NONE) => {
                self.submit_input().await?;
            }
            (KeyCode::Char('j'), KeyModifiers::CONTROL) => self.insert_char('\n'),
            (KeyCode::Backspace, _) => self.delete_before_cursor(),
            (KeyCode::Delete, _) => self.delete_at_cursor(),
            (KeyCode::Left, _) => self.move_cursor_left(),
            (KeyCode::Right, _) => self.move_cursor_right(),
            (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.move_cursor_home()
            }
            (KeyCode::End, _) | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.move_cursor_end()
            }
            (KeyCode::Char(ch), modifiers)
                if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT =>
            {
                self.insert_char(ch);
            }
            _ => {}
        }
        Ok(())
    }

    async fn submit_input(&mut self) -> Result<(), String> {
        if self.pending_send.is_some() {
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
        let mut preview_messages = self.current_messages();
        preview_messages.push(ChatMessage::new_with_content("user".to_string(), input.clone()));
        preview_messages.push(ChatMessage::new_with_content(
            "ai".to_string(),
            "connecting...".to_string(),
        ));
        self.pending_preview = Some(PendingSendPreview {
            chat_id: chat_id.clone(),
            messages: preview_messages,
        });
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
        self.pending_send = Some(thread::spawn(move || {
            let runtime = Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            let mut application = create_cli_application();
            application.onCreate()?;
            runtime.block_on(async move {
                send_chat_message_with_application(&mut application, send_args).await
            })
        }));
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
                let chat_id = parts
                    .get(1)
                    .ok_or_else(|| "usage: /switch <chat-id>".to_string())?
                    .clone();
                self.switch_to_chat(chat_id)?;
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
        if self.pending_send.is_some() {
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
        self.refresh_chats()?;
        self.select_chat_by_id(&chat_id);
        self.status_message = format!("new chat={chat_id}");
        Ok(())
    }

    fn switch_to_chat(&mut self, chat_id: String) -> Result<(), String> {
        if self.pending_send.is_some() {
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

    fn refresh_chats(&mut self) -> Result<(), String> {
        let current_chat_id = self.current_chat_id().ok();
        self.chats = load_chat_list()?;
        if let Some(chat_id) = current_chat_id {
            self.select_chat_by_id(&chat_id);
        } else if self.selected_chat_index >= self.chats.len() {
            self.selected_chat_index = self.chats.len().saturating_sub(1);
        }
        Ok(())
    }

    fn select_chat_by_id(&mut self, chat_id: &str) {
        if let Some(index) = self.chats.iter().position(|item| item.id == chat_id) {
            self.selected_chat_index = index;
        }
    }

    fn current_chat_id(&mut self) -> Result<String, String> {
        if let Some(preview) = self.pending_preview.as_ref() {
            return Ok(preview.chat_id.clone());
        }

        let core = self.application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
        core.currentChatId()
            .clone()
            .ok_or_else(|| "no active chat in tui".to_string())
    }

    fn current_messages(&mut self) -> Vec<ChatMessage> {
        if let Some(preview) = self.pending_preview.as_ref() {
            return preview.messages.clone();
        }

        let core = self.application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
        core.chatHistory().clone()
    }

    fn insert_char(&mut self, ch: char) {
        let byte_index = char_to_byte_index(&self.input, self.input_cursor);
        self.input.insert(byte_index, ch);
        self.input_cursor += 1;
    }

    fn delete_before_cursor(&mut self) {
        if self.input_cursor == 0 {
            return;
        }
        let start = char_to_byte_index(&self.input, self.input_cursor - 1);
        let end = char_to_byte_index(&self.input, self.input_cursor);
        self.input.replace_range(start..end, "");
        self.input_cursor -= 1;
    }

    fn delete_at_cursor(&mut self) {
        if self.input_cursor >= self.input.chars().count() {
            return;
        }
        let start = char_to_byte_index(&self.input, self.input_cursor);
        let end = char_to_byte_index(&self.input, self.input_cursor + 1);
        self.input.replace_range(start..end, "");
    }

    fn move_cursor_left(&mut self) {
        self.input_cursor = self.input_cursor.saturating_sub(1);
    }

    fn move_cursor_right(&mut self) {
        let char_count = self.input.chars().count();
        if self.input_cursor < char_count {
            self.input_cursor += 1;
        }
    }

    fn move_cursor_home(&mut self) {
        self.input_cursor = 0;
    }

    fn move_cursor_end(&mut self) {
        self.input_cursor = self.input.chars().count();
    }

    fn input_view_text(&self, width: usize, height: usize) -> String {
        let text = if self.input.is_empty() {
            String::new()
        } else {
            self.input.clone()
        };
        let lines = wrap_approx_lines(&text, width.max(1));
        let visible_height = height.saturating_sub(1).max(1);
        let start = lines.len().saturating_sub(visible_height);
        lines[start..].join("\n")
    }

    fn cursor_position(&self, width: usize, height: usize) -> (usize, usize) {
        let prefix = self.input.chars().take(self.input_cursor).collect::<String>();
        let lines = wrap_approx_lines(&prefix, width.max(1));
        let visible_height = height.saturating_sub(1).max(1);
        let line_index = lines.len().saturating_sub(1);
        let start = lines.len().saturating_sub(visible_height);
        let visible_line = line_index.saturating_sub(start);
        let col = lines.last().map(|line| line.chars().count()).unwrap_or(0);
        (
            col.min(width.saturating_sub(1)),
            visible_line.min(height.saturating_sub(1)),
        )
    }
}

fn load_chat_list() -> Result<Vec<ChatListItem>, String> {
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    let chats = manager
        .chatHistoriesFlow()
        .map_err(|error| error.to_string())?;
    Ok(chats
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
        .collect())
}
