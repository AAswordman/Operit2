use std::collections::VecDeque;

use crate::util::stream::Stream::{Stream, VecStream};
use crate::util::stream::StreamGroup::StreamGroup;
use crate::util::stream::plugins::StreamMarkdownPlugin::{
    StreamMarkdownBlockBracketLaTeXPlugin, StreamMarkdownBlockLaTeXPlugin,
    StreamMarkdownBlockQuotePlugin, StreamMarkdownBoldPlugin, StreamMarkdownFencedCodeBlockPlugin,
    StreamMarkdownHeaderPlugin, StreamMarkdownHorizontalRulePlugin, StreamMarkdownImagePlugin,
    StreamMarkdownInlineCodePlugin, StreamMarkdownInlineLaTeXPlugin,
    StreamMarkdownInlineParenLaTeXPlugin, StreamMarkdownItalicPlugin, StreamMarkdownLinkPlugin,
    StreamMarkdownOrderedListPlugin, StreamMarkdownStrikethroughPlugin, StreamMarkdownTablePlugin,
    StreamMarkdownUnderlinePlugin, StreamMarkdownUnorderedListPlugin,
};
use crate::util::stream::plugins::StreamPlugin::{PluginState, StreamPlugin};
use crate::util::stream::plugins::StreamXmlPlugin::StreamXmlPlugin;

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
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MarkdownNodeStable {
    pub r#type: MarkdownProcessorType,
    pub content: String,
    pub children: Vec<MarkdownNodeStable>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Segment {
    pub r#type: i32,
    pub start: usize,
    pub end: usize,
}

const SEG_BREAK: i32 = -1;

pub struct PluginEntry {
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
    global_index: usize,
    should_emit: bool,
}

#[derive(Debug, Clone, Copy)]
struct PendingChar {
    c: char,
    global_index: usize,
}

pub struct MarkdownSession {
    plugins: Vec<PluginEntry>,
    global_offset: usize,
    at_start_of_line: bool,
    active_index: Option<usize>,
    active_tag: MarkdownProcessorType,
    eval_start_global: Option<usize>,
    evaluation_buffer: Vec<char>,
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
            let (c, forced_index) = if let Some(pending) = self.pending_chars.pop_front() {
                (pending.c, Some(pending.global_index))
            } else {
                let c = chars[i];
                i += 1;
                (c, None)
            };

            let sol = at_start_of_line;
            at_start_of_line = c == '\n';
            self.process_one(c, sol, forced_index, &mut out, &mut run);
        }

        self.at_start_of_line = at_start_of_line;
        Self::flush_run(&mut out, &mut run);
        out
    }

    pub fn destroy(self) {}

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

        for pending in self.waitfor_pending.drain(..) {
            if pending.should_emit {
                Self::emit_index(out, MarkdownProcessorType::PlainText, pending.global_index, run);
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
        });
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
        }

        self.evaluation_buffer.push(c);
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
        let pattern_chars = pattern.chars().collect::<Vec<_>>();
        if buffer.len() > pattern_chars.len() {
            return false;
        }
        buffer
            .iter()
            .zip(pattern_chars.iter())
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    }

    fn matches_case_insensitive_full(buffer: &[char], pattern: &str) -> bool {
        let pattern_chars = pattern.chars().collect::<Vec<_>>();
        buffer.len() == pattern_chars.len()
            && buffer
                .iter()
                .zip(pattern_chars.iter())
                .all(|(left, right)| left.eq_ignore_ascii_case(right))
    }
}

#[derive(Debug, Clone, Default)]
pub struct NativeMarkdownSplitter;

impl NativeMarkdownSplitter {
    pub fn create_block_session() -> MarkdownSession {
        MarkdownSession::new(Self::get_block_plugins())
    }

    pub fn create_inline_session() -> MarkdownSession {
        MarkdownSession::new(Self::get_inline_plugins())
    }

    pub fn get_block_plugins() -> Vec<PluginEntry> {
        vec![
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
                plugin: Box::new(StreamMarkdownUnorderedListPlugin::new(false)),
                tag: MarkdownProcessorType::UnorderedList,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownHorizontalRulePlugin::new(true)),
                tag: MarkdownProcessorType::HorizontalRule,
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

    pub fn get_inline_plugins() -> Vec<PluginEntry> {
        vec![
            PluginEntry {
                plugin: Box::new(StreamMarkdownBoldPlugin::new(false)),
                tag: MarkdownProcessorType::Bold,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownItalicPlugin::new(false)),
                tag: MarkdownProcessorType::Italic,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownInlineCodePlugin::new(false)),
                tag: MarkdownProcessorType::InlineCode,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownLinkPlugin::default()),
                tag: MarkdownProcessorType::Link,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownStrikethroughPlugin::new(false)),
                tag: MarkdownProcessorType::Strikethrough,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownUnderlinePlugin::new(true)),
                tag: MarkdownProcessorType::Underline,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownInlineLaTeXPlugin::new(false)),
                tag: MarkdownProcessorType::InlineLatex,
            },
            PluginEntry {
                plugin: Box::new(StreamMarkdownInlineParenLaTeXPlugin::new(true)),
                tag: MarkdownProcessorType::InlineLatex,
            },
        ]
    }

    pub fn parse_inline_to_stable_nodes(content: &str) -> Vec<MarkdownNodeStable> {
        if content.is_empty() {
            return Vec::new();
        }
        let mut session = Self::create_inline_session();
        Self::segments_to_stable_nodes(content, session.push(content), false)
    }

    pub fn native_markdown_split_by_block(content: &str) -> Vec<MarkdownNodeStable> {
        let mut session = Self::create_block_session();
        Self::segments_to_stable_nodes(content, session.push(content), true)
    }

    pub fn native_markdown_split_stream_by_block(
        mut chars: impl Stream<Item = char>,
    ) -> Vec<MarkdownNodeStable> {
        let mut content = String::new();
        chars.collect(&mut |ch| content.push(ch));
        Self::native_markdown_split_by_block(&content)
    }

    pub fn native_markdown_split_string_stream_by_block(
        mut source: impl Stream<Item = String>,
    ) -> Vec<MarkdownNodeStable> {
        let mut content = String::new();
        source.collect(&mut |chunk| content.push_str(&chunk));
        Self::native_markdown_split_by_block(&content)
    }

    pub fn native_markdown_split_by_inline(content: &str) -> Vec<MarkdownNodeStable> {
        let mut session = Self::create_inline_session();
        Self::segments_to_stable_nodes(content, session.push(content), false)
    }

    pub fn native_markdown_split_stream_by_inline(
        mut chars: impl Stream<Item = char>,
    ) -> Vec<MarkdownNodeStable> {
        let mut content = String::new();
        chars.collect(&mut |ch| content.push(ch));
        Self::native_markdown_split_by_inline(&content)
    }

    pub fn native_markdown_split_string_stream_by_inline(
        mut source: impl Stream<Item = String>,
    ) -> Vec<MarkdownNodeStable> {
        let mut content = String::new();
        source.collect(&mut |chunk| content.push_str(&chunk));
        Self::native_markdown_split_by_inline(&content)
    }

    pub fn native_markdown_split_by_block_groups(
        content: &str,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        let mut session = Self::create_block_session();
        Self::segments_to_groups(content, session.push(content))
    }

    pub fn native_markdown_split_by_inline_groups(
        content: &str,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        let mut session = Self::create_inline_session();
        Self::segments_to_groups(content, session.push(content))
    }

    fn segments_to_stable_nodes(
        content: &str,
        segments: Vec<Segment>,
        parse_inline_children: bool,
    ) -> Vec<MarkdownNodeStable> {
        let mut nodes = Vec::new();
        for segment in segments {
            if segment.r#type < 0 {
                continue;
            }
            let node_type = MarkdownProcessorType::from_ordinal(segment.r#type)
                .unwrap_or(MarkdownProcessorType::PlainText);
            if segment.start > segment.end || segment.end > content.chars().count() {
                continue;
            }
            let node_content = if node_type == MarkdownProcessorType::HtmlBreak {
                "\n".to_string()
            } else {
                char_slice(content, segment.start, segment.end)
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
            });
        }
        nodes
    }

    fn segments_to_groups(
        content: &str,
        segments: Vec<Segment>,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        let groups = segments
            .into_iter()
            .filter_map(|segment| {
                if segment.r#type < 0 {
                    return None;
                }
                let node_type = MarkdownProcessorType::from_ordinal(segment.r#type)
                    .unwrap_or(MarkdownProcessorType::PlainText);
                let node_content = if node_type == MarkdownProcessorType::HtmlBreak {
                    "\n".to_string()
                } else {
                    char_slice(content, segment.start, segment.end)
                };
                Some(StreamGroup::new(
                    if node_type == MarkdownProcessorType::PlainText {
                        None
                    } else {
                        Some(node_type)
                    },
                    Box::new(VecStream::new(vec![node_content])),
                ))
            })
            .collect::<Vec<_>>();
        VecStream::new(groups)
    }

    fn is_inline_container(block_type: MarkdownProcessorType) -> bool {
        block_type != MarkdownProcessorType::CodeBlock
            && block_type != MarkdownProcessorType::BlockLatex
            && block_type != MarkdownProcessorType::Table
            && block_type != MarkdownProcessorType::XmlBlock
    }
}

fn char_slice(content: &str, start: usize, end: usize) -> String {
    content
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}
