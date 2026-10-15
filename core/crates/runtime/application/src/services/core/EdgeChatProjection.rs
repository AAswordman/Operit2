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

// Private display tokens, not plugin invocations. They share the bounded text
// cursor/stream with prose so ordering and paging need no second business cache.
// RS <state> | <tool name> US; never parameters, call payloads or result bodies.
fn toolDisplay(name: &str, status: &str) -> String {
    let state = match status {
        "success" | "succeeded" | "ok" => 'S',
        "error" | "failed" | "failure" | "cancelled" | "canceled" => 'F',
        "running" | "pending" => 'R',
        _ => 'U',
    };
    let mut displayName = String::new();
    for c in name.chars().filter(|c| !c.is_control() && *c != '<' && *c != '>') {
        if displayName.len() + c.len_utf8() > EDGE_ID_LIMIT { break; }
        displayName.push(c);
    }
    format!("\x1e{state}|{displayName}\x1f")
}

fn visitProse(source: &str, mut emit: impl FnMut(&str)) {
    // Only this projection may create card delimiters. Model/user text cannot.
    for text in source.split(['\x1e', '\x1f']) { emit(text); }
}

pub fn visibleEdgeText(source: &str) -> String {
    let mut result = String::new();
    visitVisibleText(source, |text| appendBounded(&mut result, text, EDGE_TEXT_LIMIT));
    result
}

/// A cursor addresses visible UTF-8 bytes within one message, so a single long
/// reply can be paged without truncating it or hydrating it on the device.
fn visitVisibleText(source: &str, mut emit: impl FnMut(&str)) {
    let mut rest = source;
    while !rest.is_empty() {
        let Some(open) = rest.find('<') else { visitProse(rest, &mut emit); break; };
        visitProse(&rest[..open], &mut emit); rest = &rest[open..];
        let Some(close) = rest.find('>') else { break; };
        let tag = &rest[1..close];
        let name = tag.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("");
        if name == "link" && (tag.contains("type=\"image\"") || tag.contains("type='image'")) {
            let after = &rest[close + 1..];
            let Some(end) = after.find("</link>") else { break; };
            emit(&rest[..=close]); emit("</link>"); rest = &after[end + 7..]; continue;
        }
        if name == "tool" && !tag.starts_with('/') {
            if let Some(tool) = tag.split_once("name=\"").and_then(|(_, t)| t.split_once('"').map(|(n, _)| n))
                .or_else(|| tag.split_once("name='").and_then(|(_, t)| t.split_once('\'').map(|(n, _)| n))) {
                emit("\n"); emit(&toolDisplay(tool, "running")); emit("\n");
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
    let parts = MessagePartCodec::orderedParts(&message.parts);
    for (index, part) in parts.iter().enumerate() {
        match part.kind {
            MessagePartKind::Markdown | MessagePartKind::Status => {
                if !first { emit("\n"); } first = false;
                visitVisibleText(&part.content, &mut emit);
            }
            MessagePartKind::ToolCall => {
                if !first { emit("\n"); } first = false;
                let following = &parts[index + 1..];
                let result = part.toolCallId.as_ref().and_then(|id| following.iter().find(|next|
                    next.kind == MessagePartKind::ToolResult && next.toolCallId.as_ref() == Some(id)))
                    .or_else(|| following.iter()
                        .take_while(|next| !(next.kind == MessagePartKind::ToolCall && next.toolName == part.toolName))
                        .find(|next| next.kind == MessagePartKind::ToolResult && next.toolName == part.toolName &&
                            (part.toolCallId.is_none() || next.toolCallId.is_none())));
                let status = result.and_then(|p| p.attributes.get("status")).map(String::as_str)
                    .unwrap_or(if message.contentStream.is_some() { "running" } else { "unknown" });
                emit(&toolDisplay(part.toolName.as_deref().unwrap_or("未知工具"), status));
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
            if s.starts_with('\x1e') && (lo > 0 || hi < s.len()) {
                if hi == s.len() { actualStart = actualStart.min(at + s.len()); }
                at += s.len(); return;
            }
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
        item.workspaceId = None;
        item.workspaceName = None;
        item
    }).collect()
}

fn projectStreamFields(fields: &BTreeMap<String, CoreValue>, tools: &mut std::collections::BTreeSet<String>) -> Option<BTreeMap<String, CoreValue>> {
    if fields.get("parentBlockId").is_some_and(|v| *v != CoreValue::Null) { return None; }
    let kind = match fields.get("type") { Some(CoreValue::String(v)) => v.as_str(), _ => return None };
    let mut output = BTreeMap::new();
    let text = if let Some(CoreValue::Map(xml)) = fields.get("xml") {
        let tag = match xml.get("tagName") { Some(CoreValue::String(name)) => name.as_str(), _ => return None };
        if !matches!(tag, "tool" | "tool_result") { return None; }
        let Some(CoreValue::Map(attrs)) = xml.get("attributes") else { return None; };
        let Some(CoreValue::String(name)) = attrs.get("name") else { return None; };
        let status = if tag == "tool" { "running" } else {
            match attrs.get("status") { Some(CoreValue::String(status)) => status.as_str(), _ => "unknown" }
        };
        let token = toolDisplay(name, status);
        let block = fields.get("blockId").cloned().unwrap_or(CoreValue::Null);
        let key = format!("{block:?}:{token}");
        if tools.contains(&key) || tools.len() >= EDGE_PART_LIMIT * 2 { return None; }
        tools.insert(key);
        output.insert("type".into(), CoreValue::String(if tag == "tool" { "chunk" } else { "toolStatus" }.into()));
        if tag == "tool" { format!("\n{token}\n") } else { token }
    } else {
        if !matches!(kind, "chunk" | "reset" | "savepoint" | "rollback") { return None; }
        output.insert("type".into(), CoreValue::String(kind.into()));
        if matches!(kind, "reset" | "rollback") { tools.clear(); }
        if let Some(CoreValue::String(id)) = fields.get("id") {
            output.insert("id".into(), CoreValue::String(bounded(id, EDGE_ID_LIMIT)));
        }
        match fields.get("value") {
            Some(CoreValue::String(value)) => visibleEdgeText(value),
            _ => String::new(),
        }
    };
    output.insert("value".into(), CoreValue::String(text));
    Some(output)
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
                    let Some(output) = projectStreamFields(fields, &mut tools) else { continue; };
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

#[cfg(test)]
mod tool_display_tests {
    use super::*;
    fn call(id: &str, sequence: i32) -> MessagePart {
        MessagePart::toolCall(id.into(), sequence, id.into(), "daily_life:get_current_date".into(),
            BTreeMap::from([("secret".into(), "PRIVATE-ARGUMENT".into())]))
    }
    fn result(id: &str, sequence: i32, status: &str) -> MessagePart {
        MessagePart::toolResult(format!("result-{id}"), sequence, Some(id.into()),
            "daily_life:get_current_date".into(), status.into(), "PRIVATE-RESULT-BODY".into())
    }
    #[test]
    fn tool_cards_pair_by_call_id_without_exposing_parameters_or_result_bodies() {
        let mut message = ChatMessage::new_with_markdown_timestamp("assistant".into(), "".into(), 1);
        message.parts = vec![call("a", 0), call("b", 1), result("b", 2, "error"), result("a", 3, "success")];
        let page = chatMessageWindow(vec![message], None, None, 640, 15);
        assert_eq!(page.messages[0].text, "\x1eS|daily_life:get_current_date\x1f\n\x1eF|daily_life:get_current_date\x1f");
        let wire = serde_json::to_string(&page).unwrap();
        assert!(!wire.contains("PRIVATE")); assert!(!wire.contains("toolCallId"));
    }
    #[test]
    fn display_tokens_are_atomic_under_utf8_paging_and_do_not_leak_markup_bodies() {
        let original = format!("前{}\n<tool name='插件:{}'><param>PRIVATE-ARGUMENT</param></tool>\n后{}",
            "文".repeat(90), "名".repeat(60), "文".repeat(90));
        let message = ChatMessage::new_with_markdown_timestamp("assistant".into(), original, 10);
        let (mut timestamp, mut offset) = (None, None); let mut pages = Vec::new();
        loop {
            let page = chatMessageWindow(vec![message.clone()], timestamp, offset, 128, 6);
            let text = page.messages[0].text.clone();
            assert!(text.len() <= 128);
            assert_eq!(text.matches('\x1e').count(), text.matches('\x1f').count());
            assert!(!text.contains("PRIVATE"));
            pages.push(text);
            let Some(cursor) = page.older else { break; };
            timestamp = Some(cursor.timestamp); offset = Some(cursor.offset);
            assert!(pages.len() < 30);
        }
        pages.reverse(); let text = pages.concat();
        assert_eq!(text.matches('\x1e').count(), 1);
        assert!(text.contains("\x1eR|插件:"));
        assert!(!visibleEdgeText("user\x1eS|fake\x1f").contains('\x1e'));
    }
    #[test]
    fn live_xml_projection_only_sends_names_and_normalized_states() {
        let mut tools = std::collections::BTreeSet::new();
        let event = |tag: &str, status: &str, block: u32| {
            let CoreValue::Map(fields) = operit_link::toCoreValue(serde_json::json!({
                "type":"markdownBlockChunk", "blockId":block,
                "value":"PRIVATE-RAW-XML", "xml": {"tagName":tag,
                    "bodyChunk":"PRIVATE-RESULT", "children":[{"bodyChunk":"PRIVATE-ARGUMENT"}],
                    "attributes":{"name":"daily_life:get_current_date","status":status,"secret":"PRIVATE"}}
            })).unwrap() else { panic!() }; fields
        };
        let pending = event("tool", "", 1);
        let call = projectStreamFields(&pending, &mut tools).unwrap();
        assert_eq!(call["value"], CoreValue::String("\n\x1eR|daily_life:get_current_date\x1f\n".into()));
        assert!(projectStreamFields(&pending, &mut tools).is_none());
        let result = projectStreamFields(&event("tool_result", "success", 2), &mut tools).unwrap();
        assert_eq!(result["type"], CoreValue::String("toolStatus".into()));
        assert_eq!(result["value"], CoreValue::String("\x1eS|daily_life:get_current_date\x1f".into()));
        assert!(!format!("{call:?}{result:?}").contains("PRIVATE"));
        let CoreValue::Map(rollback) = operit_link::toCoreValue(serde_json::json!({"type":"rollback","id":"a"})).unwrap() else { panic!() };
        projectStreamFields(&rollback, &mut tools).unwrap();
        assert!(projectStreamFields(&pending, &mut tools).is_some());
    }
}
