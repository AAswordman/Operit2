use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Wrap,
};
use ratatui::Frame;
use std::time::{SystemTime, UNIX_EPOCH};

use operit_node_runtime::RuntimeRemoteLinkService::{
    RuntimeDeviceSpaceConnectionStatus, RuntimeDeviceSpaceDevice,
};
use operit_util::GithubReleaseUtil::FullUpdateStage;

use super::app::{
    hub_member_status_label, join_status_label, network_hub_rows, peer_transport_label,
    space_join_is_active,
    DeviceManagerAction, DeviceManagerMode, DeviceManagerModal, DeviceManagerRow, FocusArea,
    FullUpdateDownloadState, NetworkHubModal, NetworkHubRow, OperitTui, PairField, PairStage,
    PAIR_WIZARD_TRANSPORTS, StartupInstallState,
};
use crate::cli::network_control_ui::{network_device_label_by_id, network_role_summary};
use super::helpers::{
    centered_rect, display_width, short_chat_label, transcript_max_scroll, wrap_approx_lines,
};
use super::pending_queue::{
    pending_queue_preview_text, pending_queue_visible_items, pending_queue_visible_range,
};
use super::scrollbar::{
    split_transcript_inner, BEGIN_SYMBOL, END_SYMBOL, THUMB_SYMBOL, TRACK_SYMBOL,
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

        // Overlays record the topmost popup content rect for mouse selection.
        self.popup_selection_rect = None;

        if let Some(editor) = &self.compose.editor {
            let area = centered_rect(80, 25, frame.area());
            frame.render_widget(Clear, area);
            frame.render_widget(
                Paragraph::new(editor.value.as_str())
                    .wrap(Wrap { trim: false })
                    .block(
                        Block::default()
                            .title("Edit · Enter: apply · Esc: cancel")
                            .borders(Borders::ALL),
                    ),
                area,
            );
        }

        if self.show_model_chooser {
            self.render_model_chooser(frame);
        }

        if self.show_config_popup {
            self.config_ui.render(frame, self.text());
            self.popup_selection_rect = self.config_ui.selection_rect;
        }

        if self.show_help {
            self.render_help_modal(frame);
        }

        if self.startup_install_prompt.is_some() {
            self.render_startup_install_prompt(frame);
        }

        if self.startup_update_prompt.is_some() && self.startup_install_prompt.is_none() {
            self.render_startup_update_prompt(frame);
        }

        if self.startup_workspace_prompt.is_some()
            && self.startup_update_prompt.is_none()
            && self.startup_install_prompt.is_none()
        {
            self.render_startup_workspace_prompt(frame);
        }

        if !self.current_tool_permission_requests.is_empty() {
            self.render_approval_modal(frame);
        }

        if self.network_hub.is_some() {
            self.render_network_hub(frame);
        }

        // The hub opens list popups (discover, audit, policy, token, leave,
        // unpair); they render above the panel and take keys first, so they
        // behave like the device window instead of being hidden behind it.
        if self.show_list_popup {
            self.render_list_popup(frame);
        }

        if self.device_manager.is_some() {
            self.render_device_manager(frame);
        }

        if self.pair_wizard.is_some() {
            self.render_pair_wizard(frame);
        }

        if self.join_decision.is_some() {
            self.render_join_decision_modal(frame);
        }

        self.finish_popup_selection(frame.buffer_mut());
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
        let thinking_line = thinking_indicator_line(text.thinking_process());
        let split = split_transcript_inner(area);
        let content_width = split.content.width.max(1) as usize;
        let current_chat_id = self.current_chat_id_cache.clone();
        let mut transcript_lines = render_transcript_lines(
            &messages,
            current_chat_id.as_deref(),
            is_loading,
            &input_state,
            &thinking_line,
            content_width,
            &mut self.typewriter_state,
            &mut self.transcript_render_cache,
            text,
        );
        self.compose.project(
            &mut transcript_lines,
            &self.transcript_render_cache.xml,
            &mut self.transcript_render_cache.fold_hits,
            content_width,
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

        let block = Block::default()
            .title(self.text().conversation_title())
            .borders(Borders::ALL);
        frame.render_widget(block, area);
        let paragraph =
            Paragraph::new(Text::from(transcript_lines)).scroll((self.transcript_scroll, 0));
        frame.render_widget(paragraph, split.content);
        if max_scroll > 0 {
            self.render_transcript_scrollbar(frame, split.scrollbar, max_scroll);
        }
    }

    /// Draws the inner transcript scrollbar with classic block and triangle glyphs.
    fn render_transcript_scrollbar(&self, frame: &mut Frame, area: Rect, max_scroll: u16) {
        let pressed = self.scrollbar_pressed || self.scrollbar_dragging;
        let hovered = self.scrollbar_hovered;
        let thumb_style = if pressed {
            Style::default().fg(theme::ACCENT)
        } else if hovered {
            Style::default().fg(theme::TEXT_MUTED)
        } else {
            Style::default()
                .fg(theme::TEXT_SUBTLE)
                .add_modifier(Modifier::DIM)
        };
        let track_style = if pressed {
            Style::default().fg(theme::TEXT_MUTED)
        } else {
            Style::default()
                .fg(theme::TEXT_SUBTLE)
                .add_modifier(Modifier::DIM)
        };
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_symbol(THUMB_SYMBOL)
            .track_symbol(Some(TRACK_SYMBOL))
            .begin_symbol(Some(BEGIN_SYMBOL))
            .end_symbol(Some(END_SYMBOL))
            .thumb_style(thumb_style)
            .track_style(track_style)
            .begin_style(track_style)
            .end_style(track_style);
        let mut scrollbar_state = ScrollbarState::new(max_scroll.saturating_add(1) as usize)
            .position(self.transcript_scroll as usize);
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
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
                    Span::styled(spec.usage().to_string(), style),
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

    fn render_model_chooser(&mut self, frame: &mut Frame) {
        let popup = centered_rect(84, 70, frame.area());
        frame.render_widget(Clear, popup);

        let block = Block::default()
            .title(if self.model_list_mode {
                self.text().model_list_title()
            } else {
                self.text().choose_chat_model_title()
            })
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT));
        let inner = block.inner(popup);
        self.popup_selection_rect = Some(inner);
        frame.render_widget(block, popup);

        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        let search_display = if self.model_chooser_search.is_empty() {
            Span::styled(
                self.text().model_chooser_search_hint(),
                Style::default().fg(theme::TEXT_SUBTLE),
            )
        } else {
            Span::styled(&self.model_chooser_search, Style::default())
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(theme::ACCENT)),
                search_display,
            ])),
            areas[0],
        );

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "─".repeat(areas[1].width as usize),
                Style::default().fg(theme::TEXT_SUBTLE),
            ))),
            areas[1],
        );

        let items = self
            .model_chooser_filtered_indices
            .iter()
            .map(|&original_index| {
                let choice = &self.model_choices[original_index];
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
            .highlight_style(
                Style::default()
                    .bg(theme::ACCENT_BG)
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");
        let mut state = ListState::default();
        if !self.model_chooser_filtered_indices.is_empty() {
            state
                .select(Some(self.selected_model_choice_index.min(
                    self.model_chooser_filtered_indices.len().saturating_sub(1),
                )));
        }
        frame.render_stateful_widget(list, areas[2], &mut state);
    }

    fn render_list_popup(&mut self, frame: &mut Frame) {
        let popup = centered_rect(60, 70, frame.area());
        frame.render_widget(Clear, popup);

        let block = Block::default()
            .title(self.list_popup_title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT));
        let inner = block.inner(popup);
        self.popup_selection_rect = Some(inner);
        frame.render_widget(block, popup);

        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        let search_display = if self.list_popup_search.is_empty() {
            Span::styled(
                self.text().model_chooser_search_hint(),
                Style::default().fg(theme::TEXT_SUBTLE),
            )
        } else {
            Span::styled(&self.list_popup_search, Style::default())
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(theme::ACCENT)),
                search_display,
            ])),
            areas[0],
        );

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "─".repeat(areas[1].width as usize),
                Style::default().fg(theme::TEXT_SUBTLE),
            ))),
            areas[1],
        );

        let items = self
            .list_popup_filtered_indices
            .iter()
            .map(|&original_index| {
                ListItem::new(vec![Line::from(
                    self.list_popup_items[original_index].as_str(),
                )])
            })
            .collect::<Vec<_>>();
        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(theme::ACCENT_BG)
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");
        let mut state = ListState::default();
        if !self.list_popup_filtered_indices.is_empty() {
            state.select(Some(
                self.list_popup_selected_index
                    .min(self.list_popup_filtered_indices.len().saturating_sub(1)),
            ));
        }
        frame.render_stateful_widget(list, areas[2], &mut state);
    }

    fn render_help_modal(&mut self, frame: &mut Frame) {
        let popup = centered_rect(72, 60, frame.area());
        frame.render_widget(Clear, popup);
        self.popup_selection_rect = Some(Block::default().borders(Borders::ALL).inner(popup));
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

    fn render_startup_install_prompt(&mut self, frame: &mut Frame) {
        let Some(prompt) = self.startup_install_prompt.as_ref() else {
            return;
        };
        let text = self.text();
        let popup = centered_rect(82, 52, frame.area());
        frame.render_widget(Clear, popup);
        self.popup_selection_rect = Some(Block::default().borders(Borders::ALL).inner(popup));
        let lines = match &prompt.state {
            StartupInstallState::Ready => {
                let yes_style = if prompt.install_selected {
                    Style::default()
                        .fg(theme::TEXT_INVERTED)
                        .bg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::ACCENT_STRONG)
                };
                let no_style = if prompt.install_selected {
                    Style::default().fg(theme::TEXT_SUBTLE)
                } else {
                    Style::default()
                        .fg(theme::TEXT_INVERTED)
                        .bg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD)
                };
                vec![
                    Line::from(Span::styled(
                        text.install_command_question(),
                        Style::default()
                            .fg(theme::ACCENT)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(text.install_command_description()),
                    Line::from(""),
                    Line::from(text.install_command_decline_hint()),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(text.yes_button(), yes_style),
                        Span::raw("  "),
                        Span::styled(text.no_button(), no_style),
                    ]),
                ]
            }
            StartupInstallState::Installing { message } => vec![
                Line::from(Span::styled(
                    text.install_command_installing(),
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(message.clone()),
            ],
            StartupInstallState::Complete => vec![
                Line::from(Span::styled(
                    text.install_command_installed(),
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(text.enter_closes()),
            ],
            StartupInstallState::Error { message } => vec![
                Line::from(Span::styled(
                    text.install_command_failed(),
                    Style::default()
                        .fg(theme::ERROR)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(message.clone()),
                Line::from(""),
                Line::from(text.enter_closes()),
            ],
        };
        let modal = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .title(text.install_command_title())
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::ACCENT_DIM)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(modal, popup);
    }

    fn render_startup_workspace_prompt(&mut self, frame: &mut Frame) {
        let Some(prompt) = self.startup_workspace_prompt.as_ref() else {
            return;
        };
        let text = self.text();
        let popup = centered_rect(74, 40, frame.area());
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
        let modal_block = Block::default()
            .title(text.workspace_title())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT_DIM));
        let inner = modal_block.inner(popup);
        self.popup_selection_rect = Some(inner);
        frame.render_widget(modal_block, popup);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(inner);
        let body_lines = vec![
            Line::from(text.workspace_question()),
            Line::from(""),
            Line::from(Span::styled(
                prompt.path.clone(),
                Style::default().fg(theme::TEXT_MUTED),
            )),
        ];
        let body = Paragraph::new(Text::from(body_lines)).wrap(Wrap { trim: false });
        frame.render_widget(body, chunks[0]);
        let actions = Paragraph::new(Line::from(vec![
            Span::styled(text.yes_button(), yes_style),
            Span::raw("  "),
            Span::styled(text.no_button(), no_style),
        ]));
        frame.render_widget(actions, chunks[1]);
    }

    fn render_startup_update_prompt(&mut self, frame: &mut Frame) {
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
                lines.push(Line::from(vec![
                    Span::styled(text.stage_label(), Style::default().fg(theme::TEXT_SUBTLE)),
                    Span::raw(full_update_stage_label(text, *stage)),
                ]));
                lines.push(Line::from(message.clone()));
                if *total_bytes > 0 {
                    let percent =
                        ((*read_bytes as f64 / *total_bytes as f64) * 100.0).round() as u64;
                    let bar = progress_bar(percent, 34);
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
            }
            FullUpdateDownloadState::Complete {
                package_path: _,
                install_status,
            } => {
                let status_text = match install_status {
                    Some(crate::cli::DownloadedUpdateInstallStatus::Installed) => {
                        text.update_installed()
                    }
                    Some(crate::cli::DownloadedUpdateInstallStatus::Scheduled) => {
                        text.update_scheduled()
                    }
                    Some(crate::cli::DownloadedUpdateInstallStatus::NotInstalled) => {
                        text.update_not_installed()
                    }
                    Some(crate::cli::DownloadedUpdateInstallStatus::TargetMismatch) => {
                        text.update_target_mismatch()
                    }
                    None => text.package_ready(),
                };
                lines.push(Line::from(Span::styled(
                    status_text,
                    Style::default()
                        .fg(theme::ACCENT_STRONG)
                        .add_modifier(Modifier::BOLD),
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
        self.popup_selection_rect = Some(Block::default().borders(Borders::ALL).inner(popup));
        frame.render_widget(modal, popup);
    }

    fn render_approval_modal(&mut self, frame: &mut Frame) {
        let pending_count = self.current_tool_permission_requests.len();
        let Some(request) = self.current_tool_permission_requests.first().cloned() else {
            return;
        };
        let text = self.text();
        let popup = centered_rect(82, 78, frame.area());
        frame.render_widget(Clear, popup);
        let now_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(request.requestedAtMillis);
        let elapsed = now_millis.saturating_sub(request.requestedAtMillis) / 1000;
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
        let title = if pending_count > 1 {
            format!("{} (1/{})", text.approval_title(), pending_count)
        } else {
            text.approval_title().to_string()
        };
        let modal_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT_DIM));
        let inner = modal_block.inner(popup);
        self.popup_selection_rect = Some(inner);
        frame.render_widget(modal_block, popup);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(5)])
            .split(inner);
        let body_lines = vec![
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
        ];
        let body = Paragraph::new(Text::from(body_lines)).wrap(Wrap { trim: false });
        frame.render_widget(body, chunks[0]);
        let action_lines = vec![
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
        let actions = Paragraph::new(Text::from(action_lines)).wrap(Wrap { trim: false });
        frame.render_widget(actions, chunks[1]);
    }

    /// Renders the Space join decision popup: the selected request's details
    /// with Y/N/Esc actions, or a selectable queue when several requests are
    /// pending.
    fn render_join_decision_modal(&mut self, frame: &mut Frame) {
        let Some(modal) = self.join_decision.as_ref() else {
            return;
        };
        let text = self.text();
        let popup = centered_rect(70, 42, frame.area());
        frame.render_widget(Clear, popup);
        let modal_block = Block::default()
            .title(text.network_join_decision_title())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT_DIM));
        let inner = modal_block.inner(popup);
        frame.render_widget(modal_block, popup);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(inner);
        let Some(request) = modal.requests.get(modal.selected) else {
            return;
        };
        let label_style = Style::default().fg(theme::TEXT_SUBTLE);
        let mut body_lines = vec![Line::from("")];
        body_lines.push(Line::from(vec![
            Span::styled(
                request.applicantName.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({})", request.applicantDeviceId),
                Style::default().fg(theme::TEXT_MUTED),
            ),
        ]));
        body_lines.push(Line::from(vec![
            Span::styled(format!("{}: ", text.network_join_decision_space()), label_style),
            Span::raw(request.spaceName.clone()),
            Span::styled("  ·  ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                join_status_label(text, &request.status).to_string(),
                Style::default(),
            ),
        ]));
        if modal.requests.len() > 1 {
            body_lines.push(Line::from(""));
            body_lines.push(Line::from(Span::styled(
                format!("{}/{}", modal.selected + 1, modal.requests.len()),
                Style::default().fg(theme::TEXT_MUTED),
            )));
        }
        let body = Paragraph::new(Text::from(body_lines)).wrap(Wrap { trim: false });
        frame.render_widget(body, chunks[0]);

        let hint = Paragraph::new(Line::from(Span::styled(
            text.network_join_decision_hint(),
            Style::default().fg(theme::TEXT_SUBTLE),
        )));
        frame.render_widget(hint, chunks[1]);
    }

    /// Renders the device management window: pending join requests and known
    /// devices with their policy state while browsing, or the mode-specific
    /// action surface otherwise. Reachability and restriction render as
    /// independent flags; admit only lifts a restriction.
    /// The `/network` hub: identity header, waiting-for-me area, and the
    /// combined member/paired-peer device list with selection.
    fn render_network_hub(&mut self, frame: &mut Frame) {
        let Some(hub) = self.network_hub.as_ref() else {
            return;
        };
        let text = self.text();
        let popup = centered_rect(78, 78, frame.area());
        frame.render_widget(Clear, popup);
        let modal_block = Block::default()
            .title(text.network_hub_title())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT_DIM));
        let inner = modal_block.inner(popup);
        frame.render_widget(modal_block, popup);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(4),
                Constraint::Min(5),
                Constraint::Length(1),
            ])
            .split(inner);

        let self_identity = hub
            .topology
            .devices
            .iter()
            .find(|device| device.deviceId == hub.topology.currentDeviceId)
            .and_then(|device| device.currentIdentity.as_ref())
            .map(|identity| identity.displayName.clone())
            .unwrap_or_else(|| text.network_devices_no_identity().to_string());
        let header = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    format!("{} ", text.network_hub_node()),
                    Style::default().fg(theme::TEXT_SUBTLE),
                ),
                Span::styled(
                    hub.topology.currentDeviceId.clone(),
                    Style::default().fg(theme::ACCENT_STRONG),
                ),
                Span::styled(
                    format!("  {} ", text.network_hub_identity()),
                    Style::default().fg(theme::TEXT_SUBTLE),
                ),
                Span::raw(self_identity),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("{} ", text.network_hub_space()),
                    Style::default().fg(theme::TEXT_SUBTLE),
                ),
                Span::raw(if hub.spaceName.is_empty() {
                    text.network_devices_not_initialized().to_string()
                } else {
                    format!(
                        "{} ({})",
                        hub.spaceName,
                        hub.topology.devices.len()
                    )
                }),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("{} ", text.network_hub_listening()),
                    Style::default().fg(theme::TEXT_SUBTLE),
                ),
                Span::raw(match &hub.listening {
                    Some(summary) => summary.clone(),
                    None => text.network_hub_not_listening().to_string(),
                }),
            ]),
        ]);
        frame.render_widget(header, chunks[0]);

        let mut todo_lines = Vec::new();
        for prompt in &hub.prompts {
            todo_lines.push(Line::from(vec![
                Span::styled("▸ ", Style::default().fg(theme::ACCENT_STRONG)),
                Span::raw(text.network_hub_todo_inbound_pairing(&prompt.displayName)),
                Span::styled(
                    format!("  {}: {}", text.network_pairing_popup_code(), prompt.confirmationCode),
                    Style::default().fg(theme::ACCENT_STRONG).add_modifier(Modifier::BOLD),
                ),
            ]));
        }
        for pending in &self.pending_pairings {
            let label = if pending.displayName.is_empty() {
                pending.peerNodeId.clone()
            } else {
                pending.displayName.clone()
            };
            todo_lines.push(Line::from(Span::raw(
                text.network_hub_todo_outbound_pairing(&label),
            )));
        }
        for request in &hub.outgoingJoins {
            if space_join_is_active(&request.status) {
                todo_lines.push(Line::from(Span::raw(
                    text.network_hub_todo_outbound_join(
                        &request.targetDeviceId,
                        join_status_label(text, &request.status),
                    ),
                )));
            }
        }
        // A restricted member (or this device itself) waits on a human
        // decision: nobody comes back until it is admitted.
        for device in &hub.topology.devices {
            if hub.blocked.contains(&device.deviceId) {
                todo_lines.push(Line::from(vec![
                    Span::styled("▸ ", Style::default().fg(theme::ERROR_DIM)),
                    Span::raw(text.network_hub_todo_restricted(&device.deviceName)),
                ]));
            }
        }
        if !hub.initialized {
            todo_lines.push(Line::from(Span::styled(
                text.network_devices_not_initialized(),
                Style::default().fg(theme::TEXT_SUBTLE),
            )));
        }
        let todo_count = todo_lines.len();
        let todo_block = Block::default()
            .title(format!(
                "{} ({todo_count})",
                text.network_hub_todo_title()
            ))
            .borders(Borders::TOP);
        let todo_inner = todo_block.inner(chunks[1]);
        frame.render_widget(todo_block, chunks[1]);
        if todo_lines.is_empty() {
            todo_lines.push(Line::from(Span::styled(
                text.network_hub_todo_none(),
                Style::default().fg(theme::TEXT_SUBTLE),
            )));
        }
        frame.render_widget(
            Paragraph::new(todo_lines).wrap(Wrap { trim: false }),
            todo_inner,
        );

        let rows = network_hub_rows(&hub.topology, &hub.paired);
        let mut items = Vec::new();
        let mut item_for_row = Vec::with_capacity(rows.len());
        let mut members_header_drawn = false;
        let mut peers_header_drawn = false;
        for row in &rows {
            match row {
                NetworkHubRow::Member(device) => {
                    if !members_header_drawn {
                        members_header_drawn = true;
                        items.push(ListItem::new(section_header_line(
                            text.network_hub_devices_title(),
                        )));
                    }
                    item_for_row.push(items.len());
                    items.push(ListItem::new(vec![self.hub_member_line(hub, device)]));
                }
                NetworkHubRow::Peer {
                    deviceId,
                    label,
                    outbound,
                } => {
                    if !peers_header_drawn {
                        peers_header_drawn = true;
                        items.push(ListItem::new(section_header_line(
                            text.network_hub_peers_title(),
                        )));
                    }
                    item_for_row.push(items.len());
                    let hint = if *outbound {
                        text.network_hub_peer_joinable()
                    } else {
                        text.network_hub_peer_inbound_only()
                    };
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(label.clone(), Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(format!(" · {deviceId} "), Style::default().fg(theme::TEXT_SUBTLE)),
                        Span::styled(hint.to_string(), Style::default().fg(theme::TEXT_SUBTLE)),
                    ])));
                }
            }
        }
        if items.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                text.network_hub_devices_none(),
                Style::default().fg(theme::TEXT_SUBTLE),
            ))));
        }
        let selected_item = item_for_row.get(hub.selected).copied();
        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(theme::SELECTION_BG)
                    .fg(theme::SELECTION_TEXT),
            )
            .highlight_symbol("▸ ");
        let mut state = ListState::default();
        if let Some(index) = selected_item {
            state.select(Some(index));
        }
        frame.render_stateful_widget(list, chunks[2], &mut state);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                text.network_hub_hint(),
                Style::default().fg(theme::TEXT_SUBTLE),
            ))),
            chunks[3],
        );
        self.popup_selection_rect = Some(inner);
    }

    /// One member row in the hub device list; mirrors the device-manager
    /// styling so the two windows read as one family.
    fn hub_member_line(
        &self,
        hub: &NetworkHubModal,
        device: &RuntimeDeviceSpaceDevice,
    ) -> Line<'static> {
        let text = self.text();
        let blocked = hub.blocked.contains(&device.deviceId);
        if device.deviceId == hub.topology.currentDeviceId {
            let mut spans = vec![
                Span::raw("  "),
                Span::styled(
                    text.network_devices_self().to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" · {}", device.platform),
                    Style::default().fg(theme::TEXT_SUBTLE),
                ),
            ];
            // Being restricted by another administrator is exactly the state
            // this device cannot infer from reachability alone.
            if blocked {
                spans.push(Span::styled(
                    format!(" · {}", text.network_hub_member_restricted()),
                    Style::default().fg(theme::ERROR_DIM),
                ));
            }
            return Line::from(spans);
        }
        let status = hub_member_status_label(text, blocked, device.online)
            .map(|status| format!(" · {status}"))
            .unwrap_or_default();
        let identity = device
            .currentIdentity
            .as_ref()
            .map(|identity| format!(" · {}", identity.displayName))
            .unwrap_or_else(|| format!(" · {}", text.network_devices_no_identity()));
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                device.deviceName.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" · {}{}", device.platform, identity),
                Style::default().fg(theme::TEXT_SUBTLE),
            ),
            Span::styled(
                status,
                Style::default().fg(if blocked {
                    theme::ERROR_DIM
                } else {
                    theme::TEXT_SUBTLE
                }),
            ),
        ])
    }

    /// The pairing wizard: address form, six-digit code entry, or the
    /// post-pairing join offer, one stage at a time.
    fn render_pair_wizard(&mut self, frame: &mut Frame) {
        let Some(wizard) = self.pair_wizard.as_ref() else {
            return;
        };
        let text = self.text();
        let (popup, stage) = match wizard.stage {
            PairStage::Address => (centered_rect(58, 34, frame.area()), wizard.stage),
            PairStage::Code | PairStage::JoinOffer => (centered_rect(50, 30, frame.area()), wizard.stage),
        };
        frame.render_widget(Clear, popup);
        let modal_block = Block::default()
            .title(text.network_pair_wizard_title())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT_DIM));
        let inner = modal_block.inner(popup);
        frame.render_widget(modal_block, popup);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1), Constraint::Length(1)])
            .split(inner);

        match stage {
            PairStage::Address => {
                let field_style = |focused: bool| {
                    if focused {
                        Style::default().fg(theme::ACCENT_STRONG).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme::TEXT_MUTED)
                    }
                };
                let transport_label =
                    peer_transport_label(&PAIR_WIZARD_TRANSPORTS[wizard.transportIndex]);
                let transport_value = if wizard.field == PairField::Transport {
                    format!("‹ {transport_label} ›")
                } else {
                    format!("  {transport_label}  ")
                };
                let body = Paragraph::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{} ", text.network_pair_wizard_address()),
                            field_style(wizard.field == PairField::Address),
                        ),
                        Span::styled(
                            format!("{}▏", wizard.address),
                            field_style(wizard.field == PairField::Address),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            format!("{} ", text.network_pair_wizard_transport()),
                            field_style(wizard.field == PairField::Transport),
                        ),
                        Span::styled(
                            transport_value,
                            field_style(wizard.field == PairField::Transport),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            format!("{} ", text.network_pair_wizard_token()),
                            field_style(wizard.field == PairField::Token),
                        ),
                        Span::styled(
                            format!("{}▏", wizard.token),
                            field_style(wizard.field == PairField::Token),
                        ),
                        Span::styled(
                            format!(" ({})", text.network_pair_wizard_token_optional()),
                            Style::default().fg(theme::TEXT_SUBTLE),
                        ),
                    ]),
                    blank_line(),
                    Line::from(Span::styled(
                        text.network_pair_wizard_start_hint(),
                        Style::default().fg(theme::TEXT_SUBTLE),
                    )),
                ]);
                frame.render_widget(body, chunks[0]);
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        text.network_pair_wizard_form_hint(),
                        Style::default().fg(theme::TEXT_SUBTLE),
                    ))),
                    chunks[2],
                );
            }
            PairStage::Code => {
                let peer_name = wizard
                    .pairing
                    .as_ref()
                    .map(|pending| {
                        if pending.displayName.is_empty() {
                            wizard.address.clone()
                        } else {
                            pending.displayName.clone()
                        }
                    })
                    .unwrap_or_default();
                let mut filled: Vec<Span<'static>> = Vec::new();
                filled.push(Span::raw("[ "));
                for index in 0..6 {
                    let cell = wizard.code.chars().nth(index).unwrap_or('·');
                    filled.push(Span::styled(
                        format!("{cell} "),
                        Style::default()
                            .fg(theme::ACCENT_STRONG)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                filled.push(Span::raw("]"));
                let body = Paragraph::new(vec![
                    Line::from(Span::raw(
                        text.network_pair_wizard_code_prompt(&peer_name),
                    )),
                    blank_line(),
                    Line::from(filled),
                    blank_line(),
                    Line::from(Span::styled(
                        text.network_pair_wizard_code_hint(),
                        Style::default().fg(theme::TEXT_SUBTLE),
                    )),
                ]);
                frame.render_widget(body, chunks[0]);
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        text.network_pair_wizard_code_footer(),
                        Style::default().fg(theme::TEXT_SUBTLE),
                    ))),
                    chunks[2],
                );
            }
            PairStage::JoinOffer => {
                let peer_name = wizard
                    .peer
                    .as_ref()
                    .map(|peer| {
                        if peer.displayName.is_empty() {
                            peer.nodeId.clone()
                        } else {
                            peer.displayName.clone()
                        }
                    })
                    .unwrap_or_default();
                let body = Paragraph::new(vec![
                    Line::from(Span::styled(
                        text.network_pair_wizard_join_prompt(&peer_name),
                        Style::default().fg(theme::TEXT),
                    )),
                    blank_line(),
                    Line::from(Span::styled(
                        text.network_pair_wizard_join_wait_hint(),
                        Style::default().fg(theme::TEXT_SUBTLE),
                    )),
                ]);
                frame.render_widget(body, chunks[0]);
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        text.network_pair_wizard_join_hint(),
                        Style::default().fg(theme::TEXT_SUBTLE),
                    ))),
                    chunks[2],
                );
            }
        }

        if let Some(error) = &wizard.error {
            // An error takes over the footer row instead of squeezing the
            // body, so long join errors stay readable.
            let error_area = Rect {
                y: chunks[1].y,
                height: chunks[1].height + chunks[2].height,
                ..chunks[0]
            };
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    error.clone(),
                    Style::default().fg(theme::ERROR),
                )))
                .wrap(Wrap { trim: false }),
                error_area,
            );
        }
        self.popup_selection_rect = Some(inner);
    }

    fn render_device_manager(&mut self, frame: &mut Frame) {
        let Some(modal) = self.device_manager.as_ref() else {
            return;
        };
        let text = self.text();
        let popup = centered_rect(72, 60, frame.area());
        frame.render_widget(Clear, popup);
        let modal_block = Block::default()
            .title(text.network_devices_title())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT_DIM));
        let inner = modal_block.inner(popup);
        frame.render_widget(modal_block, popup);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

        match modal.mode {
            DeviceManagerMode::Browsing => {
                let rows = modal.rows();
                if rows.is_empty() {
                    let body = Paragraph::new(Line::from(Span::styled(
                        text.network_devices_none(),
                        Style::default().fg(theme::TEXT_SUBTLE),
                    )));
                    frame.render_widget(body, chunks[0]);
                } else {
                    // Section headers are their own (unselectable) items; the
                    // row→item mapping keeps the highlight on actual entries,
                    // never on a header line.
                    let mut items = Vec::new();
                    let mut item_for_row = Vec::with_capacity(rows.len());
                    let mut requests_header_drawn = false;
                    let mut devices_header_drawn = false;
                    for row in &rows {
                        match row {
                            DeviceManagerRow::Request(request) => {
                                if !requests_header_drawn {
                                    requests_header_drawn = true;
                                    items.push(ListItem::new(section_header_line(
                                        text.network_devices_section_requests(),
                                    )));
                                }
                                item_for_row.push(items.len());
                                items.push(ListItem::new(vec![Line::from(vec![
                                    Span::styled(
                                        "▸ ",
                                        Style::default().fg(theme::ACCENT_STRONG),
                                    ),
                                    Span::styled(
                                        request.applicantName.clone(),
                                        Style::default().add_modifier(Modifier::BOLD),
                                    ),
                                    Span::styled(
                                        format!(" · {}", request.spaceName),
                                        Style::default().fg(theme::TEXT_SUBTLE),
                                    ),
                                ])]));
                            }
                            DeviceManagerRow::Device(device) => {
                                if !devices_header_drawn {
                                    devices_header_drawn = true;
                                    items.push(ListItem::new(section_header_line(
                                        text.network_devices_section_devices(),
                                    )));
                                }
                                item_for_row.push(items.len());
                                items.push(ListItem::new(vec![self
                                    .device_manager_device_line(modal, device)]));
                            }
                        }
                    }
                    let mut state = ListState::default();
                    state.select(Some(
                        item_for_row[modal.selected.min(item_for_row.len() - 1)],
                    ));
                    let list = List::new(items)
                        .highlight_style(
                            Style::default()
                                .bg(theme::ACCENT_BG)
                                .fg(theme::TEXT)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol(">> ");
                    frame.render_stateful_widget(list, chunks[0], &mut state);
                }
            }
            DeviceManagerMode::ActionMenu => {
                if let Some((device, actions)) = modal.menu_device_id.as_ref().and_then(|id| {
                    modal
                        .topology
                        .devices
                        .iter()
                        .find(|device| &device.deviceId == id)
                        .map(|device| (device, modal.menu_actions(id)))
                }) {
                    let mut items = vec![ListItem::new(Line::from(vec![Span::styled(
                        device_manager_device_label(modal, device),
                        Style::default().add_modifier(Modifier::BOLD),
                    )]))];
                    items.extend(actions.iter().map(|action| {
                        ListItem::new(Line::from(Span::raw(
                            device_manager_action_label(text, *action),
                        )))
                    }));
                    let mut state = ListState::default();
                    // Item 0 is the unselectable device label header.
                    state.select(Some(
                        modal.menu_index.min(actions.len().saturating_sub(1)) + 1,
                    ));
                    let list = List::new(items)
                        .highlight_style(
                            Style::default()
                                .bg(theme::ACCENT_BG)
                                .fg(theme::TEXT)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol(">> ");
                    frame.render_stateful_widget(list, chunks[0], &mut state);
                }
            }
            DeviceManagerMode::AssignIdentity => {
                if let Some(device) = modal
                    .menu_device_id
                    .as_ref()
                    .and_then(|id| {
                        modal
                            .topology
                            .devices
                            .iter()
                            .find(|device| &device.deviceId == id)
                    })
                    .cloned()
                {
                    let roles = modal.sorted_roles();
                    let mut items = vec![ListItem::new(Line::from(vec![Span::styled(
                        device_manager_device_label(modal, &device),
                        Style::default().add_modifier(Modifier::BOLD),
                    )]))];
                    items.extend(roles.iter().map(|role| {
                        ListItem::new(Line::from(Span::raw(network_role_summary(role))))
                    }));
                    let mut state = ListState::default();
                    // Item 0 is the unselectable device label header.
                    state.select(Some(
                        modal.menu_index.min(roles.len().saturating_sub(1)) + 1,
                    ));
                    let list = List::new(items)
                        .highlight_style(
                            Style::default()
                                .bg(theme::ACCENT_BG)
                                .fg(theme::TEXT)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol(">> ");
                    frame.render_stateful_widget(list, chunks[0], &mut state);
                }
            }
            DeviceManagerMode::ConfirmRemove => {
                let warning = Paragraph::new(Text::from(
                    text.network_devices_remove_warning()
                        .split('\n')
                        .map(Line::from)
                        .collect::<Vec<_>>(),
                ))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(theme::ERROR_DIM));
                frame.render_widget(warning, chunks[0]);
            }
            DeviceManagerMode::ConfirmUnpair => {
                let label = modal
                    .menu_device_id
                    .as_ref()
                    .map(|id| match modal
                        .topology
                        .devices
                        .iter()
                        .find(|device| &device.deviceId == id)
                    {
                        Some(device) => device_manager_device_label(modal, device),
                        None => id.clone(),
                    })
                    .unwrap_or_default();
                let warning_text = text.network_unpair_warning(&label);
                let warning = Paragraph::new(Text::from(
                    warning_text
                        .split('\n')
                        .map(Line::from)
                        .collect::<Vec<_>>(),
                ))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(theme::ERROR_DIM));
                frame.render_widget(warning, chunks[0]);
            }
        }

        if !modal.initialized {
            let notice = Paragraph::new(Line::from(Span::styled(
                text.network_devices_not_initialized(),
                Style::default().fg(theme::ERROR_DIM),
            )));
            frame.render_widget(notice, chunks[1]);
        }
        let hint_text = match modal.mode {
            DeviceManagerMode::Browsing => text.network_devices_browse_hint(),
            DeviceManagerMode::ActionMenu => text.network_devices_menu_hint(),
            DeviceManagerMode::AssignIdentity => text.network_devices_assign_hint(),
            // The confirm body already ends with the Y/N hint.
            DeviceManagerMode::ConfirmRemove | DeviceManagerMode::ConfirmUnpair => "",
        };
        if !hint_text.is_empty() {
            let hint = Paragraph::new(Line::from(Span::styled(
                hint_text,
                Style::default().fg(theme::TEXT_SUBTLE),
            )));
            frame.render_widget(hint, chunks[2]);
        }
    }

    /// Builds one device row: connectivity dot, device label, and the
    /// policy/diagnostic tags in display order.
    fn device_manager_device_line(
        &self,
        modal: &DeviceManagerModal,
        device: &RuntimeDeviceSpaceDevice,
    ) -> Line<'static> {
        let text = self.text();
        let connectivity = if device.online {
            Span::styled("● ", Style::default().fg(theme::TEXT))
        } else {
            Span::styled("○ ", Style::default().fg(theme::TEXT_SUBTLE))
        };
        let mut spans = vec![
            connectivity,
            Span::styled(
                device_manager_device_label(modal, device),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];
        let mut tag = |condition: bool, label: &'static str, color: ratatui::style::Color| {
            if condition {
                spans.push(Span::styled(
                    format!(" · {label}"),
                    Style::default().fg(color),
                ));
            }
        };
        tag(
            device.deviceId == modal.topology.currentDeviceId,
            text.network_devices_self(),
            theme::TEXT_MUTED,
        );
        tag(
            modal.blocked.contains(&device.deviceId),
            text.network_devices_blocked(),
            theme::ERROR_DIM,
        );
        tag(
            !device.online,
            text.network_devices_offline(),
            theme::TEXT_SUBTLE,
        );
        tag(
            modal.topology.connections.iter().any(|connection| {
                connection.status == RuntimeDeviceSpaceConnectionStatus::VersionMismatch
                    && (connection.firstDeviceId == device.deviceId
                        || connection.secondDeviceId == device.deviceId)
            }),
            text.network_devices_version_mismatch(),
            theme::ERROR_DIM,
        );
        spans.push(Span::styled(
            format!(
                " · {}",
                device
                    .currentIdentity
                    .as_ref()
                    .map(|identity| identity.displayName.clone())
                    .unwrap_or_else(|| text.network_devices_no_identity().to_string())
            ),
            Style::default().fg(theme::TEXT_SUBTLE),
        ));
        Line::from(spans)
    }
}

fn section_header_line(label: &str) -> Line<'static> {
    Line::from(Span::styled(
        label.to_string(),
        Style::default()
            .fg(theme::TEXT_SUBTLE)
            .add_modifier(Modifier::BOLD),
    ))
}

fn blank_line() -> Line<'static> {
    Line::from("")
}

fn device_manager_device_label(
    modal: &DeviceManagerModal,
    device: &RuntimeDeviceSpaceDevice,
) -> String {
    network_device_label_by_id(&modal.topology, &device.deviceId)
        .unwrap_or_else(|_| device.deviceName.clone())
}

fn device_manager_action_label(text: super::i18n::TuiText, action: DeviceManagerAction) -> &'static str {
    match action {
        DeviceManagerAction::Admit => text.network_devices_menu_admit(),
        DeviceManagerAction::Disconnect => text.network_devices_menu_disconnect(),
        DeviceManagerAction::AssignIdentity => text.network_devices_menu_assign(),
        DeviceManagerAction::ClearIdentity => text.network_devices_menu_clear(),
        DeviceManagerAction::Unpair => text.network_devices_menu_unpair(),
        DeviceManagerAction::Remove => text.network_devices_menu_remove(),
    }
}

fn progress_bar(percent: u64, width: usize) -> String {
    let filled = ((percent.min(100) as usize) * width) / 100;
    format!("[{}{}]", "#".repeat(filled), "-".repeat(width - filled))
}

fn full_update_stage_label(text: super::i18n::TuiText, stage: FullUpdateStage) -> &'static str {
    match stage {
        FullUpdateStage::DownloadingPackage => text.downloading_full_update_package(),
        FullUpdateStage::Ready => text.package_ready(),
    }
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
            1 => Style::default().fg(theme::ACCENT),
            2 => Style::default().fg(theme::TEXT_MUTED),
            _ => Style::default().fg(theme::TEXT_SUBTLE),
        };
        spans.push(Span::styled(ch.to_string(), style));
    }
    Line::from(spans)
}
