#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

use crate::util::streamnative::NativeMarkdownSplitter::{
    MarkdownProcessorType, MarkdownSession, NativeMarkdownSplitter, Segment,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarkdownStreamEvent {
    pub chatId: String,
    #[serde(rename = "type")]
    pub eventType: String,
    pub value: Option<String>,
    pub id: Option<String>,
    pub blockId: Option<u64>,
    pub inlineId: Option<u64>,
    pub parentBlockId: Option<u64>,
    pub nodeType: Option<String>,
    pub headerLevel: Option<usize>,
}

pub struct MarkdownRenderEventStream {
    chatId: String,
    parentBlockId: Option<u64>,
    parseXmlChildren: bool,
    block: MarkdownGroupSession,
    nextBlockId: u64,
    activeBlock: Option<ActiveBlock>,
}

struct ActiveBlock {
    id: u64,
    inline: Option<MarkdownGroupSession>,
    xmlChild: Option<Box<XmlChildMarkdownStream>>,
    nextInlineId: u64,
    activeInline: Option<ActiveInline>,
}

struct ActiveInline {
    id: u64,
    nodeType: Option<MarkdownProcessorType>,
}

struct MarkdownGroupSession {
    session: MarkdownSession,
    content: String,
    activeType: Option<Option<MarkdownProcessorType>>,
}

impl MarkdownStreamEvent {
    pub fn savepoint(chatId: String, id: String) -> Self {
        Self {
            chatId,
            eventType: "savepoint".to_string(),
            value: None,
            id: Some(id),
            blockId: None,
            inlineId: None,
            parentBlockId: None,
            nodeType: None,
            headerLevel: None,
        }
    }

    pub fn rollback(chatId: String, id: String) -> Self {
        Self {
            chatId,
            eventType: "rollback".to_string(),
            value: None,
            id: Some(id),
            blockId: None,
            inlineId: None,
            parentBlockId: None,
            nodeType: None,
            headerLevel: None,
        }
    }
}

impl MarkdownRenderEventStream {
    pub fn new(chatId: String) -> Self {
        Self {
            chatId,
            parentBlockId: None,
            parseXmlChildren: true,
            block: MarkdownGroupSession::block(),
            nextBlockId: 0,
            activeBlock: None,
        }
    }

    fn child(chatId: String, parentBlockId: u64) -> Self {
        Self {
            chatId,
            parentBlockId: Some(parentBlockId),
            parseXmlChildren: false,
            block: MarkdownGroupSession::block(),
            nextBlockId: 0,
            activeBlock: None,
        }
    }

    pub fn fromContent(content: String) -> Vec<MarkdownStreamEvent> {
        let mut stream = Self::new(String::new());
        let mut events = stream.pushChunk(&content);
        events.push(stream.completed());
        events
    }

    pub fn pushChunk(&mut self, chunk: &str) -> Vec<MarkdownStreamEvent> {
        let mut events = vec![MarkdownStreamEvent {
            chatId: self.chatId.clone(),
            eventType: "chunk".to_string(),
            value: Some(chunk.to_string()),
            id: None,
            blockId: None,
            inlineId: None,
            parentBlockId: self.parentBlockId,
            nodeType: None,
            headerLevel: None,
        }];

        let segments = self.block.push(chunk);
        for segment in segments {
            if segment.r#type < 0 {
                self.block.activeType = None;
                self.activeBlock = None;
                continue;
            }
            let nodeType = markdownTypeFromSegment(&segment);
            let nodeContent = markdownSegmentContent(&self.block.content, &segment, nodeType);
            if nodeContent.is_empty() {
                continue;
            }

            if self.block.activeType != Some(nodeType) {
                self.nextBlockId += 1;
                self.block.activeType = Some(nodeType);
                self.activeBlock = Some(ActiveBlock {
                    id: self.nextBlockId,
                    inline: if isInlineContainer(nodeType) {
                        Some(MarkdownGroupSession::inline())
                    } else {
                        None
                    },
                    xmlChild: None,
                    nextInlineId: 0,
                    activeInline: None,
                });
                events.push(MarkdownStreamEvent {
                    chatId: self.chatId.clone(),
                    eventType: "markdownBlockStart".to_string(),
                    value: None,
                    id: None,
                    blockId: Some(self.nextBlockId),
                    inlineId: None,
                    parentBlockId: self.parentBlockId,
                    nodeType: markdownTypeLabel(nodeType).map(ToString::to_string),
                    headerLevel: headerLevel(nodeType, &nodeContent),
                });
                let xmlChild =
                    if self.parseXmlChildren && nodeType == Some(MarkdownProcessorType::XmlBlock) {
                        Some(Box::new(XmlChildMarkdownStream::new(
                            self.chatId.clone(),
                            self.nextBlockId,
                        )))
                    } else {
                        None
                    };
                if let Some(block) = self.activeBlock.as_mut() {
                    block.xmlChild = xmlChild;
                }
            }

            if isInlineContainer(nodeType) {
                events.extend(self.inlineChunk(nodeContent));
            } else if let Some(blockId) = self.activeBlock.as_ref().map(|block| block.id) {
                events.push(MarkdownStreamEvent {
                    chatId: self.chatId.clone(),
                    eventType: "markdownBlockChunk".to_string(),
                    value: Some(nodeContent.clone()),
                    id: None,
                    blockId: Some(blockId),
                    inlineId: None,
                    parentBlockId: self.parentBlockId,
                    nodeType: markdownTypeLabel(nodeType).map(ToString::to_string),
                    headerLevel: None,
                });
                if nodeType == Some(MarkdownProcessorType::XmlBlock) {
                    if let Some(block) = self.activeBlock.as_mut() {
                        if let Some(child) = block.xmlChild.as_mut() {
                            events.extend(child.pushChunk(
                                self.chatId.clone(),
                                blockId,
                                &nodeContent,
                            ));
                        }
                    }
                }
            }
        }

        events
    }

    pub fn completed(&self) -> MarkdownStreamEvent {
        MarkdownStreamEvent {
            chatId: self.chatId.clone(),
            eventType: "completed".to_string(),
            value: None,
            id: None,
            blockId: None,
            inlineId: None,
            parentBlockId: self.parentBlockId,
            nodeType: None,
            headerLevel: None,
        }
    }

    fn inlineChunk(&mut self, content: String) -> Vec<MarkdownStreamEvent> {
        let Some(block) = self.activeBlock.as_mut() else {
            return Vec::new();
        };
        let Some(inline) = block.inline.as_mut() else {
            return Vec::new();
        };

        let mut events = Vec::new();
        let blockId = block.id;
        let segments = inline.push(&content);
        for segment in segments {
            if segment.r#type < 0 {
                inline.activeType = None;
                block.activeInline = None;
                continue;
            }
            let nodeType = markdownTypeFromSegment(&segment);
            let nodeContent = markdownSegmentContent(&inline.content, &segment, nodeType);
            if nodeContent.is_empty() {
                continue;
            }

            if inline.activeType != Some(nodeType) {
                block.nextInlineId += 1;
                inline.activeType = Some(nodeType);
                block.activeInline = Some(ActiveInline {
                    id: block.nextInlineId,
                    nodeType,
                });
                events.push(MarkdownStreamEvent {
                    chatId: self.chatId.clone(),
                    eventType: "markdownInlineStart".to_string(),
                    value: None,
                    id: None,
                    blockId: Some(blockId),
                    inlineId: Some(block.nextInlineId),
                    parentBlockId: self.parentBlockId,
                    nodeType: markdownTypeLabel(nodeType).map(ToString::to_string),
                    headerLevel: None,
                });
            }

            if let Some(activeInline) = block.activeInline.as_ref() {
                events.push(MarkdownStreamEvent {
                    chatId: self.chatId.clone(),
                    eventType: "markdownInlineChunk".to_string(),
                    value: Some(nodeContent),
                    id: None,
                    blockId: Some(blockId),
                    inlineId: Some(activeInline.id),
                    parentBlockId: self.parentBlockId,
                    nodeType: markdownTypeLabel(activeInline.nodeType).map(ToString::to_string),
                    headerLevel: None,
                });
            }
        }
        events
    }
}

struct XmlChildMarkdownStream {
    raw: String,
    emittedBodyEnd: usize,
    tagName: Option<String>,
    markdown: MarkdownRenderEventStream,
    closed: bool,
}

impl XmlChildMarkdownStream {
    fn new(chatId: String, parentBlockId: u64) -> Self {
        Self {
            raw: String::new(),
            emittedBodyEnd: 0,
            tagName: None,
            markdown: MarkdownRenderEventStream::child(chatId, parentBlockId),
            closed: false,
        }
    }

    fn pushChunk(
        &mut self,
        chatId: String,
        parentBlockId: u64,
        chunk: &str,
    ) -> Vec<MarkdownStreamEvent> {
        if self.closed {
            return Vec::new();
        }
        self.raw.push_str(chunk);

        let Some((tagName, bodyStart)) = self.openingThinkTag() else {
            return Vec::new();
        };
        if self.tagName.is_none() {
            self.tagName = Some(tagName.clone());
            self.emittedBodyEnd = bodyStart;
        }

        let closePattern = format!("</{}>", tagName);
        let searchStart = self.emittedBodyEnd.max(bodyStart);
        let closeStart = findAsciiCaseInsensitive(&self.raw, &closePattern, searchStart);
        let bodyEnd = if let Some(closeStart) = closeStart {
            self.closed = true;
            closeStart
        } else {
            let pendingCloseBytes = closingPrefixSuffixLength(&self.raw, bodyStart, &closePattern);
            self.raw.len() - pendingCloseBytes
        };

        let emitStart = self.emittedBodyEnd.max(bodyStart);
        if bodyEnd <= emitStart {
            if self.closed {
                return vec![self.markdown.completed()];
            }
            return Vec::new();
        }

        let bodyChunk = self.raw[emitStart..bodyEnd].to_string();
        self.emittedBodyEnd = bodyEnd;
        let mut events = self.markdown.pushChunk(&bodyChunk);
        if self.closed {
            events.push(MarkdownStreamEvent {
                chatId,
                eventType: "completed".to_string(),
                value: None,
                id: None,
                blockId: None,
                inlineId: None,
                parentBlockId: Some(parentBlockId),
                nodeType: None,
                headerLevel: None,
            });
        }
        events
    }

    fn openingThinkTag(&self) -> Option<(String, usize)> {
        let bytes = self.raw.as_bytes();
        let mut index = 0;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'<' {
            return None;
        }
        let nameStart = index + 1;
        let mut nameEnd = nameStart;
        while nameEnd < bytes.len()
            && (bytes[nameEnd].is_ascii_alphanumeric()
                || bytes[nameEnd] == b'_'
                || bytes[nameEnd] == b':'
                || bytes[nameEnd] == b'-')
        {
            nameEnd += 1;
        }
        if nameEnd == nameStart {
            return None;
        }
        let tagName = self.raw[nameStart..nameEnd].to_ascii_lowercase();
        if tagName != "think" && tagName != "thinking" {
            return None;
        }
        let tagEnd = findOpeningTagEnd(bytes, nameEnd)?;
        Some((tagName, tagEnd + 1))
    }
}

fn findOpeningTagEnd(bytes: &[u8], start: usize) -> Option<usize> {
    let mut quote: Option<u8> = None;
    let mut index = start;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(currentQuote) = quote {
            if byte == currentQuote {
                quote = None;
            }
        } else if byte == b'\'' || byte == b'"' {
            quote = Some(byte);
        } else if byte == b'>' {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn findAsciiCaseInsensitive(haystack: &str, needle: &str, start: usize) -> Option<usize> {
    let haystackBytes = haystack.as_bytes();
    let needleBytes = needle.as_bytes();
    if needleBytes.is_empty() || haystackBytes.len() < needleBytes.len() {
        return None;
    }
    let lastStart = haystackBytes.len() - needleBytes.len();
    if start > lastStart {
        return None;
    }
    let mut index = start;
    while index <= lastStart {
        let mut matched = true;
        for offset in 0..needleBytes.len() {
            if !haystackBytes[index + offset].eq_ignore_ascii_case(&needleBytes[offset]) {
                matched = false;
                break;
            }
        }
        if matched {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn closingPrefixSuffixLength(raw: &str, bodyStart: usize, closingPattern: &str) -> usize {
    let bytes = raw.as_bytes();
    let pattern = closingPattern.as_bytes();
    let bodyLength = raw.len().saturating_sub(bodyStart);
    let maxLength = pattern.len().min(bodyLength);
    for length in (1..=maxLength).rev() {
        let suffixStart = raw.len() - length;
        let mut matched = true;
        for offset in 0..length {
            if !bytes[suffixStart + offset].eq_ignore_ascii_case(&pattern[offset]) {
                matched = false;
                break;
            }
        }
        if matched {
            return length;
        }
    }
    0
}

impl MarkdownGroupSession {
    fn block() -> Self {
        Self {
            session: NativeMarkdownSplitter::create_block_session(),
            content: String::new(),
            activeType: None,
        }
    }

    fn inline() -> Self {
        Self {
            session: NativeMarkdownSplitter::create_inline_session(),
            content: String::new(),
            activeType: None,
        }
    }

    fn push(&mut self, chunk: &str) -> Vec<Segment> {
        self.content.push_str(chunk);
        self.session.push(chunk)
    }
}

fn markdownTypeFromSegment(segment: &Segment) -> Option<MarkdownProcessorType> {
    let nodeType = match segment.r#type {
        0 => MarkdownProcessorType::Header,
        1 => MarkdownProcessorType::BlockQuote,
        2 => MarkdownProcessorType::CodeBlock,
        3 => MarkdownProcessorType::OrderedList,
        4 => MarkdownProcessorType::UnorderedList,
        5 => MarkdownProcessorType::HorizontalRule,
        6 => MarkdownProcessorType::BlockLatex,
        7 => MarkdownProcessorType::Table,
        8 => MarkdownProcessorType::XmlBlock,
        9 => MarkdownProcessorType::Bold,
        10 => MarkdownProcessorType::Italic,
        11 => MarkdownProcessorType::InlineCode,
        12 => MarkdownProcessorType::Link,
        13 => MarkdownProcessorType::Image,
        14 => MarkdownProcessorType::Strikethrough,
        15 => MarkdownProcessorType::Underline,
        16 => MarkdownProcessorType::InlineLatex,
        18 => MarkdownProcessorType::HtmlBreak,
        17 => return None,
        _ => unreachable!("unknown markdown processor type ordinal"),
    };
    Some(nodeType)
}

fn markdownTypeLabel(nodeType: Option<MarkdownProcessorType>) -> Option<&'static str> {
    match nodeType {
        Some(MarkdownProcessorType::Header) => Some("Header"),
        Some(MarkdownProcessorType::BlockQuote) => Some("BlockQuote"),
        Some(MarkdownProcessorType::CodeBlock) => Some("CodeBlock"),
        Some(MarkdownProcessorType::OrderedList) => Some("OrderedList"),
        Some(MarkdownProcessorType::UnorderedList) => Some("UnorderedList"),
        Some(MarkdownProcessorType::HorizontalRule) => Some("HorizontalRule"),
        Some(MarkdownProcessorType::BlockLatex) => Some("BlockLatex"),
        Some(MarkdownProcessorType::Table) => Some("Table"),
        Some(MarkdownProcessorType::XmlBlock) => Some("XmlBlock"),
        Some(MarkdownProcessorType::Bold) => Some("Bold"),
        Some(MarkdownProcessorType::Italic) => Some("Italic"),
        Some(MarkdownProcessorType::InlineCode) => Some("InlineCode"),
        Some(MarkdownProcessorType::Link) => Some("Link"),
        Some(MarkdownProcessorType::Image) => Some("Image"),
        Some(MarkdownProcessorType::Strikethrough) => Some("Strikethrough"),
        Some(MarkdownProcessorType::Underline) => Some("Underline"),
        Some(MarkdownProcessorType::InlineLatex) => Some("InlineLatex"),
        Some(MarkdownProcessorType::HtmlBreak) => Some("HtmlBreak"),
        Some(MarkdownProcessorType::PlainText) | None => None,
    }
}

fn headerLevel(nodeType: Option<MarkdownProcessorType>, content: &str) -> Option<usize> {
    if nodeType != Some(MarkdownProcessorType::Header) {
        return None;
    }
    let level = content.chars().take_while(|ch| *ch == '#').count();
    if (1..=6).contains(&level) {
        Some(level)
    } else {
        None
    }
}

fn markdownSegmentContent(
    content: &str,
    segment: &Segment,
    nodeType: Option<MarkdownProcessorType>,
) -> String {
    if nodeType == Some(MarkdownProcessorType::HtmlBreak) {
        "\n".to_string()
    } else {
        content
            .chars()
            .skip(segment.start)
            .take(segment.end.saturating_sub(segment.start))
            .collect()
    }
}

fn isInlineContainer(nodeType: Option<MarkdownProcessorType>) -> bool {
    !matches!(
        nodeType,
        Some(MarkdownProcessorType::CodeBlock)
            | Some(MarkdownProcessorType::BlockLatex)
            | Some(MarkdownProcessorType::Table)
            | Some(MarkdownProcessorType::XmlBlock)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_tool_events_immediately_after_think_closes() {
        let mut stream = MarkdownRenderEventStream::new("chat".to_string());

        let think_open_events = stream.pushChunk("<think>");
        let think_body_events = stream.pushChunk("plan");
        let think_close_events = stream.pushChunk("</think>");
        let tool_events = stream
            .pushChunk(r#"<tool name="read_file"><param name="path">README.md</param></tool>"#);
        let tool_result_events =
            stream.pushChunk(r#"<tool_result name="read_file">ok</tool_result>"#);

        assert!(
            think_open_events.iter().any(|event| {
                event.parentBlockId.is_none()
                    && event.eventType == "markdownBlockStart"
                    && event.nodeType.as_deref() == Some("XmlBlock")
            }),
            "top-level think XML block should start immediately"
        );
        assert!(
            think_body_events.iter().any(|event| {
                event.parentBlockId.is_some()
                    && event.eventType == "markdownInlineChunk"
                    && event.value.as_deref() == Some("plan")
            }),
            "think body should emit child markdown while thinking is open"
        );
        assert!(
            think_close_events
                .iter()
                .any(|event| { event.parentBlockId.is_some() && event.eventType == "completed" }),
            "think child markdown stream should complete when </think> arrives"
        );
        assert!(
            tool_events.iter().any(|event| {
                event.parentBlockId.is_none()
                    && event.eventType == "markdownBlockChunk"
                    && event
                        .value
                        .as_deref()
                        .is_some_and(|value| value.contains("<tool name=\"read_file\""))
            }),
            "tool XML should emit as a top-level markdown block immediately after think"
        );
        assert!(
            tool_result_events.iter().any(|event| {
                event.parentBlockId.is_none()
                    && event.eventType == "markdownBlockChunk"
                    && event
                        .value
                        .as_deref()
                        .is_some_and(|value| value.contains("<tool_result name=\"read_file\""))
            }),
            "tool_result XML should emit as a top-level markdown block immediately after tool"
        );
    }
}
