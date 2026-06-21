use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem};
use ratatui::Frame;

use operit_runtime::data::model::FunctionType::FunctionType;
use operit_runtime::data::model::ModelConfigData::{
    ModelBuiltinTool, ModelCapabilities, ModelContextSpec, ModelRequestSpec,
    ModelSummarySettings, ResolvedModelConfig,
};

use super::super::i18n::TuiText;
use super::super::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainFocus {
    ToolCall,
    DirectImage,
    DirectAudio,
    DirectVideo,
    MaxContextLength,
    MaxContextMode,
    EnableSummary,
    SummaryDetails,
    StructuredTools,
    BuiltinTool(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SummaryFocus {
    EnableSummary,
    TokenThreshold,
    ByMessageCount,
    MessageCountThreshold,
}

#[derive(Clone, PartialEq)]
pub(crate) enum EditorLevel {
    /// Main editor: capabilities + context + summary toggle + actions
    Main,
    /// Summary detail editor
    Summary,
}

#[derive(Clone)]
pub(crate) struct EditorState {
    pub(crate) provider_id: String,
    pub(crate) model_id: String,
    pub(crate) provider_name: String,

    // Capabilities
    pub(crate) tool_call: bool,
    pub(crate) direct_image: bool,
    pub(crate) direct_audio: bool,
    pub(crate) direct_video: bool,

    // Context
    pub(crate) max_context_length: String,
    pub(crate) enable_max_context_mode: bool,

    // Summary
    pub(crate) enable_summary: bool,
    pub(crate) summary_token_threshold: String,
    pub(crate) enable_summary_by_message_count: bool,
    pub(crate) summary_message_count_threshold: String,

    // Builtin tools
    pub(crate) builtin_tools: Vec<ModelBuiltinTool>,

    // Request
    pub(crate) supports_structured_tools: bool,

    // UI state
    pub(crate) level: EditorLevel,
    pub(crate) focus_index: usize,
    pub(crate) editing_field: bool,

    // Connection test result (stored as Option<String> to avoid importing the full type)
    pub(crate) test_result: Option<String>,
    pub(crate) testing: bool,
}

impl EditorState {
    pub(crate) const CHAT_FUNCTION: FunctionType = FunctionType::CHAT;

    pub(crate) fn new(
        provider_id: String,
        provider_name: String,
        model_id: String,
        config: &ResolvedModelConfig,
    ) -> Self {
        Self {
            provider_id,
            model_id,
            provider_name,
            tool_call: config.capabilities.toolCall,
            direct_image: config.capabilities.directImage,
            direct_audio: config.capabilities.directAudio,
            direct_video: config.capabilities.directVideo,
            max_context_length: format!("{:.0}", config.context.maxContextLength),
            enable_max_context_mode: config.context.enableMaxContextMode,
            enable_summary: config.summary.enableSummary,
            summary_token_threshold: format!("{:.0}", config.summary.summaryTokenThreshold),
            enable_summary_by_message_count: config.summary.enableSummaryByMessageCount,
            summary_message_count_threshold: format!(
                "{}",
                config.summary.summaryMessageCountThreshold
            ),
            builtin_tools: config.builtinTools.clone(),
            supports_structured_tools: config.request.supportsStructuredTools,
            level: EditorLevel::Main,
            focus_index: 0,
            editing_field: false,
            test_result: None,
            testing: false,
        }
    }

    pub(crate) fn main_focus_items(&self) -> Vec<MainFocus> {
        let mut items = vec![
            MainFocus::ToolCall,
            MainFocus::DirectImage,
            MainFocus::DirectAudio,
            MainFocus::DirectVideo,
            MainFocus::MaxContextLength,
            MainFocus::MaxContextMode,
            MainFocus::EnableSummary,
        ];
        if self.enable_summary {
            items.push(MainFocus::SummaryDetails);
        }
        items.push(MainFocus::StructuredTools);
        items.extend((0..self.builtin_tools.len()).map(MainFocus::BuiltinTool));
        items
    }

    pub(crate) fn summary_focus_items(&self) -> Vec<SummaryFocus> {
        let mut items = vec![SummaryFocus::EnableSummary];
        if self.enable_summary {
            items.push(SummaryFocus::TokenThreshold);
            items.push(SummaryFocus::ByMessageCount);
            if self.enable_summary_by_message_count {
                items.push(SummaryFocus::MessageCountThreshold);
            }
        }
        items
    }

    pub(crate) fn clamp_focus(&mut self) {
        let len = match self.level {
            EditorLevel::Main => self.main_focus_items().len(),
            EditorLevel::Summary => self.summary_focus_items().len(),
        };
        self.focus_index = self.focus_index.min(len.saturating_sub(1));
    }

    pub(crate) fn focused_main_item(&self) -> Option<MainFocus> {
        self.main_focus_items().get(self.focus_index).copied()
    }

    pub(crate) fn focused_summary_item(&self) -> Option<SummaryFocus> {
        self.summary_focus_items().get(self.focus_index).copied()
    }

    pub(crate) fn into_config_changes(&self) -> EditorChanges {
        let ctx_len: f32 = self.max_context_length.parse().unwrap_or(128.0);
        EditorChanges {
            capabilities: ModelCapabilities {
                toolCall: self.tool_call,
                directImage: self.direct_image,
                directAudio: self.direct_audio,
                directVideo: self.direct_video,
            },
            context: ModelContextSpec {
                maxContextLength: ctx_len,
                enableMaxContextMode: self.enable_max_context_mode,
            },
            request: ModelRequestSpec {
                supportsStructuredTools: self.supports_structured_tools,
            },
            summary: ModelSummarySettings {
                enableSummary: self.enable_summary,
                summaryTokenThreshold: self
                    .summary_token_threshold
                    .parse()
                    .unwrap_or(4096.0),
                enableSummaryByMessageCount: self.enable_summary_by_message_count,
                summaryMessageCountThreshold: self
                    .summary_message_count_threshold
                    .parse()
                    .unwrap_or(10),
            },
            builtin_tools: self.builtin_tools.clone(),
        }
    }

    pub(crate) fn render(&self, area: Rect, frame: &mut Frame, text: TuiText) {
        match self.level {
            EditorLevel::Main => self.render_main(area, frame, text),
            EditorLevel::Summary => self.render_summary(area, frame, text),
        }
    }

    fn render_main(&self, area: Rect, frame: &mut Frame, text: TuiText) {
        let items = self.build_main_items(text);
        frame.render_widget(
            List::new(items),
            area,
        );
    }

    fn build_main_items(&self, text: TuiText) -> Vec<ListItem<'static>> {
        let mut items: Vec<ListItem> = Vec::new();

        // Provider/model header
        items.push(ListItem::new(vec![Line::from(Span::styled(
            format!("{} · {}", self.provider_name, self.model_id),
            Style::default()
                .fg(theme::TEXT_SUBTLE)
                .add_modifier(Modifier::ITALIC),
        ))]));

        // --- Capabilities ---
        items.push(ListItem::new(vec![Line::from(Span::styled(
            format!("── {} ──", text.config_editor_section_capabilities()),
            Style::default().fg(theme::ACCENT_DIM),
        ))]));

        self.push_toggle_item(&mut items, MainFocus::ToolCall, text.config_editor_tool_call(), self.tool_call, text);
        self.push_toggle_item(&mut items, MainFocus::DirectImage, text.config_editor_direct_image(), self.direct_image, text);
        self.push_toggle_item(&mut items, MainFocus::DirectAudio, text.config_editor_direct_audio(), self.direct_audio, text);
        self.push_toggle_item(&mut items, MainFocus::DirectVideo, text.config_editor_direct_video(), self.direct_video, text);

        // --- Context ---
        items.push(ListItem::new(vec![Line::from(Span::styled(
            format!("── {} ──", text.config_editor_section_context()),
            Style::default().fg(theme::ACCENT_DIM),
        ))]));

        self.push_edit_item(&mut items, MainFocus::MaxContextLength, text.config_editor_max_context_length(), &self.max_context_length);
        self.push_toggle_item(&mut items, MainFocus::MaxContextMode, text.config_editor_max_context_mode(), self.enable_max_context_mode, text);

        // --- Summary ---
        items.push(ListItem::new(vec![Line::from(Span::styled(
            format!("── {} ──", text.config_editor_section_summary()),
            Style::default().fg(theme::ACCENT_DIM),
        ))]));

        self.push_toggle_item(&mut items, MainFocus::EnableSummary, text.config_editor_enable_summary(), self.enable_summary, text);
        if self.enable_summary {
            self.push_nav_item(
                &mut items,
                MainFocus::SummaryDetails,
                text.config_editor_summary_details(),
                text.config_editor_enter_details(),
            );
        }

        // --- Advanced ---
        items.push(ListItem::new(vec![Line::from(Span::styled(
            format!("── {} ──", text.config_editor_section_advanced()),
            Style::default().fg(theme::ACCENT_DIM),
        ))]));

        self.push_toggle_item(&mut items, MainFocus::StructuredTools, text.config_editor_structured_tools(), self.supports_structured_tools, text);

        if !self.builtin_tools.is_empty() {
            items.push(ListItem::new(vec![Line::from(Span::styled(
                format!("── {} ──", text.config_editor_section_builtin_tools()),
                Style::default().fg(theme::ACCENT_DIM),
            ))]));

            for (i, tool) in self.builtin_tools.iter().enumerate() {
                let value = if tool.enabled { text.config_enabled() } else { text.config_disabled() };
                let desc = format!("{} ({})", tool.displayName, value);
                let is_focused = self.focused_main_item() == Some(MainFocus::BuiltinTool(i));
                if is_focused {
                    items.push(ListItem::new(vec![Line::from(Span::styled(
                        desc,
                        Style::default()
                            .bg(theme::ACCENT_BG)
                            .fg(theme::TEXT)
                            .add_modifier(Modifier::BOLD),
                    ))]));
                } else {
                    items.push(ListItem::new(vec![Line::from(Span::styled(
                        desc,
                        if tool.enabled {
                            Style::default().fg(theme::TEXT)
                        } else {
                            Style::default().fg(theme::TEXT_MUTED)
                        },
                    ))]));
                }
            }
        }

        // --- Actions ---
        items.push(ListItem::new(vec![Line::from(Span::styled(
            format!("── {} ──", text.config_editor_section_actions()),
            Style::default().fg(theme::ACCENT_DIM),
        ))]));

        items
    }

    fn push_toggle_item(
        &self,
        items: &mut Vec<ListItem<'static>>,
        focus: MainFocus,
        label: &'static str,
        value: bool,
        text: TuiText,
    ) {
        let is_focused = self.focused_main_item() == Some(focus);
        let val_str = if value { text.config_yes() } else { text.config_no() };
        if is_focused {
            items.push(ListItem::new(vec![Line::from(Span::styled(
                format!("{}: {}", label, val_str),
                Style::default()
                    .bg(theme::ACCENT_BG)
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ))]));
        } else {
            items.push(ListItem::new(vec![Line::from(Span::styled(
                format!("{}: {}", label, val_str),
                if value {
                    Style::default().fg(theme::TEXT)
                } else {
                    Style::default().fg(theme::TEXT_MUTED)
                },
            ))]));
        }
    }

    fn push_edit_item(
        &self,
        items: &mut Vec<ListItem<'static>>,
        focus: MainFocus,
        label: &'static str,
        value: &str,
    ) {
        let is_focused = self.focused_main_item() == Some(focus);
        if is_focused {
            if self.editing_field {
                items.push(ListItem::new(vec![Self::editing_value_line(label, value)]));
                return;
            }
            let display = format!("{}: {}", label, value);
            items.push(ListItem::new(vec![Line::from(Span::styled(
                display,
                Style::default()
                    .bg(theme::ACCENT_BG)
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ))]));
        } else {
            items.push(ListItem::new(vec![Line::from(Span::styled(
                format!("{}: {}", label, value),
                Style::default().fg(theme::TEXT_MUTED),
            ))]));
        }
    }

    fn editing_value_line(label: &'static str, value: &str) -> Line<'static> {
        let input_style = Style::default().bg(theme::ACCENT_BG).fg(theme::TEXT);
        Line::from(vec![
            Span::styled(format!("{}: ", label), input_style),
            Span::styled(value.to_string(), input_style),
            Span::styled(
                " ",
                Style::default()
                    .bg(theme::SELECTION_BG)
                    .fg(theme::SELECTION_TEXT),
            ),
        ])
    }

    fn push_nav_item(
        &self,
        items: &mut Vec<ListItem<'static>>,
        focus: MainFocus,
        label: &'static str,
        hint: &'static str,
    ) {
        let is_focused = self.focused_main_item() == Some(focus);
        if is_focused {
            items.push(ListItem::new(vec![
                Line::from(Span::styled(
                    format!("{}  [{}]", label, hint),
                    Style::default()
                        .bg(theme::ACCENT_BG)
                        .fg(theme::TEXT)
                        .add_modifier(Modifier::BOLD),
                )),
            ]));
        } else {
            items.push(ListItem::new(vec![Line::from(Span::styled(
                label,
                Style::default().fg(theme::TEXT),
            ))]));
        }
    }

    fn render_summary(&self, area: Rect, frame: &mut Frame, text: TuiText) {
        let items = self.build_summary_items(text);
        frame.render_widget(
            List::new(items),
            area,
        );
    }

    fn build_summary_items(&self, text: TuiText) -> Vec<ListItem<'static>> {
        let mut items = Vec::new();

        items.push(ListItem::new(vec![Line::from(Span::styled(
            format!("── {} ──", text.config_editor_summary_settings()),
            Style::default().fg(theme::ACCENT_DIM),
        ))]));

        // Enable Summary toggle
        let is_focused = self.focused_summary_item() == Some(SummaryFocus::EnableSummary);
        if is_focused {
            items.push(ListItem::new(vec![Line::from(Span::styled(
                format!("{}: {}", text.config_editor_enable_summary(), if self.enable_summary { text.config_yes() } else { text.config_no() }),
                Style::default()
                    .bg(theme::ACCENT_BG)
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ))]));
        } else {
            items.push(ListItem::new(vec![Line::from(Span::styled(
                format!("{}: {}", text.config_editor_enable_summary(), if self.enable_summary { text.config_yes() } else { text.config_no() }),
                if self.enable_summary {
                    Style::default().fg(theme::TEXT)
                } else {
                    Style::default().fg(theme::TEXT_MUTED)
                },
            ))]));
        }

        if self.enable_summary {
            // Token Threshold
            let is_focused = self.focused_summary_item() == Some(SummaryFocus::TokenThreshold);
            if is_focused {
                if self.editing_field {
                    items.push(ListItem::new(vec![Self::editing_value_line(
                        text.config_editor_token_threshold(),
                        &self.summary_token_threshold,
                    )]));
                } else {
                    items.push(ListItem::new(vec![Line::from(Span::styled(
                        format!("{}: {}", text.config_editor_token_threshold(), self.summary_token_threshold),
                        Style::default()
                            .bg(theme::ACCENT_BG)
                            .fg(theme::TEXT)
                            .add_modifier(Modifier::BOLD),
                    ))]));
                }
            } else {
                items.push(ListItem::new(vec![Line::from(Span::styled(
                    format!("{}: {}", text.config_editor_token_threshold(), self.summary_token_threshold),
                    Style::default().fg(theme::TEXT_MUTED),
                ))]));
            }

            // Enable Summary By Message Count toggle
            let is_focused = self.focused_summary_item() == Some(SummaryFocus::ByMessageCount);
            if is_focused {
                items.push(ListItem::new(vec![Line::from(Span::styled(
                    format!("{}: {}", text.config_editor_by_message_count(), if self.enable_summary_by_message_count { text.config_yes() } else { text.config_no() }),
                    Style::default()
                        .bg(theme::ACCENT_BG)
                        .fg(theme::TEXT)
                        .add_modifier(Modifier::BOLD),
                ))]));
            } else {
                items.push(ListItem::new(vec![Line::from(Span::styled(
                    format!("{}: {}", text.config_editor_by_message_count(), if self.enable_summary_by_message_count { text.config_yes() } else { text.config_no() }),
                    if self.enable_summary_by_message_count {
                        Style::default().fg(theme::TEXT)
                    } else {
                        Style::default().fg(theme::TEXT_MUTED)
                    },
                ))]));
            }

            if self.enable_summary_by_message_count {
                let is_focused = self.focused_summary_item() == Some(SummaryFocus::MessageCountThreshold);
                if is_focused {
                    if self.editing_field {
                        items.push(ListItem::new(vec![Self::editing_value_line(
                            text.config_editor_message_count_threshold(),
                            &self.summary_message_count_threshold,
                        )]));
                    } else {
                        items.push(ListItem::new(vec![Line::from(Span::styled(
                            format!("{}: {}", text.config_editor_message_count_threshold(), self.summary_message_count_threshold),
                            Style::default()
                                .bg(theme::ACCENT_BG)
                                .fg(theme::TEXT)
                                .add_modifier(Modifier::BOLD),
                        ))]));
                    }
                } else {
                    items.push(ListItem::new(vec![Line::from(Span::styled(
                        format!("{}: {}", text.config_editor_message_count_threshold(), self.summary_message_count_threshold),
                        Style::default().fg(theme::TEXT_MUTED),
                    ))]));
                }
            }
        }

        items
    }
}

#[derive(Clone)]
pub(crate) struct EditorChanges {
    pub(crate) capabilities: ModelCapabilities,
    pub(crate) context: ModelContextSpec,
    pub(crate) request: ModelRequestSpec,
    pub(crate) summary: ModelSummarySettings,
    pub(crate) builtin_tools: Vec<ModelBuiltinTool>,
}
