use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use operit_util::streamnative::NativeMarkdownSplitter::{
    MarkdownNodeStable, MarkdownProcessorType,
};
use operit_util::ChatMarkupRegex::{attr_value, tag_body, tag_ranges, ChatMarkupRegex};

use super::i18n::TuiText;

/// Stores user expand/collapse overrides for transcript fold widgets.
#[derive(Clone, Debug, Default)]
pub(super) struct TranscriptFoldState {
    user_expanded: HashMap<(i64, String), bool>,
}

/// Identifies one clickable fold header in the rendered transcript.
#[derive(Clone, Debug)]
pub(super) struct TranscriptFoldHit {
    pub(super) line_index: usize,
    pub(super) target: FoldTarget,
}

/// Names one fold widget owned by a chat message.
#[derive(Clone, Debug)]
pub(super) struct FoldTarget {
    pub(super) message_timestamp: i64,
    pub(super) stable_key: String,
    pub(super) expanded: bool,
}

/// Supplies per-message fold policy while rendering Markdown nodes.
#[derive(Clone, Copy, Debug)]
pub(super) struct FoldRenderContext<'a> {
    pub(super) message_timestamp: i64,
    pub(super) is_streaming: bool,
    pub(super) fold_state: &'a TranscriptFoldState,
    pub(super) thinking_line: Option<&'a ratatui::text::Line<'static>>,
}

/// Holds rendered lines together with clickable fold hit regions.
#[derive(Clone, Debug, Default)]
pub(super) struct FoldedLines {
    pub(super) xml: Vec<super::compose::XmlSurfaceSlot>,
    pub(super) lines: Vec<ratatui::text::Line<'static>>,
    pub(super) hits: Vec<TranscriptFoldHit>,
}

/// Describes one grouping decision over a Markdown node sequence.
#[derive(Clone, Debug)]
pub(super) enum GroupedItem {
    Single(usize),
    Group {
        start_index: usize,
        end_index_inclusive: usize,
        stable_key: String,
    },
}

/// Names the Flutter-equivalent group title for a collapsed tool sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FoldGroupKind {
    ToolsOnly,
    SearchOnly,
    ThinkingSearchTools,
    ThinkingSearch,
    ThinkingTools,
}

/// Describes one complete tool-like XML node used for merge matching.
#[derive(Clone, Debug)]
pub(super) struct ToolMergeNode {
    pub(super) kind: ToolMergeKind,
    pub(super) tool_name: String,
    pub(super) match_tool_name: String,
    pub(super) params: Vec<(String, String)>,
    pub(super) result_text: String,
    pub(super) is_success: bool,
}

/// Distinguishes a tool call from its matching result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToolMergeKind {
    Call,
    Result,
}

/// Describes one matched call/result batch.
#[derive(Clone, Debug)]
pub(super) struct ToolMergeMatch {
    pub(super) start_index: usize,
    pub(super) end_index_inclusive: usize,
    pub(super) pairs: Vec<(ToolMergeNode, ToolMergeNode)>,
}

impl TranscriptFoldState {
    /// Records an explicit user expand or collapse for one fold widget.
    pub(super) fn set_user_expanded(
        &mut self,
        message_timestamp: i64,
        stable_key: &str,
        expanded: bool,
    ) {
        self.user_expanded
            .insert((message_timestamp, stable_key.to_string()), expanded);
    }

    /// Returns whether a fold widget is expanded after applying user overrides.
    pub(super) fn is_expanded(
        &self,
        message_timestamp: i64,
        stable_key: &str,
        auto_expand: bool,
    ) -> bool {
        match self
            .user_expanded
            .get(&(message_timestamp, stable_key.to_string()))
        {
            Some(expanded) => *expanded,
            None => auto_expand,
        }
    }

    /// Drops overrides that no longer belong to a live message.
    pub(super) fn retain_messages(&mut self, timestamps: &HashSet<i64>) {
        self.user_expanded
            .retain(|(timestamp, _), _| timestamps.contains(timestamp));
    }

    /// Clears every stored override.
    pub(super) fn clear(&mut self) {
        self.user_expanded.clear();
    }

    /// Hashes user overrides that belong to one message for render-cache keys.
    pub(super) fn signature_for_message(&self, message_timestamp: i64) -> u64 {
        let mut entries = self
            .user_expanded
            .iter()
            .filter(|((timestamp, _), _)| *timestamp == message_timestamp)
            .map(|((_, key), expanded)| (key.clone(), *expanded))
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for (key, expanded) in entries {
            key.hash(&mut hasher);
            expanded.hash(&mut hasher);
        }
        hasher.finish()
    }
}

impl FoldedLines {
    /// Appends another folded block while shifting its hit line indexes.
    pub(super) fn extend(&mut self, other: FoldedLines) {
        let offset = self.lines.len();
        for mut slot in other.xml {
            slot.lines = slot.lines.start + offset..slot.lines.end + offset;
            self.xml.push(slot);
        }
        for mut hit in other.hits {
            hit.line_index += offset;
            self.hits.push(hit);
        }
        self.lines.extend(other.lines);
    }

    /// Records the click target of one disclosure covering `line_range`.
    ///
    /// A disclosure owns its whole body, so the range starts at the header and
    /// ends at the last body line. Those hits are inserted *before* the hits of
    /// the widgets rendered inside the body: a lookup then finds the innermost
    /// widget under the pointer, and clicking a nested tool row expands that row
    /// instead of collapsing the block that contains it.
    pub(super) fn push_hit_range(
        &mut self,
        line_range: std::ops::Range<usize>,
        target: FoldTarget,
    ) {
        let mut enclosing = line_range
            .map(|line_index| TranscriptFoldHit {
                line_index,
                target: target.clone(),
            })
            .collect::<Vec<_>>();
        enclosing.append(&mut self.hits);
        self.hits = enclosing;
    }
}

impl FoldRenderContext<'_> {
    /// Resolves expansion for one fold widget owned by the current message.
    pub(super) fn is_expanded(&self, stable_key: &str, auto_expand: bool) -> bool {
        self.fold_state
            .is_expanded(self.message_timestamp, stable_key, auto_expand)
    }

    /// Builds a clickable fold target for the current message.
    pub(super) fn target(&self, stable_key: String, expanded: bool) -> FoldTarget {
        FoldTarget {
            message_timestamp: self.message_timestamp,
            stable_key,
            expanded,
        }
    }
}

/// Groups thinking, tool, and search XML nodes the same way the main app does.
pub(super) fn group_markdown_nodes(nodes: &[MarkdownNodeStable]) -> Vec<GroupedItem> {
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < nodes.len() {
        let node = &nodes[index];
        let Some(tag) = xml_tag_name(node) else {
            out.push(GroupedItem::Single(index));
            index += 1;
            continue;
        };

        if tag == "think" || tag == "thinking" {
            let mut cursor = index + 1;
            let mut tool_count = 0usize;
            let mut search_count = 0usize;
            let mut xml_tool_related_count = 0usize;
            while cursor < nodes.len() {
                let next = &nodes[cursor];
                if is_tool_grouping_separator(next) {
                    cursor += 1;
                    continue;
                }
                let Some(next_tag) = xml_tag_name(next) else {
                    break;
                };
                if is_ignorable_xml_tag(&next_tag) {
                    cursor += 1;
                    continue;
                }
                let is_think_again = next_tag == "think" || next_tag == "thinking";
                let is_tool_related = next_tag == "tool" || next_tag == "tool_result";
                let is_search_related = next_tag == "search";
                if !is_think_again && !is_tool_related && !is_search_related {
                    break;
                }
                if is_tool_related {
                    if next_tag == "tool" {
                        tool_count += 1;
                    }
                    xml_tool_related_count += 1;
                }
                if is_search_related {
                    search_count += 1;
                    xml_tool_related_count += 1;
                }
                cursor += 1;
            }
            if should_collapse_think_sequence(tool_count, search_count, xml_tool_related_count) {
                out.push(GroupedItem::Group {
                    start_index: index,
                    end_index_inclusive: cursor - 1,
                    stable_key: format!("think-tools-{index}"),
                });
                index = cursor;
                continue;
            }
            out.push(GroupedItem::Single(index));
            index += 1;
            continue;
        }

        if tag == "tool" || tag == "tool_result" {
            let mut cursor = index + 1;
            let mut tool_count = if tag == "tool" { 1 } else { 0 };
            let mut xml_tool_related_count = 1usize;
            while cursor < nodes.len() {
                let next = &nodes[cursor];
                if is_tool_grouping_separator(next) {
                    cursor += 1;
                    continue;
                }
                let Some(next_tag) = xml_tag_name(next) else {
                    break;
                };
                if is_ignorable_xml_tag(&next_tag) {
                    cursor += 1;
                    continue;
                }
                if next_tag != "tool" && next_tag != "tool_result" {
                    break;
                }
                xml_tool_related_count += 1;
                if next_tag == "tool" {
                    tool_count += 1;
                }
                cursor += 1;
            }
            if should_collapse_tool_sequence(tool_count, xml_tool_related_count) {
                out.push(GroupedItem::Group {
                    start_index: index,
                    end_index_inclusive: cursor - 1,
                    stable_key: format!("tools-only-{index}"),
                });
                index = cursor;
            } else {
                out.push(GroupedItem::Single(index));
                index += 1;
            }
            continue;
        }

        if tag == "search" {
            let mut cursor = index + 1;
            while cursor < nodes.len() {
                let next = &nodes[cursor];
                if is_tool_grouping_separator(next) {
                    cursor += 1;
                    continue;
                }
                let Some(next_tag) = xml_tag_name(next) else {
                    break;
                };
                if is_ignorable_xml_tag(&next_tag) {
                    cursor += 1;
                    continue;
                }
                if next_tag != "search" {
                    break;
                }
                cursor += 1;
            }
            out.push(GroupedItem::Group {
                start_index: index,
                end_index_inclusive: cursor - 1,
                stable_key: format!("search-only-{index}"),
            });
            index = cursor;
            continue;
        }

        out.push(GroupedItem::Single(index));
        index += 1;
    }
    out
}

/// Matches ordered tool calls with equally ordered, name-matched results.
pub(super) fn match_tool_merge(
    nodes: &[MarkdownNodeStable],
    start_index: usize,
    end_index_inclusive: usize,
) -> Option<ToolMergeMatch> {
    let first = parse_tool_merge_node(&nodes[start_index])?;
    if first.kind != ToolMergeKind::Call {
        return None;
    }

    let mut call_names = Vec::new();
    let mut calls = Vec::new();
    let mut results = Vec::new();
    let mut reading_results = false;
    for index in start_index..=end_index_inclusive {
        let node = &nodes[index];
        if is_tool_merge_separator(node) {
            continue;
        }
        let parsed = parse_tool_merge_node(node)?;
        if !reading_results && parsed.kind == ToolMergeKind::Call {
            call_names.push(parsed.match_tool_name.clone());
            calls.push(parsed);
            continue;
        }
        if parsed.kind == ToolMergeKind::Result {
            reading_results = true;
            if results.len() >= call_names.len()
                || parsed.match_tool_name != call_names[results.len()]
            {
                return None;
            }
            results.push(parsed);
            if results.len() == call_names.len() {
                let pairs = calls
                    .into_iter()
                    .zip(results.into_iter())
                    .collect::<Vec<_>>();
                return Some(ToolMergeMatch {
                    start_index,
                    end_index_inclusive: index,
                    pairs,
                });
            }
            continue;
        }
        return None;
    }
    None
}

/// Classifies a grouped XML range into the main-app group title kind.
pub(super) fn fold_group_kind(
    stable_key: &str,
    tool_count: usize,
    search_count: usize,
) -> FoldGroupKind {
    if stable_key.starts_with("tools-only-") {
        FoldGroupKind::ToolsOnly
    } else if stable_key.starts_with("search-only-") {
        FoldGroupKind::SearchOnly
    } else if search_count > 0 && tool_count > 0 {
        FoldGroupKind::ThinkingSearchTools
    } else if search_count > 0 {
        FoldGroupKind::ThinkingSearch
    } else {
        FoldGroupKind::ThinkingTools
    }
}

/// Builds the localized title for one collapsed thinking/tool/search group.
pub(super) fn fold_group_title(kind: FoldGroupKind, tool_count: usize, text: TuiText) -> String {
    match kind {
        FoldGroupKind::ToolsOnly => text.tools_group_title_with_count(tool_count),
        FoldGroupKind::SearchOnly => text.search_group_title().to_string(),
        FoldGroupKind::ThinkingSearchTools => {
            text.thinking_search_tools_group_title_with_count(tool_count)
        }
        FoldGroupKind::ThinkingSearch => text.thinking_search_group_title().to_string(),
        FoldGroupKind::ThinkingTools => text.thinking_tools_group_title_with_count(tool_count),
    }
}

/// Counts tool-call XML nodes inside an inclusive node range.
pub(super) fn count_tool_nodes(
    nodes: &[MarkdownNodeStable],
    start: usize,
    end_inclusive: usize,
) -> usize {
    nodes[start..=end_inclusive]
        .iter()
        .filter(|node| xml_tag_name(node).as_deref() == Some("tool"))
        .count()
}

/// Counts search XML nodes inside an inclusive node range.
pub(super) fn count_search_nodes(
    nodes: &[MarkdownNodeStable],
    start: usize,
    end_inclusive: usize,
) -> usize {
    nodes[start..=end_inclusive]
        .iter()
        .filter(|node| xml_tag_name(node).as_deref() == Some("search"))
        .count()
}

/// Returns whether a group should auto-expand while its message is still streaming.
pub(super) fn group_should_auto_expand(
    nodes: &[MarkdownNodeStable],
    end_index_inclusive: usize,
    is_streaming: bool,
) -> bool {
    if !is_streaming {
        return false;
    }
    nodes
        .iter()
        .skip(end_index_inclusive + 1)
        .all(is_conforming_tail_node)
}

/// Returns whether a thinking XML node is still open and therefore in progress.
pub(super) fn think_is_in_progress(node: &MarkdownNodeStable) -> bool {
    !xml_is_closed(node)
}

/// Returns whether a standalone thinking panel should auto-expand.
pub(super) fn think_should_auto_expand(node: &MarkdownNodeStable) -> bool {
    think_is_in_progress(node)
}

/// Returns whether a details tag starts expanded from its open attribute.
pub(super) fn details_should_auto_expand(content: &str) -> bool {
    opening_tag_has_flag(content, "open")
}

/// Reads the canonical XML tag name for a Markdown node.
pub(super) fn xml_tag_name(node: &MarkdownNodeStable) -> Option<String> {
    if node.r#type != MarkdownProcessorType::XmlBlock {
        return None;
    }
    ChatMarkupRegex::normalize_tool_like_tag_name(
        ChatMarkupRegex::extract_opening_tag_name(&node.content).as_deref(),
    )
}

/// Returns the inner XML body, including the open-tag tail while a tag is still streaming.
pub(super) fn xml_inner_body(content: &str) -> &str {
    let Some(tag_name) = ChatMarkupRegex::extract_opening_tag_name(content) else {
        return "";
    };
    if let Some(body) = tag_body(content, &tag_name) {
        return body;
    }
    match content.find('>') {
        Some(end) => &content[end + 1..],
        None => "",
    }
}

/// Returns whether a Markdown XML node has been closed by its matching end tag.
pub(super) fn xml_is_closed(node: &MarkdownNodeStable) -> bool {
    let Some(tag_name) = ChatMarkupRegex::extract_opening_tag_name(&node.content) else {
        return false;
    };
    tag_body(&node.content, &tag_name).is_some()
}

/// Parses one complete tool-like XML node for deterministic merge matching.
pub(super) fn parse_tool_merge_node(node: &MarkdownNodeStable) -> Option<ToolMergeNode> {
    let tag = xml_tag_name(node)?;
    if tag != "tool" && tag != "tool_result" {
        return None;
    }
    let tool_name = attr_value(&node.content, "name")?;
    if tool_name.trim().is_empty() {
        return None;
    }
    if tag == "tool" {
        let match_tool_name = resolve_proxy_target_tool_name(&tool_name, &node.content)?;
        return Some(ToolMergeNode {
            kind: ToolMergeKind::Call,
            tool_name,
            match_tool_name,
            params: extract_param_pairs(xml_inner_body(&node.content)),
            result_text: String::new(),
            is_success: false,
        });
    }
    let status = attr_value(&node.content, "status")
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    Some(ToolMergeNode {
        kind: ToolMergeKind::Result,
        tool_name: tool_name.clone(),
        match_tool_name: tool_name,
        params: Vec::new(),
        result_text: tool_result_text(&node.content),
        is_success: status.is_empty() || status == "success",
    })
}

/// Extracts a details summary and remaining body from a details XML node.
pub(super) fn details_summary_and_body(content: &str) -> (String, String) {
    let inner = xml_inner_body(content);
    let summary = tag_body(inner, "summary")
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    let body = if let Some(summary_body) = tag_body(inner, "summary") {
        let summary_range = tag_ranges(inner, "summary")
            .into_iter()
            .next()
            .expect("a parsed summary tag must have a source range");
        let mut remaining = String::new();
        remaining.push_str(&inner[..summary_range.0]);
        remaining.push_str(&inner[summary_range.1..]);
        let _ = summary_body;
        remaining.trim().to_string()
    } else {
        inner.trim().to_string()
    };
    (summary, body)
}

fn resolve_proxy_target_tool_name(tool_name: &str, content: &str) -> Option<String> {
    if tool_name != "proxy" && tool_name != "package_proxy" {
        return Some(tool_name.to_string());
    }
    for (name, value) in extract_param_pairs(xml_inner_body(content)) {
        if name == "tool_name" {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return None;
            }
            return Some(trimmed.to_string());
        }
    }
    None
}

fn extract_param_pairs(content: &str) -> Vec<(String, String)> {
    tag_ranges(content, "param")
        .into_iter()
        .filter_map(|(start, end)| {
            let block = &content[start..end];
            let name = attr_value(block, "name")?;
            let body = tag_body(block, "param")?.to_string();
            Some((name, body))
        })
        .collect()
}

fn tool_result_text(content: &str) -> String {
    let inner = xml_inner_body(content).trim();
    if let Some(payload) = tag_body(inner, "content") {
        return payload.trim().to_string();
    }
    inner.to_string()
}

fn is_ignorable_xml_tag(tag: &str) -> bool {
    tag == "meta"
}

fn is_tool_grouping_separator(node: &MarkdownNodeStable) -> bool {
    matches!(
        node.r#type,
        MarkdownProcessorType::HtmlBreak | MarkdownProcessorType::PlainText
    ) && node.content.trim().is_empty()
}

fn is_tool_merge_separator(node: &MarkdownNodeStable) -> bool {
    if is_tool_grouping_separator(node) {
        return true;
    }
    xml_tag_name(node).as_deref() == Some("meta")
}

fn should_collapse_tool_sequence(tool_count: usize, xml_tool_related_count: usize) -> bool {
    xml_tool_related_count > 0 && tool_count >= 2 && xml_tool_related_count >= 2
}

fn should_collapse_think_sequence(
    tool_count: usize,
    search_count: usize,
    xml_tool_related_count: usize,
) -> bool {
    xml_tool_related_count > 0 && (search_count > 0 || tool_count > 0)
}

fn is_conforming_tail_node(node: &MarkdownNodeStable) -> bool {
    match node.r#type {
        MarkdownProcessorType::PlainText => node.content.trim().is_empty(),
        MarkdownProcessorType::HtmlBreak => true,
        MarkdownProcessorType::XmlBlock => match xml_tag_name(node).as_deref() {
            Some("think") | Some("thinking") | Some("search") | Some("meta") | Some("tool")
            | Some("tool_result") => true,
            Some(_) => false,
            None => !xml_is_closed(node),
        },
        _ => false,
    }
}

fn opening_tag_has_flag(content: &str, flag: &str) -> bool {
    let Some(end) = content.find('>') else {
        return false;
    };
    let opening = &content[..end];
    if attr_value(opening, flag).is_some() {
        return true;
    }
    let Some(rest) = opening.strip_prefix('<') else {
        return false;
    };
    let mut tokens = rest.split_whitespace();
    tokens.next();
    for token in tokens {
        let name = token
            .split('=')
            .next()
            .expect("split always yields a name token");
        if name.eq_ignore_ascii_case(flag) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use operit_util::streamnative::NativeMarkdownStreamOperators::NativeMarkdownStreamOperators;

    use super::*;

    /// Verifies consecutive tool calls collapse into one tools-only group.
    #[test]
    fn groups_two_tool_calls_into_one_fold() {
        let markup = concat!(
            "<tool name=\"read_file\" call_id=\"a\"><param name=\"path\">a.txt</param></tool>",
            "<tool name=\"list_files\" call_id=\"b\"><param name=\"path\">.</param></tool>",
        );
        let nodes = markup.nativeMarkdownSplitByBlock();
        let grouped = group_markdown_nodes(&nodes);
        assert!(matches!(
            grouped.as_slice(),
            [GroupedItem::Group {
                stable_key,
                ..
            }] if stable_key.starts_with("tools-only-")
        ));
    }

    /// Verifies a single call/result pair merges instead of forming a group.
    #[test]
    fn merges_one_call_with_its_matching_result() {
        let markup = concat!(
            "<tool name=\"read_file\" call_id=\"a\"><param name=\"path\">a.txt</param></tool>",
            "<tool_result name=\"read_file\" status=\"success\"><content>ok</content></tool_result>",
        );
        let nodes = markup.nativeMarkdownSplitByBlock();
        let grouped = group_markdown_nodes(&nodes);
        assert!(grouped
            .iter()
            .all(|item| matches!(item, GroupedItem::Single(_))));
        let merge = match_tool_merge(&nodes, 0, nodes.len() - 1).expect("pair must merge");
        assert_eq!(merge.pairs.len(), 1);
        assert_eq!(merge.pairs[0].0.tool_name, "read_file");
        assert_eq!(merge.pairs[0].1.result_text, "ok");
        assert!(merge.pairs[0].1.is_success);
    }
}
