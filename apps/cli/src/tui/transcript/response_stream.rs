use operit_util::stream::TextStreamRevisionTracker::TextStreamRevisionTracker;
use serde::Deserialize;

use super::app::OperitTui;
use super::stream_markdown::markdown_type_from_event_label;

#[derive(Clone, Debug, Deserialize)]
struct ResponseStreamLinkEvent {
    #[serde(rename = "chatId")]
    chat_id: String,
    #[serde(rename = "type")]
    event_type: String,
    value: Option<String>,
    id: Option<String>,
    #[serde(rename = "blockId")]
    block_id: Option<u64>,
    #[serde(rename = "inlineId")]
    inline_id: Option<u64>,
    #[serde(rename = "parentBlockId")]
    parent_block_id: Option<u64>,
    #[serde(rename = "nodeType")]
    node_type: Option<String>,
}

impl OperitTui {
    pub(super) async fn sync_response_stream_subscriptions(&mut self) {
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

    pub(super) fn apply_response_stream_event(&mut self, value: serde_json::Value) {
        let event = serde_json::from_value::<ResponseStreamLinkEvent>(value)
            .expect("response stream event must match TUI schema");
        if event.parent_block_id.is_some() {
            return;
        }
        let chat_id = event.chat_id.clone();
        match event.event_type.as_str() {
            "chunk" => {
                let chunk = event.value.expect("chunk event must include value");
                let tracker = self
                    .response_stream_revision_tracker_by_chat_id
                    .entry(chat_id.clone())
                    .or_insert_with(|| TextStreamRevisionTracker::new(""));
                let content = tracker.append(&chunk);
                self.response_stream_text_by_chat_id
                    .insert(chat_id, content);
            }
            "savepoint" => {
                let id = event.id.expect("savepoint event must include id");
                let tracker = self
                    .response_stream_revision_tracker_by_chat_id
                    .entry(chat_id.clone())
                    .or_insert_with(|| TextStreamRevisionTracker::new(""));
                tracker.savepoint(&id);
                self.response_stream_markdown_by_chat_id
                    .entry(chat_id)
                    .or_default()
                    .savepoint(id);
            }
            "rollback" => {
                let id = event.id.expect("rollback event must include id");
                if let Some(tracker) = self
                    .response_stream_revision_tracker_by_chat_id
                    .get_mut(&chat_id)
                {
                    if let Some(content) = tracker.rollback(&id) {
                        self.response_stream_text_by_chat_id
                            .insert(chat_id.clone(), content);
                    }
                }
                if let Some(markdown) = self.response_stream_markdown_by_chat_id.get_mut(&chat_id) {
                    markdown.rollback(&id);
                }
            }
            "markdownBlockStart" => {
                let block_id = event
                    .block_id
                    .expect("markdownBlockStart event must include blockId");
                let node_type = markdown_type_from_event_label(event.node_type.as_deref());
                self.response_stream_markdown_by_chat_id
                    .entry(chat_id)
                    .or_default()
                    .start_block(block_id, node_type);
            }
            "markdownBlockChunk" => {
                let block_id = event
                    .block_id
                    .expect("markdownBlockChunk event must include blockId");
                let content = event
                    .value
                    .expect("markdownBlockChunk event must include value");
                self.response_stream_markdown_by_chat_id
                    .entry(chat_id)
                    .or_default()
                    .append_block(block_id, &content);
            }
            "markdownInlineStart" => {
                let block_id = event
                    .block_id
                    .expect("markdownInlineStart event must include blockId");
                let inline_id = event
                    .inline_id
                    .expect("markdownInlineStart event must include inlineId");
                let node_type = markdown_type_from_event_label(event.node_type.as_deref());
                self.response_stream_markdown_by_chat_id
                    .entry(chat_id)
                    .or_default()
                    .start_inline(block_id, inline_id, node_type);
            }
            "markdownInlineChunk" => {
                let block_id = event
                    .block_id
                    .expect("markdownInlineChunk event must include blockId");
                let inline_id = event
                    .inline_id
                    .expect("markdownInlineChunk event must include inlineId");
                let content = event
                    .value
                    .expect("markdownInlineChunk event must include value");
                self.response_stream_markdown_by_chat_id
                    .entry(chat_id)
                    .or_default()
                    .append_inline(block_id, inline_id, &content);
            }
            "completed" => {
                if let Some(markdown) = self.response_stream_markdown_by_chat_id.get_mut(&chat_id) {
                    markdown.complete();
                }
            }
            _ => panic!("unknown response stream event type {}", event.event_type),
        }
    }

    pub(super) fn retain_active_response_stream_state(&mut self) {
        let active = &self.active_streaming_chat_ids_cache;
        self.response_stream_subscription_chat_ids
            .retain(|chat_id| active.contains(chat_id));
        self.response_stream_text_by_chat_id
            .retain(|chat_id, _| active.contains(chat_id));
        self.response_stream_markdown_by_chat_id
            .retain(|chat_id, _| active.contains(chat_id));
        self.response_stream_revision_tracker_by_chat_id
            .retain(|chat_id, _| active.contains(chat_id));
    }

    pub(super) fn update_current_chat_loading_from_streaming_ids(&mut self) {
        self.current_chat_is_loading_cache = self
            .current_chat_id_cache
            .as_ref()
            .map(|chat_id| self.active_streaming_chat_ids_cache.contains(chat_id))
            .unwrap_or(false);
    }
}
