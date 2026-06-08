use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use std::time::{SystemTime, UNIX_EPOCH};

use super::app::{FocusArea, FullUpdateDownloadState, OperitTui};
use super::helpers::{
    centered_rect, render_message_lines, short_chat_label, transcript_max_scroll, wrap_approx_lines,
};
use super::theme;

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
            .unwrap_or("New Chat");
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
                    .title("Chats")
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
        let messages = self.current_messages();
        let is_loading = self.current_chat_is_loading();
        let input_state = self.current_chat_input_processing_state();
        let thinking_line = thinking_indicator_line();
        let content_width = area.width.saturating_sub(2).max(1) as usize;
        let transcript_lines = render_message_lines(
            &messages,
            content_width,
            is_loading,
            &input_state,
            &thinking_line,
            &mut self.typewriter_state,
        );
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
            .scroll((self.transcript_scroll, 0));
        frame.render_widget(paragraph, area);
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focus == FocusArea::Input {
            Style::default().fg(theme::ACCENT)
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
        let prompt_indent = " ".repeat(prompt_width);
        let rendered_text = visible_text
            .split('\n')
            .enumerate()
            .map(|(index, line)| {
                if index == 0 {
                    format!("{INPUT_PROMPT}{line}")
                } else {
                    format!("{prompt_indent}{line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
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

    fn input_panel_height(&self, area_width: u16, area_height: u16) -> u16 {
        let prompt_width = INPUT_PROMPT.chars().count() as u16;
        let text_width = area_width
            .saturating_sub(2)
            .saturating_sub(prompt_width)
            .saturating_sub(1)
            .max(1) as usize;
        let content_lines = wrap_approx_lines(&self.input, text_width).len() as u16;
        let max_content_lines = area_height.saturating_sub(6).min(8).max(1);
        content_lines.min(max_content_lines).max(1) + 2
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
                        format!("  {}", spec.description),
                        Style::default().fg(theme::TEXT_SUBTLE),
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
        let status_text = if self.status_message.is_empty() {
            "Ready".to_string()
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
                let marker = if choice.selected { "current" } else { "" };
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{}[{}]", choice.config_name, choice.model_index),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(choice.provider_name, Style::default().fg(theme::ACCENT)),
                        Span::raw(" "),
                        Span::styled(marker, Style::default().fg(theme::ACCENT_STRONG)),
                    ]),
                    Line::from(vec![
                        Span::styled(&choice.model_name, Style::default()),
                        Span::styled(
                            format!("  {}", choice.config_id),
                            Style::default().fg(theme::TEXT_SUBTLE),
                        ),
                    ]),
                ])
            })
            .collect::<Vec<_>>();
        let list = List::new(items)
            .block(
                Block::default()
                    .title("Choose Chat Model")
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
        let lines = vec![
            Line::from(Span::styled(
                "Operit2 TUI",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("Tab: complete command / switch focus"),
            Line::from("Enter: send message / activate selected chat"),
            Line::from("F3 or /switch: toggle chat list"),
            Line::from("/resume: resume previous chat"),
            Line::from("Up/Down: select chat when chat list is focused"),
            Line::from("Ctrl+J: insert newline in input"),
            Line::from("Ctrl+N: create new chat"),
            Line::from("Ctrl+C: press twice to quit"),
            Line::from("Ctrl+Q: quit"),
            Line::from("PageUp/PageDown: scroll conversation by page"),
            Line::from("Ctrl+U/Ctrl+D: scroll conversation by half page"),
            Line::from("Ctrl+Home/Ctrl+End: top / bottom conversation"),
            Line::from("Esc: cancel request / close help / clear status"),
            Line::from(""),
            Line::from("Local commands:"),
            Line::from("/help"),
            Line::from("/new [--character <name>] [--group-card <id>] [--group <name>]"),
            Line::from("/switch"),
            Line::from("/resume"),
            Line::from("/max"),
            Line::from("/model current | /model list | /model choose"),
            Line::from("/model use <config-id> [model-index]"),
            Line::from("/approval | /approval list|allow|ask|forbid"),
            Line::from("/approval tool <tool> <allow|ask|forbid|clear>"),
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

    fn render_startup_workspace_prompt(&self, frame: &mut Frame) {
        let Some(prompt) = self.startup_workspace_prompt.as_ref() else {
            return;
        };
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
            Line::from("Use current folder as workspace?"),
            Line::from(""),
            Line::from(Span::styled(
                prompt.path.clone(),
                Style::default().fg(theme::TEXT_MUTED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Y Yes ", yes_style),
                Span::raw("  "),
                Span::styled(" N No ", no_style),
            ]),
        ];
        let modal = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .title("Workspace")
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
            .unwrap_or_else(|| "unknown".to_string());
        let release_page = prompt
            .release_info
            .as_ref()
            .map(|info| info.releasePageUrl.clone())
            .unwrap_or_else(String::new);
        let mut lines = vec![
            Line::from(Span::styled(
                "Full update available",
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("version: ", Style::default().fg(theme::TEXT_SUBTLE)),
                Span::styled(
                    release_version,
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("release: ", Style::default().fg(theme::TEXT_SUBTLE)),
                Span::styled(release_page, Style::default().fg(theme::TEXT_MUTED)),
            ]),
            Line::from(""),
        ];

        match &prompt.download_state {
            FullUpdateDownloadState::Ready => {
                lines.push(Line::from(vec![
                    Span::styled(" 1 Download ", download_style),
                    Span::raw("  "),
                    Span::styled(" 2 Skip ", skip_style),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Enter selects | Left/Right changes | d/s | Esc=skip",
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
                    Span::styled("stage: ", Style::default().fg(theme::TEXT_SUBTLE)),
                    Span::raw(format!("{stage:?}")),
                ]));
                lines.push(Line::from(message.clone()));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(bar, Style::default().fg(theme::ACCENT_STRONG)),
                    Span::raw(format!(" {percent}%")),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("bytes: ", Style::default().fg(theme::TEXT_SUBTLE)),
                    Span::raw(format!(
                        "{} / {}",
                        format_bytes(*read_bytes),
                        format_bytes(*total_bytes)
                    )),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("speed: ", Style::default().fg(theme::TEXT_SUBTLE)),
                    Span::raw(format!("{}/s", format_bytes(*speed_bytes_per_sec))),
                ]));
            }
            FullUpdateDownloadState::Complete { package_path } => {
                lines.push(Line::from(Span::styled(
                    "Package ready",
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
                    "Enter closes",
                    Style::default().fg(theme::TEXT_SUBTLE),
                )));
            }
            FullUpdateDownloadState::Error { message } => {
                lines.push(Line::from(Span::styled(
                    "Download failed",
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
                    "Enter closes",
                    Style::default().fg(theme::TEXT_SUBTLE),
                )));
            }
            FullUpdateDownloadState::CheckError { message } => {
                lines.push(Line::from(Span::styled(
                    "Update check failed",
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
                    "Enter closes",
                    Style::default().fg(theme::TEXT_SUBTLE),
                )));
            }
        }

        let modal = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .title("Update")
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
        let popup = centered_rect(76, 44, frame.area());
        frame.render_widget(Clear, popup);
        let elapsed = request.requested_at.elapsed().as_secs();
        let params = if request.tool.parameters.is_empty() {
            "params: none".to_string()
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
                "Tool approval required",
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("tool: ", Style::default().fg(theme::TEXT_SUBTLE)),
                Span::styled(
                    request.tool.name.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("operation: ", Style::default().fg(theme::TEXT_SUBTLE)),
                Span::raw(request.description),
            ]),
            Line::from(vec![
                Span::styled("parameters: ", Style::default().fg(theme::TEXT_SUBTLE)),
                Span::styled(params, Style::default().fg(theme::TEXT_MUTED)),
            ]),
            Line::from(vec![
                Span::styled("timeout: ", Style::default().fg(theme::TEXT_SUBTLE)),
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
                Span::raw("yes, allow once"),
            ]),
            Line::from(vec![
                Span::styled(
                    "2 ",
                    Style::default()
                        .fg(theme::ERROR)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("no, deny"),
            ]),
            Line::from(vec![
                Span::styled(
                    "3 ",
                    Style::default()
                        .fg(theme::ACCENT_DIM)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("yes, always allow this tool"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Shortcuts: y=yes, n=no, a=always, Esc=no",
                Style::default().fg(theme::TEXT_SUBTLE),
            )),
        ];
        let modal = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .title("Approval")
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

fn thinking_indicator_line() -> Line<'static> {
    let elapsed_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after unix epoch")
        .as_millis();
    let text = "thinking";
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
