use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use operit_model::ChatMessage::ChatMessage;
use operit_model::InputProcessingState::InputProcessingState;
use ratatui::text::Line;

use super::empty_state::render_blue_cat_lines;
use super::fold::{FoldedLines, TranscriptFoldHit, TranscriptFoldState};
use super::helpers::{
    is_streaming_message_for_tui, render_input_error_lines, render_loading_ai_placeholder_lines,
    render_transcript_message_lines_with_cache,
};
use super::i18n::{TuiLanguage, TuiText};
use super::typewriter::TypewriterState;

#[derive(Clone, Debug, Default)]
pub(super) struct TranscriptRenderCache {
    pub(super) xml: Vec<super::compose::XmlSurfaceSlot>,
    chat_id: Option<String>,
    pub(super) messages: HashMap<i64, TranscriptMessageRenderCache>,
    pub(super) fold_state: TranscriptFoldState,
    pub(super) fold_hits: Vec<TranscriptFoldHit>,
}

#[derive(Clone, Debug)]
pub(super) struct TranscriptMessageRenderCache {
    pub(super) xml: Vec<super::compose::XmlSurfaceSlot>,
    pub(super) key: TranscriptMessageRenderKey,
    pub(super) lines: Vec<Line<'static>>,
    pub(super) fold_hits: Vec<TranscriptFoldHit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TranscriptMessageRenderKey {
    timestamp: i64,
    content_width: usize,
    sender: String,
    role_name: String,
    provider: String,
    model_name: String,
    output_tokens: i64,
    content_hash: u64,
    fold_signature: u64,
    language: TuiLanguage,
}

impl TranscriptMessageRenderKey {
    pub(super) fn build(
        message: &ChatMessage,
        content_width: usize,
        language: TuiLanguage,
        fold_signature: u64,
    ) -> Self {
        let content = if message.sender == "ai" {
            format!("{:?}", message.parts)
        } else {
            message.displayText()
        };
        Self {
            timestamp: message.timestamp,
            content_width,
            sender: message.sender.clone(),
            role_name: message.roleName.clone(),
            provider: message.provider.clone(),
            model_name: message.modelName.clone(),
            output_tokens: message.outputTokens,
            content_hash: stable_content_hash(&content),
            fold_signature,
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
    text: TuiText,
) -> Vec<Line<'static>> {
    if messages.is_empty() {
        transcript_cache.clear();
        return render_blue_cat_lines(content_width, text);
    }

    transcript_cache.ensure_chat_id(current_chat_id);
    let active_message_timestamps = messages
        .iter()
        .map(|message| message.timestamp)
        .collect::<HashSet<_>>();
    typewriter_state.retain_messages(&active_message_timestamps);
    transcript_cache
        .fold_state
        .retain_messages(&active_message_timestamps);
    transcript_cache
        .messages
        .retain(|timestamp, _| active_message_timestamps.contains(timestamp));
    transcript_cache.fold_hits.clear();

    let mut output = FoldedLines::default();
    for (index, message) in messages.iter().enumerate() {
        if !output.lines.is_empty() {
            output.lines.push(Line::from(""));
        }
        let streaming_message =
            is_streaming_message_for_tui(message, index, messages.len(), is_loading);
        let fold_signature = transcript_cache
            .fold_state
            .signature_for_message(message.timestamp);
        if streaming_message {
            let rendered = render_transcript_message_lines_with_cache(
                message,
                index,
                messages.len(),
                content_width,
                is_loading,
                thinking_line,
                typewriter_state,
                &transcript_cache.fold_state,
                text,
            );
            let cache = transcript_cache
                .messages
                .entry(message.timestamp)
                .or_insert_with(|| TranscriptMessageRenderCache {
                    xml: Vec::new(),
                    key: TranscriptMessageRenderKey::build(
                        message,
                        content_width,
                        text.language(),
                        fold_signature,
                    ),
                    lines: Vec::new(),
                    fold_hits: Vec::new(),
                });
            cache.key = TranscriptMessageRenderKey::build(
                message,
                content_width,
                text.language(),
                fold_signature,
            );
            cache.lines = rendered.lines.clone();
            cache.xml = rendered.xml.clone();
            cache.fold_hits = rendered.hits.clone();
            output.extend(rendered);
            continue;
        }

        let key = TranscriptMessageRenderKey::build(
            message,
            content_width,
            text.language(),
            fold_signature,
        );
        if let Some(cached) = transcript_cache
            .messages
            .get(&message.timestamp)
            .filter(|cached| cached.key == key)
        {
            let cached_block = FoldedLines {
                xml: cached.xml.clone(),
                lines: cached.lines.clone(),
                hits: cached.fold_hits.clone(),
            };
            output.extend(cached_block);
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
            &transcript_cache.fold_state,
            text,
        );
        transcript_cache.messages.insert(
            message.timestamp,
            TranscriptMessageRenderCache {
                xml: rendered.xml.clone(),
                key,
                lines: rendered.lines.clone(),
                fold_hits: rendered.hits.clone(),
            },
        );
        output.extend(rendered);
    }
    if is_loading && matches!(messages.last(), Some(message) if message.sender == "user") {
        if !output.lines.is_empty() {
            output.lines.push(Line::from(""));
        }
        output.lines.extend(render_loading_ai_placeholder_lines(
            content_width,
            thinking_line,
        ));
    }
    output
        .lines
        .extend(render_input_error_lines(input_state, text));
    transcript_cache.fold_hits = output.hits;
    transcript_cache.xml = output.xml;
    output.lines
}

impl TranscriptRenderCache {
    pub(super) fn clear(&mut self) {
        self.xml.clear();
        self.chat_id = None;
        self.messages.clear();
        self.fold_state.clear();
        self.fold_hits.clear();
    }

    /// Records a user click that toggles one fold widget. An expanded fold
    /// covers its whole body, and nested widgets are recorded after the block
    /// that contains them, so the last matching hit is the innermost one under
    /// the pointer.
    pub(super) fn toggle_fold_at_line(&mut self, line_index: usize) -> bool {
        let Some(hit) = self
            .fold_hits
            .iter()
            .rev()
            .find(|hit| hit.line_index == line_index)
            .cloned()
        else {
            return false;
        };
        self.fold_state.set_user_expanded(
            hit.target.message_timestamp,
            &hit.target.stable_key,
            !hit.target.expanded,
        );
        true
    }

    fn ensure_chat_id(&mut self, chat_id: Option<&str>) {
        let next_chat_id = chat_id.map(ToString::to_string);
        if self.chat_id == next_chat_id {
            return;
        }
        self.chat_id = next_chat_id;
        self.xml.clear();
        self.messages.clear();
        self.fold_state.clear();
        self.fold_hits.clear();
    }
}

fn stable_content_hash(content: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::fold::FoldTarget;

    /// Concatenates the logical text of every line for content assertions.
    fn dump_lines(lines: &[Line<'static>]) -> String {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Renders one message through the same entry point the transcript uses.
    fn render_message(
        message: &ChatMessage,
        cache: &mut TranscriptRenderCache,
        is_loading: bool,
    ) -> FoldedLines {
        let mut typewriter_state = TypewriterState::default();
        render_transcript_message_lines_with_cache(
            message,
            0,
            1,
            48,
            is_loading,
            &Line::from("Thinking Process"),
            &mut typewriter_state,
            &cache.fold_state,
            TuiLanguage::English.text(),
        )
    }

    fn open_thinking_message() -> ChatMessage {
        ChatMessage::new_with_markdown_timestamp(
            "ai".to_string(),
            "<thinking>内部推理</thinking>\n你好！".to_string(),
            1,
        )
    }

    /// An expanded fold is one click target: clicking the body text collapses
    /// it without scrolling back to its header.
    #[test]
    fn clicking_an_expanded_fold_body_collapses_it() {
        let message = open_thinking_message();
        let mut cache = TranscriptRenderCache::default();
        cache.fold_state.set_user_expanded(1, "think-0", true);

        let expanded = render_message(&message, &mut cache, false);
        let body_line = expanded
            .hits
            .iter()
            .map(|hit| hit.line_index)
            .max()
            .expect("an expanded fold records hits");
        assert!(
            expanded.hits.len() > 1,
            "the expanded body must be clickable, not just the header"
        );
        assert!(dump_lines(&expanded.lines).contains("内部推理"));

        cache.fold_hits = expanded.hits;
        assert!(cache.toggle_fold_at_line(body_line));

        let collapsed = render_message(&message, &mut cache, false);
        let rendered = dump_lines(&collapsed.lines);
        assert!(rendered.contains("Thinking Process"));
        assert!(!rendered.contains("内部推理"));
        assert_eq!(collapsed.hits.len(), 1, "the collapsed header stays clickable");
    }

    /// Nested widgets are recorded after the block that contains them, and the
    /// innermost one under the pointer is the one that toggles.
    #[test]
    fn nested_fold_hits_toggle_the_innermost_widget() {
        let mut cache = TranscriptRenderCache::default();
        let outer = FoldTarget {
            message_timestamp: 7,
            stable_key: "tools-only-0".to_string(),
            expanded: true,
        };
        cache.fold_hits = (0..3)
            .map(|line_index| TranscriptFoldHit {
                line_index,
                target: outer.clone(),
            })
            .chain(std::iter::once(TranscriptFoldHit {
                line_index: 2,
                target: FoldTarget {
                    message_timestamp: 7,
                    stable_key: "merged-tool-0-1-0".to_string(),
                    expanded: false,
                },
            }))
            .collect();

        assert!(cache.toggle_fold_at_line(2));
        assert!(
            cache.fold_state.is_expanded(7, "merged-tool-0-1-0", false),
            "the nested tool row must expand instead of collapsing its group"
        );
        assert!(
            cache.fold_state.is_expanded(7, "tools-only-0", true),
            "the containing group must keep its state"
        );

        assert!(cache.toggle_fold_at_line(1));
        assert!(!cache.fold_state.is_expanded(7, "tools-only-0", true));
    }
}
