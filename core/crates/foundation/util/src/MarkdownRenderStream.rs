#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::streamnative::MarkdownInlineStream::{InlineKind, InlineToken, MarkdownInlineStream};
use crate::streamnative::NativeMarkdownSplitter::{
    MarkdownProcessorType, MarkdownSession, NativeMarkdownSplitter, Segment,
};
use crate::streamnative::NativeXmlSplitter::{NativeXmlSplitter, XmlNode, XmlOpeningTag};
use crate::ChatMarkupRegex::ChatMarkupRegex;

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
    pub xml: Option<MarkdownXmlStreamEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarkdownXmlChildStreamEvent {
    pub index: usize,
    pub tagName: Option<String>,
    pub attributes: Option<HashMap<String, String>>,
    pub bodyChunk: Option<String>,
    pub isClosed: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarkdownXmlStreamEvent {
    pub tagName: Option<String>,
    pub attributes: Option<HashMap<String, String>>,
    pub bodyChunk: Option<String>,
    pub children: Vec<MarkdownXmlChildStreamEvent>,
    pub isClosed: Option<bool>,
}

pub struct MarkdownRenderEventStream {
    chatId: String,
    parentBlockId: Option<u64>,
    block: MarkdownGroupSession,
    nextBlockId: u64,
    activeBlock: Option<ActiveBlock>,
}

struct ActiveBlock {
    id: u64,
    inline: Option<MarkdownInlineStream>,
    prefix: Option<BlockPrefix>,
    styleIds: Vec<(u64, InlineKind)>,
    xml: Option<XmlBlockMetadata>,
    xmlMarkdown: Option<Box<MarkdownRenderEventStream>>,
    nextInlineId: u64,
    activeInline: Option<ActiveInline>,
}

struct XmlBlockMetadata {
    raw: String,
    opening: Option<XmlOpeningTag>,
    isClosed: bool,
    emittedBodyEnd: usize,
}

impl XmlBlockMetadata {
    /// Creates metadata storage for one XML markdown block.
    fn new() -> Self {
        Self {
            raw: String::new(),
            opening: None,
            isClosed: false,
            emittedBodyEnd: 0,
        }
    }

    /// Appends one already-delimited XML block chunk from the Markdown stream.
    fn append(&mut self, chunk: &str) {
        self.raw.push_str(chunk);
        if self.opening.is_none() {
            self.opening = NativeXmlSplitter::parse_opening_tag(&self.raw);
        }
    }

    /// Emits only the outer node's body, preserving every nested tag verbatim.
    /// A possible outer closing-tag suffix stays buffered until it is resolved.
    fn takeBodyChunk(&mut self) -> String {
        let Some(opening) = self.opening.as_ref() else {
            return String::new();
        };
        let closing = format!("</{}>", opening.tag_name);
        let body = &self.raw[opening.end..];
        let withheld = (1..=closing.len())
            .rev()
            .find(|&length| body.as_bytes().ends_with(&closing.as_bytes()[..length]))
            .unwrap_or(0);
        let end = self.raw.len() - withheld;
        let start = self.emittedBodyEnd.max(opening.end);
        if end <= start {
            return String::new();
        }
        self.emittedBodyEnd = end;
        self.raw[start..end].to_string()
    }

    /// Marks the XML block closed at the boundary emitted by StreamXmlPlugin.
    fn close(&mut self) {
        self.isClosed = true;
    }

    /// Converts the accumulated XML metadata into a transport event.
    fn event(&self) -> MarkdownXmlStreamEvent {
        if self.isClosed {
            if let Some(node) = NativeXmlSplitter::parse_complete_node(&self.raw) {
                return xmlEventFromNode(node);
            }
            // A stream boundary is not proof of well-formed XML. Keep the
            // existing XML node in its incomplete state instead of panicking.
        }
        MarkdownXmlStreamEvent {
            tagName: self
                .opening
                .as_ref()
                .map(|opening| opening.tag_name.clone())
                .or_else(|| ChatMarkupRegex::extract_opening_tag_name(&self.raw)),
            attributes: self
                .opening
                .as_ref()
                .map(|opening| opening.attributes.clone()),
            bodyChunk: None,
            children: Vec::new(),
            isClosed: Some(false),
        }
    }
}

/// Converts one fully parsed XML node into its renderer event representation.
fn xmlEventFromNode(node: XmlNode) -> MarkdownXmlStreamEvent {
    MarkdownXmlStreamEvent {
        tagName: Some(node.tag_name),
        attributes: Some(node.attributes),
        bodyChunk: Some(node.body),
        children: node
            .children
            .into_iter()
            .enumerate()
            .map(|(index, child)| MarkdownXmlChildStreamEvent {
                index,
                tagName: Some(child.tag_name),
                attributes: Some(child.attributes),
                bodyChunk: Some(child.body),
                isClosed: Some(true),
            })
            .collect(),
        isClosed: Some(true),
    }
}

struct ActiveInline {
    id: u64,
    nodeType: Option<MarkdownProcessorType>,
}

struct MarkdownGroupSession {
    session: MarkdownSession,
    content: String,
    charByteOffsets: Vec<usize>,
    activeType: Option<Option<MarkdownProcessorType>>,
}

impl MarkdownStreamEvent {
    /// Nested surfaces complete independently; only this event closes transport.
    pub fn isRootCompletion(&self) -> bool {
        self.eventType == "completed" && self.parentBlockId.is_none()
    }

    /// Creates the boundary event that starts one self-contained Markdown snapshot.
    pub fn reset(chatId: String, parentBlockId: Option<u64>) -> Self {
        Self {
            chatId,
            eventType: "reset".to_string(),
            value: None,
            id: None,
            blockId: None,
            inlineId: None,
            parentBlockId,
            nodeType: None,
            headerLevel: None,
            xml: None,
        }
    }

    /// Creates one named renderer savepoint event.
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
            xml: None,
        }
    }

    /// Creates one named renderer rollback event.
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
            xml: None,
        }
    }
}

impl MarkdownRenderEventStream {
    pub fn new(chatId: String) -> Self {
        Self {
            chatId,
            parentBlockId: None,
            block: MarkdownGroupSession::block(),
            nextBlockId: 0,
            activeBlock: None,
        }
    }

    /// Creates a nested stream that continues the entire tree's block ID sequence.
    fn child(chatId: String, parentBlockId: u64, nextBlockId: u64) -> Self {
        Self {
            chatId,
            parentBlockId: Some(parentBlockId),
            block: MarkdownGroupSession::block(),
            nextBlockId,
            activeBlock: None,
        }
    }

    pub fn fromContent(content: String) -> Vec<MarkdownStreamEvent> {
        let mut stream = Self::new(String::new());
        let mut events = stream.pushChunk(&content);
        events.extend(stream.finish());
        events
    }

    /// Starts a self-contained snapshot and retains its parser state for later chunks.
    pub fn beginSnapshot(&mut self, content: &str) -> Vec<MarkdownStreamEvent> {
        self.resetParser();
        let mut events = vec![MarkdownStreamEvent::reset(
            self.chatId.clone(),
            self.parentBlockId,
        )];
        if !content.is_empty() {
            events.extend(self.pushChunk(content));
        }
        events
    }

    /// Restores the parser state for the exact content retained after a rollback.
    pub fn restoreContent(&mut self, content: &str) {
        self.resetParser();
        let _ = self.pushChunk(content);
    }

    /// Resets parser sessions while preserving this stream's identity and nesting.
    fn resetParser(&mut self) {
        let chatId = self.chatId.clone();
        let parentBlockId = self.parentBlockId;
        *self = Self::new(chatId);
        self.parentBlockId = parentBlockId;
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
            xml: None,
        }];

        let segments = self.block.push(chunk);
        events.extend(self.applySegments(segments));
        events
    }

    fn applySegments(&mut self, segments: Vec<Segment>) -> Vec<MarkdownStreamEvent> {
        let mut events = Vec::new();
        for segment in segments {
            if segment.r#type < 0 {
                events.extend(self.finishInline());
                if let Some(activeBlock) = self.activeBlock.as_mut() {
                    if let Some(xml) = activeBlock.xml.as_mut() {
                        xml.close();
                        events.push(MarkdownStreamEvent {
                            chatId: self.chatId.clone(),
                            eventType: "markdownBlockEnd".to_string(),
                            value: None,
                            id: None,
                            blockId: Some(activeBlock.id),
                            inlineId: None,
                            parentBlockId: self.parentBlockId,
                            nodeType: Some("XmlBlock".to_string()),
                            headerLevel: None,
                            xml: Some(xml.event()),
                        });
                    }
                    if let Some(child) = activeBlock.xmlMarkdown.as_mut() {
                        events.extend(child.finish());
                        self.nextBlockId = child.nextBlockId;
                    }
                }
                self.block.activeType = None;
                self.activeBlock = None;
                continue;
            }
            let nodeType = markdownTypeFromSegment(&segment);
            let nodeContent = markdownSegmentContent(&self.block, &segment, nodeType);
            if nodeContent.is_empty() {
                continue;
            }

            if self.block.activeType != Some(nodeType) {
                events.extend(self.finishInline());
                self.nextBlockId += 1;
                self.block.activeType = Some(nodeType);
                self.activeBlock = Some(ActiveBlock {
                    id: self.nextBlockId,
                    inline: if isInlineContainer(nodeType) {
                        Some(MarkdownInlineStream::default())
                    } else {
                        None
                    },
                    prefix: BlockPrefix::new(nodeType),
                    styleIds: Vec::new(),
                    xml: if nodeType == Some(MarkdownProcessorType::XmlBlock) {
                        Some(XmlBlockMetadata::new())
                    } else {
                        None
                    },
                    xmlMarkdown: None,
                    nextInlineId: 0,
                    activeInline: None,
                });
                let xml = self
                    .activeBlock
                    .as_ref()
                    .and_then(|block| block.xml.as_ref().map(XmlBlockMetadata::event));
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
                    xml,
                });
            }

            if nodeType == Some(MarkdownProcessorType::BlockQuote) {
                let block = self.activeBlock.as_mut().expect("active quote block");
                if block.xmlMarkdown.is_none() {
                    block.xmlMarkdown = Some(Box::new(Self::child(
                        self.chatId.clone(),
                        block.id,
                        self.nextBlockId,
                    )));
                }
                events.push(inlineEvent(
                    &self.chatId,
                    self.parentBlockId,
                    block.id,
                    None,
                    "markdownBlockChunk",
                    nodeType,
                    Some(nodeContent.clone()),
                ));
                let child = block.xmlMarkdown.as_mut().unwrap();
                events.extend(child.pushChunk(&nodeContent));
                self.nextBlockId = child.nextBlockId;
            } else if isInlineContainer(nodeType) {
                events.extend(self.inlineChunk(nodeContent));
            } else if let Some(blockId) = self.activeBlock.as_ref().map(|block| block.id) {
                let xml = if nodeType == Some(MarkdownProcessorType::XmlBlock) {
                    let block = self.activeBlock.as_mut().expect("active XML block");
                    let xml = block.xml.as_mut().expect("XML block metadata");
                    xml.append(&nodeContent);
                    let metadata = xml.event();
                    if matches!(
                        metadata.tagName.as_deref(),
                        Some("think") | Some("thinking")
                    ) {
                        if block.xmlMarkdown.is_none() {
                            block.xmlMarkdown = Some(Box::new(MarkdownRenderEventStream::child(
                                self.chatId.clone(),
                                block.id,
                                self.nextBlockId,
                            )));
                        }
                        let bodyChunk = xml.takeBodyChunk();
                        if let Some(child) = block.xmlMarkdown.as_mut() {
                            if !bodyChunk.is_empty() {
                                events.extend(child.pushChunk(&bodyChunk));
                                // Flattened child events must never reuse another level's block IDs.
                                self.nextBlockId = child.nextBlockId;
                            }
                        }
                    }
                    Some(metadata)
                } else {
                    None
                };
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
                    xml,
                });
            }
        }

        events
    }

    fn completed(&self) -> MarkdownStreamEvent {
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
            xml: None,
        }
    }

    /// Finishes pending syntax before publishing the terminal event.
    pub fn finish(&mut self) -> Vec<MarkdownStreamEvent> {
        let segments = self.block.session.finish();
        let mut events = self.applySegments(segments);
        events.extend(self.finishInline());
        if let Some(child) = self
            .activeBlock
            .as_mut()
            .and_then(|block| block.xmlMarkdown.as_mut())
        {
            events.extend(child.finish());
            self.nextBlockId = child.nextBlockId;
        }
        events.push(self.completed());
        events
    }

    fn inlineChunk(&mut self, content: String) -> Vec<MarkdownStreamEvent> {
        let Some(block) = self.activeBlock.as_mut() else {
            return Vec::new();
        };
        let mut events = Vec::new();
        let content = if let Some(prefix) = block.prefix.as_mut() {
            let (sourcePrefix, body) = prefix.push(&content);
            if let Some(sourcePrefix) = sourcePrefix {
                events.push(inlineEvent(
                    &self.chatId,
                    self.parentBlockId,
                    block.id,
                    None,
                    "markdownBlockPrefix",
                    None,
                    Some(sourcePrefix),
                ));
            }
            body
        } else {
            content
        };
        let tokens = block
            .inline
            .as_mut()
            .map(|inline| inline.push(&content))
            .unwrap_or_default();
        appendInlineTokens(&self.chatId, self.parentBlockId, block, tokens, &mut events);
        events
    }

    fn finishInline(&mut self) -> Vec<MarkdownStreamEvent> {
        let Some(block) = self.activeBlock.as_mut() else {
            return Vec::new();
        };
        let mut events = Vec::new();
        if let Some(prefix) = block.prefix.as_mut() {
            if prefix.kind == MarkdownProcessorType::FootnoteDefinition && !prefix.done {
                let label = prefix
                    .raw
                    .trim()
                    .strip_prefix("[^")
                    .and_then(|raw| raw.strip_suffix("]:"));
                if let Some(label) = label {
                    events.push(inlineEvent(
                        &self.chatId,
                        self.parentBlockId,
                        block.id,
                        None,
                        "markdownBlockPrefix",
                        None,
                        Some(label.to_string()),
                    ));
                }
            }
        }
        let tokens = block
            .inline
            .as_mut()
            .map(MarkdownInlineStream::finish)
            .unwrap_or_default();
        appendInlineTokens(&self.chatId, self.parentBlockId, block, tokens, &mut events);
        events
    }
}

fn inlineEvent(
    chatId: &str,
    parentBlockId: Option<u64>,
    blockId: u64,
    inlineId: Option<u64>,
    eventType: &str,
    nodeType: Option<MarkdownProcessorType>,
    value: Option<String>,
) -> MarkdownStreamEvent {
    MarkdownStreamEvent {
        chatId: chatId.to_string(),
        parentBlockId,
        blockId: Some(blockId),
        inlineId,
        eventType: eventType.to_string(),
        nodeType: markdownTypeLabel(nodeType).map(str::to_string),
        value,
        id: None,
        headerLevel: None,
        xml: None,
    }
}

fn inlineType(kind: InlineKind) -> Option<MarkdownProcessorType> {
    if kind == InlineKind::Text {
        None
    } else {
        Some(kind.processor_type())
    }
}

fn appendInlineTokens(
    chatId: &str,
    parent: Option<u64>,
    block: &mut ActiveBlock,
    tokens: Vec<InlineToken>,
    events: &mut Vec<MarkdownStreamEvent>,
) {
    for token in tokens {
        match token {
            InlineToken::Open(kind, parameter) => {
                block.activeInline = None;
                block.nextInlineId += 1;
                block.styleIds.push((block.nextInlineId, kind));
                events.push(inlineEvent(
                    chatId,
                    parent,
                    block.id,
                    Some(block.nextInlineId),
                    "markdownInlineStyleStart",
                    inlineType(kind),
                    parameter,
                ));
            }
            InlineToken::Close => {
                block.activeInline = None;
                if let Some((id, kind)) = block.styleIds.pop() {
                    events.push(inlineEvent(
                        chatId,
                        parent,
                        block.id,
                        Some(id),
                        "markdownInlineStyleEnd",
                        inlineType(kind),
                        None,
                    ));
                }
            }
            InlineToken::Text(kind, text) => {
                let nodeType = inlineType(kind);
                if block
                    .activeInline
                    .as_ref()
                    .is_none_or(|inline| inline.nodeType != nodeType)
                {
                    block.nextInlineId += 1;
                    block.activeInline = Some(ActiveInline {
                        id: block.nextInlineId,
                        nodeType,
                    });
                    events.push(inlineEvent(
                        chatId,
                        parent,
                        block.id,
                        Some(block.nextInlineId),
                        "markdownInlineStart",
                        nodeType,
                        None,
                    ));
                }
                let id = block.activeInline.as_ref().unwrap().id;
                events.push(inlineEvent(
                    chatId,
                    parent,
                    block.id,
                    Some(id),
                    "markdownInlineChunk",
                    nodeType,
                    Some(text),
                ));
            }
        }
    }
}

/// One prefix scanner per heading/list block. Prefixes can straddle transport
/// chunks; the body is sent to the inline machine only after the prefix ends.
enum PrefixPhase {
    Indent,
    Marker,
    Padding,
}

struct BlockPrefix {
    kind: MarkdownProcessorType,
    raw: String,
    done: bool,
    phase: PrefixPhase,
}
impl BlockPrefix {
    fn new(kind: Option<MarkdownProcessorType>) -> Option<Self> {
        kind.filter(|kind| {
            matches!(
                kind,
                MarkdownProcessorType::Header
                    | MarkdownProcessorType::OrderedList
                    | MarkdownProcessorType::UnorderedList
                    | MarkdownProcessorType::FootnoteDefinition
            )
        })
        .map(|kind| Self {
            kind,
            raw: String::new(),
            done: false,
            phase: PrefixPhase::Indent,
        })
    }
    fn push(&mut self, content: &str) -> (Option<String>, String) {
        if self.done {
            return (None, content.to_string());
        }
        let mut end = 0;
        for (index, ch) in content.char_indices() {
            let whitespace = ch == ' ' || ch == '\t';
            let prefix = match self.phase {
                _ if self.kind == MarkdownProcessorType::FootnoteDefinition => match self.phase {
                    PrefixPhase::Indent => {
                        if ch == ']' {
                            self.phase = PrefixPhase::Marker;
                        }
                        true
                    }
                    PrefixPhase::Marker => {
                        self.phase = PrefixPhase::Padding;
                        true
                    }
                    PrefixPhase::Padding => whitespace,
                },
                PrefixPhase::Indent if whitespace => true,
                PrefixPhase::Indent => {
                    self.phase = if self.kind == MarkdownProcessorType::UnorderedList {
                        PrefixPhase::Padding
                    } else {
                        PrefixPhase::Marker
                    };
                    match self.kind {
                        MarkdownProcessorType::Header => ch == '#',
                        MarkdownProcessorType::OrderedList => ch.is_ascii_digit(),
                        MarkdownProcessorType::UnorderedList => matches!(ch, '-' | '+' | '*'),
                        _ => false,
                    }
                }
                PrefixPhase::Marker if whitespace => {
                    self.phase = PrefixPhase::Padding;
                    true
                }
                PrefixPhase::Marker if self.kind == MarkdownProcessorType::Header => ch == '#',
                PrefixPhase::Marker if ch == '.' || ch == ')' => {
                    self.phase = PrefixPhase::Padding;
                    true
                }
                PrefixPhase::Marker => ch.is_ascii_digit(),
                PrefixPhase::Padding if whitespace => {
                    self.phase = PrefixPhase::Padding;
                    true
                }
                _ => false,
            };
            if !prefix {
                self.done = true;
                return (
                    Some(if self.kind == MarkdownProcessorType::FootnoteDefinition {
                        self.raw
                            .trim()
                            .strip_prefix("[^")
                            .and_then(|raw| raw.strip_suffix("]:"))
                            .unwrap_or_default()
                            .to_string()
                    } else {
                        std::mem::take(&mut self.raw)
                    }),
                    content[index..].to_string(),
                );
            }
            self.raw.push(ch);
            end = index + ch.len_utf8();
        }
        (None, content[end..].to_string())
    }
}

impl MarkdownGroupSession {
    /// Creates a block parser with a byte boundary for the empty input.
    fn block() -> Self {
        Self {
            session: NativeMarkdownSplitter::create_block_session(),
            content: String::new(),
            charByteOffsets: vec![0],
            activeType: None,
        }
    }

    /// Appends input and indexes new character boundaries for constant-time slicing.
    fn push(&mut self, chunk: &str) -> Vec<Segment> {
        let baseByteOffset = self.content.len();
        self.content.push_str(chunk);
        self.charByteOffsets.extend(
            chunk
                .char_indices()
                .map(|(offset, ch)| baseByteOffset + offset + ch.len_utf8()),
        );
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
        19 => MarkdownProcessorType::FootnoteReference,
        20 => MarkdownProcessorType::FootnoteDefinition,
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
        Some(MarkdownProcessorType::FootnoteReference) => Some("FootnoteReference"),
        Some(MarkdownProcessorType::FootnoteDefinition) => Some("FootnoteDefinition"),
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

/// Extracts a parser segment through its indexed UTF-8 character boundaries.
fn markdownSegmentContent(
    group: &MarkdownGroupSession,
    segment: &Segment,
    nodeType: Option<MarkdownProcessorType>,
) -> String {
    if nodeType == Some(MarkdownProcessorType::HtmlBreak) {
        "\n".to_string()
    } else {
        let start = group.charByteOffsets[segment.start];
        let end = group.charByteOffsets[segment.end];
        group.content[start..end].to_string()
    }
}

fn isInlineContainer(nodeType: Option<MarkdownProcessorType>) -> bool {
    !matches!(
        nodeType,
        Some(MarkdownProcessorType::CodeBlock)
            | Some(MarkdownProcessorType::BlockQuote)
            | Some(MarkdownProcessorType::BlockLatex)
            | Some(MarkdownProcessorType::Table)
            | Some(MarkdownProcessorType::XmlBlock)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both Markdown strong-emphasis delimiters must produce the same semantic
    /// nodes regardless of where the transport splits their delimiters.
    #[test]
    fn strong_emphasis_uses_bold_nodes_at_every_stream_split() {
        for content in ["**粗体** tail", "__粗体__ tail"] {
            for boundary in content
                .char_indices()
                .map(|(index, _)| index)
                .chain(std::iter::once(content.len()))
            {
                let mut stream = MarkdownRenderEventStream::new("test".to_string());
                let mut events = stream.pushChunk(&content[..boundary]);
                events.extend(stream.pushChunk(&content[boundary..]));
                let bold = styled_chars(&events)
                    .into_iter()
                    .filter(|(_, styles)| styles.iter().any(|kind| kind == "Bold"))
                    .map(|(ch, _)| ch)
                    .collect::<String>();
                assert_eq!(bold, "粗体", "content={content}, boundary={boundary}");
                assert!(!events
                    .iter()
                    .any(|event| event.nodeType.as_deref() == Some("Underline")));
            }
        }
    }

    /// Unordered markers are consumed by Core; the UI must use the node type
    /// and preserve any additional marker-like characters in the body.
    #[test]
    fn unordered_list_events_contain_bodies_without_markers() {
        for marker in ["-", "*", "+"] {
            let content = format!("{marker} **粗体**\n{marker} - literal\n");
            for boundary in content
                .char_indices()
                .map(|(index, _)| index)
                .chain(std::iter::once(content.len()))
            {
                let mut stream = MarkdownRenderEventStream::new("test".to_string());
                let mut events = stream.pushChunk(&content[..boundary]);
                events.extend(stream.pushChunk(&content[boundary..]));
                let blocks: Vec<_> = events
                    .iter()
                    .filter(|event| {
                        event.eventType == "markdownBlockStart"
                            && event.nodeType.as_deref() == Some("UnorderedList")
                    })
                    .collect();
                assert_eq!(blocks.len(), 2, "{content}, boundary={boundary}");
                for (block, expected) in blocks.iter().zip(["粗体\n", "- literal\n"]) {
                    let body = events
                        .iter()
                        .filter(|event| {
                            event.eventType == "markdownInlineChunk"
                                && event.blockId == block.blockId
                        })
                        .filter_map(|event| event.value.as_deref())
                        .collect::<String>();
                    assert_eq!(body, expected, "{content}, boundary={boundary}");
                }
            }
        }
    }

    #[test]
    fn footnotes_preserve_semantics_at_every_stream_split() {
        let content = "正文[^1]和**加粗[^note]**[^1]。\n\n[^1]: 第一条**脚注**\n[^note]: 第二条注释\n    续行内容\n\n结束";
        let expected = MarkdownRenderEventStream::fromContent(content.to_string());
        for boundary in content
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(content.len()))
        {
            let mut stream = MarkdownRenderEventStream::new(String::new());
            let mut events = stream.pushChunk(&content[..boundary]);
            events.extend(stream.pushChunk(&content[boundary..]));
            events.extend(stream.finish());
            assert_eq!(
                styled_chars(&events),
                styled_chars(&expected),
                "boundary={boundary}"
            );
            let labels: Vec<_> = events
                .iter()
                .filter(|event| event.eventType == "markdownBlockPrefix")
                .filter_map(|event| event.value.as_deref())
                .collect();
            assert_eq!(labels, ["1", "note"], "boundary={boundary}");
            let definitions: Vec<_> = events
                .iter()
                .filter(|event| {
                    event.eventType == "markdownBlockStart"
                        && event.nodeType.as_deref() == Some("FootnoteDefinition")
                })
                .collect();
            assert_eq!(definitions.len(), 2, "boundary={boundary}");
            let second_body: String = events
                .iter()
                .filter(|event| {
                    event.eventType == "markdownInlineChunk"
                        && event.blockId == definitions[1].blockId
                })
                .filter_map(|event| event.value.as_deref())
                .collect();
            assert_eq!(
                second_body.trim(),
                "第二条注释\n    续行内容",
                "boundary={boundary}"
            );
            let references = events
                .iter()
                .filter(|event| {
                    event.eventType == "markdownInlineStyleStart"
                        && event.nodeType.as_deref() == Some("FootnoteReference")
                })
                .count();
            assert_eq!(references, 3, "boundary={boundary}");
        }
    }

    #[test]
    fn footnotes_handle_empty_definitions_and_literal_syntax() {
        for source in ["[^empty]:", "[^empty]:   ", "  [^empty]:\n"] {
            let events = MarkdownRenderEventStream::fromContent(source.to_string());
            assert!(
                events
                    .iter()
                    .any(|event| event.eventType == "markdownBlockPrefix"
                        && event.value.as_deref() == Some("empty")),
                "{source:?}"
            );
        }
        for source in [
            r"`[^code]` \[^escaped] [^] [^two words]",
            "```\n[^code]: literal\n```",
            "[^broken",
            "[^label] ordinary",
        ] {
            let events = MarkdownRenderEventStream::fromContent(source.to_string());
            assert!(!events
                .iter()
                .any(|event| event.nodeType.as_deref() == Some("FootnoteDefinition")));
        }
        let events = MarkdownRenderEventStream::fromContent(
            r"`[^code]` \[^escaped] [^] [^two words]".to_string(),
        );
        assert!(!events
            .iter()
            .any(|event| event.nodeType.as_deref() == Some("FootnoteReference")));
        let source = r"escaped \[^escaped] and \[x^2\]";
        let events = MarkdownRenderEventStream::fromContent(source.to_string());
        assert!(styled_chars(&events)
            .iter()
            .map(|(ch, _)| ch)
            .collect::<String>()
            .contains("[^escaped]"));
        assert!(events
            .iter()
            .any(|event| event.nodeType.as_deref() == Some("BlockLatex")));
        assert!(!events
            .iter()
            .any(|event| event.nodeType.as_deref() == Some("FootnoteReference")));
        let events = MarkdownRenderEventStream::fromContent("[^a][^b]".to_string());
        assert_eq!(
            events
                .iter()
                .filter(|event| event.eventType == "markdownInlineStyleStart"
                    && event.nodeType.as_deref() == Some("FootnoteReference"))
                .count(),
            2
        );
    }

    fn styled_chars(events: &[MarkdownStreamEvent]) -> Vec<(char, Vec<String>)> {
        let mut styles: Vec<String> = Vec::new();
        let mut out = Vec::new();
        let mut leaf_type = None;
        for event in events.iter().filter(|event| event.parentBlockId.is_none()) {
            match event.eventType.as_str() {
                "markdownInlineStyleStart" => styles.push(event.nodeType.clone().unwrap()),
                "markdownInlineStyleEnd" => {
                    styles.pop().expect("balanced scope");
                }
                "markdownInlineStart" => leaf_type = event.nodeType.clone(),
                "markdownInlineChunk" => {
                    let mut active = styles.clone();
                    if let Some(kind) = leaf_type.as_ref() {
                        active.push(kind.clone());
                    }
                    out.extend(
                        event
                            .value
                            .as_ref()
                            .unwrap()
                            .chars()
                            .map(|ch| (ch, active.clone())),
                    );
                }
                _ => {}
            }
        }
        assert!(styles.is_empty(), "all style scopes must be closed at EOF");
        out
    }

    #[test]
    fn semantic_inline_events_are_independent_of_chunk_boundaries() {
        for content in [
            "**粗体 *斜体* `代码` [链接](https://example.com/a_(b))** tail",
            "***粗斜体*** and __下划线形式粗体__ and ~~删除线~~",
            r"escaped \*literal\* and file_name and ``a ` b`` and [missing](url",
            "visit https://example.com/a_(b). then $x+y$ and \\(x^2\\)",
            "# **标题**\n- **第一项**\n  + __第二项__\n  3) 第三项\n---\n***正文***",
            "*a**b**c* and _foo_bar_",
            "> **quote**\n> - list\n> > nested\n\nnext",
            "plain text ending with **",
            "plain [unfinished",
            "- - literal",
            "## ",
        ] {
            let expected = MarkdownRenderEventStream::fromContent(content.to_string());
            let expected_chars = styled_chars(&expected);
            for boundary in content
                .char_indices()
                .map(|(index, _)| index)
                .chain(std::iter::once(content.len()))
            {
                let mut stream = MarkdownRenderEventStream::new("test".to_string());
                let mut actual = stream.pushChunk(&content[..boundary]);
                actual.extend(stream.pushChunk(&content[boundary..]));
                actual.extend(stream.finish());
                assert_eq!(
                    styled_chars(&actual),
                    expected_chars,
                    "content={content}, boundary={boundary}"
                );
            }
            let mut stream = MarkdownRenderEventStream::new("test".to_string());
            let mut actual = Vec::new();
            for ch in content.chars() {
                actual.extend(stream.pushChunk(&ch.to_string()));
            }
            actual.extend(stream.finish());
            assert_eq!(
                styled_chars(&actual),
                expected_chars,
                "one-char chunks: {content}"
            );
        }
    }

    #[test]
    fn list_prefixes_are_semantic_and_indentation_is_preserved() {
        let events = MarkdownRenderEventStream::fromContent(
            "- first\n  + **second**\n  3) third\n".to_string(),
        );
        let prefixes: Vec<_> = events
            .iter()
            .filter(|event| event.eventType == "markdownBlockPrefix")
            .filter_map(|event| event.value.as_deref())
            .collect();
        assert_eq!(prefixes, ["- ", "  + ", "  3) "]);
        let text: String = events
            .iter()
            .filter(|event| event.eventType == "markdownInlineChunk")
            .filter_map(|event| event.value.as_deref())
            .collect();
        assert_eq!(text, "first\nsecond\nthird\n");
    }

    #[test]
    fn triple_emphasis_is_not_a_thematic_break() {
        let events = MarkdownRenderEventStream::fromContent("***粗斜***\n---\nend".to_string());
        let rules = events
            .iter()
            .filter(|event| {
                event.eventType == "markdownBlockStart"
                    && event.nodeType.as_deref() == Some("HorizontalRule")
            })
            .count();
        assert_eq!(rules, 1);
        let text = styled_chars(&events);
        for ch in "粗斜".chars() {
            let styles = &text.iter().find(|(found, _)| *found == ch).unwrap().1;
            assert_eq!(styles, &["Bold", "Italic"]);
        }
    }

    /// Keeps character-indexed segments correct across UTF-8 chunks and parser types.
    #[test]
    fn slices_multibyte_segments_across_chunk_boundaries() {
        for mut group in [MarkdownGroupSession::block()] {
            for chunk in ["a🌙", "汉", "e\u{301}", "**粗**"] {
                let _ = group.push(chunk);
            }

            for (start, end, expected) in [
                (0, 2, "a🌙"),
                (1, 3, "🌙汉"),
                (3, 5, "e\u{301}"),
                (5, 8, "**粗"),
                (7, 10, "粗**"),
            ] {
                let segment = Segment {
                    r#type: 17,
                    start,
                    end,
                };
                assert_eq!(markdownSegmentContent(&group, &segment, None), expected);
            }
        }
    }

    /// Malformed XML retains its original block and unfinished metadata.
    #[test]
    fn malformed_xml_remains_an_incomplete_xml_block() {
        for content in [
            r#"<tool name="read_file>图片</tool>"#,
            r#"<tool name="unterminated>图片</tool>"#,
        ] {
            let events = MarkdownRenderEventStream::fromContent(content.to_string());
            assert_eq!(events.last().unwrap().eventType, "completed");
            let end = events
                .iter()
                .find(|event| event.eventType == "markdownBlockEnd")
                .unwrap();
            assert_eq!(end.nodeType.as_deref(), Some("XmlBlock"));
            let xml = end.xml.as_ref().unwrap();
            assert_eq!(xml.tagName.as_deref(), Some("tool"));
            assert_eq!(xml.isClosed, Some(false));
            let raw = events
                .iter()
                .filter(|event| {
                    event.eventType == "markdownBlockChunk" && event.parentBlockId.is_none()
                })
                .filter_map(|event| event.value.as_deref())
                .collect::<String>();
            assert_eq!(raw, content);
        }
    }

    /// Flattened nested streams must use unique IDs at every input boundary.
    #[test]
    fn nested_thinking_uses_unique_block_ids_at_every_stream_split() {
        let content = "<think>outer:<thinking>middle:<think>inner";
        for boundary in content
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(content.len()))
        {
            let mut stream = MarkdownRenderEventStream::new("chat".to_string());
            let mut events = stream.pushChunk(&content[..boundary]);
            events.extend(stream.pushChunk(&content[boundary..]));
            assert_unique_nested_block_ids(&events);
        }
        let mut stream = MarkdownRenderEventStream::new("chat".to_string());
        let mut events = Vec::new();
        for ch in content.chars() {
            events.extend(stream.pushChunk(&ch.to_string()));
        }
        assert_unique_nested_block_ids(&events);
    }

    /// Sibling XML streams must continue the sequence after nested body blocks.
    #[test]
    fn sibling_thinking_streams_do_not_reuse_descendant_ids() {
        let mut stream = MarkdownRenderEventStream::new("chat".to_string());
        let mut events = Vec::new();
        for chunk in [
            "<think>",
            "first",
            "</think>",
            "<thinking>",
            "second",
            "</thinking>",
        ] {
            events.extend(stream.pushChunk(chunk));
        }
        let mut ids = std::collections::HashSet::new();
        let starts: Vec<_> = events
            .iter()
            .filter(|event| event.eventType == "markdownBlockStart")
            .collect();
        assert!(starts.len() >= 4);
        for event in starts {
            assert!(
                ids.insert(event.blockId.unwrap()),
                "duplicate block ID: {event:?}"
            );
        }
    }

    /// Checks that every nested parent has already been declared as an XML block.
    fn assert_unique_nested_block_ids(events: &[MarkdownStreamEvent]) {
        let mut ids = std::collections::HashSet::new();
        let mut xml_ids = std::collections::HashSet::new();
        for event in events {
            if let Some(parent) = event.parentBlockId {
                assert!(xml_ids.contains(&parent), "unknown XML parent: {event:?}");
            }
            if event.eventType == "markdownBlockStart" {
                let id = event.blockId.unwrap();
                assert!(ids.insert(id), "duplicate block ID: {event:?}");
                if event.nodeType.as_deref() == Some("XmlBlock") {
                    xml_ids.insert(id);
                }
            }
        }
        assert_eq!(xml_ids.len(), 3);
    }

    #[test]
    fn unfinished_thinking_keeps_its_panel_and_child_content() {
        let events = MarkdownRenderEventStream::fromContent("<think>未完成的思考".to_string());
        let xml = events
            .iter()
            .filter_map(|event| event.xml.as_ref())
            .last()
            .unwrap();
        assert_eq!(xml.tagName.as_deref(), Some("think"));
        assert_eq!(xml.isClosed, Some(false));
        assert!(events.iter().any(|event| {
            event.parentBlockId.is_some()
                && event.eventType == "markdownInlineChunk"
                && event.value.as_deref() == Some("未完成的思考")
        }));
    }

    #[test]
    fn tool_inside_unfinished_thinking_keeps_its_nested_xml_block() {
        let content = r#"<think>排查中：<tool name="read_file">图片</tool>"#;
        let events = MarkdownRenderEventStream::fromContent(content.to_string());
        let outer = events
            .iter()
            .filter(|event| event.parentBlockId.is_none())
            .filter_map(|event| event.xml.as_ref())
            .last()
            .unwrap();
        assert_eq!(outer.tagName.as_deref(), Some("think"));
        assert_eq!(outer.isClosed, Some(false));
        let tool = events
            .iter()
            .find(|event| {
                event.parentBlockId.is_some()
                    && event.eventType == "markdownBlockEnd"
                    && event
                        .xml
                        .as_ref()
                        .is_some_and(|xml| xml.tagName.as_deref() == Some("tool"))
            })
            .unwrap();
        assert_eq!(tool.xml.as_ref().unwrap().isClosed, Some(true));
        assert_eq!(
            tool.xml.as_ref().unwrap().bodyChunk.as_deref(),
            Some("图片")
        );
    }

    /// Thinking body extraction must not lose partial nested or closing tags.
    #[test]
    fn thinking_preserves_nested_xml_at_every_stream_split() {
        let body = r#"排查中：<tool name="read_file">图片 😀</tool>继续"#;
        let content = format!("<think>{body}</think>");
        let boundaries = content
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(content.len()));
        for boundary in boundaries {
            let mut stream = MarkdownRenderEventStream::new("chat".to_string());
            let mut events = stream.pushChunk(&content[..boundary]);
            events.extend(stream.pushChunk(&content[boundary..]));
            assert_nested_tool_body(&events, body);
        }

        let mut stream = MarkdownRenderEventStream::new("chat".to_string());
        let mut events = Vec::new();
        for ch in content.chars() {
            events.extend(stream.pushChunk(&ch.to_string()));
        }
        assert_nested_tool_body(&events, body);
    }

    fn assert_nested_tool_body(events: &[MarkdownStreamEvent], body: &str) {
        let child_content = events
            .iter()
            .filter(|event| event.parentBlockId.is_some() && event.eventType == "chunk")
            .filter_map(|event| event.value.as_deref())
            .collect::<String>();
        assert_eq!(child_content, body);
        assert!(events.iter().any(|event| {
            event.parentBlockId.is_some()
                && event.eventType == "markdownBlockEnd"
                && event.xml.as_ref().is_some_and(|xml| {
                    xml.tagName.as_deref() == Some("tool") && xml.isClosed == Some(true)
                })
        }));
    }

    #[test]
    fn unfinished_nested_tool_retains_both_xml_nodes() {
        let events = MarkdownRenderEventStream::fromContent(
            r#"<think>排查中：<tool name="read_file">未完成参数"#.to_string(),
        );
        for (parent, tag) in [(false, "think"), (true, "tool")] {
            let xml = events
                .iter()
                .filter(|event| event.parentBlockId.is_some() == parent)
                .filter_map(|event| event.xml.as_ref())
                .last()
                .unwrap();
            assert_eq!(xml.tagName.as_deref(), Some(tag));
            assert_eq!(xml.isClosed, Some(false));
        }
    }

    #[test]
    fn malformed_tool_inside_thinking_does_not_panic() {
        let content = r#"<think>排查中：<tool name="read_file>图片</tool></think>"#;
        let mut stream = MarkdownRenderEventStream::new("chat".to_string());
        let mut events = Vec::new();
        for ch in content.chars() {
            events.extend(stream.pushChunk(&ch.to_string()));
        }
        let tool = events
            .iter()
            .find(|event| {
                event.parentBlockId.is_some()
                    && event.eventType == "markdownBlockEnd"
                    && event
                        .xml
                        .as_ref()
                        .is_some_and(|xml| xml.tagName.as_deref() == Some("tool"))
            })
            .unwrap();
        assert_eq!(tool.xml.as_ref().unwrap().isClosed, Some(false));
    }

    #[test]
    fn closed_tool_keeps_following_unicode_text_outside_its_block() {
        let events =
            MarkdownRenderEventStream::fromContent("<tool>文本</tool> 中文 😀".to_string());
        let end = events
            .iter()
            .find(|event| event.eventType == "markdownBlockEnd")
            .unwrap();
        assert_eq!(end.xml.as_ref().unwrap().bodyChunk.as_deref(), Some("文本"));
        assert_eq!(end.xml.as_ref().unwrap().isClosed, Some(true));
        assert!(events.iter().any(|event| {
            event.eventType == "markdownInlineChunk"
                && event
                    .value
                    .as_deref()
                    .is_some_and(|value| value.contains("中文 😀"))
        }));
    }

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

    #[test]
    fn restores_inline_state_after_a_revision_rollback() {
        let mut stream = MarkdownRenderEventStream::new("chat".to_string());

        let _ = stream.pushChunk("plain ");
        let _ = stream.pushChunk("**discarded");
        stream.restoreContent("plain ");
        let events = stream.pushChunk("**replacement");

        let inline_start_index = events
            .iter()
            .position(|event| event.eventType == "markdownInlineStart")
            .expect("replacement inline must start after the rollback");
        let inline_chunk_index = events
            .iter()
            .position(|event| event.eventType == "markdownInlineChunk")
            .expect("replacement inline must emit content after the rollback");
        assert!(inline_start_index < inline_chunk_index);
    }

    #[test]
    fn snapshot_rebuilds_dependencies_before_continuing_incrementally() {
        let mut stream = MarkdownRenderEventStream::new("chat".to_string());

        let snapshot = stream.beginSnapshot("plain ");
        let block_start_index = snapshot
            .iter()
            .position(|event| event.eventType == "markdownBlockStart")
            .expect("snapshot must start its Markdown block");
        let inline_start_index = snapshot
            .iter()
            .position(|event| event.eventType == "markdownInlineStart")
            .expect("snapshot must start its Markdown inline");
        let inline_chunk_index = snapshot
            .iter()
            .position(|event| event.eventType == "markdownInlineChunk")
            .expect("snapshot must emit its Markdown inline content");

        assert_eq!(snapshot[0].eventType, "reset");
        assert!(block_start_index < inline_start_index);
        assert!(inline_start_index < inline_chunk_index);

        let continuation = stream.pushChunk("continued");
        assert!(
            continuation
                .iter()
                .all(|event| event.eventType != "markdownBlockStart"),
            "continuation must reuse the block restored by the snapshot"
        );
    }

    /// Verifies one four-call batch emits complete ordered XML metadata.
    #[test]
    fn emits_structured_metadata_for_four_calls_and_results() {
        let content = concat!(
            r#"<tool name="daily_life:get_current_date"></tool>"#,
            r#"<tool_A1 name="daily_life:device_status"></tool_A1>"#,
            r#"<tool name="daily_life:search_weather"><param name="location">Hong Kong</param></tool>"#,
            r#"<tool_B23456 name="daily_life:search_weather"><param name="location">Shanghai</param></tool_B23456>"#,
            r#"<tool_result name="daily_life:get_current_date"><content>2026-09-11</content></tool_result>"#,
            r#"<tool_result_A1 name="daily_life:device_status"><content>ready</content></tool_result_A1>"#,
            r#"<tool_result name="daily_life:search_weather"><content>{"url":"https://x.test/?city=hk&lang=en"}</content></tool_result>"#,
            r#"<tool_result_B23456 name="daily_life:search_weather"><content>{"url":"https://x.test/?city=sh&lang=zh"}</content></tool_result_B23456>"#,
        );

        let events = MarkdownRenderEventStream::fromContent(content.to_string());
        let xml = events
            .iter()
            .filter(|event| event.eventType == "markdownBlockEnd")
            .map(|event| {
                event
                    .xml
                    .as_ref()
                    .expect("block end must carry XML metadata")
            })
            .collect::<Vec<_>>();
        let names = xml
            .iter()
            .map(|event| {
                event
                    .attributes
                    .as_ref()
                    .and_then(|attributes| attributes.get("name"))
                    .map(String::as_str)
                    .expect("tool metadata must include name")
            })
            .collect::<Vec<_>>();

        assert_eq!(xml.len(), 8);
        assert_eq!(
            names,
            vec![
                "daily_life:get_current_date",
                "daily_life:device_status",
                "daily_life:search_weather",
                "daily_life:search_weather",
                "daily_life:get_current_date",
                "daily_life:device_status",
                "daily_life:search_weather",
                "daily_life:search_weather",
            ]
        );
        assert_eq!(xml[2].children[0].bodyChunk.as_deref(), Some("Hong Kong"));
        assert_eq!(xml[3].children[0].bodyChunk.as_deref(), Some("Shanghai"));
        assert_eq!(
            xml[6].children[0].bodyChunk.as_deref(),
            Some(r#"{"url":"https://x.test/?city=hk&lang=en"}"#)
        );
    }
}
