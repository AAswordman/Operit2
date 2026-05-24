use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::collections::HashSet;

use operit_runtime::data::model::ChatMessage::ChatMessage;
use operit_runtime::data::model::InputProcessingState::InputProcessingState;
use operit_runtime::util::stream::HotStream::SharedStream;

use super::empty_state::render_blue_cat_lines;
use super::markdown::render_markdown_lines;
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
        if message.sender == "user" {
            append_message_gap(&mut lines);
            append_user_message_card(&mut lines, header_spans, &message.content, content_width);
            continue;
        }
        append_message_gap(&mut lines);
        lines.push(style_message_line(
            Line::from(header_spans),
            message.sender.as_str(),
            block_style,
            message_layout,
        ));
        let is_streaming_message = message.sender == "ai"
            && (message.contentStream.is_some() || (is_loading && index + 1 == messages.len()));
        let full_content = if is_streaming_message {
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
        let typewriter_frame =
            typewriter_state.frame(message.timestamp, &full_content, is_streaming_message);
        let rendered_content = typewriter_frame.content;
        if rendered_content.is_empty()
            && is_loading
            && index + 1 == messages.len()
            && message.sender == "ai"
        {
            lines.push(style_message_line(
                thinking_line.clone(),
                message.sender.as_str(),
                block_style,
                message_layout,
            ));
        } else {
            let mut rendered_lines = render_markdown_lines(&rendered_content, message_content_width);
            append_pending_char(&mut rendered_lines, typewriter_frame.pending_char);
            lines.extend(wrap_message_lines(rendered_lines, message_content_width).into_iter().map(|line| {
                style_message_line(line, message.sender.as_str(), block_style, message_layout)
            }));
            if rendered_content.is_empty() {
                lines.push(style_message_line(
                    Line::from(""),
                    message.sender.as_str(),
                    block_style,
                    message_layout,
                ));
            }
        }
    }
    if is_loading
        && matches!(messages.last(), Some(message) if message.sender == "user")
    {
        let block_style = message_block_style("ai");
        append_message_gap(&mut lines);
        lines.push(style_message_line(
            Line::from(Span::styled(
                "Operit",
                Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
            )),
            "ai",
            block_style,
            message_layout("ai", content_width),
        ));
        lines.push(style_message_line(
            thinking_line.clone(),
            "ai",
            block_style,
            message_layout("ai", content_width),
        ));
        lines.push(Line::from(""));
    }
    if let InputProcessingState::Error { message } = input_state {
        lines.push(Line::from(vec![
            Span::styled("error: ", Style::default().fg(theme::ERROR).add_modifier(Modifier::BOLD)),
            Span::styled(message.clone(), Style::default().fg(theme::ERROR_DIM)),
        ]));
    }
    lines
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
    lines.push(style_user_card_line(Line::from(""), block_style, layout));
    lines.push(style_user_card_line(
        Line::from(header_spans),
        block_style,
        layout,
    ));

    let mut rendered_lines = render_markdown_lines(content, layout.content_width);
    trim_blank_edge_lines(&mut rendered_lines);
    if rendered_lines.is_empty() {
        rendered_lines.push(Line::from(""));
    }
    for line in wrap_message_lines(rendered_lines, layout.content_width) {
        lines.push(style_user_card_line(line, block_style, layout));
    }
    lines.push(style_user_card_line(Line::from(""), block_style, layout));
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
        spans.push(Span::styled(
            " ".repeat(layout.inner_padding),
            block_style,
        ));
    }
    spans.extend(
        line.spans
            .into_iter()
            .map(|span| span.patch_style(block_style)),
    );
    if layout.inner_padding > 0 {
        spans.push(Span::styled(
            " ".repeat(layout.inner_padding),
            block_style,
        ));
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
        let mut emitted = false;
        for span in line.spans {
            let style = span.style;
            let mut text = String::new();
            for ch in span.content.chars() {
                let ch_width = char_display_width(ch);
                if current_width > 0 && current_width + ch_width > width {
                    push_wrapped_span(&mut current, &mut text, style);
                    wrapped.push(Line::from(std::mem::take(&mut current)));
                    current_width = 0;
                    emitted = true;
                }
                text.push(ch);
                current_width += ch_width;
                if current_width >= width {
                    push_wrapped_span(&mut current, &mut text, style);
                    wrapped.push(Line::from(std::mem::take(&mut current)));
                    current_width = 0;
                    emitted = true;
                }
            }
            push_wrapped_span(&mut current, &mut text, style);
        }
        if !current.is_empty() {
            wrapped.push(Line::from(current));
        } else if !emitted {
            wrapped.push(Line::from(""));
        }
    }
    wrapped
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
    line.spans
        .iter()
        .all(|span| span.content.trim().is_empty())
}

fn append_pending_char(lines: &mut Vec<Line<'static>>, pending_char: Option<char>) {
    let Some(pending_char) = pending_char else {
        return;
    };
    if pending_char == '\n' {
        lines.push(Line::from(Span::styled(
            " ".to_string(),
            Style::default().fg(theme::TEXT_SUBTLE),
        )));
        return;
    }
    if pending_char.is_control() {
        return;
    }
    let pending_span = Span::styled(
        pending_char.to_string(),
        Style::default()
            .fg(theme::TEXT_SUBTLE)
            .add_modifier(Modifier::DIM),
    );
    match lines.last_mut() {
        Some(line) => line.spans.push(pending_span),
        None => lines.push(Line::from(pending_span)),
    }
}

pub(super) fn transcript_max_scroll(lines: &[Line<'_>], area: Rect) -> u16 {
    let content_lines = lines.len() as u16;
    let viewport = area.height.saturating_sub(2);
    content_lines.saturating_sub(viewport)
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
    for raw_line in text.split('\n') {
        if raw_line.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        let mut count = 0usize;
        for ch in raw_line.chars() {
            current.push(ch);
            count += 1;
            if count >= width {
                lines.push(current);
                current = String::new();
                count = 0;
            }
        }
        if !current.is_empty() {
            lines.push(current);
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
    #[ignore = "debug-only TUI preview; run with --ignored --nocapture"]
    fn tui_user_card_preview() {
        let mut user = ChatMessage::new_with_timestamp(
            "user".to_string(),
            "你好".to_string(),
            1,
        );
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
        println!("{}", dump_buffer_background_mask(terminal.backend_mut().buffer()));
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
        dump_buffer(buffer, |cell| {
            if cell.bg == USER_CARD_BG {
                '#'
            } else {
                '.'
            }
        })
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
