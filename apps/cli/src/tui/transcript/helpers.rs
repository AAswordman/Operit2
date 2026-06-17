use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::collections::HashSet;

use operit_runtime::data::model::ChatMessage::ChatMessage;
use operit_runtime::data::model::InputProcessingState::InputProcessingState;
use operit_runtime::util::stream::HotStream::SharedStream;
use operit_runtime::util::streamnative::NativeMarkdownSplitter::{
    MarkdownNodeStable, MarkdownProcessorType,
};
use operit_runtime::util::ChatMarkupRegex::{attr_value, tag_body, tag_ranges, ChatMarkupRegex};

use super::empty_state::render_blue_cat_lines;
use super::markdown::{
    render_markdown_lines, render_markdown_lines_cached, render_markdown_nodes_lines,
    render_markdown_nodes_lines_cached, MarkdownRenderCache,
};
use super::selection::mark_soft_wrap_continuation;
use super::theme;
use super::typewriter::TypewriterState;

pub(super) fn render_message_lines(
    messages: &[ChatMessage],
    content_width: usize,
    is_loading: bool,
    input_state: &InputProcessingState,
    thinking_line: &Line<'static>,
    typewriter_state: &mut TypewriterState,
) -> Vec<Line<'static>> {
    if messages.is_empty() {
        return render_blue_cat_lines(content_width);
    }

    let active_message_timestamps = messages
        .iter()
        .map(|message| message.timestamp)
        .collect::<HashSet<_>>();
    typewriter_state.retain_messages(&active_message_timestamps);

    let mut lines = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        append_message_gap(&mut lines);
        lines.extend(render_transcript_message_lines(
            message,
            index,
            messages.len(),
            content_width,
            is_loading,
            thinking_line,
            typewriter_state,
        ));
    }
    if is_loading && matches!(messages.last(), Some(message) if message.sender == "user") {
        append_message_gap(&mut lines);
        lines.extend(render_loading_ai_placeholder_lines(
            content_width,
            thinking_line,
        ));
    }
    lines.extend(render_input_error_lines(input_state));
    lines
}

pub(super) fn render_transcript_message_lines(
    message: &ChatMessage,
    index: usize,
    messages_len: usize,
    content_width: usize,
    is_loading: bool,
    thinking_line: &Line<'static>,
    typewriter_state: &mut TypewriterState,
) -> Vec<Line<'static>> {
    render_transcript_message_lines_with_cache(
        message,
        index,
        messages_len,
        content_width,
        is_loading,
        thinking_line,
        typewriter_state,
        None,
        None,
    )
}

pub(super) fn render_transcript_message_lines_with_cache(
    message: &ChatMessage,
    index: usize,
    messages_len: usize,
    content_width: usize,
    is_loading: bool,
    thinking_line: &Line<'static>,
    typewriter_state: &mut TypewriterState,
    stream_markdown_nodes: Option<&[MarkdownNodeStable]>,
    mut markdown_cache: Option<&mut MarkdownRenderCache>,
) -> Vec<Line<'static>> {
    let role = message.roleName.trim();
    let sender = message_header_label(message.sender.as_str(), role);
    let color = message_header_color(message.sender.as_str());
    let block_style = message_block_style(message.sender.as_str());
    let message_layout = message_layout(message.sender.as_str(), content_width);
    let message_content_width = message_layout.content_width;
    let mut meta = String::new();
    if !message.provider.trim().is_empty() {
        meta.push_str(&message.provider);
    }
    if !message.modelName.trim().is_empty() {
        if !meta.is_empty() {
            meta.push_str(" / ");
        }
        meta.push_str(&message.modelName);
    }
    if message.outputTokens > 0 {
        if !meta.is_empty() {
            meta.push_str(" / ");
        }
        meta.push_str(&format!("out={}", message.outputTokens));
    }
    let header_spans = if meta.is_empty() {
        vec![Span::styled(
            sender,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )]
    } else {
        vec![
            Span::styled(
                format!("{sender} "),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(meta, Style::default().fg(theme::TEXT_MUTED)),
        ]
    };
    let mut lines = Vec::new();
    if message.sender == "user" {
        append_user_message_card(&mut lines, header_spans, &message.content, content_width);
        return lines;
    }
    lines.push(style_message_line(
        Line::from(header_spans),
        message.sender.as_str(),
        block_style,
        message_layout,
    ));
    let is_streaming_message =
        is_streaming_message_for_tui(message, index, messages_len, is_loading);
    let full_content = if is_streaming_message && stream_markdown_nodes.is_some() {
        message.content.clone()
    } else if is_streaming_message {
        message
            .contentStream
            .as_ref()
            .map(|stream| stream.replay_cache().join(""))
            .unwrap_or_else(|| message.content.clone())
    } else if message.content.is_empty() {
        String::new()
    } else {
        message.content.clone()
    };
    let split_content = split_thinking_for_tui(&full_content);
    let display_is_thinking = split_content.visible.trim().is_empty() && split_content.has_thinking;
    let display_content = if display_is_thinking {
        if split_content.thinking.trim().is_empty() {
            String::new()
        } else {
            format!("<thinking>{}</thinking>", split_content.thinking.trim())
        }
    } else if stream_markdown_nodes.is_some() && is_streaming_message {
        split_content.visible.trim().to_string()
    } else {
        let typewriter_frame = typewriter_state.frame(
            message.timestamp,
            &split_content.visible,
            is_streaming_message,
        );
        let mut rendered_content = typewriter_frame.content;
        if let Some(pending_char) = typewriter_frame.pending_char {
            if pending_char == '\n' || !pending_char.is_control() {
                rendered_content.push(pending_char);
            }
        }
        rendered_content.trim().to_string()
    };
    if display_content.is_empty() && display_is_thinking && message.sender == "ai" {
        lines.push(style_message_line(
            thinking_line.clone(),
            message.sender.as_str(),
            block_style,
            message_layout,
        ));
    } else if display_content.is_empty()
        && is_loading
        && index + 1 == messages_len
        && message.sender == "ai"
    {
        lines.push(style_message_line(
            thinking_line.clone(),
            message.sender.as_str(),
            block_style,
            message_layout,
        ));
    } else {
        let rendered_lines = render_message_markdown_lines(
            &display_content,
            message_content_width,
            is_streaming_message,
            stream_markdown_nodes,
            markdown_cache.as_deref_mut(),
        );
        lines.extend(
            wrap_message_lines(rendered_lines, message_content_width)
                .into_iter()
                .map(|line| {
                    style_message_line(line, message.sender.as_str(), block_style, message_layout)
                }),
        );
        if display_content.is_empty() {
            lines.push(style_message_line(
                Line::from(""),
                message.sender.as_str(),
                block_style,
                message_layout,
            ));
        }
    }
    lines
}

fn render_message_markdown_lines(
    display_content: &str,
    message_content_width: usize,
    is_streaming_message: bool,
    stream_markdown_nodes: Option<&[MarkdownNodeStable]>,
    markdown_cache: Option<&mut MarkdownRenderCache>,
) -> Vec<Line<'static>> {
    if let Some(nodes) = stream_markdown_nodes {
        let visible_nodes = visible_markdown_nodes_for_tui(nodes);
        return match markdown_cache {
            Some(cache) => render_markdown_nodes_lines_cached(
                &visible_nodes,
                message_content_width,
                cache,
                is_streaming_message,
            ),
            None => render_markdown_nodes_lines(&visible_nodes, message_content_width),
        };
    }

    match markdown_cache {
        Some(cache) => render_markdown_lines_cached(
            display_content,
            message_content_width,
            cache,
            is_streaming_message,
        ),
        None => render_markdown_lines(display_content, message_content_width),
    }
}

pub(super) fn render_loading_ai_placeholder_lines(
    content_width: usize,
    thinking_line: &Line<'static>,
) -> Vec<Line<'static>> {
    let block_style = message_block_style("ai");
    vec![
        style_message_line(
            Line::from(Span::styled(
                "Operit",
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )),
            "ai",
            block_style,
            message_layout("ai", content_width),
        ),
        style_message_line(
            thinking_line.clone(),
            "ai",
            block_style,
            message_layout("ai", content_width),
        ),
        Line::from(""),
    ]
}

pub(super) fn render_input_error_lines(input_state: &InputProcessingState) -> Vec<Line<'static>> {
    match input_state {
        InputProcessingState::Error { message } => vec![Line::from(vec![
            Span::styled(
                "error: ",
                Style::default()
                    .fg(theme::ERROR)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(message.clone(), Style::default().fg(theme::ERROR_DIM)),
        ])],
        _ => Vec::new(),
    }
}

pub(super) fn is_streaming_message_for_tui(
    message: &ChatMessage,
    index: usize,
    messages_len: usize,
    is_loading: bool,
) -> bool {
    message.sender == "ai"
        && (message.contentStream.is_some() || (is_loading && index + 1 == messages_len))
}

fn visible_markdown_nodes_for_tui(nodes: &[MarkdownNodeStable]) -> Vec<MarkdownNodeStable> {
    nodes
        .iter()
        .filter(|node| !is_thinking_markdown_node(node))
        .cloned()
        .collect()
}

fn is_thinking_markdown_node(node: &MarkdownNodeStable) -> bool {
    if node.r#type != MarkdownProcessorType::XmlBlock {
        return false;
    }
    let raw_tag = ChatMarkupRegex::extract_opening_tag_name(&node.content);
    matches!(
        ChatMarkupRegex::normalize_tool_like_tag_name(raw_tag.as_deref()).as_deref(),
        Some("think") | Some("thinking")
    )
}

struct TuiThinkingSplit {
    visible: String,
    thinking: String,
    has_thinking: bool,
}

fn split_thinking_for_tui(content: &str) -> TuiThinkingSplit {
    let ranges = ChatMarkupRegex::think_ranges(content);
    let mut thinking_parts = Vec::new();
    for &(start, end) in &ranges {
        let raw = &content[start..end];
        let tag_name = ChatMarkupRegex::extract_opening_tag_name(raw)
            .unwrap_or_else(|| "thinking".to_string());
        let body = tag_body(raw, &tag_name)
            .or_else(|| raw.split_once('>').map(|(_, body)| body))
            .unwrap_or("")
            .trim();
        if !body.is_empty() {
            thinking_parts.push(body.to_string());
        }
    }

    let mut visible = remove_content_ranges(content, &ranges);
    let mut has_thinking = !ranges.is_empty();
    if let Some(open_start) = find_thinking_open_start(&visible) {
        has_thinking = true;
        let open = visible[open_start..].to_string();
        if let Some((_, body)) = open.split_once('>') {
            let trimmed = body.trim();
            if !trimmed.is_empty() {
                thinking_parts.push(trimmed.to_string());
            }
        }
        visible.truncate(open_start);
    }

    TuiThinkingSplit {
        visible: visible.trim().to_string(),
        thinking: thinking_parts.join("\n"),
        has_thinking,
    }
}

fn remove_content_ranges(content: &str, ranges: &[(usize, usize)]) -> String {
    if ranges.is_empty() {
        return content.to_string();
    }
    let mut result = String::new();
    let mut cursor = 0;
    for &(start, end) in ranges {
        if start >= cursor {
            result.push_str(&content[cursor..start]);
            cursor = end;
        }
    }
    result.push_str(&content[cursor..]);
    result
}

fn find_thinking_open_start(content: &str) -> Option<usize> {
    content
        .match_indices('<')
        .find_map(|(index, _)| is_thinking_open_at(content, index).then_some(index))
}

fn is_thinking_open_at(content: &str, start: usize) -> bool {
    let tail = &content[start..];
    tag_open_matches(tail, "thinking") || tag_open_matches(tail, "think")
}

fn tag_open_matches(tail: &str, name: &str) -> bool {
    let prefix = format!("<{name}");
    if !tail.starts_with(&prefix) {
        return false;
    }
    matches!(
        tail.as_bytes().get(prefix.len()).copied(),
        Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n')
    )
}

fn message_header_label(sender: &str, role: &str) -> String {
    match sender {
        "user" => "Prompt".to_string(),
        "ai" if role.is_empty() => "Operit".to_string(),
        _ if role.is_empty() => sender.to_string(),
        _ => role.to_string(),
    }
}

fn message_header_color(sender: &str) -> Color {
    match sender {
        "user" => theme::TEXT_MUTED,
        "ai" => theme::ACCENT,
        _ => theme::ACCENT_STRONG,
    }
}

fn message_block_style(sender: &str) -> Style {
    match sender {
        "user" => Style::default().bg(theme::USER_CARD_BG),
        _ => Style::default(),
    }
}

#[derive(Clone, Copy)]
struct MessageLayout {
    outer_indent: usize,
    inner_padding: usize,
    block_width: usize,
    content_width: usize,
}

fn message_layout(sender: &str, available_width: usize) -> MessageLayout {
    match sender {
        "user" => {
            let outer_indent = 2usize.min(available_width);
            let block_width = available_width
                .saturating_sub(outer_indent.saturating_mul(2))
                .max(1);
            let inner_padding = 2usize.min(block_width.saturating_sub(1));
            let content_width = block_width
                .saturating_sub(inner_padding.saturating_mul(2))
                .max(1);
            MessageLayout {
                outer_indent,
                inner_padding,
                block_width,
                content_width,
            }
        }
        _ => {
            let outer_indent = 2usize.min(available_width);
            let content_width = available_width
                .saturating_sub(outer_indent.saturating_mul(2))
                .max(1);
            MessageLayout {
                outer_indent,
                inner_padding: 0,
                block_width: content_width,
                content_width,
            }
        }
    }
}

fn style_message_line(
    mut line: Line<'static>,
    _sender: &str,
    block_style: Style,
    layout: MessageLayout,
) -> Line<'static> {
    let outer_indent = " ".repeat(layout.outer_indent);
    if !outer_indent.is_empty() {
        line.spans.insert(0, Span::raw(outer_indent));
    }
    line.style = block_style;
    line
}

fn append_message_gap(lines: &mut Vec<Line<'static>>) {
    if !lines.is_empty() {
        lines.push(Line::from(""));
    }
}

fn append_user_message_card(
    lines: &mut Vec<Line<'static>>,
    header_spans: Vec<Span<'static>>,
    content: &str,
    available_width: usize,
) {
    let layout = message_layout("user", available_width);
    let block_style = message_block_style("user");
    let parsed = parse_user_message_content(content);
    if !parsed.trailing_attachments.is_empty() {
        for line in render_attachment_chip_lines(&parsed.trailing_attachments, layout) {
            lines.push(line);
        }
    }
    lines.push(style_user_card_line(Line::from(""), block_style, layout));
    lines.push(style_user_card_line(
        Line::from(header_spans),
        block_style,
        layout,
    ));

    let mut rendered_lines = render_markdown_lines(&parsed.processed_text, layout.content_width);
    trim_blank_edge_lines(&mut rendered_lines);
    if rendered_lines.is_empty() {
        rendered_lines.push(Line::from(""));
    }
    for line in wrap_message_lines(rendered_lines, layout.content_width) {
        lines.push(style_user_card_line(line, block_style, layout));
    }
    lines.push(style_user_card_line(Line::from(""), block_style, layout));
}

#[derive(Clone, Debug)]
struct UserMessageParseResult {
    processed_text: String,
    trailing_attachments: Vec<UserAttachmentData>,
}

#[derive(Clone, Debug)]
struct UserAttachmentData {
    file_name: String,
    mime_type: String,
    file_size: i64,
}

fn parse_user_message_content(content: &str) -> UserMessageParseResult {
    let mut cleaned_content = remove_tag_blocks(content, "memory").trim().to_string();
    cleaned_content = remove_proxy_sender_tag(&cleaned_content).trim().to_string();
    cleaned_content = remove_tag_blocks(&cleaned_content, "reply_to")
        .trim()
        .to_string();

    let mut trailing_attachments = Vec::new();
    let workspace_ranges = ChatMarkupRegex::workspace_attachment_ranges(&cleaned_content);
    if let Some((start, end)) = workspace_ranges.first().copied() {
        trailing_attachments.push(UserAttachmentData {
            file_name: "工作区状态".to_string(),
            mime_type: "application/vnd.workspace-context+xml".to_string(),
            file_size: (end - start) as i64,
        });
        cleaned_content.replace_range(start..end, "");
        cleaned_content = cleaned_content.trim().to_string();
    }

    let attachment_ranges = ChatMarkupRegex::attachment_ranges(&cleaned_content);
    if attachment_ranges.is_empty() {
        return UserMessageParseResult {
            processed_text: cleaned_content,
            trailing_attachments,
        };
    }

    let mut attachment_matches = Vec::new();
    for (start, end) in attachment_ranges {
        let raw = &cleaned_content[start..end];
        let file_name = attr_value(raw, "filename").unwrap_or_else(String::new);
        let mime_type = attr_value(raw, "type").unwrap_or_else(String::new);
        let file_size = attr_value(raw, "size")
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(0);
        attachment_matches.push((
            start,
            end,
            UserAttachmentData {
                file_name,
                mime_type,
                file_size,
            },
        ));
    }

    let mut trailing_indices = HashSet::new();
    if let Some((last_start, last_end, _)) = attachment_matches.last() {
        if cleaned_content[*last_end..].trim().is_empty() {
            trailing_indices.insert(attachment_matches.len() - 1);
            for index in (0..attachment_matches.len() - 1).rev() {
                let (_, current_end, _) = attachment_matches[index];
                let (next_start, _, _) = attachment_matches[index + 1];
                if cleaned_content[current_end..next_start].trim().is_empty() {
                    trailing_indices.insert(index);
                } else {
                    break;
                }
            }
        } else {
            let _ = last_start;
        }
    }

    let first_trailing_index = trailing_indices.iter().min().copied();
    let mut message_text = String::new();
    let mut last_index = 0usize;
    let mut parsed_trailing = Vec::new();
    for (index, (start, end, attachment)) in attachment_matches.into_iter().enumerate() {
        let is_trailing = trailing_indices.contains(&index)
            || (attachment.mime_type == "text/json"
                && attachment.file_name == "screen_content.json");
        if start > last_index {
            let text_before = &cleaned_content[last_index..start];
            if !is_trailing || Some(index) == first_trailing_index {
                message_text.push_str(text_before);
            }
        }
        if is_trailing {
            parsed_trailing.push(attachment);
        } else {
            message_text.push('@');
            message_text.push_str(&attachment.file_name);
        }
        last_index = end;
    }
    if last_index < cleaned_content.len() {
        message_text.push_str(&cleaned_content[last_index..]);
    }

    trailing_attachments.extend(parsed_trailing);
    UserMessageParseResult {
        processed_text: message_text.trim().to_string(),
        trailing_attachments,
    }
}

fn remove_tag_blocks(content: &str, tag_name: &str) -> String {
    let mut ranges = tag_ranges(content, tag_name);
    ranges.sort_by_key(|range| range.0);
    remove_ranges(content, &ranges)
}

fn remove_proxy_sender_tag(content: &str) -> String {
    let Some(start) = content.to_ascii_lowercase().find("<proxy_sender") else {
        return content.to_string();
    };
    let Some(relative_end) = content[start..].find("/>") else {
        return content.to_string();
    };
    let end = start + relative_end + 2;
    remove_ranges(content, &[(start, end)])
}

fn remove_ranges(content: &str, ranges: &[(usize, usize)]) -> String {
    if ranges.is_empty() {
        return content.to_string();
    }
    let mut output = String::new();
    let mut last_index = 0usize;
    for (start, end) in ranges.iter().copied() {
        if start > last_index {
            output.push_str(&content[last_index..start]);
        }
        last_index = end;
    }
    if last_index < content.len() {
        output.push_str(&content[last_index..]);
    }
    output
}

fn render_attachment_chip_lines(
    attachments: &[UserAttachmentData],
    layout: MessageLayout,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut current_width = 0usize;
    let content_width = layout.content_width.max(1);
    for attachment in attachments {
        let label = attachment_display_label(attachment);
        let chip = format!("[{}]", label);
        let chip_width = display_width(&chip);
        let separator_width = if spans.is_empty() { 0 } else { 1 };
        if !spans.is_empty() && current_width + separator_width + chip_width > content_width {
            lines.push(left_aligned_attachment_line(
                std::mem::take(&mut spans),
                layout,
            ));
            current_width = 0;
        }
        if !spans.is_empty() {
            spans.push(Span::raw(" "));
            current_width += 1;
        }
        spans.push(Span::styled(
            chip,
            Style::default()
                .fg(theme::TEXT)
                .bg(theme::USER_CARD_BG)
                .add_modifier(Modifier::BOLD),
        ));
        current_width += chip_width;
    }
    if !spans.is_empty() {
        lines.push(left_aligned_attachment_line(spans, layout));
    }
    lines
}

fn left_aligned_attachment_line(
    mut spans: Vec<Span<'static>>,
    layout: MessageLayout,
) -> Line<'static> {
    let padding = layout.outer_indent;
    if padding > 0 {
        spans.insert(0, Span::raw(" ".repeat(padding)));
    }
    Line::from(spans)
}

fn attachment_display_label(attachment: &UserAttachmentData) -> String {
    if attachment.mime_type == "text/json" && attachment.file_name == "screen_content.json" {
        "屏幕内容".to_string()
    } else if attachment.mime_type == "application/vnd.workspace-context+xml" {
        "工作区状态".to_string()
    } else if attachment.file_size > 0 {
        format!(
            "{} {}",
            attachment.file_name,
            format_file_size(attachment.file_size)
        )
    } else {
        attachment.file_name.clone()
    }
}

fn format_file_size(size: i64) -> String {
    if size >= 1024 * 1024 {
        format!("{:.1}MB", size as f64 / (1024.0 * 1024.0))
    } else if size >= 1024 {
        format!("{:.1}KB", size as f64 / 1024.0)
    } else {
        format!("{}B", size)
    }
}

fn style_user_card_line(
    line: Line<'static>,
    block_style: Style,
    layout: MessageLayout,
) -> Line<'static> {
    let mut spans = Vec::new();
    if layout.outer_indent > 0 {
        spans.push(Span::raw(" ".repeat(layout.outer_indent)));
    }
    if layout.inner_padding > 0 {
        spans.push(Span::styled(" ".repeat(layout.inner_padding), block_style));
    }
    spans.extend(
        line.spans
            .into_iter()
            .map(|span| span.patch_style(block_style)),
    );
    if layout.inner_padding > 0 {
        spans.push(Span::styled(" ".repeat(layout.inner_padding), block_style));
    }
    let visible_width = spans
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum::<usize>();
    let target_width = layout.outer_indent + layout.block_width;
    if visible_width < target_width {
        spans.push(Span::styled(
            " ".repeat(target_width - visible_width),
            block_style,
        ));
    }
    Line::from(spans)
}

fn wrap_message_lines(lines: Vec<Line<'static>>, width: usize) -> Vec<Line<'static>> {
    let width = width.max(1);
    let mut wrapped = Vec::new();
    for line in lines {
        let mut current = Vec::new();
        let mut current_width = 0usize;
        let mut emitted_count = 0usize;
        for span in line.spans {
            let style = span.style;
            let mut text = String::new();
            for ch in span.content.chars() {
                let ch_width = char_display_width(ch);
                if current_width > 0 && current_width + ch_width > width {
                    push_wrapped_span(&mut current, &mut text, style);
                    push_wrapped_line(&mut wrapped, &mut current, &mut emitted_count);
                    current_width = 0;
                }
                text.push(ch);
                current_width += ch_width;
                if current_width >= width {
                    push_wrapped_span(&mut current, &mut text, style);
                    push_wrapped_line(&mut wrapped, &mut current, &mut emitted_count);
                    current_width = 0;
                }
            }
            push_wrapped_span(&mut current, &mut text, style);
        }
        if !current.is_empty() {
            push_wrapped_line(&mut wrapped, &mut current, &mut emitted_count);
        } else if emitted_count == 0 {
            wrapped.push(Line::from(""));
        }
    }
    wrapped
}

fn push_wrapped_line(
    wrapped: &mut Vec<Line<'static>>,
    current: &mut Vec<Span<'static>>,
    emitted_count: &mut usize,
) {
    let mut line = Line::from(std::mem::take(current));
    if *emitted_count > 0 {
        mark_soft_wrap_continuation(&mut line);
    }
    wrapped.push(line);
    *emitted_count += 1;
}

fn push_wrapped_span(current: &mut Vec<Span<'static>>, text: &mut String, style: Style) {
    if !text.is_empty() {
        current.push(Span::styled(std::mem::take(text), style));
    }
}

fn trim_blank_edge_lines(lines: &mut Vec<Line<'static>>) {
    let start = lines
        .iter()
        .position(|line| !line_is_blank(line))
        .unwrap_or(lines.len());
    if start > 0 {
        lines.drain(0..start);
    }
    let end = lines
        .iter()
        .rposition(|line| !line_is_blank(line))
        .map(|index| index + 1)
        .unwrap_or(0);
    lines.truncate(end);
}

fn line_is_blank(line: &Line<'_>) -> bool {
    line.spans.iter().all(|span| span.content.trim().is_empty())
}

pub(super) fn transcript_max_scroll(lines: &[Line<'_>], area: Rect) -> u16 {
    let content_lines = lines.len() as u16;
    let viewport = area.height.saturating_sub(2);
    content_lines.saturating_sub(viewport)
}

pub(super) fn display_width(value: &str) -> usize {
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

pub(super) fn short_chat_label(chat_id: &str) -> String {
    chat_id.chars().take(8).collect()
}

pub(super) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub(super) fn char_to_byte_index(value: &str, char_index: usize) -> usize {
    match value.char_indices().nth(char_index) {
        Some((index, _)) => index,
        None => value.len(),
    }
}

pub(super) fn wrap_approx_lines(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let raw_lines = text.split('\n').collect::<Vec<_>>();
    for (raw_index, raw_line) in raw_lines.iter().enumerate() {
        let is_last_raw_line = raw_index + 1 == raw_lines.len();
        if raw_line.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        let mut width_count = 0usize;
        for ch in raw_line.chars() {
            let char_width = char_display_width(ch);
            if width_count > 0 && width_count + char_width > width {
                lines.push(current);
                current = String::new();
                width_count = 0;
            }
            current.push(ch);
            width_count += char_width;
            if width_count >= width {
                lines.push(current);
                current = String::new();
                width_count = 0;
            }
        }
        if !current.is_empty() {
            lines.push(current);
        } else if is_last_raw_line && width_count == 0 {
            lines.push(String::new());
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

pub(super) fn split_command_line(input: &str) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote = None::<char>;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match quote {
            Some(active_quote) => {
                if ch == active_quote {
                    quote = None;
                } else if ch == '\\' && active_quote == '"' {
                    match chars.next() {
                        Some(next) => current.push(next),
                        None => current.push('\\'),
                    }
                } else {
                    current.push(ch);
                }
            }
            None => match ch {
                '"' | '\'' => quote = Some(ch),
                '\\' => match chars.next() {
                    Some(next) => current.push(next),
                    None => current.push('\\'),
                },
                ch if ch.is_whitespace() => {
                    if !current.is_empty() {
                        parts.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(ch),
            },
        }
    }
    if quote.is_some() {
        return Err("unterminated quote".to_string());
    }
    if !current.is_empty() {
        parts.push(current);
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::text::Text;
    use ratatui::widgets::{Block, Borders, Paragraph};
    use ratatui::Terminal;

    const PREVIEW_WIDTH: u16 = 70;
    const PREVIEW_HEIGHT: u16 = 22;
    const USER_CARD_BG: Color = theme::USER_CARD_BG;

    #[test]
    fn user_message_workspace_attachment_is_rendered_as_attachment_chip() {
        let parsed =
            parse_user_message_content("你好 <workspace_attachment></workspace_attachment>");

        assert_eq!(parsed.processed_text, "你好");
        assert_eq!(parsed.trailing_attachments.len(), 1);
        assert_eq!(parsed.trailing_attachments[0].file_name, "工作区状态");
        assert_eq!(
            parsed.trailing_attachments[0].mime_type,
            "application/vnd.workspace-context+xml"
        );
    }

    #[test]
    fn user_message_workspace_attachment_is_rendered_above_user_card() {
        let mut user = ChatMessage::new_with_timestamp(
            "user".to_string(),
            "你好 <workspace_attachment></workspace_attachment>".to_string(),
            1,
        );
        user.roleName = String::new();
        let mut typewriter_state = TypewriterState::default();
        let lines = render_message_lines(
            &[user],
            48,
            false,
            &InputProcessingState::Idle,
            &Line::from("thinking"),
            &mut typewriter_state,
        );
        let rendered = dump_logical_lines(&lines);

        let attachment_index = rendered.find("[工作区状态]").expect("attachment chip");
        let prompt_index = rendered.find("Prompt").expect("prompt header");
        assert!(attachment_index < prompt_index);
        assert!(!rendered.contains("<workspace_attachment"));
    }

    #[test]
    fn workspace_attachment_chip_aligns_with_user_card_background_start() {
        let layout = message_layout("user", 48);
        let block_style = message_block_style("user");
        let attachment_line = render_attachment_chip_lines(
            &[UserAttachmentData {
                file_name: "工作区状态".to_string(),
                mime_type: "application/vnd.workspace-context+xml".to_string(),
                file_size: 1,
            }],
            layout,
        )
        .remove(0);
        let card_line = style_user_card_line(Line::from("Prompt"), block_style, layout);

        assert_eq!(
            first_user_card_bg_column(&attachment_line),
            first_user_card_bg_column(&card_line)
        );
    }

    #[test]
    fn ai_thinking_block_is_hidden_from_transcript() {
        let mut ai = ChatMessage::new_with_timestamp(
            "ai".to_string(),
            "<thinking>内部推理</thinking>\n你好！".to_string(),
            1,
        );
        ai.provider = "DEEPSEEK".to_string();
        ai.modelName = "deepseek-v4-flash".to_string();
        let mut typewriter_state = TypewriterState::default();
        let lines = render_message_lines(
            &[ai],
            48,
            false,
            &InputProcessingState::Idle,
            &Line::from("thinking"),
            &mut typewriter_state,
        );
        let rendered = dump_logical_lines(&lines);

        assert!(rendered.contains("你好"));
        assert!(!rendered.contains("内部推理"));
        assert!(!rendered.contains("<thinking>"));
    }

    #[test]
    fn streaming_open_thinking_block_shows_thinking_content() {
        let ai =
            ChatMessage::new_with_timestamp("ai".to_string(), "<thinking>内部推理".to_string(), 1);
        let mut typewriter_state = TypewriterState::default();
        let lines = render_message_lines(
            &[ai],
            48,
            true,
            &InputProcessingState::Idle,
            &Line::from("thinking"),
            &mut typewriter_state,
        );
        let rendered = dump_logical_lines(&lines);

        assert!(rendered.contains("thinking"));
        assert!(rendered.contains("内部推理"));
    }

    #[test]
    #[ignore = "debug-only TUI preview; run with --ignored --nocapture"]
    fn tui_user_card_preview() {
        let mut user = ChatMessage::new_with_timestamp("user".to_string(), "你好".to_string(), 1);
        user.roleName = String::new();

        let mut ai = ChatMessage::new_with_timestamp(
            "ai".to_string(),
            "你好！\n我是Operit，一个全能AI助手，很高兴为你服务！\n\n我可以帮你完成各种任务，比如：\n- 文件管理 - 浏览、创建、编辑、删除文件\n- 网页访问 - 查看网页内容、下载文件\n- 代码搜索 - 在项目中查找特定代码\n有什么我可以帮你的吗？"
                .to_string(),
            2,
        );
        ai.provider = "DEEPSEEK".to_string();
        ai.modelName = "deepseek-v4-flash".to_string();
        ai.outputTokens = 104;

        let messages = vec![user, ai];
        let mut typewriter_state = TypewriterState::default();
        let lines = render_message_lines(
            &messages,
            PREVIEW_WIDTH.saturating_sub(2) as usize,
            false,
            &InputProcessingState::Idle,
            &Line::from("thinking"),
            &mut typewriter_state,
        );

        println!("logical lines:");
        println!("{}", dump_logical_lines(&lines));

        let backend = TestBackend::new(PREVIEW_WIDTH, PREVIEW_HEIGHT);
        let mut terminal = Terminal::new(backend).expect("create test terminal");
        terminal
            .draw(|frame| {
                let paragraph = Paragraph::new(Text::from(lines))
                    .block(Block::default().title("Conversation").borders(Borders::ALL));
                frame.render_widget(paragraph, frame.area());
            })
            .expect("draw preview");

        println!("screen:");
        println!("{}", dump_buffer_screen(terminal.backend_mut().buffer()));
        println!("user-card background mask (# means user card bg):");
        println!(
            "{}",
            dump_buffer_background_mask(terminal.backend_mut().buffer())
        );
    }

    fn dump_logical_lines(lines: &[Line<'static>]) -> String {
        let mut out = String::new();
        for (index, line) in lines.iter().enumerate() {
            let content = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>();
            let has_user_bg = line
                .spans
                .iter()
                .any(|span| span.style.bg == Some(USER_CARD_BG));
            out.push_str(&format!(
                "{index:02} bg={} {:?}\n",
                if has_user_bg { "user" } else { "none" },
                content
            ));
        }
        out
    }

    fn dump_buffer_screen(buffer: &Buffer) -> String {
        dump_buffer(buffer, |cell| {
            let symbol = cell.symbol();
            if symbol.is_empty() {
                ' '
            } else {
                symbol.chars().next().unwrap_or(' ')
            }
        })
    }

    fn dump_buffer_background_mask(buffer: &Buffer) -> String {
        dump_buffer(
            buffer,
            |cell| {
                if cell.bg == USER_CARD_BG {
                    '#'
                } else {
                    '.'
                }
            },
        )
    }

    fn first_user_card_bg_column(line: &Line<'static>) -> usize {
        let mut column = 0usize;
        for span in &line.spans {
            if span.style.bg == Some(USER_CARD_BG) {
                return column;
            }
            column += display_width(span.content.as_ref());
        }
        column
    }

    fn dump_buffer<F>(buffer: &Buffer, render_cell: F) -> String
    where
        F: Fn(&ratatui::buffer::Cell) -> char,
    {
        let mut out = String::new();
        let width = buffer.area.width;
        let height = buffer.area.height;
        for y in 0..height {
            for x in 0..width {
                let index = y as usize * width as usize + x as usize;
                out.push(render_cell(&buffer.content[index]));
            }
            out.push('\n');
        }
        out
    }
}
