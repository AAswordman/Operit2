//! Chat-owned, bounded projection for constrained displays.
#![allow(non_snake_case)]
use operit_edge_contract::chat_display::{
    ChatDisplayCursor, ChatDisplayEvent, ChatDisplayMessage, ChatDisplayWindow,
};
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_link::{CoreEventKind, CoreEventStream, CoreStream, CoreStreamSource, CoreValue};
use operit_model::ChatHistoryListItem::ChatHistoryListItem;
use operit_model::ChatMessage::ChatMessage;
use operit_model::MessagePart::{MessagePart, MessagePartKind};
use operit_model::MessagePartCodec::{AssistantMarkupStreamState, MessagePartCodec};
use operit_util::stream::TextStreamRevisionTracker::TextStreamRevisionTracker;
use operit_util::ChatMarkupRegex::{attr_value, tag_ranges};
use operit_util::MarkdownRenderStream::MarkdownStreamEvent;
use std::collections::BTreeMap;
use std::sync::Arc;

const EDGE_MESSAGE_LIMIT: usize = 12;
const EDGE_TEXT_LIMIT: usize = 1536;
const EDGE_TOTAL_TEXT_LIMIT: usize = 12 * 1024;
const EDGE_ID_LIMIT: usize = 96;

/// Appends UTF-8 text without exceeding the display byte budget.
fn appendBounded(target: &mut String, source: &str, limit: usize) {
    let mut end = source.len().min(limit.saturating_sub(target.len()));
    while !source.is_char_boundary(end) {
        end -= 1;
    }
    target.push_str(&source[..end]);
}
/// Copies the bounded prefix of a display field.
fn bounded(source: &str, limit: usize) -> String {
    let mut result = String::new();
    appendBounded(&mut result, source, limit);
    result
}

/// Renders only a bounded public name from an already interpreted tool call.
fn toolName(name: &str) -> String {
    let name: String = name.chars().filter(|c| !c.is_control()).collect();
    format!("【{}】", bounded(&name, EDGE_ID_LIMIT))
}

/// Owns semantic parsing and revision history at Core, outside the device contract.
struct DisplayProjection {
    parser: AssistantMarkupStreamState,
    revisions: TextStreamRevisionTracker,
    text: String,
    bytes: usize,
    lines: usize,
}

impl DisplayProjection {
    /// Creates a display stream with the same byte and line bounds as its window.
    fn new(bytes: u32, lines: u32) -> Self {
        Self {
            parser: AssistantMarkupStreamState::new(),
            revisions: TextStreamRevisionTracker::new(""),
            text: String::new(),
            bytes: (bytes as usize).clamp(128, EDGE_TEXT_LIMIT),
            lines: (lines as usize).clamp(1, 64),
        }
    }

    /// Consumes raw owner-side revisions and emits only bounded display text changes.
    fn project(&mut self, event: MarkdownStreamEvent) -> Result<Option<ChatDisplayEvent>, String> {
        if event.parentBlockId.is_some() {
            return Ok(None);
        }
        match event.eventType.as_str() {
            "chunk" => {
                let value = event.value.ok_or("Missing response text")?;
                self.parser.push(&value)?;
                self.revisions.append(&value);
            }
            "reset" => {
                self.parser = AssistantMarkupStreamState::new();
                self.revisions.replace("");
            }
            "savepoint" => {
                self.revisions
                    .savepoint(&event.id.ok_or("Missing savepoint id")?);
                return Ok(None);
            }
            "rollback" => {
                let id = event.id.ok_or("Missing rollback id")?;
                let content = self
                    .revisions
                    .rollback(&id)
                    .ok_or("Unknown response savepoint")?;
                self.parser.resetToSnapshot(content)?;
            }
            _ => return Ok(None),
        }
        let mut text = String::new();
        visitParts(self.parser.parts(), |value| {
            appendDisplayTail(&mut text, value, self.bytes)
        });
        let start = text
            .match_indices('\n')
            .rev()
            .nth(self.lines - 1)
            .map(|(at, _)| at + 1)
            .unwrap_or(0);
        text.drain(..start);
        if text == self.text {
            return Ok(None);
        }
        let change = if let Some(suffix) = text.strip_prefix(&self.text) {
            ChatDisplayEvent::Chunk {
                value: suffix.to_owned(),
            }
        } else {
            ChatDisplayEvent::Replace {
                value: text.clone(),
            }
        };
        self.text = text;
        Ok(Some(change))
    }
}

/// Retains the latest UTF-8 display bytes without cloning a full response.
fn appendDisplayTail(target: &mut String, value: &str, limit: usize) {
    if value.len() >= limit {
        let mut start = value.len() - limit;
        while !value.is_char_boundary(start) {
            start += 1;
        }
        target.clear();
        target.push_str(&value[start..]);
    } else {
        let mut start = target
            .len()
            .saturating_add(value.len())
            .saturating_sub(limit);
        while !target.is_char_boundary(start) {
            start += 1;
        }
        target.drain(..start);
        target.push_str(value);
    }
}

/// Resolves attachment markup with the shared Core parser before projecting prose.
fn visitVisibleText(source: &str, mut emit: impl FnMut(&str)) {
    let mut start = 0;
    for (open, end) in tag_ranges(source, "link") {
        emit(&source[start..open]);
        let tag = &source[open..end];
        if attr_value(tag, "type").as_deref() == Some("image") {
            emit("[图片：请在 Core 端查看]");
        } else {
            emit(tag);
        }
        start = end;
    }
    emit(&source[start..]);
}

/// Selects canonical prose and public names; tool results never enter the projection.
fn visitParts(parts: &[MessagePart], mut emit: impl FnMut(&str)) {
    let mut first = true;
    for part in MessagePartCodec::orderedParts(parts) {
        if !matches!(
            part.kind,
            MessagePartKind::Markdown | MessagePartKind::Status | MessagePartKind::ToolCall
        ) {
            continue;
        }
        if !first {
            emit("\n");
        }
        first = false;
        match part.kind {
            MessagePartKind::ToolCall => {
                if let Some(name) = &part.toolName {
                    emit(&toolName(name));
                }
            }
            _ => visitVisibleText(&part.content, &mut emit),
        }
    }
}

/// Interprets historical assistant Markdown before it crosses the display boundary.
fn historyDisplayParts(
    message: &ChatMessage,
) -> Result<std::borrow::Cow<'_, [MessagePart]>, String> {
    if !matches!(message.sender.as_str(), "ai" | "assistant") {
        return Ok(std::borrow::Cow::Borrowed(&message.parts));
    }
    let mut parts = Vec::new();
    for part in MessagePartCodec::orderedParts(&message.parts) {
        match part.kind {
            MessagePartKind::Markdown => {
                parts.extend(MessagePartCodec::parseAssistantMarkup(&part.content)?)
            }
            _ => parts.push(part.clone()),
        }
    }
    // Parsing each legacy Markdown fragment creates local order numbers.
    // Preserve the original canonical traversal across those fragment boundaries.
    for (order, part) in parts.iter_mut().enumerate() {
        part.sequence = order as i32;
    }
    Ok(std::borrow::Cow::Owned(parts))
}
/// Bounded display page, not a replica. Text/tool payloads stay at the owning
/// Core; older windows are addressed by timestamp + UTF-8 offset.
pub fn chatMessageWindow(
    messages: Vec<ChatMessage>,
    beforeTimestamp: Option<i64>,
    beforeTextOffset: Option<u32>,
    textBytes: u32,
    textLines: u32,
) -> ChatDisplayWindow {
    let mut remaining = (textBytes as usize).clamp(128, EDGE_TOTAL_TEXT_LIMIT);
    let mut remainingLines = (textLines as usize).clamp(1, 64);
    let mut rows = Vec::new();
    let mut older = None;
    let mut hasEarlier = false;
    for message in messages.iter().rev() {
        if beforeTimestamp.is_some_and(|t| {
            message.timestamp > t || (message.timestamp == t && beforeTextOffset == Some(0))
        }) {
            continue;
        }
        if remaining < 4 || remainingLines == 0 || rows.len() == EDGE_MESSAGE_LIMIT {
            hasEarlier = true;
            break;
        }
        let parts = match historyDisplayParts(message) {
            Ok(parts) => parts,
            Err(error) => {
                return ChatDisplayWindow {
                    messages: Vec::new(),
                    older: None,
                    error: Some(bounded(&error, EDGE_TEXT_LIMIT)),
                }
            }
        };
        let mut length = 0usize;
        let endLimit = if beforeTimestamp == Some(message.timestamp) {
            beforeTextOffset.map(|n| n as usize).unwrap_or(usize::MAX)
        } else {
            usize::MAX
        };
        let mut breaks = std::collections::VecDeque::new();
        visitParts(&parts, |s| {
            for (index, _) in s.match_indices('\n') {
                let offset = length + index;
                if offset >= endLimit {
                    break;
                }
                if breaks.len() == remainingLines {
                    breaks.pop_front();
                }
                breaks.push_back(offset);
            }
            length += s.len();
        });
        let end = endLimit.min(length);
        let lineStart = if breaks.len() == remainingLines {
            breaks[0] + 1
        } else {
            0
        };
        let start = end.saturating_sub(remaining).max(lineStart);
        let mut text = String::new();
        let mut at = 0usize;
        let mut actualStart = end;
        visitParts(&parts, |s| {
            let mut lo = start.saturating_sub(at).min(s.len());
            let mut hi = end.saturating_sub(at).min(s.len());
            while lo < s.len() && !s.is_char_boundary(lo) {
                lo += 1;
            }
            while hi > 0 && !s.is_char_boundary(hi) {
                hi -= 1;
            }
            if lo < hi {
                actualStart = actualStart.min(at + lo);
                text.push_str(&s[lo..hi]);
            }
            at += s.len();
        });
        let stream = if beforeTimestamp.is_none() {
            message
                .contentStream
                .clone()
                .and_then(|stream| compactStream(stream, textBytes, textLines))
        } else {
            None
        };
        if text.is_empty() && stream.is_none() {
            continue;
        }
        remaining = remaining.saturating_sub(text.len());
        remainingLines = remainingLines.saturating_sub(1 + text.matches('\n').count());
        rows.push(ChatDisplayMessage {
            sender: bounded(&message.sender, 16),
            timestamp: message.timestamp,
            text,
            contentStream: stream,
        });
        older = Some(ChatDisplayCursor {
            timestamp: message.timestamp,
            offset: actualStart.try_into().unwrap_or(u32::MAX),
        });
        if actualStart > 0 {
            hasEarlier = true;
            break;
        }
    }
    rows.reverse();
    if !hasEarlier {
        older = None;
    }
    ChatDisplayWindow {
        messages: rows,
        older,
        error: None,
    }
}

/// Limits conversation metadata without exporting business messages.
pub fn compactEdgeHistories(histories: Vec<ChatHistoryListItem>) -> Vec<ChatHistoryListItem> {
    histories
        .into_iter()
        .take(24)
        .map(|mut item| {
            item.id = bounded(&item.id, EDGE_ID_LIMIT);
            item.title = bounded(&item.title, 192);
            item.updatedAt = bounded(&item.updatedAt, 32);
            item.workspaceId = None;
            item.workspaceName = None;
            item
        })
        .collect()
}

/// Wraps the local source; raw XML bodies never enter the Edge stream.
fn compactStream(
    stream: CoreStream<MarkdownStreamEvent>,
    bytes: u32,
    lines: u32,
) -> Option<CoreStream<ChatDisplayEvent>> {
    let source = stream.localSource()?;
    let id = format!("edge:{}", bounded(&stream.descriptor.streamId, 128));
    let projected = CoreStreamSource::new(move |request| {
        let mut upstream = source.open(request)?;
        let (sender, receiver) = CoreEventStream::channel();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "edge-chat-stream",
                Box::new(move || {
                    Box::pin(async move {
                        let mut projection = DisplayProjection::new(bytes, lines);
                        loop {
                            let mut event = tokio::select! {
                                _ = sender.closed() => break,
                                event = upstream.recv() => match event {
                                    Some(event) => event,
                                    None => break,
                                },
                            };
                            if event.kind == CoreEventKind::Completed {
                                match projection.parser.complete() {
                                    Ok(()) => event.value = CoreValue::Null,
                                    Err(message) => {
                                        event.kind = CoreEventKind::Delta;
                                        event.value =
                                            operit_link::toCoreValue(ChatDisplayEvent::Error {
                                                message: bounded(&message, EDGE_TEXT_LIMIT),
                                            })
                                            .expect("display errors serialize");
                                    }
                                }
                                let _ = sender.send(event);
                                break;
                            }
                            let output =
                                operit_link::fromCoreValue::<MarkdownStreamEvent>(event.value)
                                    .map_err(|e| e.to_string())
                                    .and_then(|upstream| projection.project(upstream));
                            let output = match output {
                                Ok(Some(output)) => output,
                                Ok(None) => continue,
                                Err(message) => {
                                    event.value =
                                        operit_link::toCoreValue(ChatDisplayEvent::Error {
                                            message: bounded(&message, EDGE_TEXT_LIMIT),
                                        })
                                        .expect("display errors serialize");
                                    let _ = sender.send(event);
                                    break;
                                }
                            };
                            event.value =
                                operit_link::toCoreValue(output).expect("display edits serialize");
                            if sender.send(event).is_err() {
                                break;
                            }
                        }
                    })
                }),
            )
            .map_err(|error| {
                operit_link::CoreLinkError::new("EDGE_STREAM_SCHEDULE_FAILED", error.to_string())
            })?;
        Ok(receiver)
    });
    Some(CoreStream::fromSourceWithId(id, Arc::new(projected)))
}

#[cfg(test)]
mod window_tests {
    use super::*;
    /// Exposes the typed projection as JSON for window assertions.
    fn decode(value: ChatDisplayWindow) -> serde_json::Value {
        serde_json::to_value(value).unwrap()
    }
    #[test]
    fn long_utf8_reply_pages_without_truncation_or_large_wire_payload() {
        let original = "这是完整的长回复，不可以被截掉。".repeat(4000);
        let message =
            ChatMessage::new_with_markdown_timestamp("assistant".into(), original.clone(), 10);
        let mut timestamp = None;
        let mut offset = None;
        let mut chunks = Vec::new();
        loop {
            let page = decode(chatMessageWindow(
                vec![message.clone()],
                timestamp,
                offset,
                1024,
                15,
            ));
            let text = page["messages"][0]["text"].as_str().unwrap();
            assert!(text.len() <= 1024);
            assert!(text.is_char_boundary(text.len()));
            assert!(serde_json::to_vec(&page).unwrap().len() < 4096);
            chunks.push(text.to_owned());
            if page["older"].is_null() {
                break;
            }
            timestamp = page["older"]["timestamp"].as_i64();
            offset = page["older"]["offset"].as_u64().map(|v| v as u32);
            assert!(chunks.len() < 1000);
        }
        chunks.reverse();
        assert_eq!(chunks.concat(), original);
    }
    #[test]
    fn short_lines_page_by_screen_budget_without_losing_separators() {
        let original = (0..240).map(|n| format!("第{n}行\n")).collect::<String>();
        let message = ChatMessage::new_with_markdown_timestamp("ai".into(), original.clone(), 20);
        let (mut timestamp, mut offset) = (None, None);
        let mut chunks = Vec::new();
        loop {
            let page = chatMessageWindow(vec![message.clone()], timestamp, offset, 1024, 15);
            let text = &page.messages[0].text;
            assert!(1 + text.matches('\n').count() <= 15);
            assert!(text.len() <= 1024);
            chunks.push(text.clone());
            let Some(cursor) = page.older else {
                break;
            };
            timestamp = Some(cursor.timestamp);
            offset = Some(cursor.offset);
            assert!(chunks.len() < 100);
        }
        assert!(chunks.len() > 10);
        chunks.reverse();
        assert_eq!(chunks.concat(), original);
    }
    #[test]
    fn display_window_preserves_typed_live_stream_attachments() {
        let source = CoreStreamSource::new(|_| Ok(CoreEventStream::channel().1));
        let mut message =
            ChatMessage::new_with_markdown_timestamp("assistant".into(), String::new(), 1);
        message.contentStream = Some(CoreStream::fromSourceWithId(
            "live".into(),
            Arc::new(source),
        ));
        let page = chatMessageWindow(vec![message], None, None, 1024, 15);
        let (encoded, attachments) =
            operit_link::withCoreStreamCaptureSync(|| operit_link::toCoreValue(page));
        assert!(encoded.is_ok());
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].streamId, "edge:live");
    }
    #[test]
    fn older_cursor_avoids_duplicates_and_sanitizes_tool_payloads() {
        let messages: Vec<_> = (1..=30)
            .map(|time| {
                ChatMessage::new_with_timestamp(
                    "assistant".into(),
                    vec![
                        MessagePart::markdown("text".into(), 0, format!("正文-{time}")),
                        MessagePart::toolResult(
                            "result".into(),
                            1,
                            None,
                            "date".into(),
                            "success".into(),
                            "SECRET-PAYLOAD".into(),
                        ),
                    ],
                    time,
                )
            })
            .collect();
        let mut timestamp = None;
        let mut offset = None;
        let mut seen = std::collections::BTreeSet::new();
        loop {
            let page = decode(chatMessageWindow(
                messages.clone(),
                timestamp,
                offset,
                1024,
                15,
            ));
            assert!(!serde_json::to_string(&page)
                .unwrap()
                .contains("SECRET-PAYLOAD"));
            for row in page["messages"].as_array().unwrap() {
                assert!(seen.insert(row["timestamp"].as_i64().unwrap()));
            }
            if page["older"].is_null() {
                break;
            }
            timestamp = page["older"]["timestamp"].as_i64();
            offset = page["older"]["offset"].as_u64().map(|v| v as u32);
        }
        assert_eq!(seen.len(), 30);
    }
}

#[cfg(test)]
mod display_contract_tests {
    use super::*;

    /// Canonical call and result parts expose only the public tool name.
    #[test]
    fn tool_projection_has_names_without_call_or_result_state() {
        let mut message =
            ChatMessage::new_with_markdown_timestamp("assistant".into(), "".into(), 1);
        message.parts = vec![
            MessagePart::toolCall(
                "call".into(),
                0,
                "id".into(),
                "daily_life:get_current_date".into(),
                BTreeMap::from([("secret".into(), "PRIVATE-ARGUMENT".into())]),
            ),
            MessagePart::toolResult(
                "result".into(),
                1,
                Some("id".into()),
                "daily_life:get_current_date".into(),
                "success".into(),
                "PRIVATE-RESULT".into(),
            ),
        ];
        let page = chatMessageWindow(vec![message], None, None, 640, 15);
        assert_eq!(
            page.messages[0].text.trim(),
            "【daily_life:get_current_date】"
        );
        let wire = serde_json::to_string(&page).unwrap();
        for private in ["PRIVATE", "toolCallId", "success", "toolStatus"] {
            assert!(!wire.contains(private));
        }
    }

    /// Every UTF-8 transport split preserves the same semantic display without tool bodies.
    #[test]
    fn streaming_projection_is_independent_of_provider_chunk_boundaries() {
        let text = "正文<tool name='date'><param name='secret'>PRIVATE-ARGUMENT</param></tool><tool_result name='date' status='success'><content>PRIVATE-RESULT</content></tool_result>结束";
        let mut expected = String::new();
        visitParts(
            &MessagePartCodec::parseAssistantMarkup(text).unwrap(),
            |s| expected.push_str(s),
        );
        assert!(expected.contains("【date】"));
        assert!(
            !expected.contains("PRIVATE"),
            "parts: {:?}, text: {expected}",
            MessagePartCodec::parseAssistantMarkup(text).unwrap()
        );
        for split in (0..=text.len()).filter(|&n| text.is_char_boundary(n)) {
            let mut projection = DisplayProjection::new(1536, 64);
            let mut actual = String::new();
            for value in [&text[..split], &text[split..]] {
                let mut event = MarkdownStreamEvent::reset("chat".into(), None);
                event.eventType = "chunk".into();
                event.value = Some(value.into());
                if let Some(projected) = projection.project(event).unwrap() {
                    let wire = serde_json::to_string(&projected).unwrap();
                    assert!(!wire.contains("PRIVATE"));
                    match projected {
                        ChatDisplayEvent::Chunk { value } => actual.push_str(&value),
                        ChatDisplayEvent::Replace { value } => actual = value,
                        _ => panic!("unexpected error"),
                    }
                }
            }
            assert_eq!(actual, expected, "split at {split}");
        }
    }

    /// Historical semantic bodies must pass the same Core interpretation before display.
    #[test]
    fn historical_markup_never_exposes_tool_bodies() {
        let content = "before<tool name='date'><param name='secret'>PRIVATE-ARGUMENT</param></tool><tool_result name='date' status='success'><content>PRIVATE-RESULT</content></tool_result>after";
        let message =
            ChatMessage::new_with_markdown_timestamp("assistant".into(), content.into(), 1);
        let page = chatMessageWindow(vec![message], None, None, 640, 15);
        assert_eq!(page.error, None);
        assert!(page.messages[0].text.contains("【date】"));
        assert!(!serde_json::to_string(&page).unwrap().contains("PRIVATE"));
        let message = ChatMessage::new_with_markdown_timestamp(
            "assistant".into(),
            "before<tool name='date'><param name='secret'>PRIVATE-ARGUMENT".into(),
            1,
        );
        let page = chatMessageWindow(vec![message], None, None, 640, 15);
        assert!(page.error.is_some());
        assert!(page.messages.is_empty());
        assert!(!serde_json::to_string(&page).unwrap().contains("PRIVATE"));
    }

    /// Stream completion rejects a semantic body that never received its closing tag.
    #[test]
    fn unfinished_semantics_fail_at_completion() {
        let mut projection = DisplayProjection::new(640, 15);
        let mut event = MarkdownStreamEvent::reset("chat".into(), None);
        event.eventType = "chunk".into();
        event.value = Some("before<tool name='date'><param name='secret'>PRIVATE".into());
        let projected = projection.project(event).unwrap();
        assert!(!serde_json::to_string(&projected)
            .unwrap()
            .contains("PRIVATE"));
        assert!(projection.parser.complete().is_err());
    }

    /// Media references become display labels before reaching the device contract.
    #[test]
    fn attachments_are_plain_display_text() {
        let mut text = String::new();
        visitVisibleText(
            "前<link id='image-id' type='image'>PRIVATE-IMAGE</link>后",
            |s| text.push_str(s),
        );
        assert_eq!(text, "前[图片：请在 Core 端查看]后");
    }
}
