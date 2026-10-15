//! Owner-scoped native admission, cancellation and commit receipts for existing chat execution.
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use operit_plugin_sdk::js_sdk::chat::{ChatInitialTurn, ChatRuntime, ChatSendRequest, ChatTurnInput};
use operit_plugin_sdk::js_sdk::results::{
    MessageSendOutcome, MessageSendResultData, MessageSendStatus,
};
use operit_tools::runtime_support::{ChatSequenceFinishResultData, ChatSequenceStatus, ChatTurnHandle};
use operit_model::ChatTurnOptions::ChatOutputObserver;
use operit_store::ChatExecutionLease::ChatExecutionLease;
use operit_store::repository::ChatHistoryManager::ChatHistoryManager;
use tokio::sync::watch;

use crate::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;

/// Owns exactly one real conversation lease and the receipts of its admitted native executions.
pub(crate) struct NativeChatSequence {
    pub owner: String,
    pub chatId: String,
    pub slot: ChatRuntimeSlot,
    pub lease: ChatExecutionLease,
    pub notifyReply: bool,
    pub userMessageTimestamp: Option<i64>,
    executions: Vec<String>,
    activeExecution: Option<String>,
    finishing: bool,
}

/// Keeps the actual terminal result observable independently from the Holder and plugin-local request keys.
struct NativeChatExecution {
    owner: String,
    sequenceId: String,
    cancelled: Arc<AtomicBool>,
    terminal: watch::Sender<Option<Result<MessageSendResultData, String>>>,
    committedReceipt: Option<MessageSendResultData>,
}

/// Serializes sequence admission while leaving all plugin and model awaits outside this mutex.
#[derive(Default)]
pub(crate) struct ChatTurnTransport {
    sequences: HashMap<String, NativeChatSequence>,
    executions: HashMap<String, NativeChatExecution>,
}

/// Carries immutable native-owned admission state into one asynchronous execution.
#[derive(Clone)]
pub(crate) struct AdmittedChatTurn {
    pub handle: ChatTurnHandle,
    pub slot: ChatRuntimeSlot,
    pub lease: ChatExecutionLease,
    pub cancelled: Arc<AtomicBool>,
    pub work: AdmittedChatTurnWork,
    /// Retains the exact admitted completion-notification policy for authoritative submit hooks.
    pub notifyReply: bool,
    pub outputObserver: Option<Arc<dyn ChatOutputObserver>>,
}

/// Preserves the original input only on submission and only a canonical timestamp on continuation.
#[derive(Clone)]
pub(crate) enum AdmittedChatTurnWork {
    Submit { input: ChatTurnInput, turn: ChatInitialTurn },
    Continue { userMessageTimestamp: i64, participantId: String },
}

impl ChatTurnTransport {
    /// Admits one send, acquiring its real store lease and validating an exact persisted continuation.
    pub fn admit(&mut self, owner: &str, request: ChatSendRequest, manager: &ChatHistoryManager,
        outputObserver: Option<Arc<dyn ChatOutputObserver>>) -> Result<AdmittedChatTurn, String> {
        if owner.trim().is_empty() { return Err("Chat admission requires an authenticated owner".to_string()); }
        request.validate()?;
        let (chatId, runtime, notifyReply, userMessageTimestamp, work) = match request {
            ChatSendRequest::Submit { chatId, runtime, input, turn, notifyReply } => {
                if let Some(timestamp) = input.replyToMessageTimestamp.as_value().copied() {
                    manager.loadChatMessageVariant(&chatId, timestamp, 0).map_err(|error| error.to_string())?;
                }
                (chatId, runtime, notifyReply, None, AdmittedChatTurnWork::Submit { input, turn })
            }
            ChatSendRequest::Continue { chatId, runtime, userMessageTimestamp, participantId, notifyReply } => {
                let source = manager.loadChatMessageVariant(&chatId, userMessageTimestamp, 0).map_err(|error| error.to_string())?;
                if source.sender != "user" { return Err("Continuation source is not a committed user message".to_string()); }
                let messages = manager.loadChatMessages(&chatId).map_err(|error| error.to_string())?;
                if messages.iter().any(|message| message.sender == "user" && message.timestamp > userMessageTimestamp) {
                    return Err("Continuation source was superseded by another committed user message".to_string());
                }
                (chatId, runtime, notifyReply, Some(userMessageTimestamp), AdmittedChatTurnWork::Continue { userMessageTimestamp, participantId })
            }
        };
        let lease = manager.beginChatExecution(&chatId).map_err(|error| error.to_string())?;
        let sequenceId = uuid::Uuid::new_v4().to_string();
        let executionId = uuid::Uuid::new_v4().to_string();
        let cancelled = Arc::new(AtomicBool::new(false));
        let (terminal, _) = watch::channel(None);
        let slot = match runtime { ChatRuntime::Main => ChatRuntimeSlot::MAIN, ChatRuntime::Floating => ChatRuntimeSlot::FLOATING };
        self.sequences.insert(sequenceId.clone(), NativeChatSequence {
            owner: owner.to_string(), chatId: chatId.clone(), slot: slot.clone(), lease: lease.clone(), notifyReply,
            userMessageTimestamp, executions: vec![executionId.clone()], activeExecution: Some(executionId.clone()), finishing: false,
        });
        let handle = ChatTurnHandle { sequenceId: sequenceId.clone(), executionId: executionId.clone(), chatId };
        self.executions.insert(executionId, NativeChatExecution { owner: owner.to_string(), sequenceId, cancelled: cancelled.clone(), terminal, committedReceipt: None });
        Ok(AdmittedChatTurn { handle, slot, lease, cancelled, work, notifyReply, outputObserver })
    }

    /// Captures only the authenticated owner's current execution in this exact conversation.
    pub fn ownedActiveExecution(&self, owner: &str, chatId: &str) -> Result<Option<String>, String> {
        if owner.trim().is_empty() || chatId.trim().is_empty() { return Err("Chat cancellation requires an authenticated owner and chatId".to_string()); }
        let sequence = self.sequences.values().find(|sequence| sequence.owner == owner && sequence.chatId == chatId && sequence.activeExecution.is_some());
        Ok(sequence.and_then(|sequence| sequence.activeExecution.clone()))
    }

    /// Rejects unknown or foreign handles without inspecting plugin data or selecting another execution.
    fn requireSequence(&self, owner: &str, sequenceId: &str) -> Result<&NativeChatSequence, String> {
        let sequence = self.sequences.get(sequenceId).ok_or_else(|| format!("Unknown chat sequence: {sequenceId}"))?;
        if sequence.owner != owner { return Err("Chat sequence belongs to another authenticated owner".to_string()); }
        Ok(sequence)
    }

    /// Removes a rejected admission and releases its actual lease without publishing an unreachable send.
    pub fn rejectScheduling(&mut self, owner: &str, executionId: &str, error: String) -> Result<(), String> {
        let execution = self.executions.get(executionId).ok_or_else(|| "Rejected execution disappeared".to_string())?;
        if execution.owner != owner { return Err("Rejected execution belongs to another owner".to_string()); }
        let sequenceId = execution.sequenceId.clone();
        self.requireSequence(owner, &sequenceId)?;
        let sequence = self.sequences.remove(&sequenceId).ok_or_else(|| "Rejected send disappeared".to_string())?;
        self.executions.remove(executionId);
        sequence.lease.release().map_err(|release| format!("{error}; admission lease release failed: {release}"))
    }

    /// Marks only the authenticated handle for cancellation, including the admitted-before-start interval.
    pub fn requestCancellation(&mut self, owner: &str, executionId: &str) -> Result<Option<(String, ChatRuntimeSlot)>, String> {
        let execution = self.executions.get(executionId).ok_or_else(|| format!("Unknown chat execution: {executionId}"))?;
        if execution.owner != owner { return Err("Chat execution belongs to another authenticated owner".to_string()); }
        if execution.terminal.borrow().is_some() { return Ok(None); }
        let sequence = self.requireSequence(owner, &execution.sequenceId)?;
        if sequence.activeExecution.as_deref() != Some(executionId) || sequence.finishing {
            return Err("Chat execution no longer owns an active sequence turn".to_string());
        }
        execution.cancelled.store(true, Ordering::Release);
        Ok(Some((sequence.chatId.clone(), sequence.slot.clone())))
    }

    /// Stores only receipts delivered by the originating native pipeline and makes each execution terminal once.
    pub fn settle(
        &mut self, executionId: &str, result: Result<MessageSendResultData, String>,
        capturedReceipt: Option<MessageSendResultData>,
    ) -> Result<(), String> {
        let execution = self.executions.get_mut(executionId).ok_or_else(|| format!("Unknown settling chat execution: {executionId}"))?;
        if execution.terminal.borrow().is_some() { return Err("Chat execution already settled".to_string()); }
        let sequence = self.sequences.get_mut(&execution.sequenceId).ok_or_else(|| "Chat execution lost its sequence".to_string())?;
        if sequence.activeExecution.as_deref() != Some(executionId) { return Err("Terminal receipt does not belong to the active sequence execution".to_string()); }
        let committedReceipt = match &result { Ok(receipt) => Some(receipt.clone()), Err(_) => capturedReceipt };
        if let Some(receipt) = &committedReceipt {
            if receipt.chatId != sequence.chatId { return Err("Terminal receipt names a different conversation".to_string()); }
            if let MessageSendOutcome::Committed { userMessageTimestamp: Some(timestamp), .. } = &receipt.outcome {
                if sequence.userMessageTimestamp.is_some_and(|existing| existing != *timestamp) {
                    return Err("Native sequence changed its original committed user identity".to_string());
                }
                sequence.userMessageTimestamp = Some(*timestamp);
            }
        }
        execution.committedReceipt = committedReceipt;
        sequence.activeExecution = None;
        execution.terminal.send_replace(Some(result));
        Ok(())
    }

    /// Captures native aggregate receipts and prevents concurrent finish or continuation until publication commits.
    pub fn prepareFinish(&mut self, owner: &str, sequenceId: &str) -> Result<(ChatRuntimeSlot, bool, ChatSequenceFinishResultData), String> {
        let sequence = self.requireSequence(owner, sequenceId)?;
        if sequence.finishing || sequence.activeExecution.is_some() { return Err("Chat sequence is active or already finishing".to_string()); }
        let mut status = ChatSequenceStatus::Completed;
        let mut assistants = Vec::new();
        let mut failure = None;
        for id in &sequence.executions {
            let execution = self.executions.get(id).ok_or_else(|| "Sequence receipt disappeared".to_string())?;
            if let Some(receipt) = &execution.committedReceipt {
                if let MessageSendOutcome::Committed { assistant: Some(assistant), .. } = &receipt.outcome {
                    assistants.push(assistant.clone());
                }
            }
            let result = execution.terminal.borrow();
            match result.as_ref().ok_or_else(|| "Chat sequence contains an unsettled execution".to_string())? {
                Err(error) => { status = ChatSequenceStatus::Failed; failure = Some(error.clone()); }
                Ok(receipt) => match &receipt.outcome {
                    MessageSendOutcome::Committed { status: turnStatus, .. } => {
                        if status != ChatSequenceStatus::Failed {
                            status = match turnStatus { MessageSendStatus::Completed => ChatSequenceStatus::Completed, MessageSendStatus::Cancelled => ChatSequenceStatus::Cancelled };
                        }
                    }
                    MessageSendOutcome::NotPersisted { status: turnStatus } => {
                        if status != ChatSequenceStatus::Failed {
                            status = match turnStatus { MessageSendStatus::Completed => ChatSequenceStatus::Completed, MessageSendStatus::Cancelled => ChatSequenceStatus::Cancelled };
                        }
                    }
                    MessageSendOutcome::Blocked { .. } | MessageSendOutcome::Consumed { .. } => {
                        status = ChatSequenceStatus::Failed;
                        failure = Some("Sequence input was blocked or consumed; no native sequence generation completed".to_string());
                    }
                },
            }
        }
        let result = ChatSequenceFinishResultData { sequenceId: sequenceId.to_string(), chatId: sequence.chatId.clone(), status,
            userMessageTimestamp: sequence.userMessageTimestamp, assistants, error: failure };
        let slot = sequence.slot.clone();
        let notifyReply = sequence.notifyReply;
        self.sequences.get_mut(sequenceId).ok_or_else(|| "Finishing sequence disappeared".to_string())?.finishing = true;
        Ok((slot, notifyReply, result))
    }

    /// Removes only the successfully finalized sequence and its handles, releasing its one canonical store lease.
    pub fn completeFinish(&mut self, owner: &str, sequenceId: &str) -> Result<(), String> {
        let sequence = self.requireSequence(owner, sequenceId)?;
        if !sequence.finishing || sequence.activeExecution.is_some() { return Err("Chat sequence was not prepared for finalization".to_string()); }
        let sequence = self.sequences.remove(sequenceId).ok_or_else(|| "Finalized sequence disappeared".to_string())?;
        for id in &sequence.executions { self.executions.remove(id); }
        sequence.lease.release().map_err(|error| error.to_string())
    }


}

/// Shares only native execution coordination; conversation and message data remain in canonical records.
pub(crate) type SharedChatTurnTransport = Arc<Mutex<ChatTurnTransport>>;
