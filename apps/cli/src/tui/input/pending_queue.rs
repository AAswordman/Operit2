use std::ops::Range;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use operit_runtime::data::model::InputProcessingState::InputProcessingState;

use super::app::{FocusArea, OperitTui};
use super::helpers::display_width;
use crate::ChatSendArgs;

const PENDING_QUEUE_AUTO_SEND_DELAY: Duration = Duration::from_millis(250);
const PENDING_QUEUE_VISIBLE_ITEMS: usize = 3;

#[derive(Clone, Debug)]
pub(super) struct PendingQueueMessage {
    pub(super) id: u64,
    pub(super) text: String,
}

impl OperitTui {
    pub(super) fn ensure_pending_queue_chat_id(&mut self) {
        let chat_id = self.current_chat_id_cache.clone();
        if self.pending_queue_chat_id == chat_id {
            return;
        }
        self.pending_queue_chat_id = chat_id;
        self.pending_queue_messages.clear();
        self.selected_pending_queue_index = 0;
        self.next_pending_queue_id = 1;
        self.was_pending_queue_blocked = self.is_pending_queue_blocked();
        self.suppress_next_pending_queue_auto_send = false;
        self.pending_queue_auto_send_at = None;
        self.pending_queue_manual_send = None;
        if self.focus == FocusArea::Queue {
            self.focus = FocusArea::Input;
        }
    }

    pub(super) fn enqueue_pending_message_from_input(&mut self) -> bool {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return false;
        }
        let id = self.next_pending_queue_id;
        self.next_pending_queue_id += 1;
        self.pending_queue_messages
            .push_back(PendingQueueMessage { id, text });
        self.clamp_pending_queue_selection();
        self.input.clear();
        self.input_cursor = 0;
        self.autocomplete_index = 0;
        self.status_message = self
            .text()
            .queued_message(id, self.pending_queue_messages.len());
        true
    }

    pub(super) async fn advance_pending_message_queue(&mut self) -> Result<(), String> {
        self.ensure_pending_queue_chat_id();
        let blocked = self.is_pending_queue_blocked();
        if blocked {
            self.was_pending_queue_blocked = true;
            self.pending_queue_auto_send_at = None;
            return Ok(());
        }

        if let Some(message) = self.pending_queue_manual_send.take() {
            self.suppress_next_pending_queue_auto_send = false;
            self.pending_queue_auto_send_at = None;
            return self.send_pending_queue_message(message).await;
        }

        if self.was_pending_queue_blocked {
            if self.suppress_next_pending_queue_auto_send {
                self.suppress_next_pending_queue_auto_send = false;
                self.pending_queue_auto_send_at = None;
            } else if !self.pending_queue_messages.is_empty()
                && self.pending_queue_auto_send_at.is_none()
            {
                self.pending_queue_auto_send_at =
                    Some(Instant::now() + PENDING_QUEUE_AUTO_SEND_DELAY);
            }
        }

        self.was_pending_queue_blocked = false;
        if self.pending_queue_messages.is_empty() {
            self.pending_queue_auto_send_at = None;
            return Ok(());
        }

        let Some(send_at) = self.pending_queue_auto_send_at else {
            return Ok(());
        };
        if Instant::now() < send_at {
            return Ok(());
        }

        self.pending_queue_auto_send_at = None;
        let message = self.remove_pending_queue_message_at(0);
        self.send_pending_queue_message(message).await
    }

    pub(super) async fn handle_pending_queue_command(
        &mut self,
        args: &[String],
    ) -> Result<(), String> {
        self.ensure_pending_queue_chat_id();
        match args.first().map(String::as_str) {
            None | Some("status") => {
                self.status_message = self.pending_queue_status();
            }
            Some("clear") => {
                let count = self.pending_queue_messages.len();
                self.pending_queue_messages.clear();
                self.pending_queue_auto_send_at = None;
                self.clamp_pending_queue_selection();
                self.status_message = self.text().message_queue_cleared(count);
            }
            Some("delete") => {
                let id = pending_queue_command_id(args, self.text().queue_delete_usage())?;
                self.remove_pending_queue_message_by_id(id)?;
                self.status_message = self.text().deleted_queued_message(id);
            }
            Some("edit") => {
                let id = pending_queue_command_id(args, self.text().queue_edit_usage())?;
                let message = self.remove_pending_queue_message_by_id(id)?;
                self.input = message.text;
                self.input_cursor = self.input.chars().count();
                self.autocomplete_index = 0;
                self.focus = FocusArea::Input;
                self.status_message = self.text().editing_queued_message(id);
            }
            Some("send") => {
                let id = pending_queue_command_id(args, self.text().queue_send_usage())?;
                let message = self.remove_pending_queue_message_by_id(id)?;
                self.send_pending_queue_message_now(message).await?;
            }
            Some(command) => {
                return Err(self.text().unknown_queue_command(command));
            }
        }
        Ok(())
    }

    pub(super) async fn handle_pending_queue_key(&mut self, key: KeyEvent) -> Result<(), String> {
        self.ensure_pending_queue_chat_id();
        if self.pending_queue_messages.is_empty() {
            self.focus = FocusArea::Input;
            return Ok(());
        }

        match key.code {
            KeyCode::Up => self.move_pending_queue_selection_up(),
            KeyCode::Down => self.move_pending_queue_selection_down(),
            KeyCode::Home => {
                self.selected_pending_queue_index = 0;
            }
            KeyCode::End => {
                self.selected_pending_queue_index = self.pending_queue_messages.len() - 1;
            }
            KeyCode::Enter => {
                self.send_selected_pending_queue_message().await?;
            }
            KeyCode::Delete | KeyCode::Backspace => {
                self.delete_selected_pending_queue_message();
            }
            KeyCode::Char(ch) if pending_queue_plain_key(key.modifiers) => match ch {
                'e' | 'E' => self.edit_selected_pending_queue_message(),
                's' | 'S' => self.send_selected_pending_queue_message().await?,
                _ => {}
            },
            _ => {}
        }
        self.clamp_pending_queue_selection();
        Ok(())
    }

    pub(super) fn clamp_pending_queue_selection(&mut self) {
        if self.pending_queue_messages.is_empty() {
            self.selected_pending_queue_index = 0;
            if self.focus == FocusArea::Queue {
                self.focus = FocusArea::Input;
            }
            return;
        }

        let last_index = self.pending_queue_messages.len() - 1;
        self.selected_pending_queue_index = self.selected_pending_queue_index.min(last_index);
    }

    pub(super) fn pending_queue_panel_line_count(&self) -> u16 {
        if self.pending_queue_messages.is_empty() {
            return 0;
        }
        let item_count = self
            .pending_queue_messages
            .len()
            .min(PENDING_QUEUE_VISIBLE_ITEMS);
        let more_line =
            usize::from(self.pending_queue_messages.len() > PENDING_QUEUE_VISIBLE_ITEMS);
        (1 + item_count + more_line) as u16
    }

    fn is_pending_queue_blocked(&mut self) -> bool {
        self.current_chat_is_loading()
            || matches!(
                &self.current_chat_input_processing_state_cache,
                InputProcessingState::Connecting { .. }
                    | InputProcessingState::ExecutingTool { .. }
                    | InputProcessingState::ToolProgress { .. }
                    | InputProcessingState::Processing { .. }
                    | InputProcessingState::ProcessingToolResult { .. }
                    | InputProcessingState::Summarizing { .. }
                    | InputProcessingState::Receiving { .. }
            )
    }

    async fn send_pending_queue_message_now(
        &mut self,
        message: PendingQueueMessage,
    ) -> Result<(), String> {
        if self.is_pending_queue_blocked() {
            let id = message.id;
            self.pending_queue_manual_send = Some(message);
            self.suppress_next_pending_queue_auto_send = true;
            self.pending_queue_auto_send_at = None;
            self.cancel_current_request().await?;
            self.status_message = self.text().queued_message_selected(id);
            return Ok(());
        }
        self.send_pending_queue_message(message).await
    }

    async fn send_pending_queue_message(
        &mut self,
        message: PendingQueueMessage,
    ) -> Result<(), String> {
        let chat_id = self.current_chat_id()?;
        self.follow_transcript = true;
        self.status_message = self.text().connecting().to_string();
        let send_args = ChatSendArgs {
            chatId: Some(chat_id),
            message: message.text,
            attachmentPaths: Vec::new(),
            replyToTimestamp: None,
        };
        let active_chat_id = self.begin_chat_message(send_args, Vec::new()).await?;
        self.refresh_chats().await;
        self.select_chat_by_id(&active_chat_id);
        self.last_current_chat_loading = true;
        self.awaiting_runtime_loading = true;
        self.status_message = self.text().streaming().to_string();
        Ok(())
    }

    fn move_pending_queue_selection_up(&mut self) {
        self.selected_pending_queue_index = self.selected_pending_queue_index.saturating_sub(1);
    }

    fn move_pending_queue_selection_down(&mut self) {
        if self.selected_pending_queue_index + 1 < self.pending_queue_messages.len() {
            self.selected_pending_queue_index += 1;
        }
    }

    fn delete_selected_pending_queue_message(&mut self) {
        let message = self.remove_selected_pending_queue_message();
        self.status_message = self.text().deleted_queued_message(message.id);
    }

    fn edit_selected_pending_queue_message(&mut self) {
        let message = self.remove_selected_pending_queue_message();
        let id = message.id;
        self.input = message.text;
        self.input_cursor = self.input.chars().count();
        self.autocomplete_index = 0;
        self.focus = FocusArea::Input;
        self.status_message = self.text().editing_queued_message(id);
    }

    async fn send_selected_pending_queue_message(&mut self) -> Result<(), String> {
        let message = self.remove_selected_pending_queue_message();
        self.send_pending_queue_message_now(message).await
    }

    fn remove_selected_pending_queue_message(&mut self) -> PendingQueueMessage {
        self.clamp_pending_queue_selection();
        self.remove_pending_queue_message_at(self.selected_pending_queue_index)
    }

    fn remove_pending_queue_message_by_id(
        &mut self,
        id: u64,
    ) -> Result<PendingQueueMessage, String> {
        let index = self
            .pending_queue_messages
            .iter()
            .position(|message| message.id == id)
            .ok_or_else(|| self.text().queued_message_not_found(id))?;
        Ok(self.remove_pending_queue_message_at(index))
    }

    fn remove_pending_queue_message_at(&mut self, index: usize) -> PendingQueueMessage {
        let message = self
            .pending_queue_messages
            .remove(index)
            .expect("pending queue index must exist");
        if index < self.selected_pending_queue_index {
            self.selected_pending_queue_index -= 1;
        }
        self.clamp_pending_queue_selection();
        message
    }

    fn pending_queue_status(&self) -> String {
        if self.pending_queue_messages.is_empty() {
            return self.text().queue_none().to_string();
        }
        let items = self
            .pending_queue_messages
            .iter()
            .map(|message| {
                format!(
                    "#{} {}",
                    message.id,
                    pending_queue_preview_text(&message.text, 32)
                )
            })
            .collect::<Vec<_>>();
        self.text().queue_status(&items.join(" | "))
    }
}

pub(super) fn pending_queue_visible_items() -> usize {
    PENDING_QUEUE_VISIBLE_ITEMS
}

pub(super) fn pending_queue_visible_range(len: usize, selected_index: usize) -> Range<usize> {
    if len == 0 {
        return 0..0;
    }
    let visible_count = len.min(PENDING_QUEUE_VISIBLE_ITEMS);
    let selected_index = selected_index.min(len - 1);
    let start = selected_index
        .saturating_add(1)
        .saturating_sub(visible_count);
    start..start + visible_count
}

pub(super) fn pending_queue_preview_text(text: &str, max_width: usize) -> String {
    let normalized = text.lines().collect::<Vec<_>>().join(" ");
    truncate_display_columns(&normalized, max_width)
}

fn pending_queue_command_id(args: &[String], usage: &str) -> Result<u64, String> {
    let value = args.get(1).ok_or_else(|| usage.to_string())?;
    value
        .trim_start_matches('#')
        .parse::<u64>()
        .map_err(|_| usage.to_string())
}

fn pending_queue_plain_key(modifiers: KeyModifiers) -> bool {
    modifiers.is_empty() || modifiers == KeyModifiers::SHIFT
}

fn truncate_display_columns(value: &str, max_width: usize) -> String {
    let marker = "...";
    let marker_width = display_width(marker);
    if display_width(value) <= max_width {
        return value.to_string();
    }
    let text_width = max_width.saturating_sub(marker_width);
    let mut output = String::new();
    let mut width = 0usize;
    for ch in value.chars() {
        let ch_width = display_width(&ch.to_string());
        if width + ch_width > text_width {
            break;
        }
        output.push(ch);
        width += ch_width;
    }
    output.push_str(marker);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_text_flattens_lines() {
        assert_eq!(pending_queue_preview_text("one\ntwo", 20), "one two");
    }

    #[test]
    fn command_id_accepts_hash_prefix() {
        let args = vec!["delete".to_string(), "#7".to_string()];
        assert_eq!(pending_queue_command_id(&args, "usage").unwrap(), 7);
    }

    #[test]
    fn visible_range_keeps_selected_item_visible() {
        assert_eq!(pending_queue_visible_range(5, 0), 0..3);
        assert_eq!(pending_queue_visible_range(5, 2), 0..3);
        assert_eq!(pending_queue_visible_range(5, 3), 1..4);
        assert_eq!(pending_queue_visible_range(5, 4), 2..5);
    }
}
