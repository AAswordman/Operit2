use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crossterm::event::{KeyCode, KeyEvent};

use operit_model::ModelConfigData::{
    ModelCapabilities, ModelContextSpec, ProviderProfile, ResolvedModelConfig,
};

use super::helpers::centered_rect;
use super::i18n::{TuiLanguage, TuiText};
use super::link_proxy_rs::TuiCore;
use super::theme;

pub(crate) mod model_editor;
pub(crate) mod provider_edit;

pub(crate) use model_editor::{EditorChanges, EditorState};
pub(crate) use provider_edit::FormState;

pub(crate) enum ConfigState {
    ProviderList,
    ProviderForm(FormState),
    ModelList {
        provider_index: usize,
        provider_id: String,
    },
    ModelEditor(EditorState),
}

impl ConfigState {
    fn title(&self, text: TuiText) -> &str {
        match self {
            ConfigState::ProviderList => text.model_config_title(),
            ConfigState::ProviderForm(_) => text.model_config_new_provider(),
            ConfigState::ModelList { .. } => text.model_config_models_title(),
            ConfigState::ModelEditor(_) => text.model_config_edit_model(),
        }
    }
}

pub(crate) struct ConfigUi {
    pub(crate) state: ConfigState,
    pub(crate) providers: Vec<ProviderProfile>,
    pub(crate) search: String,
    pub(crate) filtered_indices: Vec<usize>,
    pub(crate) selected_index: usize,
    pub(crate) editing_provider_id: Option<String>,
    pub(crate) error_message: Option<String>,
    pub(crate) status_message: Option<String>,
    pub(crate) chat_provider_id: String,
    pub(crate) chat_model_id: String,
    pub(crate) show_confirm_dialog: bool,
    pub(crate) confirm_title: String,
    pub(crate) confirm_message: String,
    pub(crate) confirm_action: ConfirmAction,
    pub(crate) show_add_model_popup: bool,
    pub(crate) add_model_custom_mode: bool,
    pub(crate) add_model_search: String,
    pub(crate) add_model_filtered: Vec<usize>,
    pub(crate) add_model_index: usize,
    pub(crate) available_models:
        Vec<operit_model::ModelConfigData::AvailableProviderModel>,
}

pub(crate) enum ConfirmAction {
    None,
    DeleteModel {
        provider_id: String,
        model_id: String,
    },
    DeleteProvider {
        provider_id: String,
    },
}

impl ConfigUi {
    pub(crate) fn new() -> Self {
        Self {
            state: ConfigState::ProviderList,
            providers: Vec::new(),
            search: String::new(),
            filtered_indices: Vec::new(),
            selected_index: 0,
            editing_provider_id: None,
            error_message: None,
            status_message: None,
            chat_provider_id: String::new(),
            chat_model_id: String::new(),
            show_confirm_dialog: false,
            confirm_title: String::new(),
            confirm_message: String::new(),
            confirm_action: ConfirmAction::None,
            show_add_model_popup: false,
            add_model_custom_mode: false,
            add_model_search: String::new(),
            add_model_filtered: Vec::new(),
            add_model_index: 0,
            available_models: Vec::new(),
        }
    }

    pub(crate) async fn refresh_providers(&mut self, core: &mut TuiCore) {
        match core
            .preferences_model_config_manager()
            .getProviderProfiles()
            .await
        {
            Ok(profiles) => {
                self.providers = profiles;
                self.update_filter();
            }
            Err(e) => {
                self.error_message = Some(e.to_string());
            }
        }
    }

    pub(crate) fn update_filter(&mut self) {
        let search = self.search.to_ascii_lowercase();
        let count = match &self.state {
            ConfigState::ModelList { provider_index, .. } => self
                .providers
                .get(*provider_index)
                .map(|p| p.models.len())
                .unwrap_or(0),
            _ => self.providers.len(),
        };
        if search.is_empty() {
            self.filtered_indices = (0..count).collect();
        } else {
            let names: Vec<&str> = match &self.state {
                ConfigState::ModelList { provider_index, .. } => self
                    .providers
                    .get(*provider_index)
                    .map(|p| p.models.iter().map(|m| m.id.as_str()).collect())
                    .unwrap_or_default(),
                _ => self.providers.iter().map(|p| p.name.as_str()).collect(),
            };
            self.filtered_indices = names
                .iter()
                .enumerate()
                .filter(|(_, n)| n.to_ascii_lowercase().contains(&search))
                .map(|(i, _)| i)
                .collect();
        }
        if self.selected_index >= self.filtered_indices.len() {
            self.selected_index = self.filtered_indices.len().saturating_sub(1);
        }
    }

    pub(crate) fn selected_provider(&self) -> Option<&ProviderProfile> {
        self.filtered_indices
            .get(self.selected_index)
            .and_then(|&i| self.providers.get(i))
    }

    pub(crate) fn current_model_provider(&self) -> Option<&ProviderProfile> {
        match &self.state {
            ConfigState::ModelList { provider_index, .. } => self.providers.get(*provider_index),
            _ => None,
        }
    }

    pub(crate) fn selected_model(
        &self,
    ) -> Option<&operit_model::ModelConfigData::ModelProfile> {
        match &self.state {
            ConfigState::ModelList { provider_index, .. } => {
                let provider = self.providers.get(*provider_index)?;
                self.filtered_indices
                    .get(self.selected_index)
                    .and_then(|&i| provider.models.get(i))
            }
            _ => None,
        }
    }

    fn update_add_model_filter(&mut self) {
        let search = self.add_model_search.to_ascii_lowercase();
        if search.is_empty() {
            self.add_model_filtered = (0..self.available_models.len()).collect();
        } else {
            self.add_model_filtered = self
                .available_models
                .iter()
                .enumerate()
                .filter(|(_, model)| model.modelId.to_ascii_lowercase().contains(&search))
                .map(|(index, _)| index)
                .collect();
        }
        self.add_model_index = 0;
    }

    pub(crate) fn render(&self, frame: &mut Frame, text: TuiText) {
        let popup = centered_rect(72, 70, frame.area());
        frame.render_widget(Clear, popup);

        let title = self.state.title(text);
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(inner);

        // Search bar
        let search_display = if self.search.is_empty() {
            Span::styled(
                text.model_chooser_search_hint(),
                Style::default().fg(theme::TEXT_SUBTLE),
            )
        } else {
            Span::styled(&self.search, Style::default())
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(theme::ACCENT)),
                search_display,
            ])),
            areas[0],
        );

        // Separator
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "─".repeat(areas[1].width as usize),
                Style::default().fg(theme::TEXT_SUBTLE),
            ))),
            areas[1],
        );

        // Items list
        match &self.state {
            ConfigState::ProviderList => self.render_provider_list(areas[2], frame, text),
            ConfigState::ModelList { .. } => self.render_model_list(areas[2], frame, text),
            ConfigState::ProviderForm(form) => {
                form.render(areas[2], frame);
            }
            ConfigState::ModelEditor(editor) => {
                editor.render(areas[2], frame, text);
            }
        }

        // Footer help / status
        let footer = if let Some(msg) = &self.status_message {
            ratatui::text::Text::from(Span::styled(
                msg.clone(),
                Style::default().fg(theme::TEXT_SUBTLE),
            ))
        } else {
            self.footer_text(text)
        };
        frame.render_widget(
            Paragraph::new(footer).wrap(ratatui::widgets::Wrap { trim: false }),
            areas[3],
        );

        // Overlays: confirm dialog, add model popup
        if self.show_confirm_dialog {
            self.render_confirm_dialog(frame, popup, text);
        }
        if self.show_add_model_popup {
            self.render_add_model_popup(frame, popup, text);
        }
    }

    fn render_provider_list(&self, area: Rect, frame: &mut Frame, _text: TuiText) {
        let items: Vec<ListItem> = self
            .filtered_indices
            .iter()
            .map(|&i| {
                let p = &self.providers[i];
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(&p.name, Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw("  "),
                        Span::styled(&p.providerTypeId, Style::default().fg(theme::ACCENT)),
                    ]),
                    Line::from(vec![
                        Span::styled(&p.endpoint, Style::default().fg(theme::TEXT_SUBTLE)),
                        Span::raw("  "),
                        Span::styled(
                            format!("{} models", p.models.len()),
                            Style::default().fg(theme::TEXT_SUBTLE),
                        ),
                    ]),
                ])
            })
            .collect();

        let mut state = ListState::default();
        if !self.filtered_indices.is_empty() {
            state.select(Some(
                self.selected_index
                    .min(self.filtered_indices.len().saturating_sub(1)),
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
            area,
            &mut state,
        );
    }

    fn render_model_list(&self, area: Rect, frame: &mut Frame, _text: TuiText) {
        let provider = self.current_model_provider();
        let items: Vec<ListItem> = self
            .filtered_indices
            .iter()
            .map(|&i| {
                let model = provider.and_then(|p| p.models.get(i));
                let Some(model) = model else {
                    return ListItem::new(Line::from(Span::styled("?", Style::default())));
                };

                let is_active = self.chat_provider_id
                    == self.editing_provider_id.as_deref().unwrap_or("")
                    && self.chat_model_id == model.id;

                let marker = if is_active { "★ " } else { "  " };
                let mut line1 = vec![
                    Span::styled(marker, Style::default().fg(theme::ACCENT_STRONG)),
                    Span::styled(&model.id, Style::default().add_modifier(Modifier::BOLD)),
                ];

                // Show capabilities summary from the model profile if available
                if let Some(caps) = &model.capabilitiesOverride {
                    let mut tags = Vec::new();
                    if caps.toolCall {
                        tags.push("⚒");
                    }
                    if caps.directImage {
                        tags.push("🖼");
                    }
                    if caps.directAudio {
                        tags.push("🎤");
                    }
                    if caps.directVideo {
                        tags.push("🎬");
                    }
                    if !tags.is_empty() {
                        line1.push(Span::raw("  "));
                        line1.push(Span::styled(
                            tags.join(" "),
                            Style::default().fg(theme::ACCENT),
                        ));
                    }
                }

                // Context length from override
                let ctx_info = model
                    .contextOverride
                    .as_ref()
                    .map(|c| format!("{}k", c.maxContextLength));
                if let Some(ctx) = ctx_info {
                    line1.push(Span::raw("  "));
                    line1.push(Span::styled(ctx, Style::default().fg(theme::TEXT_SUBTLE)));
                }

                ListItem::new(Line::from(line1))
            })
            .collect();

        let mut state = ListState::default();
        if !self.filtered_indices.is_empty() {
            state.select(Some(
                self.selected_index
                    .min(self.filtered_indices.len().saturating_sub(1)),
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
            area,
            &mut state,
        );
    }

    fn footer_text(&self, text: TuiText) -> ratatui::text::Text<'static> {
        // Show confirm dialog / add model popup footers if those overlays are active
        if self.show_confirm_dialog {
            return ratatui::text::Text::from(Line::from(Span::styled(
                "Enter: confirm  |  Esc: cancel",
                Style::default().fg(theme::TEXT_SUBTLE),
            )));
        }
        if self.show_add_model_popup {
            return ratatui::text::Text::from(Line::from(Span::styled(
                "Up/Down: select  |  Enter: add model  |  Type to filter  |  Esc: cancel",
                Style::default().fg(theme::TEXT_SUBTLE),
            )));
        }

        let lines: Vec<Line<'static>> = match &self.state {
            ConfigState::ProviderList => {
                let count = self.filtered_indices.len();
                let total = self.providers.len();
                vec![Line::from(Span::styled(
                    text.config_provider_list_footer(count, total),
                    Style::default().fg(theme::TEXT_SUBTLE),
                ))]
            }
            ConfigState::ModelList { .. } => {
                let count = self.filtered_indices.len();
                let total = self
                    .current_model_provider()
                    .map(|p| p.models.len())
                    .unwrap_or(0);
                vec![Line::from(Span::styled(
                    text.config_model_list_footer(count, total),
                    Style::default().fg(theme::TEXT_SUBTLE),
                ))]
            }
            ConfigState::ProviderForm(form) => {
                if form.show_type_selector {
                    vec![Line::from(Span::styled(
                        text.config_type_selector_footer(),
                        Style::default().fg(theme::TEXT_SUBTLE),
                    ))]
                } else if form.editing_field {
                    vec![Line::from(Span::styled(
                        text.config_form_editing_footer(),
                        Style::default().fg(theme::TEXT_SUBTLE),
                    ))]
                } else if form.advanced {
                    vec![
                        Line::from(Span::styled(
                            text.config_form_nav_footer(),
                            Style::default().fg(theme::TEXT_SUBTLE),
                        )),
                        Line::from(Span::styled(
                            text.config_form_advanced_footer(),
                            Style::default().fg(theme::TEXT_SUBTLE),
                        )),
                    ]
                } else {
                    vec![
                        Line::from(Span::styled(
                            text.config_form_nav_footer(),
                            Style::default().fg(theme::TEXT_SUBTLE),
                        )),
                        Line::from(Span::styled(
                            text.config_form_simple_footer(),
                            Style::default().fg(theme::TEXT_SUBTLE),
                        )),
                    ]
                }
            }
            ConfigState::ModelEditor(editor) => match editor.level {
                model_editor::EditorLevel::Main => {
                    vec![Line::from(Span::styled(
                        text.config_editor_main_footer(),
                        Style::default().fg(theme::TEXT_SUBTLE),
                    ))]
                }
                model_editor::EditorLevel::Summary => {
                    vec![Line::from(Span::styled(
                        text.config_editor_summary_footer(),
                        Style::default().fg(theme::TEXT_SUBTLE),
                    ))]
                }
            },
        };
        ratatui::text::Text::from(lines)
    }

    fn render_confirm_dialog(&self, frame: &mut Frame, parent: Rect, text: TuiText) {
        let w = 40.min(parent.width);
        let h = 6.min(parent.height);
        let x = parent.x + (parent.width.saturating_sub(w)) / 2;
        let y = parent.y + (parent.height.saturating_sub(h)) / 2;
        let popup = Rect {
            x,
            y,
            width: w,
            height: h,
        };
        frame.render_widget(Clear, popup);
        let block = Block::default()
            .title(self.confirm_title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT_STRONG));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        frame.render_widget(
            Paragraph::new(ratatui::text::Text::from(Line::from(Span::styled(
                &self.confirm_message,
                Style::default(),
            ))))
            .wrap(ratatui::widgets::Wrap { trim: false }),
            inner,
        );
    }

    fn render_add_model_popup(&self, frame: &mut Frame, parent: Rect, text: TuiText) {
        let w = 50.min(parent.width);
        let h = 20.min(parent.height);
        let x = parent.x + (parent.width.saturating_sub(w)) / 2;
        let y = parent.y + (parent.height.saturating_sub(h)) / 2;
        let popup = Rect {
            x,
            y,
            width: w,
            height: h,
        };
        frame.render_widget(Clear, popup);
        let block = Block::default()
            .title(text.config_add_model_title())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(inner);

        if self.add_model_custom_mode {
            let value = if self.add_model_search.is_empty() {
                vec![Span::styled(
                    text.config_custom_model_id_hint(),
                    Style::default().fg(theme::TEXT_SUBTLE),
                )]
            } else {
                vec![
                    Span::styled(
                        self.add_model_search.clone(),
                        Style::default().bg(theme::ACCENT_BG).fg(theme::TEXT),
                    ),
                    Span::styled(
                        " ",
                        Style::default()
                            .bg(theme::SELECTION_BG)
                            .fg(theme::SELECTION_TEXT),
                    ),
                ]
            };
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled(
                        text.config_custom_model_title(),
                        Style::default()
                            .fg(theme::ACCENT_STRONG)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(value),
                ]),
                areas[1],
            );
            return;
        }

        // Search
        let search_display = if self.add_model_search.is_empty() {
            Span::styled(
                text.config_filter_hint(),
                Style::default().fg(theme::TEXT_SUBTLE),
            )
        } else {
            Span::styled(&self.add_model_search, Style::default())
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(theme::ACCENT)),
                search_display,
            ])),
            areas[0],
        );

        // Model list
        let items: Vec<ListItem> = self
            .add_model_filtered
            .iter()
            .map(|&i| {
                let model = self.available_models.get(i);
                let name = model.map(|m| m.modelId.as_str()).unwrap_or("?");
                ListItem::new(Line::from(Span::styled(name, Style::default())))
            })
            .collect();

        let mut items = items;
        items.push(ListItem::new(vec![
            Line::from(Span::styled(
                text.config_custom_model_title(),
                Style::default()
                    .fg(theme::ACCENT_STRONG)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                text.config_custom_model_id_hint(),
                Style::default().fg(theme::TEXT_SUBTLE),
            )),
        ]));

        let mut state = ListState::default();
        state.select(Some(
            self.add_model_index.min(items.len().saturating_sub(1)),
        ));
        frame.render_stateful_widget(
            List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(theme::ACCENT_BG)
                        .fg(theme::TEXT)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> "),
            areas[1],
            &mut state,
        );
    }

    pub(crate) async fn handle_key(
        &mut self,
        key: KeyEvent,
        core: &mut TuiCore,
        text: TuiText,
    ) -> Result<bool, String> {
        use crossterm::event::KeyCode as KC;

        match (&mut self.state, key.code) {
            // Provider list
            (ConfigState::ProviderList, KC::Esc) => {
                return Ok(true);
            }
            (ConfigState::ProviderList, KC::Up) => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            (ConfigState::ProviderList, KC::Down) => {
                if self.selected_index + 1 < self.filtered_indices.len() {
                    self.selected_index += 1;
                }
            }
            (ConfigState::ProviderList, KC::Char('n')) => {
                self.state = ConfigState::ProviderForm(provider_edit::FormState::new_create());
            }
            (ConfigState::ProviderList, KC::Enter) => {
                if let Some(provider) = self.selected_provider().cloned() {
                    let idx = self.filtered_indices[self.selected_index];
                    self.state = ConfigState::ModelList {
                        provider_index: idx,
                        provider_id: provider.id.clone(),
                    };
                    self.editing_provider_id = Some(provider.id.clone());
                    self.search.clear();
                    self.selected_index = 0;
                    self.update_filter();
                }
            }
            (ConfigState::ProviderList, KC::Char('e')) => {
                if let Some(provider) = self.selected_provider().cloned() {
                    self.state =
                        ConfigState::ProviderForm(provider_edit::FormState::new_edit(&provider));
                }
            }
            (ConfigState::ProviderList, KC::Char('d')) => {
                if let Some(provider) = self.selected_provider().cloned() {
                    self.show_confirm_dialog = true;
                    self.confirm_title = text.config_delete_provider().to_string();
                    self.confirm_message = text.config_delete_provider_msg(&provider.name);
                    self.confirm_action = ConfirmAction::DeleteProvider {
                        provider_id: provider.id.clone(),
                    };
                }
            }
            (ConfigState::ProviderList, KC::Char(c)) => {
                self.search.push(c);
                self.status_message = None;
                self.update_filter();
            }
            (ConfigState::ProviderList, KC::Backspace) => {
                self.search.pop();
                self.status_message = None;
                self.update_filter();
            }

            // Model list
            (ConfigState::ModelList { .. }, KC::Esc) => {
                self.state = ConfigState::ProviderList;
                self.search.clear();
                self.selected_index = 0;
                self.update_filter();
            }
            (ConfigState::ModelList { .. }, KC::Up) => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            (ConfigState::ModelList { .. }, KC::Down) => {
                if self.selected_index + 1 < self.filtered_indices.len() {
                    self.selected_index += 1;
                }
            }
            (ConfigState::ModelList { .. }, KC::Enter) => {
                if let Some(model) = self.selected_model().cloned() {
                    let pid = match &self.state {
                        ConfigState::ModelList { provider_id, .. } => provider_id.clone(),
                        _ => return Ok(false),
                    };
                    let provider_name = self
                        .current_model_provider()
                        .map(|p| p.name.clone())
                        .unwrap_or_default();
                    if let Ok(config) = core
                        .preferences_model_config_manager()
                        .getResolvedModelConfig(&pid, &model.id)
                        .await
                    {
                        self.state = ConfigState::ModelEditor(model_editor::EditorState::new(
                            pid,
                            provider_name,
                            model.id.clone(),
                            &config,
                        ));
                    }
                }
            }
            (ConfigState::ModelList { .. }, KC::Char('s')) => {
                if let Some(model) = self.selected_model().cloned() {
                    let pid = match &self.state {
                        ConfigState::ModelList { provider_id, .. } => provider_id.clone(),
                        _ => return Ok(false),
                    };
                    let _ = core
                        .preferences_functional_config_manager()
                        .setModelForFunction(
                            model_editor::EditorState::CHAT_FUNCTION,
                            pid.clone(),
                            model.id.clone(),
                        )
                        .await;
                    self.chat_provider_id = pid;
                    self.chat_model_id = model.id;
                    self.status_message = Some(text.config_model_activated().to_string());
                }
            }
            (ConfigState::ModelList { .. }, KC::Char('t')) => {
                if let Some(model) = self.selected_model().cloned() {
                    let pid = match &self.state {
                        ConfigState::ModelList { provider_id, .. } => provider_id.clone(),
                        _ => return Ok(false),
                    };
                    self.status_message = Some(text.config_testing_connection().to_string());
                    match core
                        .preferences_model_config_manager()
                        .testModelConnection(&pid, &model.id)
                        .await
                    {
                        Ok(report) => {
                            let success_count = report.items.iter().filter(|i| i.success).count();
                            let total = report.items.len();
                            self.status_message =
                                Some(text.config_test_result(success_count, total));
                        }
                        Err(e) => {
                            self.status_message = Some(text.config_test_failed(e.to_string()));
                        }
                    }
                }
            }
            (ConfigState::ModelList { .. }, KC::Char('a')) => {
                let pid = match &self.state {
                    ConfigState::ModelList { provider_id, .. } => provider_id.clone(),
                    _ => return Ok(false),
                };
                let existing_model_ids = self
                    .current_model_provider()
                    .map(|provider| {
                        provider
                            .models
                            .iter()
                            .map(|model| model.id.to_ascii_lowercase())
                            .collect::<std::collections::HashSet<_>>()
                    })
                    .unwrap_or_default();
                match core
                    .preferences_model_config_manager()
                    .getAvailableProviderModels(&pid)
                    .await
                {
                    Ok(available) => {
                        self.available_models = available
                            .into_iter()
                            .filter(|model| {
                                !existing_model_ids.contains(&model.modelId.to_ascii_lowercase())
                            })
                            .collect();
                        self.show_add_model_popup = true;
                        self.add_model_custom_mode = false;
                        self.add_model_search.clear();
                        self.add_model_index = 0;
                        self.add_model_filtered = (0..self.available_models.len()).collect();
                    }
                    Err(error) => {
                        self.error_message = Some(error.to_string());
                    }
                }
            }
            (ConfigState::ModelList { .. }, KC::Char('d')) => {
                if let Some(model) = self.selected_model().cloned() {
                    let pid = match &self.state {
                        ConfigState::ModelList { provider_id, .. } => provider_id.clone(),
                        _ => return Ok(false),
                    };
                    self.show_confirm_dialog = true;
                    self.confirm_title = text.config_delete_model().to_string();
                    self.confirm_message = text.config_delete_model_msg(&model.id);
                    self.confirm_action = ConfirmAction::DeleteModel {
                        provider_id: pid,
                        model_id: model.id.clone(),
                    };
                }
            }
            (ConfigState::ModelList { .. }, KC::Char(c)) => {
                self.search.push(c);
                self.update_filter();
            }
            (ConfigState::ModelList { .. }, KC::Backspace) => {
                self.search.pop();
                self.update_filter();
            }

            // Provider form
            (ConfigState::ProviderForm(form), KC::Esc) => {
                if form.show_type_selector {
                    form.show_type_selector = false;
                } else if form.editing_field {
                    form.editing_field = false;
                } else {
                    self.state = ConfigState::ProviderList;
                    self.status_message = None;
                }
            }
            (ConfigState::ProviderForm(form), KC::Up) => {
                if form.show_type_selector {
                    if form.type_selector_index > 0 {
                        form.type_selector_index -= 1;
                    }
                } else if !form.editing_field && form.focus_index > 0 {
                    form.focus_index -= 1;
                }
            }
            (ConfigState::ProviderForm(form), KC::Down) => {
                if form.show_type_selector {
                    if form.type_selector_index + 1 < form.type_selector_filtered.len() {
                        form.type_selector_index += 1;
                    }
                } else if !form.editing_field {
                    let visible = if form.advanced { form.fields.len() } else { 4 };
                    if form.focus_index + 1 < visible {
                        form.focus_index += 1;
                    }
                }
            }
            (ConfigState::ProviderForm(form), KC::Enter) => {
                if form.show_type_selector {
                    form.confirm_type_selection();
                } else if form.focus_index == 1 {
                    form.open_type_selector();
                } else if form.editing_field {
                    form.editing_field = false;
                    let msg = Self::save_provider_form(core, form, text).await;
                    if let Some(m) = msg {
                        self.status_message = Some(m);
                    }
                    self.refresh_providers(core).await;
                } else {
                    form.editing_field = true;
                }
            }
            (ConfigState::ProviderForm(form), KC::Char(c))
                if key.modifiers.is_empty()
                    || key.modifiers == crossterm::event::KeyModifiers::SHIFT =>
            {
                if form.show_type_selector {
                    form.type_selector_filter.push(c);
                    form.update_type_filter();
                } else if form.editing_field {
                    if let Some(field) = form.fields.get_mut(form.focus_index) {
                        field.value.insert(field.cursor, c);
                        field.cursor += 1;
                    }
                }
            }
            (ConfigState::ProviderForm(form), KC::Backspace) => {
                if form.show_type_selector {
                    form.type_selector_filter.pop();
                    form.update_type_filter();
                } else if form.editing_field {
                    if let Some(field) = form.fields.get_mut(form.focus_index) {
                        if field.cursor > 0 {
                            field.value.remove(field.cursor - 1);
                            field.cursor -= 1;
                        }
                    }
                }
            }
            (ConfigState::ProviderForm(form), KC::Delete) => {
                if form.editing_field {
                    if let Some(field) = form.fields.get_mut(form.focus_index) {
                        if field.cursor < field.value.len() {
                            field.value.remove(field.cursor);
                        }
                    }
                }
            }
            (ConfigState::ProviderForm(form), KC::Left) => {
                if form.editing_field {
                    if let Some(field) = form.fields.get_mut(form.focus_index) {
                        if field.cursor > 0 {
                            field.cursor -= 1;
                        }
                    }
                }
            }
            (ConfigState::ProviderForm(form), KC::Right) => {
                if form.editing_field {
                    if let Some(field) = form.fields.get_mut(form.focus_index) {
                        if field.cursor < field.value.len() {
                            field.cursor += 1;
                        }
                    }
                }
            }
            (ConfigState::ProviderForm(form), KC::Home) => {
                if form.editing_field {
                    if let Some(field) = form.fields.get_mut(form.focus_index) {
                        field.cursor = 0;
                    }
                }
            }
            (ConfigState::ProviderForm(form), KC::End) => {
                if form.editing_field {
                    if let Some(field) = form.fields.get_mut(form.focus_index) {
                        field.cursor = field.value.len();
                    }
                }
            }

            // Model editor — Main level
            (ConfigState::ModelEditor(editor), KC::Esc) => {
                if matches!(editor.level, model_editor::EditorLevel::Main) {
                    let changes = editor.into_config_changes();
                    let pid = editor.provider_id.clone();
                    let mid = editor.model_id.clone();
                    let _ = core
                        .preferences_model_config_manager()
                        .updateCapabilitiesForModel(&pid, &mid, changes.capabilities)
                        .await;
                    let _ = core
                        .preferences_model_config_manager()
                        .updateContextForModel(&pid, &mid, changes.context)
                        .await;
                    let _ = core
                        .preferences_model_config_manager()
                        .updateRequestForModel(&pid, &mid, changes.request)
                        .await;
                    let _ = core
                        .preferences_model_config_manager()
                        .updateSummaryForModel(&pid, &mid, changes.summary)
                        .await;
                    if !changes.builtin_tools.is_empty() {
                        let _ = core
                            .preferences_model_config_manager()
                            .updateBuiltinToolsForModel(&pid, &mid, changes.builtin_tools)
                            .await;
                    }
                    self.refresh_providers(core).await;
                    self.state = ConfigState::ModelList {
                        provider_index: self
                            .providers
                            .iter()
                            .position(|provider| provider.id == pid)
                            .expect("model editor provider must exist after refresh"),
                        provider_id: pid,
                    };
                    self.search.clear();
                    self.selected_index = 0;
                    self.update_filter();
                } else {
                    editor.level = model_editor::EditorLevel::Main;
                    editor.focus_index = 0;
                }
                self.status_message = None;
            }
            (ConfigState::ModelEditor(editor), KC::Up) => {
                editor.clamp_focus();
                if editor.focus_index > 0 {
                    editor.focus_index -= 1;
                }
            }
            (ConfigState::ModelEditor(editor), KC::Down) => {
                editor.clamp_focus();
                editor.focus_index += 1;
                editor.clamp_focus();
            }
            (ConfigState::ModelEditor(editor), KC::Enter) => {
                editor.clamp_focus();
                match editor.level {
                    model_editor::EditorLevel::Main => match editor.focused_main_item() {
                        Some(model_editor::MainFocus::ToolCall) => {
                            editor.tool_call = !editor.tool_call
                        }
                        Some(model_editor::MainFocus::DirectImage) => {
                            editor.direct_image = !editor.direct_image
                        }
                        Some(model_editor::MainFocus::DirectAudio) => {
                            editor.direct_audio = !editor.direct_audio
                        }
                        Some(model_editor::MainFocus::DirectVideo) => {
                            editor.direct_video = !editor.direct_video
                        }
                        Some(model_editor::MainFocus::MaxContextLength) => {
                            editor.editing_field = !editor.editing_field
                        }
                        Some(model_editor::MainFocus::MaxContextMode) => {
                            editor.enable_max_context_mode = !editor.enable_max_context_mode
                        }
                        Some(model_editor::MainFocus::EnableSummary) => {
                            editor.enable_summary = !editor.enable_summary;
                            editor.clamp_focus();
                        }
                        Some(model_editor::MainFocus::SummaryDetails) => {
                            editor.level = model_editor::EditorLevel::Summary;
                            editor.focus_index = 0;
                        }
                        Some(model_editor::MainFocus::StructuredTools) => {
                            editor.supports_structured_tools = !editor.supports_structured_tools
                        }
                        Some(model_editor::MainFocus::BuiltinTool(index)) => {
                            if let Some(tool) = editor.builtin_tools.get_mut(index) {
                                tool.enabled = !tool.enabled;
                            }
                        }
                        None => {}
                    },
                    model_editor::EditorLevel::Summary => match editor.focused_summary_item() {
                        Some(model_editor::SummaryFocus::EnableSummary) => {
                            editor.enable_summary = !editor.enable_summary;
                            editor.clamp_focus();
                        }
                        Some(model_editor::SummaryFocus::TokenThreshold) => {
                            editor.editing_field = !editor.editing_field
                        }
                        Some(model_editor::SummaryFocus::ByMessageCount) => {
                            editor.enable_summary_by_message_count =
                                !editor.enable_summary_by_message_count;
                            editor.clamp_focus();
                        }
                        Some(model_editor::SummaryFocus::MessageCountThreshold) => {
                            editor.editing_field = !editor.editing_field
                        }
                        None => {}
                    },
                }
            }
            (ConfigState::ModelEditor(editor), KC::Char(c))
                if key.modifiers.is_empty() && editor.editing_field =>
            {
                match editor.level {
                    model_editor::EditorLevel::Main
                        if editor.focused_main_item()
                            == Some(model_editor::MainFocus::MaxContextLength) =>
                    {
                        if c.is_ascii_digit() || c == '.' {
                            editor.max_context_length.push(c);
                        }
                    }
                    model_editor::EditorLevel::Summary => {
                        if c.is_ascii_digit() || c == '.' {
                            match editor.focused_summary_item() {
                                Some(model_editor::SummaryFocus::TokenThreshold) => {
                                    editor.summary_token_threshold.push(c)
                                }
                                Some(model_editor::SummaryFocus::MessageCountThreshold) => {
                                    editor.summary_message_count_threshold.push(c)
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            (ConfigState::ModelEditor(editor), KC::Backspace) => {
                if editor.editing_field {
                    match editor.level {
                        model_editor::EditorLevel::Main
                            if editor.focused_main_item()
                                == Some(model_editor::MainFocus::MaxContextLength) =>
                        {
                            editor.max_context_length.pop();
                        }
                        model_editor::EditorLevel::Summary => match editor.focused_summary_item() {
                            Some(model_editor::SummaryFocus::TokenThreshold) => {
                                editor.summary_token_threshold.pop();
                            }
                            Some(model_editor::SummaryFocus::MessageCountThreshold) => {
                                editor.summary_message_count_threshold.pop();
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
            // Model editor actions
            (ConfigState::ModelEditor(editor), KC::Char('s')) => {
                if matches!(editor.level, model_editor::EditorLevel::Main) {
                    let _ = core
                        .preferences_functional_config_manager()
                        .setModelForFunction(
                            model_editor::EditorState::CHAT_FUNCTION,
                            editor.provider_id.clone(),
                            editor.model_id.clone(),
                        )
                        .await;
                    self.chat_provider_id = editor.provider_id.clone();
                    self.chat_model_id = editor.model_id.clone();
                    self.status_message = Some(text.config_model_activated().to_string());
                }
            }
            (ConfigState::ModelEditor(editor), KC::Char('t')) => {
                if matches!(editor.level, model_editor::EditorLevel::Main) {
                    editor.testing = true;
                    match core
                        .preferences_model_config_manager()
                        .testModelConnection(&editor.provider_id, &editor.model_id)
                        .await
                    {
                        Ok(report) => {
                            let success_count = report.items.iter().filter(|i| i.success).count();
                            let total = report.items.len();
                            editor.test_result =
                                Some(text.config_test_result(success_count, total));
                        }
                        Err(e) => {
                            self.status_message = Some(text.config_test_failed(e.to_string()));
                        }
                    }
                    editor.testing = false;
                }
            }
            (ConfigState::ModelEditor(editor), KC::Char('d')) => {
                if matches!(editor.level, model_editor::EditorLevel::Main) {
                    self.show_confirm_dialog = true;
                    self.confirm_title = text.config_delete_model().to_string();
                    self.confirm_message = text.config_delete_model_msg(&editor.model_id);
                    self.confirm_action = ConfirmAction::DeleteModel {
                        provider_id: editor.provider_id.clone(),
                        model_id: editor.model_id.clone(),
                    };
                }
            }

            // Confirm dialog
            _ if self.show_confirm_dialog => match key.code {
                KC::Enter => {
                    let action = std::mem::replace(&mut self.confirm_action, ConfirmAction::None);
                    self.show_confirm_dialog = false;
                    match action {
                        ConfirmAction::DeleteModel {
                            provider_id,
                            model_id,
                        } => {
                            let _ = core
                                .preferences_model_config_manager()
                                .deleteModel(&provider_id, &model_id)
                                .await;
                            self.refresh_providers(core).await;
                            self.state = ConfigState::ProviderList;
                        }
                        ConfirmAction::DeleteProvider { provider_id } => {
                            let _ = core
                                .preferences_model_config_manager()
                                .deleteProvider(&provider_id)
                                .await;
                            self.refresh_providers(core).await;
                        }
                        ConfirmAction::None => {}
                    }
                }
                KC::Esc => {
                    self.show_confirm_dialog = false;
                    self.confirm_action = ConfirmAction::None;
                }
                _ => {}
            },

            // Add model popup
            _ if self.show_add_model_popup => {
                match key.code {
                    KC::Up => {
                        if self.add_model_custom_mode {
                            // Cursor stays at the end in the single-line custom id input.
                        } else if self.add_model_index > 0 {
                            self.add_model_index -= 1;
                        }
                    }
                    KC::Down => {
                        if self.add_model_custom_mode {
                            // Cursor stays at the end in the single-line custom id input.
                        } else if self.add_model_index < self.add_model_filtered.len() {
                            self.add_model_index += 1;
                        }
                    }
                    KC::Enter => {
                        let pid = match &self.state {
                            ConfigState::ModelList { provider_id, .. } => provider_id.clone(),
                            _ => String::new(),
                        };
                        if self.add_model_custom_mode {
                            let model_id = self.add_model_search.trim().to_string();
                            if !model_id.is_empty() {
                                match core
                                    .preferences_model_config_manager()
                                    .createProviderModel(&pid, model_id)
                                    .await
                                {
                                    Ok(_) => {
                                        self.show_add_model_popup = false;
                                        self.add_model_custom_mode = false;
                                        self.refresh_providers(core).await;
                                    }
                                    Err(error) => {
                                        self.error_message = Some(error.to_string());
                                    }
                                }
                            }
                        } else if self.add_model_index == self.add_model_filtered.len() {
                            self.add_model_custom_mode = true;
                            self.add_model_search.clear();
                        } else if let Some(&i) = self.add_model_filtered.get(self.add_model_index) {
                            if let Some(model) = self.available_models.get(i) {
                                match core
                                    .preferences_model_config_manager()
                                    .addProviderModelFromAvailable(&pid, model.modelId.clone())
                                    .await
                                {
                                    Ok(_) => {
                                        self.show_add_model_popup = false;
                                        self.refresh_providers(core).await;
                                    }
                                    Err(error) => {
                                        self.error_message = Some(error.to_string());
                                    }
                                }
                            }
                        }
                    }
                    KC::Esc => {
                        if self.add_model_custom_mode {
                            self.add_model_custom_mode = false;
                            self.add_model_search.clear();
                            self.add_model_index = self.add_model_filtered.len();
                        } else {
                            self.show_add_model_popup = false;
                        }
                    }
                    KC::Char(c)
                        if key.modifiers.is_empty()
                            || key.modifiers == crossterm::event::KeyModifiers::SHIFT =>
                    {
                        self.add_model_search.push(c);
                        if !self.add_model_custom_mode {
                            self.update_add_model_filter();
                        }
                    }
                    KC::Backspace => {
                        self.add_model_search.pop();
                        if !self.add_model_custom_mode {
                            self.update_add_model_filter();
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(false)
    }

    async fn save_provider_form(
        core: &mut TuiCore,
        form: &provider_edit::FormState,
        text: super::i18n::TuiText,
    ) -> Option<String> {
        let name = form.fields[0].value.clone();
        let provider_type_id = form.fields[1].value.clone();
        let endpoint = form.fields[2].value.clone();
        let api_key = form.fields[3].value.clone();
        let custom_headers = form
            .fields
            .get(4)
            .map(|f| f.value.clone())
            .unwrap_or_else(|| "{}".to_string());
        let req_limit = form
            .fields
            .get(5)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(0);
        let max_concurrent = form
            .fields
            .get(6)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(0);

        match &form.editing_provider_id {
            None => {
                match core
                    .preferences_model_config_manager()
                    .createProvider(name.clone(), provider_type_id.clone(), endpoint.clone())
                    .await
                {
                    Ok(provider_id) => {
                        if !api_key.is_empty()
                            || !custom_headers.is_empty()
                            || req_limit > 0
                            || max_concurrent > 0
                        {
                            if let Ok(profile) = core
                                .preferences_model_config_manager()
                                .getProviderProfile(&provider_id)
                                .await
                            {
                                let _ = core
                                    .preferences_model_config_manager()
                                    .updateProviderProfile(
                                        operit_model::ModelConfigData::ProviderProfile {
                                            apiKey: api_key,
                                            customHeaders: custom_headers,
                                            requestLimitPerMinute: req_limit,
                                            maxConcurrentRequests: max_concurrent,
                                            ..profile
                                        },
                                    )
                                    .await;
                            }
                        }
                        Some(text.config_provider_created(&provider_id))
                    }
                    Err(e) => Some(text.config_error(&e.to_string())),
                }
            }
            Some(pid) => {
                if let Ok(profile) = core
                    .preferences_model_config_manager()
                    .getProviderProfile(pid)
                    .await
                {
                    let updated = operit_model::ModelConfigData::ProviderProfile {
                        name,
                        providerTypeId: provider_type_id,
                        endpoint,
                        apiKey: api_key,
                        customHeaders: custom_headers,
                        requestLimitPerMinute: req_limit,
                        maxConcurrentRequests: max_concurrent,
                        ..profile
                    };
                    match core
                        .preferences_model_config_manager()
                        .updateProviderProfile(updated)
                        .await
                    {
                        Ok(_) => Some(text.config_provider_updated().to_string()),
                        Err(e) => Some(text.config_error(&e.to_string())),
                    }
                } else {
                    None
                }
            }
        }
    }
}
