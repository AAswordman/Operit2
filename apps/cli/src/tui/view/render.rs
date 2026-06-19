use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use std::time::{SystemTime, UNIX_EPOCH};

use super::app::{FocusArea, FullUpdateDownloadState, OperitTui};
use super::helpers::{
    centered_rect, display_width, short_chat_label, transcript_max_scroll, wrap_approx_lines,
};
use super::pending_queue::{
    pending_queue_preview_text, pending_queue_visible_items, pending_queue_visible_range,
};
use super::selection::{apply_transcript_selection, transcript_copy_line};
use super::theme;
use super::transcript::render_transcript_lines;

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

        let input_height = self.input_panel_height(main_area.width, main_area.height);
        let main = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(input_height)])
            .split(main_area);

        self.render_transcript(frame, main[0]);
        self.render_input(frame, main[1]);
        self.render_footer(frame, root[2]);
        self.render_command_popup(frame, main[1]);

        if self.show_model_chooser {
            self.render_model_chooser(frame);
        }

        if self.show_help {
            self.render_help_modal(frame);
        }

        if self.startup_update_prompt.is_some() {
            self.render_startup_update_prompt(frame);
        }

        if self.startup_workspace_prompt.is_some() && self.startup_update_prompt.is_none() {
            self.render_startup_workspace_prompt(frame);
        }

        if self.approval_bridge.current().is_some() {
            self.render_approval_modal(frame);
        }
    }

    fn render_header(&mut self, frame: &mut Frame, area: Rect) {
        let current_chat_id = self.current_chat_id().unwrap_or_default();
        let title = self
            .chats
            .iter()
            .find(|item| item.id == current_chat_id)
            .map(|item| item.title.as_str())
            .unwrap_or(self.text().new_chat_title());
        let spans = Line::from(vec![
            Span::styled(
                format!(" {} ", short_chat_label(&current_chat_id)),
                Style::default().fg(theme::TEXT_INVERTED).bg(theme::ACCENT),
            ),
            Span::raw(" "),
            Span::styled(
                title.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]);
        frame.render_widget(Paragraph::new(spans), area);
    }

    fn render_chat_list(&self, frame: &mut Frame, area: Rect) {
        let items = if self.chats.is_empty() {
            vec![ListItem::new(Line::from(self.text().no_chats()))]
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
                            Style::default().fg(theme::TEXT_SUBTLE),
                        )),
                    ])
                })
                .collect::<Vec<_>>()
        };

        let border_style = if self.focus == FocusArea::Chats {
            Style::default().fg(theme::ACCENT)
        } else {
            Style::default()
        };
        let list = List::new(items)
            .block(
                Block::default()
                    .title(self.text().chats_title())
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(
                Style::default()
                    .bg(theme::ACCENT_BG)
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");
        let mut state = ListState::default();
        if !self.chats.is_empty() {
            state.select(Some(
                self.selected_chat_index
                    .min(self.chats.len().saturating_sub(1)),
            ));
        }
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn render_transcript(&mut self, frame: &mut Frame, area: Rect) {
        self.transcript_area = area;
        let messages = self.current_messages();
        let is_loading = self.current_chat_is_loading();
        let input_state = self.current_chat_input_processing_state();
        let text = self.text();
        let thinking_line = thinking_indicator_line(text.thinking());
        let content_width = area.width.saturating_sub(2).max(1) as usize;
        let current_chat_id = self.current_chat_id_cache.clone();
        let stream_markdown_state = current_chat_id
            .as_ref()
            .and_then(|chat_id| self.response_stream_markdown_by_chat_id.get(chat_id));
        let mut transcript_lines = render_transcript_lines(
            &messages,
            current_chat_id.as_deref(),
            is_loading,
            &input_state,
            &thinking_line,
            content_width,
            &mut self.typewriter_state,
            &mut self.transcript_render_cache,
            stream_markdown_state,
            text,
        );
        self.transcript_copy_lines = transcript_lines.iter().map(transcript_copy_line).collect();
        apply_transcript_selection(&mut transcript_lines, &self.transcript_selection);
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
            .block(
                Block::default()
                    .title(self.text().conversation_title())
                    .borders(Borders::ALL),
            )
            .scroll((self.transcript_scroll, 0));
        frame.render_widget(paragraph, area);
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let border_style = match self.focus {
            FocusArea::Input => Style::default().fg(theme::ACCENT),
            FocusArea::Queue => Style::default().fg(theme::SELECTION_BG),
            _ => Style::default(),
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
        let queue_lines = self.pending_queue_lines(inner.width as usize);
        let queue_height = queue_lines.len() as u16;
        let input_height = inner.height.saturating_sub(queue_height).max(1) as usize;
        let visible_text = self.input_view_text(text_width, input_height);
        let prompt_indent = " ".repeat(prompt_width);
        let mut rendered_lines = queue_lines;
        rendered_lines.extend(visible_text.split('\n').enumerate().map(|(index, line)| {
            if index == 0 {
                Line::from(format!("{INPUT_PROMPT}{line}"))
            } else {
                Line::from(format!("{prompt_indent}{line}"))
            }
        }));
        let input = Paragraph::new(Text::from(rendered_lines))
            .block(input_block)
            .wrap(Wrap { trim: false });
        frame.render_widget(input, area);

        if self.focus == FocusArea::Input && !self.show_help {
            let (cursor_x, cursor_y) = self.cursor_position(text_width, input_height);
            frame.set_cursor_position((
                inner.x + prompt_width as u16 + cursor_x as u16,
                inner.y + queue_height + cursor_y as u16,
            ));
        }
    }

    fn pending_queue_lines(&self, width: usize) -> Vec<Line<'static>> {
        if self.pending_queue_messages.is_empty() {
            return Vec::new();
        }
        let mut lines = vec![Line::from(Span::styled(
            self.text().queue_title(self.pending_queue_messages.len()),
            Style::default()
                .fg(theme::TEXT_SUBTLE)
                .add_modifier(Modifier::BOLD),
        ))];
        let visible_range = pending_queue_visible_range(
            self.pending_queue_messages.len(),
            self.selected_pending_queue_index,
        );
        for (index, message) in self
            .pending_queue_messages
            .iter()
            .enumerate()
            .skip(visible_range.start)
            .take(visible_range.len())
        {
            let prefix = format!(" #{} ", message.id);
            let preview_width = width.saturating_sub(display_width(&prefix)).max(1);
            let preview = pending_queue_preview_text(&message.text, preview_width);
            let selected =
                self.focus == FocusArea::Queue && index == self.selected_pending_queue_index;
            let line_width = display_width(&prefix) + display_width(&preview);
            let padding = " ".repeat(width.saturating_sub(line_width));
            if selected {
                let selected_style = Style::default()
                    .fg(theme::SELECTION_TEXT)
                    .bg(theme::SELECTION_BG);
                lines.push(Line::from(vec![
                    Span::styled(prefix, selected_style),
                    Span::styled(preview, selected_style),
                    Span::styled(padding, selected_style),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(theme::ACCENT_STRONG)),
                    Span::styled(preview, Style::default().fg(theme::TEXT_MUTED)),
                    Span::raw(padding),
                ]));
            }
        }
        if self.pending_queue_messages.len() > pending_queue_visible_items() {
            let hidden_before = visible_range.start;
            let hidden_after = self
                .pending_queue_messages
                .len()
                .saturating_sub(visible_range.end);
            let hidden_label = match (hidden_before, hidden_after) {
                (0, after) => self.text().queue_more_below(after),
                (before, 0) => self.text().queue_more_above(before),
                (before, after) => self.text().queue_more_around(before, after),
            };
            lines.push(Line::from(Span::styled(
                hidden_label,
                Style::default().fg(theme::TEXT_SUBTLE),
            )));
        }
        lines
    }

    fn input_panel_height(&self, area_width: u16, area_height: u16) -> u16 {
        let prompt_width = INPUT_PROMPT.chars().count() as u16;
        let text_width = area_width
            .saturating_sub(2)
            .saturating_sub(prompt_width)
            .saturating_sub(1)
            .max(1) as usize;
        let queue_lines = self.pending_queue_panel_line_count();
        let max_content_lines = area_height
            .saturating_sub(2)
            .saturating_sub(queue_lines)
            .min(8)
            .max(1);
        let content_lines = wrap_approx_lines(&self.input, text_width).len() as u16;
        content_lines.min(max_content_lines).max(1) + queue_lines + 2
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
                        .fg(theme::TEXT_INVERTED)
                        .bg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(spec.usage.to_string(), style),
                    Span::styled(
                        format!("  {}", spec.description(self.language)),
                        Style::default().fg(theme::TEXT_SUBTLE),
                    ),
                ]))
            })
            .collect::<Vec<_>>();
        let popup = List::new(items)
            .block(
                Block::default()
                    .title(self.text().commands_title())
                    .borders(Borders::ALL),
            )
            .highlight_symbol("");
        frame.render_widget(Clear, area);
        frame.render_widget(popup, area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let status_text = if self.status_message.is_empty() {
            self.text().ready().to_string()
        } else {
            self.status_message.clone()
        };
        let text = if self.context_usage_label.is_empty() {
            status_text
        } else {
            format!("{status_text} | {}", self.context_usage_label)
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {text}"),
                Style::default().fg(theme::TEXT_SUBTLE),
            ))),
            area,
        );
    }

    fn render_model_chooser(&self, frame: &mut Frame) {
        let popup = centered_rect(84, 70, frame.area());
        frame.render_widget(Clear, popup);
        let items = self
            .model_choices
            .iter()
            .map(|choice| {
                let marker = if choice.selected {
                    self.text().current_marker()
                } else {
                    ""
                };
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            &choice.model_id,
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(&choice.provider_name, Style::default().fg(theme::ACCENT)),
                        Span::raw(" "),
                        Span::styled(
                            format!("({})", choice.provider_type_id),
                            Style::default().fg(theme::TEXT_SUBTLE),
                        ),
                        Span::raw(" "),
                        Span::styled(marker, Style::default().fg(theme::ACCENT_STRONG)),
                    ]),
                    Line::from(vec![
                        Span::styled(&choice.provider_id, Style::default()),
                        Span::styled(
                            format!("  {}", choice.provider_type_id),
                            Style::default().fg(theme::TEXT_SUBTLE),
                        ),
                    ]),
                ])
            })
            .collect::<Vec<_>>();
        let list = List::new(items)
            .block(
                Block::default()
                    .title(self.text().choose_chat_model_title())
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::ACCENT)),
            )
            .highlight_style(
                Style::default()
                    .bg(theme::ACCENT_BG)
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");
        let mut state = ListState::default();
        if !self.model_choices.is_empty() {
            state.select(Some(
                self.selected_model_choice_index
                    .min(self.model_choices.len().saturating_sub(1)),
            ));
        }
        frame.render_stateful_widget(list, popup, &mut state);
    }

    fn render_help_modal(&self, frame: &mut Frame) {
        let popup = centered_rect(72, 60, frame.area());
        frame.render_widget(Clear, popup);
        let lines = self
            .text()
            .help_lines()
            .iter()
            .enumerate()
            .map(|(index, line)| {
                if index == 0 {
                    Line::from(Span::styled(
                        *line,
                        Style::default().add_modifier(Modifier::BOLD),
                    ))
                } else {
                    Line::from(*line)
                }
            })
            .collect::<Vec<_>>();
        let help = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .title(self.text().help_title())
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(help, popup);
    }

    fn render_startup_workspace_prompt(&self, frame: &mut Frame) {
        let Some(prompt) = self.startup_workspace_prompt.as_ref() else {
            return;
        };
        let text = self.text();
        let popup = centered_rect(70, 22, frame.area());
        frame.render_widget(Clear, popup);
        let yes_style = if prompt.accept_selected {
            Style::default()
                .fg(theme::TEXT_INVERTED)
                .bg(theme::ACCENT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::ACCENT_STRONG)
        };
        let no_style = if prompt.accept_selected {
            Style::default().fg(theme::TEXT_SUBTLE)
        } else {
            Style::default()
                .fg(theme::TEXT_INVERTED)
                .bg(theme::ACCENT)
                .add_modifier(Modifier::BOLD)
        };
        let lines = vec![
            Line::from(text.workspace_question()),
            Line::from(""),
            Line::from(Span::styled(
                prompt.path.clone(),
                Style::default().fg(theme::TEXT_MUTED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(text.yes_button(), yes_style),
                Span::raw("  "),
                Span::styled(text.no_button(), no_style),
            ]),
        ];
        let modal = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .title(text.workspace_title())
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::ACCENT_DIM)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(modal, popup);
    }

    fn render_startup_update_prompt(&self, frame: &mut Frame) {
        let Some(prompt) = self.startup_update_prompt.as_ref() else {
            return;
        };
        let text = self.text();
        let popup = centered_rect(78, 42, frame.area());
        frame.render_widget(Clear, popup);
        let download_style = if prompt.download_selected {
            Style::default()
                .fg(theme::TEXT_INVERTED)
                .bg(theme::ACCENT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::ACCENT_STRONG)
        };
        let skip_style = if prompt.download_selected {
            Style::default().fg(theme::TEXT_SUBTLE)
        } else {
            Style::default()
                .fg(theme::TEXT_INVERTED)
                .bg(theme::ACCENT)
                .add_modifier(Modifier::BOLD)
        };
        let release_version = prompt
            .release_info
            .as_ref()
            .map(|info| info.version.clone())
            .unwrap_or_else(|| text.release_unknown().to_string());
        let release_page = prompt
            .release_info
            .as_ref()
            .map(|info| info.releasePageUrl.clone())
            .unwrap_or_else(String::new);
        let mut lines = vec![
            Line::from(Span::styled(
                text.full_update_available(),
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    text.version_label(),
                    Style::default().fg(theme::TEXT_SUBTLE),
                ),
                Span::styled(
                    release_version,
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    text.release_label(),
                    Style::default().fg(theme::TEXT_SUBTLE),
                ),
                Span::styled(release_page, Style::default().fg(theme::TEXT_MUTED)),
            ]),
            Line::from(""),
        ];

        match &prompt.download_state {
            FullUpdateDownloadState::Ready => {
                lines.push(Line::from(vec![
                    Span::styled(text.download_button(), download_style),
                    Span::raw("  "),
                    Span::styled(text.skip_button(), skip_style),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    text.update_prompt_help(),
                    Style::default().fg(theme::TEXT_SUBTLE),
                )));
            }
            FullUpdateDownloadState::Downloading {
                stage,
                message,
                read_bytes,
                total_bytes,
                speed_bytes_per_sec,
            } => {
                let percent = if *total_bytes > 0 {
                    ((*read_bytes as f64 / *total_bytes as f64) * 100.0).round() as u64
                } else {
                    0
                };
                let bar = progress_bar(percent, 34);
                lines.push(Line::from(vec![
                    Span::styled(text.stage_label(), Style::default().fg(theme::TEXT_SUBTLE)),
                    Span::raw(format!("{stage:?}")),
                ]));
                lines.push(Line::from(message.clone()));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(bar, Style::default().fg(theme::ACCENT_STRONG)),
                    Span::raw(format!(" {percent}%")),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(text.bytes_label(), Style::default().fg(theme::TEXT_SUBTLE)),
                    Span::raw(format!(
                        "{} / {}",
                        format_bytes(*read_bytes),
                        format_bytes(*total_bytes)
                    )),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(text.speed_label(), Style::default().fg(theme::TEXT_SUBTLE)),
                    Span::raw(format!("{}/s", format_bytes(*speed_bytes_per_sec))),
                ]));
            }
            FullUpdateDownloadState::Complete { package_path } => {
                lines.push(Line::from(Span::styled(
                    text.package_ready(),
                    Style::default()
                        .fg(theme::ACCENT_STRONG)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    package_path.to_string_lossy().to_string(),
                    Style::default().fg(theme::TEXT_MUTED),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    text.enter_closes(),
                    Style::default().fg(theme::TEXT_SUBTLE),
                )));
            }
            FullUpdateDownloadState::Error { message } => {
                lines.push(Line::from(Span::styled(
                    text.download_failed(),
                    Style::default()
                        .fg(theme::ERROR)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    message.clone(),
                    Style::default().fg(theme::ERROR_DIM),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    text.enter_closes(),
                    Style::default().fg(theme::TEXT_SUBTLE),
                )));
            }
            FullUpdateDownloadState::CheckError { message } => {
                lines.push(Line::from(Span::styled(
                    text.update_check_failed(),
                    Style::default()
                        .fg(theme::ERROR)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    message.clone(),
                    Style::default().fg(theme::ERROR_DIM),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    text.enter_closes(),
                    Style::default().fg(theme::TEXT_SUBTLE),
                )));
            }
        }

        let modal = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .title(text.update_title())
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::ACCENT_DIM)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(modal, popup);
    }

    fn render_approval_modal(&self, frame: &mut Frame) {
        let Some(request) = self.approval_bridge.current() else {
            return;
        };
        let text = self.text();
        let popup = centered_rect(76, 44, frame.area());
        frame.render_widget(Clear, popup);
        let elapsed = request.requested_at.elapsed().as_secs();
        let params = if request.tool.parameters.is_empty() {
            text.params_none().to_string()
        } else {
            request
                .tool
                .parameters
                .iter()
                .map(|parameter| format!("{}={}", parameter.name, parameter.value))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let lines = vec![
            Line::from(Span::styled(
                text.tool_approval_required(),
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(text.tool_label(), Style::default().fg(theme::TEXT_SUBTLE)),
                Span::styled(
                    request.tool.name.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    text.operation_label(),
                    Style::default().fg(theme::TEXT_SUBTLE),
                ),
                Span::raw(request.description),
            ]),
            Line::from(vec![
                Span::styled(
                    text.parameters_label(),
                    Style::default().fg(theme::TEXT_SUBTLE),
                ),
                Span::styled(params, Style::default().fg(theme::TEXT_MUTED)),
            ]),
            Line::from(vec![
                Span::styled(
                    text.timeout_label(),
                    Style::default().fg(theme::TEXT_SUBTLE),
                ),
                Span::raw(format!("{}s / 60s", elapsed.min(60))),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "1 ",
                    Style::default()
                        .fg(theme::ACCENT_STRONG)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(text.approval_yes_once()),
            ]),
            Line::from(vec![
                Span::styled(
                    "2 ",
                    Style::default()
                        .fg(theme::ERROR)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(text.approval_no()),
            ]),
            Line::from(vec![
                Span::styled(
                    "3 ",
                    Style::default()
                        .fg(theme::ACCENT_DIM)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(text.approval_yes_always()),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                text.approval_shortcuts(),
                Style::default().fg(theme::TEXT_SUBTLE),
            )),
        ];
        let modal = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .title(text.approval_title())
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::ACCENT_DIM)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(modal, popup);
    }
}

fn progress_bar(percent: u64, width: usize) -> String {
    let filled = ((percent.min(100) as usize) * width) / 100;
    format!("[{}{}]", "#".repeat(filled), "-".repeat(width - filled))
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let value = bytes as f64;
    if value >= GIB {
        format!("{:.1} GiB", value / GIB)
    } else if value >= MIB {
        format!("{:.1} MiB", value / MIB)
    } else if value >= KIB {
        format!("{:.1} KiB", value / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn thinking_indicator_line(text: &'static str) -> Line<'static> {
    let elapsed_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after unix epoch")
        .as_millis();
    let chars = text.chars().collect::<Vec<_>>();
    let sweep_len = chars.len() + 5;
    let sweep = ((elapsed_ms / 145) % sweep_len as u128) as isize - 2;
    let mut spans = Vec::new();
    for (index, ch) in chars.into_iter().enumerate() {
        let distance = (index as isize - sweep).abs();
        let style = match distance {
            0 => Style::default()
                .fg(theme::ACCENT_STRONG)
                .add_modifier(Modifier::BOLD),
            1 => Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::ITALIC),
            2 => Style::default()
                .fg(theme::TEXT_MUTED)
                .add_modifier(Modifier::ITALIC),
            _ => Style::default()
                .fg(theme::TEXT_SUBTLE)
                .add_modifier(Modifier::DIM | Modifier::ITALIC),
        };
        spans.push(Span::styled(ch.to_string(), style));
    }
    let dots = ((elapsed_ms / 360) % 4) as usize;
    spans.push(Span::styled(
        ".".repeat(dots),
        Style::default()
            .fg(theme::TEXT_SUBTLE)
            .add_modifier(Modifier::DIM | Modifier::ITALIC),
    ));
    Line::from(spans)
}
