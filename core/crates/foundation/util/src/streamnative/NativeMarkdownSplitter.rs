use std::collections::VecDeque;

use crate::stream::plugins::StreamMarkdownPlugin::{
    StreamMarkdownBlockBracketLaTeXPlugin, StreamMarkdownBlockLaTeXPlugin,
    StreamMarkdownBlockQuotePlugin, StreamMarkdownFencedCodeBlockPlugin,
    StreamMarkdownFootnoteDefinitionPlugin, StreamMarkdownHeaderPlugin, StreamMarkdownImagePlugin,
    StreamMarkdownOrderedListPlugin, StreamMarkdownTablePlugin, StreamMarkdownUnorderedListPlugin,
};
use crate::stream::plugins::StreamPlugin::{PluginState, StreamPlugin};
use crate::stream::plugins::StreamXmlPlugin::StreamXmlPlugin;
use crate::streamnative::MarkdownInlineStream::{InlineToken, MarkdownInlineStream};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(i32)]
pub enum MarkdownProcessorType {
    Header = 0,
    BlockQuote = 1,
    CodeBlock = 2,
    OrderedList = 3,
    UnorderedList = 4,
    HorizontalRule = 5,
    BlockLatex = 6,
    Table = 7,
    XmlBlock = 8,
    Bold = 9,
    Italic = 10,
    InlineCode = 11,
    Link = 12,
    Image = 13,
    Strikethrough = 14,
    Underline = 15,
    InlineLatex = 16,
    PlainText = 17,
    HtmlBreak = 18,
    FootnoteReference = 19,
    FootnoteDefinition = 20,
}

impl MarkdownProcessorType {
    fn from_ordinal(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Header),
            1 => Some(Self::BlockQuote),
            2 => Some(Self::CodeBlock),
            3 => Some(Self::OrderedList),
            4 => Some(Self::UnorderedList),
            5 => Some(Self::HorizontalRule),
            6 => Some(Self::BlockLatex),
            7 => Some(Self::Table),
            8 => Some(Self::XmlBlock),
            9 => Some(Self::Bold),
            10 => Some(Self::Italic),
            11 => Some(Self::InlineCode),
            12 => Some(Self::Link),
            13 => Some(Self::Image),
            14 => Some(Self::Strikethrough),
            15 => Some(Self::Underline),
            16 => Some(Self::InlineLatex),
            17 => Some(Self::PlainText),
            18 => Some(Self::HtmlBreak),
            19 => Some(Self::FootnoteReference),
            20 => Some(Self::FootnoteDefinition),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MarkdownNodeStable {
    pub r#type: MarkdownProcessorType,
    pub content: String,
    pub children: Vec<MarkdownNodeStable>,
    pub link_url: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Segment {
    pub r#type: i32,
    pub start: usize,
    pub end: usize,
}

const SEG_BREAK: i32 = -1;

struct PluginEntry {
    plugin: Box<dyn StreamPlugin>,
    tag: MarkdownProcessorType,
}

#[derive(Debug, Clone, Copy)]
struct RunState {
    tag: MarkdownProcessorType,
    start: Option<usize>,
    end: usize,
}

impl Default for RunState {
    fn default() -> Self {
        Self {
            tag: MarkdownProcessorType::PlainText,
            start: None,
            end: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct WaitforPending {
    c: char,
    at_start_of_line: bool,
    global_index: usize,
    should_emit: bool,
}

#[derive(Debug, Clone, Copy)]
struct PendingChar {
    at_start_of_line: bool,
    c: char,
    global_index: usize,
}

struct RulePrefix {
    marker: Option<char>,
    count: usize,
    valid: bool,
}
impl Default for RulePrefix {
    fn default() -> Self {
        Self {
            marker: None,
            count: 0,
            valid: true,
        }
    }
}
impl RulePrefix {
    fn push(&mut self, ch: char) {
        if !self.valid || matches!(ch, ' ' | '\t' | '\r' | '\n') {
            return;
        }
        if !matches!(ch, '-' | '*' | '_') || self.marker.is_some_and(|marker| marker != ch) {
            self.valid = false;
        } else {
            self.marker = Some(ch);
            self.count += 1;
        }
    }
    fn count(&self) -> Option<usize> {
        if self.valid && self.marker.is_some() {
            Some(self.count)
        } else {
            None
        }
    }
}

pub struct MarkdownSession {
    plugins: Vec<PluginEntry>,
    global_offset: usize,
    at_start_of_line: bool,
    active_index: Option<usize>,
    active_tag: MarkdownProcessorType,
    eval_start_global: Option<usize>,
    evaluation_buffer: Vec<char>,
    rule_prefix: RulePrefix,
    evaluation_emit_mask: Vec<u32>,
    waitfor_active: bool,
    waitfor_at_start_of_line: bool,
    waitfor_pending: Vec<WaitforPending>,
    pending_chars: VecDeque<PendingChar>,
}

impl MarkdownSession {
    fn new(mut plugins: Vec<PluginEntry>) -> Self {
        for entry in &mut plugins {
            entry.plugin.init_plugin();
        }
        Self {
            plugins,
            global_offset: 0,
            at_start_of_line: true,
            active_index: None,
            active_tag: MarkdownProcessorType::PlainText,
            eval_start_global: None,
            evaluation_buffer: Vec::new(),
            rule_prefix: RulePrefix::default(),
            evaluation_emit_mask: Vec::new(),
            waitfor_active: false,
            waitfor_at_start_of_line: false,
            waitfor_pending: Vec::new(),
            pending_chars: VecDeque::new(),
        }
    }

    pub fn push(&mut self, chunk: &str) -> Vec<Segment> {
        let chars = chunk.chars().collect::<Vec<_>>();
        let mut out = Vec::with_capacity(64);
        let mut run = RunState::default();
        let mut at_start_of_line = self.at_start_of_line;
        let mut i = 0usize;

        while i < chars.len() || !self.pending_chars.is_empty() {
            let (c, forced_index, sol) = if let Some(pending) = self.pending_chars.pop_front() {
                (
                    pending.c,
                    Some(pending.global_index),
                    pending.at_start_of_line,
                )
            } else {
                let c = chars[i];
                i += 1;
                (c, None, at_start_of_line)
            };
            at_start_of_line = c == '\n';
            self.process_one(c, sol, forced_index, &mut out, &mut run);
        }

        self.at_start_of_line = at_start_of_line;
        Self::flush_run(&mut out, &mut run);
        out
    }

    pub fn finish(&mut self) -> Vec<Segment> {
        let mut out = Vec::new();
        let mut run = RunState::default();
        if let Some(start) = self.eval_start_global {
            let tag = if self.rule_prefix.count().is_some_and(|count| count >= 3) {
                MarkdownProcessorType::HorizontalRule
            } else {
                MarkdownProcessorType::PlainText
            };
            for offset in 0..self.evaluation_buffer.len() {
                Self::emit_index(&mut out, tag, start + offset, &mut run);
            }
        }
        self.clear_evaluation();
        for pending in self.waitfor_pending.drain(..) {
            Self::emit_index(&mut out, self.active_tag, pending.global_index, &mut run);
        }
        self.waitfor_active = false;
        Self::flush_run(&mut out, &mut run);
        out
    }

    fn process_one(
        &mut self,
        c: char,
        at_start_of_line: bool,
        forced_global_index: Option<usize>,
        out: &mut Vec<Segment>,
        run: &mut RunState,
    ) {
        let global_index = forced_global_index.unwrap_or(self.global_offset);
        if forced_global_index.is_none() {
            self.global_offset += 1;
        }

        if self.waitfor_active {
            self.process_waitfor(c, global_index, out, run);
            return;
        }

        if let Some(active_index) = self.active_index {
            let should_emit = self.plugins[active_index]
                .plugin
                .process_char(c, at_start_of_line);
            let state = self.plugins[active_index].plugin.state();

            if state == PluginState::WaitFor {
                self.waitfor_active = true;
                self.waitfor_at_start_of_line = c == '\n';
                self.waitfor_pending.push(WaitforPending {
                    c,
                    at_start_of_line,
                    global_index,
                    should_emit,
                });
                return;
            }

            if should_emit {
                Self::emit_index(out, self.active_tag, global_index, run);
            }

            if state != PluginState::Processing {
                Self::emit_break(out, global_index + 1, run);
                self.active_index = None;
                self.active_tag = MarkdownProcessorType::PlainText;
            }
            return;
        }

        self.process_evaluation(c, at_start_of_line, global_index, out, run);
    }

    fn process_waitfor(
        &mut self,
        c: char,
        global_index: usize,
        out: &mut Vec<Segment>,
        run: &mut RunState,
    ) {
        let Some(active_index) = self.active_index else {
            self.waitfor_active = false;
            return;
        };

        let next_should_emit = self.plugins[active_index]
            .plugin
            .process_char(c, self.waitfor_at_start_of_line);

        if self.plugins[active_index].plugin.state() == PluginState::WaitFor {
            self.waitfor_pending.push(WaitforPending {
                c,
                at_start_of_line: self.waitfor_at_start_of_line,
                global_index,
                should_emit: next_should_emit,
            });
            self.waitfor_at_start_of_line = c == '\n';
            return;
        }

        if self.plugins[active_index].plugin.state() == PluginState::Processing {
            for pending in self.waitfor_pending.drain(..) {
                if pending.should_emit {
                    Self::emit_index(out, self.active_tag, pending.global_index, run);
                }
            }
            self.waitfor_active = false;
            if next_should_emit {
                Self::emit_index(out, self.active_tag, global_index, run);
            }
            return;
        }

        let mut replay = Vec::new();
        for pending in self.waitfor_pending.drain(..) {
            if pending.c == '\n' && replay.is_empty() {
                if pending.should_emit {
                    Self::emit_index(
                        out,
                        MarkdownProcessorType::PlainText,
                        pending.global_index,
                        run,
                    );
                }
            } else {
                replay.push(PendingChar {
                    c: pending.c,
                    global_index: pending.global_index,
                    at_start_of_line: pending.at_start_of_line,
                });
            }
        }
        self.waitfor_active = false;
        Self::emit_break(out, global_index, run);
        self.active_index = None;
        self.active_tag = MarkdownProcessorType::PlainText;

        for entry in &mut self.plugins {
            entry.plugin.reset();
        }

        self.pending_chars.push_front(PendingChar {
            c,
            global_index,
            at_start_of_line: self.waitfor_at_start_of_line,
        });
        for pending in replay.into_iter().rev() {
            self.pending_chars.push_front(pending);
        }
    }

    fn process_evaluation(
        &mut self,
        c: char,
        at_start_of_line: bool,
        global_index: usize,
        out: &mut Vec<Segment>,
        run: &mut RunState,
    ) {
        if self.eval_start_global.is_none() {
            self.eval_start_global = Some(global_index);
            self.rule_prefix.valid = at_start_of_line;
        }

        self.evaluation_buffer.push(c);
        self.rule_prefix.push(c);
        let mut emit_mask = 0u32;
        let mut successful = None::<usize>;

        for (index, entry) in self.plugins.iter_mut().enumerate() {
            let should_emit = entry.plugin.process_char(c, at_start_of_line);
            if should_emit {
                emit_mask |= 1u32 << index;
            }
        }
        self.evaluation_emit_mask.push(emit_mask);

        for (index, entry) in self.plugins.iter().enumerate() {
            if entry.plugin.state() == PluginState::Processing {
                successful = Some(index);
                break;
            }
        }

        if Self::is_html_break_full_match(&self.evaluation_buffer) {
            let start = self.eval_start_global.unwrap_or(global_index);
            for index in 0..self.evaluation_buffer.len() {
                Self::emit_index(out, MarkdownProcessorType::HtmlBreak, start + index, run);
            }
            self.clear_evaluation();
            for entry in &mut self.plugins {
                entry.plugin.reset();
            }
            return;
        }

        if let Some(count) = self.rule_prefix.count() {
            if self.evaluation_buffer.last() == Some(&'\n') && count >= 3 {
                let start = self.eval_start_global.unwrap_or(global_index);
                for offset in 0..self.evaluation_buffer.len() {
                    Self::emit_index(
                        out,
                        MarkdownProcessorType::HorizontalRule,
                        start + offset,
                        run,
                    );
                }
                Self::emit_break(out, global_index + 1, run);
                self.clear_evaluation();
                for entry in &mut self.plugins {
                    entry.plugin.reset();
                }
                return;
            }
            if self.evaluation_buffer.last() != Some(&'\n') {
                return;
            }
        }

        if let Some(index) = successful {
            self.active_index = Some(index);
            self.active_tag = self.plugins[index].tag;
            Self::flush_run(out, run);

            let start = self.eval_start_global.unwrap_or(global_index);
            for (buffer_index, mask) in self.evaluation_emit_mask.iter().copied().enumerate() {
                if (mask & (1u32 << index)) != 0 {
                    Self::emit_index(out, self.active_tag, start + buffer_index, run);
                }
            }

            self.clear_evaluation();
            for (other_index, entry) in self.plugins.iter_mut().enumerate() {
                if other_index != index {
                    entry.plugin.reset();
                }
            }
            return;
        }

        if Self::is_html_break_prefix(&self.evaluation_buffer) {
            return;
        }

        let any_trying = self
            .plugins
            .iter()
            .any(|entry| entry.plugin.state() == PluginState::Trying);

        if !any_trying {
            let start = self.eval_start_global.unwrap_or(global_index);
            for index in 0..self.evaluation_buffer.len() {
                Self::emit_index(out, MarkdownProcessorType::PlainText, start + index, run);
            }
            self.clear_evaluation();
            for entry in &mut self.plugins {
                entry.plugin.reset();
            }
        }
    }

    fn clear_evaluation(&mut self) {
        self.evaluation_buffer.clear();
        self.rule_prefix = RulePrefix::default();
        self.evaluation_emit_mask.clear();
        self.eval_start_global = None;
    }

    fn emit_index(
        out: &mut Vec<Segment>,
        tag: MarkdownProcessorType,
        index: usize,
        run: &mut RunState,
    ) {
        if let Some(start) = run.start {
            if run.tag != tag || run.end != index {
                out.push(Segment {
                    r#type: run.tag as i32,
                    start,
                    end: run.end,
                });
                run.start = None;
                run.end = 0;
            }
        }

        if run.start.is_none() {
            run.tag = tag;
            run.start = Some(index);
            run.end = index + 1;
        } else {
            run.end = index + 1;
        }
    }

    fn flush_run(out: &mut Vec<Segment>, run: &mut RunState) {
        if let Some(start) = run.start.take() {
            out.push(Segment {
                r#type: run.tag as i32,
                start,
                end: run.end,
            });
        }
        run.end = 0;
    }

    fn emit_break(out: &mut Vec<Segment>, pos: usize, run: &mut RunState) {
        Self::flush_run(out, run);
        out.push(Segment {
            r#type: SEG_BREAK,
            start: pos,
            end: pos,
        });
    }

    fn is_html_break_prefix(buffer: &[char]) -> bool {
        Self::matches_case_insensitive_prefix(buffer, "<br>")
            || Self::matches_case_insensitive_prefix(buffer, "<br/>")
            || Self::matches_case_insensitive_prefix(buffer, "<br />")
            || Self::matches_case_insensitive_prefix(buffer, "<br >")
    }

    fn is_html_break_full_match(buffer: &[char]) -> bool {
        Self::matches_case_insensitive_full(buffer, "<br>")
            || Self::matches_case_insensitive_full(buffer, "<br/>")
            || Self::matches_case_insensitive_full(buffer, "<br />")
            || Self::matches_case_insensitive_full(buffer, "<br >")
    }

    fn matches_case_insensitive_prefix(buffer: &[char], pattern: &str) -> bool {
        buffer.len() <= pattern.len()
            && buffer
                .iter()
                .zip(pattern.bytes())
                .all(|(left, right)| left.eq_ignore_ascii_case(&(right as char)))
    }

    fn matches_case_insensitive_full(buffer: &[char], pattern: &str) -> bool {
        buffer.len() == pattern.len() && Self::matches_case_insensitive_prefix(buffer, pattern)
    }
}

#[derive(Debug, Clone, Default)]
pub struct NativeMarkdownSplitter;

impl NativeMarkdownSplitter {
    /// Creates the block-level Markdown parser session.
    pub fn create_block_session() -> MarkdownSession {
        MarkdownSession::new(Self::get_block_plugins())
    }

    /// Creates the inline Markdown parser session.
    pub fn create_inline_session() -> MarkdownInlineStream {
        MarkdownInlineStream::default()
    }

    /// Creates a stream session that emits only an XML block's inner content.
    pub fn create_xml_content_session() -> MarkdownSession {
        MarkdownSession::new(vec![PluginEntry {
            plugin: Box::new(StreamXmlPlugin::new(false)),
            tag: MarkdownProcessorType::XmlBlock,
        }])
    }

    fn get_block_plugins() -> Vec<PluginEntry> {
        vec![
            PluginEntry {
                plugin: Box::new(StreamMarkdownFootnoteDefinitionPlugin::default()),
                tag: MarkdownProcessorType::FootnoteDefinition,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownHeaderPlugin::new(true)),
                tag: MarkdownProcessorType::Header,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownFencedCodeBlockPlugin::new(true)),
                tag: MarkdownProcessorType::CodeBlock,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownBlockQuotePlugin::new(false)),
                tag: MarkdownProcessorType::BlockQuote,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownOrderedListPlugin::new(true)),
                tag: MarkdownProcessorType::OrderedList,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownUnorderedListPlugin::new(true)),
                tag: MarkdownProcessorType::UnorderedList,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownBlockLaTeXPlugin::new(false)),
                tag: MarkdownProcessorType::BlockLatex,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownBlockBracketLaTeXPlugin::new(true)),
                tag: MarkdownProcessorType::BlockLatex,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownTablePlugin::new(true)),
                tag: MarkdownProcessorType::Table,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownImagePlugin::new(true)),
                tag: MarkdownProcessorType::Image,
            },
            PluginEntry {
                plugin: Box::new(StreamXmlPlugin::new(true)),
                tag: MarkdownProcessorType::XmlBlock,
            },
        ]
    }

    pub fn parse_inline_to_stable_nodes(content: &str) -> Vec<MarkdownNodeStable> {
        Self::native_markdown_split_by_inline(content)
    }

    pub fn native_markdown_split_by_block(content: &str) -> Vec<MarkdownNodeStable> {
        let mut session = Self::create_block_session();
        let mut segments = session.push(content);
        segments.extend(session.finish());
        Self::segments_to_stable_nodes(content, segments, true)
    }

    pub fn native_markdown_split_by_inline(content: &str) -> Vec<MarkdownNodeStable> {
        let mut session = Self::create_inline_session();
        let mut tokens = session.push(content);
        tokens.extend(session.finish());
        let mut stack = vec![MarkdownNodeStable {
            r#type: MarkdownProcessorType::PlainText,
            content: String::new(),
            children: Vec::new(),
            link_url: None,
        }];
        for token in tokens {
            match token {
                InlineToken::Open(kind, link_url) => stack.push(MarkdownNodeStable {
                    r#type: kind.processor_type(),
                    content: String::new(),
                    children: Vec::new(),
                    link_url,
                }),
                InlineToken::Close => {
                    if stack.len() > 1 {
                        let node = stack.pop().unwrap();
                        stack.last_mut().unwrap().children.push(node);
                    }
                }
                InlineToken::Text(kind, text) => {
                    for parent in &mut stack {
                        parent.content.push_str(&text);
                    }
                    let children = &mut stack.last_mut().unwrap().children;
                    if let Some(last) = children.last_mut().filter(|last| {
                        last.r#type == kind.processor_type() && last.children.is_empty()
                    }) {
                        last.content.push_str(&text);
                    } else {
                        children.push(MarkdownNodeStable {
                            r#type: kind.processor_type(),
                            content: text,
                            children: Vec::new(),
                            link_url: None,
                        });
                    }
                }
            }
        }
        stack.pop().unwrap().children
    }

    /// Converts character-indexed parser segments into nodes in linear total slice work.
    fn segments_to_stable_nodes(
        content: &str,
        segments: Vec<Segment>,
        parse_inline_children: bool,
    ) -> Vec<MarkdownNodeStable> {
        let charByteOffsets = content
            .char_indices()
            .map(|(offset, _)| offset)
            .chain(std::iter::once(content.len()))
            .collect::<Vec<_>>();
        let mut nodes = Vec::new();
        for segment in segments {
            if segment.r#type < 0 {
                continue;
            }
            let node_type = MarkdownProcessorType::from_ordinal(segment.r#type)
                .unwrap_or(MarkdownProcessorType::PlainText);
            assert!(
                segment.start <= segment.end && segment.end < charByteOffsets.len(),
                "Markdown segment must have valid character offsets"
            );
            let node_content = if node_type == MarkdownProcessorType::HtmlBreak {
                "\n".to_string()
            } else {
                content[charByteOffsets[segment.start]..charByteOffsets[segment.end]].to_string()
            };
            if node_content.is_empty() {
                continue;
            }
            let children = if parse_inline_children && Self::is_inline_container(node_type) {
                Self::native_markdown_split_by_inline(&node_content)
            } else {
                Vec::new()
            };
            nodes.push(MarkdownNodeStable {
                r#type: node_type,
                content: node_content,
                children,
                link_url: None,
            });
        }
        nodes
    }

    fn is_inline_container(block_type: MarkdownProcessorType) -> bool {
        block_type != MarkdownProcessorType::CodeBlock
            && block_type != MarkdownProcessorType::BlockLatex
            && block_type != MarkdownProcessorType::Table
            && block_type != MarkdownProcessorType::XmlBlock
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extracts multibyte segments through character-aligned UTF-8 boundaries.
    #[test]
    fn stable_nodes_preserve_multibyte_segment_boundaries() {
        let content = "a🌙汉e\u{301}**粗**";
        let segments = vec![
            Segment {
                r#type: MarkdownProcessorType::PlainText as i32,
                start: 0,
                end: 2,
            },
            Segment {
                r#type: MarkdownProcessorType::PlainText as i32,
                start: 1,
                end: 3,
            },
            Segment {
                r#type: MarkdownProcessorType::PlainText as i32,
                start: 7,
                end: 10,
            },
        ];
        let nodes = NativeMarkdownSplitter::segments_to_stable_nodes(content, segments, false);
        assert_eq!(
            nodes
                .iter()
                .map(|node| node.content.as_str())
                .collect::<Vec<_>>(),
            vec!["a🌙", "🌙汉", "粗**"]
        );
    }
}
