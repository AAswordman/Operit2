use serde_json::json;

use super::ModelParameter::{ParameterCategory, ParameterValueType};

#[derive(Clone, Debug, PartialEq)]
pub struct ParameterDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub apiName: &'static str,
    pub description: &'static str,
    pub defaultValue: serde_json::Value,
    pub valueType: ParameterValueType,
    pub category: ParameterCategory,
    pub minValue: Option<serde_json::Value>,
    pub maxValue: Option<serde_json::Value>,
}

pub struct StandardModelParameters;

impl StandardModelParameters {
    pub const DEFAULT_MAX_TOKENS: i32 = 4096;
    pub const DEFAULT_TEMPERATURE: f32 = 1.0;
    pub const DEFAULT_TOP_P: f32 = 1.0;
    pub const DEFAULT_TOP_K: i32 = 0;
    pub const DEFAULT_PRESENCE_PENALTY: f32 = 0.0;
    pub const DEFAULT_FREQUENCY_PENALTY: f32 = 0.0;
    pub const DEFAULT_REPETITION_PENALTY: f32 = 1.0;

    pub fn DEFINITIONS() -> Vec<ParameterDefinition> {
        vec![
            ParameterDefinition {
                id: "max_tokens",
                name: "Max tokens",
                apiName: "max_tokens",
                description: "Maximum number of tokens to generate in one response",
                defaultValue: json!(Self::DEFAULT_MAX_TOKENS),
                valueType: ParameterValueType::INT,
                category: ParameterCategory::GENERATION,
                minValue: Some(json!(1)),
                maxValue: None,
            },
            ParameterDefinition {
                id: "temperature",
                name: "Temperature",
                apiName: "temperature",
                description: "Controls randomness: lower is more deterministic, higher is more random",
                defaultValue: json!(Self::DEFAULT_TEMPERATURE),
                valueType: ParameterValueType::FLOAT,
                category: ParameterCategory::CREATIVITY,
                minValue: Some(json!(0.0)),
                maxValue: Some(json!(2.0)),
            },
            ParameterDefinition {
                id: "top_p",
                name: "Top-p sampling",
                apiName: "top_p",
                description: "Alternative to temperature: consider only tokens within cumulative probability top-p",
                defaultValue: json!(Self::DEFAULT_TOP_P),
                valueType: ParameterValueType::FLOAT,
                category: ParameterCategory::CREATIVITY,
                minValue: Some(json!(0.0)),
                maxValue: Some(json!(1.0)),
            },
            ParameterDefinition {
                id: "top_k",
                name: "Top-k sampling",
                apiName: "top_k",
                description: "Consider only the top-k tokens by probability. 0 disables",
                defaultValue: json!(Self::DEFAULT_TOP_K),
                valueType: ParameterValueType::INT,
                category: ParameterCategory::CREATIVITY,
                minValue: Some(json!(0)),
                maxValue: Some(json!(100)),
            },
            ParameterDefinition {
                id: "presence_penalty",
                name: "Presence penalty",
                apiName: "presence_penalty",
                description: "Encourages new topics: higher values reduce repetition of existing tokens",
                defaultValue: json!(Self::DEFAULT_PRESENCE_PENALTY),
                valueType: ParameterValueType::FLOAT,
                category: ParameterCategory::REPETITION,
                minValue: Some(json!(-2.0)),
                maxValue: Some(json!(2.0)),
            },
            ParameterDefinition {
                id: "frequency_penalty",
                name: "Frequency penalty",
                apiName: "frequency_penalty",
                description: "Reduces repetition: higher values penalize tokens based on frequency",
                defaultValue: json!(Self::DEFAULT_FREQUENCY_PENALTY),
                valueType: ParameterValueType::FLOAT,
                category: ParameterCategory::REPETITION,
                minValue: Some(json!(-2.0)),
                maxValue: Some(json!(2.0)),
            },
            ParameterDefinition {
                id: "repetition_penalty",
                name: "Repetition penalty",
                apiName: "repetition_penalty",
                description: "Further reduces repetition: 1.0 means no penalty; values > 1.0 discourage repetition",
                defaultValue: json!(Self::DEFAULT_REPETITION_PENALTY),
                valueType: ParameterValueType::FLOAT,
                category: ParameterCategory::REPETITION,
                minValue: Some(json!(0.0)),
                maxValue: Some(json!(2.0)),
            },
        ]
    }
}
