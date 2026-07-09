use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use operit_model::ChatMessage::ChatMessage;
use operit_model::InputProcessingState::InputProcessingState;
use operit_util::streamnative::NativeMarkdownSplitter::MarkdownNodeStable;
use ratatui::text::Line;

use super::empty_state::render_blue_cat_lines;
use super::helpers::{
    is_streaming_message_for_tui, render_input_error_lines, render_loading_ai_placeholder_lines,
    render_transcript_message_lines_with_cache,
};
use super::i18n::{TuiLanguage, TuiText};
use super::markdown::MarkdownRenderCache;
use super::stream_markdown::TuiMarkdownStreamState;
use super::typewriter::TypewriterState;

#[derive(Clone, Debug, Default)]
pub(super) struct TranscriptRenderCache {
    chat_id: Option<String>,
    pub(super) messages: HashMap<i64, TranscriptMessageRenderCache>,
}

#[derive(Clone, Debug)]
pub(super) struct TranscriptMessageRenderCache {
    pub(super) key: TranscriptMessageRenderKey,
    pub(super) lines: Vec<Line<'static>>,
    pub(super) markdown: MarkdownRenderCache,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TranscriptMessageRenderKey {
    timestamp: i64,
    content_width: usize,
    sender: String,
    role_name: String,
    provider: String,
    model_name: String,
    output_tokens: i32,
    content_hash: u64,
    language: TuiLanguage,
}

impl TranscriptMessageRenderKey {
    pub(super) fn build(
        message: &ChatMessage,
        content_width: usize,
        language: TuiLanguage,
    ) -> Self {
        Self {
            timestamp: message.timestamp,
            content_width,
            sender: message.sender.clone(),
            role_name: message.roleName.clone(),
            provider: message.provider.clone(),
            model_name: message.modelName.clone(),
            output_tokens: message.outputTokens,
            content_hash: stable_content_hash(&message.content),
            language,
        }
    }
}

pub(super) fn render_transcript_lines(
    messages: &[ChatMessage],
    current_chat_id: Option<&str>,
    is_loading: bool,
    input_state: &InputProcessingState,
    thinking_line: &Line<'static>,
    content_width: usize,
    typewriter_state: &mut TypewriterState,
    transcript_cache: &mut TranscriptRenderCache,
    stream_markdown_state: Option<&TuiMarkdownStreamState>,
    text: TuiText,
) -> Vec<Line<'static>> {
    if messages.is_empty() {
        return render_blue_cat_lines(content_width, text);
    }

    transcript_cache.ensure_chat_id(current_chat_id);
    let active_message_timestamps = messages
        .iter()
        .map(|message| message.timestamp)
        .collect::<HashSet<_>>();
    typewriter_state.retain_messages(&active_message_timestamps);
    transcript_cache
        .messages
        .retain(|timestamp, _| active_message_timestamps.contains(timestamp));

    let stream_markdown_nodes = stream_markdown_state.map(TuiMarkdownStreamState::stable_nodes);
    let stream_markdown_nodes = stream_markdown_nodes.as_deref();

    let mut lines = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        append_message_gap(&mut lines);
        let streaming_message =
            is_streaming_message_for_tui(message, index, messages.len(), is_loading);
        if streaming_message {
            let cache = transcript_cache
                .messages
                .entry(message.timestamp)
                .or_insert_with(|| TranscriptMessageRenderCache {
                    key: TranscriptMessageRenderKey::build(message, content_width, text.language()),
                    lines: Vec::new(),
                    markdown: MarkdownRenderCache::default(),
                });
            let rendered = render_transcript_message_lines_with_cache(
                message,
                index,
                messages.len(),
                content_width,
                is_loading,
                thinking_line,
                typewriter_state,
                text,
                stream_markdown_nodes,
                Some(&mut cache.markdown),
            );
            cache.key = TranscriptMessageRenderKey::build(message, content_width, text.language());
            cache.lines = rendered.clone();
            lines.extend(rendered);
            continue;
        }

        let key = TranscriptMessageRenderKey::build(message, content_width, text.language());
        if let Some(cached) = transcript_cache
            .messages
            .get(&message.timestamp)
            .filter(|cached| cached.key == key)
        {
            lines.extend(cached.lines.clone());
            continue;
        }

        let rendered = render_transcript_message_lines_with_cache(
            message,
            index,
            messages.len(),
            content_width,
            is_loading,
            thinking_line,
            typewriter_state,
            text,
            None,
            None,
        );
        transcript_cache.messages.insert(
            message.timestamp,
            TranscriptMessageRenderCache {
                key,
                lines: rendered.clone(),
                markdown: MarkdownRenderCache::default(),
            },
        );
        lines.extend(rendered);
    }
    if is_loading && matches!(messages.last(), Some(message) if message.sender == "user") {
        append_message_gap(&mut lines);
        lines.extend(render_loading_ai_placeholder_lines(
            content_width,
            thinking_line,
        ));
    }
    lines.extend(render_input_error_lines(input_state, text));
    lines
}

impl TranscriptRenderCache {
    pub(super) fn clear(&mut self) {
        self.chat_id = None;
        self.messages.clear();
    }

    fn ensure_chat_id(&mut self, chat_id: Option<&str>) {
        let next_chat_id = chat_id.map(ToString::to_string);
        if self.chat_id == next_chat_id {
            return;
        }
        self.chat_id = next_chat_id;
        self.messages.clear();
    }
}

fn append_message_gap(lines: &mut Vec<Line<'static>>) {
    if !lines.is_empty() {
        lines.push(Line::from(""));
    }
}

fn stable_content_hash(content: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}
