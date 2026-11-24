use crate::data::preferences::FunctionalConfigManager::FunctionModelBinding;
use crate::data::preferences::ModelConfigManager::{ModelConfigError, ModelConfigManager};
use operit_model::PluginExtensionTarget::PluginExtensionTarget;
use operit_store::repository::ChatHistoryManager::ChatHistoryManager;

/// Owns the native per-conversation model selection in the synchronized chat record.
pub(crate) struct ChatModelBindingService;

/// Keeps model selection separate from plugin configuration and message author identity.
const OWNER: &str = "host.chat.model.selection";

impl ChatModelBindingService {
    /// Cleans obsolete selections before reserving the conversation's execution generation.
    pub(crate) fn beginExecution(
        history: &ChatHistoryManager,
        chat_id: &str,
    ) -> Result<operit_store::ChatExecutionLease::ChatExecutionLease, String> {
        Self::read(history, &ModelConfigManager::default(), chat_id)?;
        history
            .beginChatExecution(chat_id)
            .map_err(|error| error.to_string())
    }

    /// Reads a chat override and durably removes references to deleted providers or models.
    pub(crate) fn read(
        history: &ChatHistoryManager,
        models: &ModelConfigManager,
        chat_id: &str,
    ) -> Result<Option<FunctionModelBinding>, String> {
        let target = PluginExtensionTarget::Chat {
            chatId: chat_id.into(),
        };
        let Some(value) = history
            .readPluginExtension(OWNER, &target)
            .map_err(|error| error.to_string())?
        else {
            return Ok(None);
        };
        let binding: FunctionModelBinding =
            serde_json::from_value(value).map_err(|error| error.to_string())?;
        match models.getModelProfile(&binding.providerId, &binding.modelId) {
            Ok(_) => Ok(Some(FunctionModelBinding::new(
                binding.providerId,
                binding.modelId,
            ))),
            Err(ModelConfigError::ProviderNotFound(_) | ModelConfigError::ModelNotFound(_)) => {
                history
                    .deletePluginExtension(OWNER, &target)
                    .map_err(|error| error.to_string())?;
                Ok(None)
            }
            Err(error) => Err(error.to_string()),
        }
    }

    /// Writes only this conversation's exact model pair, or clears its independent selection.
    pub(crate) fn write(
        history: &ChatHistoryManager,
        models: &ModelConfigManager,
        chat_id: &str,
        binding: Option<FunctionModelBinding>,
    ) -> Result<(), String> {
        let target = PluginExtensionTarget::Chat {
            chatId: chat_id.into(),
        };
        match binding {
            Some(binding) => {
                models
                    .getModelProfile(&binding.providerId, &binding.modelId)
                    .map_err(|error| error.to_string())?;
                let value = serde_json::json!({
                    "providerId": binding.providerId,
                    "modelId": binding.modelId,
                });
                history
                    .writePluginExtension(OWNER, &target, value)
                    .map_err(|error| error.to_string())
            }
            None => history
                .deletePluginExtension(OWNER, &target)
                .map(|_| ())
                .map_err(|error| error.to_string()),
        }
    }
}
