use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub(super) fn render_blue_cat_lines(content_width: usize) -> Vec<Line<'static>> {
    let cat_style = Style::default()
        .fg(Color::LightBlue)
        .add_modifier(Modifier::BOLD);
    let hint_style = Style::default().fg(Color::DarkGray);
    let cat_lines = [
        r" /\_/\ ",
        r"( o.o )",
        r" > ^ < ",
    ];

    let mut lines = Vec::new();
    lines.push(Line::from(""));
    for line in cat_lines {
        lines.push(centered_styled_line(line, content_width, cat_style));
    }
    lines.push(Line::from(""));
    lines.push(centered_styled_line(
        "Type a message to start.",
        content_width,
        hint_style,
    ));
    lines
}

fn centered_styled_line(content: &str, content_width: usize, style: Style) -> Line<'static> {
    let padding = content_width.saturating_sub(content.chars().count()) / 2;
    Line::from(vec![
        Span::raw(" ".repeat(padding)),
        Span::styled(content.to_string(), style),
    ])
}
