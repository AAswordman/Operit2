use crate::SqliteStore::{SqliteStore, SqliteStoreError};
use operit_model::PluginExtensionTarget::PluginExtensionTarget;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

/// Tracks running conversations and the exact revisions whose execution snapshots are still dirty.
#[derive(Default)]
pub(crate) struct ChatExecutionLeaseState {
    nextId: u64,
    active: BTreeMap<String, ActiveExecution>,
}

/// Identifies one running generation without interpreting any plugin extension contents.
struct ActiveExecution {
    id: u64,
    dirtyRevisions: BTreeSet<(i64, i32)>,
}

/// Shares one idempotently releasable lease across the awaited request and streaming completion lifecycle.
#[derive(Clone)]
pub struct ChatExecutionLease {
    inner: Arc<ExecutionLeaseOwner>,
}

/// Releases only its own generation when the last request or runtime owner disappears.
struct ExecutionLeaseOwner {
    state: Arc<Mutex<ChatExecutionLeaseState>>,
    chatId: String,
    id: u64,
}

impl std::fmt::Debug for ChatExecutionLease {
    /// Formats generic lease identity without exposing stored extension values.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChatExecutionLease")
            .field("chatId", &self.inner.chatId)
            .field("id", &self.inner.id)
            .finish()
    }
}

impl ExecutionLeaseOwner {
    /// Removes only this generation and never clears a newer execution lease for the same conversation.
    fn release(&self) -> Result<(), SqliteStoreError> {
        let mut state = self.state.lock().map_err(|_| {
            SqliteStoreError::Message("Chat execution lease mutex poisoned".to_string())
        })?;
        if state
            .active
            .get(&self.chatId)
            .is_some_and(|execution| execution.id == self.id)
        {
            state.active.remove(&self.chatId);
        }
        Ok(())
    }
}

impl Drop for ExecutionLeaseOwner {
    /// Guarantees failed requests and cancelled futures release their generation ownership.
    fn drop(&mut self) {
        self.release()
            .expect("Chat execution lease must release without a poisoned mutex");
    }
}

impl ChatExecutionLease {
    /// Arms cleanup for awaited request failures until ownership is handed to the streaming runtime.
    pub fn requestGuard(&self) -> ChatExecutionRequestGuard {
        ChatExecutionRequestGuard {
            lease: Some(self.clone()),
        }
    }

    /// Freezes one exact revision snapshot until this generation finishes or is cancelled.
    pub fn protectRevision(
        &self,
        messageTimestamp: i64,
        variantIndex: i32,
    ) -> Result<(), SqliteStoreError> {
        if variantIndex < 0 {
            return Err(SqliteStoreError::Message(
                "Dirty revision index must be nonnegative".to_string(),
            ));
        }
        let mut state = self.inner.state.lock().map_err(|_| {
            SqliteStoreError::Message("Chat execution lease mutex poisoned".to_string())
        })?;
        let execution = state
            .active
            .get_mut(&self.inner.chatId)
            .filter(|execution| execution.id == self.inner.id)
            .ok_or_else(|| {
                SqliteStoreError::Message("Chat execution lease is no longer active".to_string())
            })?;
        execution
            .dirtyRevisions
            .insert((messageTimestamp, variantIndex));
        Ok(())
    }

    /// Ends this execution explicitly while allowing stale callback clones to drop safely.
    pub fn release(&self) -> Result<(), SqliteStoreError> {
        self.inner.release()
    }
}

impl SqliteStore {
    /// Acquires metadata protection before configuration resolution, releasing all locks before awaiting plugins.
    pub fn beginChatExecution(&self, chatId: &str) -> Result<ChatExecutionLease, SqliteStoreError> {
        if chatId.trim().is_empty() {
            return Err(SqliteStoreError::Message(
                "Execution requires a real chat id".to_string(),
            ));
        }
        let mut state = self.executionLeases.lock().map_err(|_| {
            SqliteStoreError::Message("Chat execution lease mutex poisoned".to_string())
        })?;
        if state.active.contains_key(chatId) {
            return Err(SqliteStoreError::Message(format!(
                "Conversation is executing: {chatId}"
            )));
        }
        if self
            .queryOne(
                "SELECT id FROM chats WHERE id = ?1",
                crate::sqliteParams![chatId],
            )?
            .is_none()
        {
            return Err(SqliteStoreError::Message(format!(
                "Execution chat does not exist: {chatId}"
            )));
        }
        state.nextId = state.nextId.checked_add(1).ok_or_else(|| {
            SqliteStoreError::Message("Chat execution lease id overflow".to_string())
        })?;
        let id = state.nextId;
        state.active.insert(
            chatId.to_string(),
            ActiveExecution {
                id,
                dirtyRevisions: BTreeSet::new(),
            },
        );
        Ok(ChatExecutionLease {
            inner: Arc::new(ExecutionLeaseOwner {
                state: self.executionLeases.clone(),
                chatId: chatId.to_string(),
                id,
            }),
        })
    }

    /// Holds the shared execution guard through one metadata transaction to prevent acquire-versus-write races.
    pub(crate) fn withPluginExtensionMutation<T>(
        &self,
        target: &PluginExtensionTarget,
        action: impl FnOnce() -> Result<T, SqliteStoreError>,
    ) -> Result<T, SqliteStoreError> {
        let state = self.executionLeases.lock().map_err(|_| {
            SqliteStoreError::Message("Chat execution lease mutex poisoned".to_string())
        })?;
        if let Some(execution) = state.active.get(target.chatId()) {
            let protected = match target {
                PluginExtensionTarget::Chat { .. } => true,
                PluginExtensionTarget::Message {
                    messageTimestamp,
                    variantIndex,
                    ..
                } => execution
                    .dirtyRevisions
                    .contains(&(*messageTimestamp, *variantIndex)),
            };
            if protected {
                return Err(SqliteStoreError::Message(format!(
                    "Cannot mutate executing conversation identity or a dirty revision: {target:?}"
                )));
            }
        }
        let result = action();
        drop(state);
        result
    }
}

/// Releases metadata protection on every early error even when a runtime clone retains the lease.
pub struct ChatExecutionRequestGuard {
    lease: Option<ChatExecutionLease>,
}

impl ChatExecutionRequestGuard {
    /// Transfers successful request ownership to the already-installed streaming runtime.
    pub fn handoff(&mut self) {
        self.lease = None;
    }
}

impl Drop for ChatExecutionRequestGuard {
    /// Releases a failed or cancelled awaited request without unlocking any newer generation.
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take() {
            lease
                .release()
                .expect("Failed execution request lease must release");
        }
    }
}
