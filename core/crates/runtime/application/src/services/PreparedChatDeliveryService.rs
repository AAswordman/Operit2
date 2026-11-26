//! Durable scheduled assistant messages. All platform policy stays inside the host.
#![allow(non_snake_case)]
use crate::core::chat::{ChatRuntimeHolder::ChatRuntimeHolder, ChatRuntimeSlot::ChatRuntimeSlot};
use operit_host_api::{
    HostManager::HostManager, RuntimeStorageHost, SystemNotificationActivation,
    SystemNotificationRequest, TimeUtils::currentTimeMillis,
};
use operit_plugin_sdk::js_sdk::chat::{
    validatePreparedNotificationId, ChatPreparedMessageRequest, ChatScheduledMessageRequest,
};
use operit_plugin_sdk::js_sdk::results::{
    ChatCancelScheduledMessageResultData, ChatPreparedDeliveryResultData,
    ChatScheduleMessageResultData, ChatScheduledMessage, ChatScheduledMessagesResultData,
    JsNullable,
};
use operit_store::repository::ChatHistoryManager::ChatHistoryManager;
use operit_model::ChatMessage::ChatMessage;
use operit_providers::chat::EnhancedAIService::{EnhancedAIService, SendMessageCallbacks, SendMessageOptions};
use crate::core::chat::AIMessageManager::AIMessageManager;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, AtomicI64, Ordering},
    Arc,
};

const STORE: &str = "runtime/scheduled_chat_messages.json";
const MAX_PENDING: usize = 60;

/// Forwards provider retry notices while a scheduled reply is being prepared.
struct PreparedReplyCallbacks {
    errors: operit_store::PreferencesDataStore::MutableStateFlow<Option<String>>,
}

impl SendMessageCallbacks for PreparedReplyCallbacks {
    /// Delivers the provider's original error to the existing chat notice surface.
    fn onNonFatalError(&self, error: String) {
        self.errors.set_value(Some(format!("定时回复生成：{error}")));
    }

    /// Reports an exhausted request window through the scheduled reply notice surface.
    fn onTokenLimitExceeded(&self) {
        self.errors.set_value(Some("定时回复生成：上下文已达到压缩阈值".to_string()));
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    owner: String,
    request: ChatScheduledMessageRequest,
    status: String,
    #[serde(default)]
    cancelling: bool,
    #[serde(default)]
    invalidated: bool,
    #[serde(default)]
    notificationAttempted: bool,
    #[serde(default)]
    notificationRegistered: bool,
    #[serde(default)]
    notificationError: Option<String>,
    error: Option<String>,
    #[serde(default)]
    retryAt: i64,
    #[serde(default)]
    prepared: Option<PreparedReply>,
}
/// Stores the genuine generated message and the context on which it depends.
#[derive(Clone, Serialize, Deserialize)]
struct PreparedReply {
    message: ChatMessage,
    historyFingerprint: String,
    configurationFingerprint: String,
}
impl Entry {
    /// Returns the accepted body without replacing the immutable scheduling request.
    fn text(&self) -> String {
        match &self.prepared {
            Some(prepared) => prepared.message.displayText(),
            None => self.request.text.clone(),
        }
    }
    /// Publishes the durable scheduling outcome and the actual prepared body.
    fn receipt(&self) -> ChatScheduledMessage {
        ChatScheduledMessage {
            id: self.request.id.clone(),
            chatId: self.request.chatId.clone(),
            timestamp: self.request.timestamp,
            status: self.status.clone(),
            error: JsNullable::from_option(self.error.clone()),
            text: self.text(),
            notificationAttempted: self.notificationAttempted,
            notificationRegistered: self.notificationRegistered,
            notificationError: JsNullable::from_option(self.notificationError.clone()),
        }
    }
}
/// Owns one device-local queue independently of plugin runtime or UI lifetime.
pub struct PreparedChatDeliveryService {
    queue: tokio::sync::Mutex<()>,
    preparationQueue: tokio::sync::Mutex<()>,
    foreground: AtomicBool,
    nextContextCheck: AtomicI64,
}
impl Default for PreparedChatDeliveryService {
    fn default() -> Self {
        Self {
            queue: tokio::sync::Mutex::new(()),
            preparationQueue: tokio::sync::Mutex::new(()),
            foreground: AtomicBool::new(true),
            nextContextCheck: AtomicI64::new(0),
        }
    }
}
impl PreparedChatDeliveryService {
    /// Builds plain-text input for the same snapshot commit used by generated replies.
    fn textMessage(text: &str, timestamp: i64) -> ChatMessage {
        let mut message = ChatMessage::new_with_markdown_timestamp("ai".into(), text.into(), timestamp);
        message.sentAt = timestamp;
        message.completedAt = currentTimeMillis();
        message.pluginExtensions.insert("host.chat.scheduled".into(), serde_json::json!({ "preparedAt": message.completedAt }));
        message
    }

    pub fn setForeground(&self, foreground: bool) {
        self.foreground.store(foreground, Ordering::SeqCst);
    }
    fn storage(host: &HostManager) -> Result<Arc<dyn RuntimeStorageHost>, String> {
        host.runtimeStorageHost
            .clone()
            .ok_or_else(|| "Scheduled messages require runtime storage".into())
    }
    fn load(storage: &dyn RuntimeStorageHost) -> Result<Vec<Entry>, String> {
        if !storage.exists(STORE).map_err(|e| e.message)? {
            return Ok(vec![]);
        }
        serde_json::from_slice(&storage.readBytes(STORE).map_err(|e| e.message)?)
            .map_err(|e| format!("Scheduled message store is invalid: {e}"))
    }
    fn save(storage: &dyn RuntimeStorageHost, entries: &[Entry]) -> Result<(), String> {
        storage
            .writeBytesAtomically(
                STORE,
                &serde_json::to_vec(entries).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.message)
    }
    pub async fn deliver(
        host: &HostManager,
        holder: &Arc<tokio::sync::Mutex<ChatRuntimeHolder>>,
        request: ChatPreparedMessageRequest,
    ) -> Result<ChatPreparedDeliveryResultData, String> {
        operit_plugin_sdk::js_sdk::chat::validatePreparedMessageRequest(
            &request,
            currentTimeMillis(),
        )?;
        let duplicate = holder
            .lock()
            .await
            .commitPreparedAssistantMessage(&request.chatId, Self::textMessage(&request.text, request.timestamp))?;
        let error = if request.notify && !duplicate {
            send(host, &request.chatId, "Operit", &request.text).err()
        } else {
            None
        };
        Ok(ChatPreparedDeliveryResultData {
            chatId: request.chatId,
            timestamp: request.timestamp,
            duplicate,
            notificationError: JsNullable::from_option(error),
        })
    }
    /// Prepares contextual replies through the existing chat service and persists the full accepted message.
    pub async fn schedule(
        &self,
        host: &HostManager,
        manager: &ChatHistoryManager,
        owner: &str,
        request: ChatScheduledMessageRequest,
        holder: &Arc<tokio::sync::Mutex<ChatRuntimeHolder>>,
        service: Option<EnhancedAIService>,
    ) -> Result<ChatScheduleMessageResultData, String> {
        requireOwner(owner)?;
        let _preparation = self.preparationQueue.lock().await;
        let mut guard = self.queue.lock().await;
        let storage = Self::storage(host)?;
        let mut entries = Self::load(storage.as_ref())?;
        if let Some(entry) = entries
            .iter_mut()
            .find(|e| e.owner == owner && e.request.id == request.id)
        {
            if serde_json::to_value(&entry.request).map_err(|e| e.to_string())?
                != serde_json::to_value(&request).map_err(|e| e.to_string())?
            {
                return Err("Scheduled message ID already exists with different content; cancel it and use a new ID".into());
            }
            // An accepted request remains accepted across response loss or restart, even after its time passes.
            return Ok(ChatScheduleMessageResultData {
                message: entry.receipt(),
            });
        }
        operit_plugin_sdk::js_sdk::chat::validateChatScheduledMessageRequest(
            &request,
            currentTimeMillis(),
        )?;
        if !manager
            .chatExists(request.chatId.clone())
            .map_err(|e| e.to_string())?
        {
            return Err("Scheduled message target chat does not exist".into());
        }
        if host.hostRuntimeTaskSchedulerHost.is_none() {
            return Err("Scheduled messages require a runtime scheduler".into());
        }
        if entries.iter().filter(|e| matches!(e.status.as_str(), "scheduled" | "preparing")).count() >= MAX_PENDING {
            return Err("Pending scheduled message capacity is full".into());
        }
        let entry = Entry {
            owner: owner.into(), request: request.clone(), status: "preparing".into(),
            cancelling: false, invalidated: false, notificationAttempted: false,
            notificationRegistered: false, error: None, retryAt: 0, prepared: None,
            notificationError: None,
        };
        entries.push(entry);
        Self::save(storage.as_ref(), &entries)?;
        drop(guard);
        let prepared: Result<Option<PreparedReply>, String> = async { Ok(match &request.generation {
            Some(generation) => {
                let mut service = service.ok_or("Scheduled reply generation requires an initialized AI service")?;
                let (history, mut options) = context(holder, &request.chatId).await?;
                let configuration = resolveConfiguration(&service, &options).await?;
                let historyFingerprint = fingerprint(&history)?;
                let configurationFingerprint = fingerprint(&(&configuration, &configuration.extensionOwner))?;
                options.chatHistory = AIMessageManager::getMemoryFromMessages(history, &configuration).map_err(|e| e.to_string())?;
                options.message = format!("{}\n\nThis is a proactive assistant message prepared for Unix milliseconds {}. Follow the conversation's participant identity and language. The user has not just sent this instruction. Do not claim knowledge of future events. Tools are unavailable; output only the message body.", generation.instruction, request.timestamp);
                let mut message = service.prepareScheduledReply(options, configuration.clone()).await.map_err(|e| e.to_string())?;
                AIMessageManager::applyConfigurationIdentity(&mut message, &configuration);
                message.timestamp = request.timestamp;
                message.pluginExtensions.insert("host.chat.scheduled".into(), serde_json::json!({ "preparedAt": message.completedAt }));
                // A user turn or role change during generation invalidates the result before acceptance.
                let (latest, options) = context(holder, &request.chatId).await?;
                let latestConfiguration = resolveConfiguration(&service, &options).await?;
                if historyFingerprint != fingerprint(&latest)? || configurationFingerprint != fingerprint(&(&latestConfiguration, &latestConfiguration.extensionOwner))? {
                    return Err("Conversation changed while preparing the scheduled reply".into());
                }
                if request.timestamp <= currentTimeMillis() + 1000 { return Err("Scheduled reply preparation missed its delivery time".into()); }
                Some(PreparedReply { message, historyFingerprint, configurationFingerprint })
            }
            None => None,
        }) }.await;
        guard = self.queue.lock().await;
        entries = Self::load(storage.as_ref())?;
        let index = entries.iter().position(|entry| entry.owner == owner && entry.request.id == request.id)
            .ok_or("Scheduled preparation receipt disappeared")?;
        let mut entry = entries[index].clone();
        if entry.status != "preparing" { return Ok(ChatScheduleMessageResultData { message: entry.receipt() }); }
        match prepared {
            Ok(prepared) => { entry.prepared = prepared; entry.status = "scheduled".into(); }
            Err(error) => { entry.status = "failed".into(); entry.error = Some(error); }
        }
        entries[index] = entry.clone();
        Self::save(storage.as_ref(), &entries)?;
        if entry.status == "failed" { return Ok(ChatScheduleMessageResultData { message: entry.receipt() }); }
        #[cfg(target_os = "ios")]
        {
            // Persist the identity before asking iOS. Unknown results can safely retry the same ID.
            entry.notificationAttempted = true;
            entries[index] = entry.clone();
            Self::save(storage.as_ref(), &entries)?;
            match nativeNotification("schedule_notification", &entry).await {
                Ok(()) => { entry.notificationRegistered = true; entry.notificationError = None; }
                Err(error) => {
                    entry.notificationError = Some(error.clone());
                    entry.retryAt = currentTimeMillis() + 60000;
                    entries[index] = entry;
                    Self::save(storage.as_ref(), &entries)?;
                    return Err(error);
                }
            }
            entries[index] = entry.clone();
            Self::save(storage.as_ref(), &entries)?;
        }
        Ok(ChatScheduleMessageResultData {
            message: entry.receipt(),
        })
    }
    pub async fn cancel(
        &self,
        host: &HostManager,
        owner: &str,
        id: String,
    ) -> Result<ChatCancelScheduledMessageResultData, String> {
        requireOwner(owner)?;
        validatePreparedNotificationId(&id)?;
        let _guard = self.queue.lock().await;
        let storage = Self::storage(host)?;
        let mut entries = Self::load(storage.as_ref())?;
        if let Some(index) = entries
            .iter()
            .position(|e| e.owner == owner && e.request.id == id)
        {
            if matches!(entries[index].status.as_str(), "scheduled" | "preparing") {
                entries[index].cancelling = true;
                Self::save(storage.as_ref(), &entries)?;
                Self::cancelNative(&entries[index]).await?;
                entries[index].status = "cancelled".into();
                entries[index].cancelling = false;
                entries[index].error = None;
                Self::save(storage.as_ref(), &entries)?;
            }
        }
        Ok(ChatCancelScheduledMessageResultData {
            id,
            cancelled: true,
        })
    }
    async fn cancelNative(entry: &Entry) -> Result<(), String> {
        #[cfg(target_os = "ios")]
        if entry.notificationAttempted {
            nativeNotification("cancel_scheduled_notification", entry).await?;
        }
        let _ = entry;
        Ok(())
    }
    pub async fn list(
        &self,
        host: &HostManager,
        owner: &str,
    ) -> Result<ChatScheduledMessagesResultData, String> {
        requireOwner(owner)?;
        let _guard = self.queue.lock().await;
        let storage = Self::storage(host)?;
        Ok(ChatScheduledMessagesResultData {
            messages: Self::load(storage.as_ref())?
                .iter()
                .filter(|e| e.owner == owner)
                .map(Entry::receipt)
                .collect(),
        })
    }
    /// Runs automatically from the application scheduler, including after restart or resume.
    pub async fn reconcile(
        &self,
        host: &HostManager,
        holder: &Arc<tokio::sync::Mutex<ChatRuntimeHolder>>,
    ) -> Result<(), String> {
        if cfg!(target_os = "ios") && !self.foreground.load(Ordering::SeqCst) {
            return Ok(());
        }
        let _guard = self.queue.lock().await;
        let storage = Self::storage(host)?;
        let mut entries = Self::load(storage.as_ref())?;
        let now = currentTimeMillis();
        let checkFutureContext = now >= self.nextContextCheck.load(Ordering::SeqCst);
        if checkFutureContext { self.nextContextCheck.store(now + 15000, Ordering::SeqCst); }
        let mut contexts = std::collections::HashMap::new();
        for index in 0..entries.len() {
            if entries[index].status == "preparing" && self.preparationQueue.try_lock().is_ok() {
                entries[index].status = "failed".into();
                entries[index].error = Some("Scheduled reply preparation was interrupted".into());
                Self::save(storage.as_ref(), &entries)?;
            }
            if entries[index].status != "scheduled" {
                continue;
            }
            if entries[index].cancelling {
                if entries[index].retryAt > now {
                    continue;
                }
                match Self::cancelNative(&entries[index]).await {
                    Ok(()) => {
                        entries[index].status = if entries[index].invalidated { "invalidated" } else { "cancelled" }.into();
                        entries[index].cancelling = false;
                        if !entries[index].invalidated { entries[index].error = None; }
                    }
                    Err(error) => {
                        entries[index].error = Some(error);
                        entries[index].retryAt = now + 60000;
                    }
                }
                Self::save(storage.as_ref(), &entries)?;
                continue;
            }
            let exists = holder
                .lock()
                .await
                .getCore(ChatRuntimeSlot::MAIN)
                .chatHistoryDelegate
                .chatHistoryManager
                .chatExists(entries[index].request.chatId.clone())
                .map_err(|error| error.to_string())?;
            if !exists {
                Self::cancelNative(&entries[index]).await?;
                entries[index].status = "failed".into();
                entries[index].error =
                    Some("Scheduled message target chat no longer exists".into());
                Self::save(storage.as_ref(), &entries)?;
                continue;
            }
            // Recover a commit whose receipt write was interrupted before considering context invalidation.
            if let Some(prepared) = &entries[index].prepared {
                let mut runtime = holder.lock().await;
                if runtime.getCore(ChatRuntimeSlot::MAIN).chatHistoryDelegate.chatHistoryManager.loadChatMessageIndexPosition(&entries[index].request.chatId, prepared.message.timestamp).map_err(|e| e.to_string())?.is_some() {
                    runtime.commitPreparedAssistantMessage(&entries[index].request.chatId, prepared.message.clone())?;
                    entries[index].status = "delivered".into();
                    entries[index].error = None;
                    Self::save(storage.as_ref(), &entries)?;
                    continue;
                }
            }
            if entries[index].prepared.is_some() && (checkFutureContext || entries[index].request.timestamp <= now) {
                let chatId = entries[index].request.chatId.clone();
                if !contexts.contains_key(&chatId) {
                    let busy = holder.lock().await.cores.values().any(|core| core.messageProcessingDelegate.isChatProcessing(&chatId));
                    if busy { continue; }
                    let current: Result<_, String> = async {
                        let (history, options) = context(holder, &chatId).await?;
                        let service = holder.lock().await.getCore(ChatRuntimeSlot::MAIN).enhancedAiService.clone()
                            .ok_or("Scheduled reply context requires an initialized AI service")?;
                        let configuration = resolveConfiguration(&service, &options).await?;
                        Ok((fingerprint(&history)?, fingerprint(&(&configuration, &configuration.extensionOwner))?))
                    }.await;
                    contexts.insert(chatId.clone(), current);
                }
                let prepared = entries[index].prepared.as_ref().unwrap();
                let error = match &contexts[&chatId] {
                    Ok((history, configuration)) if &prepared.historyFingerprint == history && &prepared.configurationFingerprint == configuration => None,
                    Ok(_) => Some("Conversation or participant changed; prepare a new reply".into()),
                    Err(error) => Some(error.clone()),
                };
                if let Some(error) = error {
                    entries[index].cancelling = true;
                    entries[index].invalidated = true;
                    entries[index].error = Some(error);
                    Self::save(storage.as_ref(), &entries)?;
                    Self::cancelNative(&entries[index]).await?;
                    entries[index].status = "invalidated".into();
                    entries[index].cancelling = false;
                    Self::save(storage.as_ref(), &entries)?;
                    continue;
                }
            }
            if entries[index].request.timestamp > now {
                #[cfg(target_os = "ios")]
                if !entries[index].notificationRegistered
                    && entries[index].retryAt <= now
                    && self.foreground.load(Ordering::SeqCst)
                {
                    entries[index].notificationAttempted = true;
                    Self::save(storage.as_ref(), &entries)?;
                    match nativeNotification("schedule_notification", &entries[index]).await {
                        Ok(()) => {
                            entries[index].notificationRegistered = true;
                            entries[index].notificationError = None;
                        }
                        Err(error) => {
                            entries[index].notificationError = Some(error);
                            entries[index].retryAt = now + 60000;
                        }
                    }
                    Self::save(storage.as_ref(), &entries)?;
                }
                continue;
            }
            let request = entries[index].request.clone();
            if !cfg!(target_os = "ios")
                && (now - request.timestamp > 86_400_000
                    || entries.iter().any(|e| {
                        e.status == "scheduled"
                            && !e.cancelling
                            && e.request.chatId == request.chatId
                            && e.request.timestamp > request.timestamp
                            && e.request.timestamp <= now
                    }))
            {
                entries[index].status = "skipped".into();
                entries[index].error = Some(
                    "Missed delivery window; skipped to avoid a burst of overdue messages".into(),
                );
                Self::save(storage.as_ref(), &entries)?;
                continue;
            }
            let result = {
                let mut runtime = holder.lock().await;
                if runtime.cores.values().any(|core| {
                    core.messageProcessingDelegate
                        .isChatProcessing(&request.chatId)
                }) {
                    continue;
                }
                if let Some(prepared) = &entries[index].prepared {
                    if fingerprint(&runtime.getCore(ChatRuntimeSlot::MAIN).chatHistoryDelegate.getRuntimeChatHistory(request.chatId.clone()))? != prepared.historyFingerprint { continue; }
                }
                let message = match &entries[index].prepared {
                    Some(prepared) => prepared.message.clone(),
                    None => Self::textMessage(&request.text, request.timestamp),
                };
                runtime.commitPreparedAssistantMessage(&request.chatId, message)
            };
            match result {
                Ok(duplicate) => {
                    entries[index].status = "delivered".into();
                    entries[index].error = None;
                    // iOS owns the pre-registered alert. Resume only reconciles the chat message.
                    if !duplicate && !cfg!(target_os = "ios") {
                        entries[index].notificationAttempted = true;
                        Self::save(storage.as_ref(), &entries)?;
                        entries[index].notificationError = send(host, &request.chatId, &request.title, &entries[index].text()).err();
                        entries[index].notificationRegistered = entries[index].notificationError.is_none();
                    }
                }
                Err(error) => {
                    entries[index].error = Some(error); /* Keep retryable on a transient database error. */
                }
            }
            Self::save(storage.as_ref(), &entries)?;
        }
        // Retain bounded receipts independently of plugin polling, while never evicting pending work.
        if entries.iter().filter(|e| !matches!(e.status.as_str(), "scheduled" | "preparing")).count() > 256 {
            let mut keep = 256;
            entries.reverse();
            entries.retain(|e| {
                if matches!(e.status.as_str(), "scheduled" | "preparing") {
                    true
                } else if keep > 0 {
                    keep -= 1;
                    true
                } else {
                    false
                }
            });
            entries.reverse();
            Self::save(storage.as_ref(), &entries)?;
        }
        Ok(())
    }
}
fn requireOwner(owner: &str) -> Result<(), String> {
    if owner.trim().is_empty() {
        Err("Authenticated plugin owner is required".into())
    } else {
        Ok(())
    }
}
/// Hashes canonical serializable context without persisting prompt or permission snapshots.
fn fingerprint(value: &impl Serialize) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value).map_err(|e| e.to_string())?)))
}

/// Requires the selected configuration owner to resolve the reply instead of accepting a substituted native profile.
async fn resolveConfiguration(service: &EnhancedAIService, options: &SendMessageOptions)
    -> Result<operit_providers::runtime_support::ChatConfigurationResult, String> {
    let api = crate::services::ProviderRuntimeSupportService::ChatConfigurationApi::ready(&service.tool_handler).await;
    let configuration = service.resolveChatConfigurationForOptions(options).await.map_err(|e| e.to_string())?;
    if configuration.extensionOwner != api.extensionOwner() {
        return Err("The conversation's selected participant could not be resolved; scheduled reply was stopped".into());
    }
    Ok(configuration)
}

/// Captures the named conversation's complete summary-aware history and workspace context.
async fn context(
    holder: &Arc<tokio::sync::Mutex<ChatRuntimeHolder>>, chatId: &str,
) -> Result<(Vec<ChatMessage>, SendMessageOptions), String> {
    let mut runtime = holder.lock().await;
    if runtime.cores.values().any(|core| core.messageProcessingDelegate.isChatProcessing(chatId)) {
        return Err("Target conversation is processing".into());
    }
    let core = runtime.getCore(ChatRuntimeSlot::MAIN);
    let history = &core.chatHistoryDelegate;
    let chat = history.chatHistoryManager.loadChatHistory(chatId.into()).map_err(|e| e.to_string())?
        .ok_or("Scheduled reply target conversation does not exist")?;
    let options = SendMessageOptions {
        chatId: Some(chatId.into()), workspacePath: history.primaryWorkspacePathForChat(&chat),
        workspaceFolders: history.workspaceFolderPathsForChat(&chat),
        callbacks: Some(Arc::new(PreparedReplyCallbacks {
            errors: core.messageProcessingDelegate.toastEventFlow.clone(),
        })),
        ..SendMessageOptions::new()
    };
    Ok((history.getRuntimeChatHistory(chatId.into()), options))
}

fn send(host: &HostManager, chatId: &str, title: &str, text: &str) -> Result<(), String> {
    host.systemOperationHost
        .as_ref()
        .ok_or("System notification host is unavailable")?
        .sendNotification(&SystemNotificationRequest {
            title: title.into(),
            message: text.into(),
            activation: SystemNotificationActivation::OpenChat {
                chatId: chatId.into(),
            },
        })
        .map_err(|e| e.message)
}
#[cfg(any(target_os = "ios", test))]
fn nativeNotificationId(owner: &str, id: &str) -> String {
    use sha2::{Digest, Sha256};
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(owner, id)).expect("notification identity"))
    )
}
#[cfg(target_os = "ios")]
async fn nativeNotification(operation: &'static str, entry: &Entry) -> Result<(), String> {
    use crate::services::RuntimeHostInteractionService::{
        requestOwnerSystemOperation, RuntimeHostInteractionSystemOperationPayload,
    };
    let mut params = serde_json::json!({
        "id": entry.request.id, "chatId": entry.request.chatId, "title": entry.request.title,
        "text": entry.text(), "timestamp": entry.request.timestamp,
    });
    params["id"] = nativeNotificationId(&entry.owner, &entry.request.id).into();
    let response = tokio::task::spawn_blocking(move || {
        requestOwnerSystemOperation(
            RuntimeHostInteractionSystemOperationPayload {
                operation: operation.into(),
                paramsJson: params.to_string(),
            },
            std::time::Duration::from_secs(30),
        )
    })
    .await
    .map_err(|e| e.to_string())??;
    let result: serde_json::Value =
        serde_json::from_str(&response.resultJson).map_err(|e| e.to_string())?;
    if result["success"].as_bool() != Some(true) {
        return Err("System notification was not acknowledged".into());
    }
    Ok(())
}
