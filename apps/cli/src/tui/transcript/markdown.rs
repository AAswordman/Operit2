use operit_runtime::util::streamnative::NativeMarkdownSplitter::{
    MarkdownNodeStable, MarkdownProcessorType,
};
use operit_runtime::util::streamnative::NativeMarkdownStreamOperators::NativeMarkdownStreamOperators;
use operit_runtime::util::ChatMarkupRegex::{attr_value, tag_body, ChatMarkupRegex};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use super::theme;

const TOOL_CALL_INLINE_DETAIL_CHAR_LIMIT: usize = 160;
const TOOL_RESULT_INLINE_DETAIL_CHAR_LIMIT: usize = 80;
const TOOL_RESULT_PREFIX_DISPLAY_WIDTH: usize = 8;
const STREAMING_MARKDOWN_TAIL_BLOCKS: usize = 4;

#[derive(Clone, Debug, Default)]
pub(super) struct MarkdownRenderCache {
    blocks: HashMap<usize, CachedMarkdownBlock>,
}

#[derive(Clone, Debug)]
struct CachedMarkdownBlock {
    key: MarkdownBlockRenderKey,
    lines: Vec<Line<'static>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MarkdownBlockRenderKey {
    content_width: usize,
    node_hash: u64,
}

pub(super) fn render_markdown_lines(content: &str, content_width: usize) -> Vec<Line<'static>> {
    let nodes = content.nativeMarkdownSplitByBlock();
    render_markdown_nodes_lines(&nodes, content_width)
}

pub(super) fn render_markdown_lines_cached(
    content: &str,
    content_width: usize,
    cache: &mut MarkdownRenderCache,
    is_streaming: bool,
) -> Vec<Line<'static>> {
    let nodes = content.nativeMarkdownSplitByBlock();
    render_markdown_nodes_lines_cached(&nodes, content_width, cache, is_streaming)
}

pub(super) fn render_markdown_nodes_lines(
    nodes: &[MarkdownNodeStable],
    content_width: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut previous_kind: Option<RenderedBlockKind> = None;
    for node in nodes {
        if is_blank_text_block(node) {
            continue;
        }
        let kind = rendered_block_kind(node);
        if should_insert_block_spacing(previous_kind, kind) && !last_line_is_blank(&lines) {
            lines.push(Line::from(""));
        }
        let before_len = lines.len();
        lines.extend(render_markdown_block_lines(node, content_width));
        if lines.len() > before_len {
            previous_kind = Some(kind);
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines
}

pub(super) fn render_markdown_nodes_lines_cached(
    nodes: &[MarkdownNodeStable],
    content_width: usize,
    cache: &mut MarkdownRenderCache,
    is_streaming: bool,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut previous_kind: Option<RenderedBlockKind> = None;
    let tail_start_index = if is_streaming {
        streaming_tail_start_index(nodes)
    } else {
        nodes.len()
    };
    let mut live_indexes = HashSet::new();

    for (index, node) in nodes.iter().enumerate() {
        if is_blank_text_block(node) {
            continue;
        }
        live_indexes.insert(index);
        let kind = rendered_block_kind(node);
        if should_insert_block_spacing(previous_kind, kind) && !last_line_is_blank(&lines) {
            lines.push(Line::from(""));
        }

        let before_len = lines.len();
        let block_lines = render_cached_markdown_block(
            index,
            node,
            content_width,
            cache,
            index >= tail_start_index,
        );
        lines.extend(block_lines);
        if lines.len() > before_len {
            previous_kind = Some(kind);
        }
    }

    cache.blocks.retain(|index, _| live_indexes.contains(index));

    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RenderedBlockKind {
    Text,
    Header,
    Tool,
    List,
    Table,
    HorizontalRule,
    Code,
    Other,
}

fn render_cached_markdown_block(
    index: usize,
    node: &MarkdownNodeStable,
    content_width: usize,
    cache: &mut MarkdownRenderCache,
    render_live_tail: bool,
) -> Vec<Line<'static>> {
    let key = MarkdownBlockRenderKey {
        content_width,
        node_hash: stable_markdown_node_hash(node),
    };
    if !render_live_tail {
        if let Some(cached) = cache.blocks.get(&index).filter(|cached| cached.key == key) {
            return cached.lines.clone();
        }
    }
    let lines = render_markdown_block_lines(node, content_width);
    cache.blocks.insert(
        index,
        CachedMarkdownBlock {
            key,
            lines: lines.clone(),
        },
    );
    lines
}

fn render_markdown_block_lines(
    node: &MarkdownNodeStable,
    content_width: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    render_block_node(node, content_width, &mut lines);
    lines
}

fn streaming_tail_start_index(nodes: &[MarkdownNodeStable]) -> usize {
    let mut seen = 0usize;
    for (index, node) in nodes.iter().enumerate().rev() {
        if is_blank_text_block(node) {
            continue;
        }
        seen += 1;
        if seen == STREAMING_MARKDOWN_TAIL_BLOCKS {
            return index;
        }
    }
    0
}

fn stable_markdown_node_hash(node: &MarkdownNodeStable) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_markdown_node(node, &mut hasher);
    hasher.finish()
}

fn hash_markdown_node(node: &MarkdownNodeStable, hasher: &mut impl Hasher) {
    (node.r#type as i32).hash(hasher);
    node.content.hash(hasher);
    node.children.len().hash(hasher);
    for child in &node.children {
        hash_markdown_node(child, hasher);
    }
}

fn rendered_block_kind(node: &MarkdownNodeStable) -> RenderedBlockKind {
    match node.r#type {
        MarkdownProcessorType::Header => RenderedBlockKind::Header,
        MarkdownProcessorType::CodeBlock | MarkdownProcessorType::BlockLatex => {
            RenderedBlockKind::Code
        }
        MarkdownProcessorType::OrderedList | MarkdownProcessorType::UnorderedList => {
            RenderedBlockKind::List
        }
        MarkdownProcessorType::HorizontalRule => RenderedBlockKind::HorizontalRule,
        MarkdownProcessorType::Table => RenderedBlockKind::Table,
        MarkdownProcessorType::XmlBlock => {
            let raw_tag = ChatMarkupRegex::extract_opening_tag_name(&node.content);
            match ChatMarkupRegex::normalize_tool_like_tag_name(raw_tag.as_deref()).as_deref() {
                Some("tool") | Some("tool_result") => RenderedBlockKind::Tool,
                _ => RenderedBlockKind::Other,
            }
        }
        MarkdownProcessorType::PlainText | MarkdownProcessorType::HtmlBreak => {
            RenderedBlockKind::Text
        }
        _ => RenderedBlockKind::Other,
    }
}

fn should_insert_block_spacing(
    previous: Option<RenderedBlockKind>,
    current: RenderedBlockKind,
) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    if previous == RenderedBlockKind::Tool && current == RenderedBlockKind::Tool {
        return false;
    }
    if previous == RenderedBlockKind::List && current == RenderedBlockKind::List {
        return false;
    }
    if previous == RenderedBlockKind::Table && current == RenderedBlockKind::Table {
        return false;
    }
    matches!(
        (previous, current),
        (RenderedBlockKind::Tool, RenderedBlockKind::Text)
            | (RenderedBlockKind::Tool, RenderedBlockKind::Header)
            | (RenderedBlockKind::Text, RenderedBlockKind::Tool)
            | (RenderedBlockKind::Header, RenderedBlockKind::Tool)
            | (RenderedBlockKind::HorizontalRule, RenderedBlockKind::Text)
            | (RenderedBlockKind::HorizontalRule, RenderedBlockKind::Header)
            | (RenderedBlockKind::Text, RenderedBlockKind::HorizontalRule)
            | (RenderedBlockKind::Header, RenderedBlockKind::HorizontalRule)
            | (RenderedBlockKind::Text, RenderedBlockKind::Text)
            | (RenderedBlockKind::Table, RenderedBlockKind::Text)
            | (RenderedBlockKind::Text, RenderedBlockKind::Table)
            | (RenderedBlockKind::Code, RenderedBlockKind::Text)
            | (RenderedBlockKind::Text, RenderedBlockKind::Code)
    )
}

fn last_line_is_blank(lines: &[Line<'static>]) -> bool {
    lines.last().map(line_is_blank).unwrap_or(false)
}

fn is_blank_text_block(node: &MarkdownNodeStable) -> bool {
    matches!(
        node.r#type,
        MarkdownProcessorType::PlainText | MarkdownProcessorType::HtmlBreak
    ) && node.content.trim().is_empty()
        && node.children.iter().all(is_blank_text_block)
}

fn render_block_node(
    node: &MarkdownNodeStable,
    content_width: usize,
    lines: &mut Vec<Line<'static>>,
) {
    match node.r#type {
        MarkdownProcessorType::Header => render_header(node, lines),
        MarkdownProcessorType::BlockQuote => render_block_quote(node, lines),
        MarkdownProcessorType::CodeBlock => render_code_block(&node.content, lines),
        MarkdownProcessorType::OrderedList => render_list_block(node, true, lines),
        MarkdownProcessorType::UnorderedList => render_list_block(node, false, lines),
        MarkdownProcessorType::HorizontalRule => lines.push(Line::from(Span::styled(
            "--------------------------------",
            Style::default().fg(theme::TEXT_SUBTLE),
        ))),
        MarkdownProcessorType::BlockLatex => render_latex_block(&node.content, lines),
        MarkdownProcessorType::Table => render_table_block(&node.content, lines),
        MarkdownProcessorType::XmlBlock => render_xml_block(&node.content, content_width, lines),
        MarkdownProcessorType::Image => {
            lines.extend(render_inline_nodes(&[node.clone()], Style::default()))
        }
        MarkdownProcessorType::PlainText | MarkdownProcessorType::HtmlBreak => {
            render_plain_text_node(node, lines);
        }
        _ => lines.extend(render_inline_nodes(&[node.clone()], Style::default())),
    }
}

fn render_plain_text_node(node: &MarkdownNodeStable, lines: &mut Vec<Line<'static>>) {
    let rendered = if node.children.is_empty() {
        render_plain_lines(&node.content, Style::default())
    } else {
        render_inline_nodes(&node.children, Style::default())
    };
    lines.extend(compact_plain_text_lines(rendered));
}

fn render_header(node: &MarkdownNodeStable, lines: &mut Vec<Line<'static>>) {
    let trimmed = node.content.trim_start();
    let level = trimmed
        .chars()
        .take_while(|ch| *ch == '#')
        .count()
        .clamp(1, 6);
    let text = trimmed.get(level..).unwrap_or("").trim_start();
    let prefix = "#".repeat(level.min(4));
    let mut spans = vec![Span::styled(
        format!("{prefix} "),
        Style::default().fg(theme::TEXT_SUBTLE),
    )];
    let inline_nodes = text.nativeMarkdownSplitByInline();
    spans.extend(render_inline_spans(
        &inline_nodes,
        Style::default()
            .fg(theme::ACCENT)
            .add_modifier(Modifier::BOLD),
    ));
    lines.push(Line::from(spans));
}

fn render_prefixed_inline_block(
    content: &str,
    prefix: &str,
    prefix_style: Style,
    content_style: Style,
    lines: &mut Vec<Line<'static>>,
) {
    let inline_nodes = content.nativeMarkdownSplitByInline();
    let inline_lines = render_inline_nodes(&inline_nodes, content_style);
    for line in inline_lines {
        let mut spans = vec![Span::styled(prefix.to_string(), prefix_style)];
        spans.extend(line.spans);
        lines.push(Line::from(spans));
    }
}

fn render_block_quote(node: &MarkdownNodeStable, lines: &mut Vec<Line<'static>>) {
    let content = strip_block_quote_marker(&node.content);
    render_prefixed_inline_block(
        &content,
        "> ",
        Style::default().fg(theme::TEXT_SUBTLE),
        Style::default()
            .fg(theme::TEXT_MUTED)
            .add_modifier(Modifier::ITALIC),
        lines,
    );
}

fn render_code_block(content: &str, lines: &mut Vec<Line<'static>>) {
    let mut iter = content.lines();
    let first = iter.next().unwrap_or("");
    let language = first
        .trim_start()
        .strip_prefix("```")
        .map(str::trim)
        .unwrap_or("");
    let title = if language.is_empty() {
        "``` code".to_string()
    } else {
        format!("``` code {language}")
    };
    lines.push(Line::from(Span::styled(
        title,
        Style::default()
            .fg(theme::TEXT_SUBTLE)
            .add_modifier(Modifier::BOLD),
    )));
    for raw in iter {
        if raw.trim_start().starts_with("```") {
            continue;
        }
        lines.push(Line::from(Span::styled(
            format!("  {raw}"),
            Style::default()
                .fg(theme::ACCENT_STRONG)
                .bg(theme::ACCENT_BG),
        )));
    }
    lines.push(Line::from(Span::styled(
        "```",
        Style::default().fg(theme::TEXT_SUBTLE),
    )));
}

fn render_list_block(node: &MarkdownNodeStable, ordered: bool, lines: &mut Vec<Line<'static>>) {
    let (marker, text) = if ordered {
        split_ordered_marker(&node.content)
    } else {
        ("- ".to_string(), strip_unordered_marker(&node.content))
    };
    let inline_nodes = text.trim_end().nativeMarkdownSplitByInline();
    let mut spans = vec![Span::styled(marker, Style::default().fg(theme::ACCENT))];
    spans.extend(render_inline_spans(&inline_nodes, Style::default()));
    lines.push(Line::from(spans));
}

fn render_latex_block(content: &str, lines: &mut Vec<Line<'static>>) {
    lines.push(Line::from(Span::styled(
        "$$",
        Style::default()
            .fg(theme::TEXT_SUBTLE)
            .add_modifier(Modifier::BOLD),
    )));
    for raw in strip_latex_block_delimiters(content).lines() {
        lines.push(Line::from(Span::styled(
            raw.to_string(),
            Style::default().fg(theme::ACCENT_STRONG),
        )));
    }
    lines.push(Line::from(Span::styled(
        "$$",
        Style::default().fg(theme::TEXT_SUBTLE),
    )));
}

fn render_table_block(content: &str, lines: &mut Vec<Line<'static>>) {
    for raw in content.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if is_table_separator(trimmed) {
            lines.push(Line::from(Span::styled(
                "--------------------------------",
                Style::default().fg(theme::TEXT_SUBTLE),
            )));
            continue;
        }
        let cells = trimmed
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>();
        let mut spans = Vec::new();
        for (index, cell) in cells.iter().enumerate() {
            if index > 0 {
                spans.push(Span::styled(
                    " | ".to_string(),
                    Style::default().fg(theme::TEXT_SUBTLE),
                ));
            }
            let inline_nodes = cell.nativeMarkdownSplitByInline();
            spans.extend(render_inline_spans(
                &inline_nodes,
                Style::default().fg(theme::TEXT_MUTED),
            ));
        }
        lines.push(Line::from(spans));
    }
}

fn render_xml_block(content: &str, content_width: usize, lines: &mut Vec<Line<'static>>) {
    let raw_tag = ChatMarkupRegex::extract_opening_tag_name(content);
    let tag = ChatMarkupRegex::normalize_tool_like_tag_name(raw_tag.as_deref());
    match tag.as_deref() {
        Some("tool") => render_tool_xml(content, false, content_width, lines),
        Some("tool_result") => render_tool_xml(content, true, content_width, lines),
        Some("attachment") => render_attachment_xml(content, lines),
        Some("image_link") | Some("audio_link") | Some("video_link") | Some("media_link") => {
            render_media_link_xml(content, lines)
        }
        Some("error") => render_error_xml(content, lines),
        Some("think") | Some("thinking") => render_named_xml_body("thinking", content, lines),
        Some("status") => render_status_xml(content, lines),
        Some("meta") => {}
        Some(name) => render_named_xml_body(name, content, lines),
        None => lines.extend(render_plain_lines(
            content,
            Style::default().fg(theme::TEXT_SUBTLE),
        )),
    }
}

fn render_tool_xml(
    content: &str,
    is_result: bool,
    content_width: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let name = attr_value(content, "name").unwrap_or_else(|| "tool".to_string());
    let status = attr_value(content, "status");
    let tag_name = ChatMarkupRegex::extract_opening_tag_name(content).unwrap_or_else(|| {
        if is_result {
            "tool_result".to_string()
        } else {
            "tool".to_string()
        }
    });
    let body = tag_body(content, &tag_name).unwrap_or("").trim();
    if is_result {
        render_tool_result_xml(&name, status.as_deref(), body, content_width, lines);
        return;
    }

    let params = extract_param_pairs(body);
    let is_strict_proxy = name == "package_proxy" || name == "proxy";
    let (display_name, display_params) = normalize_tool_display_for_strict_proxy(&name, &params);
    let summary =
        render_tool_param_summary(&display_params, if is_strict_proxy { "" } else { body });
    let leading_symbol = tool_leading_symbol(&display_name);
    let name_width = display_width(&display_name);
    let prefix_width = display_width(leading_symbol) + 1 + name_width;
    let summary_limit = content_width
        .saturating_sub(prefix_width)
        .saturating_sub(1)
        .min(TOOL_CALL_INLINE_DETAIL_CHAR_LIMIT);
    let mut header = vec![
        Span::styled(
            leading_symbol.to_string(),
            Style::default().fg(theme::ACCENT),
        ),
        Span::raw(" "),
        Span::styled(
            display_name,
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    let summary = if summary_limit >= 4 {
        compact_tool_summary(&summary, summary_limit)
    } else {
        String::new()
    };
    if !summary.is_empty() && summary_limit > 0 {
        header.push(Span::styled(" ".to_string(), Style::default()));
        header.push(Span::styled(
            summary,
            Style::default().fg(theme::TEXT_SUBTLE),
        ));
    }
    lines.push(Line::from(header));
}

fn render_attachment_xml(content: &str, lines: &mut Vec<Line<'static>>) {
    let file_name = attr_value(content, "filename").unwrap_or_else(|| "attachment".to_string());
    let mime_type = attr_value(content, "type").unwrap_or_default();
    let size = attr_value(content, "size")
        .and_then(|value| value.parse::<i64>().ok())
        .map(format_attachment_size);
    let mut spans = vec![
        Span::styled("@".to_string(), Style::default().fg(theme::ACCENT)),
        Span::styled(
            file_name,
            Style::default()
                .fg(theme::TEXT_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if !mime_type.is_empty() {
        spans.push(Span::styled(" ".to_string(), Style::default()));
        spans.push(Span::styled(
            mime_type,
            Style::default().fg(theme::TEXT_SUBTLE),
        ));
    }
    if let Some(size) = size {
        spans.push(Span::styled(" ".to_string(), Style::default()));
        spans.push(Span::styled(size, Style::default().fg(theme::TEXT_SUBTLE)));
    }
    lines.push(Line::from(spans));
}

fn render_media_link_xml(content: &str, lines: &mut Vec<Line<'static>>) {
    let id = attr_value(content, "id").unwrap_or_else(|| "media".to_string());
    let label = id
        .rsplit(|ch| ch == '/' || ch == '\\' || ch == ':')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or(id.as_str())
        .to_string();
    lines.push(Line::from(vec![
        Span::styled("@".to_string(), Style::default().fg(theme::ACCENT)),
        Span::styled(
            label,
            Style::default()
                .fg(theme::TEXT_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
}

fn tool_leading_symbol(tool_name: &str) -> &'static str {
    if tool_name.split_once(':').is_some() {
        return "→";
    }
    match tool_name {
        "list_files"
        | "read_file"
        | "read_file_part"
        | "read_file_full"
        | "read_file_binary"
        | "write_file"
        | "write_file_binary"
        | "delete_file"
        | "file_exists"
        | "move_file"
        | "copy_file"
        | "file_info"
        | "create_file"
        | "edit_file"
        | "zip_files"
        | "unzip_files"
        | "open_file"
        | "share_file"
        | "download_file"
        | "apply_file"
        | "browser_file_upload"
        | "read_environment_variable"
        | "write_environment_variable" => "▣",
        "search_tools" | "find_chat" | "query_memory" | "query_memory_links" => "⌕",
        "get_terminal_info"
        | "create_terminal_session"
        | "execute_in_terminal_session"
        | "execute_in_terminal_session_streaming"
        | "execute_hidden_terminal_command"
        | "close_terminal_session"
        | "input_in_terminal_session"
        | "get_terminal_session_screen"
        | "execute_cli_command" => ">",
        "browser_run_code" | "grep_code" => "{}",
        "visit_web" | "http_request" => "◎",
        "sleep"
        | "use_package"
        | "proxy"
        | "package_proxy"
        | "start_chat_service"
        | "stop_chat_service"
        | "create_new_chat"
        | "list_chats"
        | "agent_status"
        | "switch_chat"
        | "update_chat_title"
        | "delete_chat"
        | "send_message_to_ai"
        | "send_message_to_ai_streaming"
        | "list_character_cards"
        | "get_chat_messages"
        | "toast"
        | "send_notification"
        | "modify_system_setting"
        | "get_system_setting"
        | "install_app"
        | "uninstall_app"
        | "list_installed_apps"
        | "start_app"
        | "stop_app"
        | "get_notifications"
        | "get_app_usage_time"
        | "get_device_location"
        | "device_info"
        | "make_directory"
        | "find_files"
        | "grep_context"
        | "multipart_request"
        | "manage_cookies"
        | "browser_click"
        | "browser_close"
        | "browser_close_all"
        | "browser_console_messages"
        | "browser_drag"
        | "browser_evaluate"
        | "browser_fill_form"
        | "browser_handle_dialog"
        | "browser_hover"
        | "browser_navigate"
        | "browser_navigate_back"
        | "browser_network_requests"
        | "browser_press_key"
        | "browser_resize"
        | "browser_select_option"
        | "browser_snapshot"
        | "browser_tabs"
        | "browser_take_screenshot"
        | "browser_type"
        | "browser_wait_for"
        | "get_memory_by_title"
        | "create_memory"
        | "update_memory"
        | "delete_memory"
        | "move_memory"
        | "update_user_preferences"
        | "link_memories"
        | "update_memory_link"
        | "delete_memory_link" => "→",
        _ => panic!("unclassified tool for tui display: {tool_name}"),
    }
}

fn render_tool_result_xml(
    _name: &str,
    status: Option<&str>,
    body: &str,
    content_width: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let content = extract_first_tag_body(body, "content")
        .unwrap_or(body)
        .trim();
    let error = extract_first_tag_body(content, "error").map(str::trim);
    let is_error = status
        .map(|value| value.eq_ignore_ascii_case("error"))
        .unwrap_or(false)
        || error.is_some();
    let result_limit = content_width
        .saturating_sub(TOOL_RESULT_PREFIX_DISPLAY_WIDTH)
        .min(TOOL_RESULT_INLINE_DETAIL_CHAR_LIMIT)
        .max(8);
    let result = compact_tool_summary(
        &normalize_tool_display_text(error.unwrap_or(content)),
        result_limit,
    );
    let mut header = vec![
        Span::raw("    "),
        Span::styled(
            "↳".to_string(),
            Style::default().fg(if is_error {
                theme::ERROR
            } else {
                theme::TOOL_RESULT
            }),
        ),
        Span::raw(" "),
        Span::styled(
            if is_error { "×" } else { "✓" }.to_string(),
            Style::default().fg(if is_error {
                theme::ERROR
            } else {
                theme::TOOL_RESULT
            }),
        ),
        Span::raw(" "),
    ];
    if !result.is_empty() {
        header.push(Span::styled(
            result.clone(),
            Style::default().fg(theme::TOOL_RESULT),
        ));
    }
    lines.push(Line::from(header));
}

fn render_tool_param_summary(params: &[(String, String)], body: &str) -> String {
    if params.is_empty() {
        return normalize_tool_display_text(body);
    }
    params
        .iter()
        .map(|(name, value)| {
            let normalized_value = normalize_tool_display_text(value);
            format!("{name}={normalized_value}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn normalize_tool_display_for_strict_proxy(
    tool_name: &str,
    params: &[(String, String)],
) -> (String, Vec<(String, String)>) {
    if tool_name != "package_proxy" && tool_name != "proxy" {
        return (tool_name.to_string(), params.to_vec());
    }

    let raw_target_tool_name = params
        .iter()
        .find(|(name, _)| name == "tool_name")
        .map(|(_, value)| value.trim())
        .unwrap_or("");
    let raw_proxied_params = params
        .iter()
        .find(|(name, _)| name == "params")
        .map(|(_, value)| value.trim())
        .unwrap_or("");

    let display_tool_name = normalize_escaped_text_for_display(raw_target_tool_name);
    let display_params = if raw_proxied_params.is_empty() {
        params.to_vec()
    } else {
        parse_proxy_json_params(normalize_escaped_text_for_display(raw_proxied_params).trim())
            .unwrap_or_else(|| params.to_vec())
    };

    (
        if display_tool_name.trim().is_empty() {
            tool_name.to_string()
        } else {
            display_tool_name
        },
        display_params,
    )
}

fn normalize_escaped_text_for_display(input: &str) -> String {
    let unescaped = xml_unescape(input).replace("\\\"", "\"");
    let trimmed = unescaped.trim();
    if (trimmed.starts_with("\"{") && trimmed.ends_with("}\""))
        || (trimmed.starts_with("\"[") && trimmed.ends_with("]\""))
    {
        trimmed[1..trimmed.len().saturating_sub(1)].replace("\\\"", "\"")
    } else {
        unescaped
    }
}

fn parse_proxy_json_params(input: &str) -> Option<Vec<(String, String)>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Some(Vec::new());
    }
    let value = serde_json::from_str::<serde_json::Value>(trimmed).ok()?;
    match value {
        serde_json::Value::Object(object) => Some(
            object
                .into_iter()
                .map(|(name, value)| (name, json_value_to_param_text(value)))
                .collect(),
        ),
        serde_json::Value::Array(array) => Some(
            array
                .into_iter()
                .enumerate()
                .map(|(index, value)| (index.to_string(), json_value_to_param_text(value)))
                .collect(),
        ),
        _ => None,
    }
}

fn json_value_to_param_text(value: serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::String(text) => text,
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => value.to_string(),
    }
}

fn compact_tool_summary(value: &str, char_limit: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if display_width(&normalized) <= char_limit {
        return normalized;
    }
    let max_width = char_limit.saturating_sub(3);
    let mut summary = String::new();
    let mut width = 0usize;
    for ch in normalized.chars() {
        let char_width = char_display_width(ch);
        if width + char_width > max_width {
            break;
        }
        summary.push(ch);
        width += char_width;
    }
    summary.push_str("...");
    summary
}

fn format_attachment_size(size: i64) -> String {
    if size < 1024 {
        format!("{size} B")
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else {
        format!("{:.1} MB", size as f64 / 1024.0 / 1024.0)
    }
}

fn display_width(value: &str) -> usize {
    value.chars().map(char_display_width).sum()
}

fn char_display_width(ch: char) -> usize {
    if ch == '\0' || ch.is_control() {
        0
    } else if is_wide_char(ch) {
        2
    } else {
        1
    }
}

fn is_wide_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x115F
            | 0x2329..=0x232A
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1FAFF
            | 0x20000..=0x3FFFD
    )
}

fn extract_param_pairs(content: &str) -> Vec<(String, String)> {
    extract_tag_blocks(content, "param")
        .into_iter()
        .filter_map(|block| {
            let name = attr_value(block.opening_tag, "name")?;
            Some((name, xml_unescape(block.body.trim())))
        })
        .collect()
}

struct SimpleXmlBlock<'a> {
    opening_tag: &'a str,
    body: &'a str,
}

fn extract_first_tag_body<'a>(content: &'a str, tag_name: &str) -> Option<&'a str> {
    extract_tag_blocks(content, tag_name)
        .into_iter()
        .next()
        .map(|block| block.body)
}

fn extract_tag_blocks<'a>(content: &'a str, tag_name: &str) -> Vec<SimpleXmlBlock<'a>> {
    let mut blocks = Vec::new();
    let mut cursor = 0usize;
    let lower = content.to_ascii_lowercase();
    let open_prefix = format!("<{}", tag_name.to_ascii_lowercase());
    let close = format!("</{}>", tag_name.to_ascii_lowercase());
    while let Some(relative_start) = lower[cursor..].find(&open_prefix) {
        let start = cursor + relative_start;
        let after_name = start + open_prefix.len();
        if !lower
            .as_bytes()
            .get(after_name)
            .map(|byte| is_xml_tag_boundary(*byte))
            .unwrap_or(false)
        {
            cursor = after_name;
            continue;
        }
        let Some(open_end_relative) = lower[start..].find('>') else {
            break;
        };
        let open_end = start + open_end_relative + 1;
        let Some(close_relative) = lower[open_end..].find(&close) else {
            break;
        };
        let close_start = open_end + close_relative;
        let end = close_start + close.len();
        blocks.push(SimpleXmlBlock {
            opening_tag: &content[start..open_end],
            body: &content[open_end..close_start],
        });
        cursor = end;
    }
    blocks
}

fn is_xml_tag_boundary(byte: u8) -> bool {
    byte.is_ascii_whitespace() || byte == b'>' || byte == b'/'
}

fn normalize_tool_display_text(value: &str) -> String {
    xml_unescape(value)
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn xml_unescape(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn render_status_xml(content: &str, lines: &mut Vec<Line<'static>>) {
    let title = attr_value(content, "title");
    let status_type = attr_value(content, "type");
    let label = title
        .or(status_type)
        .unwrap_or_else(|| "status".to_string());
    lines.push(Line::from(vec![
        Span::styled("* ".to_string(), Style::default().fg(theme::TEXT_SUBTLE)),
        Span::styled(label, Style::default().fg(theme::TEXT_MUTED)),
    ]));
}

fn render_error_xml(content: &str, lines: &mut Vec<Line<'static>>) {
    let body = tag_body(content, "error")
        .map(xml_unescape)
        .or_else(|| attr_value(content, "message"))
        .or_else(|| attr_value(content, "error"))
        .unwrap_or_else(|| xml_unescape(content))
        .trim()
        .to_string();
    lines.push(Line::from(vec![
        Span::styled(
            "error: ".to_string(),
            Style::default()
                .fg(theme::ERROR)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(body, Style::default().fg(theme::ERROR_DIM)),
    ]));
}

fn render_named_xml_body(name: &str, content: &str, lines: &mut Vec<Line<'static>>) {
    let tag_name =
        ChatMarkupRegex::extract_opening_tag_name(content).unwrap_or_else(|| name.to_string());
    let body = tag_body(content, &tag_name).unwrap_or(content).trim();
    lines.push(Line::from(Span::styled(
        format!("<{name}>"),
        Style::default().fg(theme::TEXT_SUBTLE),
    )));
    if !body.is_empty() {
        lines.extend(render_plain_lines(
            body,
            Style::default().fg(theme::TEXT_MUTED),
        ));
    }
}

fn render_inline_nodes(nodes: &[MarkdownNodeStable], base_style: Style) -> Vec<Line<'static>> {
    let spans = render_inline_spans(nodes, base_style);
    split_spans_by_newline(spans)
}

fn render_inline_spans(nodes: &[MarkdownNodeStable], base_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for node in nodes {
        match node.r#type {
            MarkdownProcessorType::Bold => spans.push(Span::styled(
                strip_pair(&node.content, "**").unwrap_or_else(|| node.content.clone()),
                base_style.add_modifier(Modifier::BOLD),
            )),
            MarkdownProcessorType::Italic => spans.push(Span::styled(
                strip_pair(&node.content, "*").unwrap_or_else(|| node.content.clone()),
                base_style.add_modifier(Modifier::ITALIC),
            )),
            MarkdownProcessorType::InlineCode => spans.push(Span::styled(
                strip_pair(&node.content, "`").unwrap_or_else(|| node.content.clone()),
                Style::default()
                    .fg(theme::ACCENT_STRONG)
                    .bg(theme::ACCENT_BG),
            )),
            MarkdownProcessorType::Link => spans.extend(render_link_spans(&node.content)),
            MarkdownProcessorType::Image => spans.extend(render_image_spans(&node.content)),
            MarkdownProcessorType::Strikethrough => spans.push(Span::styled(
                strip_pair(&node.content, "~~").unwrap_or_else(|| node.content.clone()),
                base_style.fg(theme::TEXT_SUBTLE),
            )),
            MarkdownProcessorType::Underline => spans.push(Span::styled(
                strip_pair(&node.content, "__").unwrap_or_else(|| node.content.clone()),
                base_style.add_modifier(Modifier::UNDERLINED),
            )),
            MarkdownProcessorType::InlineLatex => spans.push(Span::styled(
                strip_inline_latex_delimiters(&node.content),
                Style::default().fg(theme::ACCENT_STRONG),
            )),
            MarkdownProcessorType::PlainText | MarkdownProcessorType::HtmlBreak => {
                spans.push(Span::styled(node.content.clone(), base_style))
            }
            _ => spans.push(Span::styled(node.content.clone(), base_style)),
        }
    }
    if spans.is_empty() {
        spans.push(Span::raw(""));
    }
    spans
}

fn render_link_spans(content: &str) -> Vec<Span<'static>> {
    let Some((label, url)) = parse_markdown_link(content) else {
        return vec![Span::styled(content.to_string(), Style::default())];
    };
    vec![
        Span::styled(
            label,
            Style::default()
                .fg(theme::ACCENT_STRONG)
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(format!(" ({url})"), Style::default().fg(theme::TEXT_SUBTLE)),
    ]
}

fn render_image_spans(content: &str) -> Vec<Span<'static>> {
    let text = content.strip_prefix('!').unwrap_or(content);
    let Some((label, url)) = parse_markdown_link(text) else {
        return vec![Span::styled(
            content.to_string(),
            Style::default().fg(theme::ACCENT_STRONG),
        )];
    };
    vec![
        Span::styled(
            format!("[image: {label}]"),
            Style::default()
                .fg(theme::ACCENT_STRONG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {url}"), Style::default().fg(theme::TEXT_SUBTLE)),
    ]
}

fn render_plain_lines(content: &str, style: Style) -> Vec<Line<'static>> {
    content
        .trim()
        .lines()
        .map(|line| Line::from(Span::styled(line.to_string(), style)))
        .collect::<Vec<_>>()
}

fn compact_plain_text_lines(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    let mut out = Vec::with_capacity(lines.len());
    let mut previous_blank = false;
    for line in trim_empty_edge_lines(lines) {
        let blank = line_is_blank(&line);
        if blank && previous_blank {
            continue;
        }
        previous_blank = blank;
        out.push(line);
    }
    out
}

fn trim_empty_edge_lines(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    let start = lines
        .iter()
        .position(|line| !line_is_blank(line))
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .rposition(|line| !line_is_blank(line))
        .map(|index| index + 1)
        .unwrap_or(start);
    lines[start..end].to_vec()
}

fn line_is_blank(line: &Line<'static>) -> bool {
    line.spans.iter().all(|span| span.content.trim().is_empty())
}

fn split_spans_by_newline(spans: Vec<Span<'static>>) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut current = Vec::new();
    for span in spans {
        let style = span.style;
        let value = span.content.to_string();
        let parts = value.split('\n').collect::<Vec<_>>();
        for (index, part) in parts.iter().enumerate() {
            if index > 0 {
                lines.push(Line::from(std::mem::take(&mut current)));
            }
            if !part.is_empty() {
                current.push(Span::styled((*part).to_string(), style));
            }
        }
    }
    lines.push(Line::from(current));
    lines
}

fn split_ordered_marker(content: &str) -> (String, String) {
    let trimmed = content.trim_start();
    let Some(dot) = trimmed.find('.') else {
        return ("1. ".to_string(), trimmed.to_string());
    };
    if trimmed[..dot].chars().all(|ch| ch.is_ascii_digit()) {
        let text = trimmed
            .get(dot + 1..)
            .unwrap_or("")
            .trim_start()
            .to_string();
        (format!("{}. ", &trimmed[..dot]), text)
    } else {
        ("1. ".to_string(), trimmed.to_string())
    }
}

fn strip_unordered_marker(content: &str) -> String {
    let trimmed = content.trim_start();
    for marker in ["- ", "* ", "+ "] {
        if let Some(value) = trimmed.strip_prefix(marker) {
            return value.to_string();
        }
    }
    trimmed.to_string()
}

fn strip_block_quote_marker(content: &str) -> String {
    content
        .lines()
        .map(|line| {
            line.trim_start()
                .strip_prefix('>')
                .map(str::trim_start)
                .unwrap_or(line)
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_table_separator(line: &str) -> bool {
    line.trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().replace(':', ""))
        .all(|cell| !cell.is_empty() && cell.chars().all(|ch| ch == '-'))
}

fn parse_markdown_link(content: &str) -> Option<(String, String)> {
    let close_label = content.find("](")?;
    let label = content
        .strip_prefix('[')?
        .get(..close_label - 1)?
        .to_string();
    let url_start = close_label + 2;
    let url_end = content[url_start..].find(')')? + url_start;
    Some((label, content[url_start..url_end].to_string()))
}

fn strip_pair(content: &str, delimiter: &str) -> Option<String> {
    content
        .strip_prefix(delimiter)
        .and_then(|value| value.strip_suffix(delimiter))
        .map(ToString::to_string)
}

fn strip_latex_block_delimiters(content: &str) -> String {
    let trimmed = content.trim();
    if let Some(value) = strip_pair(trimmed, "$$") {
        return value.trim().to_string();
    }
    if trimmed.starts_with("\\[") && trimmed.ends_with("\\]") {
        return trimmed[2..trimmed.len().saturating_sub(2)]
            .trim()
            .to_string();
    }
    trimmed.to_string()
}

fn strip_inline_latex_delimiters(content: &str) -> String {
    if let Some(value) = strip_pair(content, "$") {
        return value;
    }
    if content.starts_with("\\(") && content.ends_with("\\)") {
        return content[2..content.len().saturating_sub(2)].to_string();
    }
    content.to_string()
}
