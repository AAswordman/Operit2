use operit_runtime::util::ChatMarkupRegex::{attr_value, tag_body, ChatMarkupRegex};
use operit_runtime::util::streamnative::NativeMarkdownSplitter::{
    MarkdownNodeStable, MarkdownProcessorType,
};
use operit_runtime::util::streamnative::NativeMarkdownStreamOperators::NativeMarkdownStreamOperators;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub(super) fn render_markdown_lines(content: &str) -> Vec<Line<'static>> {
    let nodes = content.nativeMarkdownSplitByBlock();
    let mut lines = Vec::new();
    for node in nodes {
        render_block_node(&node, &mut lines);
    }
    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines
}

fn render_block_node(node: &MarkdownNodeStable, lines: &mut Vec<Line<'static>>) {
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
        MarkdownProcessorType::XmlBlock => render_xml_block(&node.content, lines),
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

fn render_xml_block(content: &str, lines: &mut Vec<Line<'static>>) {
    let raw_tag = ChatMarkupRegex::extract_opening_tag_name(content);
    let tag = ChatMarkupRegex::normalize_tool_like_tag_name(raw_tag.as_deref());
    match tag.as_deref() {
        Some("tool") => render_tool_xml(content, false, lines),
        Some("tool_result") => render_tool_xml(content, true, lines),
        Some("think") | Some("thinking") => render_named_xml_body("thinking", content, lines),
        Some("status") => render_status_xml(content, lines),
        Some("meta") => {}
        Some(name) => render_named_xml_body(name, content, lines),
        None => lines.extend(render_plain_lines(content, Style::default().fg(Color::DarkGray))),
    }
}

fn render_tool_xml(content: &str, is_result: bool, lines: &mut Vec<Line<'static>>) {
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
    let symbol = if is_result { "->" } else { "*" };
    let mut header = vec![
        Span::styled(symbol.to_string(), Style::default().fg(Color::Cyan)),
        Span::raw(" "),
        Span::styled(name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ];
    if let Some(status) = status {
        header.push(Span::styled(
            format!(" [{status}]"),
            Style::default().fg(Color::DarkGray),
        ));
    }
    lines.push(Line::from(header));
    if !body.is_empty() {
        for raw in body.lines() {
            lines.push(Line::from(vec![
                Span::styled("  ".to_string(), Style::default().fg(Color::DarkGray)),
                Span::styled(raw.to_string(), Style::default().fg(Color::Gray)),
            ]));
        }
    }
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
