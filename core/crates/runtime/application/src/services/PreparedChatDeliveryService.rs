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
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

const STORE: &str = "runtime/scheduled_chat_messages.json";
const MAX_PENDING: usize = 60;
#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    owner: String,
    request: ChatScheduledMessageRequest,
    status: String,
    #[serde(default)]
    cancelling: bool,
    #[serde(default)]
    notificationAttempted: bool,
    #[serde(default)]
    notificationRegistered: bool,
    error: Option<String>,
    #[serde(default)]
    retryAt: i64,
}
impl Entry {
    fn receipt(&self) -> ChatScheduledMessage {
        ChatScheduledMessage {
            id: self.request.id.clone(),
            chatId: self.request.chatId.clone(),
            timestamp: self.request.timestamp,
            status: self.status.clone(),
            error: JsNullable::from_option(self.error.clone()),
        }
    }
}
/// Owns one device-local queue independently of plugin runtime or UI lifetime.
pub struct PreparedChatDeliveryService {
    queue: tokio::sync::Mutex<()>,
    foreground: AtomicBool,
}
impl Default for PreparedChatDeliveryService {
    fn default() -> Self {
        Self {
            queue: tokio::sync::Mutex::new(()),
            foreground: AtomicBool::new(true),
        }
    }
}
impl PreparedChatDeliveryService {
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
            .getCore(ChatRuntimeSlot::MAIN)
            .chatHistoryDelegate
            .commitPreparedAssistantMessage(&request.chatId, &request.text, request.timestamp)?;
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
    pub async fn schedule(
        &self,
        host: &HostManager,
        manager: &ChatHistoryManager,
        owner: &str,
        request: ChatScheduledMessageRequest,
    ) -> Result<ChatScheduleMessageResultData, String> {
        requireOwner(owner)?;
        let _guard = self.queue.lock().await;
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
        if entries.iter().filter(|e| e.status == "scheduled").count() >= MAX_PENDING {
            return Err("Pending scheduled message capacity is full".into());
        }
        #[allow(unused_mut)]
        let mut entry = Entry {
            owner: owner.into(),
            request,
            status: "scheduled".into(),
            cancelling: false,
            notificationAttempted: false,
            notificationRegistered: false,
            error: None,
            retryAt: 0,
        };
        entries.push(entry.clone());
        Self::save(storage.as_ref(), &entries)?;
        #[cfg(target_os = "ios")]
        {
            // Persist the identity before asking iOS. Unknown results can safely retry the same ID.
            entry.notificationAttempted = true;
            *entries.last_mut().unwrap() = entry.clone();
            Self::save(storage.as_ref(), &entries)?;
            match nativeNotification("schedule_notification", &entry).await {
                Ok(()) => entry.notificationRegistered = true,
                Err(error) => {
                    entry.error = Some(error.clone());
                    entry.retryAt = currentTimeMillis() + 60000;
                    *entries.last_mut().unwrap() = entry;
                    Self::save(storage.as_ref(), &entries)?;
                    return Err(error);
                }
            }
            entry.error = None;
            *entries.last_mut().unwrap() = entry.clone();
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
            if entries[index].status == "scheduled" {
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
        for index in 0..entries.len() {
            if entries[index].status != "scheduled" {
                continue;
            }
            if entries[index].cancelling {
                if entries[index].retryAt > now {
                    continue;
                }
                match Self::cancelNative(&entries[index]).await {
                    Ok(()) => {
                        entries[index].status = "cancelled".into();
                        entries[index].cancelling = false;
                        entries[index].error = None;
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
                            entries[index].error = None;
                        }
                        Err(error) => {
                            entries[index].error = Some(error);
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
                runtime
                    .getCore(ChatRuntimeSlot::MAIN)
                    .chatHistoryDelegate
                    .commitPreparedAssistantMessage(
                        &request.chatId,
                        &request.text,
                        request.timestamp,
                    )
            };
            match result {
                Ok(duplicate) => {
                    // iOS owns the pre-registered alert. Resume only reconciles the chat message.
                    let error = if !duplicate && !cfg!(target_os = "ios") {
                        send(host, &request.chatId, &request.title, &request.text).err()
                    } else if cfg!(target_os = "ios") && !entries[index].notificationRegistered {
                        entries[index].error.clone()
                    } else {
                        None
                    };
                    entries[index].status = "delivered".into();
                    entries[index].error = error;
                }
                Err(error) => {
                    entries[index].error = Some(error); /* Keep retryable on a transient database error. */
                }
            }
            Self::save(storage.as_ref(), &entries)?;
        }
        // Retain bounded receipts independently of plugin polling, while never evicting pending work.
        if entries.iter().filter(|e| e.status != "scheduled").count() > 256 {
            let mut keep = 256;
            entries.reverse();
            entries.retain(|e| {
                if e.status == "scheduled" {
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
    let mut params = serde_json::to_value(&entry.request).map_err(|e| e.to_string())?;
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
