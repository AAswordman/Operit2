use operit_runtime::util::ChatMarkupRegex::{attr_value, tag_body, ChatMarkupRegex};
use operit_runtime::util::streamnative::NativeMarkdownSplitter::{
    MarkdownNodeStable, MarkdownProcessorType,
};
use operit_runtime::util::streamnative::NativeMarkdownStreamOperators::NativeMarkdownStreamOperators;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

const TOOL_CALL_INLINE_DETAIL_CHAR_LIMIT: usize = 160;
const TOOL_RESULT_INLINE_DETAIL_CHAR_LIMIT: usize = 80;
const TOOL_RESULT_PREFIX_DISPLAY_WIDTH: usize = 8;

pub(super) fn render_markdown_lines(content: &str, content_width: usize) -> Vec<Line<'static>> {
    let nodes = content.nativeMarkdownSplitByBlock();
    let mut lines = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        if is_blank_block_between_tool_blocks(&nodes, index) {
            continue;
        }
        render_block_node(&node, content_width, &mut lines);
    }
    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines
}

fn is_blank_block_between_tool_blocks(nodes: &[MarkdownNodeStable], index: usize) -> bool {
    let Some(node) = nodes.get(index) else {
        return false;
    };
    if !is_blank_text_block(node) {
        return false;
    }
    let previous_is_tool = previous_non_blank_node(nodes, index).map(is_tool_xml_block).unwrap_or(false);
    let next_is_tool = next_non_blank_node(nodes, index).map(is_tool_xml_block).unwrap_or(false);
    previous_is_tool && next_is_tool
}

fn is_blank_text_block(node: &MarkdownNodeStable) -> bool {
    matches!(
        node.r#type,
        MarkdownProcessorType::PlainText | MarkdownProcessorType::HtmlBreak
    ) && node.content.trim().is_empty()
        && node.children.iter().all(is_blank_text_block)
}

fn previous_non_blank_node(nodes: &[MarkdownNodeStable], index: usize) -> Option<&MarkdownNodeStable> {
    nodes
        .get(..index)?
        .iter()
        .rev()
        .find(|node| !is_blank_text_block(node))
}

fn next_non_blank_node(nodes: &[MarkdownNodeStable], index: usize) -> Option<&MarkdownNodeStable> {
    nodes
        .get(index + 1..)?
        .iter()
        .find(|node| !is_blank_text_block(node))
}

fn is_tool_xml_block(node: &MarkdownNodeStable) -> bool {
    if node.r#type != MarkdownProcessorType::XmlBlock {
        return false;
    }
    let raw_tag = ChatMarkupRegex::extract_opening_tag_name(&node.content);
    matches!(
        ChatMarkupRegex::normalize_tool_like_tag_name(raw_tag.as_deref()).as_deref(),
        Some("tool") | Some("tool_result")
    )
}

fn render_block_node(node: &MarkdownNodeStable, content_width: usize, lines: &mut Vec<Line<'static>>) {
    match node.r#type {
        MarkdownProcessorType::Header => render_header(node, lines),
        MarkdownProcessorType::BlockQuote => render_block_quote(node, lines),
        MarkdownProcessorType::CodeBlock => render_code_block(&node.content, lines),
        MarkdownProcessorType::OrderedList => render_list_block(node, true, lines),
        MarkdownProcessorType::UnorderedList => render_list_block(node, false, lines),
        MarkdownProcessorType::HorizontalRule => lines.push(Line::from(Span::styled(
            "--------------------------------",
            Style::default().fg(Color::DarkGray),
        ))),
        MarkdownProcessorType::BlockLatex => render_latex_block(&node.content, lines),
        MarkdownProcessorType::Table => render_table_block(&node.content, lines),
        MarkdownProcessorType::XmlBlock => render_xml_block(&node.content, content_width, lines),
        MarkdownProcessorType::Image => lines.extend(render_inline_nodes(&[node.clone()], Style::default())),
        MarkdownProcessorType::PlainText | MarkdownProcessorType::HtmlBreak => {
            lines.extend(render_inline_nodes(&node.children, Style::default()));
            if node.children.is_empty() {
                lines.extend(render_plain_lines(&node.content, Style::default()));
            }
        }
        _ => lines.extend(render_inline_nodes(&[node.clone()], Style::default())),
    }
}

fn render_header(node: &MarkdownNodeStable, lines: &mut Vec<Line<'static>>) {
    let trimmed = node.content.trim_start();
    let level = trimmed.chars().take_while(|ch| *ch == '#').count().clamp(1, 6);
    let text = trimmed.get(level..).unwrap_or("").trim_start();
    let prefix = "#".repeat(level.min(4));
    let mut spans = vec![Span::styled(
        format!("{prefix} "),
        Style::default().fg(Color::DarkGray),
    )];
    let inline_nodes = text.nativeMarkdownSplitByInline();
    spans.extend(render_inline_spans(
        &inline_nodes,
        Style::default()
            .fg(Color::Cyan)
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
        Style::default().fg(Color::DarkGray),
        Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC),
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
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
    )));
    for raw in iter {
        if raw.trim_start().starts_with("```") {
            continue;
        }
        lines.push(Line::from(Span::styled(
            format!("  {raw}"),
            Style::default().fg(Color::LightYellow).bg(Color::Black),
        )));
    }
    lines.push(Line::from(Span::styled(
        "```",
        Style::default().fg(Color::DarkGray),
    )));
}

fn render_list_block(node: &MarkdownNodeStable, ordered: bool, lines: &mut Vec<Line<'static>>) {
    let (marker, text) = if ordered {
        split_ordered_marker(&node.content)
    } else {
        ("- ".to_string(), strip_unordered_marker(&node.content))
    };
    let inline_nodes = text.trim_end().nativeMarkdownSplitByInline();
    let mut spans = vec![Span::styled(marker, Style::default().fg(Color::Cyan))];
    spans.extend(render_inline_spans(&inline_nodes, Style::default()));
    lines.push(Line::from(spans));
}

fn render_latex_block(content: &str, lines: &mut Vec<Line<'static>>) {
    lines.push(Line::from(Span::styled(
        "$$",
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
    )));
    for raw in strip_latex_block_delimiters(content).lines() {
        lines.push(Line::from(Span::styled(
            raw.to_string(),
            Style::default().fg(Color::LightMagenta),
        )));
    }
    lines.push(Line::from(Span::styled("$$", Style::default().fg(Color::DarkGray))));
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
                Style::default().fg(Color::DarkGray),
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
                spans.push(Span::styled(" | ".to_string(), Style::default().fg(Color::DarkGray)));
            }
            let inline_nodes = cell.nativeMarkdownSplitByInline();
            spans.extend(render_inline_spans(
                &inline_nodes,
                Style::default().fg(Color::Gray),
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
        Some("error") => render_error_xml(content, lines),
        Some("think") | Some("thinking") => render_named_xml_body("thinking", content, lines),
        Some("status") => render_status_xml(content, lines),
        Some("meta") => {}
        Some(name) => render_named_xml_body(name, content, lines),
        None => lines.extend(render_plain_lines(content, Style::default().fg(Color::DarkGray))),
    }
}

fn render_tool_xml(content: &str, is_result: bool, content_width: usize, lines: &mut Vec<Line<'static>>) {
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
    let summary = render_tool_param_summary(&params, body);
    let mut header = vec![
        Span::styled("*".to_string(), Style::default().fg(Color::Cyan)),
        Span::raw(" "),
        Span::styled(name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ];
    let summary = compact_tool_summary(&summary, TOOL_CALL_INLINE_DETAIL_CHAR_LIMIT);
    if !summary.is_empty() {
        header.push(Span::styled(" ".to_string(), Style::default()));
        header.push(Span::styled(summary, Style::default().fg(Color::DarkGray)));
    }
    lines.push(Line::from(header));
}

fn render_tool_result_xml(
    _name: &str,
    status: Option<&str>,
    body: &str,
    content_width: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let content = extract_first_tag_body(body, "content").unwrap_or(body).trim();
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
        Span::styled("↳".to_string(), Style::default().fg(if is_error {
            Color::Red
        } else {
            Color::DarkGray
        })),
        Span::raw(" "),
        Span::styled(
            if is_error { "×" } else { "✓" }.to_string(),
            Style::default().fg(if is_error { Color::Red } else { Color::DarkGray }),
        ),
        Span::raw(" "),
    ];
    if !result.is_empty() {
        header.push(Span::styled(
            result.clone(),
            Style::default().fg(Color::DarkGray),
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

fn compact_tool_summary(value: &str, char_limit: usize) -> String {
    let normalized = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
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
    let label = title.or(status_type).unwrap_or_else(|| "status".to_string());
    lines.push(Line::from(vec![
        Span::styled("* ".to_string(), Style::default().fg(Color::DarkGray)),
        Span::styled(label, Style::default().fg(Color::Gray)),
    ]));
}

fn render_error_xml(content: &str, lines: &mut Vec<Line<'static>>) {
    let body = xml_unescape(
        tag_body(content, "error").expect("error xml block must contain an error body"),
    )
        .trim()
        .to_string();
    lines.push(Line::from(vec![
        Span::styled("error: ".to_string(), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled(body, Style::default().fg(Color::LightRed)),
    ]));
}

fn render_named_xml_body(name: &str, content: &str, lines: &mut Vec<Line<'static>>) {
    let tag_name = ChatMarkupRegex::extract_opening_tag_name(content).unwrap_or_else(|| name.to_string());
    let body = tag_body(content, &tag_name).unwrap_or(content).trim();
    lines.push(Line::from(Span::styled(
        format!("<{name}>"),
        Style::default().fg(Color::DarkGray),
    )));
    if !body.is_empty() {
        lines.extend(render_plain_lines(body, Style::default().fg(Color::Gray)));
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
                Style::default().fg(Color::LightYellow).bg(Color::Black),
            )),
            MarkdownProcessorType::Link => spans.extend(render_link_spans(&node.content)),
            MarkdownProcessorType::Image => spans.extend(render_image_spans(&node.content)),
            MarkdownProcessorType::Strikethrough => {
                spans.push(Span::styled(
                    strip_pair(&node.content, "~~").unwrap_or_else(|| node.content.clone()),
                    base_style.fg(Color::DarkGray),
                ))
            }
            MarkdownProcessorType::Underline => spans.push(Span::styled(
                strip_pair(&node.content, "__").unwrap_or_else(|| node.content.clone()),
                base_style.add_modifier(Modifier::UNDERLINED),
            )),
            MarkdownProcessorType::InlineLatex => spans.push(Span::styled(
                strip_inline_latex_delimiters(&node.content),
                Style::default().fg(Color::LightMagenta),
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
                .fg(Color::LightBlue)
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(format!(" ({url})"), Style::default().fg(Color::DarkGray)),
    ]
}

fn render_image_spans(content: &str) -> Vec<Span<'static>> {
    let text = content.strip_prefix('!').unwrap_or(content);
    let Some((label, url)) = parse_markdown_link(text) else {
        return vec![Span::styled(content.to_string(), Style::default().fg(Color::LightMagenta))];
    };
    vec![
        Span::styled(
            format!("[image: {label}]"),
            Style::default().fg(Color::LightMagenta).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {url}"), Style::default().fg(Color::DarkGray)),
    ]
}

fn render_plain_lines(content: &str, style: Style) -> Vec<Line<'static>> {
    content
        .lines()
        .map(|line| Line::from(Span::styled(line.to_string(), style)))
        .collect::<Vec<_>>()
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
        let text = trimmed.get(dot + 1..).unwrap_or("").trim_start().to_string();
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
    let label = content.strip_prefix('[')?.get(..close_label - 1)?.to_string();
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
        return trimmed[2..trimmed.len().saturating_sub(2)].trim().to_string();
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
