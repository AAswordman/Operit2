use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use operit_runtime::data::model::ChatMessage::ChatMessage;
use operit_runtime::util::stream::HotStream::SharedStream;

use super::markdown::render_markdown_lines;

pub(super) fn render_message_lines(messages: &[ChatMessage]) -> Vec<Line<'static>> {
    if messages.is_empty() {
        return vec![Line::from(Span::styled(
            "No messages yet. Type below to start.",
            Style::default().fg(Color::DarkGray),
        ))];
    }

    let mut lines = Vec::new();
    for message in messages {
        let role = message.roleName.trim();
        let sender = if role.is_empty() {
            message.sender.as_str()
        } else {
            role
        };
        let color = match message.sender.as_str() {
            "user" => Color::Green,
            "ai" => Color::Cyan,
            _ => Color::Magenta,
        };
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
        lines.push(Line::from(vec![
            Span::styled(
                format!("{sender}: "),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(meta, Style::default().fg(Color::DarkGray)),
        ]));
        let rendered_content = if message.content.is_empty() {
            message
                .contentStream
                .as_ref()
                .map(|stream| stream.replay_cache().join(""))
                .unwrap_or_default()
        } else {
            message.content.clone()
        };
        lines.extend(render_markdown_lines(&rendered_content));
        if rendered_content.is_empty() {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(""));
    }
    lines
}

pub(super) fn transcript_max_scroll(lines: &[Line<'_>], area: Rect) -> u16 {
    let content_lines = lines.len() as u16;
    let viewport = area.height.saturating_sub(2);
    content_lines.saturating_sub(viewport)
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
