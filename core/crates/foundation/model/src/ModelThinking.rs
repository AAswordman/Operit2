//! How a model can be told to think, and the level a request thinks at.
//!
//! models.dev describes each model's reasoning controls per host: a request field that turns
//! reasoning on and off, graded effort levels, and a reasoning-token budget. A model that
//! reasons with none of them always thinks and takes no control. Every model's choices map
//! onto one shared scale of levels, so a level picked on one model can be carried to another.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// How hard a model reasons, on one scale shared by every model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    /// No reasoning.
    Off,
    /// Reasoning at the model's own strength, for a model that can only be switched on.
    Enabled,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

impl ThinkingLevel {
    /// Every level, weakest first.
    pub const ALL: [ThinkingLevel; 8] = [
        ThinkingLevel::Off,
        ThinkingLevel::Enabled,
        ThinkingLevel::Minimal,
        ThinkingLevel::Low,
        ThinkingLevel::Medium,
        ThinkingLevel::High,
        ThinkingLevel::Xhigh,
        ThinkingLevel::Max,
    ];

    /// Reads a models.dev effort value; `default` and values outside the scale name no level.
    pub fn fromEffort(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" => Some(Self::Off),
            "minimal" => Some(Self::Minimal),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "xhigh" => Some(Self::Xhigh),
            "max" => Some(Self::Max),
            _ => None,
        }
    }

    /// Reads the name a level is stored and typed as.
    pub fn fromName(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|level| level.name().eq_ignore_ascii_case(value.trim()))
    }

    /// Returns the name the level is stored and typed as.
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Enabled => "enabled",
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
            Self::Max => "max",
        }
    }

    /// Returns the effort value requests spell this level with; `Enabled` is no effort.
    pub fn effortValue(self) -> Option<&'static str> {
        match self {
            Self::Off => Some("none"),
            Self::Enabled => None,
            Self::Minimal => Some("minimal"),
            Self::Low => Some("low"),
            Self::Medium => Some("medium"),
            Self::High => Some("high"),
            Self::Xhigh => Some("xhigh"),
            Self::Max => Some("max"),
        }
    }

    /// Returns the level's place on the shared scale; `Enabled` sits with `Medium`.
    pub fn rank(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Minimal => 1,
            Self::Low => 2,
            Self::Medium | Self::Enabled => 3,
            Self::High => 4,
            Self::Xhigh => 5,
            Self::Max => 6,
        }
    }

    /// Returns whether the model reasons at this level.
    pub fn isOn(self) -> bool {
        self != Self::Off
    }

    /// Picks, of the levels a model offers, the one this level carries over to: the level
    /// itself when offered, else the nearest strength, keeping thinking on or off when the
    /// model can; a tie goes to the stronger level. `None` when the model offers no level.
    pub fn nearestIn(self, levels: &[ThinkingLevel]) -> Option<ThinkingLevel> {
        if levels.contains(&self) {
            return Some(self);
        }
        let sameSide = levels
            .iter()
            .copied()
            .filter(|level| level.isOn() == self.isOn())
            .collect::<Vec<_>>();
        let candidates = if sameSide.is_empty() {
            levels.to_vec()
        } else {
            sameSide
        };
        candidates.into_iter().min_by_key(|level| {
            (
                level.rank().abs_diff(self.rank()),
                std::cmp::Reverse(level.rank()),
            )
        })
    }
}

/// Bounds of a reasoning-token budget; a bound models.dev does not give is `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThinkingBudgetRange {
    pub min: Option<i64>,
    pub max: Option<i64>,
}

impl ThinkingBudgetRange {
    /// Token counts of the budget levels, before the range clamps them.
    const PRESETS: [(ThinkingLevel, i64); 4] = [
        (ThinkingLevel::Low, 2_048),
        (ThinkingLevel::Medium, 8_192),
        (ThinkingLevel::High, 16_384),
        (ThinkingLevel::Xhigh, 32_768),
    ];

    /// Returns the levels a budget offers, each with the tokens it asks for, weakest first.
    /// A range above the largest preset adds its top as `Max`.
    pub fn levels(&self) -> Vec<ThinkingLevelOption> {
        let mut options: Vec<ThinkingLevelOption> = Vec::new();
        for (level, tokens) in Self::PRESETS {
            let tokens = self.clamp(tokens);
            if options
                .iter()
                .any(|option| option.budgetTokens == Some(tokens))
            {
                continue;
            }
            options.push(ThinkingLevelOption {
                level,
                budgetTokens: Some(tokens),
            });
        }
        let largest = Self::PRESETS[Self::PRESETS.len() - 1].1;
        if let Some(max) = self.max.filter(|max| *max > largest) {
            options.push(ThinkingLevelOption {
                level: ThinkingLevel::Max,
                budgetTokens: Some(max),
            });
        }
        options
    }

    /// Moves a token count into the range; a negative bound means a dynamic budget and holds
    /// no count back.
    fn clamp(&self, tokens: i64) -> i64 {
        let tokens = match self.min.filter(|min| *min > 0) {
            Some(min) => tokens.max(min),
            None => tokens,
        };
        match self.max.filter(|max| *max > 0) {
            Some(max) => tokens.min(max),
            None => tokens,
        }
    }
}

/// The reasoning controls one model accepts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelThinkingSpec {
    /// Whether the model reasons at all.
    pub supported: bool,
    /// Whether a request field turns reasoning on and off.
    pub toggle: bool,
    /// Effort levels the model takes, weakest first; `Off` when it takes effort `none`.
    pub efforts: Vec<ThinkingLevel>,
    /// The reasoning-token budget the model takes, when it takes one.
    pub budget: Option<ThinkingBudgetRange>,
}

impl ModelThinkingSpec {
    /// Describes a model that reasons and can only be switched on and off.
    pub fn switchable() -> Self {
        Self {
            supported: true,
            toggle: true,
            efforts: Vec::new(),
            budget: None,
        }
    }

    /// Reads models.dev's `reasoning_options` list of a model that reasons.
    pub fn fromReasoningOptions(options: &Value) -> Self {
        let mut spec = Self {
            supported: true,
            ..Self::default()
        };
        for option in options.as_array().into_iter().flatten() {
            match option.get("type").and_then(Value::as_str) {
                Some("toggle") => spec.toggle = true,
                Some("effort") => {
                    for value in option
                        .get("values")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                    {
                        let Some(level) = value.as_str().and_then(ThinkingLevel::fromEffort)
                        else {
                            continue;
                        };
                        if !spec.efforts.contains(&level) {
                            spec.efforts.push(level);
                        }
                    }
                }
                Some("budget_tokens") => {
                    spec.budget = Some(ThinkingBudgetRange {
                        min: option.get("min").and_then(Value::as_i64),
                        max: option.get("max").and_then(Value::as_i64),
                    });
                }
                _ => {}
            }
        }
        spec.efforts.sort_by_key(|level| level.rank());
        spec
    }

    /// Returns whether the model reasons on every request and takes no control.
    pub fn alwaysOn(&self) -> bool {
        self.supported && !self.toggle && self.efforts.is_empty() && self.budget.is_none()
    }
}

/// One level a model can be set to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ThinkingLevelOption {
    pub level: ThinkingLevel,
    /// Reasoning tokens the level asks for, when the model takes a budget rather than an effort.
    pub budgetTokens: Option<i64>,
}

/// Where the description of a model's thinking comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThinkingSpecSource {
    /// The model catalog describes the model.
    Catalog,
    /// The user described the model.
    Declared,
    /// The model runs on the built-in local runtime, which takes an on/off switch for every model.
    LocalRuntime,
    /// Nothing describes the model's thinking.
    Unknown,
}

/// The thinking choices of one configured model, for settings and the composer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelThinkingSettings {
    /// What the model accepts.
    pub spec: ModelThinkingSpec,
    pub source: ThinkingSpecSource,
    /// What the model catalog says of the model, `None` when it does not know the model.
    pub catalog: Option<ModelThinkingSpec>,
    /// Levels the model can be set to, weakest first; empty when requests cannot change how
    /// the model thinks.
    pub levels: Vec<ThinkingLevelOption>,
    /// The level requests use; `None` when `levels` is empty.
    pub current: Option<ThinkingLevel>,
    /// Whether the user set `current` on this model; otherwise it carries over the level last
    /// set on any model.
    pub chosen: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use super::ThinkingLevel::*;

    #[test]
    fn reads_reasoning_options_in_scale_order() {
        let spec = ModelThinkingSpec::fromReasoningOptions(&json!([
            {"type": "toggle"},
            {"type": "effort", "values": ["max", "low", "high", "default", null]},
            {"type": "budget_tokens", "min": 1024}
        ]));
        assert!(spec.supported && spec.toggle);
        assert_eq!(spec.efforts, vec![Low, High, Max]);
        assert_eq!(
            spec.budget,
            Some(ThinkingBudgetRange {
                min: Some(1024),
                max: None
            })
        );
        assert!(!spec.alwaysOn());
        assert!(ModelThinkingSpec::fromReasoningOptions(&json!([])).alwaysOn());
    }

    #[test]
    fn effort_none_reads_as_off() {
        let spec = ModelThinkingSpec::fromReasoningOptions(&json!([
            {"type": "effort", "values": ["none", "low", "medium", "high", "xhigh"]}
        ]));
        assert_eq!(spec.efforts, vec![Off, Low, Medium, High, Xhigh]);
    }

    #[test]
    fn nearest_level_keeps_on_or_off_then_strength() {
        assert_eq!(High.nearestIn(&[Off, Low, High, Max]), Some(High));
        assert_eq!(Medium.nearestIn(&[Off, Low, High, Max]), Some(High));
        assert_eq!(Minimal.nearestIn(&[Off, Enabled]), Some(Enabled));
        assert_eq!(Max.nearestIn(&[Off, Enabled]), Some(Enabled));
        assert_eq!(Off.nearestIn(&[Medium, High, Xhigh]), Some(Medium));
        assert_eq!(Enabled.nearestIn(&[Low, High]), Some(High));
        assert_eq!(Xhigh.nearestIn(&[Off, Low, Medium, High]), Some(High));
        assert_eq!(Low.nearestIn(&[]), None);
    }

    #[test]
    fn budget_levels_clamp_into_the_range() {
        let flashLite = ThinkingBudgetRange {
            min: Some(512),
            max: Some(24_576),
        };
        assert_eq!(
            flashLite
                .levels()
                .iter()
                .map(|option| (option.level, option.budgetTokens))
                .collect::<Vec<_>>(),
            vec![
                (Low, Some(2_048)),
                (Medium, Some(8_192)),
                (High, Some(16_384)),
                (Xhigh, Some(24_576)),
            ]
        );
        let large = ThinkingBudgetRange {
            min: Some(0),
            max: Some(262_144),
        };
        assert_eq!(
            large.levels().last().copied(),
            Some(ThinkingLevelOption {
                level: Max,
                budgetTokens: Some(262_144)
            })
        );
        let small = ThinkingBudgetRange {
            min: None,
            max: Some(4_096),
        };
        assert_eq!(
            small
                .levels()
                .iter()
                .map(|option| option.level)
                .collect::<Vec<_>>(),
            vec![Low, Medium]
        );
    }

    #[test]
    fn level_names_round_trip() {
        for level in ThinkingLevel::ALL {
            assert_eq!(ThinkingLevel::fromName(level.name()), Some(level));
            assert_eq!(
                serde_json::to_value(level).unwrap(),
                json!(level.name())
            );
        }
    }
}
