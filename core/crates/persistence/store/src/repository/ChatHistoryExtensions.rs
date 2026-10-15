use super::*;
use operit_model::PluginExtensionTarget::PluginExtensionTarget;
use serde_json::Value;

impl ChatHistoryManager {
    /// Freezes generic conversation identity before invoking a generation's configuration and hooks.
    pub fn beginChatExecution(
        &self,
        chatId: &str,
    ) -> ChatHistoryManagerResult<crate::ChatExecutionLease::ChatExecutionLease> {
        Ok(self.database.store().beginChatExecution(chatId)?)
    }

    /// Reads one authenticated owner's object from an exact persisted record.
    pub fn readPluginExtension(
        &self,
        owner: &str,
        target: &PluginExtensionTarget,
    ) -> ChatHistoryManagerResult<Option<Value>> {
        if owner.trim().is_empty() {
            return Err(ChatHistoryManagerError::IllegalArgument(
                "Plugin extension owner must be nonempty".to_string(),
            ));
        }
        Ok(self.readPluginExtensions(target)?.get(owner).cloned())
    }

    /// Updates only one owner namespace with transactionally recorded sync state.
    pub fn writePluginExtension(
        &self,
        owner: &str,
        target: &PluginExtensionTarget,
        value: Value,
    ) -> ChatHistoryManagerResult<()> {
        self.syncStore
            .mutatePluginExtension(owner, target, Some(value))?;
        Ok(())
    }

    /// Removes only the requesting owner's namespace and returns whether it existed.
    pub fn deletePluginExtension(
        &self,
        owner: &str,
        target: &PluginExtensionTarget,
    ) -> ChatHistoryManagerResult<bool> {
        Ok(self.syncStore.mutatePluginExtension(owner, target, None)?)
    }

    /// Loads the complete generic map for internal lifecycle drafts without interpreting plugin data.
    pub fn readPluginExtensions(
        &self,
        target: &PluginExtensionTarget,
    ) -> ChatHistoryManagerResult<BTreeMap<String, Value>> {
        Ok(self.syncStore.readPluginExtensions(target)?)
    }

    /// Loads one exact revision's persisted fields and parts without consulting the selected revision or role name.
    pub fn loadChatMessageVariant(
        &self,
        chatId: &str,
        messageTimestamp: i64,
        variantIndex: i32,
    ) -> ChatHistoryManagerResult<ChatMessage> {
        let target = PluginExtensionTarget::Message {
            chatId: chatId.to_string(),
            messageTimestamp,
            variantIndex,
        };
        target
            .validate()
            .map_err(ChatHistoryManagerError::IllegalArgument)?;
        let base = self
            .messageDao
            .getMessageByTimestamp(chatId, messageTimestamp)?
            .ok_or_else(|| {
                ChatHistoryManagerError::IllegalArgument(format!(
                    "Message does not exist: {chatId}:{messageTimestamp}"
                ))
            })?;
        let variants = self
            .messageVariantDao
            .getVariantsForMessage(chatId, messageTimestamp)?;
        let parts = self
            .messagePartDao
            .getPartsForMessages(chatId, vec![messageTimestamp])?
            .into_iter()
            .filter(|part| part.variantIndex == variantIndex)
            .map(|part| part.toMessagePart())
            .collect();
        if variantIndex == 0 {
            let mut message = base.toChatMessage(parts);
            message.selectedVariantIndex = 0;
            message.variantCount = variants.len() as i32 + 1;
            return Ok(message);
        }
        let variant = variants
            .iter()
            .find(|variant| variant.variantIndex == variantIndex)
            .ok_or_else(|| {
                ChatHistoryManagerError::IllegalArgument(format!(
                    "Variant does not exist: {chatId}:{messageTimestamp}:{variantIndex}"
                ))
            })?;
        Ok(variant.applyTo(
            base.toChatMessage(Vec::new()),
            parts,
            variants.len() as i32 + 1,
        ))
    }
}
