//! Writes a model's thinking level into a provider request, in the fields its API reads.
//!
//! A model accepts some of three reasoning controls: an on/off switch, graded effort levels
//! and a reasoning-token budget (models.dev says which). The API a provider speaks decides how
//! each control is spelt, and some APIs cannot spell some of them. A model offers the levels
//! its API can spell for it, and a request asks for one of those.

use serde_json::{json, Map, Value};

use operit_model::ModelConfigData::ApiProviderType;
use operit_model::ModelThinking::{ModelThinkingSpec, ThinkingLevel, ThinkingLevelOption};

use crate::chat::llmprovider::AIService::AiServiceError;
use crate::chat::llmprovider::DeepseekProvider::deepseek_uses_responses_protocol;
use crate::chat::llmprovider::OpenCodeProvider::OpenCodeRouting;

/// The thinking one provider request asks for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RequestThinking {
    /// The thinking controls the model accepts.
    pub spec: ModelThinkingSpec,
    /// The level to ask for; `None` leaves the request as the model's defaults have it.
    pub level: Option<ThinkingLevel>,
}

impl RequestThinking {
    /// Asks for the level set on a model, or for no thinking when the caller turns it off.
    pub fn for_model(spec: &ModelThinkingSpec, level: Option<ThinkingLevel>, enabled: bool) -> Self {
        Self {
            spec: spec.clone(),
            level: if enabled {
                level
            } else {
                Some(ThinkingLevel::Off)
            },
        }
    }

    /// Returns whether the model reasons on this request.
    pub fn is_on(&self) -> bool {
        match self.level {
            Some(level) => level.isOn(),
            None => self.spec.supported,
        }
    }
}

/// The request fields one API reads a thinking level from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThinkingDialect {
    /// Chat Completions `reasoning_effort`; no field switches reasoning.
    ChatEffort,
    /// Chat Completions `thinking.type` and `reasoning_effort` (DeepSeek, Zhipu, Moonshot,
    /// MiMo, Doubao, OpenCode's chat route).
    ChatThinkingType,
    /// DashScope `enable_thinking`, `thinking_budget` and `reasoning_effort`, a budget first.
    DashScope,
    /// SiliconFlow `enable_thinking`, `reasoning_effort` and `thinking_budget`, an effort first.
    SiliconFlow,
    /// NVIDIA `chat_template_kwargs.enable_thinking` and `reasoning_effort`.
    ChatTemplate,
    /// OpenAI Responses `reasoning.effort`, with reasoning summaries kept for later turns.
    Responses,
    /// DeepSeek's Responses API `reasoning.effort`.
    DeepSeekResponses,
    /// Anthropic Messages `thinking`: a budget where the model takes one, else adaptive
    /// thinking with `output_config.effort`.
    Anthropic,
    /// Gemini `generationConfig.thinkingConfig`.
    Gemini,
    /// OpenRouter `reasoning`.
    OpenRouter,
    /// The built-in local runtime's `enable_thinking`.
    LocalRuntime,
}

/// Which of a model's controls its levels come from.
enum LevelSource {
    Budget(operit_model::ModelThinking::ThinkingBudgetRange),
    Effort(Vec<ThinkingLevel>),
    Switch,
}

impl ThinkingDialect {
    /// Returns the API a configured provider of this type speaks to this model, `None` when
    /// it takes no thinking control.
    pub fn for_provider(provider_type_id: &str, endpoint: &str, model_id: &str) -> Option<Self> {
        let provider_type = ApiProviderType::fromProviderTypeId(provider_type_id)?;
        match provider_type {
            ApiProviderType::OPENAI_RESPONSES
            | ApiProviderType::OPENAI_RESPONSES_GENERIC
            | ApiProviderType::OPENAI_CODEX => Some(Self::Responses),
            ApiProviderType::ANTHROPIC | ApiProviderType::ANTHROPIC_GENERIC => {
                Some(Self::Anthropic)
            }
            ApiProviderType::GOOGLE | ApiProviderType::GEMINI_GENERIC => Some(Self::Gemini),
            ApiProviderType::OPENROUTER | ApiProviderType::NOUS_PORTAL => Some(Self::OpenRouter),
            ApiProviderType::LOCAL_MODEL => Some(Self::LocalRuntime),
            ApiProviderType::DEEPSEEK => {
                if deepseek_uses_responses_protocol(endpoint).ok()? {
                    Some(Self::DeepSeekResponses)
                } else {
                    Some(Self::ChatThinkingType)
                }
            }
            ApiProviderType::OPENCODE => {
                match OpenCodeRouting::protocol_for(endpoint, model_id).ok()? {
                    ApiProviderType::GEMINI_GENERIC => Some(Self::Gemini),
                    ApiProviderType::ANTHROPIC_GENERIC => Some(Self::Anthropic),
                    ApiProviderType::OPENAI_RESPONSES_GENERIC => Some(Self::Responses),
                    _ => Self::for_chat_provider(provider_type_id),
                }
            }
            _ => Self::for_chat_provider(provider_type_id),
        }
    }

    /// Returns the API a Chat Completions provider of this type speaks, `None` when it takes no
    /// thinking control or is not a Chat Completions provider.
    pub fn for_chat_provider(provider_type_id: &str) -> Option<Self> {
        match ApiProviderType::fromProviderTypeId(provider_type_id)? {
            ApiProviderType::OPENAI
            | ApiProviderType::OPENAI_GENERIC
            | ApiProviderType::XAI
            | ApiProviderType::OPENAI_LOCAL
            | ApiProviderType::LMSTUDIO
            | ApiProviderType::OLLAMA
            | ApiProviderType::OTHER
            | ApiProviderType::PPINFRA
            | ApiProviderType::NOVITA
            | ApiProviderType::FOUR_ROUTER
            | ApiProviderType::INFINIAI
            | ApiProviderType::MINIMAX => Some(Self::ChatEffort),
            ApiProviderType::DEEPSEEK
            | ApiProviderType::ZHIPU
            | ApiProviderType::MOONSHOT
            | ApiProviderType::MIMO
            | ApiProviderType::DOUBAO
            | ApiProviderType::OPENCODE => Some(Self::ChatThinkingType),
            ApiProviderType::ALIYUN => Some(Self::DashScope),
            ApiProviderType::SILICONFLOW => Some(Self::SiliconFlow),
            ApiProviderType::NVIDIA => Some(Self::ChatTemplate),
            _ => None,
        }
    }

    /// Returns the levels a model offers through this API, weakest first; empty when the API
    /// can change nothing about how the model thinks.
    pub fn levels(self, spec: &ModelThinkingSpec) -> Vec<ThinkingLevelOption> {
        if !spec.supported {
            return Vec::new();
        }
        let Some(source) = self.level_source(spec) else {
            return Vec::new();
        };
        let mut levels = Vec::new();
        if self.turns_off(spec, &source) {
            levels.push(ThinkingLevelOption {
                level: ThinkingLevel::Off,
                budgetTokens: None,
            });
        }
        match source {
            LevelSource::Budget(budget) => levels.extend(budget.levels()),
            LevelSource::Effort(efforts) => levels.extend(efforts.into_iter().map(|level| {
                ThinkingLevelOption {
                    level,
                    budgetTokens: None,
                }
            })),
            LevelSource::Switch => levels.push(ThinkingLevelOption {
                level: ThinkingLevel::Enabled,
                budgetTokens: None,
            }),
        }
        levels
    }

    /// Writes the level a request asks for. A request naming no level is left as built; one
    /// asking a model that cannot be switched off for no thinking is left as built too.
    pub fn apply(self, request: &mut Value, thinking: &RequestThinking) -> Result<(), AiServiceError> {
        let Some(level) = thinking.level else {
            return Ok(());
        };
        let levels = self.levels(&thinking.spec);
        let Some(option) = levels.iter().copied().find(|option| option.level == level) else {
            if level == ThinkingLevel::Off {
                return Ok(());
            }
            return Err(AiServiceError::RequestFailed(format!(
                "thinking level `{}` is not one this model takes here; pick another level",
                level.name()
            )));
        };
        if level == ThinkingLevel::Off {
            self.write_off(request, &thinking.spec)
        } else {
            self.write_on(request, &thinking.spec, option)
        }
    }

    /// Picks the control the model's levels come from: a budget or an effort, as this API
    /// prefers when the model takes both, else a bare switch.
    fn level_source(self, spec: &ModelThinkingSpec) -> Option<LevelSource> {
        let efforts = if self.spells_effort() {
            spec.efforts
                .iter()
                .copied()
                .filter(|level| level.isOn())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let budget = spec.budget.filter(|_| self.spells_budget());
        match budget {
            Some(budget) if self.prefers_budget() || efforts.is_empty() => {
                Some(LevelSource::Budget(budget))
            }
            _ if !efforts.is_empty() => Some(LevelSource::Effort(efforts)),
            _ if spec.toggle && self.spells_switch() => Some(LevelSource::Switch),
            _ => None,
        }
    }

    /// Returns whether a request can ask this model for no thinking.
    fn turns_off(self, spec: &ModelThinkingSpec, source: &LevelSource) -> bool {
        (spec.toggle && self.spells_switch())
            || (spec.efforts.contains(&ThinkingLevel::Off) && self.spells_effort())
            // Anthropic models that take a budget only think when a request asks them to.
            || (self == Self::Anthropic && matches!(source, LevelSource::Budget(_)))
    }

    /// Returns whether a field of this API turns reasoning on and off.
    fn spells_switch(self) -> bool {
        self != Self::ChatEffort
    }

    /// Returns whether this API takes an effort level.
    fn spells_effort(self) -> bool {
        self != Self::LocalRuntime
    }

    /// Returns whether this API takes a reasoning-token budget.
    fn spells_budget(self) -> bool {
        matches!(
            self,
            Self::Anthropic | Self::Gemini | Self::DashScope | Self::SiliconFlow | Self::OpenRouter
        )
    }

    /// Returns whether this API reads a budget more reliably than an effort.
    fn prefers_budget(self) -> bool {
        matches!(self, Self::Anthropic | Self::DashScope)
    }

    /// Asks the model for no thinking.
    fn write_off(self, request: &mut Value, spec: &ModelThinkingSpec) -> Result<(), AiServiceError> {
        let switch = spec.toggle && self.spells_switch();
        match self {
            Self::ChatEffort => put_json_path(request, "reasoning_effort", json!("none")),
            Self::ChatThinkingType if switch => {
                put_json_path(request, "thinking.type", json!("disabled"))
            }
            Self::DashScope | Self::SiliconFlow if switch => {
                put_json_path(request, "enable_thinking", json!(false))
            }
            Self::ChatTemplate if switch => {
                put_json_path(request, "chat_template_kwargs.enable_thinking", json!(false))
            }
            Self::ChatThinkingType | Self::DashScope | Self::SiliconFlow | Self::ChatTemplate => {
                put_json_path(request, "reasoning_effort", json!("none"))
            }
            Self::Responses | Self::DeepSeekResponses => {
                put_json_path(request, "reasoning.effort", json!("none"))
            }
            Self::Anthropic => {
                if switch || spec.efforts.contains(&ThinkingLevel::Off) {
                    put_json_path(request, "thinking", json!({ "type": "disabled" }))
                } else {
                    Ok(())
                }
            }
            Self::Gemini => {
                put_json_path(request, "generationConfig.thinkingConfig.thinkingBudget", json!(0))?;
                put_json_path(
                    request,
                    "generationConfig.thinkingConfig.includeThoughts",
                    json!(false),
                )
            }
            Self::OpenRouter => {
                if spec.efforts.contains(&ThinkingLevel::Off) {
                    put_json_path(request, "reasoning.effort", json!("none"))
                } else {
                    put_json_path(request, "reasoning.enabled", json!(false))
                }
            }
            Self::LocalRuntime => put_json_path(request, "enable_thinking", json!(false)),
        }
    }

    /// Asks the model to think at one of its levels.
    fn write_on(
        self,
        request: &mut Value,
        spec: &ModelThinkingSpec,
        option: ThinkingLevelOption,
    ) -> Result<(), AiServiceError> {
        let budget = option.budgetTokens;
        let effort = option
            .level
            .effortValue()
            .filter(|_| budget.is_none());
        let switch = spec.toggle && self.spells_switch();
        match self {
            Self::ChatEffort => {
                if let Some(effort) = effort {
                    put_json_path(request, "reasoning_effort", json!(effort))?;
                }
                Ok(())
            }
            Self::ChatThinkingType => {
                if switch {
                    put_json_path(request, "thinking.type", json!("enabled"))?;
                }
                if let Some(effort) = effort {
                    put_json_path(request, "reasoning_effort", json!(effort))?;
                }
                Ok(())
            }
            Self::DashScope | Self::SiliconFlow => {
                if switch {
                    put_json_path(request, "enable_thinking", json!(true))?;
                }
                if let Some(budget) = budget {
                    put_json_path(request, "thinking_budget", json!(budget))?;
                }
                if let Some(effort) = effort {
                    put_json_path(request, "reasoning_effort", json!(effort))?;
                }
                Ok(())
            }
            Self::ChatTemplate => {
                if switch {
                    put_json_path(request, "chat_template_kwargs.enable_thinking", json!(true))?;
                }
                if let Some(effort) = effort {
                    put_json_path(request, "reasoning_effort", json!(effort))?;
                }
                Ok(())
            }
            Self::Responses => {
                if let Some(effort) = effort {
                    put_json_path(request, "reasoning.effort", json!(effort))?;
                }
                put_json_path(request, "reasoning.summary", json!("auto"))?;
                put_json_path(request, "include", json!(["reasoning.encrypted_content"]))
            }
            Self::DeepSeekResponses => {
                if let Some(effort) = effort {
                    put_json_path(request, "reasoning.effort", json!(effort))?;
                }
                Ok(())
            }
            Self::Anthropic => match (budget, effort) {
                (Some(budget), _) => put_json_path(
                    request,
                    "thinking",
                    json!({ "type": "enabled", "budget_tokens": budget }),
                ),
                (None, effort) => {
                    put_json_path(
                        request,
                        "thinking",
                        json!({ "type": "adaptive", "display": "summarized" }),
                    )?;
                    if let Some(effort) = effort {
                        put_json_path(request, "output_config.effort", json!(effort))?;
                    }
                    Ok(())
                }
            },
            Self::Gemini => {
                put_json_path(
                    request,
                    "generationConfig.thinkingConfig.includeThoughts",
                    json!(true),
                )?;
                if let Some(budget) = budget {
                    put_json_path(
                        request,
                        "generationConfig.thinkingConfig.thinkingBudget",
                        json!(budget),
                    )?;
                }
                if let Some(effort) = effort {
                    put_json_path(
                        request,
                        "generationConfig.thinkingConfig.thinkingLevel",
                        json!(effort.to_ascii_uppercase()),
                    )?;
                }
                Ok(())
            }
            Self::OpenRouter => match (effort, budget) {
                (Some(effort), _) => put_json_path(request, "reasoning.effort", json!(effort)),
                (None, Some(budget)) => put_json_path(request, "reasoning.max_tokens", json!(budget)),
                (None, None) => put_json_path(request, "reasoning.enabled", json!(true)),
            },
            Self::LocalRuntime => put_json_path(request, "enable_thinking", json!(true)),
        }
    }
}

/// Writes the thinking a request asks for through an API that may take no thinking control.
pub fn apply_thinking(
    dialect: Option<ThinkingDialect>,
    request: &mut Value,
    thinking: &RequestThinking,
) -> Result<(), AiServiceError> {
    match dialect {
        Some(dialect) => dialect.apply(request, thinking),
        None => Ok(()),
    }
}

/// Writes a value to a dot-separated JSON object path, creating the objects on the way.
fn put_json_path(root: &mut Value, path: &str, value: Value) -> Result<(), AiServiceError> {
    let segments = path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let (last, parents) = segments
        .split_last()
        .expect("thinking request paths are never empty");
    let mut object = root.as_object_mut().ok_or_else(|| {
        AiServiceError::RequestFailed("thinking request root must be an object".to_string())
    })?;
    for parent in parents {
        let entry = object
            .entry((*parent).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        object = entry.as_object_mut().ok_or_else(|| {
            AiServiceError::RequestFailed(format!(
                "thinking request path conflicts with request field: {parent}"
            ))
        })?;
    }
    object.insert((*last).to_string(), value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{RequestThinking, ThinkingDialect};
    use operit_model::ModelThinking::{ModelThinkingSpec, ThinkingBudgetRange, ThinkingLevel};
    use serde_json::{json, Value};
    use operit_model::ModelThinking::ThinkingLevel::*;

    fn spec(toggle: bool, efforts: &[ThinkingLevel], budget: Option<(Option<i64>, Option<i64>)>) -> ModelThinkingSpec {
        ModelThinkingSpec {
            supported: true,
            toggle,
            efforts: efforts.to_vec(),
            budget: budget.map(|(min, max)| ThinkingBudgetRange { min, max }),
        }
    }

    fn levels(dialect: ThinkingDialect, spec: &ModelThinkingSpec) -> Vec<ThinkingLevel> {
        dialect.levels(spec).into_iter().map(|option| option.level).collect()
    }

    fn written(dialect: ThinkingDialect, spec: &ModelThinkingSpec, level: ThinkingLevel) -> Value {
        let mut request = json!({});
        dialect
            .apply(
                &mut request,
                &RequestThinking {
                    spec: spec.clone(),
                    level: Some(level),
                },
            )
            .expect("level applies");
        request
    }

    #[test]
    fn deepseek_switches_and_takes_effort() {
        let v4 = spec(true, &[Low, High, Max], None);
        assert_eq!(levels(ThinkingDialect::ChatThinkingType, &v4), vec![Off, Low, High, Max]);
        assert_eq!(
            written(ThinkingDialect::ChatThinkingType, &v4, High),
            json!({"thinking": {"type": "enabled"}, "reasoning_effort": "high"})
        );
        assert_eq!(
            written(ThinkingDialect::ChatThinkingType, &v4, Off),
            json!({"thinking": {"type": "disabled"}})
        );
    }

    #[test]
    fn anthropic_budget_models_think_only_when_asked() {
        let sonnet45 = spec(false, &[], Some((Some(1024), None)));
        assert_eq!(
            levels(ThinkingDialect::Anthropic, &sonnet45),
            vec![Off, Low, Medium, High, Xhigh]
        );
        assert_eq!(written(ThinkingDialect::Anthropic, &sonnet45, Off), json!({}));
        assert_eq!(
            written(ThinkingDialect::Anthropic, &sonnet45, High),
            json!({"thinking": {"type": "enabled", "budget_tokens": 16384}})
        );
        // Opus 4.5 and 4.6 take both; the budget works on each.
        let opus46 = spec(false, &[Low, Medium, High, Max], Some((Some(1024), None)));
        assert_eq!(
            written(ThinkingDialect::Anthropic, &opus46, Low),
            json!({"thinking": {"type": "enabled", "budget_tokens": 2048}})
        );
    }

    #[test]
    fn anthropic_effort_models_use_adaptive_thinking() {
        let opus = spec(false, &[Low, Medium, High, Xhigh, Max], None);
        assert_eq!(levels(ThinkingDialect::Anthropic, &opus), vec![Low, Medium, High, Xhigh, Max]);
        assert_eq!(
            written(ThinkingDialect::Anthropic, &opus, Max),
            json!({
                "thinking": {"type": "adaptive", "display": "summarized"},
                "output_config": {"effort": "max"}
            })
        );
        let sonnet5 = spec(true, &[Low, Medium, High, Xhigh, Max], None);
        assert_eq!(
            written(ThinkingDialect::Anthropic, &sonnet5, Off),
            json!({"thinking": {"type": "disabled"}})
        );
    }

    #[test]
    fn gemini_levels_and_budgets() {
        let flash25 = spec(true, &[], Some((Some(0), Some(24_576))));
        assert_eq!(
            written(ThinkingDialect::Gemini, &flash25, Off),
            json!({"generationConfig": {"thinkingConfig": {"thinkingBudget": 0, "includeThoughts": false}}})
        );
        assert_eq!(
            written(ThinkingDialect::Gemini, &flash25, Medium),
            json!({"generationConfig": {"thinkingConfig": {"includeThoughts": true, "thinkingBudget": 8192}}})
        );
        let gemini3 = spec(false, &[Minimal, Low, Medium, High], None);
        assert_eq!(levels(ThinkingDialect::Gemini, &gemini3), vec![Minimal, Low, Medium, High]);
        assert_eq!(
            written(ThinkingDialect::Gemini, &gemini3, Low),
            json!({"generationConfig": {"thinkingConfig": {"includeThoughts": true, "thinkingLevel": "LOW"}}})
        );
    }

    #[test]
    fn responses_turn_off_with_effort_none() {
        let gpt = spec(false, &[Off, Low, Medium, High, Xhigh], None);
        assert_eq!(levels(ThinkingDialect::Responses, &gpt), vec![Off, Low, Medium, High, Xhigh]);
        assert_eq!(
            written(ThinkingDialect::Responses, &gpt, Off),
            json!({"reasoning": {"effort": "none"}})
        );
        assert_eq!(
            written(ThinkingDialect::Responses, &gpt, High),
            json!({
                "reasoning": {"effort": "high", "summary": "auto"},
                "include": ["reasoning.encrypted_content"]
            })
        );
    }

    #[test]
    fn apis_offer_only_what_they_can_spell() {
        let qwen = spec(true, &[], Some((None, Some(38_912))));
        assert!(ThinkingDialect::ChatEffort.levels(&qwen).is_empty());
        assert_eq!(
            written(ThinkingDialect::DashScope, &qwen, Low),
            json!({"enable_thinking": true, "thinking_budget": 2048})
        );
        let switch = spec(true, &[], None);
        assert_eq!(levels(ThinkingDialect::OpenRouter, &switch), vec![Off, Enabled]);
        assert_eq!(
            written(ThinkingDialect::OpenRouter, &switch, Enabled),
            json!({"reasoning": {"enabled": true}})
        );
        assert!(ThinkingDialect::Gemini
            .levels(&ModelThinkingSpec { supported: true, ..ModelThinkingSpec::default() })
            .is_empty());
        assert!(ThinkingDialect::Gemini.levels(&ModelThinkingSpec::default()).is_empty());
    }

    #[test]
    fn requests_name_only_offered_levels() {
        let pro = spec(false, &[Medium, High, Xhigh], None);
        let mut request = json!({});
        ThinkingDialect::ChatEffort
            .apply(
                &mut request,
                &RequestThinking {
                    spec: pro.clone(),
                    level: Some(Off),
                },
            )
            .expect("a model that cannot be switched off is left as built");
        assert_eq!(request, json!({}));
        assert!(ThinkingDialect::ChatEffort
            .apply(
                &mut request,
                &RequestThinking {
                    spec: pro,
                    level: Some(Low),
                },
            )
            .is_err());
    }

    #[test]
    fn provider_types_map_to_their_apis() {
        assert_eq!(
            ThinkingDialect::for_provider("OPENCODE", "https://opencode.ai/zen", "claude-opus-5"),
            Some(ThinkingDialect::Anthropic)
        );
        assert_eq!(
            ThinkingDialect::for_provider("DEEPSEEK", "https://api.deepseek.com/v1/responses", "deepseek-v4-flash"),
            Some(ThinkingDialect::DeepSeekResponses)
        );
        assert_eq!(
            ThinkingDialect::for_provider("DEEPSEEK", "https://api.deepseek.com/v1/chat/completions", "deepseek-v4-flash"),
            Some(ThinkingDialect::ChatThinkingType)
        );
        assert_eq!(ThinkingDialect::for_chat_provider("OPENROUTER_PARENT"), None);
        assert_eq!(ThinkingDialect::for_provider("MISTRAL", "https://api.mistral.ai/v1", "magistral"), None);
        assert_eq!(
            ThinkingDialect::for_provider("ALIYUN", "", "qwen3-max"),
            Some(ThinkingDialect::DashScope)
        );
    }
}
