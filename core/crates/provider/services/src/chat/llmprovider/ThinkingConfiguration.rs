use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::chat::llmprovider::AIService::AiServiceError;

/// Applies model-aware thinking settings to an already-built provider request.
pub struct ThinkingConfigurationApplier;

#[derive(Debug, Deserialize)]
struct ThinkingRule {
    model_prefix: Vec<String>,
    model_regex: Vec<String>,
    endpoint_suffix: Vec<String>,
    control: ThinkingControl,
    required: bool,
    enable: Vec<ThinkingAction>,
    disable: Vec<ThinkingAction>,
    options: Vec<ThinkingOption>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingControl {
    Levels,
    ToggleOnly,
    Unsupported,
}

#[derive(Debug, Deserialize)]
struct ThinkingOption {
    #[serde(default)]
    id: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    value: Option<Value>,
    #[serde(default)]
    actions: Vec<ThinkingAction>,
}

/// Describes one model-specific thinking choice for configuration user interfaces.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ThinkingOptionDescriptor {
    pub id: String,
    pub label: String,
}

/// Describes the thinking controls supported by one resolved provider model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ThinkingSettingsDescriptor {
    pub control: ThinkingControl,
    pub required: bool,
    pub options: Vec<ThinkingOptionDescriptor>,
}

impl ThinkingOption {
    /// Returns the persisted option identifier used by the model selector.
    fn option_id(&self) -> String {
        if !self.id.is_empty() {
            return self.id.clone();
        }
        match &self.value {
            Some(Value::String(value)) => value.clone(),
            Some(value) => value.to_string(),
            None => String::new(),
        }
    }

    /// Returns the human-readable label persisted by the thinking configuration.
    fn option_label(&self) -> String {
        if self.label.is_empty() {
            self.option_id()
        } else {
            self.label.clone()
        }
    }
}

#[derive(Debug, Deserialize)]
struct ThinkingAction {
    path: String,
    value: Value,
}

impl ThinkingConfigurationApplier {
    /// Validates persisted thinking rules before they are saved to a provider profile.
    pub fn validate(thinking_configurations: &str) -> Result<(), String> {
        let rules = serde_json::from_str::<Vec<ThinkingRule>>(thinking_configurations)
            .map_err(|error| error.to_string())?;
        for rule in rules {
            for expression in rule.model_regex {
                Regex::new(&expression).map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }

    /// Resolves thinking controls for one provider-owned model/endpoint configuration.
    pub fn describe(
        _provider_type_id: &str,
        model_name: &str,
        api_endpoint: &str,
        thinking_configurations: &str,
    ) -> Result<ThinkingSettingsDescriptor, String> {
        let rules = serde_json::from_str::<Vec<ThinkingRule>>(thinking_configurations)
            .map_err(|error| error.to_string())?;
        for rule in rules {
            if rule.matches(model_name, api_endpoint)? {
                return Ok(ThinkingSettingsDescriptor {
                    control: rule.control,
                    required: rule.required,
                    options: rule
                        .options
                        .iter()
                        .map(|option| {
                            let id = option.option_id();
                            ThinkingOptionDescriptor {
                                label: option.option_label(),
                                id,
                            }
                        })
                        .collect(),
                });
            }
        }
        Ok(ThinkingSettingsDescriptor {
            control: ThinkingControl::Unsupported,
            required: false,
            options: Vec::new(),
        })
    }

    /// Applies the first matching thinking rule to one provider request object.
    pub fn apply(
        request: &mut Value,
        provider_type_id: &str,
        model_name: &str,
        api_endpoint: &str,
        enable_thinking: bool,
        thinking_quality_level: i32,
        thinking_configurations: &str,
        thinking_option_id: &str,
    ) -> Result<(), AiServiceError> {
        let rules = serde_json::from_str::<Vec<ThinkingRule>>(thinking_configurations)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let rule = rules
            .into_iter()
            .map(|candidate| {
                let matches = candidate
                    .matches(model_name, api_endpoint)
                    .map_err(AiServiceError::RequestFailed)?;
                Ok((candidate, matches))
            })
            .collect::<Result<Vec<_>, AiServiceError>>()?
            .into_iter()
            .find_map(|(candidate, matches)| matches.then_some(candidate));
        let Some(rule) = rule else {
            return Ok(());
        };
        if rule.control == ThinkingControl::Unsupported {
            return Ok(());
        }
        let thinking_enabled = enable_thinking || rule.required;
        let actions = if thinking_enabled {
            &rule.enable
        } else {
            &rule.disable
        };
        for action in actions {
            put_json_path(request, &action.path, action.value.clone())?;
        }
        if thinking_enabled && rule.control == ThinkingControl::Levels {
            let option = if thinking_option_id.is_empty() {
                let index = quality_option_index(rule.options.len(), thinking_quality_level);
                rule.options.get(index).ok_or_else(|| {
                    AiServiceError::RequestFailed(format!(
                        "thinking quality has no option: provider={provider_type_id} model={model_name} index={index}"
                    ))
                })?
            } else {
                rule.options
                    .iter()
                    .find(|candidate| candidate.option_id() == thinking_option_id)
                    .ok_or_else(|| {
                        AiServiceError::RequestFailed(format!(
                            "thinking option is not supported: provider={provider_type_id} model={model_name} option={thinking_option_id}"
                        ))
                    })?
            };
            apply_option(request, option)?;
        }
        Ok(())
    }
}

impl ThinkingRule {
    /// Determines whether this provider-owned rule matches one model/endpoint combination.
    fn matches(&self, model_name: &str, api_endpoint: &str) -> Result<bool, String> {
        let model = model_name.trim();
        if !self.model_prefix.is_empty()
            && !self.model_prefix.iter().any(|prefix| {
                model
                    .to_ascii_lowercase()
                    .starts_with(&prefix.to_ascii_lowercase())
            })
        {
            return Ok(false);
        }
        if !self.model_regex.is_empty()
            && !self
                .model_regex
                .iter()
                .map(|expression| Regex::new(expression).map(|regex| regex.is_match(model)))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?
                .into_iter()
                .any(|matches| matches)
        {
            return Ok(false);
        }
        if self.endpoint_suffix.is_empty() {
            return Ok(true);
        }
        let endpoint = normalized_endpoint(api_endpoint);
        Ok(self
            .endpoint_suffix
            .iter()
            .map(|suffix| normalized_endpoint(suffix))
            .any(|suffix| endpoint.ends_with(&suffix)))
    }
}

/// Normalizes an endpoint before suffix matching.
fn normalized_endpoint(value: &str) -> String {
    value
        .trim()
        .split(['?', '#'])
        .next()
        .expect("split always produces one endpoint segment")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

/// Maps the global four-step UI control onto a rule's ordered option list.
fn quality_option_index(option_count: usize, quality_level: i32) -> usize {
    assert!(
        option_count > 0,
        "thinking levels rule must declare options"
    );
    if option_count == 1 {
        return 0;
    }
    let quality_step = match quality_level {
        1 => 0usize,
        2 => 1usize,
        3 => 2usize,
        4 => 3usize,
        value => panic!("thinking quality level is invalid: {value}"),
    };
    quality_step * (option_count - 1) / 3
}

/// Applies every direct value and action declared by one thinking option.
fn apply_option(request: &mut Value, option: &ThinkingOption) -> Result<(), AiServiceError> {
    if !option.path.is_empty() {
        let value = option.value.clone().ok_or_else(|| {
            AiServiceError::RequestFailed(format!(
                "thinking option {} has no value for path {}",
                option.option_id(),
                option.path
            ))
        })?;
        put_json_path(request, &option.path, value)?;
    }
    for action in &option.actions {
        put_json_path(request, &action.path, action.value.clone())?;
    }
    Ok(())
}

/// Writes a value to a dot-separated JSON object path.
fn put_json_path(root: &mut Value, path: &str, value: Value) -> Result<(), AiServiceError> {
    let segments = path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let Some((last, parents)) = segments.split_last() else {
        return Err(AiServiceError::RequestFailed(
            "thinking rule action path is empty".to_string(),
        ));
    };
    let mut object = root.as_object_mut().ok_or_else(|| {
        AiServiceError::RequestFailed("thinking request root must be an object".to_string())
    })?;
    for parent in parents {
        let entry = object
            .entry((*parent).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        object = entry.as_object_mut().ok_or_else(|| {
            AiServiceError::RequestFailed(format!(
                "thinking rule path conflicts with request field: {parent}"
            ))
        })?;
    }
    object.insert((*last).to_string(), value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ThinkingConfigurationApplier;
    use operit_model::ModelConfigData::thinkingConfigurationsForProvider;
    use serde_json::json;

    /// Applies a nested Responses API reasoning option.
    #[test]
    fn applies_responses_reasoning_option() {
        let mut request = json!({});
        let thinking_configurations = thinkingConfigurationsForProvider("OPENAI_RESPONSES");
        ThinkingConfigurationApplier::apply(
            &mut request,
            "OPENAI_RESPONSES",
            "gpt-5",
            "https://api.openai.com/v1/responses",
            true,
            3,
            &thinking_configurations,
            "",
        )
        .unwrap();
        assert_eq!(request["reasoning"]["effort"], "high");
        assert_eq!(request["reasoning"]["summary"], "auto");
    }

    /// Applies a Gemini thinking budget without matching on raw string fragments.
    #[test]
    fn applies_gemini_thinking_budget() {
        let mut request = json!({"generationConfig": {}});
        let thinking_configurations = thinkingConfigurationsForProvider("GOOGLE");
        ThinkingConfigurationApplier::apply(
            &mut request,
            "GOOGLE",
            "gemini-2.5-pro",
            "https://generativelanguage.googleapis.com",
            true,
            2,
            &thinking_configurations,
            "",
        )
        .unwrap();
        assert_eq!(
            request["generationConfig"]["thinkingConfig"]["thinkingBudget"],
            4096
        );
    }
}
