use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem};
use ratatui::Frame;

use operit_model::FunctionType::FunctionType;
use operit_model::ModelConfigData::{
    ModelBuiltinTool, ModelCapabilities, ModelContextSpec, ModelRequestSpec, ModelSummarySettings,
    ResolvedModelConfig,
};
use operit_model::ModelThinking::ThinkingLevel;

use super::super::i18n::TuiText;
use super::super::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainFocus {
    ToolCall,
    DirectImage,
    DirectAudio,
    DirectVideo,
    MaxContextLength,
    EnableSummary,
    SummaryDetails,
    StructuredTools,
    Thinking,
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

    // Summary
    pub(crate) enable_summary: bool,
    pub(crate) summary_token_threshold: String,
    pub(crate) enable_summary_by_message_count: bool,
    pub(crate) summary_message_count_threshold: String,

    // Builtin tools
    pub(crate) builtin_tools: Vec<ModelBuiltinTool>,

    // Request
    pub(crate) supports_structured_tools: bool,
    pub(crate) enable_claude_1h_prompt_cache: bool,

    // Thinking: the levels the model offers and the one its requests use.
    pub(crate) thinking_levels: Vec<ThinkingLevel>,
    pub(crate) thinking_level: Option<ThinkingLevel>,
    pub(crate) thinking_level_changed: bool,

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
            enable_summary: config.summary.enableSummary,
            summary_token_threshold: config.summary.summaryTokenThreshold.to_string(),
            enable_summary_by_message_count: config.summary.enableSummaryByMessageCount,
            summary_message_count_threshold: format!(
                "{}",
                config.summary.summaryMessageCountThreshold
            ),
            builtin_tools: config.builtinTools.clone(),
            supports_structured_tools: config.request.supportsStructuredTools,
            enable_claude_1h_prompt_cache: config.request.enableClaude1hPromptCache,
            thinking_levels: Vec::new(),
            thinking_level: config.thinkingLevel,
            thinking_level_changed: false,
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
            MainFocus::EnableSummary,
        ];
        if self.enable_summary {
            items.push(MainFocus::SummaryDetails);
        }
        items.push(MainFocus::StructuredTools);
        if !self.thinking_levels.is_empty() {
            items.push(MainFocus::Thinking);
        }
        items.extend((0..self.builtin_tools.len()).map(MainFocus::BuiltinTool));
        items
    }

    /// Moves the thinking level to the next one the model offers, wrapping around.
    pub(crate) fn cycle_thinking_level(&mut self) {
        if self.thinking_levels.is_empty() {
            return;
        }
        let next = match self
            .thinking_level
            .and_then(|level| self.thinking_levels.iter().position(|offered| *offered == level))
        {
            Some(index) => (index + 1) % self.thinking_levels.len(),
            None => 0,
        };
        self.thinking_level = Some(self.thinking_levels[next]);
        self.thinking_level_changed = true;
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
            },
            request: ModelRequestSpec {
                supportsStructuredTools: self.supports_structured_tools,
                enableClaude1hPromptCache: self.enable_claude_1h_prompt_cache,
            },
            summary: ModelSummarySettings {
                enableSummary: self.enable_summary,
                summaryTokenThreshold: self.summary_token_threshold.parse().unwrap_or(4096.0),
                enableSummaryByMessageCount: self.enable_summary_by_message_count,
                summaryMessageCountThreshold: self
                    .summary_message_count_threshold
                    .parse()
                    .unwrap_or(10),
            },
            builtin_tools: self.builtin_tools.clone(),
            thinking_level: self.thinking_level.filter(|_| self.thinking_level_changed),
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
        frame.render_widget(List::new(items), area);
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

        self.push_toggle_item(
            &mut items,
            MainFocus::ToolCall,
            text.config_editor_tool_call(),
            self.tool_call,
            text,
        );

        if !self.thinking_levels.is_empty() {
            items.push(ListItem::new(vec![Line::from(Span::styled(
                "── Thinking ──",
                Style::default().fg(theme::ACCENT_DIM),
            ))]));
            let current = match self.thinking_level {
                Some(level) => level.name(),
                None => "-",
            };
            self.push_edit_item(&mut items, MainFocus::Thinking, "Thinking level", current);
            items.push(ListItem::new(vec![Line::from(Span::styled(
                format!(
                    "Enter cycles: {}",
                    self.thinking_levels
                        .iter()
                        .map(|level| level.name())
                        .collect::<Vec<_>>()
                        .join(" / ")
                ),
                Style::default().fg(theme::TEXT_SUBTLE),
            ))]));
        }
        self.push_toggle_item(
            &mut items,
            MainFocus::DirectImage,
            text.config_editor_direct_image(),
            self.direct_image,
            text,
        );
        self.push_toggle_item(
            &mut items,
            MainFocus::DirectAudio,
            text.config_editor_direct_audio(),
            self.direct_audio,
            text,
        );
        self.push_toggle_item(
            &mut items,
            MainFocus::DirectVideo,
            text.config_editor_direct_video(),
            self.direct_video,
            text,
        );

        // --- Context ---
        items.push(ListItem::new(vec![Line::from(Span::styled(
            format!("── {} ──", text.config_editor_section_context()),
            Style::default().fg(theme::ACCENT_DIM),
        ))]));

        self.push_edit_item(
            &mut items,
            MainFocus::MaxContextLength,
            text.config_editor_max_context_length(),
            &self.max_context_length,
        );

        // --- Summary ---
        items.push(ListItem::new(vec![Line::from(Span::styled(
            format!("── {} ──", text.config_editor_section_summary()),
            Style::default().fg(theme::ACCENT_DIM),
        ))]));

        self.push_toggle_item(
            &mut items,
            MainFocus::EnableSummary,
            text.config_editor_enable_summary(),
            self.enable_summary,
            text,
        );
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

        self.push_toggle_item(
            &mut items,
            MainFocus::StructuredTools,
            text.config_editor_structured_tools(),
            self.supports_structured_tools,
            text,
        );

        if !self.builtin_tools.is_empty() {
            items.push(ListItem::new(vec![Line::from(Span::styled(
                format!("── {} ──", text.config_editor_section_builtin_tools()),
                Style::default().fg(theme::ACCENT_DIM),
            ))]));

            for (i, tool) in self.builtin_tools.iter().enumerate() {
                let value = if tool.enabled {
                    text.config_enabled()
                } else {
                    text.config_disabled()
                };
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
        let val_str = if value {
            text.config_yes()
        } else {
            text.config_no()
        };
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
            items.push(ListItem::new(vec![Line::from(Span::styled(
                format!("{}  [{}]", label, hint),
                Style::default()
                    .bg(theme::ACCENT_BG)
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ))]));
        } else {
            items.push(ListItem::new(vec![Line::from(Span::styled(
                label,
                Style::default().fg(theme::TEXT),
            ))]));
        }
    }

    fn render_summary(&self, area: Rect, frame: &mut Frame, text: TuiText) {
        let items = self.build_summary_items(text);
        frame.render_widget(List::new(items), area);
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
                format!(
                    "{}: {}",
                    text.config_editor_enable_summary(),
                    if self.enable_summary {
                        text.config_yes()
                    } else {
                        text.config_no()
                    }
                ),
                Style::default()
                    .bg(theme::ACCENT_BG)
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ))]));
        } else {
            items.push(ListItem::new(vec![Line::from(Span::styled(
                format!(
                    "{}: {}",
                    text.config_editor_enable_summary(),
                    if self.enable_summary {
                        text.config_yes()
                    } else {
                        text.config_no()
                    }
                ),
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
                        format!(
                            "{}: {}",
                            text.config_editor_token_threshold(),
                            self.summary_token_threshold
                        ),
                        Style::default()
                            .bg(theme::ACCENT_BG)
                            .fg(theme::TEXT)
                            .add_modifier(Modifier::BOLD),
                    ))]));
                }
            } else {
                items.push(ListItem::new(vec![Line::from(Span::styled(
                    format!(
                        "{}: {}",
                        text.config_editor_token_threshold(),
                        self.summary_token_threshold
                    ),
                    Style::default().fg(theme::TEXT_MUTED),
                ))]));
            }

            // Enable Summary By Message Count toggle
            let is_focused = self.focused_summary_item() == Some(SummaryFocus::ByMessageCount);
            if is_focused {
                items.push(ListItem::new(vec![Line::from(Span::styled(
                    format!(
                        "{}: {}",
                        text.config_editor_by_message_count(),
                        if self.enable_summary_by_message_count {
                            text.config_yes()
                        } else {
                            text.config_no()
                        }
                    ),
                    Style::default()
                        .bg(theme::ACCENT_BG)
                        .fg(theme::TEXT)
                        .add_modifier(Modifier::BOLD),
                ))]));
            } else {
                items.push(ListItem::new(vec![Line::from(Span::styled(
                    format!(
                        "{}: {}",
                        text.config_editor_by_message_count(),
                        if self.enable_summary_by_message_count {
                            text.config_yes()
                        } else {
                            text.config_no()
                        }
                    ),
                    if self.enable_summary_by_message_count {
                        Style::default().fg(theme::TEXT)
                    } else {
                        Style::default().fg(theme::TEXT_MUTED)
                    },
                ))]));
            }

            if self.enable_summary_by_message_count {
                let is_focused =
                    self.focused_summary_item() == Some(SummaryFocus::MessageCountThreshold);
                if is_focused {
                    if self.editing_field {
                        items.push(ListItem::new(vec![Self::editing_value_line(
                            text.config_editor_message_count_threshold(),
                            &self.summary_message_count_threshold,
                        )]));
                    } else {
                        items.push(ListItem::new(vec![Line::from(Span::styled(
                            format!(
                                "{}: {}",
                                text.config_editor_message_count_threshold(),
                                self.summary_message_count_threshold
                            ),
                            Style::default()
                                .bg(theme::ACCENT_BG)
                                .fg(theme::TEXT)
                                .add_modifier(Modifier::BOLD),
                        ))]));
                    }
                } else {
                    items.push(ListItem::new(vec![Line::from(Span::styled(
                        format!(
                            "{}: {}",
                            text.config_editor_message_count_threshold(),
                            self.summary_message_count_threshold
                        ),
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
    /// The thinking level picked in the editor; `None` when it was left as it was.
    pub(crate) thinking_level: Option<ThinkingLevel>,
}
