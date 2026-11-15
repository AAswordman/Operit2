//! Startup and plain chat must not wait for unrelated owner or recognition services.
#![allow(non_snake_case)]

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use operit_host_api::HostManager::HostManager;
use operit_host_api::*;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_model::FunctionType::FunctionType;
use operit_providers::chat::EnhancedAIService::{EnhancedAIService, SendMessageOptions};
use operit_providers::runtime_support::{ChatConfigurationResult, ChatParticipantProfile};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::data::preferences::FunctionalConfigManager::{
    FunctionModelBinding, FunctionalConfigManager,
};
use operit_runtime::data::preferences::ModelConfigManager::{ModelConfigError, ModelConfigManager};

/// Rejects network access during local startup and model construction.
struct UnusedHttp;
impl HttpStreamHost for UnusedHttp {
    /// Rejects opening a network stream.
    fn openHttpByteStream(
        &self,
        _: String,
        _: HttpRequestData,
        _: HttpStreamOpenedCallback,
        _: HttpStreamChunkCallback,
        _: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        panic!("unexpected HTTP stream")
    }
    /// Rejects closing an unopened stream.
    fn closeHttpByteStream(&self, _: &str) -> HostResult<()> {
        panic!("unexpected HTTP stream")
    }
}
impl HttpHost for UnusedHttp {
    /// Declares host-owned byte delivery.
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }
    /// Rejects buffered network requests.
    fn executeHttpRequest(&self, _: HttpRequestData) -> HostResult<HttpResponseData> {
        panic!("unexpected HTTP request")
    }
    /// Rejects network downloads.
    fn downloadFiles(
        &self,
        _: HttpDownloadRequest,
        _: HttpDownloadControl,
        _: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        panic!("unexpected HTTP download")
    }
}

/// Keeps background tasks outside the synchronous startup and model-construction assertions.
struct StartupScheduler;
impl HostRuntimeTaskSchedulerHost for StartupScheduler {
    /// Supplies a deterministic clock.
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        Ok(0)
    }
    /// Retains background work outside this isolated test boundary.
    fn scheduleHostRuntimeTask(&self, _: &str, _: HostRuntimeTask) -> HostResult<()> {
        Ok(())
    }
    /// Leaves plugin loading unexecuted while startup completes.
    fn scheduleHostRuntimeAsyncTask(&self, _: &str, _: HostRuntimeAsyncTask) -> HostResult<()> {
        Ok(())
    }
    /// Leaves cooperative work unexecuted while startup completes.
    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self,
        _: &str,
        _: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        Ok(())
    }
    /// Leaves timers unexecuted while startup completes.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        _: &str,
        _: u64,
        _: HostRuntimeTask,
    ) -> HostResult<()> {
        Ok(())
    }
    /// Completes observer turns immediately.
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture {
        Box::pin(async { Ok(()) })
    }
    /// Rejects unplanned waits.
    fn waitForHostRuntimeDelay(&self, _: u64) -> HostRuntimeTurnFuture {
        panic!("unexpected delay")
    }
}

/// Rejects JS execution because auxiliary packages must not be needed for these operations.
struct UnusedJavaScript;
impl HostJavaScriptRuntimeHost for UnusedJavaScript {
    /// Rejects constructing an engine.
    fn createHostJavaScriptRuntime(&self) -> HostResult<Box<dyn HostJavaScriptRuntime>> {
        panic!("unexpected JS")
    }
    /// Rejects constructing an engine state.
    fn createHostJavaScriptRuntimeState(
        &self,
        _: &str,
        _: HostJavaScriptRuntimeStateKind,
        _: HostJavaScriptRuntimeStateFactory,
    ) -> HostResult<HostJavaScriptRuntimeStateHandle> {
        panic!("unexpected JS")
    }
    /// Rejects executing an engine state.
    fn executeHostJavaScriptRuntimeStateTask(
        &self,
        _: HostJavaScriptRuntimeStateHandle,
        _: HostJavaScriptExecutionInterrupt,
        _: HostJavaScriptRuntimeStateTask,
    ) -> HostResult<HostJavaScriptRuntimeStateOutput> {
        panic!("unexpected JS")
    }
    /// Rejects enqueueing an engine task.
    fn enqueueHostJavaScriptRuntimeStateTask(
        &self,
        _: HostJavaScriptRuntimeStateHandle,
        _: HostJavaScriptExecutionInterrupt,
        _: HostJavaScriptRuntimeStateTask,
    ) -> HostResult<()> {
        panic!("unexpected JS")
    }
    /// Rejects executing an asynchronous engine task.
    fn executeHostJavaScriptRuntimeStateAsyncTask(
        &self,
        _: HostJavaScriptRuntimeStateHandle,
        _: HostJavaScriptExecutionInterrupt,
        _: HostJavaScriptRuntimeStateAsyncTask,
    ) -> HostJavaScriptRuntimeStateOutputFuture {
        panic!("unexpected JS")
    }
    /// Rejects destroying an engine state.
    fn destroyHostJavaScriptRuntimeState(
        &self,
        _: HostJavaScriptRuntimeStateHandle,
    ) -> HostResult<()> {
        panic!("unexpected JS")
    }
}

/// Counts owner package queries and rejects all unrelated system operations.
struct OwnerSystem(AtomicUsize);
impl SystemOperationHost for OwnerSystem {
    /// Models Android's package query crossing an owner interaction boundary.
    fn applicationUpdateBase(&self) -> HostResult<ApplicationUpdateBase> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(HostError::new("owner package query"))
    }
    /// Enables updater cleanup for this application.
    fn applicationUpdateTarget(&self) -> HostResult<Option<ApplicationUpdateTarget>> {
        Ok(Some(ApplicationUpdateTarget {
            installTiming: AppInstallTiming::Immediate,
            arch: "x86_64".into(),
            package: ApplicationUpdatePackage::AndroidApk,
        }))
    }
    /// Supplies the startup locale without owner interaction.
    fn getSystemLanguageCode(&self) -> HostResult<String> {
        Ok("en".into())
    }
    /// Rejects notifications.
    fn sendNotification(&self, _: &SystemNotificationRequest) -> HostResult<()> {
        panic!("unexpected system operation")
    }
    /// Rejects settings writes.
    fn modifySystemSetting(&self, _: &str, _: &str, _: &str) -> HostResult<SystemSettingData> {
        panic!("unexpected system operation")
    }
    /// Rejects settings reads.
    fn getSystemSetting(&self, _: &str, _: &str) -> HostResult<SystemSettingData> {
        panic!("unexpected system operation")
    }
    /// Rejects installation.
    fn installApp(&self, _: &dyn FileSource, _: AppInstallTiming) -> HostResult<AppOperationData> {
        panic!("unexpected system operation")
    }
    /// Rejects uninstallation.
    fn uninstallApp(&self, _: &str) -> HostResult<AppOperationData> {
        panic!("unexpected system operation")
    }
    /// Rejects package listing.
    fn listInstalledApps(&self, _: bool) -> HostResult<AppListData> {
        panic!("unexpected system operation")
    }
    /// Rejects application launch.
    fn startApp(&self, _: &str) -> HostResult<AppOperationData> {
        panic!("unexpected system operation")
    }
    /// Rejects application termination.
    fn stopApp(&self, _: &str) -> HostResult<AppOperationData> {
        panic!("unexpected system operation")
    }
    /// Rejects notification reads.
    fn getNotifications(&self, _: i32, _: bool) -> HostResult<NotificationData> {
        panic!("unexpected system operation")
    }
    /// Rejects usage reads.
    fn getAppUsageTime(
        &self,
        _: &str,
        _: i32,
        _: i32,
        _: bool,
    ) -> HostResult<AppUsageTimeResultData> {
        panic!("unexpected system operation")
    }
    /// Rejects location reads.
    fn getDeviceLocation(&self, _: i32, _: bool, _: bool) -> HostResult<LocationData> {
        panic!("unexpected system operation")
    }
    /// Rejects device information reads.
    fn getDeviceInfo(&self) -> HostResult<DeviceInfoData> {
        panic!("unexpected system operation")
    }
    /// Rejects screenshot capture.
    fn captureScreenshot(&self) -> HostResult<String> {
        panic!("unexpected system operation")
    }
    /// Rejects OCR.
    fn recognizeText(
        &self,
        _: &dyn FileSource,
        _: OCRLanguage,
        _: OCRQuality,
    ) -> HostResult<String> {
        panic!("unexpected system operation")
    }
}

/// Builds a valid native chat execution descriptor for the selected chat model.
fn configuration(binding: &FunctionModelBinding) -> ChatConfigurationResult {
    let profile: ChatParticipantProfile = serde_json::from_value(serde_json::json!({
        "id":"model:chat", "name":binding.modelId, "avatarUri":null,
        "introPrompt":"", "userPreferencesText":"", "openingStatement":"",
        "modelBinding":{"providerId":binding.providerId,"modelId":binding.modelId}, "ttsConfigId":"voice",
        "toolAccess":{"enabled":false,"allowedBuiltinTools":[],"allowedPackages":[],"allowedSkills":[],"allowedMcpServers":[]},
        "resources":[]
    })).unwrap();
    ChatConfigurationResult {
        contextKey: "model:chat".into(),
        historyPolicy: operit_providers::runtime_support::ChatHistoryPolicy {
            isolateParticipants: false,
            labelUserMessages: false,
        },
        executionData: serde_json::json!({"schemaVersion":1,"contextKey":"model:chat",
            "profile":profile,"participants":[profile]})
        .as_object()
        .unwrap()
        .clone(),
        profile: profile.clone(),
        participants: vec![profile],
        extensionOwner: "host.chat.model".into(),
    }
}

/// Reproduces both regressions with production startup, preferences and chat runtime construction.
#[tokio::test]
async fn startup_and_plain_chat_do_not_require_owner_or_missing_auxiliary_models() {
    let root = std::env::temp_dir().join(format!(
        "operit-startup-dependencies-{}",
        uuid::Uuid::new_v4()
    ));
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));
    let owner = Arc::new(OwnerSystem(AtomicUsize::new(0)));
    let mut context = HostManager::withFileSystemHost(Arc::new(
        operit_host_native_filesystem::PosixFileSystemHost::new(),
    ))
    .withRuntimeStorageHost(storage.clone())
    .withHttpHost(Arc::new(UnusedHttp))
    .withHostRuntimeTaskSchedulerHost(Arc::new(StartupScheduler))
    .withHostJavaScriptRuntimeHost(Arc::new(UnusedJavaScript));
    context.runtimeSqliteHost = Some(storage);
    context.systemOperationHost = Some(owner.clone());
    let mut application = OperitApplication::newWithContext(context);
    application.onCreate().unwrap();
    assert!(application.initialized);
    assert_eq!(
        owner.0.load(Ordering::SeqCst),
        0,
        "startup must not query the owner before Core connects"
    );
    assert_eq!(
        application.checkApplicationUpdate().await.unwrap_err(),
        "owner package query"
    );
    assert_eq!(
        owner.0.load(Ordering::SeqCst),
        1,
        "cleanup still runs when updates are checked"
    );

    let functions = FunctionalConfigManager::default();
    let mut bindings = functions
        .functionModelBindingFlow()
        .unwrap()
        .first()
        .unwrap();
    let chat = bindings[&FunctionType::CHAT].clone();
    let missing = FunctionModelBinding::new("removed-provider".into(), "removed-model".into());
    for role in [
        FunctionType::IMAGE_RECOGNITION,
        FunctionType::AUDIO_RECOGNITION,
        FunctionType::VIDEO_RECOGNITION,
    ] {
        bindings.insert(role, missing.clone());
    }
    functions
        .saveFunctionModelBinding(bindings.clone())
        .unwrap();
    let mut service = EnhancedAIService::new(
        application.toolHandler.clone(),
        application.providerRuntimeContext.clone(),
    );
    let runtime = service
        .createSendMessageRuntime(&SendMessageOptions::new(), configuration(&chat))
        .unwrap();
    assert_eq!(runtime.modelConfig.providerId, chat.providerId);
    assert!(
        !runtime.hasImageRecognition
            && !runtime.hasAudioRecognition
            && !runtime.hasVideoRecognition
    );
    assert!(matches!(
        ModelConfigManager::default().getResolvedModelConfig(&missing.providerId, &missing.modelId),
        Err(ModelConfigError::ProviderNotFound(_))
    ));

    let models = ModelConfigManager::default();
    let mut provider = models.getProviderProfile(&chat.providerId).unwrap();
    let model = provider
        .models
        .iter_mut()
        .find(|model| model.id == chat.modelId)
        .unwrap();
    model.capabilitiesOverride.directImage = Some(true);
    model.capabilitiesOverride.directAudio = Some(true);
    model.capabilitiesOverride.directVideo = Some(false);
    models.updateProviderProfile(provider).unwrap();
    for role in [
        FunctionType::IMAGE_RECOGNITION,
        FunctionType::AUDIO_RECOGNITION,
        FunctionType::VIDEO_RECOGNITION,
    ] {
        bindings.insert(role, chat.clone());
    }
    functions
        .saveFunctionModelBinding(bindings.clone())
        .unwrap();
    let runtime = service
        .createSendMessageRuntime(&SendMessageOptions::new(), configuration(&chat))
        .unwrap();
    assert!(runtime.hasImageRecognition && runtime.hasAudioRecognition);
    assert!(!runtime.hasVideoRecognition);
    for role in [
        FunctionType::IMAGE_RECOGNITION,
        FunctionType::AUDIO_RECOGNITION,
        FunctionType::VIDEO_RECOGNITION,
    ] {
        bindings.insert(
            role,
            FunctionModelBinding::new(chat.providerId.clone(), "removed-model".into()),
        );
    }
    functions.saveFunctionModelBinding(bindings).unwrap();
    let runtime = service
        .createSendMessageRuntime(&SendMessageOptions::new(), configuration(&chat))
        .unwrap();
    assert!(
        !runtime.hasImageRecognition
            && !runtime.hasAudioRecognition
            && !runtime.hasVideoRecognition
    );
    assert!(matches!(
        models.getResolvedModelConfig(&chat.providerId, "removed-model"),
        Err(ModelConfigError::ModelNotFound(_))
    ));
    assert!(
        service
            .createSendMessageRuntime(&SendMessageOptions::new(), configuration(&missing))
            .is_err(),
        "a missing selected chat model must still fail explicitly"
    );
}
