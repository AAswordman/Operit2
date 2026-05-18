use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum PromptTurnKind {
    SYSTEM,
    USER,
    ASSISTANT,
    TOOL_CALL,
    TOOL_RESULT,
    SUMMARY,
}

impl PromptTurnKind {
    pub fn from_role(role: &str) -> Self {
        match role.trim().to_ascii_lowercase().as_str() {
            "system" => Self::SYSTEM,
            "user" => Self::USER,
            "assistant" | "ai" => Self::ASSISTANT,
            "tool" | "tool_result" => Self::TOOL_RESULT,
            "tool_call" | "tool_use" => Self::TOOL_CALL,
            "summary" => Self::SUMMARY,
            _ => Self::USER,
        }
    }

    pub fn role(&self) -> &'static str {
        match self {
            Self::SYSTEM => "system",
            Self::USER => "user",
            Self::ASSISTANT => "assistant",
            Self::TOOL_CALL => "tool_call",
            Self::TOOL_RESULT => "tool_result",
            Self::SUMMARY => "summary",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PromptTurn {
    pub kind: PromptTurnKind,
    pub content: String,
    #[serde(rename = "toolName", skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    pub metadata: HashMap<String, Value>,
}

impl PromptTurn {
    pub fn new(kind: PromptTurnKind, content: impl Into<String>) -> Self {
        Self {
            kind,
            content: content.into(),
            tool_name: None,
            metadata: HashMap::new(),
        }
    }

    pub fn from_role(
        role: impl AsRef<str>,
        content: impl Into<String>,
        tool_name: Option<String>,
        metadata: HashMap<String, Value>,
    ) -> Self {
        Self {
            kind: PromptTurnKind::from_role(role.as_ref()),
            content: content.into(),
            tool_name,
            metadata,
        }
    }

    pub fn role(&self) -> &'static str {
        self.kind.role()
    }

    pub fn with_content(&self, new_content: impl Into<String>) -> Self {
        let new_content = new_content.into();
        if new_content == self.content {
            self.clone()
        } else {
            let mut copied = self.clone();
            copied.content = new_content;
            copied
        }
    }
}

#[allow(non_snake_case)]
pub fn appendUserTurnIfMissing(mut turns: Vec<PromptTurn>, message: impl Into<String>) -> Vec<PromptTurn> {
    let message = message.into();
    if message.trim().is_empty() {
        return turns;
    }

    let should_append = match turns.last() {
        Some(turn) => turn.kind != PromptTurnKind::USER || turn.content != message,
        None => true,
    };

    if should_append {
        turns.push(PromptTurn::new(PromptTurnKind::USER, message));
    }
    turns
}

#[allow(non_snake_case)]
pub fn mergeAdjacentTurns(turns: &[PromptTurn]) -> Vec<PromptTurn> {
    merge_adjacent_turns_by(turns, |previous, current| {
        let excluded: HashSet<PromptTurnKind> = [
            PromptTurnKind::SYSTEM,
            PromptTurnKind::TOOL_CALL,
            PromptTurnKind::TOOL_RESULT,
        ]
        .into_iter()
        .collect();
        previous.kind == current.kind
            && !excluded.contains(&previous.kind)
            && previous.tool_name == current.tool_name
    })
}

pub fn merge_adjacent_turns_by<F>(turns: &[PromptTurn], should_merge: F) -> Vec<PromptTurn>
where
    F: Fn(&PromptTurn, &PromptTurn) -> bool,
{
    if turns.len() <= 1 {
        return turns.to_vec();
    }

    let mut merged: Vec<PromptTurn> = Vec::new();
    for turn in turns {
        let last_index = merged.len().checked_sub(1);
        match last_index {
            Some(index) if should_merge(&merged[index], turn) => {
                let mut previous = merged[index].clone();
                previous.content = format!("{}\n{}", previous.content, turn.content);
                if !turn.metadata.is_empty() {
                    previous.metadata.extend(turn.metadata.clone());
                }
                merged[index] = previous;
            }
            _ => merged.push(turn.clone()),
        }
    }
    merged
}

#[allow(non_snake_case)]
pub fn toPromptTurns(pairs: &[(String, String)]) -> Vec<PromptTurn> {
    pairs
        .iter()
        .map(|(role, content)| PromptTurn::from_role(role, content.clone(), None, HashMap::new()))
        .collect()
}

#[allow(non_snake_case)]
pub fn toRoleContentPairs(turns: &[PromptTurn]) -> Vec<(String, String)> {
    turns
        .iter()
        .map(|turn| (turn.role().to_string(), turn.content.clone()))
        .collect()
}
