use std::collections::BTreeMap;
use std::sync::Arc;

use operit_model::ModelParameter::ModelParameter;
use operit_model::PromptTurn::PromptTurn;
use operit_model::ToolPrompt::ToolPrompt;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::chat::llmprovider::ThinkingDialect::RequestThinking;
use crate::runtime_support::ProviderFunctionModelBinding;

/// Retains a completed conversation's actual provider input, including tool rounds.
#[derive(Clone, Debug)]
pub struct ConversationContext {
    pub history: Vec<PromptTurn>,
    pub available_tools: Vec<ToolPrompt>,
    pub thinking: RequestThinking,
    pub model_parameters: Vec<ModelParameter<Value>>,
    pub model_binding: ProviderFunctionModelBinding,
}

/// Associates prepared context with the exact persisted history it represents.
#[derive(Clone, Debug)]
struct CommittedContext {
    source_digest: [u8; 32],
    context: Arc<ConversationContext>,
}

/// Memoizes one completed context per chat participant without copying it into messages.
#[derive(Clone, Debug, Default)]
pub struct ConversationContextCache {
    pending: BTreeMap<(String, String), Arc<ConversationContext>>,
    committed: BTreeMap<(String, String), CommittedContext>,
}

impl ConversationContextCache {
    /// Captures completed provider history before the runtime closes its response stream.
    pub fn record(
        &mut self,
        chat_id: String,
        participant_id: String,
        context: ConversationContext,
    ) {
        self.pending
            .insert((chat_id, participant_id), Arc::new(context));
    }

    /// Binds a prepared context only after its assistant message has been persisted.
    pub fn commit(&mut self, chat_id: &str, participant_id: &str, source: Vec<PromptTurn>) {
        let key = (chat_id.to_string(), participant_id.to_string());
        if let Some(context) = self.pending.remove(&key) {
            self.committed.insert(
                key,
                CommittedContext {
                    source_digest: sourceFingerprint(&source),
                    context,
                },
            );
        }
    }

    /// Reuses preparation only for the same participant and byte-identical source history.
    pub fn get(
        &self,
        chat_id: &str,
        participant_id: &str,
        source: &[PromptTurn],
    ) -> Option<Arc<ConversationContext>> {
        self.committed
            .get(&(chat_id.to_string(), participant_id.to_string()))
            .filter(|entry| entry.source_digest == sourceFingerprint(source))
            .map(|entry| entry.context.clone())
    }
}

/// Hashes canonical source fields so map iteration order cannot invalidate a context.
#[allow(non_snake_case)]
pub fn sourceFingerprint(source: &[PromptTurn]) -> [u8; 32] {
    let mut canonical = serde_json::to_value(source).expect("prompt turns must serialize");
    canonical.sort_all_objects();
    Sha256::digest(canonical.to_string().as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_model::PromptTurn::PromptTurnKind;

    /// Creates a provider context whose prepared system differs from the stored source.
    fn context() -> ConversationContext {
        ConversationContext {
            history: vec![PromptTurn::new(
                PromptTurnKind::SYSTEM,
                "actual system from hooks",
            )],
            available_tools: Vec::new(),
            thinking: RequestThinking::default(),
            model_parameters: Vec::new(),
            model_binding: ProviderFunctionModelBinding {
                providerId: "provider".into(),
                modelId: "model".into(),
            },
        }
    }

    /// Requires persistence acknowledgement and rejects edits, later turns, and other participants.
    #[test]
    fn context_reuse_requires_the_exact_committed_source() {
        let mut cache = ConversationContextCache::default();
        let source = vec![PromptTurn::new(PromptTurnKind::USER, "question")];
        cache.record("chat".into(), "alice".into(), context());
        assert!(cache.get("chat", "alice", &source).is_none());
        cache.commit("chat", "alice", source.clone());
        let replay = cache.get("chat", "alice", &source).unwrap();
        assert_eq!(replay.history[0].content, "actual system from hooks");
        assert!(Arc::ptr_eq(
            &replay,
            &cache.get("chat", "alice", &source).unwrap()
        ));
        assert!(cache.get("other chat", "alice", &source).is_none());
        assert!(cache.get("chat", "bob", &source).is_none());
        assert!(cache.get("chat", "alice", &[]).is_none());
        let mut edited = source.clone();
        edited[0].content = "edited question".into();
        assert!(cache.get("chat", "alice", &edited).is_none());
        let mut later = source;
        later.push(PromptTurn::new(PromptTurnKind::USER, "later question"));
        assert!(cache.get("chat", "alice", &later).is_none());
    }

    /// A response still streaming must not replace the previous committed background-summary source.
    #[test]
    fn pending_response_preserves_the_previous_committed_context() {
        let mut cache = ConversationContextCache::default();
        let source = vec![PromptTurn::new(PromptTurnKind::USER, "question")];
        cache.record("chat".into(), "alice".into(), context());
        cache.commit("chat", "alice", source.clone());
        let previous = cache.get("chat", "alice", &source).unwrap();
        let mut next = context();
        next.history
            .push(PromptTurn::new(PromptTurnKind::ASSISTANT, "next reply"));
        cache.record("chat".into(), "alice".into(), next);
        assert!(Arc::ptr_eq(
            &previous,
            &cache.get("chat", "alice", &source).unwrap()
        ));
    }

    /// Source fingerprints depend on metadata values rather than hash-map insertion order.
    #[test]
    fn source_metadata_is_canonical_and_part_of_the_cache_key() {
        let mut left = PromptTurn::new(PromptTurnKind::ASSISTANT, "answer");
        left.metadata
            .insert("b".into(), serde_json::json!({"z": 1, "a": 2}));
        left.metadata.insert("a".into(), serde_json::json!("alice"));
        let mut right = PromptTurn::new(PromptTurnKind::ASSISTANT, "answer");
        right
            .metadata
            .insert("a".into(), serde_json::json!("alice"));
        right
            .metadata
            .insert("b".into(), serde_json::json!({"a": 2, "z": 1}));
        assert_eq!(
            sourceFingerprint(&[left.clone()]),
            sourceFingerprint(&[right.clone()])
        );
        right.metadata.insert("a".into(), serde_json::json!("bob"));
        assert_ne!(sourceFingerprint(&[left]), sourceFingerprint(&[right]));
    }
}
