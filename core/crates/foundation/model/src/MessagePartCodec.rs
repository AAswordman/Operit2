use std::collections::BTreeMap;

use operit_util::ChatMarkupRegex::{attr_value, tag_ranges, ChatMarkupRegex};

use super::MessagePart::{MessagePart, MessagePartKind};

/// Converts protocol markup at an integration boundary into canonical message parts.
pub struct MessagePartCodec;

impl MessagePartCodec {
    /// Parses assistant output, including interrupted semantic parts.
    pub fn parseAssistantMarkup(content: &str) -> Result<Vec<MessagePart>, String> {
        let mut state = AssistantMarkupStreamState::new();
        state.push(content)?;
        state.finish()
    }

    /// Imports historical assistant text without requiring live-output markup rules.
    ///
    /// Unsupported self-closing semantic tags remain literal Markdown, matching
    /// Operit1's text-based storage. Valid semantic parts around them still parse.
    /// Never reject or discard a historical message because its markup is invalid.
    pub fn parseAssistantMarkupForImport(content: &str) -> Vec<MessagePart> {
        let mut state = AssistantMarkupStreamState {
            preserveUnsupportedTags: true,
            ..AssistantMarkupStreamState::new()
        };
        state
            .push(content)
            .and_then(|()| state.finish())
            .unwrap_or_else(|_| {
                vec![MessagePart::markdown(
                    "part-0".to_string(),
                    0,
                    content.to_string(),
                )]
            })
    }

    /// Serializes canonical parts for a text-only provider protocol boundary.
    pub fn assistantMarkup(parts: &[MessagePart]) -> String {
        let mut markup = String::new();
        for part in Self::orderedParts(parts) {
            match part.kind {
                MessagePartKind::Markdown => markup.push_str(&part.content),
                MessagePartKind::Thinking => {
                    markup.push_str("<think>");
                    markup.push_str(&part.content);
                    markup.push_str("</think>");
                }
                MessagePartKind::ToolCall => Self::appendToolCallMarkup(&mut markup, part),
                MessagePartKind::ToolResult => Self::appendToolResultMarkup(&mut markup, part),
                MessagePartKind::Status => Self::appendStatusMarkup(&mut markup, part),
            }
        }
        markup
    }

    /// Returns parts sorted by their stable message sequence.
    pub fn orderedParts(parts: &[MessagePart]) -> Vec<&MessagePart> {
        let mut ordered = parts.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|part| part.sequence);
        ordered
    }

    /// Returns textual content represented by parts rendered directly in the chat transcript.
    pub fn visibleText(parts: &[MessagePart]) -> String {
        Self::orderedParts(parts)
            .into_iter()
            .filter(|part| {
                matches!(
                    part.kind,
                    MessagePartKind::Markdown | MessagePartKind::Status
                )
            })
            .map(|part| part.content.as_str())
            .collect()
    }

    /// Parses one status tag into a semantic message part.
    fn parseStatus(partId: String, sequence: i32, raw: &str) -> Result<MessagePart, String> {
        let tagName = ChatMarkupRegex::extract_opening_tag_name(raw)
            .ok_or_else(|| "status is missing an opening tag name".to_string())?;
        Ok(MessagePart::status(
            partId,
            sequence,
            Self::tagBody(raw, &tagName)?,
            Self::openingAttributes(raw, &["type"]),
        ))
    }

    /// Parses one tool-call tag into a semantic message part.
    fn parseToolCall(partId: String, sequence: i32, raw: &str) -> Result<MessagePart, String> {
        let toolName = attr_value(raw, "name")
            .ok_or_else(|| "tool call is missing its name attribute".to_string())?;
        let tagName = ChatMarkupRegex::extract_opening_tag_name(raw)
            .ok_or_else(|| "tool call is missing an opening tag name".to_string())?;
        let body = Self::tagBody(raw, &tagName)?;
        let mut parameters = BTreeMap::new();
        for (start, end) in tag_ranges(&body, "param") {
            let parameter = &body[start..end];
            let parameterName = attr_value(parameter, "name")
                .ok_or_else(|| "tool parameter is missing its name attribute".to_string())?;
            parameters.insert(parameterName, Self::tagBody(parameter, "param")?);
        }
        Ok(MessagePart::toolCall(
            partId.clone(),
            sequence,
            partId,
            toolName,
            parameters,
        ))
    }

    /// Retains an interrupted tool call with its received parameters and original source.
    fn parseUnfinishedToolCall(partId: String, sequence: i32, raw: &str) -> MessagePart {
        let mut part = MessagePart::new(
            partId.clone(), sequence, MessagePartKind::ToolCall, raw.to_string(),
        );
        part.toolCallId = Some(partId);
        let openingEnd = raw.find('>').map_or(raw.len(), |end| end + 1);
        part.toolName = attr_value(&raw[..openingEnd], "name");
        if let Some(openEnd) = raw.find('>') {
            for (start, end) in tag_ranges(&raw[openEnd + 1..], "param") {
                let parameter = &raw[openEnd + 1 + start..openEnd + 1 + end];
                if let (Some(name), Some(body)) = (
                    attr_value(parameter, "name"), unfinishedTagBody(parameter, "param"),
                ) {
                    part.attributes.insert(name, body.to_string());
                }
            }
        }
        part
    }

    /// Captures received result metadata and payload without requiring the outer closing tag.
    fn parseUnfinishedToolResult(
        partId: String, sequence: i32, raw: &str, opening: &str,
    ) -> MessagePart {
        let mut part = MessagePart::new(
            partId, sequence, MessagePartKind::ToolResult, String::new(),
        );
        part.toolName = attr_value(opening, "name");
        part.toolCallId = attr_value(opening, "call_id");
        part.attributes = Self::openingAttributes(opening, &["status", "display", "result_data"]);
        for (start, end) in tag_ranges(&raw[opening.len()..], "content") {
            let content = &raw[opening.len() + start..opening.len() + end];
            if let Some(body) = unfinishedTagBody(content, "content") {
                part.content = body.to_string();
            }
        }
        part
    }

    /// Parses one tool-result tag into a semantic message part.
    fn parseToolResult(partId: String, sequence: i32, raw: &str) -> Result<MessagePart, String> {
        let tagName = ChatMarkupRegex::extract_opening_tag_name(raw)
            .ok_or_else(|| "tool result is missing an opening tag name".to_string())?;
        let body = Self::tagBody(raw, &tagName)?;
        let payload = tag_ranges(&body, "content").into_iter().next()
            .and_then(|(start, end)| unfinishedTagBody(&body[start..end], "content"))
            .ok_or_else(|| "tool result is missing its content tag".to_string())?
            .to_string();
        let toolName = attr_value(raw, "name")
            .ok_or_else(|| "tool result is missing its name attribute".to_string())?;
        let status = attr_value(raw, "status")
            .ok_or_else(|| "tool result is missing its status attribute".to_string())?;
        let mut part = MessagePart::toolResult(
            partId,
            sequence,
            attr_value(raw, "call_id"),
            toolName,
            status,
            payload,
        );
        for name in ["display", "result_data"] {
            if let Some(value) = attr_value(raw, name) {
                let decoded = value.replace("&quot;", "\"")
                    .replace("&apos;", "'")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&amp;", "&");
                part.attributes.insert(name.to_string(), decoded);
            }
        }
        Ok(part)
    }

    /// Extracts the received tag body without requiring a closing delimiter.
    fn tagBody(raw: &str, tagName: &str) -> Result<String, String> {
        unfinishedTagBody(raw, tagName)
            .map(str::to_string)
            .ok_or_else(|| format!("message markup tag {tagName} is missing its opening delimiter"))
    }

    /// Reads the selected opening-tag attributes into a stable map.
    fn openingAttributes(raw: &str, names: &[&str]) -> BTreeMap<String, String> {
        names
            .iter()
            .filter_map(|name| attr_value(raw, name).map(|value| ((*name).to_string(), value)))
            .collect()
    }

    /// Creates a deterministic part identifier within one message revision.
    fn partId(sequence: i32) -> String {
        format!("part-{sequence}")
    }

    /// Appends escaped XML attributes to an opening tag.
    fn appendAttributes(markup: &mut String, attributes: &BTreeMap<String, String>) {
        for (name, value) in attributes {
            markup.push(' ');
            markup.push_str(name);
            markup.push_str("=\"");
            markup.push_str(&Self::escapeAttribute(value));
            markup.push('\"');
        }
    }

    /// Appends one structured tool call as protocol markup.
    fn appendToolCallMarkup(markup: &mut String, part: &MessagePart) {
        if !part.content.is_empty() {
            markup.push_str(&part.content);
            return;
        }
        markup.push_str("<tool name=\"");
        markup.push_str(&Self::escapeAttribute(
            part.toolName
                .as_deref()
                .expect("tool-call parts require a tool name"),
        ));
        markup.push_str("\" call_id=\"");
        markup.push_str(&Self::escapeAttribute(
            part.toolCallId
                .as_deref()
                .expect("tool-call parts require a tool-call id"),
        ));
        markup.push_str("\">");
        for (name, value) in &part.attributes {
            markup.push_str("<param name=\"");
            markup.push_str(&Self::escapeAttribute(name));
            markup.push_str("\">");
            markup.push_str(value);
            markup.push_str("</param>");
        }
        markup.push_str("</tool>");
    }

    /// Appends one structured tool result as protocol markup.
    fn appendToolResultMarkup(markup: &mut String, part: &MessagePart) {
        markup.push_str("<tool_result");
        if let Some(toolName) = &part.toolName {
            markup.push_str(" name=\"");
            markup.push_str(&Self::escapeAttribute(toolName));
            markup.push('\"');
        }
        if let Some(callId) = &part.toolCallId {
            markup.push_str(" call_id=\"");
            markup.push_str(&Self::escapeAttribute(callId));
            markup.push('\"');
        }
        let providerAttributes = part.attributes.iter()
            .filter(|(name, _)| !matches!(name.as_str(), "display" | "result_data"))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        Self::appendAttributes(markup, &providerAttributes);
        markup.push_str("><content>");
        markup.push_str(&part.content);
        markup.push_str("</content></tool_result>");
    }

    /// Appends one structured status as protocol markup.
    fn appendStatusMarkup(markup: &mut String, part: &MessagePart) {
        markup.push_str("<status");
        Self::appendAttributes(markup, &part.attributes);
        markup.push('>');
        markup.push_str(&part.content);
        markup.push_str("</status>");
    }

    /// Escapes a string used as an XML attribute value.
    fn escapeAttribute(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('\"', "&quot;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
}

/// Incrementally converts an assistant response stream into canonical message parts.
#[derive(Clone)]
pub struct AssistantMarkupStreamState {
    source: String,
    pending: String,
    parts: Vec<MessagePart>,
    activeThinkingOpenTag: Option<String>,
    activeThinkingCloseTag: Option<String>,
    preserveUnsupportedTags: bool,
    segments: Option<Vec<AssistantMarkupSegment>>,
    activeOpaqueCloseTag: Option<String>,
}

/// Carries original markup or semantic text without serializing canonical parts.
#[derive(Clone, Debug)]
pub struct AssistantMarkupSegment {
    pub kind: Option<MessagePartKind>,
    pub content: String,
}

impl Default for AssistantMarkupStreamState {
    /// Creates an assistant stream state with its canonical empty Markdown part.
    fn default() -> Self {
        Self {
            source: String::new(),
            pending: String::new(),
            parts: vec![MessagePart::markdown(
                "part-0".to_string(),
                0,
                String::new(),
            )],
            activeThinkingOpenTag: None,
            activeThinkingCloseTag: None,
            preserveUnsupportedTags: false,
            segments: None,
            activeOpaqueCloseTag: None,
        }
    }
}

impl AssistantMarkupStreamState {
    /// Creates an assistant-markup stream state with a canonical empty message part.
    pub fn new() -> Self {
        Self::default()
    }

    /// Emits semantic chunks without retaining the full source or building canonical parts.
    pub fn forSegments() -> Self {
        Self { segments: Some(Vec::new()), ..Self::default() }
    }

    /// Drains newly classified source chunks in their original order.
    pub fn takeSegments(&mut self) -> Vec<AssistantMarkupSegment> {
        self.segments.as_mut().map(std::mem::take).unwrap_or_default()
    }

    /// Appends an original semantic delimiter or opaque tool payload to the segment output.
    fn appendOpaque(&mut self, content: &str) {
        if let Some(segments) = &mut self.segments {
            if !content.is_empty() {
                segments.push(AssistantMarkupSegment { kind: None, content: content.into() });
            }
        }
    }

    /// Appends a newly received response chunk and updates only its affected message part.
    pub fn push(&mut self, chunk: &str) -> Result<(), String> {
        if self.segments.is_none() { self.source.push_str(chunk); }
        self.pending.push_str(chunk);
        self.consumePending(false)
    }

    /// Applies a monotonic response snapshot without reparsing its already consumed prefix.
    pub fn pushSnapshot(&mut self, snapshot: &str) -> Result<(), String> {
        let chunk = snapshot.strip_prefix(&self.source).ok_or_else(|| {
            "assistant stream snapshot does not extend the previously consumed source".to_string()
        })?;
        self.push(chunk)
    }

    /// Rebuilds the semantic state from one explicit revision snapshot.
    pub fn resetToSnapshot(&mut self, snapshot: &str) -> Result<(), String> {
        *self = Self::new();
        self.pushSnapshot(snapshot)
    }

    /// Returns the canonical parts accumulated from the response stream so far.
    pub fn parts(&self) -> &[MessagePart] {
        &self.parts
    }

    /// Finalizes semantic parts without requiring interrupted output to close its tags.
    pub fn complete(&mut self) -> Result<(), String> {
        self.consumePending(true)
    }

    /// Completes parsing once the provider response stream has closed.
    pub fn finish(&mut self) -> Result<Vec<MessagePart>, String> {
        self.complete()?;
        Ok(self.parts.clone())
    }

    /// Consumes source buffered across chunk boundaries into complete semantic parts.
    fn consumePending(&mut self, isFinal: bool) -> Result<(), String> {
        loop {
            if let Some(closeTag) = self.activeOpaqueCloseTag.clone() {
                let close = findAsciiCaseInsensitive(&self.pending, &closeTag);
                let end = match close {
                    Some(start) => start + closeTag.len(),
                    None if isFinal => self.pending.len(),
                    None => self.pending.len() - asciiCaseInsensitiveSuffixPrefixLength(&self.pending, &closeTag),
                };
                let content = self.pending[..end].to_string();
                self.appendOpaque(&content);
                self.pending.drain(..end);
                if close.is_some() || isFinal {
                    self.activeOpaqueCloseTag = None;
                    continue;
                }
                return Ok(());
            }
            if let Some(closeTag) = self.activeThinkingCloseTag.clone() {
                if let Some(closeStart) = findAsciiCaseInsensitive(&self.pending, &closeTag) {
                    let content = self.pending[..closeStart].to_string();
                    self.appendThinkingContent(&content);
                    let delimiter = self.pending[closeStart..closeStart + closeTag.len()].to_string();
                    self.appendOpaque(&delimiter);
                    self.pending = self.pending[closeStart + closeTag.len()..].to_string();
                    self.activeThinkingOpenTag = None;
                    self.activeThinkingCloseTag = None;
                    continue;
                }
                if isFinal {
                    let content = std::mem::take(&mut self.pending);
                    self.appendThinkingContent(&content);
                    self.activeThinkingOpenTag = None;
                    self.activeThinkingCloseTag = None;
                    return Ok(());
                }
                let retainedLength =
                    asciiCaseInsensitiveSuffixPrefixLength(&self.pending, &closeTag);
                let committedLength = self.pending.len() - retainedLength;
                if committedLength > 0 {
                    let content = self.pending[..committedLength].to_string();
                    self.appendThinkingContent(&content);
                    self.pending = self.pending[committedLength..].to_string();
                }
                return Ok(());
            }

            let Some(tagStart) = self.pending.find('<') else {
                if !self.pending.is_empty() {
                    let content = std::mem::take(&mut self.pending);
                    self.appendMarkdownContent(&content);
                }
                return Ok(());
            };
            if tagStart > 0 {
                let content = self.pending[..tagStart].to_string();
                self.appendMarkdownContent(&content);
                self.pending = self.pending[tagStart..].to_string();
                continue;
            }

            let Some(openEndOffset) = self.pending.find('>') else {
                if self.isPotentialSemanticOpening() {
                    if isFinal {
                        let content = std::mem::take(&mut self.pending);
                        let tagName = ChatMarkupRegex::extract_opening_tag_name(&content);
                        let isTool = tagName.as_deref()
                            .is_some_and(|name| ChatMarkupRegex::is_tool_tag_name(Some(name)));
                        if !self.preserveUnsupportedTags && isTool {
                            self.appendOpaque(&content);
                            let sequence = self.parts.len() as i32;
                            self.parts.push(MessagePartCodec::parseUnfinishedToolCall(
                                MessagePartCodec::partId(sequence), sequence, &content,
                            ));
                        } else {
                            self.appendMarkdownContent(&content);
                        }
                    }
                    return Ok(());
                }
                self.appendLeadingMarkdownByte();
                continue;
            };
            let openEnd = openEndOffset + 1;
            let openingTag = self.pending[..openEnd].to_string();
            let Some(tagName) = ChatMarkupRegex::extract_opening_tag_name(&openingTag) else {
                self.appendLeadingMarkdownByte();
                continue;
            };
            let normalizedTagName = ChatMarkupRegex::normalize_tool_like_tag_name(Some(&tagName))
                .expect("message markup tag name must normalize")
                .to_ascii_lowercase();
            if !isSemanticTag(&normalizedTagName) {
                self.appendMarkdownContent(&openingTag);
                self.pending = self.pending[openEnd..].to_string();
                continue;
            }
            if openingTag.trim_end().ends_with("/>") {
                if self.segments.is_some() {
                    self.appendOpaque(&openingTag);
                    self.pending.drain(..openEnd);
                    continue;
                }
                if self.preserveUnsupportedTags {
                    self.appendMarkdownContent(&openingTag);
                    self.pending = self.pending[openEnd..].to_string();
                    continue;
                }
                return Err(format!(
                    "assistant semantic tag cannot be self-closing: {tagName}"
                ));
            }
            if normalizedTagName == "think" || normalizedTagName == "thinking" {
                let sequence = self.parts.len() as i32;
                if self.segments.is_none() { self.parts.push(MessagePart::thinking(
                    MessagePartCodec::partId(sequence),
                    sequence,
                    String::new(),
                )); }
                self.appendOpaque(&openingTag);
                self.activeThinkingOpenTag = Some(openingTag);
                self.pending = self.pending[openEnd..].to_string();
                self.activeThinkingCloseTag = Some(format!("</{tagName}>"));
                continue;
            }

            let closeTag = format!("</{tagName}>");
            if self.segments.is_some() {
                self.appendOpaque(&openingTag);
                self.pending.drain(..openEnd);
                self.activeOpaqueCloseTag = Some(closeTag);
                continue;
            }
            let Some(relativeCloseStart) =
                findAsciiCaseInsensitive(&self.pending[openEnd..], &closeTag)
            else {
                if isFinal {
                    // Historical storage includes unfinished output as literal source.
                    if self.preserveUnsupportedTags {
                        let content = std::mem::take(&mut self.pending);
                        self.appendMarkdownContent(&content);
                        return Ok(());
                    }
                    let raw = std::mem::take(&mut self.pending);
                    let sequence = self.parts.len() as i32;
                    let partId = MessagePartCodec::partId(sequence);
                    let part = match normalizedTagName.as_str() {
                        "tool" => MessagePartCodec::parseUnfinishedToolCall(partId, sequence, &raw),
                        "tool_result" => MessagePartCodec::parseUnfinishedToolResult(
                            partId, sequence, &raw, &openingTag,
                        ),
                        "status" => MessagePart::status(
                            partId, sequence, raw[openEnd..].to_string(),
                            MessagePartCodec::openingAttributes(&openingTag, &["type"]),
                        ),
                        _ => unreachable!("unfinished semantic tags must match their parser"),
                    };
                    self.parts.push(part);
                }
                return Ok(());
            };
            let closeStart = openEnd + relativeCloseStart;
            let rawEnd = closeStart + closeTag.len();
            let raw = self.pending[..rawEnd].to_string();
            let sequence = self.parts.len() as i32;
            let partId = MessagePartCodec::partId(sequence);
            let parsedPart = match normalizedTagName.as_str() {
                "tool" => MessagePartCodec::parseToolCall(partId, sequence, &raw),
                "tool_result" => MessagePartCodec::parseToolResult(partId, sequence, &raw),
                "status" => MessagePartCodec::parseStatus(partId, sequence, &raw),
                _ => unreachable!("semantic tag classification must match its parser"),
            };
            let part = parsedPart?;
            self.parts.push(part);
            self.pending = self.pending[rawEnd..].to_string();
        }
    }

    /// Appends Markdown source to the active Markdown part or creates the next one.
    fn appendMarkdownContent(&mut self, content: &str) {
        if content.is_empty() {
            return;
        }
        if let Some(segments) = &mut self.segments {
            segments.push(AssistantMarkupSegment { kind: Some(MessagePartKind::Markdown), content: content.into() });
            return;
        }
        let extendsMarkdownPart = self
            .parts
            .last()
            .map(|part| part.kind == MessagePartKind::Markdown)
            .unwrap_or(false);
        if extendsMarkdownPart {
            let part = self
                .parts
                .last_mut()
                .expect("a Markdown part must exist when extending Markdown content");
            part.content.push_str(content);
            return;
        }
        let sequence = self.parts.len() as i32;
        let mut part =
            MessagePart::markdown(MessagePartCodec::partId(sequence), sequence, String::new());
        part.content.push_str(content);
        self.parts.push(part);
    }

    /// Appends source to the currently open thinking part.
    fn appendThinkingContent(&mut self, content: &str) {
        if content.is_empty() {
            return;
        }
        if let Some(segments) = &mut self.segments {
            segments.push(AssistantMarkupSegment { kind: Some(MessagePartKind::Thinking), content: content.into() });
            return;
        }
        let part = self
            .parts
            .last_mut()
            .expect("an active thinking tag must have a thinking part");
        assert_eq!(part.kind, MessagePartKind::Thinking);
        part.content.push_str(content);
    }

    /// Appends the first pending byte to Markdown text to make parsing progress.
    fn appendLeadingMarkdownByte(&mut self) {
        let byte = self.pending.as_bytes()[0];
        let content = (byte as char).to_string();
        self.appendMarkdownContent(&content);
        self.pending = self.pending[1..].to_string();
    }

    /// Returns whether the trailing partial opening tag can still become a semantic tag.
    fn isPotentialSemanticOpening(&self) -> bool {
        let Some(namePrefix) = self.pending.strip_prefix('<') else {
            return false;
        };
        if namePrefix.is_empty() {
            return true;
        }
        if namePrefix.starts_with('/') {
            return false;
        }
        let candidate = namePrefix
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect::<String>();
        if candidate.is_empty() {
            return false;
        }
        let normalized = candidate.to_ascii_lowercase();
        if candidate.len() < namePrefix.len() {
            return isSemanticTagName(&normalized);
        }
        isSemanticTagName(&normalized)
            || ["think", "thinking", "tool", "tool_result", "status"]
                .iter()
                .any(|name| name.starts_with(&normalized))
    }
}

/// Reads the received body of a tag whose closing delimiter may not have arrived.
fn unfinishedTagBody<'a>(raw: &'a str, tagName: &str) -> Option<&'a str> {
    let start = raw.find('>')? + 1;
    let body = &raw[start..];
    let closing = format!("</{tagName}>");
    let end = match findAsciiCaseInsensitive(body, &closing) {
        Some(end) => end,
        None => body.len() - asciiCaseInsensitiveSuffixPrefixLength(body, &closing),
    };
    Some(&body[..end])
}

/// Returns whether a complete tag name denotes a semantic message part.
fn isSemanticTagName(tagName: &str) -> bool {
    isSemanticTag(
        &ChatMarkupRegex::normalize_tool_like_tag_name(Some(tagName))
            .expect("message markup tag name must normalize")
            .to_ascii_lowercase(),
    )
}

/// Returns whether a normalized tag has an independent semantic message-part representation.
fn isSemanticTag(tagName: &str) -> bool {
    matches!(
        tagName,
        "think" | "thinking" | "tool" | "tool_result" | "status"
    )
}

/// Finds an ASCII-only tag without changing the byte offsets of the source string.
fn findAsciiCaseInsensitive(content: &str, pattern: &str) -> Option<usize> {
    let patternBytes = pattern.as_bytes();
    if patternBytes.is_empty() || content.len() < patternBytes.len() {
        return None;
    }
    content
        .as_bytes()
        .windows(patternBytes.len())
        .position(|candidate| candidate.eq_ignore_ascii_case(patternBytes))
}

/// Returns the longest ASCII-case-insensitive suffix matching a closing-tag prefix.
fn asciiCaseInsensitiveSuffixPrefixLength(content: &str, closingTag: &str) -> usize {
    let contentBytes = content.as_bytes();
    let closingBytes = closingTag.as_bytes();
    for length in (1..closingTag.len()).rev() {
        if length <= contentBytes.len()
            && contentBytes[contentBytes.len() - length..]
                .eq_ignore_ascii_case(&closingBytes[..length])
        {
            return length;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::{AssistantMarkupStreamState, MessagePartCodec, MessagePartKind};

    /// Historical self-closing tags must survive import exactly, not reject a chat.
    #[test]
    fn import_preserves_self_closing_semantic_tags() {
        for source in [
            "前文<tool_NtfY/>后文",
            "before<tool_NtfY name=\"read_file\" />after",
            "<status type=\"completion\"/>",
            "<tool_result_NtfY name=\"read_file\"/>",
            "<think/>正文<thinking />",
        ] {
            let parts = MessagePartCodec::parseAssistantMarkupForImport(source);
            assert!(parts
                .iter()
                .all(|part| part.kind == MessagePartKind::Markdown));
            assert_eq!(MessagePartCodec::assistantMarkup(&parts), source);
        }
    }

    /// Invalid historical fragments must not suppress valid neighboring semantics.
    #[test]
    fn import_preserves_valid_parts_around_unsupported_tags() {
        let parts = MessagePartCodec::parseAssistantMarkupForImport(
            "before<tool_NtfY/><think>reasoning</think><tool name=\"read_file\"><param name=\"path\">README.md</param></tool><status type=\"completion\"/>after",
        );
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0].kind, MessagePartKind::Markdown);
        assert_eq!(parts[0].content, "before<tool_NtfY/>");
        assert_eq!(parts[1].kind, MessagePartKind::Thinking);
        assert_eq!(parts[1].content, "reasoning");
        assert_eq!(parts[2].kind, MessagePartKind::ToolCall);
        assert_eq!(parts[2].toolName.as_deref(), Some("read_file"));
        assert_eq!(
            parts[2].attributes.get("path").map(String::as_str),
            Some("README.md")
        );
        assert_eq!(parts[3].kind, MessagePartKind::Markdown);
        assert_eq!(parts[3].content, "<status type=\"completion\"/>after");
        for (sequence, part) in parts.iter().enumerate() {
            assert_eq!(part.sequence, sequence as i32);
            assert_eq!(part.partId, format!("part-{sequence}"));
        }
    }

    /// Incomplete or unrecognizable historical markup remains readable source text.
    #[test]
    fn import_preserves_malformed_and_incomplete_markup() {
        for source in [
            "前文<tool name=\"read_file\">unfinished",
            "text<tool>missing name</tool>end",
            "text<tool_result name=\"read_file\">missing payload</tool_result>end",
            "text<tool_NtfY",
            "text<unknown/>end",
            "",
        ] {
            let parts = MessagePartCodec::parseAssistantMarkupForImport(source);
            assert_eq!(MessagePartCodec::assistantMarkup(&parts), source);
        }
    }

    /// Valid historical markup retains the normal canonical representation.
    #[test]
    fn import_matches_strict_parser_for_valid_markup() {
        let source = "answer<think>reasoning</think><tool name=\"read_file\"><param name=\"path\">README.md</param></tool><tool_result name=\"read_file\" status=\"success\"><content>done</content></tool_result><status type=\"warning\">careful</status>";
        assert_eq!(
            MessagePartCodec::parseAssistantMarkupForImport(source),
            MessagePartCodec::parseAssistantMarkup(source).unwrap(),
        );
    }

    /// Leniency at the import boundary must not change validation of live output.
    #[test]
    fn import_does_not_relax_live_assistant_markup_validation() {
        let source = "<tool_NtfY/>";
        assert!(MessagePartCodec::parseAssistantMarkup(source).is_err());
        let mut stream = AssistantMarkupStreamState::new();
        assert!(stream.push(source).is_err());
        let parts = MessagePartCodec::parseAssistantMarkupForImport(source);
        assert_eq!(MessagePartCodec::assistantMarkup(&parts), source);
    }

    /// Verifies that protocol markup is converted into independent semantic parts.
    #[test]
    fn parses_assistant_markup_into_semantic_parts() {
        let parts = MessagePartCodec::parseAssistantMarkup(
            "Answer<think>reasoning</think><tool name=\"read_file\"><param name=\"path\">README.md</param></tool><tool_result name=\"read_file\" status=\"success\"><content>done</content></tool_result><status type=\"warning\">careful</status>",
        )
        .expect("complete assistant markup must parse");

        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].kind, MessagePartKind::Markdown);
        assert_eq!(parts[0].content, "Answer");
        assert_eq!(parts[1].kind, MessagePartKind::Thinking);
        assert_eq!(parts[1].content, "reasoning");
        assert_eq!(parts[2].kind, MessagePartKind::ToolCall);
        assert_eq!(parts[2].toolName.as_deref(), Some("read_file"));
        assert_eq!(
            parts[2].attributes.get("path"),
            Some(&"README.md".to_string())
        );
        assert_eq!(parts[3].kind, MessagePartKind::ToolResult);
        assert_eq!(parts[3].toolName.as_deref(), Some("read_file"));
        assert_eq!(parts[3].content, "done");
        assert_eq!(parts[4].kind, MessagePartKind::Status);
        assert_eq!(
            parts[4].attributes.get("type"),
            Some(&"warning".to_string())
        );
    }

    /// Verifies that structured parts recreate provider protocol markup only on demand.
    #[test]
    fn serializes_semantic_parts_for_provider_protocols() {
        let parts = MessagePartCodec::parseAssistantMarkup(
            "Answer<think>reasoning</think><tool name=\"read_file\"><param name=\"path\">README.md</param></tool><tool_result name=\"read_file\" status=\"success\"><content>done</content></tool_result><status type=\"warning\">careful</status>",
        )
        .expect("complete assistant markup must parse");

        assert_eq!(
            MessagePartCodec::assistantMarkup(&parts),
            "Answer<think>reasoning</think><tool name=\"read_file\" call_id=\"part-2\"><param name=\"path\">README.md</param></tool><tool_result name=\"read_file\" status=\"success\"><content>done</content></tool_result><status type=\"warning\">careful</status>",
        );
    }

    /// Verifies that stream chunks update only their active semantic message part.
    #[test]
    fn stream_state_preserves_completed_parts() {
        let mut state = AssistantMarkupStreamState::new();
        state
            .push("Answer<th")
            .expect("partial thinking tag must be accepted while streaming");
        assert_eq!(state.parts().len(), 1);

        state
            .push("ink>plan **bold**</thi")
            .expect("partial thinking close tag must be accepted while streaming");
        assert_eq!(state.parts().len(), 2);
        assert_eq!(state.parts()[0].content, "Answer");
        assert_eq!(state.parts()[1].kind, MessagePartKind::Thinking);
        assert_eq!(state.parts()[1].content, "plan **bold**");

        state
            .push("nk><status type=\"warning\">careful</status>Done")
            .expect("completed semantic tags must be converted into message parts");
        let parts = state
            .finish()
            .expect("complete assistant stream must finish");

        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0].kind, MessagePartKind::Markdown);
        assert_eq!(parts[0].content, "Answer");
        assert_eq!(parts[1].kind, MessagePartKind::Thinking);
        assert_eq!(parts[1].content, "plan **bold**");
        assert_eq!(parts[2].kind, MessagePartKind::Status);
        assert_eq!(parts[2].content, "careful");
        assert_eq!(parts[3].kind, MessagePartKind::Markdown);
        assert_eq!(parts[3].content, "Done");
    }

    /// Verifies case-insensitive thinking close tags split across stream chunks.
    #[test]
    fn stream_state_retains_case_insensitive_closing_tag_prefixes() {
        let mut state = AssistantMarkupStreamState::new();
        state
            .push("<THINK>reasoning</TH")
            .expect("partial uppercase thinking close tag must remain pending");
        state
            .push("INK>Answer")
            .expect("uppercase thinking close tag must complete");
        let parts = state
            .finish()
            .expect("complete assistant stream must finish");

        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].kind, MessagePartKind::Markdown);
        assert_eq!(parts[0].content, "");
        assert_eq!(parts[1].kind, MessagePartKind::Thinking);
        assert_eq!(parts[1].content, "reasoning");
        assert_eq!(parts[2].kind, MessagePartKind::Markdown);
        assert_eq!(parts[2].content, "Answer");
    }

    /// Verifies generated tool tag names remain semantic when their names span chunks.
    #[test]
    fn stream_state_recognizes_generated_tool_tags_across_chunks() {
        let mut state = AssistantMarkupStreamState::new();
        state
            .push("<tool_read")
            .expect("partial generated tool tag name must remain pending");
        state
            .push("_42 name=\"read_file\"><param name=\"path\">a.txt</param></tool_read_42>")
            .expect("generated tool tag must complete");
        let parts = state
            .finish()
            .expect("complete assistant stream must finish");

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].kind, MessagePartKind::Markdown);
        assert_eq!(parts[0].content, "");
        assert_eq!(parts[1].kind, MessagePartKind::ToolCall);
        assert_eq!(parts[1].toolName.as_deref(), Some("read_file"));
        assert_eq!(
            parts[1].attributes.get("path").map(String::as_str),
            Some("a.txt")
        );
    }

    /// Verifies that a response with no provider chunks retains a persistable Markdown part.
    #[test]
    fn stream_state_keeps_canonical_part_without_provider_output() {
        let mut state = AssistantMarkupStreamState::new();
        let parts = state
            .finish()
            .expect("an empty assistant stream must finish");

        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].kind, MessagePartKind::Markdown);
        assert_eq!(parts[0].content, "");
    }

    /// Unfinished thinking remains persistable without a closing tag or lost source bytes.
    #[test]
    fn preserves_unfinished_thinking_content() {
        for opening in ["think", "thinking", "THINK"] {
            for content in ["", "partial reasoning", "reasoning</thi", "reasoning</TH"] {
                let source = format!("Before<{opening}>{content}");
                let parts = MessagePartCodec::parseAssistantMarkup(&source)
                    .expect("unfinished thinking must remain persistable");
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0].content, "Before");
                assert_eq!(parts[1].kind, MessagePartKind::Thinking);
                assert_eq!(parts[1].content, content);
                assert_eq!(MessagePartCodec::parseAssistantMarkupForImport(&source), parts);
            }
        }
    }

    /// Persisting an unfinished snapshot must not interrupt the live thinking stream.
    #[test]
    fn persists_thinking_snapshots_before_stream_completion() {
        let mut state = AssistantMarkupStreamState::new();
        state.push("Before<think>partial reasoning</thi").unwrap();

        let mut snapshot = AssistantMarkupStreamState::new();
        snapshot.resetToSnapshot(&state.source).unwrap();
        let parts = snapshot.finish().expect("unfinished snapshot must finish");
        assert_eq!(parts[1].kind, MessagePartKind::Thinking);
        assert_eq!(parts[1].content, "partial reasoning</thi");
        assert_eq!(snapshot.finish().unwrap(), parts);

        state.push("nk>After").unwrap();
        let parts = state.finish().unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].content, "partial reasoning");
        assert_eq!(parts[2].content, "After");
    }

    /// Persistence snapshots keep unfinished semantic markup buffered until later chunks close it.
    #[test]
    fn streaming_snapshots_do_not_finalize_pending_semantic_tags() {
        for markup in [
            "<tool_KtEh name=\"read_file\"><param name=\"path\">a.txt</param></tool_KtEh>",
            "<tool_result name=\"read_file\" status=\"success\"><content>payload</content></tool_result>",
            "<status type=\"warning\">careful</status>",
        ] {
            let source = format!("Before{markup}");
            let expected = MessagePartCodec::parseAssistantMarkup(&source).unwrap();
            let mut live = AssistantMarkupStreamState::new();
            for end in 0..=source.len() {
                let content = &source[..end];
                live.pushSnapshot(content).unwrap();
                let mut snapshot = AssistantMarkupStreamState::new();
                snapshot.resetToSnapshot(content).unwrap();
                let persisted = snapshot.parts().to_vec();
                assert_eq!(persisted, live.parts(), "{content}");
                if end >= "Before<".len() && end < source.len() {
                    assert_eq!(persisted.len(), 1, "{content}");
                    assert_eq!(persisted[0].content, "Before");
                }
            }
            assert_eq!(live.finish().unwrap(), expected);
        }
    }

    /// Interrupted tool calls retain their semantic kind, received arguments, and exact source.
    #[test]
    fn preserves_unfinished_tool_calls_at_every_character_boundary() {
        let source = "Before<tool_G543 name=\"read_file\"><param name=\"path\">文档/a.txt</param></tool_G543>";
        let mut live = AssistantMarkupStreamState::new();
        for end in source.char_indices().map(|(end, _)| end)
            .chain(std::iter::once(source.len()))
        {
            let prefix = &source[..end];
            live.pushSnapshot(prefix).unwrap();
            let mut snapshot = AssistantMarkupStreamState::new();
            snapshot.resetToSnapshot(prefix).unwrap();
            let parts = snapshot.finish().expect("interrupted tool snapshots must finish");
            assert_eq!(snapshot.finish().unwrap(), parts);
            if end >= "Before<tool".len() && end < source.len() {
                assert_eq!(parts.len(), 2, "{prefix}");
                assert_eq!(parts[1].kind, MessagePartKind::ToolCall, "{prefix}");
                assert_eq!(MessagePartCodec::assistantMarkup(&parts), prefix);
            }
        }
        assert_eq!(
            live.finish().unwrap(), MessagePartCodec::parseAssistantMarkup(source).unwrap(),
        );
    }

    /// Cancellation retains incomplete parameter values while later chunks can still complete the live call.
    #[test]
    fn unfinished_tool_parameters_remain_structured() {
        let source = "Before<tool name=\"read_file\"><param name=\"path\">文档/a.txt</pa";
        let parts = MessagePartCodec::parseAssistantMarkup(source).unwrap();
        assert_eq!(parts[1].toolName.as_deref(), Some("read_file"));
        assert_eq!(
            parts[1].attributes.get("path").map(String::as_str), Some("文档/a.txt"),
        );
        assert_eq!(MessagePartCodec::assistantMarkup(&parts), source);
        let encoded = serde_json::to_string(&parts).unwrap();
        let restored: Vec<super::MessagePart> = serde_json::from_str(&encoded).unwrap();
        assert_eq!(parts, restored);
    }

    /// Inner parameter and content tags do not require closing delimiters either.
    #[test]
    fn parses_unclosed_bodies_inside_completed_outer_tags() {
        let parts = MessagePartCodec::parseAssistantMarkup(
            "<tool name=\"read_file\"><param name=\"path\">文档/a.txt</tool>",
        ).unwrap();
        assert_eq!(
            parts[1].attributes.get("path").map(String::as_str), Some("文档/a.txt"),
        );
        let parts = MessagePartCodec::parseAssistantMarkup(
            "<tool_result name=\"read_file\" status=\"success\"><content>payload</tool_result>",
        ).unwrap();
        assert_eq!(parts[1].content, "payload");
    }

    /// Result and status snapshots may end before any semantic closing delimiter arrives.
    #[test]
    fn finishes_unclosed_results_and_statuses() {
        for source in [
            "<tool_result name=\"read_file\" status=\"success\"><content>文档/a.txt",
            "<tool_result name=\"read_file\" status=\"success\"><content>文档/a.txt</content>",
        ] {
            let parts = MessagePartCodec::parseAssistantMarkup(source).unwrap();
            assert_eq!(parts[1].kind, MessagePartKind::ToolResult);
            assert_eq!(parts[1].toolName.as_deref(), Some("read_file"));
            assert_eq!(parts[1].content, "文档/a.txt");
        }
        let parts = MessagePartCodec::parseAssistantMarkup("<status type=\"warning\">careful").unwrap();
        assert_eq!(parts[1].kind, MessagePartKind::Status);
        assert_eq!(parts[1].content, "careful");
    }

    /// Completed malformed tool markup still reports invalid fields.
    #[test]
    fn rejects_malformed_completed_tool_markup() {
        for content in [
            "Before<tool name=\"search\"><param name=\"various_search</param></tool>After",
            "Before<tool_result name=\"search\" status=\"success\">PRIVATE-RESULT</tool_result>",
        ] { assert!(MessagePartCodec::parseAssistantMarkup(content).is_err(), "{content}"); }
    }

    /// Verifies that visible text excludes internal thinking and tool protocol payloads.
    #[test]
    fn visible_text_uses_only_directly_rendered_text_parts() {
        let parts = MessagePartCodec::parseAssistantMarkup(
            "Answer<think>internal</think><tool name=\"read_file\"><param name=\"path\">a.txt</param></tool><tool_result name=\"read_file\" status=\"success\"><content>tool payload</content></tool_result><status type=\"warning\">careful</status>",
        )
        .expect("complete assistant markup must parse");

        assert_eq!(MessagePartCodec::visibleText(&parts), "Answercareful");
    }
}
