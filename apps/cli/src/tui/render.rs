use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::app::{FocusArea, OperitTui};
use super::helpers::{
    centered_rect, render_message_lines, short_chat_label, transcript_max_scroll,
};

const INPUT_PROMPT: &str = "> ";

impl OperitTui {
    pub(super) fn render(&mut self, frame: &mut Frame) {
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.area());

        self.render_header(frame, root[0]);

        let main_area = if self.show_chat_list {
            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(28), Constraint::Min(0)])
                .split(root[1]);
            self.render_chat_list(frame, body[0]);
            body[1]
        } else {
            root[1]
        };

        let main = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(main_area);

        self.render_transcript(frame, main[0]);
        self.render_input(frame, main[1]);
        self.render_footer(frame, root[2]);
        self.render_command_popup(frame, main[1]);

        if self.show_help {
            self.render_help_modal(frame);
        }
    }

    fn render_header(&mut self, frame: &mut Frame, area: Rect) {
        let current_chat_id = self.current_chat_id().unwrap_or_default();
        let focus_label = match self.focus {
            FocusArea::Chats => "chats",
            FocusArea::Input => "input",
        };
        let attachment_count = self.queued_attachment_paths.len();
        let chat_list_label = if self.show_chat_list { "shown" } else { "hidden" };
        let spans = Line::from(vec![
            Span::styled(" Operit2 ", Style::default().fg(Color::Black).bg(Color::Cyan)),
            Span::raw(" "),
            Span::styled(
                format!("chat={} ", short_chat_label(&current_chat_id)),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("focus={} ", focus_label)),
            Span::raw(format!("chats={} ", chat_list_label)),
            Span::raw(format!("attachments={} ", attachment_count)),
        ]);
        frame.render_widget(Paragraph::new(spans), area);
    }

    fn render_chat_list(&self, frame: &mut Frame, area: Rect) {
        let items = if self.chats.is_empty() {
            vec![ListItem::new(Line::from("no chats"))]
        } else {
            self.chats
                .iter()
                .map(|item| {
                    ListItem::new(vec![
                        Line::from(Span::styled(
                            item.title.clone(),
                            Style::default().add_modifier(Modifier::BOLD),
                        )),
                        Line::from(Span::styled(
                            item.secondary.clone(),
                            Style::default().fg(Color::DarkGray),
                        )),
                    ])
                })
                .collect::<Vec<_>>()
        };

        let border_style = if self.focus == FocusArea::Chats {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };
        let list = List::new(items)
            .block(Block::default().title("Chats").borders(Borders::ALL).border_style(border_style))
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");
        let mut state = ListState::default();
        if !self.chats.is_empty() {
            state.select(Some(self.selected_chat_index.min(self.chats.len().saturating_sub(1))));
        }
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn render_transcript(&mut self, frame: &mut Frame, area: Rect) {
        let messages = self.current_messages();
        let transcript_lines = render_message_lines(&messages);
        let max_scroll = transcript_max_scroll(&transcript_lines, area);
        self.transcript_viewport_height = area.height.saturating_sub(2).max(1);
        self.transcript_max_scroll = max_scroll;
        if self.follow_transcript {
            self.transcript_scroll = max_scroll;
        } else if self.transcript_scroll > max_scroll {
            self.transcript_scroll = max_scroll;
            self.follow_transcript = true;
        }

        let paragraph = Paragraph::new(Text::from(transcript_lines))
            .block(Block::default().title("Conversation").borders(Borders::ALL))
            .wrap(Wrap { trim: false })
            .scroll((self.transcript_scroll, 0));
        frame.render_widget(paragraph, area);
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focus == FocusArea::Input {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);
        let inner = input_block.inner(area);
        let prompt_width = INPUT_PROMPT.chars().count();
        let text_width = inner
            .width
            .saturating_sub(prompt_width as u16)
            .saturating_sub(1) as usize;
        let visible_text = self.input_view_text(text_width, inner.height as usize);
        let rendered_text = format!("{INPUT_PROMPT}{visible_text}");
        let input = Paragraph::new(rendered_text)
            .block(input_block)
            .wrap(Wrap { trim: false });
        frame.render_widget(input, area);

        if self.focus == FocusArea::Input && !self.show_help {
            let (cursor_x, cursor_y) = self.cursor_position(text_width, inner.height as usize);
            frame.set_cursor_position((
                inner.x + prompt_width as u16 + cursor_x as u16,
                inner.y + cursor_y as u16,
            ));
        }
    }

    fn render_command_popup(&self, frame: &mut Frame, input_area: Rect) {
        if self.show_help || self.focus != FocusArea::Input {
            return;
        }
        let suggestions = self.command_suggestions();
        if suggestions.is_empty() {
            return;
        }
        let visible_count = (suggestions.len() as u16).min(6) as usize;
        let popup_height = visible_count as u16 + 2;
        let y = input_area.y.saturating_sub(popup_height);
        if y == input_area.y {
            return;
        }
        let width = input_area.width.min(76);
        let area = Rect {
            x: input_area.x,
            y,
            width,
            height: popup_height,
        };
        let selected = self.selected_command_index(suggestions.len());
        let first_visible = selected.saturating_add(1).saturating_sub(visible_count);
        let items = suggestions
            .iter()
            .enumerate()
            .skip(first_visible)
            .take(visible_count)
            .map(|(index, spec)| {
                let style = if index == selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(spec.usage.to_string(), style),
                    Span::styled(
                        format!("  {}", spec.description),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect::<Vec<_>>();
        let popup = List::new(items)
            .block(Block::default().title("Commands").borders(Borders::ALL))
            .highlight_symbol("");
        frame.render_widget(Clear, area);
        frame.render_widget(popup, area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let text = if self.status_message.is_empty() {
            "Ready".to_string()
        } else {
            self.status_message.clone()
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {text}"),
                Style::default().fg(Color::DarkGray),
            ))),
            area,
        );
    }

    fn render_help_modal(&self, frame: &mut Frame) {
        let popup = centered_rect(72, 60, frame.area());
        frame.render_widget(Clear, popup);
        let lines = vec![
            Line::from(Span::styled(
                "Operit2 TUI",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("Tab: complete command / switch focus"),
            Line::from("Enter: send message / activate selected chat"),
            Line::from("F3 or /switch: toggle chat list"),
            Line::from("Up/Down: select chat when chat list is focused"),
            Line::from("Ctrl+J: insert newline in input"),
            Line::from("Ctrl+N: create new chat"),
            Line::from("Ctrl+C: press twice to quit"),
            Line::from("Ctrl+Q: quit"),
            Line::from("PageUp/PageDown: scroll conversation by page"),
            Line::from("Ctrl+U/Ctrl+D: scroll conversation by half page"),
            Line::from("Ctrl+Home/Ctrl+End: top / bottom conversation"),
            Line::from("Esc: close help / clear status"),
            Line::from(""),
            Line::from("Local commands:"),
            Line::from("/help"),
            Line::from("/new [--character <name>] [--group-card <id>] [--group <name>]"),
            Line::from("/switch"),
            Line::from("/attach <path>"),
            Line::from("/attachments"),
            Line::from("/clear-attachments"),
            Line::from("/quit"),
        ];
        let help = Paragraph::new(Text::from(lines))
            .block(Block::default().title("Help").borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        frame.render_widget(help, popup);
    }
}
