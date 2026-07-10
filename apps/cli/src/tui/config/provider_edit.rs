use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::Frame;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use operit_model::ModelConfigData::ApiProviderType;

use crate::tui::theme;

fn truncate_to_width(s: &str, max_width: usize) -> String {
    let mut result = String::with_capacity(max_width);
    let mut w = 0usize;
    for ch in s.chars() {
        let cw = ch.width().unwrap_or(1);
        if w + cw > max_width {
            break;
        }
        w += cw;
        result.push(ch);
    }
    result
}

#[derive(Clone)]
pub(crate) struct FormField {
    pub(crate) label: &'static str,
    pub(crate) value: String,
    pub(crate) cursor: usize,
}

#[derive(Clone)]
pub(crate) struct FormState {
    pub(crate) fields: Vec<FormField>,
    pub(crate) focus_index: usize,
    pub(crate) editing_field: bool,
    pub(crate) advanced: bool,
    pub(crate) editing_provider_id: Option<String>,
    pub(crate) show_type_selector: bool,
    pub(crate) type_selector_index: usize,
    pub(crate) type_selector_filter: String,
    pub(crate) type_selector_filtered: Vec<usize>,
}

fn all_provider_type_names() -> Vec<String> {
    let variants = [
        ApiProviderType::OPENAI,
        ApiProviderType::OPENAI_RESPONSES,
        ApiProviderType::OPENAI_RESPONSES_GENERIC,
        ApiProviderType::OPENAI_GENERIC,
        ApiProviderType::ANTHROPIC,
        ApiProviderType::ANTHROPIC_GENERIC,
        ApiProviderType::GOOGLE,
        ApiProviderType::GEMINI_GENERIC,
        ApiProviderType::BAIDU,
        ApiProviderType::ALIYUN,
        ApiProviderType::XUNFEI,
        ApiProviderType::ZHIPU,
        ApiProviderType::BAICHUAN,
        ApiProviderType::MOONSHOT,
        ApiProviderType::MIMO,
        ApiProviderType::DEEPSEEK,
        ApiProviderType::MISTRAL,
        ApiProviderType::SILICONFLOW,
        ApiProviderType::IFLOW,
        ApiProviderType::OPENROUTER,
        ApiProviderType::FOUR_ROUTER,
        ApiProviderType::NOUS_PORTAL,
        ApiProviderType::INFINIAI,
        ApiProviderType::ALIPAY_BAILING,
        ApiProviderType::DOUBAO,
        ApiProviderType::NVIDIA,
        ApiProviderType::LMSTUDIO,
        ApiProviderType::OLLAMA,
        ApiProviderType::OPENAI_LOCAL,
        ApiProviderType::MNN,
        ApiProviderType::LLAMA_CPP,
        ApiProviderType::PPINFRA,
        ApiProviderType::NOVITA,
        ApiProviderType::OTHER,
    ];
    variants.iter().map(|v| v.name().to_string()).collect()
}

impl FormState {
    pub(crate) fn new_create() -> Self {
        Self {
            fields: vec![
                FormField {
                    label: "Name",
                    value: String::new(),
                    cursor: 0,
                },
                FormField {
                    label: "Provider Type",
                    value: String::new(),
                    cursor: 0,
                },
                FormField {
                    label: "Endpoint",
                    value: String::new(),
                    cursor: 0,
                },
                FormField {
                    label: "API Key",
                    value: String::new(),
                    cursor: 0,
                },
                FormField {
                    label: "Custom Headers (JSON)",
                    value: "{}".to_string(),
                    cursor: 2,
                },
                FormField {
                    label: "Request Limit / min",
                    value: "0".to_string(),
                    cursor: 1,
                },
                FormField {
                    label: "Max Concurrent Requests",
                    value: "0".to_string(),
                    cursor: 1,
                },
            ],
            focus_index: 0,
            editing_field: false,
            advanced: false,
            editing_provider_id: None,
            show_type_selector: false,
            type_selector_index: 0,
            type_selector_filter: String::new(),
            type_selector_filtered: (0..all_provider_type_names().len()).collect(),
        }
    }

    pub(crate) fn new_edit(profile: &operit_model::ModelConfigData::ProviderProfile) -> Self {
        Self {
            fields: vec![
                FormField {
                    label: "Name",
                    value: profile.name.clone(),
                    cursor: profile.name.len(),
                },
                FormField {
                    label: "Provider Type",
                    value: profile.providerTypeId.clone(),
                    cursor: profile.providerTypeId.len(),
                },
                FormField {
                    label: "Endpoint",
                    value: profile.endpoint.clone(),
                    cursor: profile.endpoint.len(),
                },
                FormField {
                    label: "API Key",
                    value: profile.apiKey.clone(),
                    cursor: profile.apiKey.len(),
                },
                FormField {
                    label: "Custom Headers (JSON)",
                    value: if profile.customHeaders.is_empty() || profile.customHeaders == "{}" {
                        "{}".to_string()
                    } else {
                        profile.customHeaders.clone()
                    },
                    cursor: 0,
                },
                FormField {
                    label: "Request Limit / min",
                    value: profile.requestLimitPerMinute.to_string(),
                    cursor: 0,
                },
                FormField {
                    label: "Max Concurrent Requests",
                    value: profile.maxConcurrentRequests.to_string(),
                    cursor: 0,
                },
            ],
            focus_index: 0,
            editing_field: false,
            advanced: false,
            editing_provider_id: Some(profile.id.clone()),
            show_type_selector: false,
            type_selector_index: 0,
            type_selector_filter: String::new(),
            type_selector_filtered: (0..all_provider_type_names().len()).collect(),
        }
    }

    pub(crate) fn open_type_selector(&mut self) {
        self.show_type_selector = true;
        self.type_selector_filter.clear();
        let names = all_provider_type_names();
        // Find the current value in the list to pre-select it
        let current = &self.fields[1].value.to_ascii_uppercase();
        self.type_selector_index = names
            .iter()
            .position(|n| n.to_ascii_uppercase() == *current)
            .unwrap_or(0);
        self.update_type_filter();
    }

    pub(crate) fn update_type_filter(&mut self) {
        let names = all_provider_type_names();
        let filter = self.type_selector_filter.to_ascii_lowercase();
        if filter.is_empty() {
            self.type_selector_filtered = (0..names.len()).collect();
        } else {
            self.type_selector_filtered = names
                .iter()
                .enumerate()
                .filter(|(_, n)| n.to_ascii_lowercase().contains(&filter))
                .map(|(i, _)| i)
                .collect();
        }
        if self.type_selector_index >= self.type_selector_filtered.len() {
            self.type_selector_index = self.type_selector_filtered.len().saturating_sub(1);
        }
    }

    pub(crate) fn confirm_type_selection(&mut self) {
        let names = all_provider_type_names();
        if let Some(&idx) = self.type_selector_filtered.get(self.type_selector_index) {
            if let Some(name) = names.get(idx) {
                self.fields[1].value = name.clone();
                self.fields[1].cursor = name.len();
            }
        }
        self.show_type_selector = false;
    }

    pub(crate) fn render(&self, area: Rect, frame: &mut Frame) {
        if self.show_type_selector {
            self.render_type_selector(area, frame);
            return;
        }
        let visible_count = if self.advanced { self.fields.len() } else { 4 };

        let items: Vec<ListItem> = self.fields[..visible_count.min(self.fields.len())]
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let is_focused = i == self.focus_index;
                let label_style = if is_focused {
                    Style::default()
                        .fg(theme::ACCENT_STRONG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::ACCENT)
                };

                let value_line = if is_focused && field.label != "Provider Type" {
                    if self.editing_field {
                        // In edit mode: show cursor
                        let cursor = field.cursor.min(field.value.len());
                        let max_visible = area.width.saturating_sub(3) as usize;
                        let value_width = field.value.width();

                        if value_width <= max_visible {
                            if field.value.is_empty() {
                                Line::from(Span::styled(
                                    " ",
                                    Style::default()
                                        .bg(theme::SELECTION_BG)
                                        .fg(theme::SELECTION_TEXT),
                                ))
                            } else if cursor < field.value.len() {
                                let before = field.value[..cursor].to_string();
                                let at = field.value[cursor..cursor + 1].to_string();
                                let after = field.value[cursor + 1..].to_string();
                                Line::from(vec![
                                    Span::styled(
                                        before,
                                        Style::default().bg(theme::ACCENT_BG).fg(theme::TEXT),
                                    ),
                                    Span::styled(
                                        at,
                                        Style::default()
                                            .bg(theme::SELECTION_BG)
                                            .fg(theme::SELECTION_TEXT),
                                    ),
                                    Span::styled(
                                        after,
                                        Style::default().bg(theme::ACCENT_BG).fg(theme::TEXT),
                                    ),
                                ])
                            } else {
                                Line::from(vec![
                                    Span::styled(
                                        field.value.clone(),
                                        Style::default().bg(theme::ACCENT_BG).fg(theme::TEXT),
                                    ),
                                    Span::styled(" ", Style::default().bg(theme::SELECTION_BG)),
                                ])
                            }
                        } else {
                            // Need horizontal scroll
                            let cursor_width = field.value[..cursor].width();
                            let target_offset = (max_visible * 2 / 3).saturating_sub(1);
                            let scroll_to = if cursor_width > target_offset {
                                cursor_width.saturating_sub(target_offset)
                            } else {
                                0usize
                            };

                            let byte_offset = {
                                let mut w = 0usize;
                                let mut bo = 0usize;
                                for (b, ch) in field.value.char_indices() {
                                    if w >= scroll_to {
                                        bo = b;
                                        break;
                                    }
                                    w += ch.width().unwrap_or(1);
                                    bo = b + ch.len_utf8();
                                }
                                bo
                            };

                            let visible_sub = &field.value[byte_offset..];
                            let visible_str = truncate_to_width(visible_sub, max_visible);

                            let cursor_in_visible =
                                cursor.saturating_sub(byte_offset).min(visible_str.len());
                            if cursor_in_visible < visible_str.len() {
                                let cv = &visible_str[..cursor_in_visible];
                                let cc = &visible_str[cursor_in_visible..cursor_in_visible + 1];
                                let cr = &visible_str[cursor_in_visible + 1..];
                                Line::from(vec![
                                    Span::styled(
                                        cv.to_string(),
                                        Style::default().bg(theme::ACCENT_BG).fg(theme::TEXT),
                                    ),
                                    Span::styled(
                                        cc.to_string(),
                                        Style::default()
                                            .bg(theme::SELECTION_BG)
                                            .fg(theme::SELECTION_TEXT),
                                    ),
                                    Span::styled(
                                        cr.to_string(),
                                        Style::default().bg(theme::ACCENT_BG).fg(theme::TEXT),
                                    ),
                                ])
                            } else {
                                Line::from(vec![
                                    Span::styled(
                                        visible_str,
                                        Style::default().bg(theme::ACCENT_BG).fg(theme::TEXT),
                                    ),
                                    Span::styled(" ", Style::default().bg(theme::SELECTION_BG)),
                                ])
                            }
                        }
                    } else {
                        // Not editing: show value with highlight bg but no cursor
                        Line::from(Span::styled(
                            &field.value,
                            Style::default().bg(theme::ACCENT_BG).fg(theme::TEXT),
                        ))
                    }
                } else if is_focused && field.label == "Provider Type" {
                    // Provider Type: show with accent bg, hint to press Enter
                    Line::from(Span::styled(
                        &field.value,
                        Style::default()
                            .bg(theme::ACCENT_BG)
                            .fg(theme::TEXT)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else if field.value.is_empty() {
                    Line::from(Span::styled(
                        "(empty)",
                        Style::default().fg(theme::TEXT_SUBTLE),
                    ))
                } else {
                    Line::from(Span::styled(
                        &field.value,
                        Style::default().fg(theme::TEXT_MUTED),
                    ))
                };

                ListItem::new(vec![
                    Line::from(Span::styled(field.label, label_style)),
                    value_line,
                ])
            })
            .collect();

        let mut state = ListState::default();
        state.select(Some(self.focus_index.min(visible_count.saturating_sub(1))));
        frame.render_stateful_widget(List::new(items), area, &mut state);
    }

    fn render_type_selector(&self, area: Rect, frame: &mut Frame) {
        let names = all_provider_type_names();

        // Search input
        let search_display = if self.type_selector_filter.is_empty() {
            Span::styled(
                "Type to filter provider types...",
                Style::default().fg(theme::TEXT_SUBTLE),
            )
        } else {
            Span::styled(&self.type_selector_filter, Style::default())
        };
        let search_line = Line::from(vec![
            Span::styled("> ", Style::default().fg(theme::ACCENT)),
            search_display,
        ]);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(search_line),
            ratatui::layout::Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
        );

        // Items list below search
        let list_area = ratatui::layout::Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height.saturating_sub(1),
        };

        let items: Vec<ListItem> = self
            .type_selector_filtered
            .iter()
            .map(|&i| {
                let name = names.get(i).map(|s| s.as_str()).unwrap_or("?");
                ListItem::new(Line::from(Span::styled(name, Style::default())))
            })
            .collect();

        let mut state = ListState::default();
        if !self.type_selector_filtered.is_empty() {
            state.select(Some(
                self.type_selector_index
                    .min(self.type_selector_filtered.len().saturating_sub(1)),
            ));
        }
        frame.render_stateful_widget(
            List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(theme::ACCENT_BG)
                        .fg(theme::TEXT)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> "),
            list_area,
            &mut state,
        );
    }
}
