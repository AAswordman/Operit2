use std::sync::{Arc, Mutex};

use operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost;
use operit_host_api::{
    HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    HostRuntimeTurnFuture,
};
use operit_host_native_filesystem::PosixFileSystemHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_model::ChatEntity::ChatEntity;
use operit_model::FunctionType::FunctionType;
use operit_model::ModelConfigData::{ApiProviderType, ModelProfile, ProviderProfile};
use operit_model::PluginExtensionTarget::PluginExtensionTarget;
use operit_runtime::data::preferences::FunctionalConfigManager::{
    FunctionModelBinding, FunctionalConfigManager,
};
use operit_runtime::data::preferences::ModelConfigManager::ModelConfigManager;
use operit_runtime::services::core::ChatHistoryDelegate::ChatSelectionMode;
use operit_runtime::services::ChatServiceCore::ChatServiceCore;
use operit_store::db::AppDatabase::AppDatabase;
use operit_store::repository::ChatHistoryManager::ChatHistoryManager;
use operit_store::RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_vfs::VisualFileSystem::VisualFileSystem;

/// Delivers storage notifications inline while retaining unrelated runtime jobs.
#[derive(Default)]
struct SelectionScheduler(Mutex<Vec<HostRuntimeAsyncTask>>);

impl HostRuntimeTaskSchedulerHost for SelectionScheduler {
    /// Provides a deterministic host clock for notification scheduling.
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        Ok(0)
    }
    /// Publishes committed preference changes synchronously.
    fn scheduleHostRuntimeTask(&self, _: &str, task: HostRuntimeTask) -> HostResult<()> {
        task();
        Ok(())
    }
    /// Retains background work outside the model selection assertions.
    fn scheduleHostRuntimeAsyncTask(&self, _: &str, task: HostRuntimeAsyncTask) -> HostResult<()> {
        self.0.lock().unwrap().push(task);
        Ok(())
    }
    /// Retains cooperative work using the same isolated host queue.
    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self,
        name: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        self.scheduleHostRuntimeAsyncTask(name, task)
    }
    /// Rejects unexpected timers in synchronous model selection.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        _: &str,
        _: u64,
        _: HostRuntimeTask,
    ) -> HostResult<()> {
        panic!("selection must not schedule timers")
    }
    /// Rejects unexpected delays in synchronous model selection.
    fn waitForHostRuntimeDelay(&self, _: u64) -> HostRuntimeTurnFuture {
        panic!("selection must not wait for timers")
    }
}

/// Exercises durable isolation, global updates, exact provider IDs, removal and execution protection.
#[test]
fn conversation_model_selection_is_isolated_and_removed_models_use_global() {
    let root = std::env::temp_dir().join(format!("operit-chat-model-{}", uuid::Uuid::new_v4()));
    let host = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));
    setDefaultRuntimeStorageHost(host.clone());
    setDefaultRuntimeSqliteHost(host.clone());
    setDefaultHostRuntimeTaskSchedulerHost(Arc::new(SelectionScheduler::default()));
    let database = AppDatabase::getDatabase(RuntimeStorePaths::default()).unwrap();
    for id in ["chat-a", "chat-b", "chat-c"] {
        database
            .chatDao()
            .insertChat(ChatEntity::new(id.into(), id.into(), 1))
            .unwrap();
    }
    let models = ModelConfigManager::default();
    let mut provider = ProviderProfile::new(
        "provider-a".into(),
        "A".into(),
        ApiProviderType::LOCAL_MODEL,
        String::new(),
    );
    provider.models = vec![
        ModelProfile::new("shared-model".into()),
        ModelProfile::new("global-next".into()),
    ];
    let mut other = provider.clone();
    other.id = "provider-b".into();
    other.name = "B".into();
    models
        .importAllProvidersFromBackupContent(
            &serde_json::to_string(&vec![provider, other.clone()]).unwrap(),
        )
        .unwrap();
    let functional = FunctionalConfigManager::default();
    functional
        .setModelForFunction(
            FunctionType::CHAT,
            "provider-a".into(),
            "shared-model".into(),
        )
        .unwrap();
    let filesystem = Arc::new(
        VisualFileSystem::fromStorage(Arc::new(PosixFileSystemHost::new()), host).unwrap(),
    );
    let core = ChatServiceCore::new(ChatSelectionMode::FOLLOW_GLOBAL, filesystem.clone());
    let global = FunctionModelBinding::new("provider-a".into(), "shared-model".into());
    let fixed = FunctionModelBinding::new("provider-b".into(), "shared-model".into());
    assert_eq!(core.chatModelBinding("chat-a".into()).unwrap(), global);
    core.setChatModelBinding("chat-a".into(), Some(fixed.clone()))
        .unwrap();
    assert_eq!(core.chatModelBinding("chat-a".into()).unwrap(), fixed);
    assert_eq!(core.chatModelBinding("chat-b".into()).unwrap(), global);
    assert_eq!(
        functional
            .getModelBindingForFunction(FunctionType::CHAT)
            .unwrap(),
        global
    );

    functional
        .setModelForFunction(
            FunctionType::CHAT,
            "provider-a".into(),
            "global-next".into(),
        )
        .unwrap();
    let next = FunctionModelBinding::new("provider-a".into(), "global-next".into());
    assert_eq!(core.chatModelBinding("chat-a".into()).unwrap(), fixed);
    assert_eq!(core.chatModelBinding("chat-b".into()).unwrap(), next);
    assert_eq!(core.chatModelBinding("chat-c".into()).unwrap(), next);
    let reopened = ChatServiceCore::new(ChatSelectionMode::FOLLOW_GLOBAL, filesystem);
    assert_eq!(reopened.chatModelBinding("chat-a".into()).unwrap(), fixed);
    assert!(core
        .setChatModelBinding("missing".into(), Some(fixed.clone()))
        .is_err());
    assert!(core
        .setChatModelBinding(
            "chat-b".into(),
            Some(FunctionModelBinding::new(
                "missing".into(),
                "shared-model".into()
            ))
        )
        .is_err());

    let lease = database.store().beginChatExecution("chat-a").unwrap();
    assert!(core
        .setChatModelBinding("chat-a".into(), Some(next.clone()))
        .is_err());
    lease.release().unwrap();
    models.deleteModel("provider-b", "shared-model").unwrap();
    assert_eq!(core.chatModelBinding("chat-a".into()).unwrap(), next);
    let history = ChatHistoryManager::default().unwrap();
    let target = PluginExtensionTarget::Chat {
        chatId: "chat-a".into(),
    };
    assert!(history
        .readPluginExtension("host.chat.model.selection", &target)
        .unwrap()
        .is_none());
    models
        .importAllProvidersFromBackupContent(&serde_json::to_string(&vec![other.clone()]).unwrap())
        .unwrap();
    assert_eq!(core.chatModelBinding("chat-a".into()).unwrap(), next);
    let replacement = FunctionModelBinding::new("provider-b".into(), "global-next".into());
    core.setChatModelBinding("chat-a".into(), Some(replacement))
        .unwrap();
    assert_eq!(
        core.chatModelBinding("chat-a".into()).unwrap().modelId,
        "global-next"
    );
    models.deleteProvider("provider-b").unwrap();
    assert_eq!(core.chatModelBinding("chat-a".into()).unwrap(), next);
    assert!(history
        .readPluginExtension("host.chat.model.selection", &target)
        .unwrap()
        .is_none());
    models
        .importAllProvidersFromBackupContent(&serde_json::to_string(&vec![other]).unwrap())
        .unwrap();
    assert_eq!(core.chatModelBinding("chat-a".into()).unwrap(), next);

    core.setChatModelBinding("chat-a".into(), Some(global.clone()))
        .unwrap();
    core.setChatModelBinding("chat-a".into(), Some(next.clone()))
        .unwrap();
    functional
        .setModelForFunction(
            FunctionType::CHAT,
            global.providerId.clone(),
            global.modelId.clone(),
        )
        .unwrap();
    assert_eq!(core.chatModelBinding("chat-a".into()).unwrap(), global);
}
