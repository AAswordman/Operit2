//! Chat-owned, bounded projection for constrained displays.
#![allow(non_snake_case)]
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatDisplayWindowState::{ChatDisplayWindow, ChatDisplayCursor, ChatDisplayMessage};
use operit_model::ChatHistoryListItem::ChatHistoryListItem;
use operit_model::MessagePart::{MessagePart, MessagePartKind};
use operit_model::MessagePartCodec::MessagePartCodec;
use operit_link::{CoreStream, CoreStreamSource, CoreEventStream, CoreEventKind, CoreValue};
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_util::MarkdownRenderStream::MarkdownStreamEvent;
use std::collections::BTreeMap;
use std::sync::Arc;

const EDGE_MESSAGE_LIMIT: usize = 12;
const EDGE_PART_LIMIT: usize = 4;
const EDGE_TEXT_LIMIT: usize = 1536;
const EDGE_TOTAL_TEXT_LIMIT: usize = 12 * 1024;
const EDGE_ID_LIMIT: usize = 96;

fn appendBounded(target: &mut String, source: &str, limit: usize) {
    let mut end = source.len().min(limit.saturating_sub(target.len()));
    while !source.is_char_boundary(end) { end -= 1; }
    target.push_str(&source[..end]);
}
fn bounded(source: &str, limit: usize) -> String {
    let mut result = String::new();
    appendBounded(&mut result, source, limit);
    result
}

pub fn visibleEdgeText(source: &str) -> String {
    let mut result = String::new();
    let mut rest = source;
    while !rest.is_empty() && result.len() < EDGE_TEXT_LIMIT {
        let Some(open) = rest.find('<') else {
            appendBounded(&mut result, rest, EDGE_TEXT_LIMIT);
            break;
        };
        appendBounded(&mut result, &rest[..open], EDGE_TEXT_LIMIT);
        rest = &rest[open..];
        let Some(close) = rest.find('>') else { break; };
        let tag = &rest[1..close];
        let name = tag.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("");
        if name == "link" && (tag.contains("type=\"image\"") || tag.contains("type='image'")) {
            let after_tag = &rest[close + 1..];
            let Some(end) = after_tag.find("</link>") else { break; };
            appendBounded(&mut result, &rest[..=close], EDGE_TEXT_LIMIT);
            appendBounded(&mut result, "</link>", EDGE_TEXT_LIMIT);
            rest = &after_tag[end + "</link>".len()..];
            continue;
        }
        if name == "tool" && !tag.starts_with('/') {
            let tool = tag.split_once("name=\"").and_then(|(_, tail)| tail.split_once('"').map(|(name, _)| name))
                .or_else(|| tag.split_once("name='").and_then(|(_, tail)| tail.split_once('\'').map(|(name, _)| name)));
            if let Some(tool) = tool.filter(|value| !value.is_empty()) {
                if !result.is_empty() { result.push('\n'); }
                appendBounded(&mut result, "调用工具：", EDGE_TEXT_LIMIT);
                let end = tool.len().min(96);
                let end = (0..=end).rev().find(|index| tool.is_char_boundary(*index)).unwrap_or(0);
                appendBounded(&mut result, &tool[..end], EDGE_TEXT_LIMIT);
            }
        }
        rest = &rest[close + 1..];
        if !tag.starts_with('/') && !name.is_empty() {
            let closing = format!("</{name}>");
            if let Some(end) = rest.find(&closing) { rest = &rest[end + closing.len()..]; }
            else if !tag.trim_end().ends_with('/') { break; }
        }
    }
    if result.len() > EDGE_TEXT_LIMIT {
        let end = (0..=EDGE_TEXT_LIMIT.min(result.len())).rev()
            .find(|index| result.is_char_boundary(*index)).unwrap_or(0);
        result.truncate(end);
    }
    result
}

/// A cursor addresses visible UTF-8 bytes within one message, so a single long
/// reply can be paged without truncating it or hydrating it on the device.
fn visitVisibleText(source: &str, mut emit: impl FnMut(&str)) {
    let mut rest = source;
    while !rest.is_empty() {
        let Some(open) = rest.find('<') else { emit(rest); break; };
        emit(&rest[..open]); rest = &rest[open..];
        let Some(close) = rest.find('>') else { break; };
        let tag = &rest[1..close];
        let name = tag.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("");
        if name == "link" && (tag.contains("type=\"image\"") || tag.contains("type='image'")) {
            let after = &rest[close + 1..];
            let Some(end) = after.find("</link>") else { break; };
            emit(&rest[..=close]); emit("</link>"); rest = &after[end + 7..]; continue;
        }
        if name == "tool" && !tag.starts_with('/') {
            if let Some(tool) = tag.split_once("name=\"").and_then(|(_, t)| t.split_once('"').map(|(n, _)| n)) {
                emit("\n调用工具："); emit(&bounded(tool, EDGE_ID_LIMIT)); emit("\n");
            }
        }
        rest = &rest[close + 1..];
        if !tag.starts_with('/') && !name.is_empty() {
            let closing = format!("</{name}>");
            if let Some(end) = rest.find(&closing) { rest = &rest[end + closing.len()..]; }
            else if !tag.trim_end().ends_with('/') { break; }
        }
    }
}
fn visitMessageText(message: &ChatMessage, mut emit: impl FnMut(&str)) {
    let mut first = true;
    for part in MessagePartCodec::orderedParts(&message.parts) {
        match part.kind {
            MessagePartKind::Markdown | MessagePartKind::Status => {
                if !first { emit("\n"); } first = false;
                visitVisibleText(&part.content, &mut emit);
            }
            MessagePartKind::ToolCall => {
                if !first { emit("\n"); } first = false;
                emit("调用工具："); emit(&bounded(part.toolName.as_deref().unwrap_or("未知工具"), EDGE_ID_LIMIT));
            }
            _ => {}
        }
    }
}
/// Bounded display page, not a replica. Text/tool payloads stay at the owning
/// Core; older windows are addressed by timestamp + UTF-8 offset.
pub fn chatMessageWindow(messages: Vec<ChatMessage>, beforeTimestamp: Option<i64>, beforeTextOffset: Option<u32>, textBytes: u32, textLines: u32) -> ChatDisplayWindow {
    let mut remaining = (textBytes as usize).clamp(128, EDGE_TOTAL_TEXT_LIMIT);
    let mut remainingLines = (textLines as usize).clamp(1, 64);
    let mut rows = Vec::new();
    let mut older = None;
    let mut hasEarlier = false;
    for message in messages.iter().rev() {
        if beforeTimestamp.is_some_and(|t| message.timestamp > t || (message.timestamp == t && beforeTextOffset == Some(0))) { continue; }
        if remaining < 4 || remainingLines == 0 || rows.len() == EDGE_MESSAGE_LIMIT { hasEarlier = true; break; }
        let mut length = 0usize;
        let endLimit = if beforeTimestamp == Some(message.timestamp) { beforeTextOffset.map(|n| n as usize).unwrap_or(usize::MAX) } else { usize::MAX };
        let mut breaks = std::collections::VecDeque::new();
        visitMessageText(message, |s| {
            for (index, _) in s.match_indices('\n') {
                let offset = length + index;
                if offset >= endLimit { break; }
                if breaks.len() == remainingLines { breaks.pop_front(); }
                breaks.push_back(offset);
            }
            length += s.len();
        });
        let end = endLimit.min(length);
        let lineStart = if breaks.len() == remainingLines { breaks[0] + 1 } else { 0 };
        let start = end.saturating_sub(remaining).max(lineStart);
        let mut text = String::new(); let mut at = 0usize; let mut actualStart = end;
        visitMessageText(message, |s| {
            let mut lo = start.saturating_sub(at).min(s.len());
            let mut hi = end.saturating_sub(at).min(s.len());
            while lo < s.len() && !s.is_char_boundary(lo) { lo += 1; }
            while hi > 0 && !s.is_char_boundary(hi) { hi -= 1; }
            if lo < hi { actualStart = actualStart.min(at + lo); text.push_str(&s[lo..hi]); }
            at += s.len();
        });
        let stream = if beforeTimestamp.is_none() { message.contentStream.clone().and_then(compactStream) } else { None };
        if text.is_empty() && stream.is_none() { continue; }
        remaining = remaining.saturating_sub(text.len());
        remainingLines = remainingLines.saturating_sub(1 + text.matches('\n').count());
        rows.push(ChatDisplayMessage { sender:bounded(&message.sender, 16), timestamp:message.timestamp, text, contentStream:stream });
        older = Some(ChatDisplayCursor { timestamp:message.timestamp, offset:actualStart.try_into().unwrap_or(u32::MAX) });
        if actualStart > 0 { hasEarlier = true; break; }
    }
    rows.reverse();
    if !hasEarlier { older = None; }
    ChatDisplayWindow { messages:rows, older, error:None }
}

pub fn compactEdgeHistories(histories: Vec<ChatHistoryListItem>) -> Vec<ChatHistoryListItem> {
    histories.into_iter().take(24).map(|mut item| {
        item.id = bounded(&item.id, EDGE_ID_LIMIT);
        item.title = bounded(&item.title, 192);
        item.updatedAt = bounded(&item.updatedAt, 32);
        item.group = None;
        item.workspaceId = None;
        item.workspaceName = None;
        item.characterCardName = None;
        item.characterGroupId = None;
        item
    }).collect()
}

/// Wraps the local source; raw XML bodies never enter the Edge stream.
fn compactStream(stream: CoreStream<MarkdownStreamEvent>) -> Option<CoreStream<MarkdownStreamEvent>> {
    let source = stream.localSource()?;
    let id = format!("edge:{}", bounded(&stream.descriptor.streamId, 128));
    let projected = CoreStreamSource::new(move |request| {
        let mut upstream = source.open(request)?;
        let (sender, receiver) = CoreEventStream::channel();
        defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask("edge-chat-stream", Box::new(move || {
            Box::pin(async move {
                let remaining = EDGE_TEXT_LIMIT;
                let mut tools = std::collections::BTreeSet::new();
                loop {
                    let mut event = tokio::select! {
                        _ = sender.closed() => break,
                        event = upstream.recv() => match event {
                            Some(event) => event,
                            None => break,
                        },
                    };
                    if event.kind == CoreEventKind::Completed {
                        event.value = CoreValue::Null;
                        let _ = sender.send(event);
                        break;
                    }
                    let CoreValue::Map(fields) = &event.value else { continue; };
                    if fields.get("parentBlockId").is_some_and(|v| *v != CoreValue::Null) { continue; }
                    let kind = match fields.get("type") { Some(CoreValue::String(v)) => v.as_str(), _ => continue };
                    let mut output = BTreeMap::new();
                    let text = if let Some(CoreValue::Map(xml)) = fields.get("xml") {
                        let tool = matches!(xml.get("tagName"), Some(CoreValue::String(name)) if name == "tool");
                        let block = fields.get("blockId").cloned().unwrap_or(CoreValue::Null);
                        let key = format!("{block:?}");
                        if !tool || tools.contains(&key) || tools.len() >= EDGE_PART_LIMIT { continue; }
                        tools.insert(key);
                        let Some(CoreValue::Map(attrs)) = xml.get("attributes") else { continue; };
                        let Some(CoreValue::String(name)) = attrs.get("name") else { continue; };
                        output.insert("type".into(), CoreValue::String("chunk".into()));
                        bounded(&format!("\n调用工具：{}\n", bounded(name, EDGE_ID_LIMIT)), remaining)
                    } else {
                        if !matches!(kind, "chunk" | "reset" | "savepoint" | "rollback") { continue; }
                        output.insert("type".into(), CoreValue::String(kind.into()));
                        if kind == "reset" { tools.clear(); }
                        if let Some(CoreValue::String(id)) = fields.get("id") {
                            output.insert("id".into(), CoreValue::String(bounded(id, EDGE_ID_LIMIT)));
                        }
                        match fields.get("value") {
                            Some(CoreValue::String(value)) => bounded(&visibleEdgeText(value), remaining),
                            _ => String::new(),
                        }
                    };
                    output.insert("value".into(), CoreValue::String(text));
                    event.value = CoreValue::Map(output);
                    if sender.send(event).is_err() { break; }
                }
            })
        }))
            .map_err(|error| operit_link::CoreLinkError::new("EDGE_STREAM_SCHEDULE_FAILED", error.to_string()))?;
        Ok(receiver)
    });
    Some(CoreStream::fromSourceWithId(id, Arc::new(projected)))
}

#[cfg(test)]
mod window_tests {
    use super::*;
    fn decode(value: ChatDisplayWindow) -> serde_json::Value { serde_json::to_value(value).unwrap() }
    #[test]
    fn long_utf8_reply_pages_without_truncation_or_large_wire_payload() {
        let original = "这是完整的长回复，不可以被截掉。".repeat(4000);
        let message = ChatMessage::new_with_markdown_timestamp("assistant".into(), original.clone(), 10);
        let mut timestamp = None; let mut offset = None; let mut chunks = Vec::new();
        loop {
            let page = decode(chatMessageWindow(vec![message.clone()], timestamp, offset, 1024, 15));
            let text = page["messages"][0]["text"].as_str().unwrap();
            assert!(text.len() <= 1024); assert!(text.is_char_boundary(text.len()));
            assert!(serde_json::to_vec(&page).unwrap().len() < 4096);
            chunks.push(text.to_owned());
            if page["older"].is_null() { break; }
            timestamp = page["older"]["timestamp"].as_i64(); offset = page["older"]["offset"].as_u64().map(|v|v as u32);
            assert!(chunks.len() < 1000);
        }
        chunks.reverse(); assert_eq!(chunks.concat(), original);
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
            let Some(cursor) = page.older else { break; };
            timestamp = Some(cursor.timestamp); offset = Some(cursor.offset);
            assert!(chunks.len() < 100);
        }
        assert!(chunks.len() > 10);
        chunks.reverse(); assert_eq!(chunks.concat(), original);
    }
    #[test]
    fn display_window_preserves_typed_live_stream_attachments() {
        let source = CoreStreamSource::new(|_| Ok(CoreEventStream::channel().1));
        let mut message = ChatMessage::new_with_markdown_timestamp("assistant".into(), String::new(), 1);
        message.contentStream = Some(CoreStream::fromSourceWithId("live".into(), Arc::new(source)));
        let page = chatMessageWindow(vec![message], None, None, 1024, 15);
        let (encoded, attachments) = operit_link::withCoreStreamCaptureSync(|| operit_link::toCoreValue(page));
        assert!(encoded.is_ok()); assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].streamId, "edge:live");
    }
    #[test]
    fn older_cursor_avoids_duplicates_and_sanitizes_tool_payloads() {
        let messages: Vec<_> = (1..=30).map(|time| ChatMessage::new_with_markdown_timestamp("assistant".into(), format!("正文-{time}<tool_result>SECRET-PAYLOAD</tool_result>"), time)).collect();
        let mut timestamp=None; let mut offset=None; let mut seen=std::collections::BTreeSet::new();
        loop {
            let page=decode(chatMessageWindow(messages.clone(),timestamp,offset,1024,15));
            assert!(!serde_json::to_string(&page).unwrap().contains("SECRET-PAYLOAD"));
            for row in page["messages"].as_array().unwrap() { assert!(seen.insert(row["timestamp"].as_i64().unwrap())); }
            if page["older"].is_null(){break;}
            timestamp=page["older"]["timestamp"].as_i64();offset=page["older"]["offset"].as_u64().map(|v|v as u32);
        }
        assert_eq!(seen.len(),30);
    }
}
