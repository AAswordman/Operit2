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
    pub nodeType: Option<String>,
    pub headerLevel: Option<usize>,
}

pub struct MarkdownRenderEventStream {
    chatId: String,
    block: MarkdownGroupSession,
    nextBlockId: u64,
    activeBlock: Option<ActiveBlock>,
}

struct ActiveBlock {
    id: u64,
    inline: Option<MarkdownGroupSession>,
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
            nodeType: None,
            headerLevel: None,
        }
    }
}

impl MarkdownRenderEventStream {
    pub fn new(chatId: String) -> Self {
        Self {
            chatId,
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
                    nodeType: markdownTypeLabel(nodeType).map(ToString::to_string),
                    headerLevel: headerLevel(nodeType, &nodeContent),
                });
            }

            if isInlineContainer(nodeType) {
                events.extend(self.inlineChunk(nodeContent));
            } else if let Some(blockId) = self.activeBlock.as_ref().map(|block| block.id) {
                events.push(MarkdownStreamEvent {
                    chatId: self.chatId.clone(),
                    eventType: "markdownBlockChunk".to_string(),
                    value: Some(nodeContent),
                    id: None,
                    blockId: Some(blockId),
                    inlineId: None,
                    nodeType: markdownTypeLabel(nodeType).map(ToString::to_string),
                    headerLevel: None,
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
                    nodeType: markdownTypeLabel(activeInline.nodeType).map(ToString::to_string),
                    headerLevel: None,
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
