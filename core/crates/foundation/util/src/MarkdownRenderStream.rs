#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::streamnative::NativeMarkdownSplitter::{
    MarkdownProcessorType, MarkdownSession, NativeMarkdownSplitter, Segment,
};
use crate::streamnative::NativeXmlSplitter::{NativeXmlSplitter, XmlNode, XmlOpeningTag};

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
    inline: Option<MarkdownGroupSession>,
    xml: Option<XmlBlockMetadata>,
    xmlContent: Option<MarkdownGroupSession>,
    xmlMarkdown: Option<Box<MarkdownRenderEventStream>>,
    nextInlineId: u64,
    activeInline: Option<ActiveInline>,
}

struct XmlBlockMetadata {
    raw: String,
    opening: Option<XmlOpeningTag>,
    isClosed: bool,
}

impl XmlBlockMetadata {
    /// Creates metadata storage for one XML markdown block.
    fn new() -> Self {
        Self {
            raw: String::new(),
            opening: None,
            isClosed: false,
        }
    }

    /// Appends one already-delimited XML block chunk from the Markdown stream.
    fn append(&mut self, chunk: &str) {
        self.raw.push_str(chunk);
        if self.opening.is_none() {
            self.opening = NativeXmlSplitter::parse_opening_tag(&self.raw);
        }
    }

    /// Marks the XML block closed at the boundary emitted by StreamXmlPlugin.
    fn close(&mut self) {
        self.isClosed = true;
    }

    /// Converts the accumulated XML metadata into a transport event.
    fn event(&self) -> MarkdownXmlStreamEvent {
        if self.isClosed {
            let node = NativeXmlSplitter::parse_complete_node(&self.raw)
                .expect("StreamXmlPlugin closed an invalid XML block");
            return xmlEventFromNode(node);
        }
        MarkdownXmlStreamEvent {
            tagName: self
                .opening
                .as_ref()
                .map(|opening| opening.tag_name.clone()),
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
    activeType: Option<Option<MarkdownProcessorType>>,
}

impl MarkdownStreamEvent {
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

    /// Creates a nested Markdown stream for the body of one XML block.
    fn child(chatId: String, parentBlockId: u64) -> Self {
        Self {
            chatId,
            parentBlockId: Some(parentBlockId),
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
        for segment in segments {
            if segment.r#type < 0 {
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
                    if let Some(child) = activeBlock.xmlMarkdown.as_ref() {
                        events.push(child.completed());
                    }
                }
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
                    xml: if nodeType == Some(MarkdownProcessorType::XmlBlock) {
                        Some(XmlBlockMetadata::new())
                    } else {
                        None
                    },
                    xmlContent: if nodeType == Some(MarkdownProcessorType::XmlBlock) {
                        Some(MarkdownGroupSession::xmlContent())
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

            if isInlineContainer(nodeType) {
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
                            )));
                        }
                        let bodySegments = block
                            .xmlContent
                            .as_mut()
                            .expect("XML content stream")
                            .push(&nodeContent);
                        if let Some(child) = block.xmlMarkdown.as_mut() {
                            for segment in bodySegments {
                                if segment.r#type < 0 {
                                    continue;
                                }
                                let xmlContent =
                                    block.xmlContent.as_ref().expect("XML content stream");
                                let bodyChunk = markdownSegmentContent(
                                    &xmlContent.content,
                                    &segment,
                                    markdownTypeFromSegment(&segment),
                                );
                                if !bodyChunk.is_empty() {
                                    events.extend(child.pushChunk(&bodyChunk));
                                }
                            }
                        }
                    } else {
                        let _ = block
                            .xmlContent
                            .as_mut()
                            .expect("XML content stream")
                            .push(&nodeContent);
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
            xml: None,
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
                    xml: None,
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
                    xml: None,
                });
            }
        }
        events
    }
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

    /// Creates a group session backed by the XML plugin's inner-content output.
    fn xmlContent() -> Self {
        Self {
            session: NativeMarkdownSplitter::create_xml_content_session(),
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
