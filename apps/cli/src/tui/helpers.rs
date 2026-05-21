use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use operit_runtime::data::model::ChatMessage::ChatMessage;
use operit_runtime::data::model::InputProcessingState::InputProcessingState;
use operit_runtime::util::stream::HotStream::SharedStream;

use super::empty_state::render_blue_cat_lines;
use super::markdown::render_markdown_lines;

pub(super) fn render_message_lines(
    messages: &[ChatMessage],
    content_width: usize,
    is_loading: bool,
    input_state: &InputProcessingState,
    thinking_text: &str,
) -> Vec<Line<'static>> {
    if messages.is_empty() {
        return render_blue_cat_lines(content_width);
    }

    let mut lines = Vec::new();
    for (index, message) in messages.iter().enumerate() {
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
        if rendered_content.is_empty()
            && is_loading
            && index + 1 == messages.len()
            && message.sender == "ai"
        {
            lines.push(Line::from(Span::styled(
                thinking_text.to_string(),
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            )));
        } else {
            lines.extend(render_markdown_lines(&rendered_content, content_width));
            if rendered_content.is_empty() {
                lines.push(Line::from(""));
            }
        }
        lines.push(Line::from(""));
    }
    if is_loading
        && matches!(messages.last(), Some(message) if message.sender == "user")
    {
        lines.push(Line::from(Span::styled(
            "Operit: ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            thinking_text.to_string(),
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        )));
        lines.push(Line::from(""));
    }
    if let InputProcessingState::Error { message } = input_state {
        lines.push(Line::from(vec![
            Span::styled("error: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(message.clone(), Style::default().fg(Color::LightRed)),
        ]));
    }
    lines
}

pub(super) fn transcript_max_scroll(lines: &[Line<'_>], area: Rect) -> u16 {
    let content_width = area.width.saturating_sub(2).max(1) as usize;
    let content_lines = lines
        .iter()
        .map(|line| wrapped_visual_line_count(line, content_width))
        .sum::<usize>() as u16;
    let viewport = area.height.saturating_sub(2);
    content_lines.saturating_sub(viewport)
}

fn wrapped_visual_line_count(line: &Line<'_>, content_width: usize) -> usize {
    let width = line
        .spans
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum::<usize>();
    width.max(1).div_ceil(content_width)
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
